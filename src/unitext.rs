//! Index tokens from Unicode text (spec 09). Pipeline: NFKD (ligatures such as U+FB01 fall apart),
//! drop nonspacing marks (accents, comma below, cedilla), lower case, a small table for letters
//! without canonical decomposition, word segmentation per UAX #29, format characters (soft
//! hyphen...) removed inside words, internal marks (apostrophe, dot) handled as the spec says,
//! tokens shorter than two characters dropped, the rest cut to 7 characters. Each token carries
//! the position of the word it came from (all words count, also the ones too short to index).

use unicode_normalization::UnicodeNormalization;
use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};
use unicode_segmentation::UnicodeSegmentation;

/// Bump whenever the rules above change: cached indexes derived from tokens are rebuilt.
pub const RULES_VERSION: u32 = 2;

/// Prefix length kept per token (cheap stemming that works across RO/EN/FR inflections).
pub const STEM: usize = 7;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tok {
    pub text: String,
    /// Index of the source word among all words of the text.
    pub pos: usize,
}

/// Letters that NFKD leaves whole, folded by a short documented table.
fn extra_fold(c: char) -> Option<&'static str> {
    Some(match c {
        'ß' => "ss",
        'æ' => "ae",
        'œ' => "oe",
        'ø' => "o",
        'đ' | 'ð' => "d",
        'ł' => "l",
        'ı' => "i",
        'þ' => "th",
        _ => return None,
    })
}

/// NFKD, marks removed, lower case, extra folding. Idempotent.
pub fn fold_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.nfkd() {
        if c.general_category() == GeneralCategory::NonspacingMark {
            continue;
        }
        for lc in c.to_lowercase() {
            match extra_fold(lc) {
                Some(r) => out.push_str(r),
                None => out.push(lc),
            }
        }
    }
    out
}

fn is_format(c: char) -> bool {
    c.general_category() == GeneralCategory::Format
}

fn push_token(out: &mut Vec<Tok>, t: &str, pos: usize) {
    if t.chars().count() >= 2 {
        out.push(Tok {
            text: t.chars().take(STEM).collect(),
            pos,
        });
    }
}

/// Tokens of `s` with word positions.
pub fn tokens(s: &str) -> Vec<Tok> {
    let folded = fold_text(s);
    let mut out = Vec::new();
    // `split_word_bounds` keeps every segment, so positions count words, not only indexable ones
    let mut pos = 0usize;
    for seg in folded.split_word_bounds() {
        let word: String = seg.chars().filter(|c| !is_format(*c)).collect();
        if !word.chars().any(|c| c.is_alphanumeric()) {
            continue; // spaces, punctuation, symbols
        }
        let internal = word.chars().any(|c| !c.is_alphanumeric());
        if !internal {
            push_token(&mut out, &word, pos);
        } else {
            let numeric = word
                .chars()
                .all(|c| c.is_ascii_digit() || !c.is_alphanumeric());
            // whole word without its internal marks, or with them when it is a number
            let whole: String = if numeric {
                word.clone()
            } else {
                word.chars().filter(|c| c.is_alphanumeric()).collect()
            };
            push_token(&mut out, &whole, pos);
            // and the parts around the marks
            let parts: Vec<&str> = word
                .split(|c: char| !c.is_alphanumeric())
                .filter(|p| !p.is_empty())
                .collect();
            if parts.len() > 1 || (parts.len() == 1 && parts[0] != whole) {
                for p in parts {
                    push_token(&mut out, p, pos);
                }
            }
        }
        pos += 1;
    }
    out
}

/// Token strings only (what the index and `coverage` use).
pub fn token_strings(s: &str) -> Vec<String> {
    tokens(s).into_iter().map(|t| t.text).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Vec<String> {
        token_strings(s)
    }

    #[test]
    fn ligatures_are_expanded() {
        assert_eq!(t("\u{fb01}nal"), t("final"));
        assert_eq!(t("\u{fb01}nal"), vec!["final"]);
        assert_eq!(t("o\u{fb00}ice o\u{fb02}ow"), vec!["office", "oflow"]);
    }

    #[test]
    fn composed_and_decomposed_accents_agree() {
        assert_eq!(fold_text("\u{e9}"), "e");
        assert_eq!(fold_text("e\u{301}"), "e");
        for s in ["\u{e9}t\u{e9}", "Știință", "ﬁnal", "Ǆ", "ß"] {
            let once = fold_text(s);
            assert_eq!(fold_text(&once), once, "{s}");
        }
    }

    #[test]
    fn apostrophe_keeps_one_word_and_adds_parts() {
        let v = tokens("l\u{2019}\u{e9}t\u{e9}");
        let texts: Vec<(&str, usize)> = v.iter().map(|x| (x.text.as_str(), x.pos)).collect();
        assert_eq!(texts, [("lete", 0), ("ete", 0)]);
    }

    #[test]
    fn hyphenated_compounds_are_separate_words() {
        let v = tokens("state-of-the-art");
        let texts: Vec<(&str, usize)> = v.iter().map(|x| (x.text.as_str(), x.pos)).collect();
        assert_eq!(texts, [("state", 0), ("of", 1), ("the", 2), ("art", 3)]);
    }

    #[test]
    fn decimal_numbers_keep_their_dot() {
        let v = t("value 3.5 here");
        assert!(v.contains(&"3.5".to_string()), "{v:?}");
        assert!(!v.contains(&"35".to_string()));
        assert_eq!(tokens("3.5")[0].pos, 0);
    }

    #[test]
    fn soft_hyphen_is_removed_inside_words() {
        assert_eq!(t("con\u{ad}stitution"), vec!["constit"]); // cut to 7
        assert_eq!(t("con\u{ad}stitution"), t("constitution"));
    }

    #[test]
    fn s_and_t_with_comma_or_cedilla_fold_together() {
        let a = t("\u{218}tefan \u{15e}tefan stefan");
        assert_eq!(a, vec!["stefan", "stefan", "stefan"]);
        assert_eq!(
            t("\u{21a}ar\u{103} \u{162}ar\u{103} tara"),
            vec!["tara", "tara", "tara"]
        );
        assert_eq!(t("Științific ȘI țară"), t("stiintific si tara"));
    }

    #[test]
    fn positions_count_short_words() {
        let v = tokens("a bb c dd");
        let p: Vec<usize> = v.iter().map(|x| x.pos).collect();
        assert_eq!(p, [1, 3]);
    }

    #[test]
    fn edge_inputs() {
        assert!(tokens("").is_empty());
        assert!(tokens(" \t\n").is_empty());
        assert!(tokens("!!! ??? \u{1f600}").is_empty());
        assert_eq!(t("Stra\u{df}e"), vec!["strasse"]);
        assert_eq!(t("a\u{200b}b"), t("a b").into_iter().collect::<Vec<_>>());
    }
}
