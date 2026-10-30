//! Reading books and articles (pdf, docx, epub, txt, md, html, tex) into page-tagged chunks.

use crate::index::{Chunk, Doc, Store};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const CHUNK_WORDS: usize = 200;
pub const OVERLAP_WORDS: usize = 40;

pub fn is_supported(p: &Path) -> bool {
    matches!(
        p.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .as_deref(),
        Some("pdf" | "docx" | "epub" | "txt" | "md" | "markdown" | "html" | "htm" | "tex")
    )
}

pub fn collect_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in paths {
        if p.is_dir() {
            for e in WalkDir::new(p).follow_links(true).into_iter().flatten() {
                if e.file_type().is_file() && is_supported(e.path()) {
                    out.push(e.path().to_path_buf());
                }
            }
        } else if p.is_file() {
            out.push(p.clone());
        } else {
            eprintln!("skip (not found): {}", p.display());
        }
    }
    out.sort();
    out.dedup();
    out
}

pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn pdf_pages(path: &Path) -> Result<Vec<String>> {
    let p = path.to_path_buf();
    let r = std::panic::catch_unwind(move || pdf_extract::extract_text_by_pages(&p));
    match r {
        Ok(Ok(v)) => Ok(v),
        other => {
            // Fallback: poppler's pdftotext, if installed (pages split by form feed).
            let out = run_timeout(
                std::process::Command::new("pdftotext").arg(path).arg("-"),
                180,
            );
            if let Some(o) = out
                && o.status.success()
            {
                let t = String::from_utf8_lossy(&o.stdout).to_string();
                return Ok(t.split('\x0c').map(str::to_string).collect());
            }
            match other {
                Ok(Err(e)) => bail!("PDF parse failed: {e}"),
                _ => bail!("PDF parser crashed (and pdftotext not available)"),
            }
        }
    }
}

/// Languages for OCR: `TEZAUR_OCR_LANG`, else Romanian+English when tesseract has them, else English.
fn ocr_lang() -> String {
    if let Ok(l) = std::env::var("TEZAUR_OCR_LANG")
        && !l.trim().is_empty()
    {
        return l;
    }
    let have = std::process::Command::new("tesseract")
        .arg("--list-langs")
        .output()
        .ok();
    let list = have
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let pick: Vec<&str> = ["ron", "eng"]
        .into_iter()
        .filter(|l| list.lines().any(|x| x.trim() == *l))
        .collect();
    if pick.is_empty() {
        "eng".into()
    } else {
        pick.join("+")
    }
}

/// OCR for scanned PDFs: poppler's `pdftoppm` renders pages, `tesseract` reads them. `None` when the tools
/// are missing or OCR is switched off (`TEZAUR_OCR=off`), so the caller can explain instead of failing.
fn ocr_pdf(path: &Path) -> Option<Vec<String>> {
    if std::env::var("TEZAUR_OCR")
        .map(|v| v == "off")
        .unwrap_or(false)
    {
        return None;
    }
    let tmp = std::env::temp_dir().join(format!(
        "tezaur-ocr-{}-{}",
        std::process::id(),
        fnv1a(path.to_string_lossy().as_bytes())
    ));
    std::fs::create_dir_all(&tmp).ok()?;
    let done = (|| {
        let ok = std::process::Command::new("pdftoppm")
            .args(["-r", "200", "-gray", "-png"])
            .arg(path)
            .arg(tmp.join("p"))
            .status()
            .ok()?
            .success();
        if !ok {
            return None;
        }
        let mut imgs: Vec<PathBuf> = std::fs::read_dir(&tmp)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "png"))
            .collect();
        imgs.sort(); // pdftoppm zero-pads page numbers, so lexical order is page order
        let lang = ocr_lang();
        let mut pages = Vec::new();
        for (i, img) in imgs.iter().enumerate() {
            eprint!("\r  OCR page {}/{} ", i + 1, imgs.len());
            let o = std::process::Command::new("tesseract")
                .arg(img)
                .arg("stdout")
                .args(["-l", &lang])
                .output()
                .ok()?;
            if !o.status.success() {
                return None;
            }
            pages.push(String::from_utf8_lossy(&o.stdout).to_string());
        }
        eprintln!();
        Some(pages)
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    done
}

/// A PDF whose pages together hold almost no text is a scan.
fn looks_scanned(pages: &[String]) -> bool {
    let chars: usize = pages
        .iter()
        .map(|p| p.chars().filter(|c| !c.is_whitespace()).count())
        .sum();
    chars < 25 * pages.len().max(1)
}

fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let lower = s.to_ascii_lowercase(); // ASCII-only: keeps byte offsets identical to `s`
    let mut i = 0;
    let bytes = s.as_bytes();
    let mut in_tag = false;
    while i < bytes.len() {
        if !in_tag {
            for blk in ["<script", "<style"] {
                if lower[i..].starts_with(blk) {
                    let close = format!("</{}", &blk[1..]);
                    i = lower[i..]
                        .find(&close)
                        .map(|j| i + j)
                        .unwrap_or(bytes.len());
                }
            }
            if i >= bytes.len() {
                break;
            }
        }
        let c = s[i..].chars().next().unwrap();
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
        i += c.len_utf8();
    }
    decode_entities(&out)
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        match rest.find(';').filter(|&j| j <= 10) {
            Some(j) => {
                let ent = &rest[1..j];
                let rep: Option<String> = match ent {
                    "amp" => Some("&".into()),
                    "lt" => Some("<".into()),
                    "gt" => Some(">".into()),
                    "quot" => Some("\"".into()),
                    "apos" => Some("'".into()),
                    "nbsp" => Some(" ".into()),
                    e if e.starts_with("#x") || e.starts_with("#X") => {
                        u32::from_str_radix(&e[2..], 16)
                            .ok()
                            .and_then(char::from_u32)
                            .map(String::from)
                    }
                    e if e.starts_with('#') => e[1..]
                        .parse::<u32>()
                        .ok()
                        .and_then(char::from_u32)
                        .map(String::from),
                    _ => None,
                };
                match rep {
                    Some(r) => {
                        out.push_str(&r);
                        rest = &rest[j + 1..];
                    }
                    None => {
                        out.push('&');
                        rest = &rest[1..];
                    }
                }
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Largest text a single archive member may expand to: a hostile .docx/.epub/bundle must not be able to exhaust memory (zip bombs).
pub const MAX_MEMBER: u64 = 256 << 20;

pub fn read_capped(r: &mut impl std::io::Read, out: &mut String) -> Result<()> {
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut std::io::Read::take(r, MAX_MEMBER + 1), &mut buf)?;
    if buf.len() as u64 > MAX_MEMBER {
        bail!(
            "archive member expands to more than {} MB — refusing (possible zip bomb)",
            MAX_MEMBER >> 20
        );
    }
    out.push_str(
        &String::from_utf8(buf)
            .map_err(|_| anyhow::anyhow!("archive member is not valid UTF-8"))?,
    );
    Ok(())
}

/// Run an external converter with a deadline, so one pathological file cannot hang an ingest.
pub fn run_timeout(cmd: &mut std::process::Command, secs: u64) -> Option<std::process::Output> {
    use std::process::Stdio;
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut out = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = std::io::Read::read_to_end(&mut out, &mut b);
        b
    });
    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if start.elapsed().as_secs() >= secs => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
            Err(_) => return None,
        }
    };
    Some(std::process::Output {
        status,
        stdout: reader.join().unwrap_or_default(),
        stderr: Vec::new(),
    })
}

fn zip_text(path: &Path, want: impl Fn(&str) -> bool) -> Result<Vec<(String, String)>> {
    let f = std::fs::File::open(path)?;
    let mut z = zip::ZipArchive::new(f).context("not a valid zip container")?;
    let mut names: Vec<String> = z
        .file_names()
        .filter_map(|n| n.ok().map(|n| n.to_string()))
        .filter(|n| want(n))
        .collect();
    names.sort();
    let mut out = Vec::new();
    for n in names {
        let mut s = String::new();
        read_capped(&mut z.by_name(&n)?, &mut s)?;
        out.push((n, s));
    }
    Ok(out)
}

/// Word document -> plain text, one line per paragraph.
fn docx_text(path: &Path) -> Result<String> {
    let parts = zip_text(path, |n| n == "word/document.xml")?;
    let xml = parts
        .first()
        .map(|p| p.1.as_str())
        .context("word/document.xml missing")?;
    let xml = xml
        .replace("</w:p>", "\n")
        .replace("<w:tab/>", " ")
        .replace("<w:br/>", "\n");
    Ok(decode_entities(&strip_html(&xml))
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Value of `name="…"` (or single-quoted) inside one opening tag.
fn attr(tag: &str, name: &str) -> Option<String> {
    for q in ['"', '\''] {
        let pat = format!("{name}={q}");
        if let Some(i) = tag.find(&pat) {
            let rest = &tag[i + pat.len()..];
            if let Some(j) = rest.find(q) {
                return Some(rest[..j].to_string());
            }
        }
    }
    None
}

/// Text between `<name` and `>` for every opening tag called `name` (also `opf:name`).
fn open_tags<'a>(xml: &'a str, name: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    for open in [format!("<{name}"), format!("<opf:{name}")] {
        let mut rest = xml;
        while let Some(i) = rest.find(&open) {
            let after = &rest[i + open.len()..];
            if after.starts_with(|c: char| c.is_whitespace() || c == '/' || c == '>')
                && let Some(j) = after.find('>')
            {
                out.push(&after[..j]);
                rest = &after[j..];
                continue;
            }
            rest = after;
        }
    }
    out
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            // Work on bytes: slicing the &str here could split a multi-byte character.
            let hex = std::str::from_utf8(&b[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok());
            if let Some(v) = hex {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Joins `dir` and `href`, resolving `.` / `..` and dropping any `#fragment`.
fn resolve_href(dir: &str, href: &str) -> String {
    let href = percent_decode(href.split('#').next().unwrap_or(""));
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for seg in href.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            x => parts.push(x),
        }
    }
    parts.join("/")
}

fn zip_read(z: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    let mut s = String::new();
    read_capped(&mut z.by_name(name).ok()?, &mut s).ok()?;
    Some(s)
}

/// Content documents in the book's own reading order (OPF spine), navigation documents skipped.
fn epub_order(z: &mut zip::ZipArchive<std::fs::File>) -> Option<Vec<String>> {
    let container = zip_read(z, "META-INF/container.xml")?;
    let opf_path = open_tags(&container, "rootfile")
        .iter()
        .find_map(|t| attr(t, "full-path"))?;
    let opf = zip_read(z, &opf_path)?;
    let dir = opf_path
        .rsplit_once('/')
        .map(|(d, _)| d.to_string())
        .unwrap_or_default();
    let mut manifest: std::collections::HashMap<String, (String, bool)> =
        std::collections::HashMap::new();
    for t in open_tags(&opf, "item") {
        if let (Some(id), Some(href)) = (attr(t, "id"), attr(t, "href")) {
            let nav = attr(t, "properties")
                .map(|p| p.split_whitespace().any(|w| w == "nav"))
                .unwrap_or(false);
            manifest.insert(id, (href, nav));
        }
    }
    let order: Vec<String> = open_tags(&opf, "itemref")
        .iter()
        .filter_map(|t| attr(t, "idref"))
        .filter_map(|id| manifest.get(&id))
        .filter(|(_, nav)| !nav)
        .map(|(href, _)| resolve_href(&dir, href))
        .collect();
    if order.is_empty() { None } else { Some(order) }
}

/// EPUB -> text of the content documents in reading order (spine); if the package file cannot be
/// understood, falls back to file-name order.
fn epub_text(path: &Path) -> Result<String> {
    let f = std::fs::File::open(path)?;
    let mut z = zip::ZipArchive::new(f).context("not a valid zip container")?;
    if let Some(order) = epub_order(&mut z) {
        let text = order
            .iter()
            .filter_map(|n| zip_read(&mut z, n))
            .map(|x| strip_html(&x))
            .collect::<Vec<_>>()
            .join("\n\n");
        if !text.trim().is_empty() {
            return Ok(text);
        }
    }
    drop(z);
    let parts = zip_text(path, |n| {
        let l = n.to_lowercase();
        (l.ends_with(".xhtml") || l.ends_with(".html") || l.ends_with(".htm"))
            && !l.contains("toc")
            && !l.contains("nav")
    })?;
    Ok(parts
        .into_iter()
        .map(|(_, x)| strip_html(&x))
        .collect::<Vec<_>>()
        .join("\n\n"))
}

/// (page number, text); page 0 = format has no pages.
pub fn read_pages(path: &Path) -> Result<Vec<(usize, String)>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext == "pdf" {
        let mut pages = pdf_pages(path)?;
        if looks_scanned(&pages) {
            match ocr_pdf(path) {
                Some(o) if !looks_scanned(&o) => pages = o,
                Some(_) => {}
                None => eprintln!(
                    "  (no text layer; install tesseract + poppler to OCR it automatically)"
                ),
            }
        } else if crate::pdfx::enabled() {
            // Advanced reading: on pages where the layout shows a table or a display equation, store them as Markdown / LaTeX-style text.
            if let Some(lp) = crate::pdfx::layout_pages(path)
                && lp.len() == pages.len()
            {
                for (i, l) in lp.iter().enumerate() {
                    let items = crate::pdfx::parse_page(l);
                    if crate::pdfx::has_structure(&items) {
                        pages[i] = crate::pdfx::render(&items);
                    }
                }
            }
        }
        return Ok(pages
            .into_iter()
            .enumerate()
            .map(|(i, t)| (i + 1, t))
            .collect());
    }
    if ext == "docx" {
        return Ok(vec![(0, docx_text(path)?)]);
    }
    if ext == "epub" {
        return Ok(vec![(0, epub_text(path)?)]);
    }
    let raw = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&raw).to_string();
    let text = if ext == "html" || ext == "htm" {
        strip_html(&text)
    } else {
        text
    };
    Ok(vec![(0, text)])
}

/// A word that plausibly ends a sentence ("…proof." / "…?" / "…!"), ignoring closing quotes/brackets.
/// Very short tokens ("p.", "1.") do not count, which avoids most abbreviation cuts.
fn ends_sentence(w: &str) -> bool {
    let t = w.trim_end_matches(['"', '\'', ')', ']', '”', '’', '»']);
    t.chars().count() > 2 && t.ends_with(['.', '!', '?', '…'])
}

/// Splits text into overlapping word windows of about `size` words. Windows end at a sentence
/// boundary when one lies in the last 30% of the window, and the overlap starts at a sentence
/// start when possible, so passages read as whole sentences. Text with no punctuation falls
/// back to plain fixed windows.
pub fn chunk_words(text: &str, size: usize, overlap: usize) -> Vec<String> {
    let text = text.replace("-\n", ""); // re-join hyphenated line breaks
    let words: Vec<&str> = text.split_whitespace().collect();
    let n = words.len();
    if n == 0 {
        return vec![];
    }
    let size = size.max(1);
    let mut out = Vec::new();
    let mut i = 0;
    loop {
        let hard = (i + size).min(n);
        let mut end = hard;
        if hard < n {
            let lo = i + size * 7 / 10;
            if let Some(j) = (lo..hard).rev().find(|&j| ends_sentence(words[j])) {
                end = j + 1;
            }
        }
        out.push(words[i..end].join(" "));
        if end == n {
            break;
        }
        let mut start = end.saturating_sub(overlap).max(i + 1);
        if let Some(k) = (start..end).find(|&k| k > 0 && ends_sentence(words[k - 1])) {
            start = k;
        }
        i = start;
    }
    // Drop a tiny trailing fragment that is entirely overlap.
    if out.len() > 1
        && out
            .last()
            .map(|c| c.split_whitespace().count() <= overlap)
            .unwrap_or(false)
    {
        out.pop();
    }
    out
}

pub fn pretty_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .replace(['_', '-'], " ")
        .trim()
        .to_string()
}

pub enum Outcome {
    Added { chunks: usize },
    Unchanged,
    NoText,
}

/// Canonical path without the Windows `\\?\` verbatim prefix.
fn canonical(p: &Path) -> PathBuf {
    let c = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    match c.to_str().and_then(|s| s.strip_prefix(r"\\?\")) {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => c,
    }
}

/// Identity of a document inside the index. Files under `base` (the index's folder, i.e. the
/// portable `Data` dir) are stored relative to it with `/` separators, so the whole folder can
/// move to another drive letter or machine and keep working. Other files keep absolute paths.
pub fn doc_key(path: &Path, base: &Path) -> String {
    let canon = canonical(path);
    let b = canonical(base);
    match canon.strip_prefix(&b) {
        Ok(rel) if !rel.as_os_str().is_empty() => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/"),
        _ => canon.to_string_lossy().to_string(),
    }
}

/// Adds or refreshes one file. `base` is the folder the index lives in (see `doc_key`).
/// `force` re-reads files even when unchanged (use after the chunking or readers improve).
pub fn ingest_file(store: &mut Store, path: &Path, base: &Path, force: bool) -> Result<Outcome> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let hash = fnv1a(&bytes);
    drop(bytes);
    let key = doc_key(path, base);

    if let Some(id) = store.doc_by_path(&key) {
        if store.docs[id].hash == hash && !force {
            return Ok(Outcome::Unchanged);
        }
        store.remove_doc(id);
    }

    let pages = read_pages(path)?;
    let npages = if pages.first().map(|p| p.0).unwrap_or(0) == 0 {
        0
    } else {
        pages.len()
    };
    let mut new_chunks = Vec::new();
    let doc_id = store.docs.len();
    for (page, text) in pages {
        for c in crate::pdfx::chunk_page(&text, CHUNK_WORDS, OVERLAP_WORDS, &chunk_words) {
            new_chunks.push(Chunk {
                doc: doc_id,
                page,
                text: c,
                emb: None,
            });
        }
    }
    if new_chunks.is_empty() {
        return Ok(Outcome::NoText);
    }
    let n = new_chunks.len();
    store.docs.push(Doc {
        path: key,
        title: pretty_title(path),
        hash,
        pages: npages,
    });
    store.chunks.extend(new_chunks);
    Ok(Outcome::Added { chunks: n })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunking_overlaps_and_covers() {
        let text = (0..450)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let c = chunk_words(&text, 200, 40);
        assert_eq!(c.len(), 3);
        assert!(c[0].starts_with("w0 "));
        assert!(c[1].starts_with("w160 "));
        assert!(c[2].ends_with("w449"));
    }

    #[test]
    fn keys_are_relative_under_base_and_absolute_elsewhere() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp")
            .join("keys");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Data/Library/sub")).unwrap();
        std::fs::create_dir_all(root.join("elsewhere")).unwrap();
        let inside = root.join("Data/Library/sub/book.txt");
        let outside = root.join("elsewhere/paper.txt");
        std::fs::write(&inside, "alpha beta gamma").unwrap();
        std::fs::write(&outside, "delta").unwrap();
        let base = root.join("Data");
        assert_eq!(doc_key(&inside, &base), "Library/sub/book.txt");
        assert!(
            doc_key(&outside, &base)
                .replace('\\', "/")
                .ends_with("elsewhere/paper.txt")
        );
        assert!(Path::new(&doc_key(&outside, &base)).is_absolute());
        // Re-ingest is recognised through the relative key.
        let mut st = Store::default();
        assert!(matches!(
            ingest_file(&mut st, &inside, &base, false).unwrap(),
            Outcome::Added { .. }
        ));
        assert!(matches!(
            ingest_file(&mut st, &inside, &base, false).unwrap(),
            Outcome::Unchanged
        ));
        assert_eq!(st.docs[0].path, "Library/sub/book.txt");
        // --force re-reads an unchanged file instead of skipping it
        assert!(matches!(
            ingest_file(&mut st, &inside, &base, true).unwrap(),
            Outcome::Added { .. }
        ));
        assert_eq!(st.docs.len(), 1);
    }

    #[test]
    fn chunks_end_on_sentence_boundaries() {
        // 60 sentences of 10 words each.
        let text = (0..60)
            .map(|i| format!("Sentence number {i} says something about the topic at hand today."))
            .collect::<Vec<_>>()
            .join(" ");
        let c = chunk_words(&text, 100, 20);
        assert!(c.len() > 3);
        for ch in &c {
            assert!(
                ch.starts_with("Sentence"),
                "chunk must start at a sentence start: {ch:.40}"
            );
            assert!(
                ch.ends_with("today."),
                "chunk must end at a sentence end: …{}",
                &ch[ch.len().saturating_sub(30)..]
            );
        }
        // overlap exists and nothing is lost
        assert!(
            c[1].starts_with(&format!("Sentence number {}", 8))
                || c[1].starts_with("Sentence number")
        );
        let joined = c.join(" ");
        for i in 0..60 {
            assert!(
                joined.contains(&format!("number {i} ")),
                "sentence {i} lost"
            );
        }
    }

    #[test]
    fn epub_follows_spine_not_file_names() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp")
            .join("spine");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let e = dir.join("s.epub");
        make_zip(
            &e,
            &[
                (
                    "META-INF/container.xml",
                    "<container><rootfiles><rootfile full-path=\"OPS/book.opf\" media-type=\"application/oebps-package+xml\"/></rootfiles></container>",
                ),
                (
                    "OPS/book.opf",
                    "<package><manifest><item id=\"n\" href=\"nav.xhtml\" properties=\"nav\"/><item id=\"a\" href=\"text/a%20first.xhtml\"/><item id=\"b\" href=\"text/b.xhtml#top\"/><item id=\"c\" href=\"../OPS/text/c.xhtml\"/></manifest><spine><itemref idref=\"n\"/><itemref idref=\"c\"/><itemref idref=\"a\"/><itemref idref=\"b\"/></spine></package>",
                ),
                ("OPS/nav.xhtml", "<p>NAVIGATION</p>"),
                ("OPS/text/a first.xhtml", "<p>ALPHA</p>"),
                ("OPS/text/b.xhtml", "<p>BRAVO</p>"),
                ("OPS/text/c.xhtml", "<p>CHARLIE</p>"),
            ],
        );
        let t = &read_pages(&e).unwrap()[0].1;
        let (c, a, b) = (
            t.find("CHARLIE").unwrap(),
            t.find("ALPHA").unwrap(),
            t.find("BRAVO").unwrap(),
        );
        assert!(c < a && a < b, "spine order is C, A, B: {t:?}");
        assert!(!t.contains("NAVIGATION"));
    }

    #[test]
    fn percent_decoding_is_safe_on_odd_input() {
        assert_eq!(percent_decode("a%20b%C8%99"), "a bș");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%é9"), "%é9"); // would panic if the &str were sliced
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn dehyphenates() {
        let c = chunk_words("recogni-\ntion of dip-\nlomas", 50, 10);
        assert_eq!(c[0], "recognition of diplomas");
    }

    #[test]
    fn decodes_entities() {
        assert_eq!(
            decode_entities("a &amp; b &lt;c&gt; &#8211; &#x21; &bogus; &"),
            "a & b <c> – ! &bogus; &"
        );
    }

    fn make_zip(path: &Path, files: &[(&str, &str)]) {
        use std::io::Write;
        let mut z = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (n, c) in files {
            z.start_file(*n, zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(c.as_bytes()).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn reads_docx_and_epub() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp")
            .join("zips");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let d = dir.join("t.docx");
        make_zip(
            &d,
            &[(
                "word/document.xml",
                "<w:document><w:p><w:r><w:t>Prima &amp; a doua</w:t></w:r></w:p><w:p><w:r><w:t>Al doilea paragraf</w:t></w:r></w:p></w:document>",
            )],
        );
        let t = &read_pages(&d).unwrap()[0].1;
        assert!(t.contains("Prima & a doua") && t.contains("Al doilea paragraf"));
        assert!(
            t.lines().filter(|l| !l.is_empty()).count() == 2,
            "paragraphs become lines: {t:?}"
        );
        let e = dir.join("b.epub");
        make_zip(
            &e,
            &[
                (
                    "OEBPS/ch01.xhtml",
                    "<html><body><p>Capitolul unu</p></body></html>",
                ),
                (
                    "OEBPS/ch02.xhtml",
                    "<html><body><p>Capitolul doi</p></body></html>",
                ),
                ("OEBPS/toc.xhtml", "<p>SKIPPED</p>"),
                ("OEBPS/cover.jpg", "x"),
            ],
        );
        let t = &read_pages(&e).unwrap()[0].1;
        assert!(t.find("unu").unwrap() < t.find("doi").unwrap());
        assert!(!t.contains("SKIPPED"));
    }

    #[test]
    fn html_strip_drops_scripts() {
        let t = strip_html("<p>Hello</p><script>var x=1;</script><b>world</b>");
        assert!(t.contains("Hello") && t.contains("world") && !t.contains("var x"));
    }
}
