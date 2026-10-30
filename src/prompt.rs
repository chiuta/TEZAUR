//! Prompt construction and citation checking. The model may only use numbered excerpts.

use crate::bib::BibMap;
use crate::index::{Hit, Store};
use std::collections::BTreeSet;

pub const SYSTEM: &str = "You are a careful academic writing assistant helping a researcher write a thesis. \
You are given numbered SOURCE EXCERPTS from the researcher's own library. Rules:\n\
1. Base every factual claim ONLY on the excerpts. Never use outside knowledge for facts, figures, dates or quotations.\n\
2. After each claim, cite the excerpt number(s) in square brackets, like [3] or [2, 5]. Never invent a number.\n\
3. If the excerpts do not contain enough to answer, say exactly what is missing instead of guessing.\n\
4. If excerpts disagree, present both positions and cite each.\n\
5. Keep quotations short and exact; otherwise paraphrase in scholarly prose.\n\
6. Write in the same language as the TASK unless told otherwise.";

pub const SYSTEM_MERGE: &str = "You merge partial summaries of one document into a single coherent summary. Rules:\n\
1. Use ONLY what the partial summaries say; add no facts, interpretation or outside knowledge.\n\
2. Keep every page reference such as (p. 12) exactly as written, attached to the claim it supports.\n\
3. Remove repetition, keep the document's own order, and write in the same language as the partials.";

pub const SYSTEM_QUERIES: &str = "You turn a research question into search queries for a document library. \
Output 2 to 4 short, self-contained queries, one per line, no numbering, no commentary. \
Each query should cover a different aspect of the question. Use the same language as the question.";

/// "Smith et al. (2019), p. 3" when a .bib entry matches, else the file title.
pub fn label(store: &Store, bib: &BibMap, h: &Hit) -> String {
    let c = &store.chunks[h.chunk];
    let d = &store.docs[c.doc];
    let name = bib
        .find(&d.path)
        .map(|e| e.cite())
        .unwrap_or_else(|| d.title.clone());
    if c.page > 0 {
        format!("{name}, p. {}", c.page)
    } else {
        name
    }
}

pub fn user_prompt(
    store: &Store,
    bib: &BibMap,
    hits: &[Hit],
    task: &str,
    lang: Option<&str>,
) -> String {
    let mut s = String::from("SOURCE EXCERPTS:\n\n");
    for (i, h) in hits.iter().enumerate() {
        s.push_str(&format!(
            "[{}] ({})\n{}\n\n",
            i + 1,
            label(store, bib, h),
            store.chunks[h.chunk].text
        ));
    }
    s.push_str("TASK:\n");
    s.push_str(task);
    if let Some(l) = lang {
        s.push_str(&format!("\n\nWrite the answer in: {l}."));
    }
    s
}

/// If `b[i]` opens a `[n]`, `[n, m]` or `[n; m]` group, returns its numbers and the index of `]`.
fn bracket_group(b: &[char], i: usize) -> Option<(Vec<usize>, usize)> {
    if b.get(i) != Some(&'[') {
        return None;
    }
    let mut j = i + 1;
    while j < b.len() && j - i < 24 && b[j] != ']' && b[j] != '[' {
        j += 1;
    }
    if j >= b.len() || b[j] != ']' {
        return None;
    }
    let inner: String = b[i + 1..j].iter().collect();
    let parts: Vec<&str> = inner.split([',', ';']).map(str::trim).collect();
    if parts
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    {
        Some((parts.iter().filter_map(|p| p.parse().ok()).collect(), j))
    } else {
        None
    }
}

/// Numbers appearing inside [..] brackets, e.g. "[2, 5]" -> {2, 5}.
pub fn cited_numbers(text: &str) -> BTreeSet<usize> {
    let b: Vec<char> = text.chars().collect();
    let mut out = BTreeSet::new();
    let mut i = 0;
    while i < b.len() {
        if let Some((nums, end)) = bracket_group(&b, i) {
            out.extend(nums);
            i = end;
        }
        i += 1;
    }
    out
}

/// Replaces `[n]` markers by `(label)` using `labels[n-1]`; unknown numbers become `(?)`.
pub fn rewrite_cites(text: &str, labels: &[String]) -> String {
    let b: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        if let Some((nums, end)) = bracket_group(&b, i) {
            let parts: Vec<String> = nums
                .iter()
                .map(|&n| {
                    if n >= 1 && n <= labels.len() {
                        labels[n - 1].clone()
                    } else {
                        "?".to_string()
                    }
                })
                .collect();
            out.push_str(&format!("({})", parts.join("; ")));
            i = end + 1;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

/// Page numbers cited as "p. 12" (also inside "(p. 3; p. 5)").
pub fn page_refs(text: &str) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    let mut rest = text;
    while let Some(i) = rest.find("p. ") {
        let after = &rest[i + 3..];
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse() {
            out.insert(n);
        }
        rest = after;
    }
    out
}

/// Flashcards the model wrote as `Q: question | A: answer [n]`, one per line (bullets, numbering
/// and a few spelling variants tolerated). Returns (question, answer-with-markers).
pub fn parse_cards(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let l = line.trim().trim_start_matches(|c: char| {
            c.is_ascii_digit() || matches!(c, '-' | '*' | '•' | '.' | ')' | ' ')
        });
        let lower = l.to_lowercase();
        let Some(q0) = lower.find("q:") else { continue };
        let after_q = &l[q0 + 2..];
        let lower_after = after_q.to_lowercase();
        let Some(a0) = lower_after
            .find("| a:")
            .or_else(|| lower_after.find("|a:"))
            .or_else(|| lower_after.find(" a:"))
        else {
            continue;
        };
        let question = after_q[..a0].trim().trim_end_matches('|').trim();
        let rest = &after_q[a0..];
        let answer = rest[rest.to_lowercase().find("a:").unwrap() + 2..].trim();
        if !question.is_empty() && !answer.is_empty() {
            out.push((question.to_string(), answer.to_string()));
        }
    }
    out
}

pub struct Report {
    pub references: String,
    pub warnings: Vec<String>,
}

/// Deterministic post-check: reference list for cited excerpts, plus warnings.
pub fn check(store: &Store, bib: &BibMap, hits: &[Hit], answer: &str) -> Report {
    let cited = cited_numbers(answer);
    let mut warnings = Vec::new();
    let bad: Vec<String> = cited
        .iter()
        .filter(|&&n| n == 0 || n > hits.len())
        .map(|n| n.to_string())
        .collect();
    if !bad.is_empty() {
        warnings.push(format!(
            "model cited non-existent excerpt(s): {} — treat nearby claims as unsupported",
            bad.join(", ")
        ));
    }
    if cited.is_empty() {
        warnings.push("no citations at all — do not rely on this text".into());
    }
    let uncited = answer
        .split("\n\n")
        .filter(|p| p.chars().count() > 200 && cited_numbers(p).is_empty())
        .count();
    if uncited > 0 {
        warnings.push(format!(
            "{uncited} long paragraph(s) carry no citation — verify or cut"
        ));
    }
    let mut refs = String::new();
    for &n in cited.iter().filter(|&&n| n >= 1 && n <= hits.len()) {
        let h = &hits[n - 1];
        let d = &store.docs[store.chunks[h.chunk].doc];
        let full = match bib.find(&d.path) {
            Some(e) => format!("{}  [{}]", e.reference(), d.path),
            None => d.path.clone(),
        };
        refs.push_str(&format!("[{}] {} — {}\n", n, label(store, bib, h), full));
    }
    Report {
        references: refs,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{Chunk, Doc};

    fn store() -> Store {
        let mut st = Store::default();
        st.docs.push(Doc {
            path: "Library/smith2019.pdf".into(),
            title: "smith2019".into(),
            hash: 0,
            pages: 9,
        });
        st.chunks.push(Chunk {
            doc: 0,
            page: 3,
            text: "x".into(),
            emb: None,
        });
        st
    }

    #[test]
    fn parses_citations() {
        let s = cited_numbers("A claim [1]. Another [2, 5]; list [see 3] and [x] and [7;8].");
        assert_eq!(s.into_iter().collect::<Vec<_>>(), vec![1, 2, 5, 7, 8]);
    }

    #[test]
    fn flags_invented_numbers() {
        let st = store();
        let hits = vec![Hit {
            chunk: 0,
            score: 1.0,
        }];
        let r = check(&st, &BibMap::default(), &hits, "Fine [1]. Made up [9].");
        assert!(r.warnings.iter().any(|w| w.contains("non-existent")));
        assert!(r.references.contains("smith2019, p. 3"));
    }

    #[test]
    fn bibtex_makes_citations_readable() {
        let st = store();
        let bib = BibMap {
            entries: crate::bib::parse(
                "@article{smith2019, author={Smith, J. and Doe, A.}, year={2019}, title={On recognition}, journal={JoE}}",
            ),
        };
        let hits = vec![Hit {
            chunk: 0,
            score: 1.0,
        }];
        assert_eq!(label(&st, &bib, &hits[0]), "Smith & Doe (2019), p. 3");
        let r = check(&st, &bib, &hits, "Claim [1].");
        assert!(
            r.references.contains("On recognition")
                && r.references.contains("Library/smith2019.pdf")
        );
    }

    #[test]
    fn parses_flashcard_lines() {
        let t = "Here are cards:\n1. Q: What is X? | A: A thing [1].\n- q: Who bears the burden? |A: The assessing body [2, 3]\nQ: no answer here\n* Q: Define Y | A: ";
        let c = parse_cards(t);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0], ("What is X?".to_string(), "A thing [1].".to_string()));
        assert_eq!(c[1].0, "Who bears the burden?");
        assert!(c[1].1.ends_with("[2, 3]"));
    }

    #[test]
    fn rewrites_markers_to_pages() {
        let labels = vec!["p. 4".to_string(), "p. 5".to_string()];
        assert_eq!(
            rewrite_cites("A [1]. B [1, 2]. C [7]. D [x].", &labels),
            "A (p. 4). B (p. 4; p. 5). C (?). D [x]."
        );
        assert_eq!(
            page_refs("A (p. 4). B (p. 4; p. 15). C (part 2)")
                .into_iter()
                .collect::<Vec<_>>(),
            vec![4, 15]
        );
    }
}
