#!/bin/sh
# Linux / macOS: detect and install. Scan first with no arguments; `./install.sh --yes` applies everything ready.
cd "$(dirname "$0")" || exit 1
if [ "$#" -eq 0 ]; then exec python3 tezaur-setup.py scan; fi
exec python3 tezaur-setup.py install "$@"
