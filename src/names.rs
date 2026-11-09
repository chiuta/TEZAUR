//! BibTeX person-name parsing (spec 04): `First von Last`, `von Last, First` and
//! `von Last, Jr, First`, brace-aware, with the `and others` marker. Every part is a list of
//! tokens that keep their original braces; LaTeX clean-up happens afterwards, per token.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Person {
    pub first: Vec<String>,
    pub von: Vec<String>,
    pub last: Vec<String>,
    pub jr: Vec<String>,
    /// Written with a comma (`Last, First`); lets callers tell "Council of Europe" from a person.
    pub inverted_form: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NameList {
    pub people: Vec<Person>,
    /// The list ended with `and others` (styles print "et al.").
    pub et_al: bool,
}

enum Piece {
    Word(String),
    Comma,
}

/// Splits at depth-0 whitespace and depth-0 commas; braces stay inside the tokens.
fn pieces(s: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    let flush = |cur: &mut String, out: &mut Vec<Piece>| {
        if !cur.is_empty() {
            out.push(Piece::Word(std::mem::take(cur)));
        }
    };
    for c in s.chars() {
        match c {
            '{' => {
                depth += 1;
                cur.push(c);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                cur.push(c);
            }
            c if depth == 0 && c.is_whitespace() => flush(&mut cur, &mut out),
            ',' if depth == 0 => {
                flush(&mut cur, &mut out);
                out.push(Piece::Comma);
            }
            c => cur.push(c),
        }
    }
    flush(&mut cur, &mut out);
    out
}

/// True when the first letter at brace level 0 is lower case. A brace group that starts with a
/// backslash (a LaTeX special character) counts as level 0, by its letter. No letter: not von.
fn is_von(tok: &str) -> bool {
    let c: Vec<char> = tok.chars().collect();
    let mut i = 0;
    let mut depth = 0usize;
    let mut special_depth: Option<usize> = None;
    while i < c.len() {
        match c[i] {
            '{' => {
                depth += 1;
                if special_depth.is_none() && c.get(i + 1) == Some(&'\\') {
                    special_depth = Some(depth);
                    i += 2; // skip '\'
                    // an accent symbol (non-letter) is skipped; a command name is read as letters
                    if c.get(i).is_some_and(|x| !x.is_alphabetic()) {
                        i += 1;
                    }
                    continue;
                }
            }
            '}' => {
                if special_depth == Some(depth) {
                    special_depth = None;
                }
                depth = depth.saturating_sub(1);
            }
            ch if ch.is_alphabetic() && (depth == 0 || special_depth.is_some()) => {
                return ch.is_lowercase();
            }
            _ => {}
        }
        i += 1;
    }
    false
}

/// Splits `von Last` tokens: Last always keeps the final token; von runs from the first to the
/// last lower-case token before it.
fn split_von_last(t: &[String], first_form: bool) -> (Vec<String>, Vec<String>, Vec<String>) {
    if t.is_empty() {
        return (vec![], vec![], vec![]);
    }
    let body = &t[..t.len() - 1];
    let lo = body.iter().position(|x| is_von(x));
    let hi = body.iter().rposition(|x| is_von(x));
    match (lo, hi) {
        (Some(a), Some(b)) => (t[..a].to_vec(), t[a..=b].to_vec(), t[b + 1..].to_vec()),
        // no von: in `First Last` everything but the final token is First
        _ if first_form => (t[..t.len() - 1].to_vec(), vec![], t[t.len() - 1..].to_vec()),
        _ => (vec![], vec![], t.to_vec()),
    }
}

fn parse_one(ps: &[Piece]) -> Option<Person> {
    let mut parts: Vec<Vec<String>> = vec![vec![]];
    for p in ps {
        match p {
            Piece::Word(w) => parts.last_mut().unwrap().push(w.clone()),
            Piece::Comma => parts.push(vec![]),
        }
    }
    if parts.iter().all(|p| p.is_empty()) {
        return None;
    }
    let mut person = Person {
        inverted_form: parts.len() > 1,
        ..Default::default()
    };
    match parts.len() {
        1 => {
            let (f, v, l) = split_von_last(&parts[0], true);
            (person.first, person.von, person.last) = (f, v, l);
        }
        n => {
            // before the first comma only `von Last` is written: anything ahead of von is Last
            let (pre, v, l) = split_von_last(&parts[0], false);
            person.von = v;
            person.last = pre.into_iter().chain(l).collect();
            if n == 2 {
                person.first = parts[1].clone();
            } else {
                person.jr = parts[1].clone();
                // more than two commas: the rest stays in First, commas included
                let mut first = vec![];
                for (k, p) in parts[2..].iter().enumerate() {
                    if k > 0
                        && let Some(l) = first.last_mut()
                    {
                        *l = format!("{l},");
                    }
                    first.extend(p.iter().cloned());
                }
                person.first = first;
            }
        }
    }
    if person.last.is_empty() {
        // keep the invariant: Last is non-empty whenever any part is
        if let Some(x) = person.von.pop().or_else(|| person.first.pop()) {
            person.last.push(x);
        }
    }
    Some(person)
}

/// Parses a whole author / editor field.
pub fn parse_names(field: &str) -> NameList {
    let all = pieces(field);
    let mut groups: Vec<Vec<Piece>> = vec![vec![]];
    for p in all {
        match &p {
            Piece::Word(w) if w == "and" => groups.push(vec![]),
            _ => groups.last_mut().unwrap().push(p),
        }
    }
    let mut list = NameList::default();
    let n = groups.len();
    for (i, g) in groups.iter().enumerate() {
        let is_others = matches!(g.as_slice(), [Piece::Word(w)] if w == "others");
        if is_others && i + 1 == n && i > 0 {
            list.et_al = true;
            continue;
        }
        if let Some(p) = parse_one(g) {
            list.people.push(p);
        }
    }
    list
}

fn clean_join(toks: &[String]) -> String {
    toks.iter()
        .map(|t| crate::bib::clean(t))
        .collect::<Vec<_>>()
        .join(" ")
}

impl Person {
    /// Whole-brace single-token name (an institution): `{Barnes and Noble, Inc}`.
    pub fn is_literal(&self) -> bool {
        self.first.is_empty()
            && self.von.is_empty()
            && self.jr.is_empty()
            && self.last.len() == 1
            && self.last[0].starts_with('{')
            && self.last[0].ends_with('}')
    }
    pub fn given(&self) -> String {
        clean_join(&self.first)
    }
    pub fn particle(&self) -> String {
        clean_join(&self.von)
    }
    pub fn family(&self) -> String {
        clean_join(&self.last)
    }
    pub fn suffix(&self) -> String {
        clean_join(&self.jr)
    }
    /// Name shown in a citation: von + Last, no first name; literal names whole.
    pub fn short(&self) -> String {
        [self.particle(), self.family()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
    /// `von Last, Jr, First` (the unambiguous BibTeX form), cleaned.
    pub fn inverted(&self) -> String {
        let mut s = self.short();
        for x in [self.suffix(), self.given()] {
            if !x.is_empty() {
                s.push_str(", ");
                s.push_str(&x);
            }
        }
        s
    }
    /// CSL JSON name object (von maps to `non-dropping-particle`, Jr to `suffix`).
    pub fn to_csl(&self) -> serde_json::Value {
        if self.is_literal() {
            return serde_json::json!({"literal": self.family()});
        }
        let mut v = serde_json::Map::new();
        v.insert("family".into(), self.family().into());
        for (k, x) in [
            ("given", self.given()),
            ("non-dropping-particle", self.particle()),
            ("suffix", self.suffix()),
        ] {
            if !x.is_empty() {
                v.insert(k.into(), x.into());
            }
        }
        v.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn manual_example_four_two_two() {
        let l = parse_names(r"Charles Louis Xavier Joseph de la Vall{\'e}e Poussin");
        assert_eq!(l.people.len(), 1);
        let p = &l.people[0];
        assert_eq!(p.first, t(&["Charles", "Louis", "Xavier", "Joseph"]));
        assert_eq!(p.von, t(&["de", "la"]));
        assert_eq!(p.last, t(&[r"Vall{\'e}e", "Poussin"]));
        assert_eq!(p.family(), "Vallee Poussin");
    }

    #[test]
    fn comma_forms() {
        let p = &parse_names("Brinch Hansen, Per").people[0];
        assert_eq!(
            (p.last.clone(), p.first.clone()),
            (t(&["Brinch", "Hansen"]), t(&["Per"]))
        );
        let q = &parse_names("Per Brinch Hansen").people[0];
        assert_eq!(
            (q.first.clone(), q.last.clone()),
            (t(&["Per", "Brinch"]), t(&["Hansen"]))
        );
        let b = &parse_names("von Beethoven, Ludwig").people[0];
        assert_eq!(
            (b.von.clone(), b.last.clone(), b.first.clone()),
            (t(&["von"]), t(&["Beethoven"]), t(&["Ludwig"]))
        );
        let f = &parse_names("Ford, Jr., Henry").people[0];
        assert_eq!(
            (f.last.clone(), f.jr.clone(), f.first.clone()),
            (t(&["Ford"]), t(&["Jr."]), t(&["Henry"]))
        );
    }

    #[test]
    fn braces_and_lists() {
        let l = parse_names("{Barnes and Noble, Inc}");
        assert_eq!(l.people.len(), 1);
        let p = &l.people[0];
        assert!(p.first.is_empty() && p.von.is_empty() && p.is_literal());
        assert_eq!(p.to_csl()["literal"], "Barnes and Noble, Inc");
        assert_eq!(
            parse_names("Alpha, A. and Beta, B. and Gamma, C.")
                .people
                .len(),
            3
        );
        let a = &parse_names("Andreas {Thurn and Taxis}").people[0];
        assert_eq!(
            (a.first.clone(), a.last.clone()),
            (t(&["Andreas"]), t(&["{Thurn and Taxis}"]))
        );
        assert_eq!(a.family(), "Thurn and Taxis");
        assert!(!a.is_literal());
    }

    #[test]
    fn and_others() {
        let l = parse_names("Doe, John and others");
        assert!(l.et_al);
        assert_eq!(l.people.len(), 1);
        assert_eq!(l.people[0].family(), "Doe");
        // not last: stays a name
        let m = parse_names("Doe, John and others and Roe, R.");
        assert!(!m.et_al);
        assert_eq!(m.people.len(), 3);
    }

    #[test]
    fn csl_export() {
        let c = parse_names("von Beethoven, Ludwig").people[0].to_csl();
        assert_eq!(c["family"], "Beethoven");
        assert_eq!(c["non-dropping-particle"], "von");
        assert_eq!(c["given"], "Ludwig");
        let j = parse_names("Ford, Jr., Henry").people[0].to_csl();
        assert_eq!(j["suffix"], "Jr.");
    }

    #[test]
    fn edge_cases_never_panic() {
        assert!(parse_names("").people.is_empty());
        assert!(parse_names("   \n ").people.is_empty());
        assert_eq!(parse_names("Plato").people[0].last, t(&["Plato"]));
        assert_eq!(parse_names("A AND B").people.len(), 1); // uppercase AND is not a separator
        let w = &parse_names("Doe, Jr., John, Extra").people[0];
        assert_eq!(w.first, t(&["John,", "Extra"]));
        for s in ["{", "}}{", ",,,", "and", "a and", "x, y, z, w, v", "{\\"] {
            let _ = parse_names(s);
        }
        assert_eq!(parse_names("van der").people[0].last, t(&["der"]));
    }

    #[test]
    fn display_forms() {
        let p = &parse_names("Ludwig van Beethoven").people[0];
        assert_eq!(p.short(), "van Beethoven");
        assert_eq!(p.inverted(), "van Beethoven, Ludwig");
    }
}
