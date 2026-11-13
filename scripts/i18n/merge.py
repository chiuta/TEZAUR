#!/usr/bin/env python3
"""Merge translation batches (scripts/i18n/batch*.py, dict name B<n>) into the I18N table of tezaur.html.
Every entry needs all 7 languages; {placeholders} must be the same set in every language. Existing keys are kept."""
import glob, importlib.util, json, os, re, sys
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
LANGS = ["en", "fr", "it", "es", "pt", "de", "hu"]
PH = re.compile(r"\{(\w+)\}")
path = os.path.join(ROOT, "tezaur.html")
lines = open(path, encoding="utf-8").read().split("\n")
i = next(n for n, l in enumerate(lines) if l.startswith("const I18N="))
table = json.loads(re.match(r"const I18N=(\{.*\});?$", lines[i]).group(1))
added = 0
for f in sorted(glob.glob(os.path.join(ROOT, "scripts", "i18n", "batch*.py"))):
    spec = importlib.util.spec_from_file_location("b", f); m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
    for name in [n for n in dir(m) if re.fullmatch(r"B\d+", n)]:
        for ro, tr in getattr(m, name).items():
            if len(tr) != 7: sys.exit(f"{f}: {ro[:40]!r}: {len(tr)} translations, need 7")
            want = set(PH.findall(ro))
            for l, t in zip(LANGS, tr):
                if set(PH.findall(t)) != want: sys.exit(f"{f}: {ro[:40]!r} [{l}]: placeholders {set(PH.findall(t))} != {want}")
            if ro not in table: table[ro] = dict(zip(LANGS, tr)); added += 1
lines[i] = "const I18N=" + json.dumps(table, ensure_ascii=False, separators=(",", ":")) + ";"
open(path, "w", encoding="utf-8").write("\n".join(lines))
print(f"{added} intrări noi, {len(table)} în total")
