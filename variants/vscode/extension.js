const vscode = require('vscode');
const core = require('./tezaur-core.js');

const http = async (url, o) => { const r = await fetch(url, { method: o.method, headers: o.headers, body: o.body }); return { status: r.status, text: await r.text() }; };
let index = new core.Index(), status;

const cfg = () => { const c = vscode.workspace.getConfiguration('tezaur'); return { api: c.get('api'), host: c.get('host'), model: c.get('model'), key: c.get('apiKey'), k: c.get('k') || 8, lang: c.get('language'), include: c.get('include'), exclude: c.get('exclude') }; };
const rel = p => vscode.workspace.asRelativePath(p, false);

async function readFile(uri) {
  const text = Buffer.from(await vscode.workspace.fs.readFile(uri)).toString('utf8');
  index.addDocument({ id: uri.toString(), path: uri.fsPath, title: rel(uri.fsPath), text });
}
async function reindex(announce) {
  const c = cfg();
  index = new core.Index();
  const files = await vscode.workspace.findFiles(c.include, c.exclude, 5000);
  for (const f of files) { try { await readFile(f); } catch (e) { /* unreadable file: skip */ } }
  status.text = `$(book) TEZAUR ${files.length}`;
  status.tooltip = `${files.length} files, ${index.chunks.length} passages`;
  if (announce) vscode.window.showInformationMessage(`TEZAUR: ${files.length} files, ${index.chunks.length} passages.`);
}
const where = (d, c) => `${d.title}:${c.line + 1}`;

async function ask(mode) {
  const q = await vscode.window.showInputBox({ prompt: mode === 'compare' ? 'What should the sources be compared on?' : 'Question for your workspace', ignoreFocusOut: true });
  if (!q) return;
  const c = cfg();
  if (!c.model) return vscode.window.showWarningMessage('Set “tezaur.model” in settings first (e.g. llama3.1:8b).');
  if (!index.chunks.length) return vscode.window.showWarningMessage('TEZAUR: no files indexed (check tezaur.include).');
  await vscode.window.withProgress({ location: vscode.ProgressLocation.Notification, title: 'TEZAUR is reading your sources…' }, async () => {
    try {
      const r = await core.answer(index, c, http, mode, q, { k: c.k, labelFn: where, lang: c.lang });
      const used = [...core.citedNumbers(r.answer)].filter(n => n >= 1 && n <= r.hits.length).sort((a, b) => a - b);
      const refs = used.map(n => { const ch = index.chunks[r.hits[n - 1].chunk], d = index.docs[ch.doc]; return `- [${n}] [${where(d, ch)}](${encodeURI(rel(d.path))}#L${ch.line + 1})${ch.heading ? ' — ' + ch.heading : ''}`; });
      const md = `# ${q}\n\n${r.answer}\n\n${r.warnings.length ? r.warnings.map(w => `> ⚠ ${w}`).join('\n') + '\n\n' : ''}## Sources\n\n${refs.join('\n') || '_none cited_'}\n`;
      const doc = await vscode.workspace.openTextDocument({ language: 'markdown', content: md });
      await vscode.window.showTextDocument(doc, { preview: false, viewColumn: vscode.ViewColumn.Beside });
    } catch (e) { vscode.window.showErrorMessage('TEZAUR: ' + (e.message || e)); }
  });
}

async function search() {
  const q = await vscode.window.showInputBox({ prompt: 'Search your workspace passages', ignoreFocusOut: true });
  if (!q) return;
  const hits = index.search(q, 20);
  if (!hits.length) return vscode.window.showInformationMessage('TEZAUR: no matching passages.');
  const items = hits.map(h => { const ch = index.chunks[h.chunk], d = index.docs[ch.doc]; return { label: ch.text.replace(/\s+/g, ' ').slice(0, 90), description: where(d, ch), detail: `${Math.round(h.cov * 100)}% of terms${ch.heading ? ' · ' + ch.heading : ''}`, file: d.path, line: ch.line }; });
  const pick = await vscode.window.showQuickPick(items, { matchOnDescription: true, placeHolder: 'Open a passage' });
  if (!pick) return;
  const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(pick.file));
  const ed = await vscode.window.showTextDocument(doc);
  const pos = new vscode.Position(pick.line, 0);
  ed.selection = new vscode.Selection(pos, pos);
  ed.revealRange(new vscode.Range(pos, pos), vscode.TextEditorRevealType.InCenter);
}

function activate(context) {
  status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 0);
  status.text = '$(book) TEZAUR'; status.command = 'tezaur.search'; status.show();
  context.subscriptions.push(
    status,
    vscode.commands.registerCommand('tezaur.ask', () => ask('ask')),
    vscode.commands.registerCommand('tezaur.compare', () => ask('compare')),
    vscode.commands.registerCommand('tezaur.search', search),
    vscode.commands.registerCommand('tezaur.reindex', () => reindex(true))
  );
  const w = vscode.workspace.createFileSystemWatcher(cfg().include);
  const change = u => readFile(u).catch(() => {});
  w.onDidChange(change); w.onDidCreate(change); w.onDidDelete(u => index.removeDocument(u.toString()));
  context.subscriptions.push(w);
  reindex(false);
}
function deactivate() {}
module.exports = { activate, deactivate };
