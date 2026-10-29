#!/usr/bin/env bash
# Packages every plugin/integration into dist/plugins/ and one dist/tezaur-plugins.zip.   Usage: variants/build-plugins.sh
set -euo pipefail
V="$(cd "$(dirname "$0")" && pwd)"; OUT="$V/../dist/plugins"; rm -rf "$OUT"; mkdir -p "$OUT"
z() { local name="$1" dir="$2"; shift 2; (cd "$dir" && zip -qr "$OUT/$name" "$@" -x '*/__pycache__/*' '*.pyc'); }
z tezaur-wordpress.zip   "$V/cms/wordpress"   tezaur-research
z tezaur-dokuwiki.zip    "$V/cms/dokuwiki"    tezaur
z tezaur-mediawiki.zip   "$V/cms/mediawiki"   Tezaur
z tezaur-vim.zip         "$V/editors/vim"     plugin autoload doc
z tezaur-neovim.zip      "$V/editors/neovim"  lua plugin
cp "$V/editors/emacs/tezaur.el" "$OUT/"
T=$(mktemp -d); mkdir "$T/TEZAUR"; cp "$V/editors/sublime/"*.* "$T/TEZAUR/" 2>/dev/null || true; z tezaur-sublime.zip "$T" TEZAUR
z tezaur-notepadpp.zip   "$V/editors/notepadpp" .
# Joplin .jpl = tar of manifest.json + index.js
J=$(mktemp -d); cp "$V/editors/joplin/manifest.json" "$J/"; mkdir -p "$J/dist"; cp "$V/editors/joplin/src/index.js" "$J/index.js"; cp "$V/editors/joplin/src/index.js" "$J/dist/index.js"
(cd "$J" && tar cf "$OUT/tezaur-joplin.jpl" manifest.json index.js dist)
z tezaur-logseq.zip      "$V/editors/logseq"  .
(cd "$V/editors/zotero" && zip -qr "$OUT/tezaur-zotero.xpi" manifest.json bootstrap.js)
z tezaur-raycast.zip     "$V/editors/raycast" .
python3 "$V/editors/alfred/build.py" "$OUT/Tezaur.alfredworkflow" >/dev/null
z tezaur-latex.zip "$V/editors/latex" .
z tezaur-pandoc-quarto.zip "$V/editors/pandoc" tezaur.lua quarto
cp "$V/editors/lsp/tezaur_lsp.py" "$V/editors/rest/openapi.yaml" "$V/editors/rest/tezaur.http" "$V/calibre/tezaur_calibre.py" "$OUT/"
z tezaur-bots.zip        "$V/bots"            .
z tezaur-gdocs.zip       "$V/gdocs"           Code.gs Sidebar.html appsscript.json
z tezaur-deploy.zip      "$V/deploy"          .
z tezaur-yunohost-server.zip "$V" yunohost-serve
tar czf "$OUT/tezaur-helm-0.1.0.tgz" -C "$V/deploy/helm" tezaur
(cd "$V/npm" && npm pack --silent --pack-destination "$OUT" >/dev/null)
(cd "$V/pypi" && python3 -m pip wheel . --no-deps -q -w "$OUT" 2>&1 | grep -v WARNING || true; rm -rf build *.egg-info)
cp "$V/MESH.md" "$V/CONNECTORS.md" "$V/editors/rest/openapi.json" "$OUT/"
# things built by their own scripts (only included when already built)
D="$V/../dist"
[ -d "$D/vscode" ]   && (cd "$D/vscode"   && zip -qr "$OUT/tezaur-vscode.zip" .)
[ -d "$D/obsidian" ] && (cd "$D/obsidian" && zip -qr "$OUT/tezaur-obsidian.zip" .)
for f in tezaur-chrome.zip tezaur-firefox.zip; do [ -f "$D/$f" ] && cp "$D/$f" "$OUT/"; done
[ -d "$D/TezaurPortable" ] && (cd "$D" && zip -qr "$OUT/TezaurPortable.zip" TezaurPortable)
cp "$V/installer/tezaur-setup.py" "$V/installer/install.sh" "$V/installer/Install-Tezaur.bat" "$V/installer/uninstall.sh" "$V/installer/Uninstall-Tezaur.bat" "$OUT/"
( cd "$OUT" && zip -qr ../tezaur-plugins.zip . )
ls -1 "$OUT" | sed 's/^/  /'; echo "dist/tezaur-plugins.zip ready"
