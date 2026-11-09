//! Persistent identifiers (spec 02): DOI, ISBN-10/13, ISSN and ORCID iD. Extraction from free
//! text, canonical forms, check-digit validation. Pure functions, no network. A syntactically
//! valid DOI proves nothing about existence; that is for the opt-in online check.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdKind {
    Doi,
    Isbn13,
    Isbn10,
    Issn,
    Orcid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Valid,
    BadChecksum,
    BadStructure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub kind: IdKind,
    /// Form used for every comparison.
    pub canonical: String,
    /// Form shown to people (original spelling for DOI, full URI for ORCID).
    pub display: String,
    pub verdict: Verdict,
    /// Byte range in the searched text (0..0 for isolated values).
    pub start: usize,
    pub end: usize,
    /// DOI only: the longer reading when trailing punctuation was cut off.
    pub alt: Option<String>,
}

fn found(kind: IdKind, canonical: String, display: String, verdict: Verdict) -> Found {
    Found {
        kind,
        canonical,
        display,
        verdict,
        start: 0,
        end: 0,
        alt: None,
    }
}

// ---------------------------------------------------------------- DOI

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|d| d as u8)
}

/// Length in bytes of a DOI prefix (`10.` + 4-9 digits + optional `.digits` groups) at the start
/// of `s`, followed by `/`; None when `s` does not start like one.
fn doi_prefix_len(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    if !s.starts_with("10.") {
        return None;
    }
    let mut i = 3;
    let st = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if !(4..=9).contains(&(i - st)) {
        return None;
    }
    while i < b.len() && b[i] == b'.' {
        let g = i + 1;
        let mut j = g;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j == g {
            return None;
        }
        i = j;
    }
    (i < b.len() && b[i] == b'/').then_some(i)
}

fn strip_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let h = s.get(..prefix.len())?;
    h.eq_ignore_ascii_case(prefix).then(|| &s[prefix.len()..])
}

/// Validates an isolated DOI value (a `doi` field, a pasted URL or `doi:` form). Nothing is
/// trimmed from the end. URL forms drop an unencoded `#fragment` and decode `%xx`.
pub fn normalize_doi(raw: &str) -> Option<Found> {
    let mut s = raw.trim();
    let mut url = false;
    for p in [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
    ] {
        if let Some(r) = strip_ci(s, p) {
            s = r;
            url = true;
            break;
        }
    }
    if !url && let Some(r) = strip_ci(s, "doi:") {
        s = r.trim_start();
    }
    let name = if url {
        let cut = s.split('#').next().unwrap_or("");
        pct_decode(cut)
    } else {
        s.to_string()
    };
    let p = doi_prefix_len(&name)?;
    let suffix = &name[p + 1..];
    if suffix.is_empty() || suffix.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    Some(found(
        IdKind::Doi,
        name.to_ascii_lowercase(),
        name,
        Verdict::Valid,
    ))
}

/// Comparison key of a DOI value (ASCII-lowercased name), if it is one.
pub fn doi_key(raw: &str) -> Option<String> {
    normalize_doi(raw).map(|f| f.canonical)
}

fn doi_suffix_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-._;()/:".contains(c)
}

fn extract_dois(text: &str, out: &mut Vec<Found>) {
    let mut i = 0;
    while let Some(off) = text[i..].find("10.") {
        let at = i + off;
        i = at + 3;
        if text[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric())
        {
            continue;
        }
        let Some(pl) = doi_prefix_len(&text[at..]) else {
            continue;
        };
        // scan the suffix (modern-DOI character class, plus %xx escapes)
        let sb = at + pl + 1;
        let rest = &text[sb..];
        let mut end = 0;
        let mut it = rest.char_indices().peekable();
        while let Some((k, c)) = it.next() {
            if c == '%' {
                let b = rest.as_bytes();
                if k + 2 < b.len() && hex(b[k + 1]).is_some() && hex(b[k + 2]).is_some() {
                    it.next();
                    it.next();
                    end = k + 3;
                    continue;
                }
                break;
            }
            if doi_suffix_char(c) {
                end = k + c.len_utf8();
            } else {
                break;
            }
        }
        let long = &text[at..sb + end];
        // free text: punctuation that likely belongs to the sentence
        let mut cut = long.len();
        loop {
            let cur = &long[..cut];
            let last = cur.chars().next_back().unwrap();
            let strip = ".,;:".contains(last)
                || (last == ')' && cur.matches('(').count() < cur.matches(')').count());
            if strip && cur.len() > pl + 2 {
                cut -= last.len_utf8();
            } else {
                break;
            }
        }
        let short = &long[..cut];
        let name = pct_decode(short);
        let mut f = found(IdKind::Doi, name.to_ascii_lowercase(), name, Verdict::Valid);
        f.start = at;
        f.end = at + cut;
        if cut != long.len() {
            f.alt = Some(pct_decode(long).to_ascii_lowercase());
        }
        i = at + long.len().max(3);
        out.push(f);
    }
}

// ---------------------------------------------------------------- check digits

fn digits_of(s: &str) -> Vec<u32> {
    s.chars().filter_map(|c| c.to_digit(10)).collect()
}

fn isbn13_ok(d: &[u32]) -> bool {
    d.len() == 13
        && d.iter()
            .enumerate()
            .map(|(i, v)| v * if i % 2 == 0 { 1 } else { 3 })
            .sum::<u32>()
            % 10
            == 0
}

fn isbn13_check(first12: &[u32]) -> u32 {
    let s: u32 = first12
        .iter()
        .enumerate()
        .map(|(i, v)| v * if i % 2 == 0 { 1 } else { 3 })
        .sum();
    (10 - s % 10) % 10
}

fn isbn10_vals(s: &str) -> Option<Vec<u32>> {
    let c: Vec<char> = s.chars().collect();
    if c.len() != 10 {
        return None;
    }
    c.iter()
        .enumerate()
        .map(|(i, ch)| match ch {
            'X' | 'x' if i == 9 => Some(10),
            d => d.to_digit(10),
        })
        .collect()
}

fn isbn10_ok(v: &[u32]) -> bool {
    v.iter()
        .enumerate()
        .map(|(i, x)| x * (10 - i as u32))
        .sum::<u32>()
        % 11
        == 0
}

/// 978 + first nine digits + new check digit. None unless `isbn10` is 10 well-formed characters.
pub fn isbn10_to_13(isbn10: &str) -> Option<String> {
    let v = isbn10_vals(&isbn10.replace(['-', ' '], ""))?;
    let mut d = vec![9, 7, 8];
    d.extend_from_slice(&v[..9]);
    let c = isbn13_check(&d);
    d.push(c);
    Some(
        d.iter()
            .map(|x| char::from_digit(*x, 10).unwrap())
            .collect(),
    )
}

/// Validates an isolated ISBN (hyphens, spaces and an `ISBN` label allowed).
pub fn check_isbn(raw: &str) -> Option<Found> {
    let mut s = raw.trim();
    if let Some(r) = strip_ci(s, "isbn") {
        // optional "-10" / "-13" marker, then optional ':' and spaces
        let r = if r.starts_with("-10") || r.starts_with("-13") {
            &r[3..]
        } else {
            r
        };
        s = r.trim_start_matches([':', ' ']);
    }
    let flat: String = s.chars().filter(|c| !matches!(c, '-' | ' ')).collect();
    let n = flat.chars().count();
    if n == 13 && flat.chars().all(|c| c.is_ascii_digit()) {
        let d = digits_of(&flat);
        let v = if (flat.starts_with("978") || flat.starts_with("979")) && isbn13_ok(&d) {
            Verdict::Valid
        } else if flat.starts_with("978") || flat.starts_with("979") {
            Verdict::BadChecksum
        } else {
            Verdict::BadStructure
        };
        return Some(found(IdKind::Isbn13, flat.clone(), flat, v));
    }
    if n == 10 {
        let up = flat.to_uppercase();
        return Some(match isbn10_vals(&up) {
            Some(v) => {
                let verdict = if isbn10_ok(&v) {
                    Verdict::Valid
                } else {
                    Verdict::BadChecksum
                };
                found(IdKind::Isbn10, up.clone(), up, verdict)
            }
            None => found(IdKind::Isbn10, up.clone(), up, Verdict::BadStructure),
        });
    }
    None
}

fn issn_vals(flat: &str) -> Option<Vec<u32>> {
    let c: Vec<char> = flat.chars().collect();
    if c.len() != 8 {
        return None;
    }
    c.iter()
        .enumerate()
        .map(|(i, ch)| match ch {
            'X' | 'x' if i == 7 => Some(10),
            d => d.to_digit(10),
        })
        .collect()
}

/// Validates an isolated ISSN (`NNNN-NNNN` or `NNNNNNNN`, check character digit or X).
pub fn check_issn(raw: &str) -> Option<Found> {
    let t = raw.trim();
    let t = strip_ci(t, "issn")
        .map(|r| r.trim_start_matches([':', ' ']))
        .unwrap_or(t);
    let flat = t.replace('-', "");
    let v = issn_vals(&flat)?;
    if t.contains('-') && t.find('-') != Some(4) {
        return None;
    }
    let sum: u32 = v[..7]
        .iter()
        .enumerate()
        .map(|(i, x)| x * (8 - i as u32))
        .sum();
    let want = (11 - sum % 11) % 11;
    let canon = format!("{}-{}", &flat[..4], flat[4..].to_uppercase());
    let verdict = if v[7] == want {
        Verdict::Valid
    } else {
        Verdict::BadChecksum
    };
    Some(found(IdKind::Issn, canon.clone(), canon, verdict))
}

/// Validates an isolated ORCID iD (bare or as `https://orcid.org/…`, MOD 11-2 check).
pub fn check_orcid(raw: &str) -> Option<Found> {
    let mut t = raw.trim();
    for p in ["https://orcid.org/", "http://orcid.org/"] {
        if let Some(r) = strip_ci(t, p) {
            t = r;
            break;
        }
    }
    let t = t.trim_end_matches('/');
    let flat = t.replace('-', "");
    let c: Vec<char> = flat.chars().collect();
    if c.len() != 16 || (t.contains('-') && t.matches('-').count() != 3) {
        return None;
    }
    let mut total = 0u32;
    for ch in &c[..15] {
        total = (total + ch.to_digit(10)?) * 2;
    }
    let want = (12 - total % 11) % 11;
    let last = match c[15] {
        'X' | 'x' => 10,
        d => d.to_digit(10)?,
    };
    let s: String = c.iter().collect::<String>().to_uppercase();
    let canon = format!("{}-{}-{}-{}", &s[0..4], &s[4..8], &s[8..12], &s[12..16]);
    let verdict = if last == want {
        Verdict::Valid
    } else {
        Verdict::BadChecksum
    };
    Some(found(
        IdKind::Orcid,
        canon.clone(),
        format!("https://orcid.org/{canon}"),
        verdict,
    ))
}

// ---------------------------------------------------------------- extraction

fn overlaps(spans: &[(usize, usize)], a: usize, b: usize) -> bool {
    spans.iter().any(|&(s, e)| a < e && s < b)
}

/// Labeled ISBN (`ISBN`, `ISBN-13:`, `ISBN 10`…): digit groups may be separated by one hyphen
/// or space. Returns (value text, end offset).
fn labeled_isbn(text: &str, from: usize) -> Option<(String, usize)> {
    let b = text.as_bytes();
    let mut i = from;
    if text[i..].starts_with("-10") || text[i..].starts_with("-13") {
        i += 3;
    }
    while i < b.len() && (b[i] == b':' || b[i] == b' ') {
        i += 1;
    }
    let mut digits = 0;
    let mut val = String::new();
    let mut end = i;
    while i < b.len() {
        let c = b[i] as char;
        if c.is_ascii_digit() || ((c == 'X' || c == 'x') && (digits == 9 || digits == 12)) {
            digits += 1;
            val.push(c);
            end = i + 1;
            i += 1;
            if digits == 13 || (digits == 10 && !(val.starts_with("978") || val.starts_with("979")))
            {
                break;
            }
        } else if (c == '-' || c == ' ')
            && i + 1 < b.len()
            && (b[i + 1] as char).is_ascii_alphanumeric()
            && digits > 0
        {
            i += 1;
        } else {
            break;
        }
    }
    (digits == 10 || digits == 13).then_some((val, end))
}

/// Finds DOI, ISBN, ISSN and ORCID in free text, in text order. Several identifiers may sit in
/// one value; each carries its own verdict.
pub fn extract(text: &str) -> Vec<Found> {
    let mut out = Vec::new();
    extract_dois(text, &mut out);
    let mut spans: Vec<(usize, usize)> = out
        .iter()
        .map(|f| {
            (
                f.start,
                f.alt
                    .as_ref()
                    .map_or(f.end, |a| f.start + a.len().max(f.end - f.start)),
            )
        })
        .collect();
    // labeled ISBNs first (spaces allowed inside)
    let lower = text.to_ascii_lowercase();
    let mut p = 0;
    while let Some(o) = lower[p..].find("isbn") {
        let at = p + o;
        p = at + 4;
        if let Some((val, end)) = labeled_isbn(text, at + 4)
            && !overlaps(&spans, at, end)
            && let Some(mut f) = check_isbn(&val)
        {
            f.start = at;
            f.end = end;
            spans.push((at, end));
            out.push(f);
        }
    }
    // bare runs of digits / hyphens, bounded by non-alphanumerics
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut k = 0;
    while k < chars.len() {
        let (s0, c0) = chars[k];
        if !c0.is_ascii_digit() {
            k += 1;
            continue;
        }
        let mut j = k;
        while j < chars.len()
            && (chars[j].1.is_ascii_digit()
                || chars[j].1 == '-'
                || chars[j].1.eq_ignore_ascii_case(&'x'))
        {
            j += 1;
        }
        let e0 = if j < chars.len() {
            chars[j].0
        } else {
            text.len()
        };
        let before_ok = k == 0 || !chars[k - 1].1.is_alphanumeric();
        let after_ok = j >= chars.len() || !chars[j].1.is_alphanumeric();
        let run = text[s0..e0].trim_end_matches('-');
        let e = s0 + run.len();
        k = j.max(k + 1);
        if !before_ok || !after_ok || overlaps(&spans, s0, e) {
            continue;
        }
        let flat: String = run.chars().filter(|c| *c != '-').collect();
        let groups: Vec<&str> = run.split('-').collect();
        let hit = if flat.len() == 16 && groups.len() == 4 && groups.iter().all(|g| g.len() == 4) {
            check_orcid(run)
        } else if groups.len() == 2 && groups.iter().all(|g| g.len() == 4) {
            check_issn(run)
        } else if flat.len() == 13
            && flat.chars().all(|c| c.is_ascii_digit())
            && (flat.starts_with("978") || flat.starts_with("979"))
        {
            check_isbn(run)
        } else if flat.len() == 10 && groups.len() >= 3 {
            check_isbn(run)
        } else {
            None
        };
        if let Some(mut f) = hit {
            f.start = s0;
            f.end = e;
            spans.push((s0, e));
            out.push(f);
        }
    }
    out.sort_by_key(|f| f.start);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doi_forms_share_a_key() {
        let a = normalize_doi("https://doi.org/10.1006/JMBI.1998.2354").unwrap();
        assert_eq!(a.canonical, "10.1006/jmbi.1998.2354");
        assert_eq!(a.display, "10.1006/JMBI.1998.2354");
        assert_eq!(doi_key("doi:10.1006/jmbi.1998.2354").unwrap(), a.canonical);
        assert_eq!(doi_key("DOI: 10.1006/jmbi.1998.2354").unwrap(), a.canonical);
        assert_eq!(
            doi_key("HTTP://DX.DOI.ORG/10.1006/jmbi.1998.2354").unwrap(),
            a.canonical
        );
    }

    #[test]
    fn doi_syntax() {
        assert!(normalize_doi("10.1000.10/123").is_some());
        assert!(normalize_doi("11.1000/abc").is_none());
        assert!(normalize_doi("10.12/abc").is_none());
        assert!(normalize_doi("10.1000/").is_none());
        assert!(normalize_doi("10.1000/a b").is_none());
        // non-ASCII is kept, only ASCII is folded
        assert_eq!(doi_key("10.1000/ÄB").unwrap(), "10.1000/Äb");
    }

    #[test]
    fn doi_trailing_punctuation() {
        let f = extract("vezi doi:10.1000/xyz123.");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].canonical, "10.1000/xyz123");
        assert_eq!(f[0].alt.as_deref(), Some("10.1000/xyz123."));
        assert_eq!(
            normalize_doi("10.1000/xyz123.").unwrap().canonical,
            "10.1000/xyz123."
        );
        let g = extract("(see 10.1000/abc(1)), and");
        assert_eq!(g[0].canonical, "10.1000/abc(1)");
        let h = extract("(10.1000/abc)");
        assert_eq!(h[0].canonical, "10.1000/abc");
    }

    #[test]
    fn doi_url_encoding_and_fragment() {
        assert_eq!(
            normalize_doi("https://doi.org/10.1000/ab%23cd")
                .unwrap()
                .canonical,
            "10.1000/ab#cd"
        );
        assert_eq!(
            normalize_doi("https://doi.org/10.1000/abcd#sec2")
                .unwrap()
                .canonical,
            "10.1000/abcd"
        );
        let f = extract("go to https://doi.org/10.1000/ab%23cd now");
        assert_eq!(f[0].canonical, "10.1000/ab#cd");
    }

    #[test]
    fn isbn_checks() {
        let ok = check_isbn("978-92-95055-12-4").unwrap();
        assert_eq!((ok.kind, ok.verdict), (IdKind::Isbn13, Verdict::Valid));
        assert_eq!(ok.canonical, "9789295055124");
        assert_eq!(
            check_isbn("978-92-95055-12-5").unwrap().verdict,
            Verdict::BadChecksum
        );
        assert_eq!(check_isbn("9780306406157").unwrap().verdict, Verdict::Valid);
        assert_eq!(
            check_isbn("ISBN: 978-0-306-40615-7").unwrap().verdict,
            Verdict::Valid
        );
        assert_eq!(
            check_isbn("ISBN-13: 9780306406157").unwrap().verdict,
            Verdict::Valid
        );
    }

    #[test]
    fn isbn10_conversion() {
        let f = check_isbn("0-306-40615-2").unwrap();
        assert_eq!((f.kind, f.verdict), (IdKind::Isbn10, Verdict::Valid));
        assert_eq!(isbn10_to_13("0-306-40615-2").unwrap(), "9780306406157");
        assert_eq!(
            check_isbn("0-306-40615-3").unwrap().verdict,
            Verdict::BadChecksum
        );
    }

    #[test]
    fn issn_checks() {
        assert_eq!(check_issn("0317-8471").unwrap().verdict, Verdict::Valid);
        let x = check_issn("2434-561x").unwrap();
        assert_eq!(
            (x.verdict, x.canonical.as_str()),
            (Verdict::Valid, "2434-561X")
        );
        assert_eq!(
            check_issn("0317-8472").unwrap().verdict,
            Verdict::BadChecksum
        );
        assert!(check_issn("03178471x1").is_none());
    }

    #[test]
    fn orcid_checks() {
        let f = check_orcid("0000-0002-1825-0097").unwrap();
        assert_eq!(f.verdict, Verdict::Valid);
        assert_eq!(f.display, "https://orcid.org/0000-0002-1825-0097");
        let x = check_orcid("0000-0002-1694-233x").unwrap();
        assert_eq!(
            (x.verdict, x.canonical.as_str()),
            (Verdict::Valid, "0000-0002-1694-233X")
        );
        assert_eq!(
            check_orcid("0000-0002-1825-0098").unwrap().verdict,
            Verdict::BadChecksum
        );
        assert_eq!(
            check_orcid("https://orcid.org/0000-0002-1825-0097")
                .unwrap()
                .canonical,
            "0000-0002-1825-0097"
        );
        assert!(check_orcid("0000-0002-1825").is_none());
    }

    #[test]
    fn extracts_mixed_text_in_order() {
        let t = "Smith, ISBN 978-92-95055-12-4 and 0-306-40615-2; ISSN 0317-8471; \
                 orcid https://orcid.org/0000-0002-1825-0097; doi 10.1000/978-92-95055-12-4.";
        let f = extract(t);
        let kinds: Vec<IdKind> = f.iter().map(|x| x.kind).collect();
        assert_eq!(
            kinds,
            vec![
                IdKind::Isbn13,
                IdKind::Isbn10,
                IdKind::Issn,
                IdKind::Orcid,
                IdKind::Doi
            ]
        );
        assert!(f.iter().all(|x| x.verdict == Verdict::Valid));
        assert_eq!(&t[f[0].start..f[0].end], "ISBN 978-92-95055-12-4");
        // digits inside the DOI are not re-read as an ISBN
        assert_eq!(f.len(), 5);
    }

    #[test]
    fn no_false_hits_on_plain_numbers() {
        assert!(extract("In 2019, 1234567 samples and 10 groups, pp. 123-456.").is_empty());
    }
}
