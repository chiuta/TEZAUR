"""Shared by build-pwa.py and build-web.py: point tezaur.html's pdf.js loader at the bundled copy in vendor/pdfjs/
instead of the CDN (the PWA and the browser extension must not need the network, and the extension's CSP blocks it)."""
import pathlib, shutil

CDN = "const base='https://cdn.jsdelivr.net/npm/pdfjs-dist@3.11.174/build/';"
LOCAL = "const base='vendor/pdfjs/';"
OLD_MSG = "PDF-urile cer biblioteca pdf.js (se încarcă o singură dată de pe internet). Conectează-te sau convertește PDF-ul în .txt/.docx."
NEW_MSG = "PDF-urile cer biblioteca pdf.js (inclusă local, nu s-a putut încărca). Reîncarcă pagina sau convertește PDF-ul în .txt/.docx."
SRC = pathlib.Path(__file__).resolve().parent.parent / "pwa" / "vendor" / "pdfjs"


def localize(text):
    """Return text with the CDN loader replaced by the local one; fail loudly if the loader is not found."""
    if CDN not in text:
        raise SystemExit("pdfjs_local: the CDN loader line was not found in tezaur.html")
    return text.replace(CDN, LOCAL, 1).replace(OLD_MSG, NEW_MSG, 1)


def copy_vendor(dest_dir):
    """Copy the vendored pdf.js (canonical copy: pwa/vendor/pdfjs) to dest_dir/vendor/pdfjs."""
    d = pathlib.Path(dest_dir) / "vendor" / "pdfjs"
    d.mkdir(parents=True, exist_ok=True)
    for f in SRC.iterdir():
        shutil.copy2(f, d / f.name)
