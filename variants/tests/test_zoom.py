#!/usr/bin/env python3
"""tezaur.html: fixed table headers, bottom zoom bar (− % + fit), automatic fit at load, remembered choice, narrow layout on
the zoomed width, translated labels. Needs playwright+chromium only (the page is opened from disk, nothing is served)."""
import os, re, sys
from playwright.sync_api import sync_playwright
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
FAIL = []
def check(name, cond, extra=""):
    print(("ok   " if cond else "FAIL ") + name + ("" if cond else " " + str(extra)))
    if not cond: FAIL.append(name)

TABLE_MD = "\n".join(["| Coloana A | Coloana B | Coloana C |", "|---|---|---|"] + [f"| rând {i} a | rând {i} b | rând {i} c |" for i in range(120)])

def open_page(pw, w, h, storage=None):
    b = pw.chromium.launch(); ctx = b.new_context(viewport={"width": w, "height": h}, locale="ro-RO")
    reqs = []; ctx.route(re.compile(r"^https?:"), lambda r: (reqs.append(r.request.url), r.abort()))
    pg = ctx.new_page(); errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.goto("file://" + ROOT + "/tezaur.html"); pg.wait_for_timeout(400)
    return b, ctx, pg, errs, reqs

def fs(pg): return pg.evaluate("parseFloat(getComputedStyle(document.documentElement).fontSize)")
def pct(pg): return pg.inner_text("#zPct").strip()

with sync_playwright() as pw:
    # ---- automatic fit at load ----
    for (w, h, want_z, narrow) in [(1920, 1080, 1.45, False), (2560, 1440, 1.95, False), (1280, 800, 1.0, False), (3840, 2160, 2.5, False), (390, 844, 1.0, True)]:
        b, ctx, pg, errs, reqs = open_page(pw, w, h)
        z = fs(pg) / 15
        check(f"{w}x{h}: zoom automat {want_z}", abs(z - want_z) < 0.011, z)
        check(f"{w}x{h}: bara arată {round(want_z * 100)}%", pct(pg) == f"{round(want_z * 100)}%", pct(pg))
        check(f"{w}x{h}: layout {'îngust' if narrow else 'lat'}", pg.evaluate("document.documentElement.classList.contains('narrow')") == narrow)
        check(f"{w}x{h}: fără defilare orizontală", pg.evaluate("document.documentElement.scrollWidth <= innerWidth + 1"), pg.evaluate("document.documentElement.scrollWidth"))
        if not narrow:
            right = pg.evaluate("document.querySelector('main').getBoundingClientRect().right")
            check(f"{w}x{h}: conținutul ocupă lățimea ferestrei", right >= w - 2, right)
        bar = pg.evaluate("(()=>{const r=document.getElementById('zbar').getBoundingClientRect();return [r.bottom,r.height,getComputedStyle(document.getElementById('zbar')).position]})()")
        check(f"{w}x{h}: bara de jos fixă, la marginea ferestrei, nezoomată", abs(bar[0] - h) < 1 and bar[1] == 46 and bar[2] == "fixed", bar)
        check(f"{w}x{h}: nicio eroare JS / cerere de rețea", not errs and not reqs, (errs, reqs))
        b.close()

    # ---- buttons, bounds, memory ----
    b, ctx, pg, errs, reqs = open_page(pw, 1280, 800)
    pg.click("#zIn"); check("+ mărește cu 10%", pct(pg) == "110%" and abs(fs(pg) - 16.5) < .01, (pct(pg), fs(pg)))
    pg.click("#zOut"); pg.click("#zOut"); check("− micșorează cu 10%", pct(pg) == "90%", pct(pg))
    pg.click("#zPct"); check("click pe procent: înapoi la 100%", pct(pg) == "100%")
    for _ in range(8): pg.click("#zIn")
    pg.reload(); pg.wait_for_timeout(300)
    check("alegerea manuală persistă după reîncărcare (180%)", pct(pg) == "180%" and abs(fs(pg) - 27) < .01, pct(pg))
    pg.set_viewport_size({"width": 1700, "height": 900}); pg.wait_for_timeout(200)
    check("zoomul manual nu e schimbat de redimensionare", pct(pg) == "180%", pct(pg))
    check("1700/1.8 = 944 px efectivi: încă pe două coloane", not pg.evaluate("document.documentElement.classList.contains('narrow')"))
    pg.set_viewport_size({"width": 1400, "height": 900}); pg.wait_for_timeout(200)
    check("1400/1.8 = 777 px efectivi ≤ 820: layout îngust", pg.evaluate("document.documentElement.classList.contains('narrow')"))
    check("fără defilare orizontală nici la zoom mare", pg.evaluate("document.documentElement.scrollWidth <= innerWidth + 1"), pg.evaluate("document.documentElement.scrollWidth"))
    pg.click("#zFit"); pg.wait_for_timeout(100)
    check("butonul de potrivire revine la automat (1400 px → 105%)", pct(pg) == "105%", pct(pg))
    pg.set_viewport_size({"width": 1920, "height": 1000}); pg.wait_for_timeout(200)
    check("în modul automat, redimensionarea reface potrivirea (1920 → 145%)", pct(pg) == "145%", pct(pg))
    pg.reload(); pg.wait_for_timeout(300)
    check("modul automat persistă (145% la 1920)", pct(pg) == "145%", pct(pg))
    for _ in range(40): pg.click("#zOut") if not pg.is_disabled("#zOut") else None
    check("limita de jos 50%: − dezactivat", pct(pg) == "50%" and pg.is_disabled("#zOut"), pct(pg))
    for _ in range(40): pg.click("#zIn") if not pg.is_disabled("#zIn") else None
    check("limita de sus 300%: + dezactivat", pct(pg) == "300%" and pg.is_disabled("#zIn"), pct(pg))
    check("nicio eroare JS / cerere de rețea", not errs and not reqs, (errs, reqs))
    b.close()

    # ---- fixed table headers ----
    for (w, h, clicks) in [(1280, 800, 0), (1920, 1080, 0), (1280, 800, 6)]:
        b, ctx, pg, errs, reqs = open_page(pw, w, h)
        for _ in range(clicks): pg.click("#zIn")
        pg.evaluate("md=>{const c=document.createElement('div');c.className='card';c.id='tbl';c.innerHTML=rich(md);document.querySelector('main').appendChild(c)}", TABLE_MD)
        info = pg.evaluate("(()=>{const t=document.querySelector('#tbl table');return {thead:!!t.querySelector('thead th'),ths:t.querySelectorAll('thead th').length,bodyRows:t.querySelectorAll('tbody tr').length,pos:getComputedStyle(t.querySelector('th')).position,bg:getComputedStyle(t.querySelector('th')).backgroundColor}})()")
        tag = f"{w}x{h} zoom+{clicks}"
        check(f"{tag}: antet în <thead>, 120 rânduri în <tbody>", info["thead"] and info["ths"] == 3 and info["bodyRows"] == 120, info)
        check(f"{tag}: antetul e sticky și opac", info["pos"] == "sticky" and not info["bg"].endswith(", 0)") and "rgba" not in info["bg"].replace("rgba(0, 0, 0, 0)", "X") or info["bg"].count(",") == 2, info)
        pg.evaluate("window.scrollTo(0, document.querySelector('#tbl table tbody tr:nth-child(60)').getBoundingClientRect().top + scrollY - 200)")
        pg.wait_for_timeout(120)
        tops = pg.evaluate("[...document.querySelectorAll('#tbl thead th')].map(e=>Math.round(e.getBoundingClientRect().top))")
        check(f"{tag}: după derulare, antetul rămâne lipit sus (top 0, cu 2 px de acoperire a marginii)", all(-2 <= t <= 0 for t in tops), tops)
        vis = pg.evaluate("(()=>{const r=document.querySelector('#tbl thead th').getBoundingClientRect();const e=document.elementFromPoint(r.left+r.width/2,r.top+r.height/2);return e&&e.closest('th')?e.closest('th').textContent:null})()")
        check(f"{tag}: antetul e în față (nu acoperit de rânduri)", vis in ("Coloana A", "Coloana B", "Coloana C"), vis)
        pg.evaluate("window.scrollTo(0, 0)"); pg.wait_for_timeout(80)
        check(f"{tag}: sus, antetul e la locul lui în tabel", pg.evaluate("document.querySelector('#tbl thead th').getBoundingClientRect().top > 50"))
        check(f"{tag}: nicio eroare JS", not errs, errs)
        b.close()

    # ---- translated labels ----
    b, ctx, pg, errs, reqs = open_page(pw, 1280, 800)
    ro = [pg.get_attribute(i, "title") for i in ("#zOut", "#zIn", "#zFit", "#zPct")]
    check("română: etichete", ro == ["Micșorează", "Mărește", "Potrivește la ecran", "Revino la 100%"], ro)
    pg.select_option("#uiLang", "de"); pg.wait_for_timeout(150)
    de = [pg.get_attribute(i, "title") for i in ("#zOut", "#zIn", "#zFit", "#zPct")]
    de_al = [pg.get_attribute(i, "aria-label") for i in ("#zOut", "#zIn", "#zFit", "#zPct")]
    check("germană: titluri traduse", de == ["Verkleinern", "Vergrößern", "An den Bildschirm anpassen", "Zurück auf 100 %"], de)
    check("germană: aria-label tradus", de_al == de, de_al)
    check("bara păstrează procentul după schimbarea limbii", pct(pg) == "100%", pct(pg))
    b.close()

print(f"{'toate trec' if not FAIL else str(len(FAIL)) + ' EȘECURI'}")
sys.exit(1 if FAIL else 0)
