"""TEZAUR for Notepad++ via the PythonScript plugin (Plugins > Plugins Admin > "Python Script").
Install: Plugins > Python Script > New Script... -> save this file as tezaur_npp.py, then run it.
Select text = the question; otherwise a prompt appears. The answer is inserted below the selection.
Optional: Plugins > Python Script > Configuration > add it to the menu / assign a shortcut.
(A native Notepad++ plugin is a C++/.NET DLL; this script needs no compiler.)"""
import json
import os

try:
    from urllib.request import Request, urlopen  # PythonScript 3
except ImportError:  # pragma: no cover
    from urllib2 import Request, urlopen

SERVER = os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8787")
TOKEN = os.environ.get("TEZAUR_TOKEN")
MODE = os.environ.get("TEZAUR_MODE", "ask")  # ask | compare | gaps | search


def call(mode, q, k=6):
    path = "/api/search" if mode == "search" else "/api/ask"
    req = Request(SERVER.rstrip("/") + path, json.dumps({"q": q, "mode": mode, "k": k}).encode("utf-8"), {"Content-Type": "application/json"})
    if TOKEN:
        req.add_header("Authorization", "Bearer " + TOKEN)
    return json.loads(urlopen(req, timeout=600).read().decode("utf-8"))


def format_result(mode, d):
    if "error" in d:
        return "Error: " + d["error"]
    if mode == "search":
        return "\n".join("[%d] %s\n    %s" % (h["n"], h["label"], " ".join(h["text"][:300].split())) for h in d.get("hits", []))
    out = d["answer"]
    if d.get("references"):
        out += "\n\n## References\n" + d["references"]
    for w in d.get("warnings", []):
        out += "\n(!) " + w
    return out


def main(editor, notepad, mode=MODE):
    q = (editor.getSelText() or "").strip() or (notepad.prompt("Question for TEZAUR:", "TEZAUR", "") or "").strip()
    if not q:
        return
    try:
        text = format_result(mode, call(mode, q))
    except Exception as e:  # noqa
        text = "Error: cannot reach %s (%s)" % (SERVER, e)
    editor.documentEnd() if not editor.getSelText() else editor.setCurrentPos(editor.getSelectionEnd())
    editor.addText("\n\n" + text + "\n")


if __name__ == "__main__":
    try:
        main(editor, notepad)  # noqa: F821 -- provided by the PythonScript plugin
    except NameError:
        print("Run this inside Notepad++ (PythonScript plugin).")
