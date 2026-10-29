#!/usr/bin/env bash
# Builds dist/tezaur_<version>_<arch>.deb from the release binary (cargo build --release first).
set -euo pipefail
cd "$(dirname "$0")/.."
VER=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
ARCH=$(dpkg --print-architecture)
R=$(mktemp -d)/tezaur_${VER}_${ARCH}
mkdir -p "$R/DEBIAN" "$R/usr/bin" "$R/usr/share/doc/tezaur" "$R/usr/share/applications" "$R/usr/share/icons/hicolor/512x512/apps" "$R/usr/share/tezaur"
install -m755 target/release/tezaur "$R/usr/bin/tezaur"
install -m644 README.md "$R/usr/share/doc/tezaur/README.md"
install -m644 LICENSE "$R/usr/share/doc/tezaur/copyright"
install -m644 tezaur.html "$R/usr/share/tezaur/tezaur.html"
install -m644 pwa/icon-512.png "$R/usr/share/icons/hicolor/512x512/apps/tezaur.png"
cat > "$R/usr/share/applications/tezaur-web.desktop" <<D
[Desktop Entry]
Type=Application
Name=TEZAUR (web)
Comment=Asistent local de cercetare pentru teză
Exec=xdg-open /usr/share/tezaur/tezaur.html
Icon=tezaur
Categories=Education;Office;
D
SIZE=$(du -sk "$R/usr" | cut -f1)
cat > "$R/DEBIAN/control" <<D
Package: tezaur
Version: $VER
Section: education
Priority: optional
Architecture: $ARCH
Installed-Size: $SIZE
Maintainer: Alexio <alexio@trom.tf>
Recommends: poppler-utils
Description: local, citation-grounded research assistant for writing a thesis
 Indexes your books and articles (pdf, docx, epub, txt, html, tex) and answers
 with citations drawn only from them, using a local model server (Ollama, Jan,
 LM Studio). Also runs as an MCP server. Includes the single-file web version.
D
mkdir -p dist; dpkg-deb --root-owner-group --build "$R" "dist/tezaur_${VER}_${ARCH}.deb" >/dev/null
echo "built dist/tezaur_${VER}_${ARCH}.deb"
