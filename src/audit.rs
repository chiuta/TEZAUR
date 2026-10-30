//! Deterministic, model-free checks on a draft and on the library: does each citation really point at a
//! passage that says what the sentence says (`audit`), which outline sections have sources (`coverage`),
//! what a new paper adds beyond the library (`novelty`), and tamper-evident receipts for AI-assisted answers.
//! None of this proves truth. It finds the places a human must open the page.

use crate::bib::BibMap;
use crate::index::{Searcher, Store, coverage, fold};
use crate::sha256::{hex, hmac_sha256, sha256_hex};
use serde_json::{Value, json};
use std::collections::HashMap;

// ------------------------------------------------------------------ citations

#[derive(Clone, Debug, PartialEq)]
pub struct Cite {
    pub key: String,
    pub page: Option<usize>,
}

/// Replaces every pandoc-style group `[@a, p. 3; @b]` with a \u{1} marker; returns the groups in order.
pub fn extract_cites(text: &str) -> (String, Vec<Vec<Cite>>) {
    let chars: Vec<char> = text.chars().collect();
    let (mut out, mut groups, mut i) = (String::new(), Vec::new(), 0);
    while i < chars.len() {
        if chars[i] == '['
            && let Some(rel) = chars[i + 1..].iter().position(|&c| c == ']' || c == '[')
            && chars[i + 1 + rel] == ']'
        {
            let inner: String = chars[i + 1..i + 1 + rel].iter().collect();
            let cites = parse_group(&inner);
            if !cites.is_empty() {
                out.push('\u{1}');
                groups.push(cites);
                i += rel + 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    (out, groups)
}

fn parse_group(inner: &str) -> Vec<Cite> {
    let mut v = Vec::new();
    for item in inner.split(';') {
        let item = item.trim();
        let Some(rest) = item
            .strip_prefix('@')
            .or_else(|| item.find('@').map(|p| &item[p + 1..]))
        else {
            continue;
        };
        let key: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || "_:.-/".contains(*c))
            .collect();
        let key = key.trim_end_matches(['.', ':', '-']).to_string();
        if key.is_empty() {
            continue;
        }
        let tail = &rest[key.len()..];
        let digits: String = tail
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        v.push(Cite {
            key,
            page: digits.parse().ok(),
        });
    }
    v
}

fn is_abbrev(before: &str) -> bool {
    let w = before
        .rsplit(|c: char| c.is_whitespace())
        .next()
        .unwrap_or("")
        .trim_start_matches(['(', '"', '“']);
    let lw = w.to_lowercase();
    matches!(
        lw.as_str(),
        "al" | "cf"
            | "vs"
            | "etc"
            | "e.g"
            | "i.e"
            | "fig"
            | "eq"
            | "no"
            | "dr"
            | "prof"
            | "p"
            | "pp"
            | "ed"
            | "eds"
            | "cap"
            | "art"
            | "nr"
            | "pag"
    ) || (w.chars().count() == 1 && w.chars().all(|c| c.is_uppercase()))
}

/// Splits into sentences; a citation marker right after the full stop stays with the sentence it supports.
pub fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.split("\n\n") {
        let para = para.lines().map(|l| l.trim()).collect::<Vec<_>>().join(" ");
        let cs: Vec<char> = para.chars().collect();
        let (mut cur, mut i) = (String::new(), 0);
        while i < cs.len() {
            cur.push(cs[i]);
            if matches!(cs[i], '.' | '!' | '?' | '…') && !is_abbrev(cur.trim_end_matches(cs[i])) {
                let mut j = i + 1;
                while j < cs.len() && matches!(cs[j], '"' | '”' | '»' | ')' | '\u{1}') {
                    cur.push(cs[j]);
                    j += 1;
                }
                let mut k = j;
                while k < cs.len() && cs[k] == ' ' {
                    k += 1;
                }
                if k < cs.len() && cs[k] == '\u{1}' && k > j {
                    // "claim. [@a]" — marker belongs to the claim before it
                    cur.push('\u{1}');
                    j = k + 1;
                }
                if j >= cs.len() || cs.get(j).is_some_and(|c| c.is_whitespace()) {
                    out.push(cur.trim().to_string());
                    cur.clear();
                    i = j;
                    continue;
                }
                i = j;
                continue;
            }
            i += 1;
        }
        if !cur.trim().is_empty() {
            out.push(cur.trim().to_string());
        }
    }
    out.into_iter()
        .filter(|s| s.chars().any(|c| c.is_alphanumeric()))
        .collect()
}

fn quotes(s: &str) -> Vec<String> {
    let cs: Vec<char> = s.chars().collect();
    let mut v = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let close: &[char] = match cs[i] {
            '"' => &['"'],
            '“' => &['”', '"'],
            '«' => &['»'],
            '„' => &['”', '“'],
            _ => {
                i += 1;
                continue;
            }
        };
        if let Some(rel) = cs[i + 1..].iter().position(|c| close.contains(c)) {
            let q: String = cs[i + 1..i + 1 + rel].iter().collect();
            if q.split_whitespace().count() >= 4 {
                v.push(q);
            }
            i += rel + 2;
        } else {
            i += 1;
        }
    }
    v
}

pub fn norm(s: &str) -> String {
    let t: String = s
        .chars()
        .flat_map(|c| c.to_lowercase())
        .map(fold)
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

// --------------------------------------------------------- LaTeX / LyX input

const CITE_CMDS: &[&str] = &[
    "cite",
    "citep",
    "citet",
    "citealp",
    "citeauthor",
    "parencite",
    "textcite",
    "autocite",
    "footcite",
    "smartcite",
    "supercite",
    "nocite",
];

fn cite_group(keys: &str, post: &str) -> String {
    let page: String = post
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let ks: Vec<String> = keys
        .split(',')
        .map(|k| k.trim())
        .filter(|k| !k.is_empty())
        .map(|k| format!("@{k}"))
        .collect();
    if ks.is_empty() {
        return String::new();
    }
    let mut out = ks.join("; ");
    if !page.is_empty() {
        out = format!("{out}, p. {page}");
    }
    format!("[{out}]")
}

/// LaTeX (`\cite[p. 3]{key}`, biblatex variants) and LyX (`.lyx` citation insets) → the Markdown the audit reads
/// (`[@key, p. 3]`). Plain Markdown passes through untouched.
pub fn normalize_markup(text: &str) -> String {
    if text.contains("\\begin_body") && text.contains("\\begin_layout") {
        return lyx_to_md(text);
    }
    if text.contains("\\cite")
        || text.contains("\\parencite")
        || text.contains("\\textcite")
        || text.contains("\\begin{document}")
    {
        return latex_to_md(text);
    }
    text.to_string()
}

fn latex_to_md(src: &str) -> String {
    let body = src.split("\\begin{document}").nth(1).unwrap_or(src);
    let mut t: String = body
        .lines()
        .map(|l| {
            let mut prev = ' ';
            let mut cut = l.len();
            for (i, c) in l.char_indices() {
                if c == '%' && prev != '\\' {
                    cut = i;
                    break;
                }
                prev = c;
            }
            &l[..cut]
        })
        .collect::<Vec<_>>()
        .join("\n");
    t = t.replace("``", "“").replace("''", "”").replace("\\\\", " ");
    let cs: Vec<char> = t.chars().collect();
    let (mut out, mut i) = (String::new(), 0);
    while i < cs.len() {
        if cs[i] == '\\' {
            let mut j = i + 1;
            while j < cs.len() && cs[j].is_ascii_alphabetic() {
                j += 1;
            }
            let name: String = cs[i + 1..j].iter().collect();
            if j < cs.len() && cs[j] == '*' && !name.is_empty() {
                j += 1;
            }
            // optional args
            let mut opts: Vec<String> = Vec::new();
            let mut k = j;
            while k < cs.len() && cs[k] == '[' {
                if let Some(e) = cs[k..].iter().position(|&c| c == ']') {
                    opts.push(cs[k + 1..k + e].iter().collect());
                    k += e + 1;
                } else {
                    break;
                }
            }
            let arg = |from: usize| -> Option<(String, usize)> {
                if from < cs.len() && cs[from] == '{' {
                    let mut depth = 0;
                    for (n, &c) in cs[from..].iter().enumerate() {
                        if c == '{' {
                            depth += 1
                        } else if c == '}' {
                            depth -= 1;
                            if depth == 0 {
                                return Some((
                                    cs[from + 1..from + n].iter().collect(),
                                    from + n + 1,
                                ));
                            }
                        }
                    }
                }
                None
            };
            if CITE_CMDS.contains(&name.as_str())
                && name != "nocite"
                && let Some((keys, end)) = arg(k)
            {
                out.push_str(&cite_group(
                    &keys,
                    opts.last().map(|s| s.as_str()).unwrap_or(""),
                ));
                i = end;
                continue;
            }
            if matches!(
                name.as_str(),
                "section" | "subsection" | "subsubsection" | "chapter" | "paragraph"
            ) && let Some((title, end)) = arg(k)
            {
                out.push_str(&format!("\n\n# {title}\n\n"));
                i = end;
                continue;
            }
            if matches!(
                name.as_str(),
                "begin"
                    | "end"
                    | "label"
                    | "ref"
                    | "bibliography"
                    | "bibliographystyle"
                    | "usepackage"
                    | "documentclass"
                    | "maketitle"
                    | "newcommand"
            ) && let Some((_, end)) = arg(k)
            {
                out.push(' ');
                i = end;
                continue;
            }
            if name.is_empty() {
                // escaped symbol such as \& or \%
                if j < cs.len() {
                    out.push(cs[j]);
                    i = j + 1;
                } else {
                    i = j;
                }
                continue;
            }
            i = j; // formatting command: drop the name, keep its argument text
            continue;
        }
        if cs[i] == '{' || cs[i] == '}' {
            i += 1;
            continue;
        }
        if cs[i] == '~' {
            out.push(' ');
            i += 1;
            continue;
        }
        out.push(cs[i]);
        i += 1;
    }
    out
}

fn lyx_to_md(src: &str) -> String {
    let body = src.split("\\begin_body").nth(1).unwrap_or(src);
    let (mut out, mut heading, mut start) = (String::new(), false, true);
    let mut lines = body.lines().peekable();
    while let Some(l) = lines.next() {
        let l = l.trim_end();
        if let Some(rest) = l.strip_prefix("\\begin_layout ") {
            heading = matches!(
                rest.trim(),
                "Chapter"
                    | "Section"
                    | "Subsection"
                    | "Subsubsection"
                    | "Part"
                    | "Title"
                    | "Paragraph"
                    | "Section*"
                    | "Subsection*"
                    | "Chapter*"
            );
            start = true;
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            continue;
        }
        if l.starts_with("\\begin_inset CommandInset citation") {
            let (mut key, mut after) = (String::new(), String::new());
            for x in lines.by_ref() {
                if x.starts_with("\\end_inset") {
                    break;
                }
                let q = |p: &str| {
                    x.strip_prefix(p)
                        .map(|v| v.trim().trim_matches('"').to_string())
                };
                if let Some(v) = q("key ") {
                    key = v;
                }
                if let Some(v) = q("after ") {
                    after = v;
                }
            }
            out.push_str(&cite_group(&key, &after));
            continue;
        }
        if let Some(q) = l.strip_prefix("\\begin_inset Quotes ") {
            let kind = q.trim();
            out.push(if kind.chars().nth(1) == Some('l') {
                '“'
            } else {
                '”'
            });
            for x in lines.by_ref() {
                if x.starts_with("\\end_inset") {
                    break;
                }
            }
            continue;
        }
        if l.starts_with("\\begin_inset") {
            // footnotes, floats…: skip nested content except plain text lines
            continue;
        }
        if l.starts_with('\\') || l.starts_with('#') || l.is_empty() {
            continue;
        }
        if heading && start {
            out.push_str("# ");
        }
        if !start && !out.ends_with(' ') && !out.ends_with('“') {
            out.push(' ');
        }
        out.push_str(l);
        start = false;
    }
    out
}

// ---------------------------------------------------------------------- audit

#[derive(Clone, Debug)]
pub struct Finding {
    pub sentence: String,
    pub cite: Option<String>,
    pub page: Option<usize>,
    pub verdict: &'static str,
    pub support: f32,
    pub note: String,
    /// The library passage the sentence was compared with (set for cited sentences; used by the semantic pass).
    pub passage: Option<String>,
}

impl Finding {
    pub fn to_json(&self) -> Value {
        json!({"sentence": self.sentence, "cite": self.cite, "page": self.page, "verdict": self.verdict,
               "support": (self.support * 100.0).round() / 100.0, "note": self.note})
    }
    pub fn is_problem(&self) -> bool {
        !matches!(self.verdict, "supported" | "quote-verbatim")
    }
}

pub(crate) fn doc_for_key(store: &Store, bib: &BibMap, key: &str) -> Option<usize> {
    let k = key.to_lowercase();
    store.docs.iter().position(|d| {
        bib.find(&d.path)
            .map(|e| e.key.to_lowercase() == k)
            .unwrap_or(false)
            || std::path::Path::new(&d.path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase() == k)
                .unwrap_or(false)
    })
}

/// Numbers (digits or number words) that the sentence states but the cited passage does not contain. A swapped figure keeps almost every other
/// word the same, so plain overlap cannot see it.
fn missing_numbers(sentence: &str, passage: &str) -> Vec<String> {
    // Figures are compared by value, not by spelling: "4", "four" and "patru" are the same number, "twenty-five" is 25, "three and a half" is 3.5,
    // "71.3" is one number (not "71" and "3"), so a correct citation is not flagged for writing a digit where the source has a word,
    // and a re-ordered figure cannot hide.
    let canon = |s: &str| -> std::collections::HashSet<String> {
        let mut v: std::collections::HashSet<String> = figures(s).into_iter().collect();
        v.extend(number_words(s).into_iter().map(|(_, val)| val));
        v
    };
    let have = canon(passage);
    let mut miss: Vec<String> = figures(sentence)
        .into_iter()
        .filter(|n| !have.contains(n))
        .collect();
    miss.extend(
        number_words(sentence)
            .into_iter()
            .filter(|(_, v)| !have.contains(v))
            .map(|(t, _)| t),
    );
    miss.dedup();
    miss
}

#[derive(Clone, Copy, PartialEq)]
enum Tok {
    Add(u32),
    Hundred,
    Scale(u32),
    And,
    Point,
    Half,
}

/// Number words (English, Romanian without diacritics), unambiguous only: "o"/"a" are not numbers here.
fn tok(w: &str) -> Option<Tok> {
    use Tok::*;
    Some(match w {
        "one" | "unu" | "una" => Add(1),
        "two" | "doi" | "doua" | "twice" => Add(2),
        "three" | "trei" => Add(3),
        "four" | "patru" => Add(4),
        "five" | "cinci" => Add(5),
        "six" | "sase" => Add(6),
        "seven" | "sapte" => Add(7),
        "eight" | "opt" => Add(8),
        "nine" | "noua" => Add(9),
        "ten" | "zece" => Add(10),
        "eleven" | "unsprezece" => Add(11),
        "twelve" | "doisprezece" => Add(12),
        "thirteen" | "treisprezece" => Add(13),
        "fourteen" | "paisprezece" => Add(14),
        "fifteen" | "cincisprezece" => Add(15),
        "sixteen" | "saisprezece" => Add(16),
        "seventeen" | "saptesprezece" => Add(17),
        "eighteen" | "optsprezece" => Add(18),
        "nineteen" | "nouasprezece" => Add(19),
        "twenty" | "douazeci" => Add(20),
        "thirty" | "treizeci" => Add(30),
        "forty" | "patruzeci" => Add(40),
        "fifty" | "cincizeci" => Add(50),
        "sixty" | "saizeci" => Add(60),
        "seventy" | "saptezeci" => Add(70),
        "eighty" | "optzeci" => Add(80),
        "ninety" | "nouazeci" => Add(90),
        "hundred" | "suta" | "sute" => Hundred,
        "thousand" | "mie" | "mii" => Scale(1000),
        "million" | "milion" | "milioane" => Scale(1_000_000),
        "and" | "si" => And,
        "point" | "virgula" => Point,
        "half" => Half,
        _ => return None,
    })
}

/// Numbers written in words, as (the words, the value as digits): "twenty five" -> 25, "one hundred and twelve" -> 112, "three and a half" -> 3.5,
/// "one point two" -> 1.2, "sapte" -> 7. `norm` has already turned hyphens into spaces.
fn number_words(s: &str) -> Vec<(String, String)> {
    use Tok::*;
    let n = norm(s);
    let ws: Vec<&str> = n.split(' ').collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < ws.len() {
        let Some(t0) = tok(ws[i]) else {
            i += 1;
            continue;
        };
        if matches!(t0, And | Point) {
            i += 1;
            continue;
        }
        let (mut total, mut cur, mut j, mut frac) = (0u64, 0u64, i, None::<String>);
        let mut used = i;
        let mut seen = false;
        while j < ws.len() {
            match tok(ws[j]) {
                Some(Add(v)) => {
                    cur += v as u64;
                    seen = true;
                    used = j;
                }
                Some(Hundred) => {
                    cur = cur.max(1) * 100;
                    seen = true;
                    used = j;
                }
                Some(Scale(m)) => {
                    total += cur.max(1) * m as u64;
                    cur = 0;
                    seen = true;
                    used = j;
                }
                Some(Half) if !seen => {
                    cur = 0;
                    frac = Some("0.5".into());
                    used = j;
                    break;
                }
                Some(And) if seen => {
                    // "and" joins numerals ("a hundred and twelve") or introduces "a half" ("three and a half"); otherwise the number ended.
                    match (ws.get(j + 1).copied().and_then(tok), ws.get(j + 2).copied()) {
                        (Some(Add(_) | Hundred | Scale(_)), _) => {}
                        (_, Some("half")) if ws[j + 1] == "a" => {
                            frac = Some(".5".into());
                            used = j + 2;
                            break;
                        }
                        _ => break,
                    }
                }
                Some(Point) if seen => {
                    let mut d = String::new();
                    let mut k = j + 1;
                    while let Some(Some(Add(v @ 0..=9))) = ws.get(k).map(|w| tok(w)) {
                        d.push_str(&v.to_string());
                        k += 1;
                    }
                    if !d.is_empty() {
                        frac = Some(format!(".{d}"));
                        used = k - 1;
                    }
                    break;
                }
                _ => break,
            }
            j += 1;
        }
        let whole = total + cur;
        let val = match frac {
            Some(f) if f == "0.5" => f,
            Some(f) => format!("{whole}{f}"),
            None => whole.to_string(),
        };
        out.push((ws[i..=used].join(" "), val));
        i = used + 1;
    }
    out
}

/// Digit figures in `s`, normalised: thousands separators removed ("1,240" = "1240" = "1 240"), decimal comma read as a point.
fn figures(s: &str) -> Vec<String> {
    let cs: Vec<char> = s.chars().collect();
    let digit = |k: usize| k < cs.len() && cs[k].is_ascii_digit();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        if !cs[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let mut j = i;
        let mut groups: Vec<String> = vec![String::new()];
        let mut seps: Vec<char> = Vec::new();
        while j < cs.len() {
            if digit(j) {
                groups.last_mut().unwrap().push(cs[j]);
                j += 1;
            } else if matches!(cs[j], '.' | ',') && digit(j + 1) {
                seps.push(cs[j]);
                groups.push(String::new());
                j += 1;
            } else if matches!(cs[j], ' ' | '\u{a0}' | '\u{202f}')
                && digit(j + 1)
                && digit(j + 2)
                && digit(j + 3)
                && !digit(j + 4)
                && groups[0].len() <= 3
                && groups.iter().skip(1).all(|g| g.len() == 3)
            {
                seps.push(' ');
                groups.push(String::new());
                j += 1;
            } else {
                break;
            }
        }
        let triples = groups.len() > 1
            && groups[0].len() <= 3
            && groups[1..].iter().all(|g| g.len() == 3)
            && seps.iter().all(|c| *c == seps[0]);
        let thousands = triples && (seps[0] == ' ' || seps[0] == ',' || groups.len() > 2);
        let n = if thousands || groups.len() == 1 {
            groups.concat()
        } else {
            format!(
                "{}.{}",
                groups[..groups.len() - 1].concat(),
                groups[groups.len() - 1]
            )
        };
        out.push(n);
        i = j.max(i + 1);
    }
    out
}

pub const SUPPORTED: f32 = 0.6;
pub const WEAK: f32 = 0.35;

pub fn audit(store: &Store, bib: &BibMap, text: &str) -> Vec<Finding> {
    let text = &normalize_markup(text);
    let body: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let (marked, groups) = extract_cites(&body);
    let mut norm_docs: HashMap<usize, String> = HashMap::new();
    let mut doc_text = |d: usize| -> String {
        norm_docs
            .entry(d)
            .or_insert_with(|| {
                norm(
                    &store
                        .chunks
                        .iter()
                        .filter(|c| c.doc == d)
                        .map(|c| c.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                )
            })
            .clone()
    };
    let mut out = Vec::new();
    let mut gi = 0usize;
    for s in sentences(&marked) {
        let mut cites: Vec<Cite> = Vec::new();
        for ch in s.chars() {
            if ch == '\u{1}' {
                if let Some(g) = groups.get(gi) {
                    cites.extend(g.iter().cloned());
                }
                gi += 1;
            }
        }
        let plain: String = s
            .replace('\u{1}', "")
            .replace(" .", ".")
            .replace(" ,", ",")
            .replace(" ;", ";")
            .trim()
            .to_string();
        let claim = plain.clone();
        // quotations: must exist word-for-word
        for q in quotes(&plain) {
            let nq = norm(&q);
            let cited_docs: Vec<usize> = cites
                .iter()
                .filter_map(|c| doc_for_key(store, bib, &c.key))
                .collect();
            let in_cited = cited_docs.iter().any(|&d| doc_text(d).contains(&nq));
            let (verdict, note) = if in_cited {
                (
                    "quote-verbatim",
                    "found word-for-word in the cited source".to_string(),
                )
            } else if let Some(d) = (0..store.docs.len()).find(|&d| doc_text(d).contains(&nq)) {
                (
                    "quote-elsewhere",
                    format!(
                        "not in the cited source, but found in {}",
                        store.docs[d].title
                    ),
                )
            } else {
                (
                    "quote-not-found",
                    "not found word-for-word anywhere in the library".to_string(),
                )
            };
            out.push(Finding {
                sentence: q,
                cite: cites.first().map(|c| c.key.clone()),
                page: cites.first().and_then(|c| c.page),
                verdict,
                support: if in_cited { 1.0 } else { 0.0 },
                note,
                passage: None,
            });
        }
        if cites.is_empty() {
            if plain.split_whitespace().count() >= 14 && !plain.contains('?') {
                out.push(Finding {
                    sentence: plain,
                    cite: None,
                    page: None,
                    verdict: "uncited",
                    support: 0.0,
                    note: "a long statement with no citation".into(),
                    passage: None,
                });
            }
            continue;
        }
        for c in &cites {
            let f = |verdict, support, note: String| Finding {
                sentence: claim.clone(),
                cite: Some(c.key.clone()),
                page: c.page,
                verdict,
                support,
                note,
                passage: None,
            };
            let Some(d) = doc_for_key(store, bib, &c.key) else {
                out.push(f(
                    "unknown-source",
                    0.0,
                    format!("no document in the library matches the key '{}'", c.key),
                ));
                continue;
            };
            let doc = &store.docs[d];
            if let Some(p) = c.page
                && doc.pages > 0
                && p > doc.pages
            {
                out.push(f(
                    "page-out-of-range",
                    0.0,
                    format!("{} has {} pages", doc.title, doc.pages),
                ));
                continue;
            }
            // Text, EPUB and DOCX sources have no page numbers: a cited page cannot be checked there.
            let paged = store.chunks.iter().any(|ch| ch.doc == d && ch.page > 0);
            let cpage = c.page.filter(|&p| p > 0 && paged);
            let best = |only_page: Option<usize>| -> (f32, usize) {
                store
                    .chunks
                    .iter()
                    .filter(|ch| ch.doc == d && only_page.map(|p| ch.page == p).unwrap_or(true))
                    .map(|ch| (coverage(&claim, &ch.text), ch.page))
                    .fold((0.0, 0), |a, b| if b.0 > a.0 { b } else { a })
            };
            let (sup, _) = best(cpage);
            // the passage the sentence matches best (on the cited page if given): used for the figure check
            let top = store
                .chunks
                .iter()
                .filter(|ch| ch.doc == d && cpage.map(|p| ch.page == p).unwrap_or(true))
                .max_by(|a, b| {
                    coverage(&claim, &a.text)
                        .partial_cmp(&coverage(&claim, &b.text))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|ch| ch.text.as_str());
            let (any, anyp) = best(None);
            let (verdict, note) = if sup >= SUPPORTED {
                (
                    "supported",
                    format!(
                        "{:.0}% of the sentence's terms appear in the cited passage",
                        sup * 100.0
                    ),
                )
            } else if cpage.is_some() && any >= SUPPORTED && Some(anyp) != cpage {
                (
                    "wrong-page",
                    format!(
                        "the cited page covers {:.0}%, but p. {} in the same source covers {:.0}% — probably p. {}",
                        sup * 100.0,
                        anyp,
                        any * 100.0,
                        anyp
                    ),
                )
            } else if sup >= WEAK {
                (
                    "weak",
                    format!(
                        "only {:.0}% of the sentence's terms appear in the cited passage",
                        sup * 100.0
                    ),
                )
            } else if any >= WEAK {
                (
                    "weak",
                    format!(
                        "the cited passage covers {:.0}%; the best match in the source is {:.0}% (p. {})",
                        sup * 100.0,
                        any * 100.0,
                        anyp
                    ),
                )
            } else {
                (
                    "unsupported",
                    format!(
                        "only {:.0}% of the sentence's terms appear anywhere in this source",
                        any * 100.0
                    ),
                )
            };
            if matches!(verdict, "supported" | "weak")
                && let Some(t) = top
            {
                let m = missing_numbers(&claim, t);
                if !m.is_empty() {
                    out.push(f(
                        "number-mismatch",
                        sup,
                        format!(
                            "the sentence states {} but the cited passage does not contain {}",
                            m.join(", "),
                            if m.len() == 1 { "it" } else { "them" }
                        ),
                    ));
                    continue;
                }
            }
            let mut fin = f(verdict, sup, note);
            fin.passage = top.map(|t| t.chars().take(1500).collect());
            out.push(fin);
        }
    }
    out
}

// ------------------------------------------------------------ semantic pass

/// What a local model says about one sentence and its passage.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Sem {
    Supports,
    Partial,
    Contradicts,
    Unrelated,
    Unclear,
}

pub const SYSTEM_SEMANTIC: &str = "You are a strict citation checker. You get ONE passage from a source and ONE sentence that cites it. \
Judge only from the passage. Answer with exactly one word: SUPPORTS (the passage says what the sentence says, even in other words), \
PARTIAL (only part of the sentence is in the passage), CONTRADICTS (the passage says the opposite or different facts) or UNRELATED.";

pub fn parse_sem(text: &str) -> Sem {
    let t = text.trim().to_uppercase();
    let first: String = t
        .chars()
        .skip_while(|c| !c.is_alphabetic())
        .take_while(|c| c.is_alphabetic())
        .collect();
    match first.as_str() {
        "SUPPORTS" | "SUPPORTED" => Sem::Supports,
        "PARTIAL" | "PARTIALLY" => Sem::Partial,
        "CONTRADICTS" | "CONTRADICTED" | "CONTRADICTION" => Sem::Contradicts,
        "UNRELATED" | "NOT" | "UNSUPPORTED" => Sem::Unrelated,
        _ => Sem::Unclear,
    }
}

/// Second opinion from a model on every cited sentence that has a passage. It can rescue a correct paraphrase (weak/unsupported → supported),
/// catch a negation that term overlap accepts (supported → contradicted), and flag overlap without meaning (supported → weak).
/// Quotations, unknown sources and page errors are left alone: they are decided by exact checks. The model can be wrong; every
/// changed finding says so in its note. `judge(sentence, passage)` returns None when the model fails (that finding is left as it was).
pub fn semantic_pass(
    findings: &mut [Finding],
    judge: &mut dyn FnMut(&str, &str) -> Option<Sem>,
) -> (usize, usize) {
    let (mut checked, mut changed) = (0, 0);
    for f in findings.iter_mut() {
        if !matches!(
            f.verdict,
            "supported" | "weak" | "unsupported" | "number-mismatch"
        ) {
            continue;
        }
        let Some(p) = f.passage.clone() else { continue };
        let Some(sem) = judge(&f.sentence, &p) else {
            continue;
        };
        checked += 1;
        let before = f.verdict;
        match sem {
            Sem::Supports => {
                if before != "supported" {
                    f.verdict = "supported";
                    f.note = format!(
                        "[model] paraphrase confirmed by the local model (term overlap was {:.0}%) — verify by reading",
                        f.support * 100.0
                    );
                } else {
                    f.note.push_str(" · [model] agrees");
                }
            }
            Sem::Contradicts => {
                f.verdict = "contradicted";
                f.note = format!(
                    "[model] the passage seems to say something different or the opposite (term overlap was {:.0}%)",
                    f.support * 100.0
                );
            }
            Sem::Partial | Sem::Unrelated => {
                if before == "supported" {
                    f.verdict = "weak";
                    f.note = format!(
                        "[model] words overlap ({:.0}%) but the model finds the passage {} the sentence",
                        f.support * 100.0,
                        if sem == Sem::Partial {
                            "only partly supporting"
                        } else {
                            "unrelated to"
                        }
                    );
                } else {
                    f.note.push_str(if sem == Sem::Partial {
                        " · [model] partial support"
                    } else {
                        " · [model] finds the passage unrelated"
                    });
                }
            }
            Sem::Unclear => {}
        }
        if f.verdict != before {
            changed += 1;
        }
    }
    (checked, changed)
}

pub fn audit_markdown(f: &[Finding]) -> String {
    let n = |v: &str| f.iter().filter(|x| x.verdict == v).count();
    let mut s = format!(
        "# Citation audit\n\n{} checks — supported {}, weak {}, contradicted {}, unsupported {}, wrong page {}, figure mismatch {}, unknown source {}, page out of range {}, uncited {}; quotations: verbatim {}, elsewhere {}, not found {}.\n\nThis is a mechanical term-overlap check, not proof. Open the page for everything below.\n\n",
        f.len(),
        n("supported"),
        n("weak"),
        n("contradicted"),
        n("unsupported"),
        n("wrong-page"),
        n("number-mismatch"),
        n("unknown-source"),
        n("page-out-of-range"),
        n("uncited"),
        n("quote-verbatim"),
        n("quote-elsewhere"),
        n("quote-not-found")
    );
    for x in f.iter().filter(|x| x.is_problem()) {
        let c = x
            .cite
            .as_deref()
            .map(|c| {
                format!(
                    " [@{}{}]",
                    c,
                    x.page.map(|p| format!(", p. {p}")).unwrap_or_default()
                )
            })
            .unwrap_or_default();
        s.push_str(&format!(
            "- **{}**{} — {}\n  > {}\n",
            x.verdict, c, x.note, x.sentence
        ));
    }
    if !f.iter().any(|x| x.is_problem()) {
        s.push_str("No problems found by this check.\n");
    }
    s
}

// ---------------------------------------------------------- reference hygiene

/// Cited keys that resolve to nothing, bibliography entries never cited, library documents never cited. Pure bookkeeping, no model.
pub fn refs_report(store: &Store, bib: &BibMap, text: &str) -> Value {
    let text = normalize_markup(text);
    let body: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let (_, groups) = extract_cites(&body);
    let mut cited: Vec<String> = groups.iter().flatten().map(|c| c.key.clone()).collect();
    cited.sort_by_key(|k| k.to_lowercase());
    cited.dedup_by_key(|k| k.to_lowercase());
    let lc: std::collections::HashSet<String> = cited.iter().map(|k| k.to_lowercase()).collect();
    let missing: Vec<&String> = cited
        .iter()
        .filter(|k| {
            !bib.entries.iter().any(|e| e.key.eq_ignore_ascii_case(k))
                && doc_for_key(store, bib, k).is_none()
        })
        .collect();
    let no_file: Vec<&String> = cited
        .iter()
        .filter(|k| {
            bib.entries.iter().any(|e| e.key.eq_ignore_ascii_case(k))
                && doc_for_key(store, bib, k).is_none()
        })
        .collect();
    let unused_bib: Vec<&str> = bib
        .entries
        .iter()
        .filter(|e| !lc.contains(&e.key.to_lowercase()))
        .map(|e| e.key.as_str())
        .collect();
    let unused_docs: Vec<&str> = store
        .docs
        .iter()
        .filter(|d| {
            let stem = std::path::Path::new(&d.path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let key = bib.find(&d.path).map(|e| e.key.to_lowercase());
            !lc.contains(&stem) && !key.map(|k| lc.contains(&k)).unwrap_or(false)
        })
        .map(|d| d.title.as_str())
        .collect();
    json!({"cited": cited, "missing_everywhere": missing, "in_bib_but_not_in_library": no_file,
           "bib_never_cited": unused_bib, "library_never_cited": unused_docs})
}

pub fn refs_markdown(v: &Value) -> String {
    let list = |k: &str| {
        v[k].as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let mut s = format!(
        "# Reference check\n\n{} distinct keys cited.\n\n",
        list("cited").len()
    );
    for (k, label) in [
        (
            "missing_everywhere",
            "Cited but in neither the .bib nor the library",
        ),
        (
            "in_bib_but_not_in_library",
            "Cited, in the .bib, but no file is indexed (nothing to audit against)",
        ),
        (
            "bib_never_cited",
            "In the .bib but never cited in this text",
        ),
        (
            "library_never_cited",
            "Indexed but never cited in this text",
        ),
    ] {
        let l = list(k);
        if l.is_empty() {
            continue;
        }
        s.push_str(&format!("## {label} ({})\n", l.len()));
        for x in l {
            s.push_str(&format!("- {x}\n"));
        }
        s.push('\n');
    }
    s
}

// ------------------------------------------------------------------- coverage

pub fn outline_headings(text: &str) -> Vec<String> {
    let strip = |l: &str| {
        l.trim_start_matches(|c: char| {
            c == '#'
                || c == '-'
                || c == '*'
                || c == '•'
                || c == '.'
                || c == ')'
                || c.is_ascii_digit()
                || c == ' '
        })
        .trim()
        .to_string()
    };
    let hashes: Vec<String> = text
        .lines()
        .filter(|l| l.trim_start().starts_with('#'))
        .map(strip)
        .filter(|s| !s.is_empty())
        .collect();
    if !hashes.is_empty() {
        return hashes;
    }
    text.lines().map(strip).filter(|s| !s.is_empty()).collect()
}

pub fn coverage_matrix(
    store: &Store,
    bib: &BibMap,
    headings: &[String],
    emb: &dyn Fn(&str) -> Option<Vec<f32>>,
) -> Value {
    let searcher = Searcher::new(store);
    let mut rows = Vec::new();
    for h in headings {
        let e = emb(h);
        let hits = searcher.search(h, e.as_deref(), 12, 3, None);
        let mut docs: Vec<(usize, f32)> = Vec::new();
        for x in &hits {
            let c = &store.chunks[x.chunk];
            let cv = coverage(h, &c.text);
            match docs.iter_mut().find(|(d, _)| *d == c.doc) {
                Some(slot) => slot.1 = slot.1.max(cv),
                None => docs.push((c.doc, cv)),
            }
        }
        let good: Vec<&(usize, f32)> = docs.iter().filter(|(_, cv)| *cv >= WEAK).collect();
        let best = docs.iter().map(|d| d.1).fold(0.0, f32::max);
        let status = if good.len() >= 3 && best >= 0.5 {
            "well-covered"
        } else if !good.is_empty() {
            "thin"
        } else {
            "empty"
        };
        let names: Vec<Value> = good
            .iter()
            .map(|(d, cv)| {
                let doc = &store.docs[*d];
                json!({"doc": bib.find(&doc.path).map(|e| e.cite()).unwrap_or_else(|| doc.title.clone()), "path": doc.path, "cov": (cv * 100.0).round() / 100.0})
            })
            .collect();
        rows.push(json!({"heading": h, "status": status, "sources": good.len(), "best": (best * 100.0).round() / 100.0, "docs": names}));
    }
    let count = |s: &str| rows.iter().filter(|r| r["status"] == s).count();
    json!({"rows": rows, "summary": {"well-covered": count("well-covered"), "thin": count("thin"), "empty": count("empty")}})
}

pub fn coverage_markdown(v: &Value) -> String {
    let mut s = String::from(
        "# Source coverage of the outline\n\n| Section | Status | Sources | Best match | Which |\n|---|---|---|---|---|\n",
    );
    for r in v["rows"].as_array().into_iter().flatten() {
        let which: Vec<String> = r["docs"]
            .as_array()
            .into_iter()
            .flatten()
            .take(4)
            .map(|d| d["doc"].as_str().unwrap_or("").to_string())
            .collect();
        s.push_str(&format!(
            "| {} | {} | {} | {}% | {} |\n",
            r["heading"].as_str().unwrap_or(""),
            r["status"].as_str().unwrap_or(""),
            r["sources"],
            (r["best"].as_f64().unwrap_or(0.0) * 100.0).round(),
            which.join("; ")
        ));
    }
    s.push_str("\nStatus is by term overlap with your indexed sources: 'empty' means nothing in the library matches the heading's words; it does not prove the topic is absent from the literature.\n");
    s
}

// -------------------------------------------------------------------- novelty

pub fn novelty(store: &Store, bib: &BibMap, text: &str) -> Value {
    let searcher = Searcher::new(store);
    let body: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let wins = crate::ingest::chunk_words(&extract_cites(&body).0.replace('\u{1}', ""), 60, 0);
    let mut passages = Vec::new();
    let (mut covered, mut partial, mut new) = (0, 0, 0);
    for w in &wins {
        let hits = searcher.search(w, None, 3, 3, None);
        let (cv, near) = hits
            .iter()
            .map(|h| (coverage(w, &store.chunks[h.chunk].text), h.chunk))
            .fold(
                (0.0f32, None),
                |a, b| if b.0 > a.0 { (b.0, Some(b.1)) } else { a },
            );
        let status = if cv >= 0.65 {
            covered += 1;
            "covered"
        } else if cv >= 0.4 {
            partial += 1;
            "partial"
        } else {
            new += 1;
            "new"
        };
        let nearest = near.map(|c| {
            let d = &store.docs[store.chunks[c].doc];
            format!(
                "{}{}",
                bib.find(&d.path)
                    .map(|e| e.cite())
                    .unwrap_or_else(|| d.title.clone()),
                if store.chunks[c].page > 0 {
                    format!(", p. {}", store.chunks[c].page)
                } else {
                    String::new()
                }
            )
        });
        passages.push(json!({"text": w, "status": status, "overlap": (cv * 100.0).round() / 100.0, "nearest": nearest}));
    }
    let n = wins.len().max(1) as f32;
    json!({"passages": passages, "covered": covered, "partial": partial, "new": new, "share_new": (new as f32 / n * 100.0).round()})
}

pub fn novelty_markdown(v: &Value) -> String {
    let mut s = format!(
        "# What this text adds to your library\n\n{} passages: {} already covered, {} partly covered, {} new ({}%).\n\n'New' means your indexed sources share few of the passage's terms. It says nothing about the wider literature.\n\n",
        v["passages"].as_array().map(|a| a.len()).unwrap_or(0),
        v["covered"],
        v["partial"],
        v["new"],
        (v["share_new"].as_f64().unwrap_or(0.0)).round()
    );
    for p in v["passages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["status"] == "new")
    {
        let t: String = p["text"].as_str().unwrap_or("").chars().take(160).collect();
        s.push_str(&format!(
            "- {}…  (nearest: {})\n",
            t,
            p["nearest"].as_str().unwrap_or("none")
        ));
    }
    s
}

// -------------------------------------------------------------------- receipts

pub fn index_fingerprint(store: &Store) -> String {
    let mut h: Vec<String> = store
        .docs
        .iter()
        .map(|d| format!("{:016x}", d.hash))
        .collect();
    h.sort();
    sha256_hex(h.join(",").as_bytes())
}

fn seal(mut body: Value) -> Value {
    let digest = sha256_hex(serde_json::to_string(&body).unwrap().as_bytes());
    body["digest"] = json!(digest);
    if let Ok(k) = std::env::var("TEZAUR_RECEIPT_KEY")
        && !k.is_empty()
    {
        body["hmac"] = json!(hex(&hmac_sha256(k.as_bytes(), digest.as_bytes())));
    }
    body
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn make_receipt(
    store: &Store,
    model: &str,
    mode: &str,
    question: &str,
    hit_chunks: &[usize],
    answer: &str,
) -> Value {
    let ex: Vec<Value> = hit_chunks
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let ch = &store.chunks[c];
            let d = &store.docs[ch.doc];
            json!({"n": i + 1, "path": d.path, "title": d.title, "doc_hash": format!("{:016x}", d.hash), "page": ch.page, "chunk_sha256": sha256_hex(ch.text.as_bytes())})
        })
        .collect();
    seal(json!({
        "format": "tezaur-receipt/1", "created": now_secs(), "tool": format!("tezaur {}", env!("CARGO_PKG_VERSION")),
        "model": model, "mode": mode, "question": question, "answer_sha256": sha256_hex(answer.as_bytes()),
        "index": index_fingerprint(store), "excerpts": ex,
    }))
}

pub fn verify_receipt(
    store: Option<&Store>,
    receipt: &Value,
    answer: Option<&str>,
) -> (bool, Vec<String>) {
    let mut lines = Vec::new();
    let mut ok = true;
    let mut body = receipt.clone();
    let digest = body
        .as_object_mut()
        .and_then(|o| o.remove("digest"))
        .and_then(|d| d.as_str().map(|s| s.to_string()));
    let hm = body
        .as_object_mut()
        .and_then(|o| o.remove("hmac"))
        .and_then(|d| d.as_str().map(|s| s.to_string()));
    let calc = sha256_hex(serde_json::to_string(&body).unwrap().as_bytes());
    if digest.as_deref() == Some(calc.as_str()) {
        lines.push("✓ receipt is intact (digest matches its contents)".into());
    } else {
        ok = false;
        lines.push("✗ receipt was altered: digest does not match its contents".into());
    }
    if let Some(h) = hm {
        match std::env::var("TEZAUR_RECEIPT_KEY") {
            Ok(k) if !k.is_empty() => {
                if hex(&hmac_sha256(k.as_bytes(), calc.as_bytes())) == h {
                    lines.push("✓ HMAC matches your TEZAUR_RECEIPT_KEY".into());
                } else {
                    ok = false;
                    lines.push("✗ HMAC does not match TEZAUR_RECEIPT_KEY".into());
                }
            }
            _ => lines.push(
                "• receipt carries an HMAC, but TEZAUR_RECEIPT_KEY is not set — not checked".into(),
            ),
        }
    }
    if let Some(a) = answer {
        if sha256_hex(a.as_bytes()) == body["answer_sha256"].as_str().unwrap_or("") {
            lines.push("✓ the answer text is exactly the one this receipt was issued for".into());
        } else {
            ok = false;
            lines.push("✗ the answer text differs from the one this receipt was issued for".into());
        }
    }
    if let Some(st) = store {
        let (mut found, mut total) = (0, 0);
        for e in body["excerpts"].as_array().into_iter().flatten() {
            total += 1;
            let (p, pg, h) = (
                e["path"].as_str().unwrap_or(""),
                e["page"].as_u64().unwrap_or(0) as usize,
                e["chunk_sha256"].as_str().unwrap_or(""),
            );
            if st.chunks.iter().any(|c| {
                c.page == pg && st.docs[c.doc].path == p && sha256_hex(c.text.as_bytes()) == h
            }) {
                found += 1;
            } else {
                lines.push(format!(
                    "✗ excerpt {} ({}, p. {}) is not in this index as recorded",
                    e["n"], p, pg
                ));
            }
        }
        if found == total {
            lines.push(format!(
                "✓ all {total} excerpts exist unchanged in the current index"
            ));
        } else {
            ok = false;
        }
    }
    (ok, lines)
}

fn civil(secs: u64) -> String {
    let z = (secs / 86400) as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

/// A plain statement of AI use, assembled from receipts. The author must confirm the checking claim.
pub fn declaration(receipts: &[Value], ro: bool) -> String {
    let mut models: Vec<String> = receipts
        .iter()
        .filter_map(|r| r["model"].as_str().map(|s| s.to_string()))
        .collect();
    models.sort();
    models.dedup();
    let mut docs: Vec<String> = receipts
        .iter()
        .flat_map(|r| r["excerpts"].as_array().cloned().unwrap_or_default())
        .filter_map(|e| e["path"].as_str().map(|s| s.to_string()))
        .collect();
    docs.sort();
    docs.dedup();
    let (mut lo, mut hi) = (u64::MAX, 0u64);
    for r in receipts {
        let t = r["created"].as_u64().unwrap_or(0);
        lo = lo.min(t);
        hi = hi.max(t);
    }
    let period = if receipts.is_empty() {
        String::new()
    } else if civil(lo) == civil(hi) {
        civil(lo)
    } else {
        format!("{} – {}", civil(lo), civil(hi))
    };
    let mut s = if ro {
        format!(
            "# Declarație privind utilizarea instrumentelor de inteligență artificială\n\nÎn elaborarea acestei lucrări am folosit TEZAUR, un instrument local care caută în sursele proprii și formulează răspunsuri cu un model de limbaj rulat pe calculatorul meu ({}). Perioada: {}. Interogări înregistrate: {}. Răspunsurile s-au bazat exclusiv pe pasaje din {} documente din biblioteca proprie; nu a fost trimis niciun text către servicii externe.\n\n[AUTOR: confirmați că ați verificat personal fiecare citare în sursă și că argumentația și textul final vă aparțin.]\n\nChitanțe (rezumat SHA-256):\n",
            models.join(", "),
            period,
            receipts.len(),
            docs.len()
        )
    } else {
        format!(
            "# AI-use statement\n\nWhile preparing this work I used TEZAUR, a local tool that searches my own sources and drafts answers with a language model running on my own computer ({}). Period: {}. Logged queries: {}. Answers were grounded only in passages from {} documents of my own library; no text was sent to an external service.\n\n[AUTHOR: confirm that you personally checked every citation against its source and that the argument and final text are your own.]\n\nReceipts (SHA-256 digests):\n",
            models.join(", "),
            period,
            receipts.len(),
            docs.len()
        )
    };
    for r in receipts {
        s.push_str(&format!(
            "- {} — {}\n",
            civil(r["created"].as_u64().unwrap_or(0)),
            r["digest"].as_str().unwrap_or("")
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{Chunk, Doc};

    fn st() -> Store {
        let mut s = Store::default();
        s.docs.push(Doc {
            path: "Library/smith2019.txt".into(),
            title: "smith2019".into(),
            hash: 7,
            pages: 10,
        });
        s.docs.push(Doc {
            path: "Library/doe2020.txt".into(),
            title: "doe2020".into(),
            hash: 9,
            pages: 4,
        });
        let t = |d, p, x: &str| Chunk {
            doc: d,
            page: p,
            text: x.into(),
            emb: None,
        };
        s.chunks.push(t(0, 3, "The burden of proof lies with the competent authority when it refuses recognition of the diploma."));
        s.chunks.push(t(0, 5, "Compensation measures include an aptitude test or an adaptation period for the migrant professional."));
        s.chunks.push(t(
            1,
            2,
            "Mutual trust between member states underpins automatic recognition of qualifications.",
        ));
        s
    }

    #[test]
    fn parses_pandoc_cites() {
        let (m, g) =
            extract_cites("A claim [@smith2019, p. 3; @doe2020]. Not a cite [1] or [see here].");
        assert_eq!(m.matches('\u{1}').count(), 1);
        assert_eq!(
            g[0],
            vec![
                Cite {
                    key: "smith2019".into(),
                    page: Some(3)
                },
                Cite {
                    key: "doe2020".into(),
                    page: None
                }
            ]
        );
    }

    #[test]
    fn splits_sentences_keeping_late_markers() {
        let (m, _) = extract_cites(
            "First claim here. [@a] Second one, see Smith et al. for more [@b]. Third.",
        );
        let s = sentences(&m);
        assert_eq!(s.len(), 3, "{s:?}");
        assert!(s[0].contains('\u{1}') && s[1].contains('\u{1}') && !s[2].contains('\u{1}'));
    }

    #[test]
    fn latex_and_lyx_input_become_pandoc_cites() {
        let tex = "\\documentclass{article}\n\\begin{document}\n\\section{Intro}\nThe \\emph{burden of proof} lies with the authority \\parencite[p.~3]{smith2019}. % comment \\cite{x}\nSee \\citep{doe2020,smith2019}.\n\\end{document}";
        let m = normalize_markup(tex);
        assert!(m.contains("# Intro"), "{m}");
        assert!(m.contains("[@smith2019, p. 3]"), "{m}");
        assert!(m.contains("[@doe2020; @smith2019]"), "{m}");
        assert!(!m.contains("comment") && !m.contains("\\emph"), "{m}");
        let lyx = "#LyX 2.3\n\\begin_header\n\\end_header\n\n\\begin_body\n\n\\begin_layout Section\nIntro\n\\end_layout\n\n\\begin_layout Standard\nThe burden of proof lies with the authority\n\\begin_inset CommandInset citation\nLatexCommand cite\nafter \"p. 3\"\nkey \"smith2019\"\nliteral \"false\"\n\n\\end_inset\n\n.\n\\end_layout\n\n\\end_body";
        let m = normalize_markup(lyx);
        assert!(
            m.contains("# Intro") && m.contains("[@smith2019, p. 3]"),
            "{m}"
        );
        let f = audit(&st(), &BibMap::default(), tex);
        assert!(
            f.iter()
                .any(|x| x.verdict == "supported" || x.verdict == "weak"),
            "{f:?}"
        );
    }

    #[test]
    fn swapped_figures_are_caught() {
        let store = st();
        let f = audit(
            &store,
            &BibMap::default(),
            "The burden of proof lies with the competent authority when it refuses recognition within ten days [@smith2019, p. 3].",
        );
        assert!(f.iter().any(|x| x.verdict == "number-mismatch"), "{f:?}");
        let g = audit(
            &store,
            &BibMap::default(),
            "The burden of proof lies with the competent authority when it refuses recognition [@smith2019, p. 3].",
        );
        assert!(g.iter().all(|x| x.verdict == "supported"), "{g:?}");
    }

    #[test]
    fn audit_verdicts() {
        let store = st();
        let bib = BibMap::default();
        let txt = "The burden of proof lies with the competent authority when it refuses recognition [@smith2019, p. 3]. \
                   The burden of proof lies with the competent authority when it refuses recognition [@smith2019, p. 5]. \
                   Penguins migrate across the frozen southern ocean every winter [@smith2019, p. 3]. \
                   Mutual trust underpins recognition [@nobody]. Anything [@doe2020, p. 99]. \
                   He wrote \"an aptitude test or an adaptation period\" [@smith2019, p. 5]. \
                   He wrote \"this sentence never appears in any book here\" [@smith2019].";
        let f = audit(&store, &bib, txt);
        let v: Vec<&str> = f.iter().map(|x| x.verdict).collect();
        assert!(v.contains(&"supported"), "{v:?}");
        assert!(v.contains(&"wrong-page"), "{v:?}");
        assert!(v.contains(&"unsupported"), "{v:?}");
        assert!(v.contains(&"unknown-source"), "{v:?}");
        assert!(v.contains(&"page-out-of-range"), "{v:?}");
        assert!(
            v.contains(&"quote-verbatim") && v.contains(&"quote-not-found"),
            "{v:?}"
        );
    }

    #[test]
    fn coverage_and_novelty() {
        let store = st();
        let bib = BibMap::default();
        let c = coverage_matrix(
            &store,
            &bib,
            &[
                "burden of proof".into(),
                "quantum chromodynamics lattice".into(),
            ],
            &|_| None,
        );
        assert_eq!(c["rows"][0]["status"], "thin");
        assert_eq!(c["rows"][1]["status"], "empty");
        let n = novelty(
            &store,
            &bib,
            "The burden of proof lies with the competent authority when it refuses recognition of the diploma. Penguins migrate across the frozen southern ocean every winter and nobody here has written about it at all.",
        );
        assert!(n["covered"].as_u64().unwrap() >= 1 || n["partial"].as_u64().unwrap() >= 1);
        assert!(
            n["new"].as_u64().unwrap()
                + n["covered"].as_u64().unwrap()
                + n["partial"].as_u64().unwrap()
                >= 1
        );
    }

    #[test]
    fn receipts_detect_tampering() {
        let store = st();
        let r = make_receipt(&store, "m", "ask", "q?", &[0, 2], "answer [1]");
        let (ok, l) = verify_receipt(Some(&store), &r, Some("answer [1]"));
        assert!(ok, "{l:?}");
        let (ok2, _) = verify_receipt(Some(&store), &r, Some("answer [1] edited"));
        assert!(!ok2);
        let mut forged = r.clone();
        forged["question"] = json!("other");
        assert!(!verify_receipt(None, &forged, None).0);
        let mut changed = st();
        changed.chunks[0].text.push_str(" changed");
        assert!(!verify_receipt(Some(&changed), &r, None).0);
        assert_eq!(civil(0), "1970-01-01");
        assert_eq!(civil(1_800_000_000), "2027-01-15");
        assert!(declaration(&[r], false).contains("AI-use statement"));
    }

    #[test]
    fn semantic_pass_rescues_and_catches() {
        let mk = |v: &'static str, sup: f32| Finding {
            sentence: "S".into(),
            cite: Some("k".into()),
            page: None,
            verdict: v,
            support: sup,
            note: String::new(),
            passage: Some("P".into()),
        };
        let mut fs = vec![
            mk("weak", 0.4),
            mk("supported", 0.9),
            mk("supported", 0.8),
            mk("unsupported", 0.1),
            mk("unknown-source", 0.0),
        ];
        let answers = [Sem::Supports, Sem::Contradicts, Sem::Partial, Sem::Unclear];
        let mut i = 0;
        let (checked, changed) = semantic_pass(&mut fs, &mut |_, _| {
            i += 1;
            Some(answers[i - 1])
        });
        assert_eq!((checked, changed), (4, 3));
        assert_eq!(fs[0].verdict, "supported");
        assert_eq!(fs[1].verdict, "contradicted");
        assert_eq!(fs[2].verdict, "weak");
        assert_eq!(fs[3].verdict, "unsupported");
        assert_eq!(fs[4].verdict, "unknown-source");
        assert!(fs[0].note.contains("[model]"));
        assert_eq!(parse_sem("Contradicts."), Sem::Contradicts);
        assert_eq!(parse_sem("SUPPORTS because"), Sem::Supports);
        assert_eq!(parse_sem("hmm"), Sem::Unclear);
    }
}

#[cfg(test)]
mod figure_tests {
    #[test]
    fn compound_number_words_are_read_by_value() {
        let v = |s: &str| number_words(s).into_iter().map(|x| x.1).collect::<Vec<_>>();
        assert_eq!(v("twenty-five hours"), ["25"]);
        assert_eq!(v("four hundred and twelve adults"), ["412"]);
        assert_eq!(v("one thousand five hundred eggs"), ["1500"]);
        assert_eq!(v("three and a half percent"), ["3.5"]);
        assert_eq!(v("one point two percent"), ["1.2"]);
        assert_eq!(v("douazeci si cinci de ore"), ["25"]);
        assert_eq!(v("two and the rest"), ["2"]);
        assert!(missing_numbers("It took twenty five hours.", "It took 25 hours.").is_empty());
        assert!(!missing_numbers("It took twenty six hours.", "It took 25 hours.").is_empty());
    }

    #[test]
    fn digits_and_number_words_are_the_same_figure() {
        let p = "The authorities shall respond within four months. The period shall not exceed three years.";
        assert!(
            super::missing_numbers("They respond within 4 months and 3 years at most.", p)
                .is_empty()
        );
        assert!(!super::missing_numbers("They respond within 5 months.", p).is_empty());
        assert!(!super::missing_numbers("The period is 3.5 years.", p).is_empty());
    }

    use super::*;
    #[test]
    fn figures_keep_decimals_and_ignore_thousands_separators() {
        assert_eq!(figures("took 71.3 days"), vec!["71.3"]);
        assert_eq!(
            figures("1,240 and 1240 and 1 240"),
            vec!["1240", "1240", "1240"]
        );
        assert_eq!(figures("a rate of 0,91 in 2024."), vec!["0.91", "2024"]);
    }
    #[test]
    fn reordered_digits_are_a_mismatch() {
        assert_eq!(
            missing_numbers("The time was 3.71 days", "The time was 71.3 days"),
            vec!["3.71"]
        );
        assert!(missing_numbers("The time was 71.3 days.", "mean time 71.3 days").is_empty());
        assert!(missing_numbers("There were 1,240 cases", "1240 applications").is_empty());
    }
}
