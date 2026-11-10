//! Neutralising text that comes from outside (a `.bib` someone sent you, an OAI-PMH archive, a draft).
//!
//! Three things go wrong when such text is copied into files or onto a terminal as it is:
//! * control characters: NUL truncates or corrupts files for other tools, ESC starts a terminal escape sequence;
//! * bidirectional overrides (U+202E…) and invisible characters change what a reader sees, so a key or title can
//!   look like something else in a terminal or in a diff;
//! * structure characters (a newline in a RIS value, an unbalanced `}` in a BibTeX value, a `\` or `}` in a LaTeX
//!   citation key) that let a value break out of its field and become new fields or commands.
//!
//! Nothing here changes ordinary text: letters, digits, punctuation, accents and every script, including ZWJ and
//! ZWNJ, which some scripts need, pass through.

/// True for characters that must never be copied from outside text into a file or a terminal line.
fn is_hostile(c: char) -> bool {
    matches!(c,
        '\u{0}'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' | '\u{7f}'..='\u{9f}'
        | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' // bidi embeddings, overrides, isolates
        | '\u{200b}' | '\u{2060}' | '\u{feff}' | '\u{180e}' // zero-width space, word joiner, BOM
        | '\u{fff9}'..='\u{fffb}')
}

/// Single-line text: hostile characters are dropped, line breaks and tabs become one space.
pub fn line(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    let mut space = false;
    for c in s.chars() {
        if matches!(c, '\n' | '\r' | '\t' | '\u{2028}' | '\u{2029}') {
            if !space && !o.is_empty() {
                o.push(' ');
            }
            space = true;
            continue;
        }
        if is_hostile(c) {
            continue;
        }
        space = false;
        o.push(c);
    }
    o.trim().to_string()
}

/// A value for a BibTeX `{…}` field: single line, and braces balanced so it cannot close its own field.
pub fn bib_value(s: &str) -> String {
    let t = line(s);
    let mut o = String::with_capacity(t.len() + 2);
    let mut depth = 0usize;
    for c in t.chars() {
        match c {
            '{' => {
                depth += 1;
                o.push(c);
            }
            '}' => {
                if depth > 0 {
                    depth -= 1;
                    o.push(c);
                } // a `}` with nothing to close is dropped
            }
            c => o.push(c),
        }
    }
    for _ in 0..depth {
        o.push('}');
    }
    // a trailing backslash would escape the closing brace of the field
    while o.ends_with('\\') {
        o.pop();
    }
    o
}

/// A citation key: what BibTeX, Biber and LaTeX accept inside `\cite{…}` without surprises. Whitespace, controls,
/// bidi characters and the characters `{}(),="#%\'~^` are dropped.
pub fn key(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !c.is_whitespace()
                && !is_hostile(*c)
                && !matches!(
                    c,
                    '{' | '}'
                        | '('
                        | ')'
                        | ','
                        | '='
                        | '"'
                        | '#'
                        | '%'
                        | '\\'
                        | '\''
                        | '~'
                        | '^'
                        | '$'
                        | '&'
                )
        })
        .collect()
}

/// Text that goes on a terminal line (warnings, diagnostics): hostile characters dropped, line breaks kept as spaces.
pub fn term(s: &str) -> String {
    line(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_text_is_untouched() {
        for s in [
            "Ada Lovelace",
            "Școala Națională — Ștefan cel Mare",
            "Zoë Müller-Çelik, 東京大学",
            "می‌خواهم", // contains ZWNJ (U+200C), needed in Persian
            "a b",
        ] {
            assert_eq!(line(s), s);
        }
    }

    #[test]
    fn controls_bidi_and_invisibles_go() {
        assert_eq!(
            line("a\u{0}b\u{1b}[31mc\u{202e}d\u{200b}e\u{feff}f"),
            "ab[31mcdef"
        );
        assert_eq!(line("x\u{85}y\u{7f}z"), "xyz");
    }

    #[test]
    fn line_breaks_become_one_space() {
        assert_eq!(line("a\r\n\r\nb\tc"), "a b c");
        assert_eq!(line("\n  a  \n"), "a");
        // a RIS value cannot start a new tag line
        assert!(!line("title\nER  - \nTY  - BOOK").contains('\n'));
    }

    #[test]
    fn bibtex_values_cannot_close_their_field() {
        assert_eq!(bib_value("x}, note = {injected"), "x, note = {injected}");
        assert_eq!(bib_value("a {b} c"), "a {b} c");
        assert_eq!(bib_value("open {{ never closed"), "open {{ never closed}}");
        assert_eq!(bib_value("ends with backslash\\"), "ends with backslash");
        assert_eq!(bib_value("}}}"), "");
        // result is always balanced
        for s in ["}{", "{{}", "}{}{", "a\\}b", "{\\}"] {
            let v = bib_value(s);
            let mut d = 0i32;
            for c in v.chars() {
                match c {
                    '{' => d += 1,
                    '}' => {
                        d -= 1;
                        assert!(d >= 0, "{s:?} -> {v:?}")
                    }
                    _ => {}
                }
            }
            assert_eq!(d, 0, "{s:?} -> {v:?}");
        }
    }

    #[test]
    fn keys_lose_everything_that_could_break_latex() {
        assert_eq!(key("smith2019"), "smith2019");
        assert_eq!(key("doe:2020-a.b/c+d_e"), "doe:2020-a.b/c+d_e");
        assert_eq!(key("a}\\write18{id}{"), "awrite18id");
        assert_eq!(key("x\u{202e}y z"), "xyz");
        assert_eq!(key("kéy_ü"), "kéy_ü");
    }
}
