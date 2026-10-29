#!/usr/bin/env bash
# Builds dist/obsidian/{main.js,manifest.json,styles.css}: copy that folder to <vault>/.obsidian/plugins/tezaur/
set -euo pipefail
cd "$(dirname "$0")"
O=../dist/obsidian; mkdir -p "$O"
{ echo "const core = (function () { const module = { exports: {} };"; cat core/tezaur-core.js; echo "return module.exports; })();"; cat obsidian/plugin.js; } > "$O/main.js"
cp obsidian/manifest.json obsidian/styles.css "$O/"
node --check "$O/main.js" && echo "built dist/obsidian"
