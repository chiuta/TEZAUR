#!/usr/bin/env bash
# Builds dist/tezaur_<version>_<arch>.deb from the release binary (cargo build --release first).
set -euo pipefail
cd "$(dirname "$0")/.."
VER=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
ARCH=$(dpkg --print-architecture)
R=$(mktemp -d)/tezaur_${VER}_${ARCH}
mkdir -p "$R/usr/share/man/man1" "$R/DEBIAN" "$R/usr/bin" "$R/usr/share/doc/tezaur" "$R/usr/share/applications" "$R/usr/share/icons/hicolor/512x512/apps" "$R/usr/share/tezaur"
install -m755 target/release/tezaur "$R/usr/bin/tezaur"
install -m644 README.md "$R/usr/share/doc/tezaur/README.md"
install -m644 LICENSE "$R/usr/share/doc/tezaur/copyright"
# changelog (Debian format, native package) and man page
printf 'tezaur (%s) unstable; urgency=medium\n\n  * Release %s. See https://github.com/chiuta/TEZAUR/releases for the notes.\n\n -- Alexio <alexio@trom.tf>  %s\n' "$VER" "$VER" "$(date -R)" | gzip -9n > "$R/usr/share/doc/tezaur/changelog.gz"
{ printf '.TH TEZAUR 1 "" "tezaur %s" "User Commands"\n.SH NAME\ntezaur \\- local, citation-grounded research assistant\n.SH SYNOPSIS\n.B tezaur\n[\\fIcommand\\fR] [\\fIoptions\\fR]\n.SH DESCRIPTION\nIndexes your own documents and answers only from them, with checkable citations. Runs against a local model server.\n.SH COMMANDS\n.nf\n' "$VER"; target/release/tezaur --help 2>&1 | sed 's/^\./\\&./; s/\\/\\\\/g'; printf '.fi\n.SH SEE ALSO\nhttps://github.com/chiuta/TEZAUR\n.SH AUTHOR\nAlexio <alexio@trom.tf>\n'; } | gzip -9n > "$R/usr/share/man/man1/tezaur.1.gz"
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
Keywords=thesis;citations;research;library;
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
Depends: libc6, libgcc-s1, xdg-utils
Recommends: poppler-utils
Homepage: https://tezaur.chiuta.ro
Description: local, citation-grounded research assistant for writing a thesis
 Indexes your books and articles (pdf, docx, epub, txt, html, tex) and answers
 with citations drawn only from them, using a local model server (Ollama, Jan,
 LM Studio). Also runs as an MCP server. Includes the single-file web version.
D
(cd "$R" && find usr -type f -print0 | sort -z | xargs -0 md5sum > DEBIAN/md5sums)
mkdir -p dist; dpkg-deb --root-owner-group --build "$R" "dist/tezaur_${VER}_${ARCH}.deb" >/dev/null
echo "built dist/tezaur_${VER}_${ARCH}.deb"
