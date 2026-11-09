//! Storage + retrieval: chunks, BM25, optional dense vectors, rank fusion.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Doc {
    pub path: String,
    pub title: String,
    pub hash: u64,
    pub pages: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chunk {
    pub doc: usize,
    /// 1-based page number, 0 when the format has no pages (txt, md, html).
    pub page: usize,
    pub text: String,
    /// L2-normalised embedding, present only when an embedding model was used.
    pub emb: Option<Vec<f32>>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Store {
    pub docs: Vec<Doc>,
    pub chunks: Vec<Chunk>,
    pub embed_model: Option<String>,
    /// Where this store was loaded from; lets the BM25 cache live next to it (`<index>.bm25`).
    #[serde(skip)]
    pub source: Option<std::path::PathBuf>,
    /// SHA-256 of each document's bytes at ingestion, keyed by doc path with the 64-bit hash it belongs to.
    /// Kept in a sidecar file (`<index>.sha256`) so the index format itself does not change.
    #[serde(skip)]
    pub sha: std::collections::HashMap<String, (u64, String)>,
}

fn sha_path(p: &Path) -> std::path::PathBuf {
    let mut o = p.as_os_str().to_owned();
    o.push(".sha256");
    o.into()
}

fn read_sha(p: &Path) -> std::collections::HashMap<String, (u64, String)> {
    let mut out = std::collections::HashMap::new();
    if let Ok(b) = std::fs::read(sha_path(p))
        && let Ok(m) =
            serde_json::from_slice::<std::collections::HashMap<String, (String, String)>>(&b)
    {
        for (k, (h, s)) in m {
            if let Ok(h) = u64::from_str_radix(&h, 16) {
                out.insert(k, (h, s));
            }
        }
    }
    out
}

impl Store {
    pub fn load(path: &Path) -> Result<Store> {
        if !path.exists() {
            return Ok(Store {
                source: Some(path.to_path_buf()),
                ..Store::default()
            });
        }
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let mut st: Store = bincode::deserialize(&bytes)
            .context("index file is corrupt or from another version")?;
        st.source = Some(path.to_path_buf());
        st.sha = read_sha(path);
        Ok(st)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        // Unique temp name per process, flushed to disk before the rename: a crash or a second writer can never leave a half-written index.
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        let bytes = bincode::serialize(self)?;
        {
            let mut f = std::fs::File::create(&tmp)?;
            std::io::Write::write_all(&mut f, &bytes)?;
            f.sync_all()?;
        }
        if let Err(e) = std::fs::rename(&tmp, path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.into());
        }
        let side = sha_path(path);
        if self.sha.is_empty() {
            let _ = std::fs::remove_file(&side);
        } else {
            let m: std::collections::BTreeMap<&String, (String, &String)> = self
                .sha
                .iter()
                .map(|(k, (h, s))| (k, (format!("{h:016x}"), s)))
                .collect();
            std::fs::write(&side, serde_json::to_vec(&m)?)?;
        }
        Ok(())
    }

    /// The SHA-256 recorded when this document was ingested, if it matches the document as stored.
    pub fn doc_sha256(&self, id: usize) -> Option<&str> {
        let d = self.docs.get(id)?;
        match self.sha.get(&d.path) {
            Some((h, s)) if *h == d.hash => Some(s),
            _ => None,
        }
    }

    pub fn doc_by_path(&self, p: &str) -> Option<usize> {
        self.docs.iter().position(|d| d.path == p)
    }

    /// Remove a document and its chunks, keeping doc ids consistent.
    pub fn remove_doc(&mut self, id: usize) {
        let gone = self.docs.remove(id);
        self.sha.remove(&gone.path);
        self.chunks.retain(|c| c.doc != id);
        for c in &mut self.chunks {
            if c.doc > id {
                c.doc -= 1;
            }
        }
    }
}

// ---------------------------------------------------------------- tokenizing

pub fn fold(c: char) -> char {
    match c {
        'ă' | 'â' | 'á' | 'à' | 'ä' | 'ã' | 'å' => 'a',
        'î' | 'í' | 'ì' | 'ï' => 'i',
        'ș' | 'ş' | 'š' | 'ś' => 's',
        'ț' | 'ţ' => 't',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' | 'ć' | 'č' => 'c',
        'ñ' => 'n',
        'ß' => 's',
        other => other,
    }
}

/// Index tokens of a text (Unicode folding and word segmentation, see `unitext`).
pub fn tokenize(s: &str) -> Vec<String> {
    crate::unitext::token_strings(s)
}

// --------------------------------------------------------------------- BM25

#[derive(Serialize, Deserialize)]
pub struct Bm25 {
    postings: HashMap<String, Vec<(u32, u32)>>,
    lens: Vec<u32>,
    avg: f32,
}

impl Bm25 {
    pub fn new(chunks: &[Chunk]) -> Bm25 {
        let mut postings: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
        let mut lens = Vec::with_capacity(chunks.len());
        for (i, c) in chunks.iter().enumerate() {
            let toks = tokenize(&c.text);
            lens.push(toks.len() as u32);
            let mut tf: HashMap<&str, u32> = HashMap::new();
            for t in &toks {
                *tf.entry(t.as_str()).or_default() += 1;
            }
            for (t, n) in tf {
                postings
                    .entry(t.to_string())
                    .or_default()
                    .push((i as u32, n));
            }
        }
        let avg = if lens.is_empty() {
            1.0
        } else {
            lens.iter().sum::<u32>() as f32 / lens.len() as f32
        };
        Bm25 {
            postings,
            lens,
            avg,
        }
    }

    /// Top-`n` chunk indices with BM25 scores, best first. `keep` filters candidates before ranking.
    pub fn search(&self, query: &str, n: usize, keep: &dyn Fn(usize) -> bool) -> Vec<(usize, f32)> {
        const K1: f32 = 1.5;
        const B: f32 = 0.75;
        let big_n = self.lens.len() as f32;
        let mut acc: HashMap<u32, f32> = HashMap::new();
        let mut seen = std::collections::HashSet::new();
        for t in tokenize(query) {
            if !seen.insert(t.clone()) {
                continue;
            }
            let Some(plist) = self.postings.get(&t) else {
                continue;
            };
            let df = plist.len() as f32;
            let idf = ((big_n - df + 0.5) / (df + 0.5) + 1.0).ln();
            for &(ci, tf) in plist {
                let tf = tf as f32;
                let dl = self.lens[ci as usize] as f32;
                let s = idf * tf * (K1 + 1.0) / (tf + K1 * (1.0 - B + B * dl / self.avg));
                *acc.entry(ci).or_default() += s;
            }
        }
        let mut v: Vec<(usize, f32)> = acc
            .into_iter()
            .map(|(i, s)| (i as usize, s))
            .filter(|(i, _)| keep(*i))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }
}

/// Fraction (0..1) of the query's distinct terms that occur in `text`. A cheap, honest
/// relevance signal: low values mean the passage barely matches what was asked.
pub fn coverage(query: &str, text: &str) -> f32 {
    let q: std::collections::HashSet<String> = tokenize(query).into_iter().collect();
    if q.is_empty() {
        return 0.0;
    }
    let t: std::collections::HashSet<String> = tokenize(text).into_iter().collect();
    q.iter().filter(|w| t.contains(*w)).count() as f32 / q.len() as f32
}

// ------------------------------------------------------------------- dense

pub fn normalize(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        v.iter_mut().for_each(|x| *x /= n);
    }
}

fn dense_search(
    chunks: &[Chunk],
    q: &[f32],
    n: usize,
    keep: &dyn Fn(usize) -> bool,
) -> Vec<(usize, f32)> {
    let mut v: Vec<(usize, f32)> = chunks
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            if !keep(i) {
                return None;
            }
            let e = c.emb.as_ref()?;
            if e.len() != q.len() {
                return None;
            }
            Some((i, e.iter().zip(q).map(|(a, b)| a * b).sum()))
        })
        .collect();
    v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
    v.truncate(n);
    v
}

// ------------------------------------------------------------------ hybrid

#[derive(Clone, Debug)]
pub struct Hit {
    pub chunk: usize,
    pub score: f32,
}

pub struct Searcher<'a> {
    pub store: &'a Store,
    bm25: std::sync::Arc<Bm25>,
    /// Quoted phrases and `~k` proximity in the query are hard filters. Off by default (a quote mark is
    /// ordinary punctuation, as it always was); on with `with_phrases(true)`, i.e. `--phrases` / `TEZAUR_PHRASES=1`.
    phrases: bool,
}

/// Building the BM25 index tokenises the whole library (seconds for a few million words), and `tezaur serve` / MCP would do it on every
/// request. Keep the last one, keyed by what the library contains (document hashes, paths and total size), so an edit rebuilds it.
fn library_key(st: &Store) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut eat = |b: &[u8]| {
        for &x in b {
            h ^= x as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    };
    for d in &st.docs {
        eat(d.path.as_bytes());
        eat(&d.hash.to_le_bytes());
    }
    eat(&crate::unitext::RULES_VERSION.to_le_bytes()); // token rules changed: rebuild the cache
    eat(&(st.chunks.len() as u64).to_le_bytes());
    eat(&(st.chunks.iter().map(|c| c.text.len()).sum::<usize>() as u64).to_le_bytes());
    h
}

static BM25_CACHE: std::sync::Mutex<Option<(u64, std::sync::Arc<Bm25>)>> =
    std::sync::Mutex::new(None);

impl<'a> Searcher<'a> {
    pub fn new(store: &'a Store) -> Self {
        let key = library_key(store);
        let mut g = BM25_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((k, b)) = g.as_ref()
            && *k == key
        {
            return Searcher {
                store,
                bm25: b.clone(),
                phrases: false,
            };
        }
        // Second level: the on-disk cache next to the index, so a fresh CLI process does not re-tokenise the library.
        let disk = store.source.as_ref().map(|p| {
            let mut f = p.clone().into_os_string();
            f.push(".bm25");
            std::path::PathBuf::from(f)
        });
        if let Some(dp) = &disk
            && let Ok(bytes) = std::fs::read(dp)
            && let Ok((k, b)) = bincode::deserialize::<(u64, Bm25)>(&bytes)
            && k == key
        {
            let b = std::sync::Arc::new(b);
            *g = Some((key, b.clone()));
            return Searcher {
                store,
                bm25: b,
                phrases: false,
            };
        }
        let b = std::sync::Arc::new(Bm25::new(&store.chunks));
        if let Some(dp) = &disk
            && let Ok(bytes) = bincode::serialize(&(key, &*b))
        {
            let tmp = dp.with_extension("bm25tmp");
            if std::fs::write(&tmp, bytes).is_ok() {
                let _ = std::fs::rename(&tmp, dp);
            }
        }
        *g = Some((key, b.clone()));
        Searcher {
            store,
            bm25: b,
            phrases: false,
        }
    }

    /// Reciprocal-rank fusion of BM25 and (optionally) dense retrieval.
    /// `max_per_doc` keeps one big book from crowding out everything else.
    /// `allow`: restrict retrieval to these document ids (None = whole library).
    /// Treat `"exact phrase"` and `"a b"~k` in queries as required phrases (spec 10). Off by default.
    pub fn with_phrases(mut self, on: bool) -> Self {
        self.phrases = on;
        self
    }

    pub fn search(
        &self,
        query: &str,
        q_emb: Option<&[f32]>,
        k: usize,
        max_per_doc: usize,
        allow: Option<&std::collections::HashSet<usize>>,
    ) -> Vec<Hit> {
        self.search_with(query, q_emb, k, max_per_doc, allow, self.phrases)
    }

    /// As `search`, but the text is always plain terms: quotes and `~k` mean nothing (headings, audit), whatever `with_phrases` says.
    pub fn search_terms(
        &self,
        query: &str,
        q_emb: Option<&[f32]>,
        k: usize,
        max_per_doc: usize,
        allow: Option<&std::collections::HashSet<usize>>,
    ) -> Vec<Hit> {
        self.search_with(query, q_emb, k, max_per_doc, allow, false)
    }

    fn search_with(
        &self,
        query: &str,
        q_emb: Option<&[f32]>,
        k: usize,
        max_per_doc: usize,
        allow: Option<&std::collections::HashSet<usize>>,
        phrases: bool,
    ) -> Vec<Hit> {
        const POOL: usize = 200;
        const RRF: f32 = 60.0;
        let chunks = &self.store.chunks;
        let parsed = if phrases {
            crate::phrase::parse(query)
        } else {
            Default::default()
        };
        for w in &parsed.warnings {
            eprintln!("warning: {w}");
        }
        if let Some(e) = &parsed.error {
            eprintln!("error: query not run: {e}");
            return vec![];
        }
        // phrase syntax removed; phrase words still count as scoring terms
        let query = if phrases {
            parsed.scoring.as_str()
        } else {
            query
        };
        // passages must contain every phrase; decided once per passage, before ranking
        let verdicts: std::cell::RefCell<HashMap<usize, bool>> = Default::default();
        let phrase_ok = |ci: usize| -> bool {
            if parsed.phrases.is_empty() {
                return true;
            }
            *verdicts
                .borrow_mut()
                .entry(ci)
                .or_insert_with(|| crate::phrase::passage_ok(&parsed.phrases, &chunks[ci].text))
        };
        let keep =
            |ci: usize| allow.map(|a| a.contains(&chunks[ci].doc)).unwrap_or(true) && phrase_ok(ci);
        let mut fused: HashMap<usize, f32> = HashMap::new();
        for (rank, (ci, _)) in self.bm25.search(query, POOL, &keep).into_iter().enumerate() {
            *fused.entry(ci).or_default() += 1.0 / (RRF + rank as f32 + 1.0);
        }
        if let Some(q) = q_emb {
            for (rank, (ci, _)) in dense_search(chunks, q, POOL, &keep).into_iter().enumerate() {
                *fused.entry(ci).or_default() += 1.0 / (RRF + rank as f32 + 1.0);
            }
        }
        let mut v: Vec<Hit> = fused
            .into_iter()
            .map(|(chunk, score)| Hit { chunk, score })
            .collect();
        v.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap()
                .then(a.chunk.cmp(&b.chunk))
        });
        let mut per_doc: HashMap<usize, usize> = HashMap::new();
        let mut out = Vec::new();
        for h in v {
            let d = self.store.chunks[h.chunk].doc;
            let n = per_doc.entry(d).or_default();
            if *n >= max_per_doc {
                continue;
            }
            *n += 1;
            out.push(h);
            if out.len() >= k {
                break;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_sidecar_roundtrips_and_index_format_is_unchanged() {
        let dir = std::env::temp_dir().join(format!("tz-sha-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("i.bin");
        let mut s = mk(&["alpha beta"]);
        s.docs[0].hash = 77;
        s.sha.insert(s.docs[0].path.clone(), (77, "ab".repeat(32)));
        s.save(&p).unwrap();
        let l = Store::load(&p).unwrap();
        assert_eq!(l.doc_sha256(0), Some("ab".repeat(32).as_str()));
        // an index written without the sidecar (older version) still loads, with no identity
        std::fs::remove_file(dir.join("i.bin.sha256")).unwrap();
        assert_eq!(Store::load(&p).unwrap().doc_sha256(0), None);
        // a sidecar for a different file version is ignored
        let mut s2 = Store::load(&p).unwrap();
        s2.sha
            .insert(s2.docs[0].path.clone(), (78, "cd".repeat(32)));
        assert_eq!(s2.doc_sha256(0), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn mk(texts: &[&str]) -> Store {
        let mut s = Store::default();
        s.docs.push(Doc {
            path: "a".into(),
            title: "A".into(),
            hash: 0,
            pages: 1,
        });
        for t in texts {
            s.chunks.push(Chunk {
                doc: 0,
                page: 1,
                text: t.to_string(),
                emb: None,
            });
        }
        s
    }

    #[test]
    fn folds_romanian_diacritics() {
        assert_eq!(
            tokenize("Științific ȘI țară"),
            tokenize("stiintific si tara")
        );
    }

    #[test]
    fn ligature_query_finds_plain_text_and_back_with_same_ranking() {
        let s = mk(&[
            "The final report of the committee",
            "Cooking pasta requires boiling water",
            "A final decision and a final appeal",
        ]);
        let plain = Searcher::new(&s).search("final", None, 3, 10, None);
        let lig = Searcher::new(&s).search("\u{fb01}nal", None, 3, 10, None);
        let ids = |r: &[Hit]| r.iter().map(|h| h.chunk).collect::<Vec<_>>();
        assert_eq!(ids(&plain), ids(&lig));
        assert!(!plain.is_empty() && plain.iter().all(|h| h.chunk != 1));
        let s2 = mk(&["The \u{fb01}nal report", "Cooking pasta"]);
        assert_eq!(
            Searcher::new(&s2).search("final", None, 3, 10, None)[0].chunk,
            0
        );
    }

    #[test]
    fn phrase_queries_filter_passages() {
        let s = mk(&[
            "The burden of proof lies with the authority",
            "The proof of the burden is heavy",
            "The burden of proof is on the claimant, not the authority or the tribunal",
        ]);
        let ids = |q: &str| {
            let mut v: Vec<usize> = Searcher::new(&s)
                .with_phrases(true)
                .search(q, None, 5, 10, None)
                .iter()
                .map(|h| h.chunk)
                .collect();
            v.sort();
            v
        };
        assert_eq!(ids("\"burden of proof\""), vec![0, 2]);
        assert_eq!(ids("burden of proof"), vec![0, 1, 2]);
        // ordinary term next to a phrase: only phrase passages, and the term raises the score
        let r = Searcher::new(&s).with_phrases(true).search(
            "\"burden of proof\" authority",
            None,
            5,
            10,
            None,
        );
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|h| h.chunk != 1));
        // proximity, mistakes, one-word phrases
        assert_eq!(ids("\"proof burden\"~2"), vec![0, 2]);
        assert_eq!(ids("\"proof burden\"~3"), vec![0, 1, 2]);
        assert!(ids("\"burden proof\"~0").is_empty());
        assert_eq!(ids("\"burden of proof"), vec![0, 1, 2]);
        assert_eq!(ids("\"proof\""), ids("proof"));
        assert!(ids("\"zzzz yyyy\"").is_empty());
    }

    #[test]
    fn quotes_are_ordinary_punctuation_unless_phrases_are_switched_on() {
        let s = mk(&[
            "The burden of proof lies with the authority",
            "The proof of the burden is heavy",
            "Pasta needs boiling water",
        ]);
        let ids = |q: &str, on: bool| {
            Searcher::new(&s)
                .with_phrases(on)
                .search(q, None, 5, 10, None)
                .iter()
                .map(|h| (h.chunk, (h.score * 1e6) as i64))
                .collect::<Vec<_>>()
        };
        // default: same hits and same scores as the unquoted text; nothing is dropped, nothing aborts
        assert_eq!(
            ids("\"burden of proof\"", false),
            ids("burden of proof", false)
        );
        assert_eq!(ids("what is the \"clean room\" burden?", false).len(), 2);
        assert_eq!(
            ids("\"proof burden\"~3", false),
            ids("proof burden 3", false)
        );
        assert_eq!(ids("\"zzzz yyyy\" burden", false), ids("burden", false));
        assert!(!ids("\"proof burden\"~x", false).is_empty());
        // opt-in: the filter applies
        assert_eq!(ids("\"burden of proof\"", true).len(), 1);
        assert!(ids("\"zzzz yyyy\" burden", true).is_empty());
    }

    #[test]
    fn phrase_does_not_cross_a_page_boundary() {
        let mut s = mk(&[
            "The burden of",
            "proof lies here",
            "Then the burden of proof returns",
        ]);
        s.chunks[0].page = 3;
        s.chunks[1].page = 4;
        s.chunks[2].page = 4;
        let r =
            Searcher::new(&s)
                .with_phrases(true)
                .search("\"burden of proof\"", None, 5, 10, None);
        assert_eq!(r.iter().map(|h| h.chunk).collect::<Vec<_>>(), vec![2]);
    }

    #[test]
    fn bm25_ranks_relevant_first() {
        let s = mk(&[
            "The recognition of professional qualifications under the directive",
            "Cooking pasta requires boiling water and salt",
            "Credential evaluation and recognition of diplomas",
        ]);
        let r = Searcher::new(&s).search("recognition of qualifications", None, 3, 10, None);
        assert_eq!(r[0].chunk, 0);
        assert!(r.iter().all(|h| h.chunk != 1));
    }

    #[test]
    fn prefix_stemming_matches_inflections() {
        let s = mk(&[
            "recunoașterea calificărilor profesionale",
            "ceva fără legătură",
        ]);
        let r = Searcher::new(&s).search("recunoastere calificare profesionala", None, 2, 10, None);
        assert_eq!(r[0].chunk, 0);
    }

    #[test]
    fn dense_fusion_and_doc_cap() {
        let mut s = mk(&["alpha", "alpha beta", "alpha gamma"]);
        s.chunks[2].emb = Some(vec![1.0, 0.0]);
        s.chunks[0].emb = Some(vec![0.0, 1.0]);
        let sr = Searcher::new(&s);
        let r = sr.search("alpha", Some(&[1.0, 0.0]), 10, 2, None);
        assert_eq!(r.len(), 2, "per-doc cap applies");
        // No lexical overlap: only the dense channel can find anything.
        let r = sr.search("zzzz", Some(&[1.0, 0.0]), 10, 10, None);
        assert_eq!(r[0].chunk, 2, "dense-only retrieval ranks by cosine");
        // Lexical + dense both contribute to fusion.
        let r = sr.search("alpha", Some(&[1.0, 0.0]), 10, 10, None);
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn only_filter_restricts_documents() {
        let mut s = mk(&["alpha one", "alpha two"]);
        s.docs.push(Doc {
            path: "b".into(),
            title: "B".into(),
            hash: 0,
            pages: 1,
        });
        s.chunks.push(Chunk {
            doc: 1,
            page: 1,
            text: "alpha three".into(),
            emb: None,
        });
        let allow: std::collections::HashSet<usize> = [1].into_iter().collect();
        let r = Searcher::new(&s).search("alpha", None, 10, 10, Some(&allow));
        assert_eq!(r.len(), 1);
        assert_eq!(s.chunks[r[0].chunk].doc, 1);
    }

    #[test]
    fn coverage_measures_term_overlap() {
        assert!((coverage("burden of proof", "the burden of proof lies here") - 1.0).abs() < 1e-6);
        assert!(coverage("burden of proof", "pasta and olive oil") < 0.01);
        assert!((coverage("burden pasta", "the burden is heavy") - 0.5).abs() < 1e-6);
    }

    #[test]
    fn remove_doc_remaps_ids() {
        let mut s = mk(&["x"]);
        s.docs.push(Doc {
            path: "b".into(),
            title: "B".into(),
            hash: 0,
            pages: 1,
        });
        s.chunks.push(Chunk {
            doc: 1,
            page: 1,
            text: "y".into(),
            emb: None,
        });
        s.remove_doc(0);
        assert_eq!(s.chunks.len(), 1);
        assert_eq!(s.chunks[0].doc, 0);
    }
}
