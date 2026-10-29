/**
 * TEZAUR for Google Docs (Apps Script). Extensions > Apps Script > paste Code.gs, Sidebar.html and appsscript.json, reload the document.
 * Menu: Add-ons > TEZAUR > Settings (https address of your server + token) > Ask about selection.
 * LIMITS: Google's servers make the request, so the server must be reachable from the internet over https (reverse proxy or tunnel;
 * localhost and LAN addresses cannot work), and Apps Script allows a request about 30-60 seconds: slow local models may time out.
 * Use `search` mode (no model) when answers are slow.
 */
function onOpen() {
  DocumentApp.getUi().createAddonMenu()
    .addItem('Ask about selection', 'askSelection')
    .addItem('Open sidebar', 'openSidebar')
    .addItem('Settings', 'openSettings')
    .addToUi();
}

function openSidebar() { DocumentApp.getUi().showSidebar(HtmlService.createHtmlOutputFromFile('Sidebar').setTitle('TEZAUR')); }

function openSettings() {
  const ui = DocumentApp.getUi(), p = PropertiesService.getUserProperties();
  const s = ui.prompt('TEZAUR server (https://…)', p.getProperty('server') || '', ui.ButtonSet.OK_CANCEL);
  if (s.getSelectedButton() !== ui.Button.OK) return;
  const t = ui.prompt('Access token (empty = none)', '', ui.ButtonSet.OK_CANCEL);
  if (t.getSelectedButton() !== ui.Button.OK) return;
  saveSettings(s.getResponseText().trim(), t.getResponseText().trim());
}

function saveSettings(server, token) {
  const p = PropertiesService.getUserProperties();
  p.setProperty('server', server); p.setProperty('token', token || '');
}

/** Calls the server. Returns {text} or {error}. mode: ask | compare | gaps | search */
function query(q, mode) {
  const p = PropertiesService.getUserProperties(), server = (p.getProperty('server') || '').replace(/\/$/, ''), token = p.getProperty('token') || '';
  if (!/^https:\/\//.test(server)) return { error: 'Set an https:// server address in Settings (Google cannot reach http or LAN addresses).' };
  const search = mode === 'search';
  let res;
  try {
    res = UrlFetchApp.fetch(server + (search ? '/api/search' : '/api/ask'), {
      method: 'post', contentType: 'application/json', muteHttpExceptions: true,
      headers: token ? { Authorization: 'Bearer ' + token } : {},
      payload: JSON.stringify({ q: q, mode: mode, k: 6 }),
    });
  } catch (e) { return { error: 'Cannot reach the server (' + e.message + '). Slow model? Try search mode.' }; }
  let d;
  try { d = JSON.parse(res.getContentText()); } catch (e) { return { error: 'Unexpected answer (HTTP ' + res.getResponseCode() + ')' }; }
  if (d.error) return { error: d.error };
  if (search) return { text: d.hits.map(function (h) { return '[' + h.n + '] ' + h.label + '\n' + h.text.replace(/\s+/g, ' ').slice(0, 300); }).join('\n\n') || 'No matching passages.' };
  let text = d.answer;
  if (d.references) text += '\n\nReferences\n' + d.references;
  (d.warnings || []).forEach(function (w) { text += '\n⚠ ' + w; });
  return { text: text };
}

function selectedText_() {
  const sel = DocumentApp.getActiveDocument().getSelection();
  if (!sel) return '';
  return sel.getRangeElements().map(function (r) {
    const el = r.getElement();
    if (!el.editAsText) return '';
    const t = el.asText().getText();
    return r.isPartial() ? t.substring(r.getStartOffset(), r.getEndOffsetInclusive() + 1) : t;
  }).join(' ').trim();
}

function askSelection() {
  const q = selectedText_();
  if (!q) { DocumentApp.getUi().alert('Select the question first.'); return; }
  const r = query(q, 'ask');
  insertAnswer(r.error ? 'TEZAUR: ' + r.error : r.text);
}

/** Inserts text as a new paragraph after the selection (or at the cursor / end of the document). */
function insertAnswer(text) {
  const doc = DocumentApp.getActiveDocument(), body = doc.getBody(), sel = doc.getSelection();
  if (sel) {
    const last = sel.getRangeElements().pop().getElement();
    const par = last.getType() === DocumentApp.ElementType.PARAGRAPH ? last : last.getParent();
    const i = body.getChildIndex(par.getType() === DocumentApp.ElementType.PARAGRAPH ? par : par.getParent());
    body.insertParagraph(i + 1, text);
  } else body.appendParagraph(text);
}
