//! Model-assisted checks on top of retrieval: re-ranking passages, and testing whether each cited passage
//! really supports the sentence that cites it. The model can be wrong here too — these are aids, not proof.

use crate::prompt::cited_numbers;

pub const SYSTEM_RERANK: &str = "You rate how useful each passage is for answering the query. Output exactly one line per passage \
in the form `n: score`, where score is an integer from 0 (irrelevant) to 10 (directly answers). No other text.";

pub const SYSTEM_VERIFY: &str = "You are a strict citation checker. You get excerpts and ONE sentence that cites them. \
Decide whether the excerpts, and nothing else, support everything the sentence claims. \
Answer with exactly one word: SUPPORTED, PARTIAL or UNSUPPORTED.";

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Verdict {
    Supported,
    Partial,
    Unsupported,
    Unclear,
}

/// Scores per passage (index = passage number - 1) from lines like `3: 8`, `[2] 5`, `1) 10`.
pub fn parse_scores(text: &str, n: usize) -> Vec<Option<u8>> {
    let mut out = vec![None; n];
    for line in text.lines() {
        let digits: Vec<&str> = line
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .collect();
        if digits.len() >= 2
            && let (Ok(i), Ok(s)) = (digits[0].parse::<usize>(), digits[1].parse::<u8>())
            && i >= 1
            && i <= n
            && s <= 10
        {
            out[i - 1] = Some(s);
        }
    }
    out
}

/// Order for the passages: best score first, ties and unscored ones keep their retrieval order.
/// `None` when the model gave no usable scores at all (caller keeps the original order).
pub fn rerank_order(text: &str, n: usize) -> Option<Vec<usize>> {
    let sc = parse_scores(text, n);
    if sc.iter().all(|s| s.is_none()) {
        return None;
    }
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by_key(|&i| std::cmp::Reverse(sc[i].map(|s| s as i32).unwrap_or(5)));
    Some(idx)
}

pub fn parse_verdict(text: &str) -> Verdict {
    let t = text.trim().to_uppercase();
    let first: String = t
        .chars()
        .skip_while(|c| !c.is_alphabetic())
        .take_while(|c| c.is_alphabetic())
        .collect();
    match first.as_str() {
        "SUPPORTED" => Verdict::Supported,
        "PARTIAL" | "PARTIALLY" => Verdict::Partial,
        "UNSUPPORTED" | "NOT" => Verdict::Unsupported,
        _ if t.contains("UNSUPPORTED") => Verdict::Unsupported,
        _ if t.contains("PARTIAL") => Verdict::Partial,
        _ if t.contains("SUPPORTED") => Verdict::Supported,
        _ => Verdict::Unclear,
    }
}

/// Sentences that carry citations, with their valid excerpt numbers (1..=n_hits), in order, at most `max`.
pub fn cite_sentences(answer: &str, n_hits: usize, max: usize) -> Vec<(String, Vec<usize>)> {
    let mut out = Vec::new();
    for para in answer.split('\n') {
        let mut cur = String::new();
        let chars: Vec<char> = para.chars().collect();
        let mut i = 0;
        let mut depth = 0i32; // do not split inside [1, 2]
        while i < chars.len() {
            let c = chars[i];
            cur.push(c);
            if c == '[' {
                depth += 1;
            } else if c == ']' {
                depth -= 1;
            }
            let end = matches!(c, '.' | '?' | '!')
                && depth <= 0
                && chars
                    .get(i + 1)
                    .is_none_or(|n| n.is_whitespace() || *n == '[');
            // a citation right after the full stop belongs to this sentence: "claim. [1]"
            if end {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if chars.get(j) == Some(&'[')
                    && chars.get(j + 1).is_some_and(|d| d.is_ascii_digit())
                {
                    i += 1;
                    continue;
                }
                push_sentence(&mut out, &cur, n_hits);
                cur.clear();
            }
            i += 1;
        }
        push_sentence(&mut out, &cur, n_hits);
    }
    out.truncate(max);
    out
}

fn push_sentence(out: &mut Vec<(String, Vec<usize>)>, s: &str, n_hits: usize) {
    let s = s.trim();
    if s.is_empty() {
        return;
    }
    let nums: Vec<usize> = cited_numbers(s)
        .into_iter()
        .filter(|&n| n >= 1 && n <= n_hits)
        .collect();
    if !nums.is_empty() {
        out.push((s.to_string(), nums));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scores_in_various_shapes() {
        let s = parse_scores("1: 9\n[2] 3\n3) 10\nfoo\n9: 4\n2: 99", 3);
        assert_eq!(s, vec![Some(9), Some(3), Some(10)]); // 99 is out of range, passage 9 does not exist
    }

    #[test]
    fn reranks_stably_and_gives_up_on_garbage() {
        assert_eq!(rerank_order("1: 2\n2: 9\n3: 2", 3), Some(vec![1, 0, 2]));
        assert_eq!(rerank_order("1: 7\n3: 7", 3), Some(vec![0, 2, 1])); // passage 2 unscored = neutral 5
        assert_eq!(rerank_order("no idea", 3), None);
    }

    #[test]
    fn parses_verdicts() {
        assert_eq!(parse_verdict("SUPPORTED"), Verdict::Supported);
        assert_eq!(parse_verdict("  Partial."), Verdict::Partial);
        assert_eq!(
            parse_verdict("The sentence is UNSUPPORTED by the excerpt"),
            Verdict::Unsupported
        );
        assert_eq!(parse_verdict("¯\\_(ツ)_/¯"), Verdict::Unclear);
    }

    #[test]
    fn finds_cited_sentences() {
        let a = "The authority bears the burden [1]. Costs may be imposed [2, 3]. No citation here. Ghost claim [9].\nSecond line. With trailing cite. [2]";
        let s = cite_sentences(a, 3, 10);
        assert_eq!(s.len(), 3);
        assert_eq!(
            s[0],
            ("The authority bears the burden [1].".to_string(), vec![1])
        );
        assert_eq!(s[1].1, vec![2, 3]);
        assert_eq!(s[2].0, "With trailing cite. [2]");
        assert_eq!(cite_sentences(a, 3, 1).len(), 1);
    }
}
