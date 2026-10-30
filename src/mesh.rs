//! TEZAUR Mesh: independent TEZAUR nodes (CLI servers, desktop apps, editors that talk to a server) exchange
//! research material without any central service.
//!
//! * **Federated search** – a query is sent to peers, only matching snippets come back; nothing is copied.
//! * **Pull** – a peer's *shared* documents are copied into your index under `mesh/<peer>/…` (extracted text +
//!   page numbers, no embeddings; they are re-embedded locally). Content-addressed: a document whose hash you
//!   already hold is never stored twice, however many peers offer it.
//! * **Bundle** (`.tzb`, a zip) – the same exchange by USB stick / e-mail / AirDrop, fully offline.
//! * **LAN discovery** – a UDP beacon answers "who is there?" on the local network, no configuration.
//!
//! Sharing is opt-in: nothing is offered until you list patterns in `mesh.json` (`tezaur mesh share`).
//! Documents received from others are re-offered only if `relay` is true.

use crate::bib::BibMap;
use crate::index::{Chunk, Doc, Searcher, Store, coverage};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DISCOVERY_PORT: u16 = 8788;
const PROBE: &[u8] = b"TEZAUR-MESH?";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Peer {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub token: Option<String>,
    /// Optional HTTP/SOCKS proxy (e.g. `socks5://127.0.0.1:9050` for a Tor onion peer).
    #[serde(default)]
    pub proxy: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Cfg {
    pub id: String,
    pub name: String,
    /// Patterns (case-insensitive substrings of path/title, or `*`). Empty = share nothing.
    #[serde(default)]
    pub share: Vec<String>,
    /// Offer documents that came from other peers too.
    #[serde(default)]
    pub relay: bool,
    #[serde(default)]
    pub peers: Vec<Peer>,
}

pub fn cfg_path(base: &Path) -> PathBuf {
    base.join("mesh.json")
}

fn random_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    );
    h.write_u32(std::process::id());
    format!("{:016x}", h.finish())
}

pub fn load_cfg(base: &Path) -> Cfg {
    let mut c: Cfg = std::fs::read(cfg_path(base))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    if c.id.is_empty() {
        c.id = random_id();
    }
    if c.name.is_empty() {
        c.name = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| format!("node-{}", &c.id[..6]));
    }
    c
}

pub fn save_cfg(base: &Path, c: &Cfg) -> Result<()> {
    std::fs::create_dir_all(base)?;
    std::fs::write(cfg_path(base), serde_json::to_vec_pretty(c)?)?;
    Ok(())
}

pub fn safe_origin(s: &str) -> String {
    let v: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .take(40)
        .collect();
    if v.is_empty() { "peer".into() } else { v }
}

pub fn is_shared(c: &Cfg, d: &Doc) -> bool {
    if d.path.starts_with("mesh/") && !c.relay {
        return false;
    }
    let (p, t) = (d.path.to_lowercase(), d.title.to_lowercase());
    c.share.iter().any(|s| {
        let s = s.trim().to_lowercase();
        s == "*" || (!s.is_empty() && (p.contains(&s) || t.contains(&s)))
    })
}

pub fn shared_docs(c: &Cfg, store: &Store) -> Vec<usize> {
    store
        .docs
        .iter()
        .enumerate()
        .filter(|(_, d)| is_shared(c, d))
        .map(|(i, _)| i)
        .collect()
}

pub fn manifest(c: &Cfg, store: &Store) -> Value {
    let docs: Vec<Value> = shared_docs(c, store)
        .into_iter()
        .map(|i| {
            let d = &store.docs[i];
            json!({"path": d.path, "title": d.title, "hash": d.hash.to_string(), "pages": d.pages,
                   "chunks": store.chunks.iter().filter(|x| x.doc == i).count()})
        })
        .collect();
    json!({"format": "tezaur-mesh/1", "node": {"id": c.id, "name": c.name}, "docs": docs})
}

pub fn doc_payload(store: &Store, i: usize) -> Value {
    let d = &store.docs[i];
    let chunks: Vec<Value> = store
        .chunks
        .iter()
        .filter(|c| c.doc == i)
        .map(|c| json!({"page": c.page, "text": c.text}))
        .collect();
    json!({"path": d.path, "title": d.title, "hash": d.hash.to_string(), "pages": d.pages, "chunks": chunks})
}

/// Insert a received document. Returns false when an identical document (same hash) is already held.
pub fn merge_doc(store: &mut Store, origin: &str, p: &Value) -> Result<bool> {
    let hash: u64 = p["hash"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| p["hash"].as_u64())
        .context("document without hash")?;
    let path = p["path"].as_str().context("document without path")?;
    let title = p["title"].as_str().unwrap_or(path);
    let chunks = p["chunks"].as_array().context("document without chunks")?;
    if chunks.is_empty() {
        return Ok(false);
    }
    if store.docs.iter().any(|d| d.hash == hash) {
        return Ok(false);
    }
    // Received paths are namespaced and sanitised: a peer must not be able to name a path outside mesh/.
    let rel: Vec<String> = path
        .split(['/', '\\'])
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .map(|s| s.to_string())
        .collect();
    let origin = safe_origin(origin);
    let local = if rel.first().map(|s| s == "mesh").unwrap_or(false) {
        rel.join("/")
    } else {
        format!("mesh/{origin}/{}", rel.join("/"))
    };
    if let Some(old) = store.doc_by_path(&local) {
        store.remove_doc(old);
    }
    let di = store.docs.len();
    store.docs.push(Doc {
        path: local,
        title: format!("{title} [via {origin}]"),
        hash,
        pages: p["pages"].as_u64().unwrap_or(0) as usize,
    });
    for c in chunks {
        let text = c["text"].as_str().unwrap_or("").to_string();
        if !text.is_empty() {
            store.chunks.push(Chunk {
                doc: di,
                page: c["page"].as_u64().unwrap_or(0) as usize,
                text,
                emb: None,
            });
        }
    }
    Ok(true)
}

// ------------------------------------------------------------------ bundles (offline exchange)

pub fn bundle_export(c: &Cfg, store: &Store, bib: &BibMap, all: bool) -> Result<Vec<u8>> {
    let idx: Vec<usize> = if all {
        (0..store.docs.len()).collect()
    } else {
        shared_docs(c, store)
    };
    let mut buf = Vec::new();
    {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let o = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut refs = String::new();
        for (n, &i) in idx.iter().enumerate() {
            z.start_file(format!("docs/{n:05}.json"), o)?;
            z.write_all(doc_payload(store, i).to_string().as_bytes())?;
            if let Some(e) = bib.find(&store.docs[i].path) {
                let mut e = e.clone();
                if let Some(stem) = Path::new(&store.docs[i].path).file_stem() {
                    e.key = stem.to_string_lossy().to_string(); // so the receiving side matches it by file name
                }
                refs.push_str(&e.to_bibtex());
                refs.push('\n');
            }
        }
        if !refs.is_empty() {
            z.start_file("refs.bib", o)?;
            z.write_all(refs.as_bytes())?;
        }
        z.start_file("manifest.json", o)?;
        z.write_all(json!({"format": "tezaur-bundle/1", "node": {"id": c.id, "name": c.name}, "docs": idx.len()}).to_string().as_bytes())?;
        z.finish()?;
    }
    Ok(buf)
}

/// Returns (added, skipped, origin name).
pub fn bundle_import(
    store: &mut Store,
    base: &Path,
    data: &[u8],
) -> Result<(usize, usize, String)> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(data))
        .context("not a TEZAUR bundle (zip expected)")?;
    let man: Value = {
        let mut s = String::new();
        crate::ingest::read_capped(
            &mut z
                .by_name("manifest.json")
                .context("no manifest.json — not a TEZAUR bundle")?,
            &mut s,
        )?;
        serde_json::from_str(&s)?
    };
    if man["format"] != "tezaur-bundle/1" {
        bail!("unknown bundle format {}", man["format"]);
    }
    let origin = safe_origin(man["node"]["name"].as_str().unwrap_or("bundle"));
    let names: Vec<String> = z
        .file_names()
        .flatten()
        .map(|n| n.to_string())
        .filter(|n| n.starts_with("docs/") && n.ends_with(".json"))
        .collect();
    let (mut added, mut skipped) = (0, 0);
    for n in names {
        let mut s = String::new();
        crate::ingest::read_capped(&mut z.by_name(&n)?, &mut s)?;
        match merge_doc(store, &origin, &serde_json::from_str(&s)?) {
            Ok(true) => added += 1,
            Ok(false) => skipped += 1,
            Err(e) => eprintln!("warning: {n}: {e}"),
        }
    }
    if let Ok(mut f) = z.by_name("refs.bib") {
        let mut s = String::new();
        crate::ingest::read_capped(&mut f, &mut s)?;
        std::fs::write(base.join(format!("mesh-{origin}.bib")), s)?;
    }
    Ok((added, skipped, origin))
}

// ------------------------------------------------------------------ peers over HTTP

fn agent(p: &Peer) -> ureq::Agent {
    let mut b = ureq::AgentBuilder::new().timeout(Duration::from_secs(25));
    if let Some(px) = p.proxy.as_deref().filter(|x| !x.is_empty()) {
        match ureq::Proxy::new(px) {
            Ok(px) => b = b.proxy(px),
            Err(e) => eprintln!("warning: peer {}: bad proxy {px}: {e}", p.name),
        }
    }
    b.build()
}

fn call(p: &Peer, path: &str) -> Result<ureq::Response> {
    let url = format!("{}{}", p.url.trim_end_matches('/'), path);
    let mut r = agent(p).get(&url);
    if let Some(t) = &p.token {
        r = r.set("Authorization", &format!("Bearer {t}"));
    }
    r.call().map_err(|e| match e {
        ureq::Error::Status(401, _) => anyhow::anyhow!("peer {} refused the token", p.name),
        ureq::Error::Status(c, r) => anyhow::anyhow!(
            "peer {} answered {c}: {}",
            p.name,
            r.into_string().unwrap_or_default()
        ),
        e => anyhow::anyhow!("cannot reach peer {} at {}: {e}", p.name, p.url),
    })
}

/// A peer is untrusted: never read more than 64 MB of its answer.
fn json_capped(r: ureq::Response) -> Result<Value> {
    let mut b = Vec::new();
    std::io::Read::take(r.into_reader(), (64 << 20) + 1).read_to_end(&mut b)?;
    if b.len() > 64 << 20 {
        bail!("peer answer larger than 64 MB — refusing");
    }
    Ok(serde_json::from_slice(&b)?)
}

pub fn pct(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

/// Copy a peer's shared documents into `store`. Returns (added, already_held).
pub fn pull(store: &mut Store, p: &Peer) -> Result<(usize, usize)> {
    let man: Value = json_capped(call(p, "/api/mesh/manifest")?)?;
    let origin = man["node"]["name"].as_str().unwrap_or(&p.name).to_string();
    let (mut added, mut held) = (0, 0);
    for d in man["docs"].as_array().context("bad manifest")? {
        let hash: Option<u64> = d["hash"].as_str().and_then(|s| s.parse().ok());
        if hash.is_some_and(|h| store.docs.iter().any(|x| x.hash == h)) {
            held += 1;
            continue;
        }
        let path = d["path"].as_str().unwrap_or("");
        let payload: Value = json_capped(call(p, &format!("/api/mesh/doc?path={}", pct(path)))?)?;
        if merge_doc(store, &origin, &payload)? {
            added += 1
        } else {
            held += 1
        }
    }
    Ok((added, held))
}

/// Ask every peer; returns hits tagged with the peer's name, merged by relevance score.
pub fn federated_search(peers: &[Peer], q: &str, k: usize) -> (Vec<Value>, Vec<String>) {
    let mut all = Vec::new();
    let mut errs = Vec::new();
    let handles: Vec<_> = peers
        .iter()
        .cloned()
        .map(|p| {
            let q = q.to_string();
            std::thread::spawn(move || -> Result<Vec<Value>> {
                let v: Value =
                    json_capped(call(&p, &format!("/api/mesh/search?q={}&k={k}", pct(&q)))?)?;
                let mut hits = v["hits"].as_array().cloned().unwrap_or_default();
                for h in &mut hits {
                    h["peer"] = json!(p.name);
                }
                Ok(hits)
            })
        })
        .collect();
    for h in handles {
        match h.join() {
            Ok(Ok(v)) => all.extend(v),
            Ok(Err(e)) => errs.push(format!("{e:#}")),
            Err(_) => errs.push("peer thread panicked".into()),
        }
    }
    all.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .partial_cmp(&a["score"].as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all.truncate(k);
    (all, errs)
}

/// Search only shared documents (what a peer is allowed to see).
pub fn shared_search(c: &Cfg, store: &Store, bib: &BibMap, q: &str, k: usize) -> Vec<Value> {
    let allow: std::collections::HashSet<usize> = shared_docs(c, store).into_iter().collect();
    if allow.is_empty() {
        return vec![];
    }
    let hits = Searcher::new(store).search(q, None, k, 3, Some(&allow));
    hits.iter()
        .map(|h| {
            let ch = &store.chunks[h.chunk];
            let d = &store.docs[ch.doc];
            json!({"label": crate::prompt::label(store, bib, h), "text": ch.text, "page": ch.page, "path": d.path,
                   "title": d.title, "cov": coverage(q, &ch.text), "score": h.score})
        })
        .collect()
}

// ------------------------------------------------------------------ LAN discovery (UDP)

/// Answer "TEZAUR-MESH?" probes on the local network with name and HTTP port. Runs until the process ends.
pub fn start_beacon(c: &Cfg, http_port: u16, udp_port: u16) {
    let sock = match std::net::UdpSocket::bind(("0.0.0.0", udp_port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mesh: discovery beacon not started ({e})");
            return;
        }
    };
    let reply = json!({"tezaur": 1, "id": c.id, "name": c.name, "port": http_port}).to_string();
    std::thread::spawn(move || {
        let mut buf = [0u8; 64];
        while let Ok((n, from)) = sock.recv_from(&mut buf) {
            if &buf[..n] == PROBE {
                let _ = sock.send_to(reply.as_bytes(), from);
            }
        }
    });
}

/// Broadcast a probe and collect answers for `wait`. `target` defaults to the LAN broadcast address.
pub fn discover(target: &str, wait: Duration) -> Result<Vec<Value>> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0")?;
    sock.set_broadcast(true)?;
    sock.set_read_timeout(Some(Duration::from_millis(250)))?;
    sock.send_to(PROBE, target)
        .with_context(|| format!("sending to {target}"))?;
    let end = std::time::Instant::now() + wait;
    let mut found: Vec<Value> = Vec::new();
    let mut buf = [0u8; 1024];
    while std::time::Instant::now() < end {
        if let Ok((n, from)) = sock.recv_from(&mut buf)
            && let Ok(mut v) = serde_json::from_slice::<Value>(&buf[..n])
            && v["tezaur"] == 1
            && !found.iter().any(|f| f["id"] == v["id"])
        {
            v["url"] = json!(format!(
                "http://{}:{}",
                from.ip(),
                v["port"].as_u64().unwrap_or(8787)
            ));
            found.push(v);
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with(path: &str, text: &str, hash: u64) -> Store {
        let mut s = Store::default();
        s.docs.push(Doc {
            path: path.into(),
            title: "T".into(),
            hash,
            pages: 1,
        });
        s.chunks.push(Chunk {
            doc: 0,
            page: 1,
            text: text.into(),
            emb: None,
        });
        s
    }

    #[test]
    fn sharing_is_opt_in_and_relay_is_off_by_default() {
        let mut c = Cfg::default();
        let d = Doc {
            path: "Library/Smith.pdf".into(),
            title: "On proof".into(),
            hash: 1,
            pages: 1,
        };
        assert!(!is_shared(&c, &d));
        c.share = vec!["smith".into()];
        assert!(is_shared(&c, &d));
        let m = Doc {
            path: "mesh/ana/Smith.pdf".into(),
            ..d.clone()
        };
        assert!(!is_shared(&c, &m));
        c.relay = true;
        assert!(is_shared(&c, &m));
    }

    #[test]
    fn bundle_roundtrip_dedupes_by_hash_and_sanitises_paths() {
        let a = store_with(
            "Library/a.txt",
            "the burden of proof lies on the claimant",
            42,
        );
        let c = Cfg {
            id: "x".into(),
            name: "ana".into(),
            share: vec!["*".into()],
            ..Default::default()
        };
        let bytes = bundle_export(&c, &a, &BibMap::default(), false).unwrap();
        let mut b = Store::default();
        let dir = std::env::temp_dir().join("tezaur-mesh-test");
        std::fs::create_dir_all(&dir).unwrap();
        let (added, skipped, origin) = bundle_import(&mut b, &dir, &bytes).unwrap();
        assert_eq!((added, skipped, origin.as_str()), (1, 0, "ana"));
        assert_eq!(b.docs[0].path, "mesh/ana/Library/a.txt");
        assert!(b.docs[0].title.contains("via ana"));
        let (added, skipped, _) = bundle_import(&mut b, &dir, &bytes).unwrap();
        assert_eq!((added, skipped), (0, 1)); // same hash: not stored twice
        let evil = json!({"path": "../../etc/passwd", "title": "x", "hash": "7", "pages": 0, "chunks": [{"page": 0, "text": "hi"}]});
        merge_doc(&mut b, "bob/../x", &evil).unwrap();
        let lp = &b.docs.last().unwrap().path;
        assert!(
            lp.starts_with("mesh/bob_.._x/") && lp.split('/').all(|c| c != ".."),
            "{lp}"
        );
    }

    #[test]
    fn beacon_answers_probe() {
        let c = Cfg {
            id: "abc".into(),
            name: "n1".into(),
            ..Default::default()
        };
        start_beacon(&c, 9999, 38788);
        std::thread::sleep(Duration::from_millis(200));
        let f = discover("127.0.0.1:38788", Duration::from_millis(800)).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0]["name"], "n1");
        assert_eq!(f[0]["url"], "http://127.0.0.1:9999");
    }
}
