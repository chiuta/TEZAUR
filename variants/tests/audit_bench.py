#!/usr/bin/env python3
"""Larger, synthetic benchmark for `tezaur audit`: 24 invented passages x 7 kinds of claim = 168 checks, with precision/recall.
It measures what the *mechanical* audit can and cannot see on template-built data; it is not a general accuracy figure for real theses
(real text is messier). Positive class = "should be flagged" (anything the audit does not call `supported`).
usage: audit_bench.py [path/to/tezaur] [--json out.json] [--md out.md] [--check]
--check exits 1 if a measured rate falls below the floors recorded in FLOORS (a regression guard, not a quality claim)."""
import json, os, subprocess, sys, tempfile, argparse
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from audit_bench_data import ITEMS, UNRELATED
ap = argparse.ArgumentParser(); ap.add_argument("bin", nargs="?"); ap.add_argument("--json"); ap.add_argument("--md"); ap.add_argument("--check", action="store_true")
a = ap.parse_args()
BIN = a.bin or os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "target", "release", "tezaur")
T = tempfile.mkdtemp(); os.makedirs(T + "/lib")
for it in ITEMS: open(f"{T}/lib/{it['key']}.txt", "w").write(it["passage"] + "\n")
env = dict(os.environ, TEZAUR_INDEX=T + "/lib/t.idx", NO_COLOR="1")
subprocess.run([BIN, "ingest", "."], cwd=T + "/lib", env=env, capture_output=True, check=True)
cases = []  # (kind, cited key, sentence, should_flag)
for i, it in enumerate(ITEMS):
    nxt = ITEMS[(i + 1) % len(ITEMS)]
    cases += [("verbatim", it["key"], it["verbatim"], False), ("paraphrase", it["key"], it["paraphrase"], None), ("figure-form", it["key"], it["form"], False),
              (("figure-swap" if any(c.isdigit() for c in it["swap"]) or any(w in it["swap"].lower().split() for w in ("nine","seventeen","eighteen")) else "word-swap"), it["key"], it["swap"], True), ("negation", it["key"], it["neg"], True),
              ("wrong-source", it["key"], nxt["verbatim"], True), ("unrelated", it["key"], UNRELATED[i % len(UNRELATED)], True)]
res = {}
for kind, key, s, should in cases:
    open(T + "/d.md", "w").write(f"{s} [@{key}, p. 1]\n")
    p = subprocess.run([BIN, "audit", T + "/d.md", "--json"], env=env, capture_output=True, text=True)
    out = json.loads(p.stdout) if p.stdout.strip() else []
    flagged = any(f["verdict"] != "supported" for f in out)
    res.setdefault(kind, []).append(dict(ok=(flagged == should) if should is not None else flagged, flagged=flagged, should=should, verdict=out[0]["verdict"] if out else "none", s=s))
INFO = {"paraphrase"}   # reported, not scored: a lexical audit cannot tell a good paraphrase from a bad one, so it sends it to a human by design
res_s = {k: l for k, l in res.items() if k not in INFO}
tp = sum(1 for l in res_s.values() for r in l if r["should"] and r["flagged"]); fn = sum(1 for l in res_s.values() for r in l if r["should"] and not r["flagged"])
fp = sum(1 for l in res_s.values() for r in l if not r["should"] and r["flagged"]); tn = sum(1 for l in res_s.values() for r in l if not r["should"] and not r["flagged"])
prec = tp / (tp + fp) if tp + fp else 0; rec = tp / (tp + fn) if tp + fn else 0
rows = []
for kind, l in res.items():
    n = len(l); good = sum(r["ok"] for r in l)
    rows.append((kind, "sent to a human (not scored)" if kind in INFO else ("should be flagged" if l[0]["should"] else "should be accepted"), good, n))
md = ["| Claim kind | Expected | Correct | Rate |", "|---|---|---|---|"] + [f"| {k} | {e} | {g}/{n} | {100*g/n:.0f} % |" for k, e, g, n in rows]
md += ["", f"Overall: flagged-when-it-should-be recall {100*rec:.0f} % ({tp}/{tp+fn}); precision {100*prec:.0f} % ({tp}/{tp+fp}); false alarms on correct claims {fp}/{fp+tn}."]
print("\n".join(md))
for kind, l in res.items():
    for r in l:
        if not r["ok"] and os.environ.get("VERBOSE"): print(f"  MISS {kind}: [{r['verdict']}] {r['s']}")
if a.json: json.dump(dict(rows=[dict(kind=k, expected=e, correct=g, total=n) for k, e, g, n in rows], recall=rec, precision=prec, tp=tp, fp=fp, fn=fn, tn=tn, n=len(cases)), open(a.json, "w"), indent=1)
if a.md: open(a.md, "w").write("\n".join(md) + "\n")
FLOORS = {"verbatim": 0.95, "figure-form": 0.85, "figure-swap": 0.95, "wrong-source": 0.95, "unrelated": 0.95}   # regression guards from the first measured run
if a.check:
    bad = [(k, g / n) for k, e, g, n in rows if k in FLOORS and g / n < FLOORS[k]]
    if bad: print("BELOW FLOOR:", bad); sys.exit(1)
