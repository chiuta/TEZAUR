#!/usr/bin/env python3
"""Alfred Script Filter: `tz burden of proof` lists matching passages (no model); Enter copies the passage with its citation."""
import json
import os
import sys
import urllib.parse
import urllib.request

q = " ".join(sys.argv[1:]).strip()
server = os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8787").rstrip("/")


def items(q):
    if not q:
        return [{"title": "Type a search…", "valid": False}]
    req = urllib.request.Request(server + "/api/search?" + urllib.parse.urlencode({"q": q, "k": 8}))
    if os.environ.get("TEZAUR_TOKEN"):
        req.add_header("Authorization", "Bearer " + os.environ["TEZAUR_TOKEN"])
    try:
        d = json.load(urllib.request.urlopen(req, timeout=30))
    except Exception as e:  # noqa
        return [{"title": "Cannot reach TEZAUR", "subtitle": str(e), "valid": False}]
    return [{"title": " ".join(h["text"].split())[:90], "subtitle": h["label"], "arg": "%s [%s]" % (" ".join(h["text"].split()), h["label"]),
             "text": {"copy": h["text"]}} for h in d.get("hits", [])] or [{"title": "No matching passages", "valid": False}]


print(json.dumps({"items": items(q)}))
