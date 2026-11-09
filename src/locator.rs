//! Citation locators (spec 08): what follows the key in `[@key, pp. 321–28]`. A locator has a
//! CSL label (page, chapter, section, ...), a value (number, Roman numeral or text) or an
//! interval. Only page locators with numeric values can be checked against document pages.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Val {
    Num(u32),
    /// Lower-cased Roman numeral (preface pages): kept, never compared with page counts.
    Roman(String),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locator {
    /// CSL locator type (`page`, `chapter`, ...); empty for text that could not be interpreted.
    pub label: String,
    pub from: Val,
    pub to: Option<Val>,
    /// "321 ff.": starts at `from`, end unknown.
    pub open_end: bool,
    /// The label was written (a bare `2019` is not).
    pub explicit_label: bool,
}

impl Locator {
    /// Inclusive numeric page interval when this is a checkable page locator.
    pub fn page_range(&self) -> Option<(usize, usize)> {
        if self.label != "page" || self.is_inverted() {
            return None;
        }
        match (&self.from, &self.to) {
            (Val::Num(a), None) => Some((*a as usize, *a as usize)),
            (Val::Num(a), Some(Val::Num(b))) if !self.open_end => Some((*a as usize, *b as usize)),
            _ => None,
        }
    }

    /// An interval whose end is before its start ("5–3").
    pub fn is_inverted(&self) -> bool {
        matches!((&self.from, &self.to), (Val::Num(a), Some(Val::Num(b))) if b < a)
    }

    /// Short human form, e.g. `chapter 2`, `page 321–328`.
    pub fn describe(&self) -> String {
        let v = |x: &Val| match x {
            Val::Num(n) => n.to_string(),
            Val::Roman(r) | Val::Text(r) => r.clone(),
        };
        let mut s = match &self.to {
            Some(t) => format!("{}\u{2013}{}", v(&self.from), v(t)),
            None => v(&self.from),
        };
        if self.open_end {
            s.push_str(" ff.");
        }
        if self.label.is_empty() {
            s
        } else {
            format!("{} {s}", self.label)
        }
    }
}

/// Written label -> CSL type. Matching ignores case and a final period.
fn label_type(tok: &str) -> Option<&'static str> {
    let t = tok.trim_end_matches('.').to_lowercase();
    Some(match t.as_str() {
        "p" | "pp" | "pag" | "pagina" | "paginile" | "page" | "pages" => "page",
        "chap" | "chaps" | "cap" | "capitol" | "capitole" | "capitolul" | "chapter"
        | "chapters" => "chapter",
        "sec" | "secs" | "sect" | "section" | "sections" | "secțiunea" | "secțiune"
        | "sectiunea" | "§" | "§§" => "section",
        "para" | "paras" | "par" | "paragr" | "paragraph" | "paragraphs" | "paragraful" | "¶"
        | "¶¶" => "paragraph",
        "fig" | "figs" | "figure" | "figures" | "figura" => "figure",
        "tbl" | "tbls" | "tab" | "table" | "tables" | "tabelul" => "table",
        "vol" | "vols" | "volume" | "volumes" | "volumul" => "volume",
        "l" | "ll" | "line" | "lines" => "line",
        "n" | "nn" | "note" | "notes" | "nota" => "note",
        _ => return None,
    })
}

fn roman_value(s: &str) -> Option<u32> {
    if s.is_empty() || !s.chars().all(|c| "ivxlcdmIVXLCDM".contains(c)) {
        return None;
    }
    let val = |c: char| match c.to_ascii_lowercase() {
        'i' => 1,
        'v' => 5,
        'x' => 10,
        'l' => 50,
        'c' => 100,
        'd' => 500,
        _ => 1000,
    };
    let cs: Vec<u32> = s.chars().map(val).collect();
    let mut total = 0;
    for (i, v) in cs.iter().enumerate() {
        if cs.get(i + 1).is_some_and(|n| n > v) {
            total -= *v as i64;
        } else {
            total += *v as i64;
        }
    }
    let total = u32::try_from(total).ok()?;
    // only canonical numerals count ("mi", "dim" are words)
    (to_roman(total).eq_ignore_ascii_case(s)).then_some(total)
}

fn to_roman(mut n: u32) -> String {
    let t = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut s = String::new();
    for (v, r) in t {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    s
}

fn atom(s: &str) -> Option<Val> {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
        return s.parse().ok().map(Val::Num);
    }
    roman_value(s).map(|_| Val::Roman(s.to_lowercase()))
}

/// Splits an interval at the first dash / en dash / em dash run.
fn split_range(item: &str) -> Option<(&str, &str)> {
    let i = item.find(['-', '\u{2013}', '\u{2014}'])?;
    let a = item[..i].trim();
    let rest = item[i..].trim_start_matches(['-', '\u{2013}', '\u{2014}']);
    let b = rest.trim();
    (!a.is_empty() && !b.is_empty()).then_some((a, b))
}

/// "321–28" -> 328, "321–8" -> 328; ends at least as long as the start are kept.
fn extend_end(a: u32, b_txt: &str) -> Option<u32> {
    let b: u32 = b_txt.parse().ok()?;
    let at = a.to_string();
    if b_txt.len() >= at.len() {
        return Some(b);
    }
    let head = &at[..at.len() - b_txt.len()];
    format!("{head}{b_txt}").parse().ok()
}

fn item_val(item: &str) -> Option<(Val, Option<Val>, bool)> {
    let mut item = item.trim();
    let mut open = false;
    for suf in ["et seq.", "et seq", "ff.", "ff", "f.", "sqq."] {
        if let Some(h) = item.strip_suffix(suf) {
            item = h.trim();
            open = true;
            break;
        }
    }
    if let Some((a, b)) = split_range(item) {
        let from = atom(a)?;
        let to = match (&from, atom(b)?) {
            (Val::Num(x), Val::Num(_)) => Val::Num(extend_end(*x, b)?),
            (_, t) => t,
        };
        return Some((from, Some(to), open));
    }
    Some((atom(item)?, None, open))
}

/// Parses the text after a citation key: `, pp. 3–5`, `, cap. 2`, `, p. 3, 5`, `, 2019`...
pub fn parse(tail: &str) -> Vec<Locator> {
    let t = tail.trim_start_matches(|c: char| {
        c == ',' || c == ':' || c == '.' || c == '-' || c.is_whitespace()
    });
    let t = t.trim();
    if t.is_empty() {
        return vec![];
    }
    // optional label: leading run without whitespace or digits
    let lab_end = t
        .find(|c: char| c.is_whitespace() || c.is_ascii_digit())
        .unwrap_or(t.len());
    let first = &t[..lab_end];
    let (label, rest, explicit) = match label_type(first) {
        Some(l) => (l, t[lab_end..].trim(), true),
        None if first.is_empty()
            || atom(first).is_some()
            || atom(first.trim_end_matches('.')).is_some() =>
        {
            ("page", t, false)
        }
        None => {
            // unknown label: kept as text, not checkable
            return vec![Locator {
                label: String::new(),
                from: Val::Text(t.to_string()),
                to: None,
                open_end: false,
                explicit_label: true,
            }];
        }
    };
    // value list: items separated by commas, "and", "și", "&"
    let norm = rest
        .replace(" and ", ",")
        .replace(" și ", ",")
        .replace(" & ", ",");
    let mut out = Vec::new();
    for item in norm.split(',') {
        let item = item.trim().trim_end_matches('.').trim();
        if item.is_empty() {
            continue;
        }
        match item_val(item) {
            Some((from, to, open_end)) => out.push(Locator {
                label: label.to_string(),
                from,
                to,
                open_end,
                explicit_label: explicit,
            }),
            None => {
                // not interpretable: the whole remainder stays one text value
                return vec![Locator {
                    label: label.to_string(),
                    from: Val::Text(rest.to_string()),
                    to: None,
                    open_end: false,
                    explicit_label: explicit,
                }];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(s: &str) -> Locator {
        let v = parse(s);
        assert_eq!(v.len(), 1, "{s}: {v:?}");
        v.into_iter().next().unwrap()
    }

    #[test]
    fn single_page() {
        let l = one(", p. 3");
        assert_eq!((l.label.as_str(), l.page_range()), ("page", Some((3, 3))));
        assert_eq!(one("p.3").page_range(), Some((3, 3)));
        assert_eq!(one("pag. 7").page_range(), Some((7, 7)));
    }

    #[test]
    fn chapter_is_not_a_page() {
        for s in [", cap. 2", ", chap. 2", ", Chapter 2", ", capitolul 2"] {
            let l = one(s);
            assert_eq!(l.label, "chapter", "{s}");
            assert_eq!((l.from.clone(), l.page_range()), (Val::Num(2), None));
        }
    }

    #[test]
    fn ranges_are_extended() {
        assert_eq!(one(", pp. 321\u{2013}28").page_range(), Some((321, 328)));
        assert_eq!(one(", pp. 321-8").page_range(), Some((321, 328)));
        assert_eq!(one(", pp. 3-5").page_range(), Some((3, 5)));
        assert_eq!(one(", pp. 3--5").page_range(), Some((3, 5)));
        assert_eq!(one(", pp. 10\u{2014}12").page_range(), Some((10, 12)));
        assert_eq!(one(", pp. 99\u{2013}101").page_range(), Some((99, 101)));
    }

    #[test]
    fn roman_pages_are_kept_unchecked() {
        let l = one(", p. xii");
        assert_eq!(
            (l.label.as_str(), l.from.clone()),
            ("page", Val::Roman("xii".into()))
        );
        assert_eq!(l.page_range(), None);
        assert_eq!(one("XIV").from, Val::Roman("xiv".into()));
        // words that look Roman are not
        assert_eq!(parse("dim")[0].label, "");
        assert_eq!(parse("civil")[0].label, "");
    }

    #[test]
    fn section_figure_and_symbols() {
        assert_eq!(
            (one("§ 4").label.as_str(), one("§ 4").from),
            ("section", Val::Num(4))
        );
        assert_eq!(one("\u{a7}4").label, "section");
        assert_eq!(one("\u{b6} 2").label, "paragraph");
        assert_eq!(one("fig. 2").label, "figure");
        assert_eq!(one("tbl. 1").label, "table");
        assert_eq!(one("vol. 3").label, "volume");
    }

    #[test]
    fn lists_of_pages() {
        let v = parse(", p. 3, 5");
        assert_eq!(v.len(), 2);
        assert_eq!(
            (v[0].page_range(), v[1].page_range()),
            (Some((3, 3)), Some((5, 5)))
        );
        let w = parse("pp. 3 and 5");
        assert_eq!(w.len(), 2);
        assert_eq!(parse("pp. 3 și 5\u{2013}7").len(), 2);
    }

    #[test]
    fn unlabeled_number_is_a_page_but_flagged() {
        let l = one(", 2019");
        assert_eq!((l.label.as_str(), l.explicit_label), ("page", false));
    }

    #[test]
    fn edge_cases() {
        assert!(parse(",").is_empty());
        assert!(parse("").is_empty());
        let bad = one("p. ???");
        assert_eq!((bad.label.as_str(), bad.page_range()), ("page", None));
        let unk = one("voi. 2");
        assert_eq!((unk.label.as_str(), unk.page_range()), ("", None));
        let inv = one("pp. 5\u{2013}3");
        assert!(inv.is_inverted());
        assert_eq!(inv.page_range(), None);
        let ff = one("p. 12 ff.");
        assert!(ff.open_end);
        assert_eq!(ff.page_range(), Some((12, 12))); // only the first page is checked
        assert_eq!(ff.from, Val::Num(12));
        assert_eq!(one("pp. 12 et seq.").from, Val::Num(12));
    }

    #[test]
    fn describe_forms() {
        assert_eq!(one("pp. 321-28").describe(), "page 321\u{2013}328");
        assert_eq!(one("cap. 2").describe(), "chapter 2");
    }
}
