// TEZAUR for Zotero 7 (bootstrapped plugin). UNTESTED: written without a Zotero installation to run it in.
// Right-click selected items > "Send to TEZAUR": each attached file is uploaded to POST /api/ingest on your server.
var SERVER = 'http://127.0.0.1:8787', TOKEN = '', ids = [];

async function sendItems(items) {
  let sent = 0, failed = 0;
  for (const it of items) {
    const atts = it.isAttachment() ? [it] : (await Zotero.Items.getAsync(it.getAttachments()));
    for (const a of atts) {
      const path = await a.getFilePathAsync();
      if (!path) continue;
      const name = PathUtils.filename(path);
      try {
        const bytes = await IOUtils.read(path);
        const headers = { 'Content-Type': 'application/octet-stream' };
        if (TOKEN) headers.Authorization = 'Bearer ' + TOKEN;
        const r = await fetch(SERVER + '/api/ingest?name=' + encodeURIComponent(name), { method: 'POST', headers, body: bytes });
        r.ok ? sent++ : failed++;
      } catch (e) { failed++; }
    }
  }
  Zotero.alert(null, 'TEZAUR', `${sent} file(s) sent, ${failed} failed.`);
}

function install() {}
function uninstall() {}
function startup({ id }) {
  Zotero.debug('TEZAUR: plugin started');
  Zotero.MenuManager?.registerMenu?.({ menuID: 'tezaur-send', pluginID: id, target: 'main/library/item',
    menus: [{ menuType: 'menuitem', l10nID: null, onCommand: () => sendItems(Zotero.getActiveZoteroPane().getSelectedItems()) }] });
}
function shutdown() {}
function onMainWindowLoad({ window }) {
  const doc = window.document, menu = doc.getElementById('zotero-itemmenu');
  if (!menu || doc.getElementById('tezaur-send-item')) return;
  const mi = doc.createXULElement('menuitem');
  mi.id = 'tezaur-send-item'; mi.setAttribute('label', 'Send to TEZAUR');
  mi.addEventListener('command', () => sendItems(Zotero.getActiveZoteroPane().getSelectedItems()));
  menu.appendChild(mi);
  Zotero.debug('TEZAUR: menu item added to the item context menu');
}
function onMainWindowUnload({ window }) { window.document.getElementById('tezaur-send-item')?.remove(); }
