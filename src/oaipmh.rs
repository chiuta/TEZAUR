//! OAI-PMH 2.0 harvester for institutional repositories (spec 13). Opt-in: runs only through the
//! explicit `oai` command. Uses Identify, ListRecords (and ListMetadataFormats to explain a
//! format error), reads Dublin Core (`oai_dc`) and turns records into bibliography entries.
//! Requests are sequential, spaced, and identify the program with a contact when one is set.

use crate::bib::Entry;
use crate::crossref::{self, Clock};
use crate::ids::{self, IdKind};
use crate::kinds::{self, Format};
use crate::xmlmini::{self, Node};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Base URL of the repository's OAI-PMH endpoint.
    pub base: String,
    /// Metadata prefix; `oai_dc` when empty.
    pub prefix: String,
    pub set: Option<String>,
    pub from: Option<String>,
    pub until: Option<String>,
    /// Stop after this many records (0 = default 1000).
    pub max_records: usize,
    pub contact: Option<String>,
    /// Tests: virtual clock.
    pub fake_clock: bool,
}

/// DOIs (canonical) and keys (lower case) already in the bibliography.
#[derive(Clone, Debug, Default)]
pub struct Known {
    pub dois: HashSet<String>,
    pub keys: HashSet<String>,
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub entries: Vec<Entry>,
    /// OAI identifiers the archive reports as deleted.
    pub deleted: Vec<String>,
    /// Newest datestamp received, as sent by the archive.
    pub newest: Option<String>,
    /// The whole list was read (not stopped by the cap). Only then is `newest` a safe next `from`.
    pub complete: bool,
    pub requests: u32,
    pub skipped_known: u32,
    pub list_size: Option<u64>,
    pub warnings: Vec<String>,
}

enum Fail {
    /// Transport problem, no usable answer.
    Net(String),
    /// Answer received but not usable (not XML, wrong root).
    Bad(String),
}

struct Http {
    clock: Clock,
    last: Option<Duration>,
    ua: String,
    requests: u32,
}

const MIN_GAP: Duration = Duration::from_secs(1);
const RETRY_DEFAULT_SECS: u64 = 10;

impl Http {
    fn fetch(&mut self, url: &str) -> Result<Node, Fail> {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(8))
            .timeout_read(Duration::from_secs(60))
            .redirects(3)
            .build();
        for attempt in 0..4 {
            if let Some(l) = self.last {
                let ready = l + MIN_GAP;
                let now = self.clock.now();
                if ready > now {
                    self.clock.sleep(ready - now);
                }
            }
            self.last = Some(self.clock.now());
            self.requests += 1;
            let res = agent
                .get(url)
                .set("User-Agent", &self.ua)
                .set("Accept", "text/xml, application/xml")
                .set("Accept-Encoding", "identity")
                .call();
            match res {
                Ok(resp) => {
                    let mut buf = Vec::new();
                    resp.into_reader()
                        .take(xmlmini::MAX_BYTES as u64 + 1)
                        .read_to_end(&mut buf)
                        .map_err(|e| Fail::Net(e.to_string()))?;
                    if buf.len() > xmlmini::MAX_BYTES {
                        return Err(Fail::Bad("answer larger than 16 MB; refused".into()));
                    }
                    let text = String::from_utf8_lossy(&buf);
                    return xmlmini::parse(&text).map_err(|e| {
                        Fail::Bad(format!("the answer is not valid OAI-PMH XML ({e})"))
                    });
                }
                Err(ureq::Error::Status(c @ (503 | 429), r)) if attempt < 3 => {
                    let _ = c;
                    let wait = r
                        .header("Retry-After")
                        .and_then(|h| h.trim().parse::<u64>().ok())
                        .unwrap_or(RETRY_DEFAULT_SECS)
                        .min(600);
                    self.clock.sleep(Duration::from_secs(wait));
                }
                Err(ureq::Error::Status(c, _)) => return Err(Fail::Net(format!("HTTP {c}"))),
                Err(e) => return Err(Fail::Net(e.to_string())),
            }
        }
        Err(Fail::Net(
            "the archive stayed unavailable (HTTP 503)".into(),
        ))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Gran {
    Day,
    Second,
}

fn gran_of(s: &str) -> Option<Gran> {
    let b = s.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    let day = b.len() >= 10
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10);
    match (day, b.len()) {
        (true, 10) => Some(Gran::Day),
        (true, 20)
            if b[10] == b'T'
                && digits(11..13)
                && b[13] == b':'
                && digits(14..16)
                && b[16] == b':'
                && digits(17..19)
                && b[19] == b'Z' =>
        {
            Some(Gran::Second)
        }
        _ => None,
    }
}

fn pad(s: &str) -> String {
    if s.len() == 10 {
        format!("{s}T00:00:00Z")
    } else {
        s.to_string()
    }
}

fn fit(date: &str, g: Gran) -> String {
    match g {
        Gran::Day => date.chars().take(10).collect(),
        Gran::Second => pad(date),
    }
}

/// Validates the date arguments before anything is sent.
fn check_dates(from: &Option<String>, until: &Option<String>) -> Result<Option<Gran>, String> {
    let mut g: Option<Gran> = None;
    for (name, v) in [("from", from), ("until", until)] {
        if let Some(v) = v {
            let k = gran_of(v).ok_or_else(|| {
                format!("`{name}` = `{v}` is not YYYY-MM-DD or YYYY-MM-DDThh:mm:ssZ (UTC)")
            })?;
            if g.is_some_and(|x| x != k) {
                return Err("`from` and `until` must have the same granularity".into());
            }
            g = Some(k);
        }
    }
    if let (Some(f), Some(u)) = (from, until)
        && f > u
    {
        return Err("`from` is after `until`".into());
    }
    Ok(g)
}

fn query(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", crossref::encode(v, false)))
        .collect::<Vec<_>>()
        .join("&")
}

fn url_for(base: &str, pairs: &[(&str, &str)]) -> String {
    let sep = if base.contains('?') { '&' } else { '?' };
    format!("{base}{sep}{}", query(pairs))
}

fn oai_error(doc: &Node) -> Option<(String, String)> {
    doc.child("error").map(|e| {
        (
            e.attr("code").unwrap_or("").to_string(),
            e.text().to_string(),
        )
    })
}

fn year_of(s: &str) -> String {
    let mut run = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            run.push(c);
            if run.len() == 4 {
                return run;
            }
        } else {
            run.clear();
        }
    }
    String::new()
}

fn people(vals: Vec<&str>) -> String {
    vals.iter()
        .map(|a| {
            if !a.contains(',') && a.split_whitespace().any(|w| w == "and") {
                format!("{{{a}}}")
            } else {
                a.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

/// Dublin Core record -> entry (without key). None when it has no title and no identifier.
fn dc_vals<'a>(dc: &'a Node, n: &str) -> Vec<&'a str> {
    dc.all(n)
        .map(|x| x.text())
        .filter(|t| !t.is_empty())
        .collect()
}

fn dc_entry(dc: &Node) -> Option<Entry> {
    let vals = |n: &str| dc_vals(dc, n);
    let mut e = Entry::default();
    e.title = vals("title")
        .first()
        .map(|s| s.to_string())
        .unwrap_or_default();
    e.raw_author = people(vals("creator"));
    e.author = e.raw_author.clone();
    e.editor = people(vals("contributor"));
    e.year = vals("date")
        .iter()
        .map(|d| year_of(d))
        .find(|y| !y.is_empty())
        .unwrap_or_default();
    e.publisher = vals("publisher")
        .first()
        .map(|s| s.to_string())
        .unwrap_or_default();
    e.container = vals("source")
        .first()
        .map(|s| s.to_string())
        .unwrap_or_else(|| e.publisher.clone());
    for id in vals("identifier") {
        if id.to_lowercase().starts_with("oai:") {
            continue; // the archive's own record id is not a bibliographic identifier
        }
        let is_url = id.starts_with("http://") || id.starts_with("https://");
        let found = ids::extract(id);
        let mut had_doi = false;
        for f in &found {
            match f.kind {
                IdKind::Doi if e.doi.is_empty() => {
                    e.doi = f.display.clone();
                    had_doi = true;
                }
                IdKind::Doi => had_doi = true,
                IdKind::Isbn10 | IdKind::Isbn13 if !is_url && e.isbn.is_empty() => {
                    e.isbn = f.canonical.clone()
                }
                IdKind::Issn if !is_url && e.issn.is_empty() => e.issn = f.canonical.clone(),
                _ => {}
            }
        }
        if is_url && !had_doi && e.url.is_empty() {
            e.url = id.to_string();
        }
    }
    let types = vals("type").join(" ").to_lowercase();
    let (csl, genre) = if ["thesis", "dissertation", "teză", "teza", "doctor"]
        .iter()
        .any(|k| types.contains(k))
    {
        ("thesis", kinds::genre_from_text(&types))
    } else if types.contains("article") {
        ("article-journal", None)
    } else if types.contains("book") {
        ("book", None)
    } else if types.contains("report") {
        ("report", None)
    } else {
        ("document", None)
    };
    e.origin = Some((Format::Csl, csl.to_string()));
    e.genre = genre;
    let notes: Vec<String> = ["relation", "coverage", "format", "rights"]
        .iter()
        .flat_map(|n| vals(n).into_iter().map(move |v| (*n, v)))
        .filter(|(_, v)| v.chars().count() <= 100)
        .map(|(n, v)| format!("{n}: {v}"))
        .collect();
    e.note = notes.join("; ");
    let has_id = !(e.doi.is_empty() && e.url.is_empty() && e.isbn.is_empty() && e.issn.is_empty());
    if e.title.is_empty() && !has_id {
        return None;
    }
    Some(e)
}

/// Harvests the archive. `known` is the bibliography already present (no duplicates are added).
pub fn harvest(cfg: &Config, known: &Known) -> Result<Outcome, String> {
    let base = cfg.base.trim().to_string();
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err("the archive address must start with http:// or https://".into());
    }
    let prefix = if cfg.prefix.is_empty() {
        "oai_dc"
    } else {
        cfg.prefix.as_str()
    };
    let want = check_dates(&cfg.from, &cfg.until)?;
    let cap = if cfg.max_records == 0 {
        1000
    } else {
        cfg.max_records
    };
    let mut http = Http {
        clock: Clock::new(cfg.fake_clock),
        last: None,
        ua: crossref::user_agent(cfg.contact.as_deref()),
        requests: 0,
    };
    let mut out = Outcome::default();
    let fail = |f: Fail| match f {
        Fail::Net(e) => format!("the archive could not be reached: {e}"),
        Fail::Bad(e) => e,
    };

    // Identify: granularity
    let ident = http
        .fetch(&url_for(&base, &[("verb", "Identify")]))
        .map_err(fail)?;
    if ident.name != "OAI-PMH" {
        return Err("the answer is not an OAI-PMH document (is this the endpoint address?)".into());
    }
    if let Some((c, t)) = oai_error(&ident) {
        return Err(format!("Identify failed: {c} {t}"));
    }
    let arch_gran = match ident
        .child("Identify")
        .and_then(|i| i.child_text("granularity"))
        .and_then(gran_of)
    {
        Some(g) => g,
        None => Gran::Day,
    };
    if want == Some(Gran::Second) && arch_gran == Gran::Day {
        return Err("the archive supports day granularity only; use YYYY-MM-DD".into());
    }
    let out_gran = want.unwrap_or(arch_gran);

    let mut used = known.keys.clone();
    let mut seen_dois = known.dois.clone();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut from = cfg.from.clone();
    let mut token: Option<String> = None;
    let mut restarts = 0;
    'outer: loop {
        let url = match &token {
            Some(t) => url_for(&base, &[("verb", "ListRecords"), ("resumptionToken", t)]),
            None => {
                let mut p = vec![("verb", "ListRecords"), ("metadataPrefix", prefix)];
                if let Some(s) = &cfg.set {
                    p.push(("set", s));
                }
                if let Some(f) = &from {
                    p.push(("from", f));
                }
                if let Some(u) = &cfg.until {
                    p.push(("until", u));
                }
                url_for(&base, &p)
            }
        };
        let doc = match http.fetch(&url) {
            Ok(d) => d,
            Err(Fail::Net(e)) if out.newest.is_some() && restarts < 3 => {
                out.warnings.push(format!(
                    "network problem ({e}); restarting from the newest datestamp"
                ));
                restarts += 1;
                token = None;
                from = out.newest.as_ref().map(|d| fit(d, out_gran));
                continue;
            }
            Err(f) => return Err(fail(f)),
        };
        if doc.name != "OAI-PMH" {
            return Err("the answer is not an OAI-PMH document".into());
        }
        if let Some((code, text)) = oai_error(&doc) {
            match code.as_str() {
                "noRecordsMatch" => {
                    out.complete = true;
                    break;
                }
                "badResumptionToken" if restarts < 3 => {
                    restarts += 1;
                    out.warnings.push(
                        "resumption token refused; restarting from the newest datestamp".into(),
                    );
                    token = None;
                    from = out.newest.as_ref().map(|d| fit(d, out_gran)).or(from);
                    if let (Some(f), Some(u)) = (&from, &cfg.until)
                        && pad(f) > pad(u)
                    {
                        out.complete = true;
                        break;
                    }
                    continue;
                }
                "cannotDisseminateFormat" => {
                    let mut msg = format!("the archive does not offer `{prefix}`");
                    if let Ok(f) = http.fetch(&url_for(&base, &[("verb", "ListMetadataFormats")])) {
                        let list: Vec<&str> = f
                            .child("ListMetadataFormats")
                            .map(|l| {
                                l.all("metadataFormat")
                                    .filter_map(|m| m.child_text("metadataPrefix"))
                                    .collect()
                            })
                            .unwrap_or_default();
                        if !list.is_empty() {
                            msg.push_str(&format!("; available: {}", list.join(", ")));
                        }
                    }
                    return Err(msg);
                }
                "noSetHierarchy" => {
                    return Err("the archive has no sets, but a set was requested".into());
                }
                _ => return Err(format!("the archive answered {code}: {text}")),
            }
        }
        let Some(lr) = doc.child("ListRecords") else {
            return Err("the answer has no ListRecords element".into());
        };
        for rec in lr.all("record") {
            let Some(h) = rec.child("header") else {
                continue;
            };
            let id = h.child_text("identifier").unwrap_or("").to_string();
            if !seen_ids.insert(id.clone()) {
                continue; // already received before a restart
            }
            if let Some(d) = h.child_text("datestamp").filter(|d| gran_of(d).is_some()) {
                if out.newest.as_ref().is_none_or(|n| pad(d) > pad(n)) {
                    out.newest = Some(d.to_string());
                }
            }
            if out.entries.len() + out.deleted.len() >= cap {
                out.warnings
                    .push(format!("stopped at the cap of {cap} records"));
                break 'outer;
            }
            if h.attr("status") == Some("deleted") {
                out.deleted.push(id);
                continue;
            }
            let Some(dc) = rec.child("metadata").and_then(|m| m.children.first()) else {
                out.warnings.push(format!("{id}: no metadata; skipped"));
                continue;
            };
            let Some(mut e) = dc_entry(dc) else {
                out.warnings
                    .push(format!("{id}: no title and no identifier; skipped"));
                continue;
            };
            if !e.doi.is_empty()
                && let Some(f) = ids::normalize_doi(&e.doi)
                && !seen_dois.insert(f.canonical)
            {
                out.skipped_known += 1;
                continue;
            }
            e.key = crate::ris::unique_key(&e, "", &mut used);
            out.entries.push(e);
        }
        let rt = lr.child("resumptionToken");
        if let Some(n) = rt
            .and_then(|r| r.attr("completeListSize"))
            .and_then(|v| v.parse().ok())
        {
            out.list_size = Some(n);
        }
        match rt.map(|r| r.text().to_string()).filter(|t| !t.is_empty()) {
            Some(t) => token = Some(t),
            None => {
                out.complete = true;
                break;
            }
        }
    }
    out.requests = http.requests;
    Ok(out)
}

// ------------------------------------------------------------------ incremental state

pub fn state_key(base: &str, set: &Option<String>) -> String {
    format!("{}|{}", base.trim(), set.as_deref().unwrap_or(""))
}

pub fn load_state(path: &Path) -> HashMap<String, String> {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save_state(path: &Path, st: &HashMap<String, String>) -> std::io::Result<()> {
    std::fs::write(
        path,
        serde_json::to_string_pretty(st).unwrap_or_default() + "\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    type Handler = Box<dyn FnMut(&str, usize) -> (u16, Vec<(String, String)>, String) + Send>;

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

    /// Returns (base url, raw request urls seen).
    fn serve(mut h: Handler) -> (String, Arc<Mutex<Vec<String>>>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let base = format!("http://{}/oai", server.server_addr().to_ip().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s2 = seen.clone();
        std::thread::spawn(move || {
            for (n, req) in server.incoming_requests().enumerate() {
                let raw = req.url().to_string();
                s2.lock().unwrap().push(raw.clone());
                let (code, hdrs, body) = h(&pct_decode(&raw), n);
                let mut r = tiny_http::Response::from_string(body).with_status_code(code);
                for (k, v) in hdrs {
                    r = r.with_header(
                        tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap(),
                    );
                }
                let _ = req.respond(r);
            }
        });
        (base, seen)
    }

    fn env(body: &str) -> (u16, Vec<(String, String)>, String) {
        (
            200,
            vec![("Content-Type".into(), "text/xml".into())],
            format!(
                "<?xml version=\"1.0\"?><OAI-PMH xmlns=\"http://www.openarchives.org/OAI/2.0/\"><responseDate>2026-01-01T00:00:00Z</responseDate><request>x</request>{body}</OAI-PMH>"
            ),
        )
    }

    fn identify(gran: &str) -> (u16, Vec<(String, String)>, String) {
        env(&format!(
            "<Identify><repositoryName>R</repositoryName><granularity>{gran}</granularity><deletedRecord>persistent</deletedRecord></Identify>"
        ))
    }

    fn rec(id: &str, stamp: &str, title: &str) -> String {
        format!(
            "<record><header><identifier>{id}</identifier><datestamp>{stamp}</datestamp></header><metadata><oai_dc:dc xmlns:oai_dc=\"http://www.openarchives.org/OAI/2.0/oai_dc/\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>{title}</dc:title><dc:creator>Smith, Jane</dc:creator><dc:date>2019-05-03</dc:date><dc:identifier>https://example.org/{id}</dc:identifier></oai_dc:dc></metadata></record>"
        )
    }

    fn cfg(base: &str) -> Config {
        Config {
            base: base.into(),
            fake_clock: true,
            ..Default::default()
        }
    }

    #[test]
    fn follows_resumption_tokens_with_only_the_token() {
        let (base, seen) = serve(Box::new(|u, _| {
            if u.contains("verb=Identify") {
                identify("YYYY-MM-DD")
            } else if u.contains("resumptionToken=T1") {
                env(&format!(
                    "<ListRecords>{}<resumptionToken completeListSize=\"3\" cursor=\"2\">T2</resumptionToken></ListRecords>",
                    rec("c", "2020-01-03", "Third")
                ))
            } else if u.contains("resumptionToken=T2") {
                env("<ListRecords><resumptionToken/></ListRecords>")
            } else {
                env(&format!(
                    "<ListRecords>{}{}<resumptionToken>T1</resumptionToken></ListRecords>",
                    rec("a", "2020-01-01", "First"),
                    rec("b", "2020-01-02", "Second")
                ))
            }
        }));
        let o = harvest(&cfg(&base), &Known::default()).unwrap();
        assert_eq!(o.entries.len(), 3);
        assert!(o.complete);
        assert_eq!(o.newest.as_deref(), Some("2020-01-03"));
        let urls = seen.lock().unwrap().clone();
        assert!(urls[1].contains("metadataPrefix=oai_dc"));
        assert!(
            urls[2].ends_with("verb=ListRecords&resumptionToken=T1"),
            "{}",
            urls[2]
        );
        assert_eq!(o.list_size, Some(3));
        let keys: Vec<&str> = o.entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["smith2019", "smith2019a", "smith2019b"]);
    }

    #[test]
    fn deleted_records_are_listed_not_imported() {
        let (base, _) = serve(Box::new(|u, _| {
            if u.contains("Identify") {
                identify("YYYY-MM-DD")
            } else {
                env(&format!(
                    "<ListRecords><record><header status=\"deleted\"><identifier>oai:x:9</identifier><datestamp>2021-02-02</datestamp></header></record>{}</ListRecords>",
                    rec("a", "2020-01-01", "Kept")
                ))
            }
        }));
        let o = harvest(&cfg(&base), &Known::default()).unwrap();
        assert_eq!(o.deleted, vec!["oai:x:9"]);
        assert_eq!(o.entries.len(), 1);
        assert_eq!(o.newest.as_deref(), Some("2021-02-02"));
    }

    #[test]
    fn no_records_match_is_an_empty_success() {
        let (base, _) = serve(Box::new(|u, _| {
            if u.contains("Identify") {
                identify("YYYY-MM-DD")
            } else {
                env("<error code=\"noRecordsMatch\">none</error>")
            }
        }));
        let o = harvest(&cfg(&base), &Known::default()).unwrap();
        assert!(o.entries.is_empty() && o.complete);
    }

    #[test]
    fn bad_token_restarts_from_the_newest_stamp_without_duplicates() {
        let (base, seen) = serve(Box::new(|u, _| {
            if u.contains("Identify") {
                identify("YYYY-MM-DD")
            } else if u.contains("resumptionToken=") {
                env("<error code=\"badResumptionToken\">expired</error>")
            } else if u.contains("from=2020-01-02") {
                env(&format!(
                    "<ListRecords>{}{}</ListRecords>",
                    rec("b", "2020-01-02", "Second"),
                    rec("c", "2020-01-03", "Third")
                ))
            } else {
                env(&format!(
                    "<ListRecords>{}{}<resumptionToken>T1</resumptionToken></ListRecords>",
                    rec("a", "2020-01-01", "First"),
                    rec("b", "2020-01-02", "Second")
                ))
            }
        }));
        let o = harvest(&cfg(&base), &Known::default()).unwrap();
        let titles: Vec<&str> = o.entries.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["First", "Second", "Third"]);
        assert!(
            seen.lock()
                .unwrap()
                .iter()
                .any(|u| u.contains("from=2020-01-02"))
        );
        assert!(o.complete);
    }

    #[test]
    fn unavailable_waits_for_retry_after() {
        let (base, _) = serve(Box::new(|u, n| {
            if n == 0 {
                (503, vec![("Retry-After".into(), "2".into())], String::new())
            } else if u.contains("Identify") {
                identify("YYYY-MM-DD")
            } else {
                env("<error code=\"noRecordsMatch\"/>")
            }
        }));
        let mut http_probe = Http {
            clock: Clock::new(true),
            last: None,
            ua: String::new(),
            requests: 0,
        };
        assert!(
            http_probe
                .fetch(&url_for(&base, &[("verb", "Identify")]))
                .is_ok()
        );
        assert!(http_probe.clock.now() >= Duration::from_secs(2));
    }

    #[test]
    fn dublin_core_becomes_a_thesis_entry() {
        let dc = xmlmini::parse("<dc xmlns:dc=\"d\"><dc:title>Teza mea</dc:title><dc:creator>Pop, Ion</dc:creator><dc:creator>Ene, Ana</dc:creator><dc:date>2019-05-03</dc:date><dc:identifier>https://doi.org/10.1000/xyz</dc:identifier><dc:identifier>oai:repo:1</dc:identifier><dc:type>Doctoral thesis</dc:type><dc:publisher>Universitatea X</dc:publisher><dc:rights>CC BY</dc:rights></dc>").unwrap();
        let e = dc_entry(&dc).unwrap();
        assert_eq!(e.to_csl()["type"], "thesis");
        assert_eq!(e.genre, Some(kinds::Genre::Doctoral));
        assert_eq!((e.doi.as_str(), e.year.as_str()), ("10.1000/xyz", "2019"));
        assert_eq!(e.author, "Pop, Ion and Ene, Ana");
        assert!(e.url.is_empty(), "a DOI link is not also the url");
        assert_eq!(e.container, "Universitatea X");
        assert!(e.note.contains("rights: CC BY"));
        assert!(e.to_bibtex().starts_with("@phdthesis{"));
        let empty = xmlmini::parse("<dc><x>1</x></dc>").unwrap();
        assert!(dc_entry(&empty).is_none());
        let idonly =
            xmlmini::parse("<dc><identifier>ISBN 978-92-95055-12-4</identifier></dc>").unwrap();
        assert_eq!(dc_entry(&idonly).unwrap().isbn, "9789295055124");
    }

    #[test]
    fn dates_are_checked_before_sending() {
        let (base, seen) = serve(Box::new(|_, _| identify("YYYY-MM-DD")));
        let c = Config {
            from: Some("2020-01-01".into()),
            until: Some("2020-06-30T00:00:00Z".into()),
            ..cfg(&base)
        };
        let err = harvest(&c, &Known::default()).unwrap_err();
        assert!(err.contains("granularity"), "{err}");
        assert!(seen.lock().unwrap().is_empty());
        let bad = Config {
            from: Some("2020/01/01".into()),
            ..cfg(&base)
        };
        assert!(harvest(&bad, &Known::default()).is_err());
        let inv = Config {
            from: Some("2021-01-01".into()),
            until: Some("2020-01-01".into()),
            ..cfg(&base)
        };
        assert!(
            harvest(&inv, &Known::default())
                .unwrap_err()
                .contains("after")
        );
        // finer than the archive supports: refused after Identify, before ListRecords
        let fine = Config {
            from: Some("2020-01-01T00:00:00Z".into()),
            ..cfg(&base)
        };
        assert!(
            harvest(&fine, &Known::default())
                .unwrap_err()
                .contains("day granularity")
        );
        assert_eq!(seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn reserved_characters_in_arguments_are_percent_encoded() {
        let (base, seen) = serve(Box::new(|u, _| {
            if u.contains("Identify") {
                identify("YYYY-MM-DD")
            } else {
                env("<error code=\"noRecordsMatch\"/>")
            }
        }));
        let c = Config {
            set: Some("theses:2019/a&b=c?".into()),
            ..cfg(&base)
        };
        harvest(&c, &Known::default()).unwrap();
        let u = seen.lock().unwrap()[1].clone();
        assert!(u.contains("set=theses%3A2019%2Fa%26b%3Dc%3F"), "{u}");
        assert!(u.contains("verb=ListRecords&metadataPrefix=oai_dc"), "{u}");
    }

    #[test]
    fn errors_are_reported_and_caps_apply() {
        let (b1, _) = serve(Box::new(|u, _| {
            if u.contains("Identify") {
                identify("YYYY-MM-DD")
            } else if u.contains("ListMetadataFormats") {
                env(
                    "<ListMetadataFormats><metadataFormat><metadataPrefix>oai_dc</metadataPrefix></metadataFormat><metadataFormat><metadataPrefix>mods</metadataPrefix></metadataFormat></ListMetadataFormats>",
                )
            } else {
                env("<error code=\"cannotDisseminateFormat\">no</error>")
            }
        }));
        let e = harvest(
            &Config {
                prefix: "marc".into(),
                ..cfg(&b1)
            },
            &Known::default(),
        )
        .unwrap_err();
        assert!(e.contains("marc") && e.contains("oai_dc, mods"), "{e}");
        let (b2, _) = serve(Box::new(|_, _| (200, vec![], "<html>nope</html>".into())));
        assert!(
            harvest(&cfg(&b2), &Known::default())
                .unwrap_err()
                .contains("not an OAI-PMH")
        );
        let (b3, _) = serve(Box::new(|u, _| {
            if u.contains("Identify") {
                identify("YYYY-MM-DD")
            } else {
                env(&format!(
                    "<ListRecords>{}{}{}</ListRecords>",
                    rec("a", "2020-01-01", "A"),
                    rec("b", "2020-01-02", "B"),
                    rec("c", "2020-01-03", "C")
                ))
            }
        }));
        let o = harvest(
            &Config {
                max_records: 2,
                ..cfg(&b3)
            },
            &Known::default(),
        )
        .unwrap();
        assert_eq!(o.entries.len(), 2);
        assert!(!o.complete);
        let err = harvest(&cfg("ftp://x"), &Known::default()).unwrap_err();
        assert!(err.contains("http"));
    }

    #[test]
    fn known_dois_are_not_duplicated_and_state_roundtrips() {
        let dc = |doi: &str| {
            format!(
                "<record><header><identifier>i{doi}</identifier><datestamp>2020-01-01</datestamp></header><metadata><dc><title>T{doi}</title><identifier>10.1000/{doi}</identifier></dc></metadata></record>"
            )
        };
        let (base, _) = serve(Box::new(move |u, _| {
            if u.contains("Identify") {
                identify("YYYY-MM-DD")
            } else {
                env(&format!(
                    "<ListRecords>{}{}</ListRecords>",
                    dc("aaa"),
                    dc("bbb")
                ))
            }
        }));
        let mut k = Known::default();
        k.dois.insert("10.1000/aaa".into());
        let o = harvest(&cfg(&base), &k).unwrap();
        assert_eq!((o.entries.len(), o.skipped_known), (1, 1));
        let p = std::env::temp_dir().join(format!("tezaur-oai-state-{}.json", std::process::id()));
        let mut st = load_state(&p);
        assert!(st.is_empty());
        st.insert(state_key(&base, &None), "2020-01-01".into());
        save_state(&p, &st).unwrap();
        assert_eq!(
            load_state(&p)
                .get(&state_key(&base, &None))
                .map(String::as_str),
            Some("2020-01-01")
        );
        let _ = std::fs::remove_file(p);
    }
}
