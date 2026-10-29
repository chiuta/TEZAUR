// Desktop shell: the web app in a window. webSecurity is off on purpose: the window only ever loads this
// app's own files, and it lets the page talk to a local Ollama / Jan server without CORS setup.
const { app, BrowserWindow, Menu, shell, dialog } = require('electron');
const path = require('path');

function create() {
  const win = new BrowserWindow({
    width: 1280, height: 860, title: 'TEZAUR', autoHideMenuBar: true,
    webPreferences: { webSecurity: false, contextIsolation: true, nodeIntegration: false }
  });
  Menu.setApplicationMenu(null);
  win.loadFile(path.join(__dirname, 'app', 'index.html'));
  win.webContents.setWindowOpenHandler(({ url }) => { if (/^https?:/.test(url)) shell.openExternal(url); return { action: 'deny' }; });
  // Downloads (Markdown, Anki .tsv) ask where to save.
  win.webContents.session.on('will-download', (e, item) => {
    const p = dialog.showSaveDialogSync(win, { defaultPath: item.getFilename() });
    if (p) item.setSavePath(p); else item.cancel();
  });
  if (process.env.TEZAUR_SELFTEST) {
    win.webContents.on('did-finish-load', async () => {
      const r = await win.webContents.executeJavaScript(
        "JSON.stringify({title:document.title,tabs:document.querySelectorAll('#tabs button').length,idb:typeof indexedDB,tokens:tokens('Șarcina probei').join(',')})");
      console.log('SELFTEST ' + r); app.quit();
    });
  }
}
app.whenReady().then(create);
app.on('window-all-closed', () => app.quit());
