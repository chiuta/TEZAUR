//! RIS reader (spec 05). Produces the same `Entry` list as the BibTeX reader, plus warnings.
//! Tolerant: CRLF / LF / CR, BOM, blank lines, continuation lines, missing `ER`, missing `TY`.

use crate::bib::Entry;
use crate::ids::{self, IdKind, Verdict};
use crate::kinds::{self, Format};
use std::collections::HashSet;

/// `(tag, value)` when the line starts like `TY  - value`: 2-5 upper-case letters or digits,
/// spaces, a hyphen, then the value.
fn split_tag(line: &str) -> Option<(&str, &str)> {
    let tag_len = line
        .chars()
        .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        .count();
    if !(2..=5).contains(&tag_len) {
        return None;
    }
    let rest = &line[tag_len..];
    let after_sp = rest.trim_start_matches(' ');
    if after_sp.len() == rest.len() && !after_sp.starts_with('-') {
        return None;
    }
    let value = after_sp.strip_prefix('-')?;
    if !(value.is_empty() || value.starts_with(' ') || value.starts_with('\t')) {
        return None;
    }
    Some((&line[..tag_len], value.trim()))
}

fn is_roman(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| "ivxlcdmIVXLCDM".contains(c))
}

fn is_pageish(s: &str) -> bool {
    !s.is_empty() && (s.chars().all(|c| c.is_ascii_digit()) || is_roman(s))
}

fn year_of(v: &str) -> String {
    let b: Vec<char> = v.chars().collect();
    let mut run = String::new();
    for c in b {
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

fn key_base(e: &Entry) -> String {
    let fam = e
        .people_for_key()
        .map(|f| {
            f.chars()
                .flat_map(|c| c.to_lowercase())
                .map(crate::index::fold)
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
        })
        .unwrap_or_default();
    let fam = if fam.is_empty() {
        "ref".to_string()
    } else {
        fam
    };
    format!("{fam}{}", e.year)
}

fn letter_suffix(mut n: usize) -> String {
    // 0 -> a, 25 -> z, 26 -> aa ...
    let mut s = Vec::new();
    loop {
        s.push((b'a' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s.iter().rev().collect()
}

struct Rec {
    fields: Vec<(String, String)>,
    closed: bool,
}

fn build(rec: &Rec, warnings: &mut Vec<String>, used: &mut HashSet<String>) -> Entry {
    let all = |t: &[&str]| -> Vec<&str> {
        rec.fields
            .iter()
            .filter(|(k, _)| t.contains(&k.as_str()))
            .map(|(_, v)| v.as_str())
            .collect()
    };
    let first = |t: &[&str]| -> String {
        t.iter()
            .find_map(|k| {
                rec.fields
                    .iter()
                    .find(|(n, _)| n == k)
                    .map(|(_, v)| v.clone())
            })
            .unwrap_or_default()
    };
    let mut e = Entry::default();
    let ty = first(&["TY"]);
    if ty.is_empty() {
        warnings.push("record without TY: generic type used".into());
    }
    let (_, genre) = kinds::to_csl(Format::Ris, &ty);
    e.genre = genre;
    e.origin = if ty.is_empty() {
        None
    } else {
        Some((Format::Ris, ty.trim().to_string()))
    };
    let people = |v: Vec<&str>| -> String {
        v.iter()
            .map(|a| {
                // a name without comma that contains the word "and" must stay one name
                if !a.contains(',') && a.split_whitespace().any(|w| w == "and") {
                    format!("{{{a}}}")
                } else {
                    a.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(" and ")
    };
    e.raw_author = people(all(&["AU", "A1"]));
    e.author = e.raw_author.clone();
    e.editor = people(all(&["A2", "ED"]));
    if e.author.is_empty() && !e.editor.is_empty() {
        e.author = e.editor.clone();
        e.raw_author = e.editor.clone();
    }
    e.title = first(&["TI", "T1"]);
    e.container = {
        let c = first(&["T2", "JF", "JO", "JA", "J1", "J2"]);
        if c.is_empty() { first(&["PB"]) } else { c }
    };
    e.publisher = first(&["PB"]);
    e.address = first(&["CY", "PP"]);
    e.volume = first(&["VL"]);
    e.number = first(&["IS"]);
    e.year = year_of(&first(&["PY", "Y1"]));
    let mut notes: Vec<String> = all(&["N1", "NO"]).iter().map(|s| s.to_string()).collect();

    let doi = first(&["DO", "DI", "DOI"]);
    if !doi.is_empty() {
        match ids::normalize_doi(&doi) {
            Some(f) => e.doi = f.display,
            None => {
                warnings.push(format!(
                    "DOI `{doi}` does not match the DOI syntax; kept as is"
                ));
                e.doi = doi;
            }
        }
    }
    for sn in all(&["SN"]) {
        let hit = ids::check_issn(sn).or_else(|| ids::check_isbn(sn));
        match hit {
            Some(f) if f.verdict != Verdict::BadStructure => {
                if f.verdict == Verdict::BadChecksum {
                    warnings.push(format!("SN `{sn}` has a wrong check digit"));
                }
                match f.kind {
                    IdKind::Issn => e.issn = f.canonical,
                    _ => e.isbn = f.canonical,
                }
            }
            _ => notes.push(format!("SN: {sn}")),
        }
    }
    // pages
    let (sp, ep) = (first(&["SP"]), first(&["EP"]));
    e.pages = if is_pageish(&sp) && is_pageish(&ep) {
        format!("{sp}\u{2013}{ep}")
    } else {
        if !ep.is_empty() {
            notes.push(format!("EP: {ep}"));
        }
        sp
    };
    // addresses
    let mut urls: Vec<String> = Vec::new();
    for u in all(&["UR", "L2"]) {
        urls.extend(
            u.split(';')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(String::from),
        );
    }
    if let Some((f, rest)) = urls.split_first() {
        e.url = f.clone();
        for r in rest {
            notes.push(format!("URL: {r}"));
        }
    }
    e.file = first(&["L1"]);
    e.note = notes.join("; ");
    if !rec.closed {
        warnings.push(format!("record `{}` has no ER", e.title));
    }
    e.key = unique_key(&e, &first(&["ID"]), used);
    e
}

/// `explicit` if non-empty and unused; else surname + year (+ a, b, c...) not in `used`.
/// The chosen key is added to `used`.
pub fn unique_key(e: &Entry, explicit: &str, used: &mut HashSet<String>) -> String {
    let key = if !explicit.is_empty() && !used.contains(&explicit.to_lowercase()) {
        explicit.to_string()
    } else {
        let base = key_base(e);
        let mut k = base.clone();
        let mut n = 0;
        while used.contains(&k.to_lowercase()) {
            k = format!("{base}{}", letter_suffix(n));
            n += 1;
        }
        k
    };
    used.insert(key.to_lowercase());
    key
}

/// Reads RIS text. Returns the entries and warnings; never fails.
pub fn parse(text: &str) -> (Vec<Entry>, Vec<String>) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let norm = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut warnings = Vec::new();
    let mut recs: Vec<Rec> = Vec::new();
    let mut cur: Option<Rec> = None;
    let mut last_idx: Option<usize> = None;
    for line in norm.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match split_tag(line) {
            Some(("ER", _)) => {
                if let Some(mut r) = cur.take() {
                    r.closed = true;
                    recs.push(r);
                }
                last_idx = None;
            }
            Some((tag, val)) => {
                if tag == "TY" {
                    if let Some(r) = cur.take() {
                        recs.push(r); // previous record never closed
                    }
                    cur = Some(Rec {
                        fields: vec![],
                        closed: false,
                    });
                } else if cur.is_none() {
                    cur = Some(Rec {
                        fields: vec![],
                        closed: false,
                    });
                }
                let r = cur.as_mut().unwrap();
                if val.is_empty() {
                    last_idx = None;
                    continue;
                }
                r.fields.push((tag.to_string(), val.to_string()));
                last_idx = Some(r.fields.len() - 1);
            }
            None => {
                if let (Some(r), Some(i)) = (cur.as_mut(), last_idx) {
                    r.fields[i].1.push(' ');
                    r.fields[i].1.push_str(line.trim());
                }
            }
        }
    }
    if let Some(r) = cur.take() {
        recs.push(r);
    }
    let mut used = HashSet::new();
    let entries: Vec<Entry> = recs
        .iter()
        .map(|r| build(r, &mut warnings, &mut used))
        .collect();
    if entries.is_empty() {
        warnings.push("no RIS records found".into());
    }
    (entries, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = "TY  - JOUR\nAU  - Smith, Jane\nAU  - Doe, John\nTI  - On proof\nT2  - Law Rev\nPY  - 2019\nDO  - 10.1/x\nER  - \n";

    #[test]
    fn basic_record() {
        let (e, w) = parse(ONE);
        assert_eq!(e.len(), 1);
        let e = &e[0];
        assert_eq!(e.author, "Smith, Jane and Doe, John");
        assert_eq!((e.container.as_str(), e.year.as_str()), ("Law Rev", "2019"));
        assert_eq!(e.doi, "10.1/x");
        assert!(e.to_csl()["type"] == "article-journal");
        assert_eq!(e.to_csl()["author"].as_array().unwrap().len(), 2);
        assert!(w.iter().all(|x| x.contains("DOI")), "{w:?}");
    }

    #[test]
    fn glued_records() {
        // `ER  - ` followed directly by TY on the next line is the normal layout
        let (g, _) = parse("TY  - JOUR\nTI  - A\nER  - \nTY  - BOOK\nTI  - B\nER  - \n");
        assert_eq!(g.len(), 2);
        assert_eq!(g[1].to_csl()["type"], "book");
    }

    #[test]
    fn pages_range() {
        let (e, _) = parse("TY  - JOUR\nSP  - 379\nEP  - 423\nER  - \n");
        assert_eq!(e[0].pages, "379\u{2013}423");
        let (r, _) = parse("TY  - JOUR\nSP  - xii\nEP  - xv\nER  - \n");
        assert_eq!(r[0].pages, "xii\u{2013}xv");
        let (s, _) = parse("TY  - JOUR\nSP  - 12\nEP  - n/a\nER  - \n");
        assert_eq!(s[0].pages, "12");
        assert!(s[0].note.contains("EP: n/a"));
    }

    #[test]
    fn missing_er_warns() {
        let (e, w) = parse("TY  - THES\nTI  - Teza\nAU  - Pop, Ion");
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].to_csl()["type"], "thesis");
        assert!(w.iter().any(|x| x.contains("no ER")));
    }

    #[test]
    fn serial_numbers() {
        let (e, _) = parse(
            "TY  - JOUR\nSN  - 0317-8471\nER  - \nTY  - BOOK\nSN  - 978-92-95055-12-4\nER  - \nTY  - GEN\nSN  - ZZ-99\nER  - \n",
        );
        assert_eq!(e[0].issn, "0317-8471");
        assert_eq!(e[1].isbn, "9789295055124");
        assert!(e[2].isbn.is_empty() && e[2].issn.is_empty());
        assert!(e[2].note.contains("ZZ-99"));
    }

    #[test]
    fn generated_keys() {
        let rec = "TY  - JOUR\nAU  - Smith, Jane\nPY  - 2019\nER  - \n";
        let (e, _) = parse(&format!("{rec}{rec}{rec}"));
        let keys: Vec<&str> = e.iter().map(|x| x.key.as_str()).collect();
        assert_eq!(keys, ["smith2019", "smith2019a", "smith2019b"]);
        let (d, _) = parse(
            "TY  - JOUR\nID  - k1\nER  - \nTY  - JOUR\nID  - K1\nAU  - Țurcanu, A\nPY  - 2001\nER  - \n",
        );
        assert_eq!(d[0].key, "k1");
        assert_eq!(d[1].key, "turcanu2001");
    }

    #[test]
    fn written_by_this_program_reads_back() {
        let src = crate::bib::parse(
            "@article{smith2019, author={Smith, Jane and Doe, John}, title={On proof}, journal={Law Rev}, year={2019}, doi={10.1000/x}}",
        );
        let (back, _) = parse(&src[0].to_ris());
        let b = &back[0];
        assert_eq!(b.key, "smith2019");
        assert_eq!(b.author, src[0].author);
        assert_eq!(
            (
                b.title.as_str(),
                b.container.as_str(),
                b.year.as_str(),
                b.doi.as_str()
            ),
            ("On proof", "Law Rev", "2019", "10.1000/x")
        );
        assert_eq!(b.to_csl()["type"], "article-journal");
    }

    #[test]
    fn line_endings_and_oddities() {
        let mixed = "\u{feff}TY  - JOUR\r\nTI  - A very\r\n long title\nAU  - Doe, J\rPY  - 2001/05/01\nER  - \r\n";
        let lf = "TY  - JOUR\nTI  - A very long title\nAU  - Doe, J\nPY  - 2001/05/01\nER  - \n";
        let (a, _) = parse(mixed);
        let (b, _) = parse(lf);
        assert_eq!(a.len(), 1);
        assert_eq!(
            (a[0].title.clone(), a[0].author.clone(), a[0].year.clone()),
            (b[0].title.clone(), b[0].author.clone(), b[0].year.clone())
        );
        assert_eq!(a[0].title, "A very long title");
        assert_eq!(a[0].year, "2001");
        let (n, w) = parse("just some text\n\u{0}\u{1}");
        assert!(n.is_empty() && !w.is_empty());
        let (u, w2) = parse("AU  - Doe, J\nTI  - No type\nER  - \n");
        assert_eq!(u.len(), 1);
        assert!(w2.iter().any(|x| x.contains("TY")));
        assert_eq!(u[0].to_csl()["type"], "document");
    }

    #[test]
    fn urls_and_links() {
        let (e, _) = parse(
            "TY  - ELEC\nUR  - http://a.example; http://b.example\nL1  - C:\\x\\y.pdf\nER  - \n",
        );
        assert_eq!(e[0].url, "http://a.example");
        assert!(e[0].note.contains("http://b.example"));
        assert_eq!(e[0].file, "C:\\x\\y.pdf");
    }
}
