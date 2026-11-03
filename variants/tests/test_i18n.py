#!/usr/bin/env python3
"""Interface languages of tezaur.html (ro source + en, fr, it, es, pt, de, hu): static labels, tabs, mode help and placeholders change with the
selector, the choice persists, no page error, no network request, and every language has the same keys as the source. Needs playwright+chromium only."""
import os, re, sys
from playwright.sync_api import sync_playwright
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
LANGS = ["ro", "en", "fr", "it", "es", "pt", "de", "hu"]
FAIL = []
def check(name, cond, extra=""):
    print(("ok   " if cond else "FAIL ") + name + ("" if cond else " " + str(extra)))
    if not cond: FAIL.append(name)
html = open(ROOT + "/tezaur.html", encoding="utf-8").read()
import json
m = re.search(r"const I18N=(\{.*?\});\nconst UI_LANGS", html, re.S)
check("dicționarul I18N există", bool(m))
D = json.loads(m.group(1)) if m else {}
for k, v in D.items():
    miss = [l for l in LANGS[1:] if not str(v.get(l, "")).strip()]
    check("cheie completă: " + k[:40], not miss, miss)
    ph = sorted(re.findall(r"\{\w+\}", k))
    check("substituenți identici: " + k[:40], all(sorted(re.findall(r"\{\w+\}", v[l])) == ph for l in LANGS[1:]), k)
with sync_playwright() as pw:
    b = pw.chromium.launch(); ctx = b.new_context(viewport={"width": 1280, "height": 900}, locale="ro-RO")
    reqs = []; ctx.route(re.compile(r"^https?:"), lambda r: (reqs.append(r.request.url), r.abort()))
    pg = ctx.new_page(); errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.goto("file://" + ROOT + "/tezaur.html"); pg.wait_for_timeout(600)
    check("limba implicită din browser (ro)", pg.input_value("#uiLang") == "ro")
    ro_h = pg.inner_text("aside h2:nth-of-type(1)")
    seen = {ro_h.lower()}
    for l in LANGS[1:]:
        pg.select_option("#uiLang", l); pg.wait_for_timeout(150)
        h = pg.inner_text("aside h2:nth-of-type(1)"); tab = pg.inner_text("#tabs button:nth-child(1)"); help_ = pg.inner_text("#modeHelp")
        ph = pg.get_attribute("#q", "placeholder"); lang_attr = pg.evaluate("document.documentElement.lang")
        check(f"{l}: titlu bibliotecă tradus ({h})", h.lower() != "bibliotecă", h)
        check(f"{l}: html lang", lang_attr == l, lang_attr)
        check(f"{l}: ajutorul modului tradus", help_ and not re.search(r"Răspuns cu citări", help_), help_)
        check(f"{l}: placeholder tradus", ph and ph != "Întrebarea ta…", ph)
        check(f"{l}: fila «Căutare» tradusă", tab != "Căutare", tab)
        seen.add(h.lower())
    check("opt limbi dau titluri distincte (≥6 variante)", len(seen) >= 6, seen)
    pg.select_option("#uiLang", "de"); pg.reload(); pg.wait_for_timeout(500)
    check("alegerea persistă după reîncărcare", pg.input_value("#uiLang") == "de" and pg.evaluate("document.documentElement.lang") == "de")
    pg.select_option("#uiLang", "ro"); pg.wait_for_timeout(150)
    check("revenire la română: textul original", pg.inner_text("aside h2:nth-of-type(1)").lower() == "bibliotecă")
    check("nicio eroare JS", not errs, errs)
    check("nicio cerere de rețea", not reqs, reqs)
    b.close()
print(f"{'toate trec' if not FAIL else str(len(FAIL)) + ' EȘECURI'}")
sys.exit(1 if FAIL else 0)
