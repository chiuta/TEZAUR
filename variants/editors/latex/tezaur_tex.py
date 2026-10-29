#!/usr/bin/env python3
"""TEZAUR for LaTeX-world editors: LyX, TeXstudio, Kate, Geany, Overleaf-export, any editor that can run a command (stdlib only).

    tezaur_tex.py audit  FILE.(tex|lyx|md)   # citations vs the cited page, quotations word-for-word; report on stdout, exit 2 on problems
    tezaur_tex.py ask    "question"          # cited answer
    tezaur_tex.py cite   "words" [--lyx]     # best passages as \\cite{key} lines; with --lyx, inserts the best \\cite into the open LyX document
    tezaur_tex.py coverage OUTLINE.md        # which sections have sources

Needs `tezaur serve` (TEZAUR_SERVER, TEZAUR_TOKEN). The server understands \\cite / \\parencite / \\textcite[p. 3]{key} and LyX citation insets itself."""
import json, os, sys, urllib.parse, urllib.request

SERVER = os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8787").rstrip("/")
TOKEN = os.environ.get("TEZAUR_TOKEN")

def call(path, body=None, params=None):
    url = SERVER + path + ("?" + urllib.parse.urlencode(params) if params else "")
    req = urllib.request.Request(url, json.dumps(body).encode() if body is not None else None, {"Content-Type": "application/json"} if body is not None else {})
    if TOKEN:
        req.add_header("Authorization", "Bearer " + TOKEN)
    try:
        return json.load(urllib.request.urlopen(req, timeout=600))
    except Exception as e:
        sys.exit("TEZAUR: cannot use %s (%s)" % (SERVER, e))

def lyx_pipe():
    for p in (os.environ.get("LYXPIPE"), os.path.expanduser("~/.lyx/lyxpipe"), os.path.expanduser("~/.config/lyx/lyxpipe")):
        if p and os.path.exists(p + ".in"):
            return p
    sys.exit("LyX server pipe not found. In LyX: Tools → Preferences → Paths → Server pipe path, then restart LyX (or set LYXPIPE).")

def lyx_send(text):
    p = lyx_pipe()
    with open(p + ".in", "w") as f:
        f.write("LYXCMD:tezaur:self-insert %s\n" % text.replace("\n", " "))

def cite_candidates(words):
    keys = call("/api/sources").get("keys", [])
    hits = call("/api/search", {"q": words, "k": 8})["hits"]
    out, seen = [], set()
    for h in hits:
        stem = os.path.splitext(os.path.basename(h["path"]))[0].lower()
        k = next((x["key"] for x in keys if x["key"].lower() == stem), None)
        if k and (k, h["page"]) not in seen:
            seen.add((k, h["page"]))
            out.append((k, h["page"], h["label"], h["text"]))
    return out

def main(a):
    if len(a) < 2:
        print(__doc__); return 1
    cmd, arg = a[0], a[1]
    if cmd == "audit":
        r = call("/api/audit", {"text": open(arg, encoding="utf-8", errors="replace").read()})
        print(r["markdown"]); return 2 if r["problems"] else 0
    if cmd == "ask":
        r = call("/api/ask", {"q": arg}); print(r["answer"]); print(r["references"])
        for w in r["warnings"]: print("WARNING:", w)
        return 0
    if cmd == "coverage":
        print(call("/api/coverage", {"text": open(arg, encoding="utf-8").read()})["markdown"]); return 0
    if cmd == "cite":
        c = cite_candidates(arg)
        if not c:
            print("no passage in the library matches"); return 1
        for k, p, label, text in c:
            print("\\cite[p. %s]{%s}   %% %s: %s" % (p, k, label, text[:80].replace("\n", " ")) if p else "\\cite{%s}   %% %s" % (k, label))
        if "--lyx" in a:
            k, p, *_ = c[0]
            lyx_send("\\cite[p. %s]{%s}" % (p, k) if p else "\\cite{%s}" % k)
        return 0
    print(__doc__); return 1

if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
