#!/usr/bin/env python3
"""Splits tezaur.html into index.html + app.js + app.css (no inline code: needed by extension CSP,
Electron/Tauri/VS Code/Obsidian wrappers and strict hosting). Usage: build-web.py OUT_DIR"""
import re, sys, pathlib
src = pathlib.Path(__file__).resolve().parent.parent / 'tezaur.html'
out = pathlib.Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
h = src.read_text(encoding='utf-8')
css = re.search(r'<style>(.*?)</style>', h, re.S).group(1)
js = re.search(r'<script>(.*?)</script>', h, re.S).group(1)
h = h.replace(re.search(r'<style>.*?</style>', h, re.S).group(0), '<link rel="stylesheet" href="app.css">')
h = h.replace(re.search(r'<script>.*?</script>', h, re.S).group(0), '<script src="app.js"></script>')
(out/'index.html').write_text(h, encoding='utf-8'); (out/'app.css').write_text(css, encoding='utf-8'); (out/'app.js').write_text(js, encoding='utf-8')
