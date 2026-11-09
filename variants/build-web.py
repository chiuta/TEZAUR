#!/usr/bin/env python3
"""Splits tezaur.html into index.html + app.js + app.css (no inline code: needed by extension CSP,
Electron/Tauri/VS Code/Obsidian wrappers and strict hosting). Usage: build-web.py OUT_DIR [--local-pdfjs]
--local-pdfjs loads pdf.js from OUT_DIR/vendor/pdfjs (copied from pwa/vendor/pdfjs) instead of the CDN; the browser
extension uses it because its CSP blocks remote scripts. Without it the output is as before."""
import re, sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import pdfjs_local
src = pathlib.Path(__file__).resolve().parent.parent / 'tezaur.html'
local = '--local-pdfjs' in sys.argv[2:]
out = pathlib.Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
h = src.read_text(encoding='utf-8')
if local:
    h = pdfjs_local.localize(h)
    pdfjs_local.copy_vendor(out)
css = re.search(r'<style>(.*?)</style>', h, re.S).group(1)
js = re.search(r'<script>(.*?)</script>', h, re.S).group(1)
h = h.replace(re.search(r'<style>.*?</style>', h, re.S).group(0), '<link rel="stylesheet" href="app.css">')
h = h.replace(re.search(r'<script>.*?</script>', h, re.S).group(0), '<script src="app.js"></script>')
(out/'index.html').write_text(h, encoding='utf-8'); (out/'app.css').write_text(css, encoding='utf-8'); (out/'app.js').write_text(js, encoding='utf-8')
