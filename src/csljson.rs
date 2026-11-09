//! CSL JSON reader (spec 06): a top-level list of items, or the `payload` of a `citation/csl`
//! envelope. Tolerant of unknown properties; bad items are skipped with a warning.

use crate::bib::Entry;
use crate::ids::{self, IdKind, Verdict};
use crate::kinds::{self, Format};
use serde_json::Value;
use std::collections::HashSet;

/// Text of a JSON string or number (ids, volumes, pages may be either).
fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn get(item: &Value, k: &str) -> String {
    item.get(k).map(text).unwrap_or_default()
}

/// Wraps a name part in braces when a comma, braces or the word "and" would confuse the BibTeX
/// style name parser (spec 04).
fn guard(s: &str) -> String {
    if s.contains([',', '{', '}']) || s.split_whitespace().any(|w| w == "and") {
        format!("{{{}}}", s.replace(['{', '}'], ""))
    } else {
        s.to_string()
    }
}

/// One CSL name object as a BibTeX-style name: `[particle ]Family[, Suffix], Given[ dropping]`.
fn name_text(n: &Value) -> Option<String> {
    let obj = n.as_object()?;
    let s = |k: &str| {
        obj.get(k)
            .map(text)
            .filter(|x| !x.is_empty())
            .unwrap_or_default()
    };
    let lit = s("literal");
    if !lit.is_empty() {
        return Some(format!("{{{}}}", lit.replace(['{', '}'], "")));
    }
    let (fam, giv) = (s("family"), s("given"));
    if fam.is_empty() && giv.is_empty() {
        return None;
    }
    let ndp = s("non-dropping-particle");
    let dp = s("dropping-particle");
    let suffix = s("suffix");
    let mut last = guard(&fam);
    if !ndp.is_empty() {
        last = format!("{ndp} {last}");
    }
    let mut given = guard(&giv);
    if !dp.is_empty() {
        given = if given.is_empty() {
            dp
        } else {
            format!("{given} {dp}")
        };
    }
    Some(match (suffix.is_empty(), given.is_empty()) {
        (true, true) => last,
        (true, false) => format!("{last}, {given}"),
        (false, _) => format!("{last}, {suffix}, {given}"),
    })
}

fn names(item: &Value, k: &str, key: &str, warn: &mut Vec<String>) -> String {
    let Some(v) = item.get(k) else {
        return String::new();
    };
    let Some(arr) = v.as_array() else {
        warn.push(format!("{key}: `{k}` is not a list; skipped"));
        return String::new();
    };
    let mut out = Vec::new();
    for n in arr {
        match name_text(n) {
            Some(t) => out.push(t),
            None => warn.push(format!("{key}: a name in `{k}` could not be read; skipped")),
        }
    }
    out.join(" and ")
}

fn first_digits4(s: &str) -> String {
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

fn year(d: Option<&Value>) -> String {
    let Some(d) = d else {
        return String::new();
    };
    if let Some(s) = d.as_str() {
        return first_digits4(s);
    }
    if let Some(first) = d
        .get("date-parts")
        .and_then(|p| p.as_array())
        .and_then(|p| p.first())
        .and_then(|p| p.as_array())
        .and_then(|p| p.first())
    {
        let y = text(first);
        if !y.is_empty() {
            return y;
        }
    }
    for k in ["raw", "literal"] {
        let y = d.get(k).and_then(|x| x.as_str()).map(first_digits4);
        if let Some(y) = y.filter(|y| !y.is_empty()) {
            return y;
        }
    }
    String::new()
}

fn item_to_entry(item: &Value, warn: &mut Vec<String>) -> Option<Entry> {
    let id = get(item, "id");
    let ty = get(item, "type");
    let ck = get(item, "citation-key");
    if (id.is_empty() && ck.is_empty()) || ty.is_empty() {
        warn.push(format!(
            "item skipped: missing `{}`",
            if ty.is_empty() { "type" } else { "id" }
        ));
        return None;
    }
    let key = if ck.is_empty() { id } else { ck };
    let (csl, genre) = kinds::to_csl(Format::Csl, &ty);
    let known = csl == ty;
    if !known {
        warn.push(format!(
            "{key}: unknown or unsupported type `{ty}`; generic type used"
        ));
    }
    let mut e = Entry {
        key: key.clone(),
        origin: Some((
            Format::Csl,
            if known { ty.clone() } else { "document".into() },
        )),
        genre,
        ..Default::default()
    };
    e.raw_author = names(item, "author", &key, warn);
    e.editor = names(item, "editor", &key, warn);
    e.author = crate::bib::clean(&e.raw_author);
    e.editor = crate::bib::clean(&e.editor);
    if e.author.is_empty() && !e.editor.is_empty() {
        e.author = e.editor.clone();
        e.raw_author = names(item, "editor", &key, &mut Vec::new());
    }
    e.title = get(item, "title");
    e.container = get(item, "container-title");
    e.publisher = get(item, "publisher");
    if e.container.is_empty() {
        e.container = e.publisher.clone();
    }
    e.year = year(item.get("issued"));
    e.volume = get(item, "volume");
    e.number = get(item, "issue");
    e.pages = get(item, "page");
    e.address = get(item, "publisher-place");
    e.edition = get(item, "edition");
    e.url = get(item, "URL");
    e.note = get(item, "note");
    if known && csl == "thesis" {
        let g = get(item, "genre");
        if let Some(gn) = kinds::genre_from_text(&g) {
            e.genre = Some(gn);
        }
    }
    let doi = get(item, "DOI");
    if !doi.is_empty() {
        match ids::normalize_doi(&doi) {
            Some(f) => e.doi = f.display,
            None => {
                warn.push(format!(
                    "{key}: DOI `{doi}` does not match the DOI syntax; kept as is"
                ));
                e.doi = doi;
            }
        }
    }
    for (k, want) in [("ISBN", IdKind::Isbn13), ("ISSN", IdKind::Issn)] {
        // several values may share one string
        for part in get(item, k).split(|c: char| c == ',' || c == ';' || c.is_whitespace()) {
            if part.is_empty() {
                continue;
            }
            let hit = if want == IdKind::Issn {
                ids::check_issn(part)
            } else {
                ids::check_isbn(part)
            };
            if let Some(f) = hit.filter(|f| f.verdict != Verdict::BadStructure) {
                if want == IdKind::Issn {
                    e.issn = f.canonical;
                } else {
                    e.isbn = f.canonical;
                }
                break;
            }
        }
    }
    Some(e)
}

/// Reads CSL JSON text. `Err` only for invalid JSON or an unusable root.
pub fn parse(text: &str) -> Result<(Vec<Entry>, Vec<String>), String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let root: Value = serde_json::from_str(text).map_err(|e| {
        format!(
            "invalid JSON at line {}, column {}: {e}",
            e.line(),
            e.column()
        )
    })?;
    let items = match &root {
        Value::Array(a) => a,
        Value::Object(o) => match (o.get("payload"), o.get("type").and_then(|t| t.as_str())) {
            (Some(Value::Array(a)), None | Some("citation/csl")) => a,
            (Some(Value::Array(_)), Some(t)) => {
                return Err(format!("envelope type `{t}` is not `citation/csl`"));
            }
            _ => return Err("not a CSL JSON list and not a `citation/csl` envelope".into()),
        },
        _ => return Err("not a CSL JSON list and not a `citation/csl` envelope".into()),
    };
    let mut warn = Vec::new();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for it in items {
        if !it.is_object() {
            warn.push("item skipped: not an object".into());
            continue;
        }
        if let Some(e) = item_to_entry(it, &mut warn) {
            if !seen.insert(e.key.to_lowercase()) {
                warn.push(format!("duplicate key `{}`; the first item is kept", e.key));
                continue;
            }
            out.push(e);
        }
    }
    if out.is_empty() {
        warn.push("no CSL items imported".into());
    }
    Ok((out, warn))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(json: &str) -> (Entry, Vec<String>) {
        let (mut e, w) = parse(json).unwrap();
        (e.remove(0), w)
    }

    #[test]
    fn basic_item() {
        let (e, _) = one(
            r#"[{"id":"a","type":"article-journal","title":"T","author":[{"family":"Smith","given":"Jane"}],"issued":{"date-parts":[[2019,5,3]]},"DOI":"10.1/x","container-title":"J"}]"#,
        );
        assert_eq!(e.key, "a");
        assert_eq!(e.raw_author, "Smith, Jane");
        assert_eq!(
            (e.year.as_str(), e.doi.as_str(), e.container.as_str()),
            ("2019", "10.1/x", "J")
        );
        assert_eq!(e.to_csl()["type"], "article-journal");
    }

    #[test]
    fn dates() {
        let (e, _) = one(r#"[{"id":"a","type":"book","issued":{"raw":"May 2019"}}]"#);
        assert_eq!(e.year, "2019");
        let (n, w) = one(r#"[{"id":"a","type":"book","issued":{"literal":"n.d."}}]"#);
        assert_eq!(n.year, "");
        assert!(!w.iter().any(|x| x.contains("error")));
        let (z, _) = one(r#"[{"id":"a","type":"book","issued":{"date-parts":[[null]]}}]"#);
        assert_eq!(z.year, "");
        let (s, _) = one(r#"[{"id":"a","type":"book","issued":"2001-02-03"}]"#);
        assert_eq!(s.year, "2001");
        let (q, _) =
            one(r#"[{"id":"a","type":"book","issued":{"date-parts":[["1999"],["2001"]]}}]"#);
        assert_eq!(q.year, "1999");
    }

    #[test]
    fn literal_and_particles() {
        let (e, _) = one(
            r#"[{"id":"a","type":"book","issued":{"raw":"2001"},"author":[{"literal":"Council of Europe"}]}]"#,
        );
        assert_eq!(e.cite(), "Council of Europe (2001)");
        assert_eq!(e.to_csl()["author"][0]["literal"], "Council of Europe");
        let (v, _) = one(
            r#"[{"id":"a","type":"book","issued":{"raw":"2001"},"author":[{"family":"Gogh","given":"Vincent","non-dropping-particle":"van"}]}]"#,
        );
        assert_eq!(v.cite(), "van Gogh (2001)");
        let (f, _) = one(
            r#"[{"id":"a","type":"book","author":[{"family":"Ford","given":"Henry","suffix":"Jr."}]}]"#,
        );
        assert_eq!(f.to_csl()["author"][0]["suffix"], "Jr.");
    }

    #[test]
    fn skips_bad_items_and_duplicates() {
        let (es, w) = parse(
            r#"[{"type":"book","title":"no id"},{"id":"b","type":"book"},{"id":"b","type":"book","title":"dup"},{"id":"c"},5,{"id":"d","type":"book","author":"x"}]"#,
        )
        .unwrap();
        let keys: Vec<&str> = es.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["b", "d"]);
        assert!(w.iter().filter(|x| x.contains("skipped")).count() >= 3);
        assert!(w.iter().any(|x| x.contains("duplicate")));
    }

    #[test]
    fn envelope_and_roundtrip() {
        let src = crate::bib::parse(
            "@article{k1, author={Smith, Jane and Doe, John}, title={On proof}, journal={Law Rev}, year={2019}, doi={10.1000/x}, url={http://u.example}}",
        );
        let env = crate::fff::csl_envelope(&src);
        let (a, _) = parse(&env).unwrap();
        let list = serde_json::to_string(&vec![src[0].to_csl()]).unwrap();
        let (b, _) = parse(&list).unwrap();
        for x in [&a[0], &b[0]] {
            assert_eq!(x.key, "k1");
            assert_eq!(x.title, "On proof");
            assert_eq!(x.author, "Smith, Jane and Doe, John");
            assert_eq!(
                (x.year.as_str(), x.doi.as_str(), x.url.as_str()),
                ("2019", "10.1000/x", "http://u.example")
            );
        }
    }

    #[test]
    fn numeric_id_unknown_type_and_errors() {
        let (e, w) = one(r#"[{"id":42,"type":"inventat"}]"#);
        assert_eq!(e.key, "42");
        assert_eq!(e.to_csl()["type"], "document");
        assert!(w.iter().any(|x| x.contains("inventat")));
        let (c, _) = one(r#"[{"id":"x","citation-key":"ck","type":"book"}]"#);
        assert_eq!(c.key, "ck");
        let err = parse("[{\"id\": ").unwrap_err();
        assert!(err.contains("line 1"));
        assert!(parse("{\"a\":1}").is_err());
        assert!(parse("{\"type\":\"document/ocr\",\"payload\":[]}").is_err());
        assert!(parse("[]").unwrap().1.iter().any(|x| x.contains("no CSL")));
    }

    #[test]
    fn identifiers_and_fields() {
        let (e, _) = one(
            r#"[{"id":"i","type":"book","ISBN":"978-92-95055-12-4 0-306-40615-2","ISSN":"0317-8471","volume":3,"issue":"2","page":"1-5","publisher":"P","publisher-place":"Bucharest","edition":2}]"#,
        );
        assert_eq!(
            (e.isbn.as_str(), e.issn.as_str()),
            ("9789295055124", "0317-8471")
        );
        assert_eq!(
            (e.volume.as_str(), e.number.as_str(), e.pages.as_str()),
            ("3", "2", "1-5")
        );
        assert_eq!((e.address.as_str(), e.edition.as_str()), ("Bucharest", "2"));
    }
}
