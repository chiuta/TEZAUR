//! TEZAUR — a local, citation-grounded research assistant for writing a thesis.
//!
//! Library of PDFs/books/articles  ->  chunk + index (BM25, optional embeddings)
//!   ->  retrieve  ->  local LLM (Ollama, Jan, LM Studio, llama.cpp…) writes ONLY from retrieved
//!   excerpts, with citations that are checked afterwards.
//!
//! Portable: when run from a PortableApps.com-style folder (App\Tezaur\tezaur.exe + Data\),
//! the index, library and settings all live in Data\ and nothing is written elsewhere.

mod audit;
mod bib;
mod eval;
mod export;
mod index;
mod ingest;
mod llm;
mod mesh;
mod online;
mod pdfx;
mod portable;
mod prompt;
mod serve;
mod sha256;
mod verify;

use anyhow::{Result, bail};
use bib::BibMap;
use clap::{Parser, Subcommand, ValueEnum};
use index::{Hit, Searcher, Store, coverage};
use llm::{Api, Llm};
use std::cell::RefCell;
use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "tezaur",
    version,
    about = "Local thesis-writing assistant grounded in your own library"
)]
struct Cli {
    /// Index file. Default: Data/tezaur.idx when portable, else ./tezaur.idx.
    #[arg(long, env = "TEZAUR_INDEX", global = true)]
    index: Option<PathBuf>,
    /// Model server protocol: ollama, jan, or openai (any OpenAI-compatible server: LM Studio, llama.cpp…).
    #[arg(long, env = "TEZAUR_API", value_enum, default_value_t = Api::Ollama, global = true)]
    api: Api,
    /// Server address. Default depends on --api (Ollama :11434, Jan http://127.0.0.1:1337/v1).
    /// For Ollama the OLLAMA_HOST variable is also honoured.
    #[arg(long, env = "TEZAUR_HOST", global = true)]
    host: Option<String>,
    /// API key, if the server requires one (Jan: only if you set one in Local API Server settings).
    #[arg(long, env = "TEZAUR_API_KEY", hide_env_values = true, global = true)]
    api_key: Option<String>,
    /// Generation model id as the server names it (`tezaur models` lists them).
    #[arg(
        long,
        env = "TEZAUR_MODEL",
        default_value = "llama3.1:8b",
        global = true
    )]
    model: String,
    /// Embedding model for hybrid retrieval, e.g. bge-m3 (multilingual) or nomic-embed-text.
    /// Optional: without it, retrieval is BM25 only and needs no model at all.
    #[arg(long, env = "TEZAUR_EMBED_MODEL", global = true)]
    embed_model: Option<String>,
    /// Context window requested from the model (Ollama only; for Jan/LM Studio set it in their model settings).
    #[arg(long, env = "TEZAUR_NUM_CTX", default_value_t = 16384, global = true)]
    num_ctx: u32,
    /// Restrict retrieval to documents whose title, path or citation contains this text.
    #[arg(long, global = true)]
    only: Option<String>,
    /// Extra BibTeX file(s) for author/year citations. *.bib files in Data/ are found automatically.
    #[arg(long, global = true)]
    bib: Vec<PathBuf>,
    /// Without a command, starts the interactive mode.
    /// Re-order the retrieved passages with the model before answering (slower, often more relevant).
    #[arg(long, global = true)]
    rerank: bool,
    /// After the answer, ask the model whether each cited passage really supports its sentence.
    #[arg(long, global = true)]
    verify: bool,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ExportTo {
    Tex,
    Docx,
    Html,
    /// Needs pandoc (and LaTeX for PDF).
    Pdf,
    Odt,
    Epub,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum BibTo {
    Ris,
    CslJson,
    Bibtex,
}

#[derive(Subcommand, Debug)]
enum MeshCmd {
    /// Show this node, what it shares and its peers.
    Status,
    /// Name this node.
    Name { name: String },
    /// Offer documents to peers: `*` = everything, otherwise text that appears in the path or title. Empty list = share nothing.
    Share {
        patterns: Vec<String>,
        /// Also re-offer documents received from other peers.
        #[arg(long)]
        relay: bool,
    },
    /// Add or replace a peer (a running `tezaur serve`, e.g. http://192.168.1.20:8787).
    Add {
        name: String,
        url: String,
        #[arg(long)]
        token: Option<String>,
        /// e.g. socks5://127.0.0.1:9050 for a Tor onion address
        #[arg(long)]
        proxy: Option<String>,
    },
    /// Forget a peer.
    Rm { name: String },
    /// Copy the shared documents of one peer (or all) into your index.
    Pull { name: Option<String> },
    /// Search peers (nothing is copied; only matching snippets come back).
    Search {
        query: Vec<String>,
        #[arg(short, default_value_t = 8)]
        k: usize,
    },
    /// Write a bundle (.tzb) of the shared documents for USB / e-mail exchange. --all ignores the sharing rules.
    Export {
        file: PathBuf,
        #[arg(long)]
        all: bool,
    },
    /// Read a bundle written by `mesh export`.
    Import { file: PathBuf },
    /// Find TEZAUR servers on the local network (UDP broadcast). Servers answer when bound to a network address.
    Discover {
        #[arg(long, default_value = "255.255.255.255:8788")]
        target: String,
        #[arg(long, default_value_t = 2)]
        seconds: u64,
    },
}

/// What kind of text `draft` should write.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum Kind {
    /// A thesis section on a topic (default).
    Section,
    /// A thematic literature review (compares sources instead of listing them).
    LitReview,
    /// An introduction: context, problem, what is known and open — your aims left as a placeholder.
    Introduction,
    /// A critical review: for each source, claim, evidence and limits.
    CriticalReview,
}

fn draft_task(kind: Kind, section: &str) -> String {
    match kind {
        Kind::Section => format!(
            "Draft the thesis section titled \"{section}\" in formal academic prose, 400-700 words, \
             structured in coherent paragraphs. Synthesise across sources, note disagreements, and \
             end with a short list of gaps the excerpts leave open."
        ),
        Kind::LitReview => format!(
            "Write a literature review on \"{section}\" (500-800 words). Organise it by theme, not source by \
             source. Compare approaches and findings, mark consensus and disagreement, and judge the strength of \
             evidence only where the excerpts allow. Close with one or two questions the literature leaves open, \
             phrased as questions to investigate, not as established facts."
        ),
        Kind::Introduction => format!(
            "Write an introduction for a thesis chapter on \"{section}\" (300-500 words): context, the problem and \
             why it matters, what is known (cited), what remains open (cited or marked as not in the excerpts). \
             Do NOT invent the thesis's own aims, method or results: where they belong, write the placeholder \
             [AUTHOR: state aim and research questions]."
        ),
        Kind::CriticalReview => format!(
            "Write a critical review of the sources on \"{section}\". For each source present in the excerpts give: \
             its claim, the evidence it offers, and its limits as far as the excerpts show them. Then compare the \
             sources in two or three sentences. Never assess evidence the excerpts do not show."
        ),
    }
}

/// In portable mode a bare file name lands in Data/Output, not in a random working directory.
fn resolve_output(ctx: &Ctx, p: &Path) -> PathBuf {
    match (&ctx.data, p.is_relative() && p.components().count() == 1) {
        (Some(d), true) => {
            let dir = d.join("Output");
            let _ = std::fs::create_dir_all(&dir);
            dir.join(p)
        }
        _ => p.to_path_buf(),
    }
}

/// One interactive line, parsed with the same command definitions.
#[derive(Parser)]
#[command(no_binary_name = true, disable_help_subcommand = true)]
struct Line {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Add files or folders (pdf, docx, epub, txt, md, html, tex) to the index. Re-running only
    /// processes changes. With no paths, portable mode indexes Data/Library.
    Ingest {
        paths: Vec<PathBuf>,
        /// Re-read files even if unchanged (after upgrading TEZAUR, to pick up better chunking/readers).
        #[arg(long)]
        force: bool,
    },
    /// Show the best matching passages (no LLM needed), with how many query terms each covers.
    Search {
        query: String,
        #[arg(short, default_value_t = 8)]
        k: usize,
    },
    /// Answer a question using only your library, with citations.
    Ask {
        question: String,
        #[arg(short, default_value_t = 10)]
        k: usize,
        /// First let the model split the question into several search queries (slower, broader).
        #[arg(long)]
        decompose: bool,
        #[arg(long)]
        lang: Option<String>,
        /// Also write a tamper-evident receipt (JSON) for this answer: question, model, answer hash, excerpt hashes.
        #[arg(long)]
        receipt: Option<PathBuf>,
    },
    /// Compare how different sources treat a question: positions, agreements, disagreements.
    Compare {
        question: String,
        #[arg(short, default_value_t = 14)]
        k: usize,
        #[arg(long)]
        lang: Option<String>,
    },
    /// Map what the library establishes on a topic, what is weak, and what it leaves open.
    Gaps {
        topic: String,
        #[arg(short, default_value_t = 16)]
        k: usize,
        #[arg(long)]
        lang: Option<String>,
    },
    /// Summarise one whole document (map-reduce over all its passages; slow on big books).
    Summarize {
        /// Part of the document's title, path or citation key.
        document: String,
        #[arg(long)]
        lang: Option<String>,
    },
    /// Draft a thesis section from your library. `--notes` lines steer retrieval and content.
    Draft {
        section: String,
        /// section (default), lit-review, introduction, critical-review.
        #[arg(long, value_enum, default_value_t = Kind::Section)]
        kind: Kind,
        #[arg(long)]
        notes: Option<String>,
        #[arg(short, default_value_t = 14)]
        k: usize,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        lang: Option<String>,
    },
    /// Propose a chapter outline for a topic, with the sources that support each part.
    Outline {
        topic: String,
        #[arg(short, default_value_t = 16)]
        k: usize,
        #[arg(long)]
        lang: Option<String>,
    },
    /// Make study flashcards from your sources (each answer must cite its excerpt); `-o cards.tsv`
    /// writes a tab-separated file Anki can import.
    Flashcards {
        topic: String,
        #[arg(short, default_value_t = 12)]
        k: usize,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        lang: Option<String>,
    },
    /// List indexed documents (with their BibTeX citation when one matches).
    Sources,
    /// Convert a Markdown draft (from `draft -o`) to LaTeX, Word or HTML. `[@key]` citations become \cite{key}.
    Export {
        input: PathBuf,
        #[arg(long, value_enum, default_value_t = ExportTo::Tex)]
        to: ExportTo,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        title: Option<String>,
        /// BibTeX file to reference from the LaTeX output (its name, without .bib, goes into \bibliography).
        #[arg(long)]
        bib_name: Option<String>,
        /// For pdf/odt/epub (pandoc): a .bib or CSL JSON file to resolve [@key] citations into a reference list.
        #[arg(long)]
        bibfile: Option<PathBuf>,
    },
    /// Check a draft: does each [@key, p. N] citation point at a passage that covers the sentence? Also verifies
    /// quotations word-for-word, and flags wrong pages, unknown keys and long uncited statements. No model needed.
    Audit {
        file: PathBuf,
        #[arg(long)]
        json: bool,
        /// Exit with an error code when any problem is found (for CI / pre-commit).
        #[arg(long)]
        fail: bool,
        /// Second opinion from the local model on every cited sentence: rescues correct paraphrases, catches negations.
        /// Slow (one model call per citation) and the model can be wrong; changed findings are marked [model].
        #[arg(long)]
        semantic: bool,
    },
    /// Reference hygiene for a draft: cited keys found nowhere, .bib entries and library documents never cited. No model needed.
    Refs {
        file: PathBuf,
        #[arg(long)]
        json: bool,
        /// Also check every reference with a DOI/title against OpenAlex and Crossref (existence, retraction). Sends titles/DOIs to those services.
        #[arg(long)]
        online: bool,
    },
    /// Measure retrieval on your own library: a JSONL of {"q","doc","page"?} lines → recall@1/3/10 and MRR. No model needed.
    Eval {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Tables found in a PDF (aligned columns in the text layer), as Markdown, CSV (`-o DIR` writes one file per table) or JSON. Heuristic; needs poppler's pdftotext.
    Tables {
        file: PathBuf,
        #[arg(long)]
        page: Option<usize>,
        #[arg(long)]
        csv: bool,
        #[arg(long)]
        json: bool,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Display equations found in a PDF, as LaTeX-style text with their numbers. Heuristic: single-line equations, big operators with limits;
    /// stacked fractions and matrices are not reconstructed. Needs poppler's pdftotext.
    Equations {
        file: PathBuf,
        #[arg(long)]
        page: Option<usize>,
        #[arg(long)]
        json: bool,
    },
    /// Search the world's literature (OpenAlex, Crossref). Opt-in and online: your query leaves this computer. Prints BibTeX with --bibtex.
    Lit {
        query: String,
        #[arg(short, long, default_value_t = 10)]
        n: usize,
        #[arg(long)]
        bibtex: bool,
        #[arg(long)]
        json: bool,
    },
    /// Which sections of an outline have sources in the library? A heading × source matrix. No model needed.
    Coverage {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// What does a paper or draft add beyond the library? Passages already covered vs new. No model needed.
    Novelty {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Verify a receipt written by `ask --receipt`: untouched, answer matches, excerpts still in the index.
    Receipt {
        file: PathBuf,
        /// The answer text the receipt was issued for.
        #[arg(long)]
        answer: Option<PathBuf>,
    },
    /// Write an AI-use statement from receipts (for the thesis front matter or a journal's disclosure form).
    Declare {
        receipts: Vec<PathBuf>,
        /// Language of the statement: ro or en.
        #[arg(long, default_value = "en")]
        lang: String,
    },
    /// Export the bibliography (all loaded .bib files) as RIS, CSL JSON or BibTeX — for Zotero, Mendeley, pandoc, Word.
    Bib {
        #[arg(long, value_enum, default_value_t = BibTo::Ris)]
        to: BibTo,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Pull your Zotero library: fetch BibTeX from Zotero's local API (Zotero must be running with "Allow other
    /// applications to communicate with Zotero" on) and index the attachments in Zotero's storage folder.
    Zotero {
        /// Zotero data folder (default: ~/Zotero).
        #[arg(long)]
        data: Option<PathBuf>,
        #[arg(long, default_value_t = 23119)]
        port: u16,
        /// Re-read files even if unchanged.
        #[arg(long)]
        force: bool,
    },
    /// Start a local web server: the web app, a JSON API over your index (native PDF/EPUB/OCR, embeddings),
    /// index sync, named workspaces and a same-origin proxy to the model server.
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
        #[arg(long, default_value_t = 8787)]
        port: u16,
        /// Required to open the server to the network (--bind 0.0.0.0).
        #[arg(long, env = "TEZAUR_TOKEN")]
        token: Option<String>,
        /// Folder of named workspaces (<name>.idx + <name>/Library): one per thesis chapter or per student.
        #[arg(long)]
        workspaces: Option<PathBuf>,
    },
    /// Peer-to-peer exchange between TEZAUR nodes: federated search, pull shared documents, offline bundles, LAN discovery.
    Mesh {
        #[command(subcommand)]
        sub: MeshCmd,
    },
    /// Run as an MCP server over stdio (for Claude Desktop, Claude Code, Cursor…): exposes your library
    /// as read-only tools — search, list documents, read a document. No model server is needed.
    Mcp,
    /// List the models the server offers and check that the configured one is among them.
    Models,
    /// Check the whole setup (server, model, embeddings, index, BibTeX) and say what to fix.
    Doctor {
        /// Also run a tiny real generation to measure speed (can be slow on big models).
        #[arg(long)]
        generate: bool,
    },
}

pub(crate) struct Ctx {
    index: PathBuf,
    /// Folder the index lives in; documents under it are stored by relative path.
    base: PathBuf,
    data: Option<PathBuf>,
    llm: Llm,
    embed_model: Option<String>,
    bib_files: Vec<PathBuf>,
    /// Session-wide document filter (`--only`, or `/only` in interactive mode).
    only: RefCell<Option<String>>,
    rerank: bool,
    verify: bool,
}

impl Ctx {
    /// Re-read per command so a freshly exported .bib is picked up without restarting.
    fn load_bib(&self) -> BibMap {
        let depth = if self.data.is_some() { 3 } else { 1 };
        BibMap::load(std::slice::from_ref(&self.base), &self.bib_files, depth)
    }
}

pub(crate) fn embed_query(ctx: &Ctx, store: &Store, q: &str) -> Option<Vec<f32>> {
    let model = ctx
        .embed_model
        .clone()
        .or_else(|| store.embed_model.clone())?;
    if store.embed_model.as_deref() != Some(model.as_str()) {
        eprintln!("note: index was not built with embedding model '{model}'; using BM25 only");
        return None;
    }
    match ctx.llm.embed(&model, &[q.to_string()]) {
        Ok(mut v) => v.pop(),
        Err(e) => {
            eprintln!("warning: embedding failed ({e}); falling back to BM25 only");
            None
        }
    }
}

fn allowed_docs(ctx: &Ctx, store: &Store, bib: &BibMap) -> Option<HashSet<usize>> {
    let only = ctx.only.borrow();
    let q = only.as_ref()?.to_lowercase();
    let set: HashSet<usize> = store
        .docs
        .iter()
        .enumerate()
        .filter(|(_, d)| {
            d.title.to_lowercase().contains(&q)
                || d.path.to_lowercase().contains(&q)
                || bib
                    .find(&d.path)
                    .map(|e| {
                        e.cite().to_lowercase().contains(&q) || e.key.to_lowercase().contains(&q)
                    })
                    .unwrap_or(false)
        })
        .map(|(i, _)| i)
        .collect();
    if set.is_empty() {
        eprintln!("⚠ --only '{q}' matches no document (see /sources)");
    }
    Some(set)
}

/// Retrieve for several queries, merge by best rank, cap at k. `per_doc` limits passages per document.
fn retrieve(
    ctx: &Ctx,
    store: &Store,
    bib: &BibMap,
    queries: &[String],
    k: usize,
    per_doc: usize,
) -> Vec<Hit> {
    if ctx.rerank {
        // Fetch a wider pool, let the model order it, keep the best k.
        let pool = retrieve_inner(ctx, store, bib, queries, k * 2, per_doc);
        let mut v = rerank_hits(ctx, store, &queries.join("; "), pool);
        v.truncate(k);
        return v;
    }
    retrieve_inner(ctx, store, bib, queries, k, per_doc)
}

fn retrieve_inner(
    ctx: &Ctx,
    store: &Store,
    bib: &BibMap,
    queries: &[String],
    k: usize,
    per_doc: usize,
) -> Vec<Hit> {
    let allow = allowed_docs(ctx, store, bib);
    let s = Searcher::new(store);
    let per = k.max(4);
    let mut rounds: Vec<Vec<Hit>> = Vec::new();
    for q in queries {
        let emb = embed_query(ctx, store, q);
        let hits = s.search(q, emb.as_deref(), per, per_doc, allow.as_ref());
        // Without embeddings, term overlap is a fair relevance signal; warn when it is poor.
        if emb.is_none() && !hits.is_empty() {
            let best = hits
                .iter()
                .take(3)
                .map(|h| coverage(q, &store.chunks[h.chunk].text))
                .fold(0.0, f32::max);
            if best < 0.4 {
                eprintln!(
                    "⚠ weak match for \"{q}\": the best passage covers only {:.0}% of its terms — the answer may lack support; rephrase or add sources",
                    best * 100.0
                );
            }
        }
        rounds.push(hits);
    }
    // Round-robin so every query contributes its best passages.
    let mut merged: Vec<Hit> = Vec::new();
    let mut i = 0;
    while merged.len() < k && rounds.iter().any(|r| i < r.len()) {
        for r in &rounds {
            if let Some(h) = r.get(i)
                && !merged.iter().any(|m| m.chunk == h.chunk)
                && merged.len() < k
            {
                merged.push(h.clone());
            }
        }
        i += 1;
    }
    merged
}

/// BibTeX of the whole library through Zotero's local API (same paging as the web API). (text, items).
/// PDF through our own LaTeX output + pdflatex (+ bibtex when a .bib file is given). No pandoc needed.
fn export_pdf(md: &str, title: Option<&str>, bib: Option<&Path>, out: &Path) -> Result<()> {
    let dir = std::env::temp_dir().join(format!("tezaur-pdf-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let stem = bib
        .filter(|b| b.extension().map(|e| e == "bib").unwrap_or(false))
        .and_then(|b| b.file_stem())
        .map(|s| s.to_string_lossy().to_string());
    if let (Some(b), Some(st)) = (bib, &stem) {
        std::fs::copy(b, dir.join(format!("{st}.bib")))?;
    }
    std::fs::write(
        dir.join("doc.tex"),
        export::to_tex(md, title, stem.as_deref()),
    )?;
    let run = |prog: &str, arg: &str| -> Result<bool> {
        match std::process::Command::new(prog)
            .args(if prog == "pdflatex" {
                vec!["-interaction=nonstopmode", "-halt-on-error", arg]
            } else {
                vec![arg]
            })
            .current_dir(&dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
        {
            Ok(s) => Ok(s.success()),
            Err(_) => bail!(
                "PDF export needs LaTeX (`{prog}` not found on PATH); alternatively export --to docx/odt/html"
            ),
        }
    };
    if !run("pdflatex", "doc.tex")? {
        bail!("pdflatex failed; keep the .tex with `export --to tex` and inspect it");
    }
    if stem.is_some() {
        run("bibtex", "doc")?;
        run("pdflatex", "doc.tex")?;
    }
    run("pdflatex", "doc.tex")?;
    std::fs::copy(dir.join("doc.pdf"), out)?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn zotero_bibtex(port: u16) -> Result<(String, usize)> {
    let mut all = String::new();
    let mut n = 0usize;
    for page in 0..200 {
        let url = format!(
            "http://127.0.0.1:{port}/api/users/0/items?format=bibtex&limit=100&start={}",
            page * 100
        );
        let body = ureq::get(&url)
            .set("Zotero-API-Version", "3")
            .call()?
            .into_string()?;
        let items = body.matches("\n@").count() + usize::from(body.trim_start().starts_with('@'));
        if body.trim().is_empty() || items == 0 {
            break;
        }
        n += items;
        all.push_str(&body);
        all.push('\n');
        if items < 100 {
            break;
        }
    }
    Ok((all, n))
}

fn rerank_hits(ctx: &Ctx, store: &Store, query: &str, hits: Vec<Hit>) -> Vec<Hit> {
    if hits.len() < 2 {
        return hits;
    }
    let mut p = format!("QUERY: {query}\n\nPASSAGES:\n");
    for (i, h) in hits.iter().enumerate() {
        let t: String = store.chunks[h.chunk].text.chars().take(700).collect();
        p.push_str(&format!("[{}] {}\n\n", i + 1, t));
    }
    eprintln!(
        "[re-ranking {} passages with {}]",
        hits.len(),
        ctx.llm.model
    );
    match ctx.llm.chat(verify::SYSTEM_RERANK, &p, |_| {}) {
        Ok(t) => match verify::rerank_order(&t, hits.len()) {
            Some(order) => order.into_iter().map(|i| hits[i].clone()).collect(),
            None => {
                eprintln!("warning: the model returned no usable scores; keeping the search order");
                hits
            }
        },
        Err(e) => {
            eprintln!("warning: re-ranking failed ({e}); keeping the search order");
            hits
        }
    }
}

/// Ask the local model for a second opinion on each cited sentence (see audit::semantic_pass).
pub(crate) fn semantic_audit(ctx: &Ctx, f: &mut [audit::Finding]) -> (usize, usize) {
    let mut failed = false;
    audit::semantic_pass(f, &mut |sentence, passage| {
        if failed {
            return None;
        }
        let u = format!("PASSAGE:\n{passage}\n\nSENTENCE: {sentence}");
        match ctx.llm.chat(audit::SYSTEM_SEMANTIC, &u, |_| {}) {
            Ok(t) => Some(audit::parse_sem(&t)),
            Err(e) => {
                eprintln!("warning: semantic pass stopped ({e})");
                failed = true;
                None
            }
        }
    })
}

fn verify_answer(ctx: &Ctx, store: &Store, hits: &[Hit], answer: &str) {
    use verify::Verdict;
    let sents = verify::cite_sentences(answer, hits.len(), 15);
    if sents.is_empty() {
        return;
    }
    eprintln!(
        "[verifying {} cited sentences — the model can be wrong here too]",
        sents.len()
    );
    let (mut ok, mut checked) = (0usize, 0usize);
    for (s, nums) in &sents {
        let mut u = String::from("EXCERPTS:\n");
        for n in nums {
            let t: String = store.chunks[hits[n - 1].chunk]
                .text
                .chars()
                .take(1200)
                .collect();
            u.push_str(&format!("[{n}] {t}\n\n"));
        }
        u.push_str(&format!("SENTENCE: {s}"));
        match ctx.llm.chat(verify::SYSTEM_VERIFY, &u, |_| {}) {
            Ok(t) => {
                checked += 1;
                match verify::parse_verdict(&t) {
                    Verdict::Supported => ok += 1,
                    Verdict::Partial => eprintln!("⚠ verify: only PARTIALLY supported — {s}"),
                    Verdict::Unsupported => eprintln!("⚠ verify: UNSUPPORTED by its excerpt — {s}"),
                    Verdict::Unclear => eprintln!("· verify: unclear answer for — {s}"),
                }
            }
            Err(e) => {
                eprintln!("warning: verification stopped ({e})");
                break;
            }
        }
    }
    eprintln!("verify: {ok}/{checked} cited sentences fully supported");
}

fn generate(
    ctx: &Ctx,
    store: &Store,
    bib: &BibMap,
    hits: &[Hit],
    task: &str,
    lang: Option<&str>,
) -> Result<String> {
    if hits.is_empty() {
        bail!(
            "nothing relevant found in the index — ingest more material, rephrase, or clear /only"
        );
    }
    let user = prompt::user_prompt(store, bib, hits, task, lang);
    let ctx_note = if ctx.llm.api == Api::Ollama {
        format!(", ctx {}", ctx.llm.num_ctx)
    } else {
        String::new()
    };
    eprintln!(
        "[{} excerpts, model {}{} — local generation can be slow]",
        hits.len(),
        ctx.llm.model,
        ctx_note
    );
    let out = ctx.llm.chat(prompt::SYSTEM, &user, |t| {
        print!("{t}");
        let _ = std::io::stdout().flush();
    })?;
    println!();
    Ok(out)
}

fn report(ctx: &Ctx, store: &Store, bib: &BibMap, hits: &[Hit], answer: &str) -> String {
    if ctx.verify {
        verify_answer(ctx, store, hits, answer);
    }
    let r = prompt::check(store, bib, hits, answer);
    let mut s = String::new();
    if !r.references.is_empty() {
        s.push_str("\n## References (excerpts cited above)\n");
        s.push_str(&r.references);
    }
    for w in &r.warnings {
        eprintln!("⚠ {w}");
    }
    s
}

/// Non-printing ask/compare/gaps for the HTTP API: answer, checked references, warnings and the excerpts used.
fn ask_json(
    ctx: &Ctx,
    store: &Store,
    bib: &BibMap,
    mode: &str,
    q: &str,
    k: usize,
    lang: Option<&str>,
) -> Result<serde_json::Value> {
    let (queries, per_doc, task) = match mode {
        "compare" => (
            vec![q.to_string()],
            2,
            format!(
                "Compare how the different sources address the question below. Group the sources by position or finding and cite the excerpts for each group. Then state plainly where the sources agree, where they disagree and why, and say so if only one source addresses a point. Do not settle disagreements with outside knowledge.\n\nQUESTION: {q}"
            ),
        ),
        "gaps" => (
            vec![
                q.to_string(),
                format!("{q} limitations and criticism"),
                format!("{q} further research needed, open questions, future work"),
            ],
            3,
            format!(
                "Using only the excerpts, map the state of the library on \"{q}\": (a) what it establishes well, (b) points that are weakly supported or contradictory, (c) questions the excerpts raise but do not answer, (d) kinds of sources that seem to be missing. Cite excerpts for (a)–(c); for (d) write 'not in excerpts'. State explicitly that this view is limited to the retrieved excerpts."
            ),
        ),
        _ => (
            vec![q.to_string()],
            4,
            format!(
                "Answer the question below as precisely as the excerpts allow.\n\nQUESTION: {q}"
            ),
        ),
    };
    let hits = retrieve(ctx, store, bib, &queries, k, per_doc);
    if hits.is_empty() {
        bail!("nothing relevant found in the index");
    }
    let user = prompt::user_prompt(store, bib, &hits, &task, lang);
    let answer = ctx.llm.chat(prompt::SYSTEM, &user, |_| {})?;
    let r = prompt::check(store, bib, &hits, &answer);
    let excerpts: Vec<serde_json::Value> = hits.iter().enumerate().map(|(i, h)| {
        let c = &store.chunks[h.chunk];
        serde_json::json!({"n": i + 1, "label": prompt::label(store, bib, h), "page": c.page, "path": store.docs[c.doc].path, "text": c.text})
    }).collect();
    let receipt = audit::make_receipt(
        store,
        &ctx.llm.model,
        mode,
        q,
        &hits.iter().map(|h| h.chunk).collect::<Vec<_>>(),
        &answer,
    );
    Ok(
        serde_json::json!({"answer": answer, "references": r.references, "warnings": r.warnings, "excerpts": excerpts, "receipt": receipt}),
    )
}

/// Ask the model for 2–4 search queries covering different aspects of the question.
fn decompose(ctx: &Ctx, question: &str) -> Vec<String> {
    match ctx.llm.chat(prompt::SYSTEM_QUERIES, question, |_| {}) {
        Ok(t) => {
            let mut v: Vec<String> = t
                .lines()
                .map(|l| {
                    l.trim()
                        .trim_start_matches(|c: char| {
                            c.is_ascii_digit() || matches!(c, '-' | '*' | '•' | '.' | ')' | ' ')
                        })
                        .trim()
                        .trim_matches('"')
                        .to_string()
                })
                .filter(|l| !l.is_empty() && l.chars().count() < 200)
                .take(4)
                .collect();
            v.insert(0, question.to_string());
            v.dedup();
            eprintln!(
                "search queries: {}",
                v.iter()
                    .map(|q| format!("“{q}”"))
                    .collect::<Vec<_>>()
                    .join(" · ")
            );
            v
        }
        Err(e) => {
            eprintln!("warning: could not decompose the question ({e}); using it as is");
            vec![question.to_string()]
        }
    }
}

fn find_doc(store: &Store, bib: &BibMap, q: &str) -> Result<usize> {
    let ql = q.to_lowercase();
    let m: Vec<usize> = store
        .docs
        .iter()
        .enumerate()
        .filter(|(_, d)| {
            d.title.to_lowercase().contains(&ql)
                || d.path.to_lowercase().contains(&ql)
                || bib
                    .find(&d.path)
                    .map(|e| {
                        e.cite().to_lowercase().contains(&ql) || e.key.to_lowercase().contains(&ql)
                    })
                    .unwrap_or(false)
        })
        .map(|(i, _)| i)
        .collect();
    match m.len() {
        0 => bail!("no document matches '{q}' (see /sources)"),
        1 => Ok(m[0]),
        n => {
            let list: Vec<String> = m
                .iter()
                .take(8)
                .map(|&i| format!("  - {}  ({})", store.docs[i].title, store.docs[i].path))
                .collect();
            bail!(
                "'{q}' matches {n} documents — be more specific:\n{}",
                list.join("\n")
            )
        }
    }
}

/// Embeds every chunk that lacks a vector, when an embedding model is configured. A failure never loses
/// the documents just indexed: keyword search keeps working and the next run resumes.
fn embed_pending(ctx: &Ctx, store: &mut Store, index: &Path) {
    // Embed any chunk that lacks a vector, when an embedding model is configured.
    // A failure here must never lose the documents just indexed: keyword search still works.
    if let Some(em) = &ctx.embed_model {
        if store.embed_model.as_deref().is_some_and(|m| m != em) {
            eprintln!(
                "index uses '{}' — dropping old vectors to switch to '{em}'",
                store.embed_model.as_ref().unwrap()
            );
            store.chunks.iter_mut().for_each(|c| c.emb = None);
            store.embed_model = None;
        }
        let todo: Vec<usize> = (0..store.chunks.len())
            .filter(|&i| store.chunks[i].emb.is_none())
            .collect();
        let mut stopped: Option<String> = None;
        for (b, batch) in todo.chunks(32).enumerate() {
            eprint!(
                "\rembedding {}/{} ",
                (b * 32 + batch.len()).min(todo.len()),
                todo.len()
            );
            let texts: Vec<String> = batch
                .iter()
                .map(|&i| store.chunks[i].text.clone())
                .collect();
            match ctx.llm.embed(em, &texts) {
                Ok(vecs) => {
                    store.embed_model = Some(em.clone());
                    for (&i, v) in batch.iter().zip(vecs) {
                        store.chunks[i].emb = Some(v);
                    }
                    if b % 20 == 19 {
                        let _ = store.save(index); // checkpoint: long jobs survive interruption
                    }
                }
                Err(e) => {
                    stopped = Some(e.to_string());
                    break;
                }
            }
        }
        if !todo.is_empty() {
            eprintln!();
        }
        if let Some(e) = stopped {
            eprintln!(
                "⚠ embedding stopped: {e}\n  Keyword search still works. Fix the model/server and run /ingest again to resume."
            );
        }
    }
}

fn ingest_paths(ctx: &Ctx, paths: &[PathBuf], force: bool) -> Result<()> {
    let mut store = Store::load(&ctx.index)?;
    let files = ingest::collect_files(paths);
    if files.is_empty() {
        eprintln!(
            "no supported files found (pdf, docx, epub, txt, md, html, tex) in: {}",
            paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let (mut added, mut same, mut failed) = (0usize, 0usize, 0usize);
    for (n, f) in files.iter().enumerate() {
        eprint!("[{}/{}] {} … ", n + 1, files.len(), f.display());
        match ingest::ingest_file(&mut store, f, &ctx.base, force) {
            Ok(ingest::Outcome::Added { chunks }) => {
                eprintln!("{chunks} chunks");
                added += 1;
            }
            Ok(ingest::Outcome::Unchanged) => {
                eprintln!("unchanged");
                same += 1;
            }
            Ok(ingest::Outcome::NoText) => {
                eprintln!("no text layer (scanned? run OCR first)");
                failed += 1;
            }
            Err(e) => {
                eprintln!("FAILED: {e}");
                failed += 1;
            }
        }
    }
    embed_pending(ctx, &mut store, &ctx.index);
    store.save(&ctx.index)?;
    eprintln!(
        "done: {added} added, {same} unchanged, {failed} skipped — {} documents, {} chunks in {}",
        store.docs.len(),
        store.chunks.len(),
        ctx.index.display()
    );
    Ok(())
}

fn run(ctx: &Ctx, cmd: &Cmd) -> Result<()> {
    let bib = ctx.load_bib();
    match cmd {
        Cmd::Ingest { paths, force } => {
            let paths: Vec<PathBuf> = if paths.is_empty() {
                match &ctx.data {
                    Some(d) => vec![d.join("Library")],
                    None => bail!("give at least one file or folder"),
                }
            } else {
                paths.clone()
            };
            ingest_paths(ctx, &paths, *force)?;
        }

        Cmd::Export {
            input,
            to,
            output,
            title,
            bib_name,
            bibfile,
        } => {
            let md = std::fs::read_to_string(input)?;
            let ext = match to {
                ExportTo::Tex => "tex",
                ExportTo::Docx => "docx",
                ExportTo::Html => "html",
                ExportTo::Pdf => "pdf",
                ExportTo::Odt => "odt",
                ExportTo::Epub => "epub",
            };
            let out = output.clone().unwrap_or_else(|| input.with_extension(ext));
            match to {
                ExportTo::Tex => std::fs::write(
                    &out,
                    export::to_tex(&md, title.as_deref(), bib_name.as_deref()),
                )?,
                ExportTo::Html => std::fs::write(&out, export::to_html(&md, title.as_deref()))?,
                ExportTo::Docx => std::fs::write(&out, export::to_docx(&md, title.as_deref())?)?,
                ExportTo::Pdf => export_pdf(&md, title.as_deref(), bibfile.as_deref(), &out)?,
                ExportTo::Odt | ExportTo::Epub => {
                    let mut c = std::process::Command::new("pandoc");
                    c.arg(input)
                        .arg("-o")
                        .arg(&out)
                        .arg("--from")
                        .arg("markdown");
                    if let Some(t) = title {
                        c.arg("--metadata").arg(format!("title={t}"));
                    }
                    if let Some(b) = bibfile {
                        c.arg("--citeproc").arg("--bibliography").arg(b);
                    }
                    match c.status() {
                        Ok(st) if st.success() => {}
                        Ok(st) => bail!(
                            "pandoc failed ({st}). For PDF it also needs a LaTeX engine (pdflatex)."
                        ),
                        Err(_) => bail!(
                            "this format needs pandoc (https://pandoc.org) on PATH; alternatively export --to tex/docx/html"
                        ),
                    }
                }
            }
            eprintln!("wrote {}", out.display());
        }

        Cmd::Bib { to, output } => {
            let bib = ctx.load_bib();
            if bib.entries.is_empty() {
                bail!("no .bib entries found next to the index or via --bib");
            }
            let text = match to {
                BibTo::Ris => bib
                    .entries
                    .iter()
                    .map(|e| e.to_ris())
                    .collect::<Vec<_>>()
                    .join("\n"),
                BibTo::Bibtex => bib
                    .entries
                    .iter()
                    .map(|e| e.to_bibtex())
                    .collect::<Vec<_>>()
                    .join("\n"),
                BibTo::CslJson => serde_json::to_string_pretty(
                    &bib.entries.iter().map(|e| e.to_csl()).collect::<Vec<_>>(),
                )?,
            };
            match output {
                Some(p) => {
                    std::fs::write(p, text)?;
                    eprintln!("wrote {} ({} entries)", p.display(), bib.entries.len());
                }
                None => println!("{text}"),
            }
        }

        Cmd::Zotero { data, port, force } => {
            let home = std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(PathBuf::from);
            let data = data
                .clone()
                .or_else(|| home.map(|h| h.join("Zotero")))
                .ok_or_else(|| anyhow::anyhow!("cannot find the Zotero folder — pass --data"))?;
            match zotero_bibtex(*port) {
                Ok((text, n)) if n > 0 => {
                    let dest = ctx.base.join("zotero.bib");
                    std::fs::write(&dest, text)?;
                    eprintln!(
                        "Zotero: {n} items → {} (used automatically for citations)",
                        dest.display()
                    );
                }
                Ok(_) => eprintln!("Zotero answered but returned no BibTeX items."),
                Err(e) => eprintln!(
                    "⚠ could not read BibTeX from Zotero ({e}). Is Zotero running, with “Allow other applications on this computer to communicate with Zotero” enabled? Continuing with the files."
                ),
            }
            let storage = data.join("storage");
            if !storage.is_dir() {
                bail!(
                    "{} is not a Zotero storage folder — pass --data <Zotero data directory>",
                    storage.display()
                );
            }
            ingest_paths(ctx, &[storage], *force)?;
        }

        Cmd::Search { query, k } => {
            let store = Store::load(&ctx.index)?;
            let allow = allowed_docs(ctx, &store, &bib);
            let emb = embed_query(ctx, &store, query);
            let hits = Searcher::new(&store).search(query, emb.as_deref(), *k, 4, allow.as_ref());
            if hits.is_empty() {
                println!("no matches");
            }
            for (i, h) in hits.iter().enumerate() {
                let text = &store.chunks[h.chunk].text;
                let snippet: String = text.chars().take(320).collect();
                println!(
                    "[{}] {}  (terms matched {:.0}%)\n    {}…\n",
                    i + 1,
                    prompt::label(&store, &bib, h),
                    coverage(query, text) * 100.0,
                    snippet
                );
            }
        }

        Cmd::Audit {
            file,
            json,
            fail,
            semantic,
        } => {
            let store = Store::load(&ctx.index)?;
            let text = std::fs::read_to_string(file)?;
            let mut f = audit::audit(&store, &bib, &text);
            if *semantic {
                let (c, ch) = semantic_audit(ctx, &mut f);
                eprintln!(
                    "[semantic pass: {c} citations judged by {}, {ch} verdicts changed — the model can be wrong]",
                    ctx.llm.model
                );
            }
            if *json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &f.iter().map(|x| x.to_json()).collect::<Vec<_>>()
                    )?
                );
            } else {
                print!("{}", audit::audit_markdown(&f));
            }
            if *fail && f.iter().any(|x| x.is_problem()) {
                std::process::exit(2);
            }
        }

        Cmd::Refs { file, json, online } => {
            let store = Store::load(&ctx.index)?;
            let text = std::fs::read_to_string(file)?;
            let mut v = audit::refs_report(&store, &bib, &text);
            if *online {
                let used: Vec<String> = v["cited"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let entries: Vec<&bib::Entry> = bib
                    .entries
                    .iter()
                    .filter(|e| used.iter().any(|k| k.eq_ignore_ascii_case(&e.key)))
                    .collect();
                eprintln!(
                    "[online: sending {} DOIs/titles to OpenAlex and Crossref]",
                    entries.len()
                );
                v["online"] = online::check_entries(&entries);
            }
            if *json {
                println!("{}", serde_json::to_string_pretty(&v)?)
            } else {
                print!("{}", audit::refs_markdown(&v));
                if *online {
                    print!("{}", online::check_markdown(&v["online"]));
                }
            }
        }

        Cmd::Eval { file, json } => {
            let store = Store::load(&ctx.index)?;
            let cases = eval::parse_cases(&std::fs::read_to_string(file)?)
                .map_err(|e| anyhow::anyhow!(e))?;
            let v = eval::evaluate(&store, &bib, &cases, &|q| embed_query(ctx, &store, q));
            if *json {
                println!("{}", serde_json::to_string_pretty(&v)?)
            } else {
                print!("{}", eval::markdown(&v))
            }
        }

        Cmd::Tables {
            file,
            page,
            csv,
            json,
            out,
        } => {
            let pages = pdfx::layout_pages(file).ok_or_else(|| {
                anyhow::anyhow!(
                    "pdftotext (poppler) is required: could not read {}",
                    file.display()
                )
            })?;
            let mut all: Vec<(usize, usize, pdfx::Table)> = Vec::new();
            for (pi, l) in pages.iter().enumerate() {
                if page.map(|p| p != pi + 1).unwrap_or(false) {
                    continue;
                }
                let mut k = 0;
                for it in pdfx::parse_page(l) {
                    if let pdfx::Item::Table(t) = it {
                        k += 1;
                        all.push((pi + 1, k, t));
                    }
                }
            }
            if let Some(dir) = out {
                std::fs::create_dir_all(dir)?;
                for (p, k, t) in &all {
                    std::fs::write(dir.join(format!("table-p{p}-{k}.csv")), t.to_csv())?;
                }
                eprintln!("wrote {} CSV files to {}", all.len(), dir.display());
            } else if *json {
                println!("{}", serde_json::to_string_pretty(&all.iter().map(|(p, k, t)| serde_json::json!({"page": p, "n": k, "caption": t.caption, "rows": t.rows})).collect::<Vec<_>>())?);
            } else {
                for (p, k, t) in &all {
                    if *csv {
                        println!(
                            "# page {p}, table {k}{}\n{}",
                            t.caption
                                .as_ref()
                                .map(|c| format!(": {c}"))
                                .unwrap_or_default(),
                            t.to_csv()
                        );
                    } else {
                        println!("<!-- page {p}, table {k} -->\n{}", t.to_markdown());
                    }
                }
            }
            if all.is_empty() {
                eprintln!(
                    "no tables found (scanned PDFs, tables drawn as images and two-column prose are not detected)"
                );
            }
        }

        Cmd::Equations { file, page, json } => {
            let pages = pdfx::layout_pages(file).ok_or_else(|| {
                anyhow::anyhow!(
                    "pdftotext (poppler) is required: could not read {}",
                    file.display()
                )
            })?;
            let mut n = 0;
            let mut rows = Vec::new();
            for (pi, l) in pages.iter().enumerate() {
                if page.map(|p| p != pi + 1).unwrap_or(false) {
                    continue;
                }
                for it in pdfx::parse_page(l) {
                    if let pdfx::Item::Equation(e) = it {
                        n += 1;
                        rows.push((pi + 1, e));
                    }
                }
            }
            if *json {
                println!("{}", serde_json::to_string_pretty(&rows.iter().map(|(p, e)| serde_json::json!({"page": p, "latex": e.latex, "tag": e.tag})).collect::<Vec<_>>())?);
            } else {
                for (p, e) in &rows {
                    println!(
                        "p. {p}{}: $$ {} $$",
                        e.tag
                            .as_ref()
                            .map(|t| format!(", eq. {t}"))
                            .unwrap_or_default(),
                        e.latex
                    );
                }
            }
            if n == 0 {
                eprintln!("no display equations found");
            }
        }

        Cmd::Lit {
            query,
            n,
            bibtex,
            json,
        } => {
            eprintln!("[online: the query is sent to OpenAlex and Crossref]");
            let works = online::search(query, *n)?;
            if *json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &works.iter().map(|w| w.to_json()).collect::<Vec<_>>()
                    )?
                );
            } else if *bibtex {
                for w in &works {
                    println!("{}", w.to_bibtex());
                }
            } else {
                for (i, w) in works.iter().enumerate() {
                    println!("{}. {}", i + 1, w.line());
                }
            }
        }

        Cmd::Coverage { file, json } => {
            let store = Store::load(&ctx.index)?;
            let heads = audit::outline_headings(&std::fs::read_to_string(file)?);
            anyhow::ensure!(!heads.is_empty(), "no headings found in {}", file.display());
            let v = audit::coverage_matrix(&store, &bib, &heads, &|h| embed_query(ctx, &store, h));
            if *json {
                println!("{}", serde_json::to_string_pretty(&v)?)
            } else {
                print!("{}", audit::coverage_markdown(&v))
            }
        }

        Cmd::Novelty { file, json } => {
            let store = Store::load(&ctx.index)?;
            let text = if ingest::is_supported(file) {
                ingest::read_pages(file)?
                    .into_iter()
                    .map(|(_, t)| t)
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                std::fs::read_to_string(file)?
            };
            let v = audit::novelty(&store, &bib, &text);
            if *json {
                println!("{}", serde_json::to_string_pretty(&v)?)
            } else {
                print!("{}", audit::novelty_markdown(&v))
            }
        }

        Cmd::Receipt { file, answer } => {
            let r: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(file)?)?;
            let store = Store::load(&ctx.index).ok();
            let ans = match answer {
                Some(p) => Some(std::fs::read_to_string(p)?),
                None => None,
            };
            let (ok, lines) = audit::verify_receipt(store.as_ref(), &r, ans.as_deref());
            for l in lines {
                println!("{l}");
            }
            if !ok {
                std::process::exit(2);
            }
        }

        Cmd::Declare { receipts, lang } => {
            anyhow::ensure!(!receipts.is_empty(), "give one or more receipt files");
            let mut rs = Vec::new();
            for p in receipts {
                rs.push(serde_json::from_str::<serde_json::Value>(
                    &std::fs::read_to_string(p)?,
                )?);
            }
            print!("{}", audit::declaration(&rs, lang.starts_with("ro")));
        }

        Cmd::Ask {
            question,
            k,
            decompose: dec,
            lang,
            receipt,
        } => {
            let store = Store::load(&ctx.index)?;
            let queries = if *dec {
                decompose(ctx, question)
            } else {
                vec![question.clone()]
            };
            let hits = retrieve(ctx, &store, &bib, &queries, *k, 4);
            let task = format!(
                "Answer the question below as precisely as the excerpts allow.\n\nQUESTION: {question}"
            );
            let ans = generate(ctx, &store, &bib, &hits, &task, lang.as_deref())?;
            print!("{}", report(ctx, &store, &bib, &hits, &ans));
            if let Some(p) = receipt {
                let r = audit::make_receipt(
                    &store,
                    &ctx.llm.model,
                    "ask",
                    question,
                    &hits.iter().map(|h| h.chunk).collect::<Vec<_>>(),
                    &ans,
                );
                std::fs::write(p, serde_json::to_string_pretty(&r)?)?;
                eprintln!("receipt written to {}", p.display());
            }
        }

        Cmd::Compare { question, k, lang } => {
            let store = Store::load(&ctx.index)?;
            // At most 2 passages per document, so several different sources get a voice.
            let hits = retrieve(ctx, &store, &bib, std::slice::from_ref(question), *k, 2);
            let task = format!(
                "Compare how the different sources address the question below. Group the sources by position \
                 or finding and cite the excerpts for each group. Then state plainly where the sources agree, \
                 where they disagree and why (definitions, method, period, scope), and say so if only one source \
                 addresses a point. Do not settle disagreements with outside knowledge.\n\nQUESTION: {question}"
            );
            let ans = generate(ctx, &store, &bib, &hits, &task, lang.as_deref())?;
            print!("{}", report(ctx, &store, &bib, &hits, &ans));
        }

        Cmd::Gaps { topic, k, lang } => {
            let store = Store::load(&ctx.index)?;
            let queries = vec![
                topic.clone(),
                format!("{topic} limitations and criticism"),
                format!("{topic} further research needed, open questions, future work"),
            ];
            let hits = retrieve(ctx, &store, &bib, &queries, *k, 3);
            let task = format!(
                "Using only the excerpts, map the state of the library on \"{topic}\": (a) what it establishes well, \
                 (b) points that are weakly supported or contradictory, (c) questions the excerpts raise but do not \
                 answer, (d) kinds of sources that seem to be missing. Cite excerpts for (a)–(c); for (d) write \
                 'not in excerpts'. State explicitly that this view is limited to the retrieved excerpts."
            );
            let ans = generate(ctx, &store, &bib, &hits, &task, lang.as_deref())?;
            print!("{}", report(ctx, &store, &bib, &hits, &ans));
        }

        Cmd::Summarize { document, lang } => {
            let store = Store::load(&ctx.index)?;
            let id = find_doc(&store, &bib, document)?;
            let idxs: Vec<usize> = (0..store.chunks.len())
                .filter(|&i| store.chunks[i].doc == id)
                .collect();
            if idxs.is_empty() {
                bail!("document has no indexed passages");
            }
            let loc = |ord: usize, c: usize| {
                let p = store.chunks[c].page;
                if p > 0 {
                    format!("p. {p}")
                } else {
                    format!("part {}", ord + 1)
                }
            };
            let batches: Vec<&[usize]> = idxs.chunks(8).collect();
            eprintln!(
                "summarising {} passages in {} steps, then merging — this can take a while",
                idxs.len(),
                batches.len()
            );
            // MAP: summarise batches; excerpt numbers are rewritten to page references straight away.
            let mut partials: Vec<(String, String)> = Vec::new();
            for (n, batch) in batches.iter().enumerate() {
                eprint!("\rmap {}/{} ", n + 1, batches.len());
                let hits: Vec<Hit> = batch
                    .iter()
                    .map(|&c| Hit {
                        chunk: c,
                        score: 0.0,
                    })
                    .collect();
                let labels: Vec<String> = batch
                    .iter()
                    .enumerate()
                    .map(|(j, &c)| loc(n * 8 + j, c))
                    .collect();
                let user = prompt::user_prompt(
                    &store,
                    &bib,
                    &hits,
                    "Summarise what these excerpts say in 3 to 6 sentences, citing the excerpt number after each point.",
                    lang.as_deref(),
                );
                let t = ctx.llm.chat(prompt::SYSTEM, &user, |_| {})?;
                partials.push((
                    format!("{} – {}", labels.first().unwrap(), labels.last().unwrap()),
                    prompt::rewrite_cites(&t, &labels),
                ));
            }
            eprintln!();
            // REDUCE: merge in groups of 8 until one summary is left; the last merge streams.
            let final_text = if partials.len() == 1 {
                println!("{}", partials[0].1);
                partials[0].1.clone()
            } else {
                loop {
                    let last = partials.len() <= 8;
                    let mut next: Vec<(String, String)> = Vec::new();
                    for group in partials.chunks(8) {
                        let mut u = String::from("PARTIAL SUMMARIES:\n\n");
                        for (range, t) in group {
                            u.push_str(&format!("[{range}]\n{t}\n\n"));
                        }
                        u.push_str("TASK:\nMerge these into one coherent summary of the document. Keep every (p. N) reference exactly as written.");
                        if let Some(l) = lang {
                            u.push_str(&format!("\n\nWrite the answer in: {l}."));
                        }
                        let t = ctx.llm.chat(prompt::SYSTEM_MERGE, &u, |tok| {
                            if last {
                                print!("{tok}");
                                let _ = std::io::stdout().flush();
                            }
                        })?;
                        let from = group
                            .first()
                            .unwrap()
                            .0
                            .split(" – ")
                            .next()
                            .unwrap()
                            .to_string();
                        let to = group
                            .last()
                            .unwrap()
                            .0
                            .split(" – ")
                            .last()
                            .unwrap()
                            .to_string();
                        next.push((format!("{from} – {to}"), t));
                    }
                    if last {
                        println!();
                        break next.remove(0).1;
                    }
                    partials = next;
                    eprintln!("merge round: {} summaries left", partials.len());
                }
            };
            // Mechanical check: every cited page must exist in this document.
            let valid: HashSet<usize> = idxs
                .iter()
                .map(|&c| store.chunks[c].page)
                .filter(|&p| p > 0)
                .collect();
            let bad: Vec<String> = prompt::page_refs(&final_text)
                .into_iter()
                .filter(|p| !valid.is_empty() && !valid.contains(p))
                .map(|p| p.to_string())
                .collect();
            if !bad.is_empty() {
                eprintln!(
                    "⚠ summary cites page(s) that do not exist in this document: {} — verify",
                    bad.join(", ")
                );
            }
            if !valid.is_empty() && prompt::page_refs(&final_text).is_empty() {
                eprintln!("⚠ the summary kept no page references — treat it as unverified");
            }
            let d = &store.docs[id];
            println!(
                "\nSource: {} — {}",
                bib.find(&d.path)
                    .map(|e| e.reference())
                    .unwrap_or_else(|| d.title.clone()),
                d.path
            );
            eprintln!(
                "note: page references were carried over from partial summaries; spot-check them against the book."
            );
        }

        Cmd::Draft {
            section,
            kind,
            notes,
            k,
            output,
            lang,
        } => {
            let store = Store::load(&ctx.index)?;
            let mut queries = vec![section.clone()];
            if let Some(n) = notes {
                queries.extend(
                    n.lines()
                        .map(str::trim)
                        .filter(|l| !l.is_empty())
                        .map(String::from),
                );
            }
            let hits = retrieve(ctx, &store, &bib, &queries, *k, 4);
            let mut task = draft_task(*kind, section);
            if let Some(n) = notes {
                task.push_str(&format!("\n\nThe author's notes on what to cover:\n{n}"));
            }
            let ans = generate(ctx, &store, &bib, &hits, &task, lang.as_deref())?;
            let refs = report(ctx, &store, &bib, &hits, &ans);
            print!("{refs}");
            let out_path = output.as_ref().map(|p| resolve_output(ctx, p));
            if let Some(p) = out_path {
                std::fs::write(&p, format!("# {section}\n\n{ans}\n{refs}"))?;
                eprintln!("saved to {}", p.display());
            }
        }

        Cmd::Outline { topic, k, lang } => {
            let store = Store::load(&ctx.index)?;
            let queries = vec![
                topic.clone(),
                format!("{topic} definitions and concepts"),
                format!("{topic} methods and approaches"),
                format!("{topic} criticism, limitations, debates"),
            ];
            let hits = retrieve(ctx, &store, &bib, &queries, *k, 4);
            let task = format!(
                "Propose a thesis chapter outline for the topic \"{topic}\". Give numbered sections and \
                 sub-sections; under each, one sentence on what it should argue, with the excerpt numbers \
                 that support it. Mark any section the excerpts cannot support as 'NEEDS SOURCES'."
            );
            let ans = generate(ctx, &store, &bib, &hits, &task, lang.as_deref())?;
            print!("{}", report(ctx, &store, &bib, &hits, &ans));
        }

        Cmd::Flashcards {
            topic,
            k,
            output,
            lang,
        } => {
            let store = Store::load(&ctx.index)?;
            let hits = retrieve(ctx, &store, &bib, std::slice::from_ref(topic), *k, 3);
            if hits.is_empty() {
                bail!("nothing relevant found for '{topic}' — ingest more material or rephrase");
            }
            let mut task = format!(
                "Make 8 to 12 study flashcards on \"{topic}\" from the excerpts. Write exactly one card per line, in \
                 this format and no other:\nQ: <question> | A: <answer> [n]\nQuestions should test understanding \
                 (definitions, distinctions, reasons, conditions), not trivia. Every answer must be fully supported \
                 by the excerpt(s) it cites; cite the excerpt number at the end of the answer."
            );
            if let Some(l) = lang {
                task.push_str(&format!("\n\nWrite the cards in: {l}."));
            }
            let user = prompt::user_prompt(&store, &bib, &hits, &task, None);
            eprintln!(
                "[{} excerpts, model {} — making cards, this can take a while]",
                hits.len(),
                ctx.llm.model
            );
            let raw = ctx.llm.chat(prompt::SYSTEM, &user, |_| {})?;
            let labels: Vec<String> = hits
                .iter()
                .map(|h| prompt::label(&store, &bib, h))
                .collect();
            let (mut cards, mut dropped) = (Vec::new(), 0usize);
            for (q, a) in prompt::parse_cards(&raw) {
                // A card is kept only if it cites at least one excerpt that really exists.
                let cited = prompt::cited_numbers(&a);
                if cited.iter().any(|&n| n >= 1 && n <= hits.len())
                    && !cited.iter().any(|&n| n == 0 || n > hits.len())
                {
                    let back = prompt::rewrite_cites(&a, &labels).replace(['\t', '\n', '\r'], " ");
                    cards.push((q.replace(['\t', '\n', '\r'], " "), back));
                } else {
                    dropped += 1;
                }
            }
            if cards.is_empty() {
                bail!(
                    "the model produced no usable cards ({dropped} dropped for missing/invalid citations) — try a larger model or a narrower topic"
                );
            }
            for (i, (q, a)) in cards.iter().enumerate() {
                println!("{}. Q: {q}\n   A: {a}\n", i + 1);
            }
            if dropped > 0 {
                eprintln!(
                    "⚠ {dropped} card(s) dropped: no valid citation, so they could not be checked against a source"
                );
            }
            if let Some(p) = output {
                let p = resolve_output(ctx, p);
                let mut tsv = String::from("#separator:tab\n#html:false\n");
                for (q, a) in &cards {
                    tsv.push_str(&format!("{q}\t{a}\n"));
                }
                std::fs::write(&p, tsv)?;
                eprintln!(
                    "saved {} cards to {} (Anki: File → Import; fields Front, Back)",
                    cards.len(),
                    p.display()
                );
            }
        }

        Cmd::Mesh { sub } => {
            let mut c = mesh::load_cfg(&ctx.base);
            match sub {
                MeshCmd::Status => {
                    let store = Store::load(&ctx.index)?;
                    println!("node {} ({})", c.name, c.id);
                    println!(
                        "sharing: {} ({} documents){}",
                        if c.share.is_empty() {
                            "nothing".to_string()
                        } else {
                            c.share.join(", ")
                        },
                        mesh::shared_docs(&c, &store).len(),
                        if c.relay {
                            ", relays received documents"
                        } else {
                            ""
                        }
                    );
                    for p in &c.peers {
                        println!(
                            "peer {} → {}{}",
                            p.name,
                            p.url,
                            p.proxy
                                .as_ref()
                                .map(|x| format!(" via {x}"))
                                .unwrap_or_default()
                        );
                    }
                    if c.peers.is_empty() {
                        println!(
                            "no peers yet — `tezaur mesh add NAME URL` or `tezaur mesh discover`"
                        );
                    }
                }
                MeshCmd::Name { name } => {
                    c.name = name.clone();
                    mesh::save_cfg(&ctx.base, &c)?;
                }
                MeshCmd::Share { patterns, relay } => {
                    c.share = patterns.clone();
                    c.relay = *relay;
                    mesh::save_cfg(&ctx.base, &c)?;
                    eprintln!(
                        "sharing: {}",
                        if patterns.is_empty() {
                            "nothing".into()
                        } else {
                            patterns.join(", ")
                        }
                    );
                }
                MeshCmd::Add {
                    name,
                    url,
                    token,
                    proxy,
                } => {
                    c.peers.retain(|p| &p.name != name);
                    c.peers.push(mesh::Peer {
                        name: name.clone(),
                        url: url.clone(),
                        token: token.clone(),
                        proxy: proxy.clone(),
                    });
                    mesh::save_cfg(&ctx.base, &c)?;
                    eprintln!("peer {name} saved");
                }
                MeshCmd::Rm { name } => {
                    c.peers.retain(|p| &p.name != name);
                    mesh::save_cfg(&ctx.base, &c)?;
                }
                MeshCmd::Pull { name } => {
                    let mut store = Store::load(&ctx.index)?;
                    let mut total = 0;
                    for p in c
                        .peers
                        .iter()
                        .filter(|p| name.as_ref().map(|n| n == &p.name).unwrap_or(true))
                    {
                        match mesh::pull(&mut store, p) {
                            Ok((a, h)) => {
                                eprintln!("{}: {a} new, {h} already held", p.name);
                                total += a;
                            }
                            Err(e) => eprintln!("⚠ {e:#}"),
                        }
                    }
                    if total > 0 {
                        store.save(&ctx.index)?;
                        embed_pending(ctx, &mut store, &ctx.index);
                        store.save(&ctx.index)?;
                    }
                }
                MeshCmd::Search { query, k } => {
                    let q = query.join(" ");
                    let (hits, errs) = mesh::federated_search(&c.peers, &q, *k);
                    for e in errs {
                        eprintln!("⚠ {e}");
                    }
                    for (i, h) in hits.iter().enumerate() {
                        println!(
                            "{}. [{}] {}\n   {}",
                            i + 1,
                            h["peer"].as_str().unwrap_or("?"),
                            h["label"].as_str().unwrap_or(""),
                            h["text"]
                                .as_str()
                                .unwrap_or("")
                                .chars()
                                .take(240)
                                .collect::<String>()
                                .replace('\n', " ")
                        );
                    }
                    if hits.is_empty() {
                        println!(
                            "no matches at the peers (they only answer for documents they share)"
                        );
                    }
                }
                MeshCmd::Export { file, all } => {
                    let store = Store::load(&ctx.index)?;
                    let bytes = mesh::bundle_export(&c, &store, &bib, *all)?;
                    std::fs::write(file, &bytes)?;
                    eprintln!("wrote {} ({} KB)", file.display(), bytes.len() / 1024);
                }
                MeshCmd::Import { file } => {
                    let mut store = Store::load(&ctx.index)?;
                    let (a, s, o) =
                        mesh::bundle_import(&mut store, &ctx.base, &std::fs::read(file)?)?;
                    if a > 0 {
                        store.save(&ctx.index)?;
                        embed_pending(ctx, &mut store, &ctx.index);
                        store.save(&ctx.index)?;
                    }
                    eprintln!("from {o}: {a} new documents, {s} already held");
                }
                MeshCmd::Discover { target, seconds } => {
                    let found = mesh::discover(target, std::time::Duration::from_secs(*seconds))?;
                    for f in &found {
                        println!(
                            "{}  {}  ({})",
                            f["name"].as_str().unwrap_or("?"),
                            f["url"].as_str().unwrap_or("?"),
                            f["id"].as_str().unwrap_or("")
                        );
                    }
                    if found.is_empty() {
                        println!(
                            "nobody answered (a server answers only when bound to a network address, or with TEZAUR_MESH_BEACON=1)"
                        );
                    }
                }
            }
        }

        Cmd::Mcp => mcp(ctx)?,
        Cmd::Serve {
            bind,
            port,
            token,
            workspaces,
        } => serve::serve(
            ctx,
            &serve::Opts {
                bind: bind.clone(),
                port: *port,
                token: token.clone(),
                workspaces: workspaces.clone(),
            },
        )?,

        Cmd::Sources => {
            let store = Store::load(&ctx.index)?;
            for (i, d) in store.docs.iter().enumerate() {
                let n = store.chunks.iter().filter(|c| c.doc == i).count();
                let pg = if d.pages > 0 {
                    format!("{} pages, ", d.pages)
                } else {
                    String::new()
                };
                let cite = bib
                    .find(&d.path)
                    .map(|e| format!(" — {}", e.cite()))
                    .unwrap_or_default();
                println!(
                    "{:>3}. {}{}  ({}{} chunks)\n     {}",
                    i + 1,
                    d.title,
                    cite,
                    pg,
                    n,
                    d.path
                );
            }
            println!(
                "{} documents, {} chunks, embeddings: {}, BibTeX entries: {}",
                store.docs.len(),
                store.chunks.len(),
                store.embed_model.as_deref().unwrap_or("none (BM25 only)"),
                bib.entries.len()
            );
        }

        Cmd::Doctor { generate } => {
            let mut problems = 0usize;
            let ok = |m: String| println!("✓ {m}");
            let info = |m: String| println!("· {m}");
            let warn = |m: String| println!("⚠ {m}");
            match &ctx.data {
                Some(d) => ok(format!("portable mode — data in {}", d.display())),
                None => {
                    info("not portable: index and settings are where you run the command".into())
                }
            }
            // Index and BibTeX
            let mut index_embeds: Option<String> = None;
            if ctx.index.exists() {
                match Store::load(&ctx.index) {
                    Ok(store) => {
                        ok(format!(
                            "index {}: {} documents, {} passages, embeddings: {}",
                            ctx.index.display(),
                            store.docs.len(),
                            store.chunks.len(),
                            store.embed_model.as_deref().unwrap_or("none")
                        ));
                        index_embeds = store.embed_model.clone();
                        if store.docs.is_empty() {
                            warn(
                                "the index is empty — put files in Data/Library and run /ingest"
                                    .into(),
                            );
                        }
                        if bib.entries.is_empty() {
                            info("no BibTeX entries found (optional: export from Zotero as .bib into Data/Library)".into());
                        } else {
                            let matched = store
                                .docs
                                .iter()
                                .filter(|d| bib.find(&d.path).is_some())
                                .count();
                            let msg = format!(
                                "BibTeX: {} entries, {}/{} documents matched",
                                bib.entries.len(),
                                matched,
                                store.docs.len()
                            );
                            if matched == 0 {
                                warn(format!(
                                    "{msg} — name files after the citation keys, or keep Zotero's `file` field"
                                ))
                            } else {
                                ok(msg)
                            }
                        }
                    }
                    Err(e) => {
                        problems += 1;
                        println!("✗ index unreadable: {e:#}");
                    }
                }
            } else {
                warn(format!(
                    "no index yet at {} — run /ingest",
                    ctx.index.display()
                ));
            }
            // Server and model
            match ctx.llm.models() {
                Ok(list) => {
                    ok(format!(
                        "server reachable: {} ({:?}), {} model(s)",
                        ctx.llm.host(),
                        ctx.llm.api,
                        list.len()
                    ));
                    if list.contains(&ctx.llm.model) {
                        ok(format!("model '{}' is available", ctx.llm.model));
                    } else {
                        problems += 1;
                        println!(
                            "✗ model '{}' is not served. Available: {}\n  fix: set `model = <id>` in settings/tezaur.conf",
                            ctx.llm.model,
                            if list.is_empty() {
                                "(none loaded)".to_string()
                            } else {
                                list.join(", ")
                            }
                        );
                    }
                }
                Err(e) => {
                    problems += 1;
                    println!("✗ {e:#}");
                }
            }
            // Embeddings
            match &ctx.embed_model {
                Some(em) => match ctx.llm.embed(em, &["test".to_string()]) {
                    Ok(v) => {
                        ok(format!("embedding model '{em}' works ({} dimensions)", v[0].len()));
                        if index_embeds.as_deref() != Some(em.as_str()) {
                            warn(format!("the index was built with embeddings '{}' — run /ingest to embed with '{em}'", index_embeds.as_deref().unwrap_or("none")));
                        }
                    }
                    Err(e) => warn(format!("embedding model '{em}' failed: {e:#}\n  keyword search still works")),
                },
                None => info("no embedding model configured — keyword search only (works offline, matches words not meaning)".into()),
            }
            // Optional real generation
            if *generate {
                let t0 = std::time::Instant::now();
                match ctx.llm.chat(
                    "You are a terse assistant.",
                    "Reply with the single word: ok",
                    |_| {},
                ) {
                    Ok(t) => ok(format!(
                        "generation works: {:.1}s for \"{}\"",
                        t0.elapsed().as_secs_f32(),
                        t.trim().chars().take(40).collect::<String>()
                    )),
                    Err(e) => {
                        problems += 1;
                        println!("✗ generation failed: {e:#}");
                    }
                }
            } else {
                info("add --generate to time a real answer".into());
            }
            if problems > 0 {
                bail!("{problems} problem(s) found");
            }
            println!("\nAll good.");
        }

        Cmd::Models => {
            let list = ctx.llm.models()?;
            println!("Server: {} ({:?})", ctx.llm.host(), ctx.llm.api);
            for m in &list {
                println!(
                    "  {}{}",
                    m,
                    if *m == ctx.llm.model {
                        "   <- configured"
                    } else {
                        ""
                    }
                );
            }
            if list.is_empty() {
                println!("  (no models reported — load or download one first)");
            } else if !list.contains(&ctx.llm.model) {
                println!(
                    "⚠ configured model '{}' is not in this list; set `model` in settings/tezaur.conf or use --model",
                    ctx.llm.model
                );
            }
        }
    }
    Ok(())
}

/// Splits on whitespace, honouring "double quotes". No backslash escapes, so Windows paths survive.
fn split_args(s: &str) -> Vec<String> {
    let (mut out, mut cur, mut in_q, mut has) = (Vec::new(), String::new(), false, false);
    for c in s.chars() {
        match c {
            '"' => {
                in_q = !in_q;
                has = true;
            }
            c if c.is_whitespace() && !in_q => {
                if has {
                    out.push(std::mem::take(&mut cur));
                    has = false;
                }
            }
            c => {
                cur.push(c);
                has = true;
            }
        }
    }
    if has {
        out.push(cur);
    }
    out
}

/// Interactive convenience: free-text commands need no quotes. Everything before the first flag
/// is one argument, so `/ask who bears the burden? --decompose` and
/// `/flashcards burden of proof -o cards.tsv` work. With explicit "quotes" normal parsing applies.
fn repl_args(rest: &str) -> Vec<String> {
    let rest = rest.trim();
    let (cmd, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let tail = tail.trim();
    let free_text = matches!(
        cmd,
        "search" | "ask" | "compare" | "gaps" | "summarize" | "outline" | "flashcards"
    );
    if free_text && !tail.is_empty() && !tail.contains('"') {
        let toks: Vec<&str> = tail.split_whitespace().collect();
        let split = toks
            .iter()
            .position(|t| t.starts_with('-'))
            .unwrap_or(toks.len());
        if split > 0 {
            let mut out = vec![cmd.to_string(), toks[..split].join(" ")];
            out.extend(toks[split..].iter().map(|t| t.to_string()));
            return out;
        }
    }
    split_args(rest)
}

const REPL_HELP: &str = "\
Type a question to answer it from your library, or use a command (quotes optional for plain text).

READ & FIND
  /search <text>              best passages, no model needed
  /ask question [--decompose] cited answer (same as typing the question); --decompose searches wider
  /compare <question>         where do the sources agree / disagree?
  /only <text>                restrict everything to matching documents; /only alone clears it
  /sources                    what is indexed
RESEARCH
  /gaps <topic>               what is established, what is weak, what is missing
  /summarize <document>       summary of one whole book/article (slow)
  /outline <topic>            chapter skeleton; marks sections lacking sources
WRITE
  /draft \"Section\" [--kind lit-review|introduction|critical-review] --notes \"idea 1\nidea 2\" [-o file.md]
STUDY
  /flashcards <topic> [-o cards.tsv]   cards whose answers cite a source; the .tsv imports into Anki
SETUP
  /ingest [folder-or-file ...] [--force]   no path = Data\\Library; --force re-reads unchanged files
  /models   /doctor [--generate]   /help   /quit
";

fn repl(ctx: &Ctx) -> Result<()> {
    println!(
        "TEZAUR — local thesis assistant. Index: {}",
        ctx.index.display()
    );
    match &ctx.data {
        Some(d) => println!(
            "Portable mode. Put books/articles (and a .bib export) in {} then type /ingest",
            d.join("Library").display()
        ),
        None => println!("Type /ingest <folder> to index your library."),
    }
    println!(
        "Backend: {:?} at {} — model {} (settings/tezaur.conf). /help for commands.",
        ctx.llm.api,
        ctx.llm.host(),
        ctx.llm.model
    );
    let indexed = Store::load(&ctx.index).map(|s| s.docs.len()).unwrap_or(0);
    if indexed == 0 {
        println!(
            "Next step: add files to the library, then type /ingest. Problems? /doctor tells you what to fix.\n"
        );
    } else {
        println!("{indexed} documents indexed. Ask a question, or /help.\n");
    }
    let stdin = std::io::stdin();
    loop {
        let only = ctx.only.borrow().clone();
        match only {
            Some(o) => print!("tezaur [{o}]> "),
            None => print!("tezaur> "),
        }
        std::io::stdout().flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        match line {
            "" => continue,
            "/quit" | "/exit" | "quit" | "exit" => break,
            "/help" | "help" | "?" => {
                print!("{REPL_HELP}");
                continue;
            }
            _ => {}
        }
        if let Some(rest) = line.strip_prefix("/only")
            && (rest.is_empty() || rest.starts_with(char::is_whitespace))
        {
            let v = rest.trim().trim_matches('"').to_string();
            *ctx.only.borrow_mut() = if v.is_empty() { None } else { Some(v) };
            match &*ctx.only.borrow() {
                Some(o) => println!("restricted to documents matching '{o}'"),
                None => println!("filter cleared — whole library"),
            }
            continue;
        }
        let args = match line.strip_prefix('/') {
            Some(rest) => repl_args(rest),
            None => vec!["ask".to_string(), line.to_string()],
        };
        match Line::try_parse_from(args) {
            Ok(l) => {
                if let Err(e) = run(ctx, &l.cmd) {
                    eprintln!("error: {e:#}");
                }
            }
            Err(e) => {
                let _ = e.print();
            }
        }
        println!();
    }
    Ok(())
}

fn index_base(index: &Path) -> PathBuf {
    match index.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

// ---------- MCP server (stdio, newline-delimited JSON-RPC 2.0) ----------

fn mcp_tools() -> serde_json::Value {
    serde_json::json!([
        {"name": "tezaur_search",
         "description": "Search the researcher's own library (books, articles). Returns numbered passages with citation labels (author/year/page when BibTeX is available). Use these passages as the only evidence for claims and cite their labels.",
         "inputSchema": {"type": "object", "properties": {
             "query": {"type": "string", "description": "Keywords or a short question"},
             "k": {"type": "integer", "description": "Number of passages (default 8, max 30)"},
             "only": {"type": "string", "description": "Restrict to documents whose title, path or citation contains this text"}},
           "required": ["query"]}},
        {"name": "tezaur_audit",
         "description": "Check a draft against the library. Each [@key, p. N] citation is tested for whether the cited passage covers the sentence; quotations are checked word-for-word; wrong pages, unknown keys and uncited long statements are flagged. Returns a report of problems to verify by hand.",
         "inputSchema": {"type": "object", "properties": {"text": {"type": "string", "description": "The draft, Markdown with pandoc-style [@key, p. 3] citations"},
             "semantic": {"type": "boolean", "description": "Also ask the local model for a second opinion on each citation (slow; catches paraphrases and negations; the model can be wrong)"}}, "required": ["text"]}},
        {"name": "tezaur_refs",
         "description": "Reference hygiene for a draft: cited keys that exist nowhere, bibliography entries and library documents never cited.",
         "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}}, "required": ["text"]}},
        {"name": "tezaur_coverage",
         "description": "Given an outline (Markdown headings or a list), report which sections have sources in the library (well-covered / thin / empty) and which documents.",
         "inputSchema": {"type": "object", "properties": {"outline": {"type": "string"}}, "required": ["outline"]}},
        {"name": "tezaur_sources",
         "description": "List the documents in the library with their citations and page counts.",
         "inputSchema": {"type": "object", "properties": {}}},
        {"name": "tezaur_read",
         "description": "Read the passages of one document in order (title, path or citation key fragment). Use for a summary of a whole source.",
         "inputSchema": {"type": "object", "properties": {
             "document": {"type": "string"},
             "offset": {"type": "integer", "description": "First passage number (default 0)"},
             "limit": {"type": "integer", "description": "Passages to return (default 10, max 40)"}},
           "required": ["document"]}}
    ])
}

fn mcp_call(ctx: &Ctx, name: &str, args: &serde_json::Value) -> Result<String> {
    let store = Store::load(&ctx.index)?;
    let bib = ctx.load_bib();
    let num = |k: &str, d: usize, max: usize| {
        args.get(k)
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(d)
            .min(max)
    };
    match name {
        "tezaur_search" => {
            let q = args
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            anyhow::ensure!(!q.is_empty(), "query is required");
            if let Some(o) = args.get("only").and_then(|v| v.as_str()) {
                *ctx.only.borrow_mut() = if o.trim().is_empty() {
                    None
                } else {
                    Some(o.to_string())
                };
            }
            let hits = retrieve(ctx, &store, &bib, &[q.to_string()], num("k", 8, 30), 3);
            if hits.is_empty() {
                return Ok("No matching passages in the library.".into());
            }
            let mut out = String::new();
            for (i, h) in hits.iter().enumerate() {
                out.push_str(&format!(
                    "[{}] ({})\n{}\n\n",
                    i + 1,
                    prompt::label(&store, &bib, h),
                    store.chunks[h.chunk].text
                ));
            }
            Ok(out)
        }
        "tezaur_audit" => {
            let t = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            anyhow::ensure!(!t.trim().is_empty(), "text is required");
            let mut f = audit::audit(&store, &bib, t);
            if args
                .get("semantic")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                semantic_audit(ctx, &mut f);
            }
            Ok(audit::audit_markdown(&f))
        }
        "tezaur_refs" => {
            let t = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            anyhow::ensure!(!t.trim().is_empty(), "text is required");
            Ok(audit::refs_markdown(&audit::refs_report(&store, &bib, t)))
        }
        "tezaur_coverage" => {
            let t = args.get("outline").and_then(|v| v.as_str()).unwrap_or("");
            let heads = audit::outline_headings(t);
            anyhow::ensure!(!heads.is_empty(), "outline is empty");
            Ok(audit::coverage_markdown(&audit::coverage_matrix(
                &store,
                &bib,
                &heads,
                &|h| embed_query(ctx, &store, h),
            )))
        }
        "tezaur_sources" => {
            let mut out = String::new();
            for (i, d) in store.docs.iter().enumerate() {
                let cite = bib
                    .find(&d.path)
                    .map(|e| e.cite())
                    .unwrap_or_else(|| d.title.clone());
                let n = store.chunks.iter().filter(|c| c.doc == i).count();
                out.push_str(&format!(
                    "{}. {} — {} passages, {} pages — {}\n",
                    i + 1,
                    cite,
                    n,
                    d.pages,
                    d.path
                ));
            }
            Ok(if out.is_empty() {
                "The library is empty.".into()
            } else {
                out
            })
        }
        "tezaur_read" => {
            let q = args
                .get("document")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            anyhow::ensure!(!q.is_empty(), "document is required");
            let di = store
                .docs
                .iter()
                .position(|d| {
                    d.title.to_lowercase().contains(&q)
                        || d.path.to_lowercase().contains(&q)
                        || bib
                            .find(&d.path)
                            .map(|e| {
                                e.key.to_lowercase().contains(&q)
                                    || e.cite().to_lowercase().contains(&q)
                            })
                            .unwrap_or(false)
                })
                .ok_or_else(|| anyhow::anyhow!("no document matches '{q}' (use tezaur_sources)"))?;
            let idx: Vec<usize> = (0..store.chunks.len())
                .filter(|&i| store.chunks[i].doc == di)
                .collect();
            let (off, lim) = (num("offset", 0, usize::MAX), num("limit", 10, 40));
            let mut out = format!(
                "{} — passages {}–{} of {}\n\n",
                store.docs[di].title,
                off + 1,
                (off + lim).min(idx.len()),
                idx.len()
            );
            for &i in idx.iter().skip(off).take(lim) {
                let h = Hit {
                    chunk: i,
                    score: 0.0,
                };
                out.push_str(&format!(
                    "({})\n{}\n\n",
                    prompt::label(&store, &bib, &h),
                    store.chunks[i].text
                ));
            }
            Ok(out)
        }
        _ => anyhow::bail!("unknown tool {name}"),
    }
}

/// Handles one JSON-RPC message; returns the reply line, or None for notifications.
pub(crate) fn mcp_handle(ctx: &Ctx, line: &str) -> Option<String> {
    use serde_json::{Value, json};
    let msg: Value =
        match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => return Some(
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"parse error"}})
                    .to_string(),
            ),
        };
    let id = msg.get("id").cloned()?; // no id = notification: never answer
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": params.get("protocolVersion").and_then(|v| v.as_str()).unwrap_or("2025-06-18"),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "tezaur", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "Read-only access to the researcher's own library. Ground claims in the returned passages and cite their labels; say so when the library lacks support."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": mcp_tools()})),
        "tools/call" => {
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            Ok(match mcp_call(ctx, name, &args) {
                Ok(t) => json!({"content": [{"type": "text", "text": t}]}),
                Err(e) => {
                    json!({"content": [{"type": "text", "text": format!("error: {e:#}")}], "isError": true})
                }
            })
        }
        _ => Err(json!({"code": -32601, "message": format!("method not found: {method}")})),
    };
    Some(
        match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}),
        }
        .to_string(),
    )
}

fn mcp(ctx: &Ctx) -> Result<()> {
    use std::io::{BufRead, Write};
    eprintln!("tezaur MCP server on stdio — index {}", ctx.index.display());
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = mcp_handle(ctx, &line) {
            writeln!(out, "{reply}")?;
            out.flush()?;
        }
    }
    Ok(())
}

fn main() {
    // One clean line on failure (no backtrace, whatever RUST_BACKTRACE says) and a non-zero exit code.
    if let Err(e) = real_main() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    let data = portable::data_dir();
    if let Some(d) = &data {
        portable::apply_conf(d);
    }
    let cli = Cli::parse();
    let index = cli
        .index
        .clone()
        .or_else(|| data.as_ref().map(|d| d.join("tezaur.idx")))
        .unwrap_or_else(|| PathBuf::from("tezaur.idx"));
    let host = cli
        .host
        .clone()
        .or_else(|| {
            if cli.api == Api::Ollama {
                std::env::var("OLLAMA_HOST").ok().filter(|h| !h.is_empty())
            } else {
                None
            }
        })
        .unwrap_or_else(|| cli.api.default_host().to_string());
    let ctx = Ctx {
        base: index_base(&index),
        index,
        data,
        llm: Llm::new(cli.api, &host, &cli.model, cli.num_ctx, cli.api_key.clone()),
        embed_model: cli.embed_model.clone(),
        bib_files: cli.bib.clone(),
        only: RefCell::new(cli.only.clone()),
        rerank: cli.rerank,
        verify: cli.verify,
    };
    match &cli.cmd {
        Some(cmd) => run(&ctx, cmd),
        None => repl(&ctx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_quoted_args_and_keeps_windows_paths() {
        assert_eq!(
            split_args(r#"draft "Recognition principles" --notes "a b" -o x.md"#),
            vec![
                "draft",
                "Recognition principles",
                "--notes",
                "a b",
                "-o",
                "x.md"
            ]
        );
        assert_eq!(
            split_args(r"ingest D:\Books\Thesis"),
            vec!["ingest", r"D:\Books\Thesis"]
        );
        assert_eq!(split_args(r#"search """#), vec!["search", ""]);
    }

    #[test]
    fn free_text_commands_need_no_quotes() {
        assert_eq!(
            repl_args("search burden of proof"),
            vec!["search", "burden of proof"]
        );
        assert_eq!(
            repl_args("compare do sources agree?"),
            vec!["compare", "do sources agree?"]
        );
        assert_eq!(
            repl_args("summarize lisbon convention"),
            vec!["summarize", "lisbon convention"]
        );
        assert_eq!(
            repl_args("outline Recunoașterea calificărilor"),
            vec!["outline", "Recunoașterea calificărilor"]
        );
        // flags or explicit quotes keep normal parsing
        assert_eq!(
            repl_args("search burden of proof -k 3"),
            vec!["search", "burden of proof", "-k", "3"]
        );
        assert_eq!(
            repl_args("flashcards burden of proof -o carduri.tsv"),
            vec!["flashcards", "burden of proof", "-o", "carduri.tsv"]
        );
        assert_eq!(
            repl_args("ask who bears it? --decompose"),
            vec!["ask", "who bears it?", "--decompose"]
        );
        assert_eq!(repl_args("ask -k 3"), vec!["ask", "-k", "3"]); // no text: normal parsing, clap will complain
        assert_eq!(
            repl_args(r#"ask "a b" -k 4"#),
            vec!["ask", "a b", "-k", "4"]
        );
        assert_eq!(repl_args("sources"), vec!["sources"]);
        assert_eq!(repl_args(r"ingest D:\Books"), vec!["ingest", r"D:\Books"]);
    }

    #[test]
    fn repl_lines_parse_into_commands() {
        let l = Line::try_parse_from(split_args(
            r#"ask "who bears the burden?" -k 4 --decompose"#,
        ))
        .unwrap();
        assert!(matches!(
            l.cmd,
            Cmd::Ask {
                k: 4,
                decompose: true,
                ..
            }
        ));
        let l = Line::try_parse_from(split_args("ingest")).unwrap();
        assert!(matches!(l.cmd, Cmd::Ingest { ref paths, force: false } if paths.is_empty()));
        assert!(matches!(
            Line::try_parse_from(split_args("models")).unwrap().cmd,
            Cmd::Models
        ));
        let l = Line::try_parse_from(split_args(r#"draft "Tema" --kind lit-review -k 6"#)).unwrap();
        assert!(matches!(
            l.cmd,
            Cmd::Draft {
                kind: Kind::LitReview,
                k: 6,
                ..
            }
        ));
        let l = Line::try_parse_from(repl_args("flashcards recognition of diplomas")).unwrap();
        assert!(matches!(l.cmd, Cmd::Flashcards { .. }));
        assert!(matches!(
            Line::try_parse_from(split_args("doctor --generate"))
                .unwrap()
                .cmd,
            Cmd::Doctor { generate: true }
        ));
        assert!(Line::try_parse_from(split_args("bogus")).is_err());
    }
}
