#!/usr/bin/env python3
"""Browser test of the Audit / References panel in tezaur.html served by a real `tezaur serve` (mock judge model). Needs playwright."""
import os, subprocess, sys, tempfile, time
from playwright.sync_api import sync_playwright

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.environ.get("TEZAUR_BIN") or os.path.join(HERE, "..", "..", "target", "release", "tezaur")
ok = True
def check(name, cond, extra=""):
    global ok
    print(("PASS " if cond else "FAIL ") + name + (" — " + str(extra)[:300] if not cond else "")); ok = ok and bool(cond)

d = tempfile.mkdtemp(); lib = os.path.join(d, "lib"); os.makedirs(lib)
open(f"{lib}/lisbon_convention.txt", "w").write("The Lisbon Recognition Convention requires parties to recognise qualifications unless a substantial difference is shown. The burden of proof lies with the body assessing the qualification.")
open(f"{lib}/refs.bib", "w").write("@article{lisbon_convention, author={Council of Europe}, title={Convention}, year={1997}}\n@book{unused_book, author={X, Y}, title={Never cited}, year={2001}}\n")
env = dict(os.environ, TEZAUR_INDEX=f"{lib}/t.idx", TEZAUR_HOST="http://127.0.0.1:11998", TEZAUR_MODEL="m")
subprocess.run([BIN, "ingest", "."], cwd=lib, env=env, capture_output=True, stdin=subprocess.DEVNULL)
procs = [subprocess.Popen([sys.executable, os.path.join(HERE, "mock_judge.py"), "11998"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL),
         subprocess.Popen([BIN, "serve", "--port", "8815"], cwd=lib, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)]
try:
    time.sleep(1.5)
    with sync_playwright() as pw:
        b = pw.chromium.launch(); pg = b.new_page(); errs = []
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto("http://127.0.0.1:8815/"); pg.wait_for_timeout(1200)
        pg.click("#tabs button[data-m=audit]")
        check("semantic checkbox appears only in audit mode", pg.is_visible("#fSem"))
        pg.fill("#q", "The burden of proof does not lie with the body assessing the qualification [@lisbon_convention]. Ghost claim [@nobody].")
        pg.click("#run"); pg.wait_for_selector("#out h3, #out p, #out li", timeout=15000); pg.wait_for_timeout(500)
        t = pg.inner_text("#out")
        check("audit panel reports unknown source", "unknown-source" in t, t)
        check("without semantic the negation passes (known weakness)", "contradicted 0" in t, t)
        pg.check("#sem"); pg.click("#run"); pg.wait_for_function("document.querySelector('#out').innerText.includes('contradicted')", timeout=20000)
        check("semantic checkbox catches the negation", True)
        check("honesty banner shown", "nu o dovad" in pg.inner_text("#warns"), pg.inner_text("#warns"))
        pg.click("#tabs button[data-m=refs]"); check("semantic hidden in refs mode", not pg.is_visible("#fSem"))
        pg.fill("#q", "Only this [@lisbon_convention]."); pg.click("#run"); pg.wait_for_function("document.querySelector('#out').innerText.includes('unused_book')", timeout=15000)
        check("refs panel lists the uncited .bib entry", True)
        check("no JS errors", not errs, errs)
        b.close()
finally:
    for p in procs: p.kill()
sys.exit(0 if ok else 1)
