#!/data/data/com.termux/files/usr/bin/bash
# TEZAUR on an Android phone, inside Termux (F-Droid build of Termux). Builds the real Rust server with OCR, no APK needed.
set -e
pkg update -y && pkg install -y rust git poppler tesseract termux-services
if [ -d tezaur ]; then cd tezaur; else git clone https://github.com/chiuta/tezaur && cd tezaur; fi
cargo install --path . --root "$PREFIX"
mkdir -p "$HOME/tezaur-data"
mkdir -p "$PREFIX/var/service/tezaur" && cat > "$PREFIX/var/service/tezaur/run" <<RUN
#!/data/data/com.termux/files/usr/bin/sh
export TEZAUR_INDEX=$HOME/tezaur-data/tezaur.idx
exec tezaur serve 2>&1
RUN
chmod +x "$PREFIX/var/service/tezaur/run"
echo "Start: sv-enable tezaur   then open http://127.0.0.1:8787 in the phone's browser (or install the PWA)."
echo "A small model on the phone is slow; usually point TEZAUR_HOST at Ollama on your PC."
