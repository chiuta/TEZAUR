// TEZAUR for Joplin. Select text in a note, run "TEZAUR: ask about selection" (Tools menu); the answer is inserted below.
// Plain JavaScript (no build step needed to read it); package as a .jpl with build-joplin.sh.
const format = (mode, d) => {
  if (d.error) return 'Error: ' + d.error;
  if (mode === 'search') return (d.hits || []).map(h => `[${h.n}] ${h.label}\n    ${h.text.slice(0, 300).replace(/\s+/g, ' ')}`).join('\n') || 'No matching passages.';
  let out = d.answer;
  if (d.references) out += '\n\n## References\n' + d.references;
  for (const w of d.warnings || []) out += '\n⚠ ' + w;
  return out;
};
const call = async (S, mode, q) => {
  const headers = { 'Content-Type': 'application/json' };
  if (S.token) headers.Authorization = 'Bearer ' + S.token;
  try {
    const r = await fetch(S.server.replace(/\/$/, '') + (mode === 'search' ? '/api/search' : '/api/ask'), { method: 'POST', headers, body: JSON.stringify({ q, mode, k: S.k }) });
    return await r.json();
  } catch (e) { return { error: 'cannot reach ' + S.server }; }
};

if (typeof module !== 'undefined') module.exports = { format, call };

if (typeof joplin !== 'undefined') {
  joplin.plugins.register({
    onStart: async function () {
      await joplin.settings.registerSection('tezaur', { label: 'TEZAUR', iconName: 'fas fa-book' });
      await joplin.settings.registerSettings({
        server: { value: 'http://127.0.0.1:8787', type: 3, section: 'tezaur', public: true, label: 'Server address' },
        token: { value: '', type: 3, section: 'tezaur', public: true, secure: true, label: 'Access token (optional)' },
        k: { value: 6, type: 1, section: 'tezaur', public: true, label: 'Passages per question' },
      });
      const settings = async () => ({ server: await joplin.settings.value('server'), token: await joplin.settings.value('token'), k: await joplin.settings.value('k') });
      for (const [id, label, mode] of [['tezaur.ask', 'TEZAUR: ask about selection', 'ask'], ['tezaur.compare', 'TEZAUR: compare sources on selection', 'compare'], ['tezaur.search', 'TEZAUR: search passages for selection', 'search']]) {
        await joplin.commands.register({
          name: id, label,
          execute: async () => {
            const sel = (await joplin.commands.execute('selectedText')) || '';
            if (!sel.trim()) return;
            const text = format(mode, await call(await settings(), mode, sel.trim()));
            await joplin.commands.execute('insertText', '\n\n' + text + '\n');
          },
        });
        await joplin.views.menuItems.create('menu_' + id, id, 'tools');
      }
    },
  });
}
