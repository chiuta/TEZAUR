# TEZAUR for LibreOffice (Writer): Python-UNO macros that talk to a running `tezaur serve`.
# Install: copy this file to  ~/.config/libreoffice/4/user/Scripts/python/   (Windows: %APPDATA%\LibreOffice\4\user\Scripts\python\)
# Run: Tools → Macros → Run Macro… → My Macros → tezaur_lo → find_sources / insert_references
# Settings: environment variables TEZAUR_URL (default http://127.0.0.1:8787) and TEZAUR_TOKEN.
import json, os, urllib.parse, urllib.request

URL = os.environ.get("TEZAUR_URL", "http://127.0.0.1:8787").rstrip("/")
TOKEN = os.environ.get("TEZAUR_TOKEN", "")


def search(query, k=5):
    r = urllib.request.Request(f"{URL}/api/search?q={urllib.parse.quote(query)}&k={k}")
    if TOKEN:
        r.add_header("Authorization", "Bearer " + TOKEN)
    with urllib.request.urlopen(r, timeout=60) as resp:
        return json.load(resp)["hits"]


def format_references(hits):
    """'[1] Smith (2019), p. 3 — first words of the passage…' one per line."""
    return "\n".join(f"[{h['n']}] {h['label']} — {' '.join(h['text'].split())[:120]}…" for h in hits)


def _selected_text(doc):
    sel = doc.getCurrentController().getSelection()
    if sel.getCount() and sel.getByIndex(0).getString():
        return sel.getByIndex(0), sel.getByIndex(0).getString()
    return None, ""


def _message(doc, text):
    from com.sun.star.awt.MessageBoxType import MESSAGEBOX
    win = doc.getCurrentController().getFrame().getContainerWindow()
    tk = win.getToolkit()
    tk.createMessageBox(win, MESSAGEBOX, 1, "TEZAUR", text).execute()


def find_sources(*args):
    """Select a claim or a few words, run this: shows the best matching passages of your library."""
    doc = XSCRIPTCONTEXT.getDocument()
    _, q = _selected_text(doc)
    if not q:
        return _message(doc, "Selectează întâi textul pentru care cauți surse.")
    try:
        hits = search(q)
    except Exception as e:
        return _message(doc, f"Nu pot contacta serverul TEZAUR ({URL}): {e}")
    _message(doc, format_references(hits) if hits else "Niciun pasaj potrivit.")


def insert_references(*args):
    """Same search, but inserts the numbered references as a new paragraph after the selection."""
    doc = XSCRIPTCONTEXT.getDocument()
    sel, q = _selected_text(doc)
    if not q:
        return _message(doc, "Selectează întâi textul pentru care cauți surse.")
    try:
        hits = search(q)
    except Exception as e:
        return _message(doc, f"Nu pot contacta serverul TEZAUR ({URL}): {e}")
    cur = doc.getText().createTextCursorByRange(sel.getEnd())
    doc.getText().insertString(cur, "\n" + (format_references(hits) if hits else "[nicio sursă găsită]"), False)


g_exportedScripts = (find_sources, insert_references)
