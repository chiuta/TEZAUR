//! Polite Crossref REST client (spec 12). Opt-in like everything in `online`: nothing here runs
//! unless the user asked for an online check or a literature search.
//!
//! Conduct: identifies itself (user agent + `mailto`), one request at a time, paced by the
//! server's announced rate limit, bounded retries on 429, stops for good on 403, remembers
//! answers on disk, percent-encodes DOIs, tells "not found" from "registered elsewhere", and
//! decides retraction from the editorial-update records of the DOI.

use crate::ids;
use crate::sha256::sha256_hex;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const OK_TTL: u64 = 30 * 24 * 3600;
const MISS_TTL: u64 = 24 * 3600;
const RETRY_PAUSES: [u64; 3] = [2, 4, 8];
const SELECT: &str =
    "DOI,title,author,issued,container-title,type,subtype,update-to,is-referenced-by-count";

#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Base URL without trailing slash.
    pub base: String,
    /// Contact address for the polite pool (user agent and `mailto` parameter).
    pub contact: Option<String>,
    /// Where answers are remembered; None disables the cache.
    pub cache_dir: Option<PathBuf>,
    /// Ignore remembered answers (new answers are still stored).
    pub bypass_cache: bool,
    /// Tests: a virtual clock, so waits cost no real time.
    pub fake_clock: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub sent: u32,
    pub cached: u32,
    /// Answers 429 or 403 received.
    pub refused: u32,
}

#[derive(Debug)]
pub enum Reply {
    Json(Value),
    NotFound,
    RateLimited,
    Blocked,
    Failed(String),
}

#[derive(Debug, PartialEq)]
pub enum Lookup {
    Found(Value),
    NotFound,
    /// Registered with another DOI agency (its id), not checkable here.
    Elsewhere(String),
    RateLimited,
    Blocked,
    Failed(String),
}

struct Clock {
    fake: bool,
    virt: Duration,
    start: Instant,
}

impl Clock {
    fn now(&self) -> Duration {
        if self.fake {
            self.virt
        } else {
            self.start.elapsed()
        }
    }
    fn sleep(&mut self, d: Duration) {
        if self.fake {
            self.virt += d;
        } else {
            std::thread::sleep(d);
        }
    }
}

pub struct Client {
    cfg: Config,
    clock: Clock,
    last_start: Option<Duration>,
    gap: Duration,
    blocked: bool,
    block_announced: bool,
    durations: Vec<Duration>,
    slow_pause: bool,
    noted_no_contact: bool,
    pub stats: Stats,
}

/// Percent-encodes everything except unreserved characters (and `/` when `keep_slash`).
pub fn encode(s: &str, keep_slash: bool) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        let keep = b.is_ascii_alphanumeric()
            || matches!(b, b'-' | b'.' | b'_' | b'~')
            || (keep_slash
                && matches!(
                    b,
                    b'/' | b':'
                        | b';'
                        | b'('
                        | b')'
                        | b','
                        | b'@'
                        | b'='
                        | b'+'
                        | b'!'
                        | b'$'
                        | b'&'
                        | b'\''
                        | b'*'
                ));
        if keep {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `X-Rate-Limit-Interval` such as `1s`, `60s`, `500ms`, `2m` (a bare number is seconds).
fn parse_interval(s: &str) -> Option<Duration> {
    let s = s.trim();
    let (num, unit) = match s.find(|c: char| !c.is_ascii_digit() && c != '.') {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, "s"),
    };
    let n: f64 = num.parse().ok()?;
    let secs = match unit.trim() {
        "s" | "" => n,
        "ms" => n / 1000.0,
        "m" => n * 60.0,
        _ => return None,
    };
    (secs.is_finite() && secs >= 0.0).then(|| Duration::from_secs_f64(secs))
}

/// Pause between request starts from the announced limit; None if unreadable.
fn gap_from_headers(limit: Option<&str>, interval: Option<&str>) -> Option<Duration> {
    let l: f64 = limit?.trim().parse().ok()?;
    let i = parse_interval(interval?)?;
    if l < 1.0 {
        return None;
    }
    Some(
        i.div_f64(l)
            .clamp(Duration::from_millis(5), Duration::from_secs(60)),
    )
}

/// Extra pause when a response was slow: over 3x the median of earlier ones and at least 5 s.
fn slow_pause_needed(previous: &[Duration], this: Duration) -> bool {
    if previous.is_empty() {
        return false;
    }
    let mut v = previous.to_vec();
    v.sort();
    let median = v[v.len() / 2];
    this > (median * 3).max(Duration::from_secs(5))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Client {
    pub fn new(cfg: Config) -> Client {
        let fake = cfg.fake_clock;
        Client {
            cfg,
            clock: Clock {
                fake,
                virt: Duration::ZERO,
                start: Instant::now(),
            },
            last_start: None,
            gap: Duration::from_secs(1),
            blocked: false,
            block_announced: false,
            durations: vec![],
            slow_pause: false,
            noted_no_contact: false,
            stats: Stats::default(),
        }
    }

    /// Time spent waiting or working so far (virtual in tests).
    pub fn elapsed(&self) -> Duration {
        self.clock.now()
    }

    pub fn user_agent(&self) -> String {
        let v = env!("CARGO_PKG_VERSION");
        match &self.cfg.contact {
            Some(c) => format!("tezaur/{v} (https://trom.tf; mailto:{c})"),
            None => format!("tezaur/{v} (https://trom.tf)"),
        }
    }

    /// True once, for the request that met the block; later calls say false.
    pub fn announce_block(&mut self) -> bool {
        let first = self.blocked && !self.block_announced;
        if first {
            self.block_announced = true;
        }
        first
    }

    pub fn is_blocked(&self) -> bool {
        self.blocked
    }

    fn pace(&mut self) {
        if let Some(last) = self.last_start {
            let ready = last + self.gap;
            let now = self.clock.now();
            if ready > now {
                self.clock.sleep(ready - now);
            }
        }
        if self.slow_pause {
            self.slow_pause = false;
            self.clock.sleep(Duration::from_secs(5));
        }
    }

    fn url(&self, path: &str, query: &[(&str, String)]) -> String {
        let mut u = format!("{}{}", self.cfg.base, path);
        let mut q: Vec<String> = query
            .iter()
            .map(|(k, v)| format!("{k}={}", encode(v, false)))
            .collect();
        if let Some(c) = &self.cfg.contact {
            q.push(format!("mailto={}", encode(c, false)));
        }
        if !q.is_empty() {
            u.push('?');
            u.push_str(&q.join("&"));
        }
        u
    }

    /// One GET with pacing, bounded 429 retries and the 403 stop. `path` is already encoded.
    pub fn get(&mut self, path: &str, query: &[(&str, String)]) -> Reply {
        if self.blocked {
            return Reply::Blocked;
        }
        if self.cfg.contact.is_none() && !self.noted_no_contact {
            self.noted_no_contact = true;
            eprintln!(
                "[note: no contact address set (--mailto / TEZAUR_MAILTO); Crossref's public pool has lower limits]"
            );
        }
        let url = self.url(path, query);
        let ua = self.user_agent();
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(8))
            .timeout_read(Duration::from_secs(25))
            .build();
        for attempt in 0..=RETRY_PAUSES.len() {
            self.pace();
            self.last_start = Some(self.clock.now());
            let t0 = Instant::now();
            self.stats.sent += 1;
            let res = agent
                .get(&url)
                .set("User-Agent", &ua)
                .set(
                    "Accept",
                    "application/vnd.crossref-api-message+json, application/json",
                )
                .call();
            let took = if self.cfg.fake_clock {
                Duration::ZERO
            } else {
                t0.elapsed()
            };
            let resp = match res {
                Ok(r) => r,
                Err(ureq::Error::Status(code, r)) => {
                    self.read_limits(&r);
                    match code {
                        404 => return Reply::NotFound,
                        429 => {
                            self.stats.refused += 1;
                            if attempt == RETRY_PAUSES.len() {
                                return Reply::RateLimited;
                            }
                            let wait = r
                                .header("Retry-After")
                                .and_then(|h| h.trim().parse::<u64>().ok())
                                .map(|s| s.min(120))
                                .unwrap_or(RETRY_PAUSES[attempt]);
                            self.clock.sleep(Duration::from_secs(wait));
                            continue;
                        }
                        403 => {
                            self.stats.refused += 1;
                            self.blocked = true;
                            return Reply::Blocked;
                        }
                        c => return Reply::Failed(format!("HTTP {c}")),
                    }
                }
                Err(e) => return Reply::Failed(e.to_string()),
            };
            self.read_limits(&resp);
            if slow_pause_needed(&self.durations, took) {
                self.slow_pause = true;
            }
            self.durations.push(took);
            return match resp.into_json::<Value>() {
                Ok(v) => Reply::Json(v),
                Err(_) => Reply::Failed("the answer is not JSON".into()),
            };
        }
        Reply::RateLimited
    }

    fn read_limits(&mut self, r: &ureq::Response) {
        self.gap = gap_from_headers(
            r.header("X-Rate-Limit-Limit"),
            r.header("X-Rate-Limit-Interval"),
        )
        .unwrap_or(Duration::from_secs(1));
    }

    // ------------------------------------------------------------ cache

    fn cache_path(&self, key: &str) -> Option<PathBuf> {
        let d = self.cfg.cache_dir.as_ref()?;
        Some(d.join(format!("{}.json", &sha256_hex(key.as_bytes())[..32])))
    }

    fn cache_get(&mut self, key: &str, now: u64) -> Option<(bool, Value)> {
        if self.cfg.bypass_cache {
            return None;
        }
        let v: Value = serde_json::from_slice(&std::fs::read(self.cache_path(key)?).ok()?).ok()?;
        let (t, hit) = (v["t"].as_u64()?, v["kind"].as_str()? == "ok");
        let ttl = if hit { OK_TTL } else { MISS_TTL };
        if now.saturating_sub(t) > ttl {
            return None;
        }
        self.stats.cached += 1;
        Some((hit, v["body"].clone()))
    }

    fn cache_put(&self, key: &str, hit: bool, body: &Value, now: u64) {
        let Some(p) = self.cache_path(key) else {
            return;
        };
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let v = json!({"t": now, "kind": if hit {"ok"} else {"miss"}, "key": key, "body": body});
        let _ = std::fs::write(p, v.to_string());
    }

    // ------------------------------------------------------------ operations

    fn doi_parts(doi: &str) -> (String, String) {
        match ids::normalize_doi(doi) {
            Some(f) => (f.display, f.canonical),
            None => {
                let t = doi.trim().to_string();
                (t.clone(), t.to_ascii_lowercase())
            }
        }
    }

    /// Work metadata for a DOI; on 404 asks the DOI agency before saying "not found".
    pub fn work(&mut self, doi: &str) -> Lookup {
        self.work_at(doi, unix_now())
    }

    fn work_at(&mut self, doi: &str, now: u64) -> Lookup {
        let (name, canon) = Self::doi_parts(doi);
        let key = format!("work:{canon}");
        if let Some((hit, body)) = self.cache_get(&key, now) {
            return if hit {
                Lookup::Found(body)
            } else {
                match body["agency"].as_str() {
                    Some(a) => Lookup::Elsewhere(a.to_string()),
                    None => Lookup::NotFound,
                }
            };
        }
        let path = format!("/works/{}", encode(&name, true));
        match self.get(&path, &[]) {
            Reply::Json(v) => {
                let m = v["message"].clone();
                self.cache_put(&key, true, &m, now);
                Lookup::Found(m)
            }
            Reply::NotFound => match self.get(&format!("{path}/agency"), &[]) {
                Reply::Json(a) => match agency_of(&a) {
                    Some(id) if !id.eq_ignore_ascii_case("crossref") => {
                        self.cache_put(&key, false, &json!({"agency": id}), now);
                        Lookup::Elsewhere(id)
                    }
                    _ => {
                        self.cache_put(&key, false, &json!({}), now);
                        Lookup::NotFound
                    }
                },
                Reply::NotFound => {
                    self.cache_put(&key, false, &json!({}), now);
                    Lookup::NotFound
                }
                Reply::RateLimited => Lookup::RateLimited,
                Reply::Blocked => Lookup::Blocked,
                Reply::Failed(e) => Lookup::Failed(e),
            },
            Reply::RateLimited => Lookup::RateLimited,
            Reply::Blocked => Lookup::Blocked,
            Reply::Failed(e) => Lookup::Failed(e),
        }
    }

    /// Is the work with this DOI retracted? True when some record that updates it carries an
    /// `update-to` entry naming this DOI with type `retraction`.
    pub fn retracted(&mut self, doi: &str) -> Result<bool, String> {
        self.retracted_at(doi, unix_now())
    }

    fn retracted_at(&mut self, doi: &str, now: u64) -> Result<bool, String> {
        let (name, canon) = Self::doi_parts(doi);
        let key = format!("updates:{canon}");
        if let Some((_, body)) = self.cache_get(&key, now) {
            return Ok(body["retracted"].as_bool().unwrap_or(false));
        }
        let q = [
            ("filter", format!("updates:{name}")),
            ("select", SELECT.to_string()),
            ("rows", "20".to_string()),
        ];
        match self.get("/works", &q) {
            Reply::Json(v) => {
                let r = v["message"]["items"].as_array().is_some_and(|items| {
                    items.iter().any(|it| {
                        it["update-to"].as_array().is_some_and(|us| {
                            us.iter().any(|u| {
                                u["type"].as_str() == Some("retraction")
                                    && u["DOI"]
                                        .as_str()
                                        .is_some_and(|d| Self::doi_parts(d).1 == canon)
                            })
                        })
                    })
                });
                self.cache_put(&key, true, &json!({"retracted": r}), now);
                Ok(r)
            }
            Reply::NotFound => Ok(false),
            Reply::RateLimited => Err("rate limited".into()),
            Reply::Blocked => Err("blocked by Crossref (HTTP 403)".into()),
            Reply::Failed(e) => Err(e),
        }
    }

    /// Up to `n` results for a free-text query, `rows` per request; beyond one page the cursor
    /// from each answer is followed until a page comes back short.
    pub fn search(&mut self, query: &str, n: usize, rows: usize) -> Result<Vec<Value>, String> {
        let rows = rows.clamp(1, 1000);
        let mut out: Vec<Value> = Vec::new();
        let paged = n > rows;
        let mut cursor = "*".to_string();
        loop {
            let want = rows.min(n - out.len());
            let mut q = vec![
                ("query", query.to_string()),
                ("rows", want.to_string()),
                ("select", SELECT.to_string()),
            ];
            if paged {
                q.push(("cursor", cursor.clone()));
            }
            let v = match self.get("/works", &q) {
                Reply::Json(v) => v,
                Reply::NotFound => return Ok(out),
                Reply::RateLimited => return Err("rate limited".into()),
                Reply::Blocked => return Err("blocked by Crossref (HTTP 403)".into()),
                Reply::Failed(e) => return Err(e),
            };
            let items = v["message"]["items"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            let got = items.len();
            out.extend(items);
            let next = v["message"]["next-cursor"].as_str().map(String::from);
            if !paged || out.len() >= n || got < want {
                break;
            }
            match next {
                Some(c) => cursor = c,
                None => break,
            }
        }
        out.truncate(n);
        Ok(out)
    }
}

/// Agency id from the `/agency` answer. The shape is not given by the specification; the common
/// forms (`agency` as string or as object with `id` / `label`) are accepted.
fn agency_of(v: &Value) -> Option<String> {
    let a = &v["message"]["agency"];
    a.as_str()
        .or_else(|| a["id"].as_str())
        .or_else(|| a["label"].as_str())
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Debug)]
    struct Seen {
        url: String,
        ua: String,
    }

    type Handler = Box<dyn FnMut(&str, usize) -> (u16, Vec<(String, String)>, String) + Send>;

    /// Local stand-in for the API: `handler(decoded url, request number)` -> (status, headers, body).
    struct Fake {
        base: String,
        seen: Arc<Mutex<Vec<Seen>>>,
    }

    fn pct_decode(s: &str) -> String {
        let b = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'%' && i + 2 < b.len() {
                if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
            }
            out.push(b[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    fn fake(mut handler: Handler) -> Fake {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let base = format!("http://{}", server.server_addr().to_ip().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s2 = seen.clone();
        std::thread::spawn(move || {
            for (n, req) in server.incoming_requests().enumerate() {
                let url = req.url().to_string();
                let ua = req
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("User-Agent"))
                    .map(|h| h.value.to_string())
                    .unwrap_or_default();
                s2.lock().unwrap().push(Seen {
                    url: url.clone(),
                    ua,
                });
                let (code, hdrs, body) = handler(&pct_decode(&url), n);
                let mut r = tiny_http::Response::from_string(body).with_status_code(code);
                for (k, v) in hdrs {
                    r = r.with_header(
                        tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap(),
                    );
                }
                let _ = req.respond(r);
            }
        });
        Fake { base, seen }
    }

    fn cfg(f: &Fake) -> Config {
        Config {
            base: f.base.clone(),
            fake_clock: true,
            ..Default::default()
        }
    }

    fn ok(body: Value) -> (u16, Vec<(String, String)>, String) {
        (200, vec![], body.to_string())
    }

    #[test]
    fn identifies_itself_with_contact() {
        let f = fake(Box::new(|_, _| {
            ok(json!({"message": {"DOI": "10.1000/a"}}))
        }));
        let mut c = Client::new(Config {
            contact: Some("me@example.org".into()),
            ..cfg(&f)
        });
        assert!(matches!(c.work("10.1000/a"), Lookup::Found(_)));
        let s = f.seen.lock().unwrap()[0].clone();
        assert!(s.ua.contains("mailto:me@example.org"), "{}", s.ua);
        assert!(s.url.contains("mailto=me%40example.org"), "{}", s.url);
        let mut d = Client::new(cfg(&f));
        assert!(!d.user_agent().contains("mailto"));
        let _ = d.work("10.1000/b");
        assert!(!f.seen.lock().unwrap()[1].url.contains("mailto"));
    }

    #[test]
    fn doi_is_percent_encoded_in_the_path() {
        let f = fake(Box::new(|_, _| ok(json!({"message": {}}))));
        let mut c = Client::new(cfg(&f));
        let _ = c.work("10.1000/a#b?c");
        let u = f.seen.lock().unwrap()[0].url.clone();
        assert!(u.starts_with("/works/10.1000/a%23b%3Fc"), "{u}");
        assert_eq!(encode("a b%", false), "a%20b%25");
    }

    #[test]
    fn announced_limit_spaces_requests() {
        let f = fake(Box::new(|_, _| {
            (
                200,
                vec![
                    ("X-Rate-Limit-Limit".into(), "1".into()),
                    ("X-Rate-Limit-Interval".into(), "1s".into()),
                ],
                json!({"message": {}}).to_string(),
            )
        }));
        let mut c = Client::new(cfg(&f));
        for i in 0..5 {
            let _ = c.work(&format!("10.1000/x{i}"));
        }
        assert!(c.elapsed() >= Duration::from_secs(4), "{:?}", c.elapsed());
        assert_eq!(f.seen.lock().unwrap().len(), 5);
        // same rule with the real clock, at a small scale
        let g = fake(Box::new(|_, _| {
            (
                200,
                vec![
                    ("X-Rate-Limit-Limit".into(), "1".into()),
                    ("X-Rate-Limit-Interval".into(), "100ms".into()),
                ],
                json!({"message": {}}).to_string(),
            )
        }));
        let mut r = Client::new(Config {
            base: g.base.clone(),
            ..Default::default()
        });
        let t = Instant::now();
        for i in 0..5 {
            let _ = r.work(&format!("10.1000/y{i}"));
        }
        assert!(t.elapsed() >= Duration::from_millis(400));
    }

    #[test]
    fn rate_limit_headers_parse() {
        assert_eq!(
            gap_from_headers(Some("50"), Some("1s")),
            Some(Duration::from_millis(20))
        );
        assert_eq!(
            gap_from_headers(Some("3000"), Some("60s")),
            Some(Duration::from_millis(20))
        );
        assert_eq!(gap_from_headers(Some("x"), Some("1s")), None);
        assert_eq!(gap_from_headers(Some("5"), Some("??")), None);
        assert_eq!(parse_interval("2m"), Some(Duration::from_secs(120)));
    }

    #[test]
    fn retries_429_then_succeeds_or_gives_up() {
        let f = fake(Box::new(|_, n| {
            if n < 1 {
                (429, vec![], String::new())
            } else {
                ok(json!({"message": {"x": 1}}))
            }
        }));
        let mut c = Client::new(cfg(&f));
        assert!(matches!(c.work("10.1000/a"), Lookup::Found(_)));
        assert_eq!(c.stats.refused, 1);
        assert!(c.elapsed() >= Duration::from_secs(2));
        let g = fake(Box::new(|_, _| (429, vec![], String::new())));
        let mut d = Client::new(cfg(&g));
        assert_eq!(d.work("10.1000/a"), Lookup::RateLimited);
        assert_eq!(g.seen.lock().unwrap().len(), 4);
        assert!(d.elapsed() >= Duration::from_secs(14)); // 2 + 4 + 8
        let h = fake(Box::new(|_, n| {
            if n == 0 {
                (429, vec![("Retry-After".into(), "7".into())], String::new())
            } else {
                ok(json!({"message": {}}))
            }
        }));
        let mut e = Client::new(cfg(&h));
        let _ = e.work("10.1000/a");
        assert!(e.elapsed() >= Duration::from_secs(7) && e.elapsed() < Duration::from_secs(9));
    }

    #[test]
    fn forbidden_stops_the_run() {
        let f = fake(Box::new(|_, _| (403, vec![], String::new())));
        let mut c = Client::new(cfg(&f));
        assert_eq!(c.work("10.1000/a"), Lookup::Blocked);
        assert!(c.announce_block());
        assert_eq!(c.work("10.1000/b"), Lookup::Blocked);
        assert!(!c.announce_block());
        assert_eq!(f.seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn not_found_versus_other_agency() {
        let other = fake(Box::new(|u, _| {
            if u.ends_with("/agency") {
                ok(
                    json!({"message": {"DOI": "10.5555/x", "agency": {"id": "datacite", "label": "DataCite"}}}),
                )
            } else {
                (404, vec![], "Resource not found.".into())
            }
        }));
        let mut c = Client::new(cfg(&other));
        assert_eq!(c.work("10.5555/x"), Lookup::Elsewhere("datacite".into()));
        let cr = fake(Box::new(|u, _| {
            if u.ends_with("/agency") {
                ok(json!({"message": {"agency": {"id": "crossref"}}}))
            } else {
                (404, vec![], String::new())
            }
        }));
        let mut d = Client::new(cfg(&cr));
        assert_eq!(d.work("10.5555/x"), Lookup::NotFound);
    }

    #[test]
    fn retraction_from_updates_filter() {
        let mk = |kind: &'static str| {
            fake(Box::new(move |u, _| {
                assert!(u.contains("filter=updates:10.1000/x"), "{u}");
                ok(
                    json!({"message": {"items": [{"DOI": "10.1000/notice", "update-to": [{"DOI": "10.1000/X", "type": kind}]}]}}),
                )
            }))
        };
        let r = mk("retraction");
        assert_eq!(Client::new(cfg(&r)).retracted("10.1000/x"), Ok(true));
        let c = mk("correction");
        assert_eq!(Client::new(cfg(&c)).retracted("10.1000/x"), Ok(false));
        let e = fake(Box::new(|_, _| ok(json!({"message": {"items": []}}))));
        assert_eq!(Client::new(cfg(&e)).retracted("10.1000/x"), Ok(false));
    }

    #[test]
    fn answers_are_remembered_and_can_be_bypassed() {
        let dir = std::env::temp_dir().join(format!("tezaur-cr-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let f = fake(Box::new(|_, _| ok(json!({"message": {"title": ["T"]}}))));
        let base = Config {
            cache_dir: Some(dir.clone()),
            ..cfg(&f)
        };
        let mut a = Client::new(base.clone());
        assert!(matches!(a.work("10.1000/ABC"), Lookup::Found(_)));
        let mut b = Client::new(base.clone());
        assert!(matches!(b.work("10.1000/abc"), Lookup::Found(_)));
        assert_eq!((b.stats.sent, b.stats.cached), (0, 1));
        assert_eq!(f.seen.lock().unwrap().len(), 1);
        let mut c = Client::new(Config {
            bypass_cache: true,
            ..base
        });
        let _ = c.work("10.1000/abc");
        assert_eq!(f.seen.lock().unwrap().len(), 2);
        // expiry: 30 days for hits
        let mut d = Client::new(Config {
            cache_dir: Some(dir.clone()),
            ..cfg(&f)
        });
        let now = unix_now();
        assert!(matches!(
            d.work_at("10.1000/abc", now + OK_TTL - 10),
            Lookup::Found(_)
        ));
        assert_eq!(d.stats.sent, 0);
        let _ = d.work_at("10.1000/abc", now + OK_TTL + 3600);
        assert_eq!(d.stats.sent, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn misses_are_remembered_for_a_day() {
        let dir = std::env::temp_dir().join(format!("tezaur-cr-miss-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let f = fake(Box::new(|u, _| {
            if u.ends_with("/agency") {
                ok(json!({"message": {"agency": {"id": "crossref"}}}))
            } else {
                (404, vec![], String::new())
            }
        }));
        let mut a = Client::new(Config {
            cache_dir: Some(dir.clone()),
            ..cfg(&f)
        });
        let now = unix_now();
        assert_eq!(a.work_at("10.1000/gone", now), Lookup::NotFound);
        let sent = a.stats.sent;
        assert_eq!(a.work_at("10.1000/gone", now + 3600), Lookup::NotFound);
        assert_eq!(a.stats.sent, sent);
        let _ = a.work_at("10.1000/gone", now + MISS_TTL + 100);
        assert!(a.stats.sent > sent);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cursor_paging_over_rows() {
        let f = fake(Box::new(|u, n| {
            let items = |k: usize| {
                (0..k)
                    .map(|i| json!({"DOI": format!("10.1000/p{n}-{i}")}))
                    .collect::<Vec<_>>()
            };
            match n {
                0 => {
                    assert!(u.contains("cursor=*") && u.contains("rows=10"), "{u}");
                    ok(json!({"message": {"items": items(10), "next-cursor": "AoE+1"}}))
                }
                1 => {
                    assert!(
                        u.contains("cursor=AoE+1") || u.contains("cursor=AoE%2B1"),
                        "{u}"
                    );
                    ok(json!({"message": {"items": items(10), "next-cursor": "AoE+2"}}))
                }
                _ => ok(json!({"message": {"items": items(5), "next-cursor": "AoE+3"}})),
            }
        }));
        let mut c = Client::new(cfg(&f));
        let r = c.search("thesis", 25, 10).unwrap();
        assert_eq!(r.len(), 25);
        assert_eq!(f.seen.lock().unwrap().len(), 3);
        assert!(f.seen.lock().unwrap()[0].url.contains("select="));
        // a single page needs no cursor
        let g = fake(Box::new(|u, _| {
            assert!(!u.contains("cursor"), "{u}");
            ok(json!({"message": {"items": [{"DOI": "10.1000/q"}]}}))
        }));
        assert_eq!(Client::new(cfg(&g)).search("x", 5, 20).unwrap().len(), 1);
    }

    #[test]
    fn slow_answers_cause_a_pause() {
        let prev = vec![Duration::from_millis(200); 4];
        assert!(!slow_pause_needed(&prev, Duration::from_secs(4)));
        assert!(slow_pause_needed(&prev, Duration::from_secs(6)));
        assert!(!slow_pause_needed(&[], Duration::from_secs(60)));
        let slow = vec![Duration::from_secs(4); 3];
        assert!(!slow_pause_needed(&slow, Duration::from_secs(11)));
        assert!(slow_pause_needed(&slow, Duration::from_secs(13)));
    }

    #[test]
    fn network_and_format_errors_are_errors_not_misses() {
        let f = fake(Box::new(|_, _| {
            (200, vec![], "<html>not json</html>".into())
        }));
        assert!(matches!(
            Client::new(cfg(&f)).work("10.1000/a"),
            Lookup::Failed(_)
        ));
        let mut dead = Client::new(Config {
            base: "http://127.0.0.1:1".into(),
            fake_clock: true,
            ..Default::default()
        });
        assert!(matches!(dead.work("10.1000/a"), Lookup::Failed(_)));
    }
}
