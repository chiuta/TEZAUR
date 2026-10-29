#!/usr/bin/env bash
# Assembles dist/TezaurPortable from the template + freshly built binaries (Linux/macOS test build).
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release
rm -rf dist/TezaurPortable && mkdir -p dist
cp -r packaging/TezaurPortable dist/
cp target/release/tezaur dist/TezaurPortable/App/Tezaur/
cp target/release/TezaurPortable dist/TezaurPortable/
echo "ready: dist/TezaurPortable"
