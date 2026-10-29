//! Retrieval evaluation on your own library: give it questions with the document (and optionally page) that answers each,
//! and it reports recall@1/3/10 and MRR. Without this, "the search is good" is a feeling. No model needed.

use crate::audit::doc_for_key;
use crate::bib::BibMap;
use crate::index::{Searcher, Store};
use serde_json::{json, Value};

pub struct Case {
    pub q: String,
    pub doc: String,
    pub page: Option<usize>,
}

pub fn parse_cases(text: &str) -> Result<Vec<Case>, String> {
    let mut out = Vec::new();
    for (i, l) in text.lines().enumerate() {
        let l = l.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let v: Value = serde_json::from_str(l).map_err(|e| format!("line {}: {e}", i + 1))?;
        let q = v["q"].as_str().ok_or(format!("line {}: missing \"q\"", i + 1))?.to_string();
        let doc = v["doc"].as_str().ok_or(format!("line {}: missing \"doc\"", i + 1))?.to_string();
        out.push(Case { q, doc, page: v["page"].as_u64().map(|p| p as usize) });
    }
    if out.is_empty() {
        return Err("no cases found (one JSON object per line: {\"q\": \"...\", \"doc\": \"key or title part\", \"page\": 12})".into());
    }
    Ok(out)
}

/// Which document does `spec` name: a citation key / file stem, else a case-insensitive part of the title or path.
fn resolve(store: &Store, bib: &BibMap, spec: &str) -> Option<usize> {
    doc_for_key(store, bib, spec).or_else(|| {
        let s = spec.to_lowercase();
        store.docs.iter().position(|d| d.title.to_lowercase().contains(&s) || d.path.to_lowercase().contains(&s))
    })
}

pub fn evaluate(store: &Store, bib: &BibMap, cases: &[Case], emb: &dyn Fn(&str) -> Option<Vec<f32>>) -> Value {
    let sr = Searcher::new(store);
    let (mut r1, mut r3, mut r10, mut rr, mut n) = (0usize, 0usize, 0usize, 0f64, 0usize);
    let mut misses = Vec::new();
    let mut unresolved = Vec::new();
    for c in cases {
        let Some(d) = resolve(store, bib, &c.doc) else {
            unresolved.push(json!({"q": c.q, "doc": c.doc}));
            continue;
        };
        n += 1;
        let qe = emb(&c.q);
        let hits = sr.search(&c.q, qe.as_deref(), 10, 10, None);
        let rank = hits.iter().position(|h| {
            let ch = &store.chunks[h.chunk];
            ch.doc == d && c.page.map(|p| ch.page == p || ch.page == 0).unwrap_or(true)
        });
        match rank {
            Some(r) => {
                rr += 1.0 / (r as f64 + 1.0);
                if r < 1 { r1 += 1 }
                if r < 3 { r3 += 1 }
                r10 += 1;
            }
            None => misses.push(json!({"q": c.q, "doc": store.docs[d].title, "page": c.page})),
        }
    }
    let f = |x: usize| if n == 0 { 0.0 } else { ((x as f64 / n as f64) * 1000.0).round() / 1000.0 };
    json!({"cases": n, "recall@1": f(r1), "recall@3": f(r3), "recall@10": f(r10),
           "mrr": if n == 0 { 0.0 } else { ((rr / n as f64) * 1000.0).round() / 1000.0 },
           "misses": misses, "unresolved_docs": unresolved})
}

pub fn markdown(v: &Value) -> String {
    let mut s = format!(
        "# Retrieval evaluation\n\n{} questions — recall@1 {}, recall@3 {}, recall@10 {}, MRR {}\n\nThis measures your library and your questions only; it says nothing about answer quality.\n\n",
        v["cases"], v["recall@1"], v["recall@3"], v["recall@10"], v["mrr"]
    );
    for m in v["misses"].as_array().into_iter().flatten() {
        s.push_str(&format!("- miss: “{}” → expected {} (p. {})\n", m["q"].as_str().unwrap_or(""), m["doc"].as_str().unwrap_or(""), m["page"]));
    }
    for m in v["unresolved_docs"].as_array().into_iter().flatten() {
        s.push_str(&format!("- unknown document “{}” in: {}\n", m["doc"].as_str().unwrap_or(""), m["q"].as_str().unwrap_or("")));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{Chunk, Doc};

    fn store() -> Store {
        let mut s = Store::default();
        for (i, (t, txt)) in [("Alpha Book", "mitochondria produce energy in the cell"), ("Beta Book", "parliament passed the budget law yesterday")].iter().enumerate() {
            s.docs.push(Doc { path: format!("/x/{}.txt", t.to_lowercase().replace(' ', "")), title: t.to_string(), hash: i as u64, pages: 0 });
            s.chunks.push(Chunk { doc: i, page: 0, text: txt.to_string(), emb: None });
        }
        s
    }

    #[test]
    fn recall_and_mrr() {
        let cases = parse_cases("{\"q\":\"energy from mitochondria\",\"doc\":\"alpha\"}\n{\"q\":\"budget law\",\"doc\":\"Beta Book\"}\n{\"q\":\"budget law\",\"doc\":\"alpha\"}\n{\"q\":\"x\",\"doc\":\"nope\"}\n").unwrap();
        let v = evaluate(&store(), &BibMap::default(), &cases, &|_| None);
        assert_eq!(v["cases"], 3);
        assert_eq!(v["recall@1"], 0.667);
        assert_eq!(v["unresolved_docs"].as_array().unwrap().len(), 1);
        assert!(parse_cases("not json").is_err());
        assert!(parse_cases("").is_err());
    }
}
