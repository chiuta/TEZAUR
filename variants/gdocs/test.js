// Loads Code.gs with mocked Apps Script services; UrlFetchApp really calls the live TEZAUR server (via curl).
const fs = require('fs'), vm = require('vm'), cp = require('child_process');
const props = {}; let inserted = [];
const ctx = {
  console, JSON,
  PropertiesService: { getUserProperties: () => ({ getProperty: k => props[k], setProperty: (k, v) => props[k] = v }) },
  UrlFetchApp: { fetch: (url, o) => { const r = cp.spawnSync('curl', ['-s', '-w', '\n%{http_code}', '-H', 'Content-Type: application/json', '-d', o.payload, url.replace('https://', 'http://')]); const out = r.stdout.toString(); const i = out.lastIndexOf('\n');
      return { getContentText: () => out.slice(0, i), getResponseCode: () => +out.slice(i + 1) }; } },
  DocumentApp: { ElementType: { PARAGRAPH: 'P' }, getActiveDocument: () => ({ getSelection: () => null, getBody: () => ({ appendParagraph: t => inserted.push(t) }) }), getUi: () => ({}) },
  HtmlService: {},
};
vm.createContext(ctx); vm.runInContext(fs.readFileSync(process.argv[2] + '/Code.gs', 'utf8'), ctx);
const a = ctx.query('x', 'ask'); if (!/https/.test(a.error)) throw new Error('should demand https: ' + JSON.stringify(a));
ctx.saveSettings('https://127.0.0.1:8810', '');
const r = ctx.query('burden of proof', 'ask'); if (!/burden of proof lies/.test(r.text) || !/References/.test(r.text)) throw new Error('ask ' + JSON.stringify(r));
const s = ctx.query('burden', 'search'); if (!/^\[1\]/.test(s.text)) throw new Error('search ' + JSON.stringify(s));
ctx.insertAnswer('hello'); if (inserted[0] !== 'hello') throw new Error('insert');
console.log('apps script ok');
