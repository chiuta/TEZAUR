//! Exact phrase and proximity queries (spec 10). `"burden of proof"` needs the words in order on
//! consecutive positions; `"employment place"~3` needs the two words within 3 positions, either
//! order. Everything outside quotes is an ordinary term. Matching is done on the candidate
//! passages with the same word segmentation as indexing (positions count every word), so the
//! index itself is unchanged.

use crate::unitext;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Phrase {
    /// Word keys (see `unitext::words`).
    pub words: Vec<String>,
    /// Maximum distance between the two words; None = exact consecutive phrase.
    pub within: Option<usize>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Parsed {
    pub phrases: Vec<Phrase>,
    /// Query text for scoring: ordinary terms plus the words of every phrase.
    pub scoring: String,
    pub warnings: Vec<String>,
    /// Syntax error: the query must not run.
    pub error: Option<String>,
}

fn is_open(c: char) -> bool {
    matches!(c, '"' | '\u{201c}' | '\u{201e}' | '\u{ab}')
}

fn is_close(c: char) -> bool {
    matches!(c, '"' | '\u{201d}' | '\u{201c}' | '\u{bb}')
}

pub fn parse(q: &str) -> Parsed {
    let ch: Vec<char> = q.chars().collect();
    let mut p = Parsed::default();
    let mut rest = String::new(); // ordinary terms
    let mut i = 0;
    while i < ch.len() {
        if !is_open(ch[i]) {
            rest.push(ch[i]);
            i += 1;
            continue;
        }
        let Some(rel) = ch[i + 1..].iter().position(|&c| is_close(c)) else {
            p.warnings.push(
                "unclosed quotation mark: the rest of the query is used as ordinary terms".into(),
            );
            rest.push(' ');
            rest.extend(&ch[i + 1..]);
            break;
        };
        let inner: String = ch[i + 1..i + 1 + rel].iter().collect();
        i += rel + 2;
        // optional proximity suffix: ~k
        let mut within = None;
        if ch.get(i) == Some(&'~') {
            let mut j = i + 1;
            let neg = ch.get(j) == Some(&'-');
            if neg {
                j += 1;
            }
            let st = j;
            while ch.get(j).is_some_and(|c| c.is_ascii_digit()) {
                j += 1;
            }
            let num: String = ch[st..j].iter().collect();
            match num.parse::<usize>() {
                Ok(k) if !neg && k >= 1 => within = Some(k),
                _ => {
                    p.error = Some("proximity needs a whole number of at least 1 after `~`".into());
                    return p;
                }
            }
            i = j;
        }
        let words = unitext::words(&inner);
        match (words.len(), within) {
            (0, _) => {} // empty phrase: ignored
            (1, None) => {
                if unitext::tokens(&inner).is_empty() {
                    p.warnings.push(format!(
                        "the one-character phrase \"{inner}\" cannot be checked; ignored"
                    ));
                } else {
                    rest.push(' ');
                    rest.push_str(&inner);
                }
            }
            (n, Some(_)) if n != 2 => {
                p.error = Some("proximity (`~k`) is defined for exactly two words".into());
                return p;
            }
            (_, w) => {
                rest.push(' ');
                rest.push_str(&inner);
                p.phrases.push(Phrase { words, within: w });
            }
        }
        rest.push(' ');
    }
    p.scoring = rest.split_whitespace().collect::<Vec<_>>().join(" ");
    p
}

impl Phrase {
    /// Does the passage (given as its word keys) satisfy the phrase?
    pub fn matches(&self, doc: &[String]) -> bool {
        let n = self.words.len();
        match self.within {
            None => doc.len() >= n && doc.windows(n).any(|w| w == self.words.as_slice()),
            Some(k) => {
                let pos = |w: &String| -> Vec<usize> {
                    doc.iter()
                        .enumerate()
                        .filter(|(_, d)| *d == w)
                        .map(|(i, _)| i)
                        .collect()
                };
                let (a, b) = (pos(&self.words[0]), pos(&self.words[1]));
                a.iter()
                    .any(|&x| b.iter().any(|&y| x != y && x.abs_diff(y) <= k))
            }
        }
    }
}

/// True when the passage text satisfies every phrase.
pub fn passage_ok(phrases: &[Phrase], text: &str) -> bool {
    if phrases.is_empty() {
        return true;
    }
    let w = unitext::words(text);
    phrases.iter().all(|p| p.matches(&w))
}

/// The query as plain terms (quotes and `~k` removed), for relevance measures such as `coverage`.
pub fn plain(q: &str) -> String {
    let p = parse(q);
    if p.error.is_some() {
        q.to_string()
    } else {
        p.scoring
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(q: &str, text: &str) -> bool {
        let p = parse(q);
        assert!(p.error.is_none(), "{:?}", p.error);
        passage_ok(&p.phrases, text)
    }

    #[test]
    fn exact_phrase_needs_order_and_adjacency() {
        let a = "The burden of proof lies with the authority";
        let b = "The proof of the burden is heavy";
        assert!(ok("\"burden of proof\"", a));
        assert!(!ok("\"burden of proof\"", b));
        assert!(ok("burden of proof", b)); // no quotes: no phrase filter
    }

    #[test]
    fn short_words_still_count_as_positions() {
        assert!(!ok("\"state claim\"", "state a claim"));
        assert!(ok("\"state a claim\"", "state a claim"));
    }

    #[test]
    fn proximity_is_two_words_either_order() {
        assert!(ok("\"employment place\"~3", "the place of employment"));
        assert!(!ok(
            "\"employment place\"~3",
            "employment is a thing at some other place"
        ));
        assert!(ok("\"employment place\"~3", "employment at the place"));
    }

    #[test]
    fn case_and_diacritics_follow_indexing() {
        assert!(ok("\"Burden of Proof\"", "the burden of proof"));
        assert!(ok(
            "\"sarcina probei\"",
            "SARCINA PROBEI revine autorității"
        ));
        assert!(ok("\"\u{fb01}nal report\"", "the final report"));
        assert!(ok("\u{201c}burden of proof\u{201d}", "burden of proof"));
        assert!(ok("\u{201e}burden of proof\u{201d}", "burden of proof"));
    }

    #[test]
    fn unclosed_quote_warns_and_degrades() {
        let p = parse("\"burden of proof");
        assert!(p.phrases.is_empty() && p.error.is_none());
        assert_eq!(p.warnings.len(), 1);
        assert_eq!(p.scoring, "burden of proof");
    }

    #[test]
    fn single_word_phrase_is_an_ordinary_term() {
        let p = parse("\"proof\"");
        assert!(p.phrases.is_empty());
        assert_eq!(p.scoring, "proof");
        assert_eq!(parse("proof").scoring, "proof");
        let one = parse("\"x\"");
        assert!(one.phrases.is_empty() && one.warnings.len() == 1 && one.scoring.is_empty());
        let empty = parse("\"\" proof");
        assert!(empty.phrases.is_empty() && empty.scoring == "proof" && empty.warnings.is_empty());
    }

    #[test]
    fn syntax_errors() {
        assert!(parse("\"a b\"~0").error.is_some());
        assert!(parse("\"a b\"~-2").error.is_some());
        assert!(parse("\"a b\"~").error.is_some());
        assert!(parse("\"a b c\"~2").error.is_some());
        assert!(parse("\"a\"~2").error.is_some());
    }

    #[test]
    fn mixed_query_keeps_terms_for_scoring() {
        let p = parse("\"burden of proof\" authority");
        assert_eq!(p.phrases.len(), 1);
        assert_eq!(p.scoring, "burden of proof authority");
        assert_eq!(plain("\"burden proof\"~3 x"), "burden proof x");
        assert_eq!(plain("no quotes"), "no quotes");
    }

    #[test]
    fn numbers_with_dots_are_one_word() {
        assert!(ok("\"3.5\" ", "value 3.5 here"));
        assert!(ok("\"value 3.5\"", "the value 3.5 here"));
        assert!(!ok("\"value 35\"", "the value 3.5 here"));
    }
}
