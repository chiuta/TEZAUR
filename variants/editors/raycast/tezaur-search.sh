#!/bin/bash
# Required parameters:
# @raycast.schemaVersion 1
# @raycast.title TEZAUR: search passages
# @raycast.mode fullOutput
# @raycast.packageName TEZAUR
# Optional parameters:
# @raycast.icon 🔎
# @raycast.argument1 { "type": "text", "placeholder": "Terms" }
# Documentation:
# @raycast.description Matching passages from your library, no model needed
SERVER="${TEZAUR_SERVER:-http://127.0.0.1:8787}"
AUTH=(); [ -n "$TEZAUR_TOKEN" ] && AUTH=(-H "Authorization: Bearer $TEZAUR_TOKEN")
curl -s -G "${AUTH[@]}" "$SERVER/api/search" --data-urlencode "q=$1" |
python3 -c '
import sys, json
d = json.load(sys.stdin)
if "error" in d: print("Error:", d["error"]); sys.exit(1)
for h in d["hits"]: print("[%d] %s\n    %s" % (h["n"], h["label"], " ".join(h["text"][:300].split())))'
