// TEZAUR for Obsidian — ask your own vault; answers cite your notes as [[links]]. Everything stays local.
const { Plugin, ItemView, PluginSettingTab, Setting, Notice, requestUrl, MarkdownRenderer, MarkdownView, Component } = require('obsidian');

const VIEW = 'tezaur-view';
const DEFAULTS = { api: 'ollama', host: '', model: '', key: '', numctx: 8192, k: 8, folder: '', lang: '' };
const http = async (url, o) => { const r = await requestUrl({ url, method: o.method, headers: o.headers, body: o.body, throw: false }); return { status: r.status, text: r.text }; };

const linkTarget = (d, c) => d.path.replace(/\.md$/, '') + (c.heading ? '#' + c.heading.replace(/[#|^\[\]]/g, ' ').replace(/\s+/g, ' ').trim() : '');
const plainLabel = (d, c) => d.title + (c.heading ? ' › ' + c.heading : '');

/** [1, 2] -> ([[note#heading|1]], [[other|2]]) so every citation is a clickable link to the exact section. */
function linkCitations(text, index, hits) {
  return text.replace(/\[(\d+(?:\s*[,;]\s*\d+)*)\]/g, (m, g) => '(' + g.split(/[,;]/).map(x => {
    const n = +x.trim(); const h = hits[n - 1];
    if (!h) return `~~${n}~~`;
    const c = index.chunks[h.chunk];
    return `[[${linkTarget(index.docs[c.doc], c)}|${n}]]`;
  }).join(', ') + ')');
}

class TezaurPlugin extends Plugin {
  async onload() {
    this.settings = Object.assign({}, DEFAULTS, await this.loadData());
    this.index = new core.Index();
    this.registerView(VIEW, leaf => new TezaurView(leaf, this));
    this.addRibbonIcon('library', 'TEZAUR', () => this.activate());
    this.addCommand({ id: 'open', name: 'Open TEZAUR panel', callback: () => this.activate() });
    this.addCommand({ id: 'reindex', name: 'Re-read all notes', callback: () => this.reindex(true) });
    this.addSettingTab(new TezaurSettings(this.app, this));
    this.lastEditor = null;
    this.registerEvent(this.app.workspace.on('active-leaf-change', leaf => { if (leaf && leaf.view instanceof MarkdownView) this.lastEditor = leaf.view; }));
    const upd = f => this.inScope(f) && this.readFile(f);
    this.registerEvent(this.app.vault.on('modify', upd));
    this.registerEvent(this.app.vault.on('create', upd));
    this.registerEvent(this.app.vault.on('delete', f => this.index.removeDocument(f.path)));
    this.registerEvent(this.app.vault.on('rename', (f, old) => { this.index.removeDocument(old); upd(f); }));
    this.app.workspace.onLayoutReady(() => this.reindex(false));
  }
  inScope(f) {
    if (!f || f.extension !== 'md') return false;
    const fo = (this.settings.folder || '').trim().replace(/^\/+|\/+$/g, '');
    return !fo || f.path.startsWith(fo + '/');
  }
  async readFile(f) { this.index.addDocument({ id: f.path, path: f.path, title: f.basename, text: await this.app.vault.cachedRead(f) }); }
  async reindex(announce) {
    this.index = new core.Index();
    const files = this.app.vault.getMarkdownFiles().filter(f => this.inScope(f));
    for (const f of files) await this.readFile(f);
    if (announce) new Notice(`TEZAUR: ${files.length} notes, ${this.index.chunks.length} passages`);
  }
  async activate() {
    let leaf = this.app.workspace.getLeavesOfType(VIEW)[0];
    if (!leaf) { leaf = this.app.workspace.getRightLeaf(false); await leaf.setViewState({ type: VIEW, active: true }); }
    this.app.workspace.revealLeaf(leaf);
  }
  saveSettings() { return this.saveData(this.settings); }
}

class TezaurView extends ItemView {
  constructor(leaf, plugin) { super(leaf); this.plugin = plugin; this.last = null; }
  getViewType() { return VIEW; }
  getDisplayText() { return 'TEZAUR'; }
  getIcon() { return 'library'; }
  async onOpen() {
    const el = this.contentEl; el.empty(); el.addClass('tezaur');
    this.mode = el.createEl('select');
    [['ask', 'Ask'], ['compare', 'Compare sources'], ['gaps', 'Gaps'], ['outline', 'Outline'], ['search', 'Search only (no model)']]
      .forEach(([v, t]) => this.mode.createEl('option', { value: v, text: t }));
    this.input = el.createEl('textarea', { attr: { rows: '3', placeholder: 'Question or topic…' } });
    const row = el.createDiv();
    this.runBtn = row.createEl('button', { text: 'Run' });
    this.insBtn = row.createEl('button', { text: 'Insert into note' });
    this.status = el.createDiv({ cls: 'tezaur-status' });
    this.out = el.createDiv({ cls: 'tezaur-out' });
    this.runBtn.onclick = () => this.run();
    this.insBtn.onclick = () => this.insert();
  }
  async run() {
    const p = this.plugin, q = this.input.value.trim(), mode = this.mode.value;
    if (!q) return;
    if (!p.index.chunks.length) { this.status.setText('No notes indexed yet (check the folder in settings).'); return; }
    this.runBtn.disabled = true; this.status.setText(mode === 'search' ? '' : 'Thinking…'); this.out.empty();
    try {
      let md, hits;
      if (mode === 'search') {
        hits = p.index.search(q, +p.settings.k || 8);
        md = hits.length ? hits.map((h, i) => `**${i + 1}.** [[${linkTarget(p.index.docs[p.index.chunks[h.chunk].doc], p.index.chunks[h.chunk])}]] — ${Math.round(h.cov * 100)}% of terms\n> ${p.index.chunks[h.chunk].text.slice(0, 300).replace(/\n+/g, ' ')}…`).join('\n\n') : 'No matching passages.';
        this.last = { md, plain: md };
      } else {
        const r = await core.answer(p.index, p.settings, http, mode, q, { k: +p.settings.k || 8, labelFn: plainLabel, lang: p.settings.lang });
        hits = r.hits;
        const refs = hits.length ? '\n\n---\n' + [...core.citedNumbers(r.answer)].filter(n => n >= 1 && n <= hits.length).sort((a, b) => a - b).map(n => `[${n}] [[${linkTarget(p.index.docs[p.index.chunks[hits[n - 1].chunk].doc], p.index.chunks[hits[n - 1].chunk])}]]`).join('\n') : '';
        md = linkCitations(r.answer, p.index, hits) + refs;
        this.status.setText(r.warnings.map(w => '⚠ ' + w).join('\n'));
        this.last = { md };
      }
      await MarkdownRenderer.render(this.app, md, this.out, '', this);
    } catch (e) { this.status.setText('Error: ' + (e.message || e)); }
    finally { this.runBtn.disabled = false; }
  }
  insert() {
    const ed = this.plugin.lastEditor && this.plugin.lastEditor.editor;
    if (!this.last) return new Notice('Nothing to insert yet.');
    if (!ed) return new Notice('Open a note first, then click Insert.');
    ed.replaceSelection('\n' + this.last.md + '\n');
  }
}

class TezaurSettings extends PluginSettingTab {
  constructor(app, plugin) { super(app, plugin); this.plugin = plugin; }
  display() {
    const el = this.containerEl, s = this.plugin.settings; el.empty();
    const text = (name, desc, key, ph, num) => new Setting(el).setName(name).setDesc(desc).addText(t => t.setPlaceholder(ph || '').setValue(String(s[key] ?? '')).onChange(async v => { s[key] = num ? (+v || DEFAULTS[key]) : v; await this.plugin.saveSettings(); }));
    new Setting(el).setName('Model server').addDropdown(d => d.addOptions({ ollama: 'Ollama', jan: 'Jan', openai: 'OpenAI-compatible (LM Studio, llama.cpp…)' }).setValue(s.api).onChange(async v => { s.api = v; await this.plugin.saveSettings(); }));
    text('Address', 'Empty = default for the chosen server', 'host', 'http://localhost:11434');
    text('Model', 'Exact model name', 'model', 'llama3.1:8b');
    text('API key', 'Optional', 'key');
    text('Passages per question', '', 'k', '8', true);
    text('Answer language', 'Empty = same as the question', 'lang');
    new Setting(el).setName('Only this folder').setDesc('Empty = whole vault. Changes apply after “Re-read all notes”.').addText(t => t.setValue(s.folder).onChange(async v => { s.folder = v; await this.plugin.saveSettings(); }));
    new Setting(el).setName('Check connection').addButton(b => b.setButtonText('List models').onClick(async () => {
      try { const m = await core.listModels(s, http); new Notice(m.length ? m.join(', ') : 'Server reachable, no models.'); } catch (e) { new Notice('TEZAUR: ' + (e.message || e)); }
    }));
  }
}

module.exports = TezaurPlugin;
