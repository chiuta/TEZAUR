#!/usr/bin/env python3
"""TEZAUR language server (stdio, stdlib only). Works in any LSP editor (VS Code, Neovim, Emacs eglot, Helix, Zed, Sublime LSP, JetBrains).
Features for Markdown/LaTeX/plain text:
  * completion after `@`  -> citation keys from your BibTeX (via `tezaur serve`)
  * hover on `@key`       -> "Smith & Doe (2019)" + title
  * diagnostics           -> `[@key]` that is not in the bibliography
  * on save               -> citation audit: sentences whose cited passage does not cover them, wrong pages, quotations not found
Settings: TEZAUR_SERVER (default http://127.0.0.1:8787), TEZAUR_TOKEN."""
import json
import os
import re
import sys
import time
import urllib.request

SERVER = os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8787").rstrip("/")
TOKEN = os.environ.get("TEZAUR_TOKEN")
KEY = r"[A-Za-z0-9_][A-Za-z0-9_:.#$%&\-+?<>~/]*"
GROUP = re.compile(r"\[[^\]\n]*@[^\]\n]*\]")
CITE = re.compile(r"@(" + KEY + ")")

_cache = {"t": 0, "keys": {}}
docs = {}


def fetch_keys(force=False):
    if not force and time.time() - _cache["t"] < 30:
        return _cache["keys"]
    req = urllib.request.Request(SERVER + "/api/sources")
    if TOKEN:
        req.add_header("Authorization", "Bearer " + TOKEN)
    try:
        d = json.load(urllib.request.urlopen(req, timeout=10))
        _cache["keys"] = {k["key"]: k for k in d.get("keys", [])}
    except Exception:  # server down: keep what we had
        pass
    _cache["t"] = time.time()
    return _cache["keys"]


def citations(text):
    """Yield (line, start_col, end_col, key) for every @key inside [ ... ]."""
    for ln, line in enumerate(text.split("\n")):
        for g in GROUP.finditer(line):
            for m in CITE.finditer(g.group(0)):
                key = m.group(1).rstrip(".:")
                s = g.start() + m.start() + 1
                yield ln, s, s + len(key), key


def diagnostics(text, keys):
    out = []
    if not keys:  # no bibliography loaded: say nothing rather than flag everything
        return out
    for ln, s, e, key in citations(text):
        if key not in keys:
            out.append({"range": {"start": {"line": ln, "character": s}, "end": {"line": ln, "character": e}}, "severity": 2,
                        "source": "tezaur", "message": "Citation key '%s' is not in the bibliography" % key})
    return out


audits = {}
VERDICT_SEVERITY = {"unsupported": 1, "unknown-source": 1, "page-out-of-range": 1, "quote-not-found": 1, "wrong-page": 2, "weak": 2, "quote-elsewhere": 2, "uncited": 3}


def audit_diagnostics(text):
    """Run the server's citation audit (no model involved) and place each finding on the line of its sentence."""
    body = json.dumps({"text": text}).encode()
    req = urllib.request.Request(SERVER + "/api/audit", body, {"Content-Type": "application/json"})
    if TOKEN:
        req.add_header("Authorization", "Bearer " + TOKEN)
    try:
        res = json.load(urllib.request.urlopen(req, timeout=30))
    except Exception:
        return []
    lines, out = text.split("\n"), []
    for f in res.get("findings", []):
        sev = VERDICT_SEVERITY.get(f["verdict"])
        if not sev:
            continue
        probe = f["sentence"][:40]
        ln = next((i for i, l in enumerate(lines) if probe in l), 0)
        msg = "%s: %s" % (f["verdict"], f["note"])
        out.append({"range": {"start": {"line": ln, "character": 0}, "end": {"line": ln, "character": len(lines[ln]) if ln < len(lines) else 0}},
                    "severity": sev, "source": "tezaur-audit", "message": msg})
    return out


def word_at(text, line, ch):
    lines = text.split("\n")
    if line >= len(lines):
        return None
    for m in CITE.finditer(lines[line]):
        if m.start() <= ch <= m.end():
            return m.group(1).rstrip(".:")
    return None


def completion(text, line, ch, keys):
    lines = text.split("\n")
    prefix = lines[line][:ch] if line < len(lines) else ""
    m = re.search(r"@(" + KEY + r")?$", prefix)
    if not m:
        return []
    typed = m.group(1) or ""
    start = ch - len(typed)
    return [{"label": k, "kind": 18, "detail": v.get("cite", ""), "documentation": v.get("title", ""),
             "textEdit": {"range": {"start": {"line": line, "character": start}, "end": {"line": line, "character": ch}}, "newText": k}}
            for k, v in sorted(keys.items()) if k.lower().startswith(typed.lower())]


def hover(text, line, ch, keys):
    k = word_at(text, line, ch)
    if k and k in keys:
        v = keys[k]
        return {"contents": {"kind": "markdown", "value": "**%s**\n\n%s" % (v.get("cite", k), v.get("title", ""))}}
    return None


# ------------------------------------------------------------------ protocol
def send(msg):
    b = json.dumps(msg).encode("utf-8")
    sys.stdout.buffer.write(b"Content-Length: %d\r\n\r\n" % len(b) + b)
    sys.stdout.buffer.flush()


def read():
    n = None
    while True:
        h = sys.stdin.buffer.readline()
        if not h:
            return None
        h = h.strip()
        if not h:
            break
        if h.lower().startswith(b"content-length:"):
            n = int(h.split(b":")[1])
    return json.loads(sys.stdin.buffer.read(n)) if n else None


def publish(uri):
    send({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": {"uri": uri, "diagnostics": diagnostics(docs.get(uri, ""), fetch_keys()) + audits.get(uri, [])}})


def handle(m):
    method, p, i = m.get("method"), m.get("params", {}), m.get("id")
    if method == "initialize":
        fetch_keys(True)
        send({"jsonrpc": "2.0", "id": i, "result": {"capabilities": {"textDocumentSync": {"openClose": True, "change": 1, "save": {"includeText": True}}, "hoverProvider": True,
              "completionProvider": {"triggerCharacters": ["@"]}}, "serverInfo": {"name": "tezaur-lsp"}}})
    elif method == "shutdown":
        send({"jsonrpc": "2.0", "id": i, "result": None})
    elif method == "exit":
        sys.exit(0)
    elif method == "textDocument/didOpen":
        d = p["textDocument"]
        docs[d["uri"]] = d["text"]
        publish(d["uri"])
    elif method == "textDocument/didChange":
        docs[p["textDocument"]["uri"]] = p["contentChanges"][-1]["text"]
        publish(p["textDocument"]["uri"])
    elif method == "textDocument/didSave":
        u = p["textDocument"]["uri"]
        if "text" in p:
            docs[u] = p["text"]
        audits[u] = audit_diagnostics(docs.get(u, ""))
        publish(u)
    elif method == "textDocument/didClose":
        docs.pop(p["textDocument"]["uri"], None)
        audits.pop(p["textDocument"]["uri"], None)
    elif method == "textDocument/completion":
        pos = p["position"]
        send({"jsonrpc": "2.0", "id": i, "result": {"isIncomplete": False, "items": completion(docs.get(p["textDocument"]["uri"], ""), pos["line"], pos["character"], fetch_keys())}})
    elif method == "textDocument/hover":
        pos = p["position"]
        send({"jsonrpc": "2.0", "id": i, "result": hover(docs.get(p["textDocument"]["uri"], ""), pos["line"], pos["character"], fetch_keys())})
    elif i is not None:
        send({"jsonrpc": "2.0", "id": i, "error": {"code": -32601, "message": "method not found"}})


if __name__ == "__main__":
    while True:
        msg = read()
        if msg is None:
            break
        handle(msg)
