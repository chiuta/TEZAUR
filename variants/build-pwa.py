#!/usr/bin/env python3
"""Regenerates pwa/index.html from tezaur.html (manifest link, theme colour, service worker registration) and bumps the
service-worker cache name from a hash of the page, so installed copies update. Run after every change to tezaur.html."""
import hashlib, pathlib, re
root = pathlib.Path(__file__).resolve().parent.parent
html = (root / 'tezaur.html').read_text(encoding='utf-8')
head = '<link rel="manifest" href="manifest.webmanifest">\n<meta name="theme-color" content="#8a4b14">\n<link rel="icon" href="icon-192.png">\n'
html = html.replace('<title>TEZAUR</title>', '<title>TEZAUR</title>\n' + head, 1)
reg = "if('serviceWorker' in navigator&&location.protocol.startsWith('http'))navigator.serviceWorker.register('sw.js').catch(()=>{});\n"
html = html.replace('</script>\n</body>', reg + '</script>\n</body>', 1)
(root / 'pwa' / 'index.html').write_text(html, encoding='utf-8')
v = hashlib.sha1(html.encode()).hexdigest()[:8]
sw = root / 'pwa' / 'sw.js'
sw.write_text(re.sub(r"const C='tezaur-[^']*'", "const C='tezaur-" + v + "'", sw.read_text(encoding='utf-8'), count=1), encoding='utf-8')
print('pwa/index.html regenerated, cache tezaur-' + v)
