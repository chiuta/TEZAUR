#!/bin/sh
# Linux / macOS: remove everything TEZAUR's installer put on this computer.
#   ./uninstall.sh              undo what the installer recorded
#   ./uninstall.sh --dry-run    only list
#   ./uninstall.sh --full       also sweep leftovers without the record, the tezaur program and ~/.tezaur
#   ./uninstall.sh --full --purge-data --yes    ALSO delete TEZAUR's index/settings folders (never your Library of PDFs)
cd "$(dirname "$0")" || exit 1
case "$1" in --full) shift; exec python3 tezaur-setup.py uninstall --sweep --purge "$@";; esac
exec python3 tezaur-setup.py uninstall "$@"
