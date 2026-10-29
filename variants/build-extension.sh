#!/usr/bin/env bash
# Builds dist/tezaur-chrome.zip (Chrome / Edge / Brave) and dist/tezaur-firefox.zip from variants/extension.
set -euo pipefail
cd "$(dirname "$0")/extension"
python3 ../build-web.py app
mkdir -p ../../dist; rm -f ../../dist/tezaur-chrome.zip ../../dist/tezaur-firefox.zip
zip -qr ../../dist/tezaur-chrome.zip manifest.json background.js icon-128.png app
T=$(mktemp -d); cp -r background.js icon-128.png app "$T"; cp manifest.firefox.json "$T/manifest.json"
(cd "$T" && zip -qr "$OLDPWD/../../dist/tezaur-firefox.zip" .)
echo "built dist/tezaur-chrome.zip and dist/tezaur-firefox.zip"
