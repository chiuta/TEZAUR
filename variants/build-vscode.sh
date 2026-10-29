#!/usr/bin/env bash
# Builds dist/vscode/ (install: copy to ~/.vscode/extensions/trom.tezaur-1.0.0 or `npx @vscode/vsce package` inside it).
set -euo pipefail
cd "$(dirname "$0")"
O=../dist/vscode; mkdir -p "$O"
cp vscode/package.json vscode/extension.js core/tezaur-core.js "$O/"
node --check "$O/extension.js" && echo "built dist/vscode"
