// TEZAUR for Logseq: type "/tezaur" in a block holding the question; the answer is added as child blocks.
const SETTINGS = [
  { key: 'server', type: 'string', default: 'http://127.0.0.1:8787', title: 'Server', description: 'Address of `tezaur serve`' },
  { key: 'token', type: 'string', default: '', title: 'Token', description: 'Access token (optional)' },
  { key: 'k', type: 'number', default: 6, title: 'Passages', description: 'Passages per question' },
];
const toBlocks = (d) => {
  if (d.error) return ['Error: ' + d.error];
  const out = (d.answer || '').split(/\n{2,}/).map(s => s.trim()).filter(Boolean);
  if (d.references) out.push('References\n' + d.references);
  for (const w of d.warnings || []) out.push('⚠ ' + w);
  return out;
};
const hitsToBlocks = (d) => d.error ? ['Error: ' + d.error] : (d.hits || []).map(h => `[${h.n}] ${h.label}: ${h.text.slice(0, 300).replace(/\s+/g, ' ')}`);
async function call(S, mode, q) {
  const headers = { 'Content-Type': 'application/json' };
  if (S.token) headers.Authorization = 'Bearer ' + S.token;
  try { return await (await fetch(S.server.replace(/\/$/, '') + (mode === 'search' ? '/api/search' : '/api/ask'), { method: 'POST', headers, body: JSON.stringify({ q, mode, k: S.k }) })).json(); }
  catch (e) { return { error: 'cannot reach ' + S.server }; }
}
if (typeof module !== 'undefined') module.exports = { toBlocks, hitsToBlocks, call };
if (typeof logseq !== 'undefined') {
  const main = () => {
    logseq.useSettingsSchema(SETTINGS);
    for (const [name, mode] of [['tezaur', 'ask'], ['tezaur compare', 'compare'], ['tezaur search', 'search']]) {
      logseq.Editor.registerSlashCommand(name, async () => {
        const b = await logseq.Editor.getCurrentBlock();
        const q = (b.content || '').replace(/^\s*\/\w+\s*/, '').trim();
        if (!q) return logseq.UI.showMsg('Write the question in the block first', 'warning');
        logseq.UI.showMsg('TEZAUR: thinking…');
        const d = await call(logseq.settings, mode, q);
        for (const t of (mode === 'search' ? hitsToBlocks(d) : toBlocks(d))) await logseq.Editor.insertBlock(b.uuid, t, { sibling: false });
      });
    }
  };
  logseq.ready(main).catch(console.error);
}
