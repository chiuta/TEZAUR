#!/usr/bin/env bash
# Builds dist/office/: taskpane.html (tezaur.html + "insert into document" button), manifest.xml, icons.
# Publish the folder over HTTPS, put the address in manifest.xml (HOST) and sideload the manifest in Word.
set -euo pipefail
cd "$(dirname "$0")"
O=../dist/office; mkdir -p "$O"
python3 - <<'PY'
import pathlib
h = pathlib.Path('../tezaur.html').read_text(encoding='utf-8')
h = h.replace('</head>', '<script src="https://appsforoffice.microsoft.com/lib/1/hosted/office.js"></script>\n</head>', 1)
add = """
if (window.Office && Office.onReady) Office.onReady(function (i) {
  if (!i || !i.host) return;
  document.getElementById('dlMd').insertAdjacentHTML('afterend', '<button class="alt" id="insDoc">Inserează în document</button>');
  document.getElementById('insDoc').onclick = function () {
    var t = lastText + (document.getElementById('refs').innerText ? '\\n\\n' + document.getElementById('refs').innerText : '');
    Word.run(function (ctx) { ctx.document.getSelection().insertText(t, 'Replace'); return ctx.sync(); }).catch(function (e) { alert('Eroare: ' + e.message); });
  };
});
"""
h = h.replace('</script>\n</body>', add + '</script>\n</body>', 1)
pathlib.Path('../dist/office/taskpane.html').write_text(h, encoding='utf-8')
PY
cp office/manifest.xml ../pwa/icon-192.png ../pwa/icon-512.png "$O/"
echo "built dist/office"
