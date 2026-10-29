'use strict';
// Shared engine for the editor integrations (Obsidian, VS Code): BM25 retrieval over text files, the same
// grounded prompts and mechanical citation check as the Rust/HTML versions. No dependencies; all I/O is injected.

const STEM = 7;
const fold = s => s.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/ß/g, 'ss').replace(/ø/g, 'o').replace(/đ/g, 'd').replace(/ł/g, 'l');
const tokens = s => (fold(s).match(/[a-z0-9]+/g) || []).filter(t => t.length >= 2).map(t => t.slice(0, STEM));

/** ~size-word windows with overlap, cut at sentence ends when possible. Returns [{text, offset}] (char offsets). */
function chunkText(text, size = 200, overlap = 40) {
  const w = [...text.matchAll(/\S+/g)];
  if (!w.length) return [];
  const out = [];
  let s = 0;
  while (s < w.length) {
    let e = Math.min(s + size, w.length);
    if (e < w.length) {
      for (let j = e; j > Math.max(s + size - 40, s + 20); j--) {
        if (/[.!?…]["')\]»”]?$/.test(w[j - 1][0])) { e = j; break; }
      }
    }
    const a = w[s].index, b = w[e - 1].index + w[e - 1][0].length;
    out.push({ text: text.slice(a, b), offset: a });
    if (e >= w.length) break;
    s = Math.max(e - overlap, s + 1);
  }
  return out;
}

class Index {
  constructor() { this.docs = []; this.chunks = []; this.inv = null; this.avg = 0; }

  /** doc: {id, path, title, text}. Markdown/LaTeX headings are tracked so passages can name their section. */
  addDocument(doc) {
    this.removeDocument(doc.id);
    const di = this.docs.length;
    const text = doc.text;
    const lineStarts = [0];
    for (let i = 0; i < text.length; i++) if (text[i] === '\n') lineStarts.push(i + 1);
    const heads = [];
    lineStarts.forEach((st, ln) => {
      const end = ln + 1 < lineStarts.length ? lineStarts[ln + 1] : text.length;
      const m = /^(#{1,6})\s+(.+?)\s*$/.exec(text.slice(st, end)) || /^\\(?:chapter|section|subsection)\*?\{(.+?)\}/.exec(text.slice(st, end));
      if (m) heads.push([ln, (m[2] || m[1]).replace(/[#*_`]/g, '').trim()]);
    });
    const lineOf = off => { let lo = 0, hi = lineStarts.length - 1; while (lo < hi) { const mid = (lo + hi + 1) >> 1; if (lineStarts[mid] <= off) lo = mid; else hi = mid - 1; } return lo; };
    this.docs.push({ id: doc.id, path: doc.path, title: doc.title });
    for (const c of chunkText(text)) {
      const line = lineOf(c.offset);
      let heading = '';
      for (const [hl, h] of heads) { if (hl <= line) heading = h; else break; }
      this.chunks.push({ doc: di, text: c.text, line, heading });
    }
    this.inv = null;
  }

  removeDocument(id) {
    const di = this.docs.findIndex(d => d.id === id);
    if (di < 0) return;
    this.docs.splice(di, 1);
    this.chunks = this.chunks.filter(c => c.doc !== di);
    this.chunks.forEach(c => { if (c.doc > di) c.doc--; });
    this.inv = null;
  }

  _build() {
    const inv = new Map(); let total = 0;
    this.chunks.forEach((c, i) => {
      const t = tokens(c.text); c.len = t.length; total += t.length;
      const tf = new Map(); t.forEach(x => tf.set(x, (tf.get(x) || 0) + 1));
      for (const [x, n] of tf) { let a = inv.get(x); if (!a) inv.set(x, a = []); a.push([i, n]); }
    });
    this.inv = inv; this.avg = total / Math.max(1, this.chunks.length);
  }

  /** allow: optional predicate on the document ({id,path,title}). */
  search(query, k = 8, perDoc = 3, allow = null) {
    if (!this.inv) this._build();
    const qt = [...new Set(tokens(query))];
    const N = this.chunks.length, sc = new Map();
    const ok = allow ? this.docs.map(allow) : null;
    for (const t of qt) {
      const p = this.inv.get(t); if (!p) continue;
      const idf = Math.log(1 + (N - p.length + .5) / (p.length + .5));
      for (const [i, tf] of p) {
        const c = this.chunks[i]; if (ok && !ok[c.doc]) continue;
        sc.set(i, (sc.get(i) || 0) + idf * tf * 2.2 / (tf + 1.2 * (.25 + .75 * c.len / this.avg)));
      }
    }
    const cnt = {}, out = [];
    for (const [i, s] of [...sc].sort((a, b) => b[1] - a[1])) {
      const d = this.chunks[i].doc;
      if ((cnt[d] || 0) >= perDoc) continue;
      cnt[d] = (cnt[d] || 0) + 1;
      out.push({ chunk: i, score: s, cov: coverage(query, this.chunks[i].text) });
      if (out.length >= k) break;
    }
    return out;
  }
}

function coverage(query, text) {
  const q = [...new Set(tokens(query))]; if (!q.length) return 1;
  const t = new Set(tokens(text)); return q.filter(x => t.has(x)).length / q.length;
}

const SYSTEM = `You are a careful academic writing assistant helping a researcher write a thesis. You are given numbered SOURCE EXCERPTS from the researcher's own notes and documents. Rules:
1. Base every factual claim ONLY on the excerpts. Never use outside knowledge for facts, figures, dates or quotations.
2. After each claim, cite the excerpt number(s) in square brackets, like [3] or [2, 5]. Never invent a number.
3. If the excerpts do not contain enough to answer, say exactly what is missing instead of guessing.
4. If excerpts disagree, present both positions and cite each.
5. Keep quotations short and exact; otherwise paraphrase in scholarly prose.
6. Write in the same language as the TASK unless told otherwise.`;

const TASKS = {
  ask: q => `Answer the question below as precisely as the excerpts allow.\n\nQUESTION: ${q}`,
  compare: q => `Compare how the different sources address the question below: group them by position, state where they agree and disagree, and say so if only one source addresses a point. Do not settle disagreements with outside knowledge.\n\nQUESTION: ${q}`,
  gaps: q => `For the topic below, list what the excerpts establish well, what is only weakly supported, what they leave unanswered, and what kinds of sources seem to be missing.\n\nTOPIC: ${q}`,
  outline: q => `Propose a chapter outline (numbered sections, one line each) for the topic below, based on and citing the excerpts. Mark with [NO SOURCES] any section they do not support.\n\nTOPIC: ${q}`,
};

function userPrompt(index, hits, task, labelFn, lang) {
  let s = 'SOURCE EXCERPTS:\n\n';
  hits.forEach((h, i) => { s += `[${i + 1}] (${labelFn(index.docs[index.chunks[h.chunk].doc], index.chunks[h.chunk])})\n${index.chunks[h.chunk].text}\n\n`; });
  s += 'TASK:\n' + task;
  if (lang) s += `\n\nWrite the answer in: ${lang}.`;
  return s;
}

const GROUP = /\[(\d+(?:\s*[,;]\s*\d+)*)\]/g;
function citedNumbers(t) { const s = new Set(); for (const m of t.matchAll(GROUP)) m[1].split(/[,;]/).forEach(x => s.add(+x.trim())); return s; }
function checkAnswer(nHits, ans) {
  const w = [], cited = citedNumbers(ans);
  const bad = [...cited].filter(n => n < 1 || n > nHits);
  if (bad.length) w.push(`the model cited non-existent excerpt(s): ${bad.join(', ')} — treat nearby claims as unsupported`);
  if (!cited.size) w.push('no citations at all — do not rely on this text');
  const un = ans.split(/\n\n/).filter(p => p.length > 200 && !citedNumbers(p).size).length;
  if (un) w.push(`${un} long paragraph(s) carry no citation — verify or cut`);
  return w;
}

const DEFAULT_HOST = { ollama: 'http://localhost:11434', jan: 'http://127.0.0.1:1337/v1', openai: 'http://127.0.0.1:8080/v1' };

/** http(url, {method, headers, body}) -> Promise<{status, text}>; injected so any host (Obsidian, Node) can supply it. */
async function chat(cfg, http, system, user) {
  const host = (cfg.host || DEFAULT_HOST[cfg.api]).replace(/\/+$/, '');
  const headers = { 'Content-Type': 'application/json' };
  if (cfg.key) headers.Authorization = 'Bearer ' + cfg.key;
  const messages = [{ role: 'system', content: system }, { role: 'user', content: user }];
  const ollama = cfg.api === 'ollama';
  const body = ollama ? { model: cfg.model, messages, stream: false, options: { num_ctx: cfg.numctx || 8192 } } : { model: cfg.model, messages, stream: false };
  let r;
  try { r = await http(host + (ollama ? '/api/chat' : '/chat/completions'), { method: 'POST', headers, body: JSON.stringify(body) }); }
  catch (e) { throw new Error(`Cannot reach ${host} — is the model server running? (${e.message || e})`); }
  if (r.status < 200 || r.status >= 300) throw new Error(`Server answered ${r.status}: ${String(r.text).slice(0, 300)}`);
  const j = JSON.parse(r.text);
  const out = ollama ? j.message && j.message.content : j.choices && j.choices[0] && j.choices[0].message && j.choices[0].message.content;
  if (typeof out !== 'string') throw new Error('Unexpected reply from the model server');
  return out;
}

async function listModels(cfg, http) {
  const host = (cfg.host || DEFAULT_HOST[cfg.api]).replace(/\/+$/, '');
  const headers = cfg.key ? { Authorization: 'Bearer ' + cfg.key } : {};
  const r = await http(host + (cfg.api === 'ollama' ? '/api/tags' : '/models'), { method: 'GET', headers });
  if (r.status < 200 || r.status >= 300) throw new Error(`Server answered ${r.status}`);
  const j = JSON.parse(r.text);
  return cfg.api === 'ollama' ? (j.models || []).map(m => m.name) : (j.data || []).map(m => m.id);
}

/** Full pipeline: retrieve -> prompt -> model -> mechanical check. Returns {answer, hits, warnings}. */
async function answer(index, cfg, http, mode, question, { k = 8, allow = null, labelFn, lang } = {}) {
  const hits = index.search(question, k, 3, allow);
  if (!hits.length) return { answer: 'No matching passages in the selected notes/files.', hits: [], warnings: [] };
  const warnings = [];
  if (Math.max(...hits.map(h => h.cov)) < .4) warnings.push('weak match: the best passage covers under 40% of the query terms');
  const text = await chat(cfg, http, SYSTEM, userPrompt(index, hits, TASKS[mode](question), labelFn, lang));
  return { answer: text, hits, warnings: warnings.concat(checkAnswer(hits.length, text)) };
}

module.exports = { tokens, chunkText, Index, coverage, SYSTEM, TASKS, userPrompt, citedNumbers, checkAnswer, chat, listModels, answer, DEFAULT_HOST };
