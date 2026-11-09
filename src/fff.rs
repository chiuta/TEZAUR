//! Plic FFF (`.fff.json`) — veriga din lanțul de surse „fără frontiere”:
//! LegatorOCR (`document/ocr`) → TEZAUR → VERIGA → AcademiSearch (`citation/csl`).
//! Specificația plicului: docs/FFF-ENVELOPE.md din hubul chiuta.github.io. Totul local, fără rețea.
//!
//! * intrare: `tezaur ingest --fff-in FILE.fff.json` — `document/ocr` (text pe pagini, SHA-256 al scanării),
//!   `text/plain` sau `text/markdown`; plicul e copiat în `<index>/fff-inbox/`, se scrie o intrare BibTeX
//!   `fff-<cheie>.bib` (autor, an, titlu, amprentă) și documentul e indexat cu paginile lui;
//! * ieșire: `tezaur bib --fff-out FILE.fff.json` (sau `--to fff`) — bibliografia ca plic `citation/csl`.

use crate::bib::Entry;
use crate::sha256::sha256_hex;
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub fn is_fff_path(p: &Path) -> bool {
    p.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_lowercase().ends_with(".fff.json"))
        .unwrap_or(false)
}

#[derive(Debug, Default, Clone)]
pub struct FffDoc {
    pub title: String,
    pub author: String,
    pub year: String,
    pub lang: String,
    pub file: String,
    pub sha256: String,
    pub app: String,
    /// (număr pagină 1-based, text); 0 = fără pagini
    pub pages: Vec<(usize, String)>,
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").trim().to_string()
}

/// Citește un plic (forma canonică `fff:2` cu `payload`, sau compat `fff:1` cu `data`).
pub fn read(path: &Path) -> Result<FffDoc> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse(&raw)
}

pub fn parse(raw: &str) -> Result<FffDoc> {
    let v: Value = serde_json::from_str(raw).context("not valid JSON")?;
    let ver = v.get("fff").and_then(|x| x.as_i64()).unwrap_or(0);
    let payload = match ver {
        2 => v.get("payload"),
        1 => v.get("data"),
        _ => None,
    }
    .context("not an FFF envelope (missing fff/payload)")?;
    let typ = s(&v, "type");
    let mut d = FffDoc {
        title: s(&v, "title"),
        lang: s(&v, "lang"),
        app: s(&v, "app"),
        ..Default::default()
    };
    if let Some(src) = v.get("source")
        && let Some(id) = src.get("id").and_then(|x| x.as_str())
    {
        d.sha256 = id.trim_start_matches("sha256:").to_string();
    }
    match typ.as_str() {
        "document/ocr" => {
            if d.title.is_empty() {
                d.title = s(payload, "title");
            }
            d.author = s(payload, "author");
            d.year = s(payload, "year");
            d.file = s(payload, "file");
            if !s(payload, "sha256").is_empty() {
                d.sha256 = s(payload, "sha256");
            }
            if d.lang.is_empty() {
                d.lang = s(payload, "lang");
            }
            for p in payload.get("pages").and_then(|x| x.as_array()).context("document/ocr without pages")? {
                let n = p.get("n").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                let t = s(p, "text");
                if !t.is_empty() {
                    d.pages.push((n.max(1), t));
                }
            }
        }
        "text/plain" | "text/markdown" => {
            let t = payload.as_str().context("text envelope with a non-text payload")?;
            d.pages.push((0, t.to_string()));
        }
        other => bail!("envelope type {other} is not accepted (document/ocr, text/plain, text/markdown)"),
    }
    if d.pages.is_empty() {
        bail!("the envelope has no text");
    }
    if d.sha256.is_empty() {
        let all: String = d.pages.iter().map(|p| p.1.as_str()).collect::<Vec<_>>().join("\n");
        d.sha256 = sha256_hex(all.as_bytes());
    }
    if d.title.is_empty() {
        d.title = "untitled".into();
    }
    Ok(d)
}

fn slug(title: &str, sha: &str) -> String {
    let mut s = String::new();
    for c in title.chars().flat_map(|c| c.to_lowercase()) {
        let c = match c {
            'ă' | 'â' => 'a',
            'î' => 'i',
            'ș' | 'ş' => 's',
            'ț' | 'ţ' => 't',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if c == '-' && (s.is_empty() || s.ends_with('-')) {
            continue;
        }
        s.push(c);
        if s.len() >= 40 {
            break;
        }
    }
    let s = s.trim_end_matches('-');
    format!("{}-{}", if s.is_empty() { "doc" } else { s }, &sha[..sha.len().min(8)])
}

fn bib_escape(t: &str) -> String {
    t.replace(['{', '}'], "")
}

/// Copiază plicul în `<base>/fff-inbox/<cheie>.fff.json` și scrie `<base>/fff-<cheie>.bib`.
/// Întoarce calea copiei din inbox (de indexat) și cheia de citare.
pub fn import(base: &Path, src: &Path) -> Result<(PathBuf, String)> {
    let doc = read(src)?;
    let key = slug(&doc.title, &doc.sha256);
    let inbox = base.join("fff-inbox");
    std::fs::create_dir_all(&inbox)?;
    let dest = inbox.join(format!("{key}.fff.json"));
    std::fs::copy(src, &dest)?;
    let mut b = format!("@misc{{{key},\n  title = {{{}}},\n", bib_escape(&doc.title));
    if !doc.author.is_empty() {
        b.push_str(&format!("  author = {{{}}},\n", bib_escape(&doc.author)));
    }
    if !doc.year.is_empty() {
        b.push_str(&format!("  year = {{{}}},\n", bib_escape(&doc.year)));
    }
    let via = if doc.app.is_empty() { "FFF envelope".to_string() } else { format!("FFF envelope from {}", doc.app) };
    let file = if doc.file.is_empty() { String::new() } else { format!(" ({})", doc.file) };
    b.push_str(&format!(
        "  note = {{sha256:{}{}; {}; lang {}}},\n  file = {{{key}.fff.json}},\n}}\n",
        doc.sha256,
        file,
        via,
        if doc.lang.is_empty() { "?" } else { &doc.lang }
    ));
    std::fs::write(base.join(format!("fff-{key}.bib")), b)?;
    Ok((dest, key))
}

fn iso_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.000Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Bibliografia ca plic `citation/csl` (forma canonică `fff:2`).
pub fn csl_envelope(entries: &[Entry]) -> String {
    let items: Vec<Value> = entries.iter().map(|e| e.to_csl()).collect();
    let env = json!({
        "fff": 2,
        "type": "citation/csl",
        "title": "TEZAUR",
        "payload": items,
        "source": {"label": "TEZAUR · bibliografie"},
        "created": iso_now(),
        "app": "TEZAUR",
    });
    serde_json::to_string_pretty(&env).unwrap_or_default() + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENV: &str = r#"{"fff":2,"type":"document/ocr","title":"Raport","payload":{"title":"Raport","author":"Ana Popescu","year":"2001","lang":"ro","file":"s.png","sha256":"abcdef0123456789","pages":[{"n":1,"text":"Primul text."},{"n":2,"text":"Al doilea text."}]},"source":{"id":"sha256:abcdef0123456789"},"created":"2026-11-03T00:00:00.000Z","app":"LegatorOCR"}"#;

    #[test]
    fn reads_document_ocr_with_pages() {
        let d = parse(ENV).unwrap();
        assert_eq!(d.title, "Raport");
        assert_eq!(d.pages.len(), 2);
        assert_eq!(d.pages[1].0, 2);
        assert_eq!(d.sha256, "abcdef0123456789");
        assert_eq!(d.author, "Ana Popescu");
    }

    #[test]
    fn reads_compat_text_and_rejects_others() {
        let c = r#"{"fff":1,"type":"text/plain","title":"Nota","data":"un text","src":"x","ts":"2026-01-01T00:00:00Z"}"#;
        let d = parse(c).unwrap();
        assert_eq!(d.pages, vec![(0, "un text".to_string())]);
        assert!(!d.sha256.is_empty());
        assert!(parse(r#"{"fff":2,"type":"table/csv","payload":"a,b","created":"x","app":"y"}"#).is_err());
        assert!(parse("{}").is_err());
    }

    #[test]
    fn import_writes_bib_and_inbox_copy() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("test-tmp").join("fff");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let src = root.join("in.fff.json");
        std::fs::write(&src, ENV).unwrap();
        let (dest, key) = import(&root, &src).unwrap();
        assert!(dest.is_file() && is_fff_path(&dest));
        let bib = std::fs::read_to_string(root.join(format!("fff-{key}.bib"))).unwrap();
        let e = crate::bib::parse(&bib);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].year, "2001");
        assert!(e[0].note.contains("sha256:abcdef"));
        let m = crate::bib::BibMap { entries: e, ..Default::default() };
        assert!(m.find(&format!("fff-inbox/{key}.fff.json")).is_some());
        let out = csl_envelope(&m.entries);
        assert!(out.contains("\"citation/csl\"") && out.contains("Popescu"));
    }
}
