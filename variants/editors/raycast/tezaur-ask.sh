#!/bin/bash
# Required parameters:
# @raycast.schemaVersion 1
# @raycast.title TEZAUR: ask
# @raycast.mode fullOutput
# @raycast.packageName TEZAUR
# Optional parameters:
# @raycast.icon 📚
# @raycast.argument1 { "type": "text", "placeholder": "Question" }
# Documentation:
# @raycast.description Cited answer from your own library (needs `tezaur serve`; TEZAUR_SERVER / TEZAUR_TOKEN to override)
SERVER="${TEZAUR_SERVER:-http://127.0.0.1:8787}"
AUTH=(); [ -n "$TEZAUR_TOKEN" ] && AUTH=(-H "Authorization: Bearer $TEZAUR_TOKEN")
curl -s --max-time 600 -G "${AUTH[@]}" "$SERVER/api/ask" --data-urlencode "q=$1" --data-urlencode "mode=${TEZAUR_MODE:-ask}" |
python3 -c '
import sys, json
d = json.load(sys.stdin)
if "error" in d: print("Error:", d["error"]); sys.exit(1)
print(d["answer"])
if d.get("references"): print("\n## References\n" + d["references"])
for w in d.get("warnings", []): print("⚠", w)'
