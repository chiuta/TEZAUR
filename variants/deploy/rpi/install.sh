#!/bin/bash
# Raspberry Pi / Debian / Ubuntu: installs TEZAUR as an always-on research box that announces itself on the LAN (mDNS + mesh beacon).
# Usage: sudo ./install.sh [path-to-tezaur-binary-or-.deb]     Without an argument it builds from source with cargo (slow on a Pi: ~20-40 min).
set -euo pipefail
[ "$(id -u)" = 0 ] || { echo "run with sudo"; exit 1; }
apt-get update -qq
apt-get install -y --no-install-recommends poppler-utils tesseract-ocr tesseract-ocr-eng tesseract-ocr-ron avahi-daemon curl ca-certificates || apt-get install -y poppler-utils tesseract-ocr avahi-daemon curl
SRC="${1:-}"
if [ -z "$SRC" ]; then
  command -v cargo >/dev/null || { apt-get install -y cargo build-essential pkg-config; }
  (cd "$(dirname "$0")/../../.." && cargo build --release) && install -m755 "$(dirname "$0")/../../../target/release/tezaur" /usr/bin/tezaur
elif [[ "$SRC" == *.deb ]]; then dpkg -i "$SRC"; else install -m755 "$SRC" /usr/bin/tezaur; fi
id tezaur >/dev/null 2>&1 || useradd --system --home /var/lib/tezaur --shell /usr/sbin/nologin tezaur
mkdir -p /var/lib/tezaur/workspaces && chown -R tezaur:tezaur /var/lib/tezaur
if [ ! -f /etc/tezaur.env ]; then
  TOKEN=$(head -c 24 /dev/urandom | base64 | tr -dc 'A-Za-z0-9' | head -c 24)
  printf 'TEZAUR_TOKEN=%s\nTEZAUR_API=ollama\nTEZAUR_HOST=http://127.0.0.1:11434\n' "$TOKEN" > /etc/tezaur.env && chmod 600 /etc/tezaur.env
  echo "Access token (saved in /etc/tezaur.env): $TOKEN"
fi
cp "$(dirname "$0")/../systemd/tezaur.service" /etc/systemd/system/tezaur.service
cp "$(dirname "$0")/tezaur.avahi.service" /etc/avahi/services/tezaur.service
systemctl daemon-reload && systemctl enable --now tezaur avahi-daemon
echo "TEZAUR runs on http://$(hostname).local:8787/?token=<token>   (a model server such as Ollama is installed separately)"
