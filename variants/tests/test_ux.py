#!/usr/bin/env python3
"""tezaur.html, user experience: no blocking alert(), toasts with undo, first-steps card, drag-and-drop, copy feedback,
remembered draft, skip link, Esc stops a run, narrow layout order, translated new strings. Playwright + chromium only."""
import os, re, sys, tempfile, json
from playwright.sync_api import sync_playwright
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
FAIL = []
def check(name, cond, extra=""):
    print(("ok   " if cond else "FAIL ") + name + ("" if cond else " " + str(extra)))
    if not cond: FAIL.append(name)

tmp = tempfile.mkdtemp()
def mk(name, text):
    p = os.path.join(tmp, name); open(p, "w").write(text); return p
SLEEP = mk("sleep.txt", ("Sleep consolidates memory. During slow-wave sleep the hippocampus replays recent experiences. " * 12 + "\n\n") * 4)
EXER = mk("exercise.md", "# Exercise\n\n" + ("Regular exercise raises BDNF and supports learning and memory in adults. " * 12 + "\n\n") * 4)
PNG = mk("pic.png", "not really a png")

def open_page(pw, w=1280, h=800, route=None):
    b = pw.chromium.launch(); ctx = b.new_context(viewport={"width": w, "height": h}, locale="ro-RO", permissions=["clipboard-read", "clipboard-write"])
    reqs = []
    def handler(r):
        if route and route(r): return
        reqs.append(r.request.url); r.abort()
    ctx.route(re.compile(r"^https?:"), handler)
    pg = ctx.new_page(); errs = []; dialogs = []
    pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.on("dialog", lambda d: (dialogs.append(d.message), d.dismiss()))
    pg.goto("file://" + ROOT + "/tezaur.html"); pg.wait_for_timeout(400)
    return b, ctx, pg, errs, reqs, dialogs

def toasts(pg): return pg.eval_on_selector_all("#toasts .toast", "e=>e.map(x=>x.innerText)")
def ndocs(pg): return len(pg.query_selector_all("#docs li"))

with sync_playwright() as pw:
    b, ctx, pg, errs, reqs, dialogs = open_page(pw)
    # ---- first steps card, skip link ----
    check("card „Primii pași” vizibil la început", pg.is_visible("#onb"))
    check("niciun pas bifat", pg.eval_on_selector_all("#onb li.done", "e=>e.length") == 0)
    pg.keyboard.press("Tab")
    check("prima tastă Tab ajunge la linkul „Sari la conținut”, vizibil", pg.evaluate("document.activeElement.className") == "skip" and pg.evaluate("document.activeElement.getBoundingClientRect().top") >= 0)
    # ---- no blocking dialogs: empty question, empty library ----
    pg.click("#run"); pg.wait_for_timeout(200)
    t = toasts(pg)
    check("bibliotecă goală: mesaj non-blocant (toast), fără alert()", len(t) == 1 and "documente" in t[0] and not dialogs, (t, dialogs))
    check("focus mutat pe butonul „+ Fișiere”", pg.evaluate("document.activeElement.id") == "addFiles")
    # ---- add documents: unsupported file is reported ----
    pg.set_input_files("#fFiles", [SLEEP, EXER, PNG]); pg.wait_for_timeout(1200)
    check("2 documente adăugate", ndocs(pg) == 2, ndocs(pg))
    t = " | ".join(toasts(pg))
    check("toast: 2 adăugate, 1 ignorat", "2 documente adăugate" in t and "1 fișiere ignorate" in t, t)
    check("pasul 1 bifat", pg.evaluate("document.getElementById('onb1').classList.contains('done')"))
    pg.click("#run")  # empty question
    check("întrebare goală: toast + focus pe câmp", any("întrebarea" in x for x in toasts(pg)) and pg.evaluate("document.activeElement.id") == "q", toasts(pg))
    # ---- undo: remove one doc, clear all ----
    pg.click("#docs li:first-child .x"); pg.wait_for_timeout(200)
    check("ștergere: 1 document rămâne", ndocs(pg) == 1)
    pg.click("#toasts .toast button"); pg.wait_for_timeout(500)
    check("Anulează readuce documentul", ndocs(pg) == 2, ndocs(pg))
    pg.click("#clearAll"); pg.wait_for_timeout(300)
    check("„Șterge tot” fără confirm(): biblioteca goală, cu Anulează", ndocs(pg) == 0 and not dialogs and len(pg.query_selector_all("#toasts .toast button")) == 1, dialogs)
    pg.click("#toasts .toast button"); pg.wait_for_timeout(500)
    check("Anulează readuce toată biblioteca", ndocs(pg) == 2)
    pg.set_input_files("#fFiles", [PNG]); pg.wait_for_timeout(300)
    check("doar format neacceptat: toast de eroare", any("format acceptat" in x for x in toasts(pg)), toasts(pg))
    # ---- search + copy feedback + scroll ----
    pg.click("#tabs button[data-m=search]"); pg.fill("#q", "hippocampus memory"); pg.click("#run"); pg.wait_for_function("!document.getElementById('run').disabled"); pg.wait_for_timeout(300)
    check("căutare: rezultate afișate", pg.is_visible("#resCard") and len(pg.inner_text("#src")) > 20)
    pg.click("#copy"); pg.wait_for_timeout(200)
    check("copiere: butonul arată „Copiat ✓”", pg.inner_text("#copy").strip() == "Copiat ✓", pg.inner_text("#copy"))
    check("copiere: anunț pentru cititoare de ecran", "clipboard" in pg.inner_text("#announce"), pg.inner_text("#announce"))
    clip = pg.evaluate("navigator.clipboard.readText()")
    check("clipboard conține textul rezultatului", len(clip) > 5, clip)
    pg.wait_for_timeout(1800)
    check("eticheta revine la „Copiază”", pg.inner_text("#copy").strip() == "Copiază", pg.inner_text("#copy"))
    pg.click("#dlMd")
    check("descărcare: toast cu numele fișierului", any("tezaur-search.md" in x for x in toasts(pg)), toasts(pg))
    # ---- draft persistence ----
    pg.click("#tabs button[data-m=ask]"); pg.fill("#q", "Ce spune literatura despre somn?"); pg.fill("#k", "5"); pg.wait_for_timeout(500)
    pg.reload(); pg.wait_for_timeout(500)
    check("ciorna persistă: întrebare, k, mod", pg.input_value("#q") == "Ce spune literatura despre somn?" and pg.input_value("#k") == "5" and pg.evaluate("document.querySelector('#tabs .on').dataset.m") == "ask")
    check("fără alert() și fără erori JS în tot parcursul", not dialogs and not errs, (dialogs, errs))
    check("nicio cerere de rețea", not reqs, reqs)
    # ---- drag and drop ----
    pg.evaluate("document.getElementById('clearAll').click()"); pg.wait_for_timeout(300)
    pg.evaluate("""()=>{const dt=new DataTransfer();dt.items.add(new File(['Dropped text about sleep and memory. '.repeat(40)],'dropped.txt',{type:'text/plain'}));
      window.__dt=dt;window.dispatchEvent(new DragEvent('dragenter',{dataTransfer:dt,bubbles:true,cancelable:true}))}""")
    check("tragere: suprapunerea apare", pg.evaluate("document.body.classList.contains('dragging')") and pg.is_visible("#drop"))
    pg.evaluate("window.dispatchEvent(new DragEvent('drop',{dataTransfer:window.__dt,bubbles:true,cancelable:true}))"); pg.wait_for_timeout(900)
    check("eliberare: fișierul e în bibliotecă, suprapunerea dispare", ndocs(pg) == 1 and not pg.evaluate("document.body.classList.contains('dragging')"), ndocs(pg))
    pg.evaluate("window.dispatchEvent(new DragEvent('dragenter',{dataTransfer:(()=>{const d=new DataTransfer();d.setData('text/plain','x');return d})(),bubbles:true,cancelable:true}))")
    check("tragerea de text nu declanșează suprapunerea", not pg.evaluate("document.body.classList.contains('dragging')"))
    b.close()

    # ---- Esc stops a running request ----
    def route(r):
        u = r.request.url
        if u.startswith("http://localhost:11434/api/tags"):
            r.fulfill(status=200, headers={"access-control-allow-origin": "*", "content-type": "application/json"}, body=json.dumps({"models": [{"name": "m1"}]})); return True
        if u.startswith("http://localhost:11434/api/chat"):
            return True  # never answered: the run stays busy until it is aborted
        return False
    b, ctx, pg, errs, reqs, dialogs = open_page(pw, route=route)
    pg.set_input_files("#fFiles", [SLEEP]); pg.wait_for_timeout(900)
    pg.fill("#model", "m1"); pg.fill("#q", "sleep and memory"); pg.click("#run"); pg.wait_for_timeout(400)
    check("în lucru: Oprește activ", not pg.is_disabled("#stop") and pg.is_disabled("#run"))
    pg.keyboard.press("Escape"); pg.wait_for_timeout(400)
    check("Esc oprește rularea", pg.is_disabled("#stop") and not pg.is_disabled("#run"), (pg.is_disabled("#stop"), pg.is_disabled("#run")))
    check("mesaj „oprit de utilizator”", "oprit" in pg.inner_text("#warns"), pg.inner_text("#warns"))
    check("pasul 2 și 3 bifate după model + prima rulare", pg.evaluate("document.getElementById('onb2').classList.contains('done')") and pg.evaluate("document.getElementById('onb3').classList.contains('done')"))
    b.close()

    # ---- narrow layout: content before the library ----
    b, ctx, pg, errs, reqs, dialogs = open_page(pw, 390, 844)
    tops = pg.evaluate("[document.querySelector('main').getBoundingClientRect().top, document.querySelector('aside').getBoundingClientRect().top]")
    check("pe ecran îngust, zona de lucru e înaintea barei laterale", tops[0] < tops[1], tops)
    check("pe ecran îngust, cardul de pași are buton „+ Fișiere”", pg.is_visible("#onbAdd"))
    check("fără defilare orizontală", pg.evaluate("document.documentElement.scrollWidth <= innerWidth + 1"))
    b.close()

    # ---- translations ----
    b, ctx, pg, errs, reqs, dialogs = open_page(pw)
    pg.select_option("#uiLang", "en"); pg.wait_for_timeout(200)
    check("engleză: card și linkuri traduse", pg.inner_text("#onb h2").lower() == "first steps" and pg.inner_text(".skip") == "Skip to content", (pg.inner_text("#onb h2"), pg.inner_text(".skip")))
    pg.click("#run"); pg.wait_for_timeout(150)
    check("engleză: toast tradus", any("First add documents" in x for x in toasts(pg)), toasts(pg))
    pg.set_input_files("#fFiles", [SLEEP]); pg.wait_for_timeout(900)
    pg.click("#docs li:first-child .x"); pg.wait_for_timeout(150)
    check("engleză: toast cu Undo", any("was removed" in x and "Undo" in x for x in toasts(pg)), toasts(pg))
    pg.select_option("#uiLang", "hu"); pg.wait_for_timeout(200)
    check("maghiară: card tradus", pg.inner_text("#onb h2").lower() == "első lépések", pg.inner_text("#onb h2"))
    b.close()

print(f"{'toate trec' if not FAIL else str(len(FAIL)) + ' EȘECURI'}")
sys.exit(1 if FAIL else 0)
