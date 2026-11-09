//! Full BibTeX file reader (spec 03): `@string` abbreviations, `#` concatenation, month names,
//! entry types, `crossref` inheritance, warnings instead of errors. Produces raw entries (field
//! name -> resolved but not yet LaTeX-cleaned text); `bib.rs` turns them into `Entry` values.

use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct RawEntry {
    pub key: String,
    /// Entry type as written, lower case.
    pub typ: String,
    /// Field name (lower case) -> resolved text, braces and LaTeX intact. Order of appearance.
    pub fields: Vec<(String, String)>,
}

impl RawEntry {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Default)]
pub struct RawFile {
    pub entries: Vec<RawEntry>,
    pub warnings: Vec<String>,
}

enum Part {
    Lit(String),
    Name(String),
}

const MONTHS: [(&str, &str); 12] = [
    ("jan", "January"),
    ("feb", "February"),
    ("mar", "March"),
    ("apr", "April"),
    ("may", "May"),
    ("jun", "June"),
    ("jul", "July"),
    ("aug", "August"),
    ("sep", "September"),
    ("oct", "October"),
    ("nov", "November"),
    ("dec", "December"),
];

/// An entry whose values wait until every @string is known: (type, key, [(field, parts)]).
type Pending = (String, String, Vec<(String, Vec<Part>)>);

fn skip_ws(c: &[char], i: &mut usize) {
    while *i < c.len() && c[*i].is_whitespace() {
        *i += 1;
    }
}

/// Index just past the group opened at `c[open]` (`{` or `(`); None if unbalanced.
fn group_end(c: &[char], open: usize) -> Option<usize> {
    let paren = c[open] == '(';
    let mut depth = 0usize;
    let mut quoted = false;
    let mut i = open + 1;
    while i < c.len() {
        match c[i] {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            '}' if !paren => return Some(i + 1),
            '"' if paren && depth == 0 => quoted = !quoted,
            ')' if paren && depth == 0 && !quoted => return Some(i + 1),
            _ => {}
        }
        i += 1;
    }
    None
}

fn is_name_char(ch: char) -> bool {
    !(ch.is_whitespace() || ",#={}\"()".contains(ch))
}

/// A value: parts joined by `#`. None when a delimiter is unbalanced.
fn parse_expr(c: &[char], i: &mut usize) -> Option<Vec<Part>> {
    let mut parts = Vec::new();
    loop {
        skip_ws(c, i);
        if *i >= c.len() {
            return Some(parts);
        }
        match c[*i] {
            '{' => {
                let mut d = 0usize;
                let st = *i + 1;
                loop {
                    if *i >= c.len() {
                        return None;
                    }
                    match c[*i] {
                        '{' => d += 1,
                        '}' => {
                            d -= 1;
                            if d == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    *i += 1;
                }
                parts.push(Part::Lit(c[st..*i].iter().collect()));
                *i += 1;
            }
            '"' => {
                let st = *i + 1;
                let mut d = 0usize;
                *i += 1;
                loop {
                    if *i >= c.len() {
                        return None;
                    }
                    match c[*i] {
                        '{' => d += 1,
                        '}' => d = d.saturating_sub(1),
                        '"' if d == 0 => break,
                        _ => {}
                    }
                    *i += 1;
                }
                parts.push(Part::Lit(c[st..*i].iter().collect()));
                *i += 1;
            }
            ch if ch.is_ascii_digit() => {
                let st = *i;
                while *i < c.len() && c[*i].is_ascii_digit() {
                    *i += 1;
                }
                parts.push(Part::Lit(c[st..*i].iter().collect()));
            }
            ch if is_name_char(ch) => {
                let st = *i;
                while *i < c.len() && is_name_char(c[*i]) {
                    *i += 1;
                }
                parts.push(Part::Name(
                    c[st..*i].iter().collect::<String>().to_lowercase(),
                ));
            }
            _ => return Some(parts),
        }
        skip_ws(c, i);
        if *i < c.len() && c[*i] == '#' {
            *i += 1;
        } else {
            return Some(parts);
        }
    }
}

type Abbrevs = HashMap<String, Vec<Part>>;

/// Resolves a value. Abbreviations resolve one level deep; unknown names stay as text.
fn resolve(parts: &[Part], strings: &Abbrevs, ctx: &str, warn: &mut Vec<String>) -> String {
    let mut out = String::new();
    for p in parts {
        match p {
            Part::Lit(s) => out.push_str(s),
            Part::Name(n) => match strings.get(n) {
                Some(def) => {
                    for d in def {
                        match d {
                            Part::Lit(s) => out.push_str(s),
                            Part::Name(inner) => {
                                // one level only: a chain of abbreviations is cut here
                                warn.push(format!(
                                    "{ctx}: abbreviation `{n}` refers to `{inner}`; chains are not followed"
                                ));
                                out.push_str(inner);
                            }
                        }
                    }
                }
                None => {
                    warn.push(format!("{ctx}: undefined abbreviation `{n}`"));
                    out.push_str(n);
                }
            },
        }
    }
    out
}

/// Reads a whole `.bib` text. Never fails; problems become warnings.
pub fn read(text: &str) -> RawFile {
    let c: Vec<char> = text.chars().collect();
    let mut strings: Abbrevs = MONTHS
        .iter()
        .map(|(a, full)| (a.to_string(), vec![Part::Lit(full.to_string())]))
        .collect();
    // (type, key, [(field, parts)]) kept unresolved until every @string is known
    let mut pending: Vec<Pending> = Vec::new();
    let mut warnings = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] != '@' {
            i += 1;
            continue;
        }
        let at = i;
        i += 1;
        let ts = i;
        while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_' || c[i] == '-') {
            i += 1;
        }
        let typ: String = c[ts..i].iter().collect::<String>().to_lowercase();
        skip_ws(&c, &mut i);
        if typ.is_empty() || i >= c.len() || (c[i] != '{' && c[i] != '(') {
            continue;
        }
        let Some(end) = group_end(&c, i) else {
            warnings.push(format!(
                "unbalanced delimiters in `@{typ}` entry; entry skipped"
            ));
            i = at + 1;
            continue;
        };
        let body = &c[i + 1..end - 1];
        i = end;
        match typ.as_str() {
            "comment" | "preamble" => {}
            "string" => {
                let mut j = 0;
                skip_ws(body, &mut j);
                let ns = j;
                while j < body.len() && is_name_char(body[j]) {
                    j += 1;
                }
                let name: String = body[ns..j].iter().collect::<String>().to_lowercase();
                skip_ws(body, &mut j);
                if name.is_empty() || j >= body.len() || body[j] != '=' {
                    warnings.push("malformed @string skipped".into());
                    continue;
                }
                j += 1;
                match parse_expr(body, &mut j) {
                    Some(parts) => {
                        strings.insert(name, parts);
                    }
                    None => warnings.push(format!("@string `{name}`: unbalanced value")),
                }
            }
            _ => {
                let mut j = 0;
                while j < body.len() && body[j] != ',' {
                    j += 1;
                }
                let key: String = body[..j].iter().collect::<String>().trim().to_string();
                let mut fields = Vec::new();
                while j < body.len() {
                    // at ',' or end
                    while j < body.len() && (body[j] == ',' || body[j].is_whitespace()) {
                        j += 1;
                    }
                    if j >= body.len() {
                        break;
                    }
                    let ns = j;
                    while j < body.len() && is_name_char(body[j]) {
                        j += 1;
                    }
                    let name: String = body[ns..j].iter().collect::<String>().to_lowercase();
                    skip_ws(body, &mut j);
                    if name.is_empty() || j >= body.len() || body[j] != '=' {
                        // field without `=`: skip to the next comma
                        while j < body.len() && body[j] != ',' {
                            j += 1;
                        }
                        continue;
                    }
                    j += 1;
                    match parse_expr(body, &mut j) {
                        Some(parts) if !parts.is_empty() => fields.push((name, parts)),
                        Some(_) => {}
                        None => {
                            warnings.push(format!("{key}: unbalanced value in field `{name}`"));
                            break;
                        }
                    }
                    while j < body.len() && body[j] != ',' {
                        j += 1; // junk after a value
                    }
                }
                pending.push((typ, key, fields));
            }
        }
    }
    // second pass: abbreviations are all known now
    let mut entries: Vec<RawEntry> = Vec::new();
    for (typ, key, fields) in pending {
        if entries.iter().any(|e| e.key.eq_ignore_ascii_case(&key)) {
            warnings.push(format!("duplicate key `{key}`; the first entry is kept"));
            continue;
        }
        let ctx = key.clone();
        let fields = fields
            .iter()
            .map(|(n, p)| (n.clone(), resolve(p, &strings, &ctx, &mut warnings)))
            .collect();
        entries.push(RawEntry { key, typ, fields });
    }
    // crossref: inherit fields the entry lacks, one level, order in the file irrelevant
    let snapshot = entries.clone();
    for e in &mut entries {
        let Some(target) = e.get("crossref").map(|s| s.trim().to_lowercase()) else {
            continue;
        };
        match snapshot.iter().find(|p| p.key.to_lowercase() == target) {
            Some(parent) => {
                for (n, v) in &parent.fields {
                    if n != "crossref" && e.get(n).is_none() {
                        e.fields.push((n.clone(), v.clone()));
                    }
                }
            }
            None => warnings.push(format!("{}: crossref `{target}` points to no entry", e.key)),
        }
    }
    for e in &entries {
        if e.typ == "article" {
            for need in ["author", "title", "journal", "year"] {
                if e.get(need).is_none()
                    && !(need == "journal" && e.get("journaltitle").is_some())
                    && !(need == "year" && e.get("date").is_some())
                {
                    warnings.push(format!("{}: article lacks required field `{need}`", e.key));
                }
            }
        }
    }
    RawFile { entries, warnings }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(src: &str) -> RawEntry {
        read(src).entries.remove(0)
    }

    #[test]
    fn string_abbreviation() {
        let f = read(r#"@string{tl = "Law Review"} @article{a, journal = tl, year = 2001}"#);
        assert_eq!(f.entries[0].get("journal"), Some("Law Review"));
        assert_eq!(f.entries[0].get("year"), Some("2001"));
    }

    #[test]
    fn defined_after_use_and_paren_form() {
        let f = read("@article{a, journal = tl}\n@STRING( tl = \"Late\" )");
        assert_eq!(f.entries[0].get("journal"), Some("Late"));
    }

    #[test]
    fn concatenation() {
        let e = one(r#"@article{b, title = "Burden " # {of} # " proof", year = 2001}"#);
        assert_eq!(e.get("title"), Some("Burden of proof"));
    }

    #[test]
    fn month_abbreviation() {
        let f = read(r#"@misc{m, month = jul # "~4"}"#);
        let m = f.entries[0].get("month").unwrap();
        assert_eq!(m, "July~4");
        assert!(!m.contains('#') && !m.contains("jul"));
        assert!(f.warnings.is_empty());
    }

    #[test]
    fn crossref_inherits_regardless_of_order() {
        let f = read(
            "@inproceedings{c, crossref = {p1}, title = {T}}\n@proceedings{p1, booktitle = {Proc. X}, year = 2005, title = {Whole}}",
        );
        let c = &f.entries[0];
        assert_eq!(c.typ, "inproceedings");
        assert_eq!(c.get("booktitle"), Some("Proc. X"));
        assert_eq!(c.get("year"), Some("2005"));
        assert_eq!(c.get("title"), Some("T")); // own field wins
        let g = read("@misc{x, crossref={nope}}");
        assert!(g.warnings.iter().any(|w| w.contains("nope")));
    }

    #[test]
    fn comments_and_free_text_ignored() {
        let f = read("@comment{ignorat} text liber intre intrari @misc{d, title = {D}}");
        assert_eq!(f.entries.len(), 1);
        assert_eq!(
            (f.entries[0].key.as_str(), f.entries[0].typ.as_str()),
            ("d", "misc")
        );
        assert!(read("@preamble{\"\\newcommand\"} @misc{e}").entries.len() == 1);
    }

    #[test]
    fn undefined_abbreviation_is_text_and_warning() {
        let f = read("@article{u, journal = zzz, author={A}, title={T}, year=1}");
        assert_eq!(f.entries[0].get("journal"), Some("zzz"));
        assert!(f.warnings.iter().any(|w| w.contains("zzz")));
    }

    #[test]
    fn entry_type_kept() {
        assert_eq!(one("@PhDThesis{e, title={X}}").typ, "phdthesis");
    }

    #[test]
    fn broken_input_degrades_gracefully() {
        let f = read("@misc{a, title = {oops\n@misc{b, title = {ok}}");
        assert!(f.entries.iter().any(|e| e.key == "b"));
        assert!(!f.warnings.is_empty());
        let d = read("@misc{k, title={1}} @misc{K, title={2}}");
        assert_eq!(d.entries.len(), 1);
        assert_eq!(d.entries[0].get("title"), Some("1"));
        assert!(d.warnings.iter().any(|w| w.contains("duplicate")));
        let n = read("@misc{k, title, year = {1999}, = {x}}");
        assert_eq!(n.entries[0].get("year"), Some("1999"));
    }

    #[test]
    fn chain_stops_with_warning() {
        let f = read("@string{a = \"A\"} @string{b = a # \"B\"} @misc{k, title = b}");
        let t = f.entries[0].get("title").unwrap();
        assert!(t.contains('B'));
        assert!(f.warnings.iter().any(|w| w.contains("chain")));
    }
}
