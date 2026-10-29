#!/usr/bin/env python3
"""Short demo videos (ro + en subtitles) for the defects found by the live-site audit.
Commands are really executed; lines marked EVIDENCE replay what the CI audit run printed (the sandbox cannot reach the live domain).
usage: review_videos.py --old-bin PATH --new-bin PATH --repo PATH_TO_CLONE_WITH_PREVIOUS_COMMIT --out DIR
"""
import argparse, os, shutil, sys, json
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "tutorials"))
import engine
engine.LANGS = ["ro", "en"]
from engine import Env, Scene, record, build
def L(en, ro): return dict(en=en, ro=ro)

ap = argparse.ArgumentParser(); ap.add_argument("--old-bin"); ap.add_argument("--new-bin"); ap.add_argument("--repo"); ap.add_argument("--out", default="out")
a = ap.parse_args(); OUT = os.path.abspath(a.out)
WORK = "/tmp/reviewwork"; shutil.rmtree(WORK, ignore_errors=True); shutil.copytree(os.path.join(HERE, "review"), WORK)
os.makedirs(WORK + "/_bin"); shutil.copy(a.new_bin, WORK + "/_bin/tezaur"); shutil.copy(a.old_bin, WORK + "/_bin/tezaur-v040")
env = dict(os.environ, PATH=WORK + "/_bin:" + os.environ["PATH"], TEZAUR_INDEX=WORK + "/t.idx", HOME=WORK, NO_COLOR="1", TERM="dumb")
sh = Env(WORK, env); sh.run("tezaur ingest lib >/dev/null 2>&1")
R = a.repo
EV = "CI audit run 40955915164 (job http), EVIDENCE"
HDR = """GET https://tezaur.chiuta.ro/   ->  200 OK, 2.81 MB gzip, 0.21 s
header-missing  content-security-policy    absent
header-missing  x-frame-options            absent
header-missing  x-content-type-options     absent
header-missing  referrer-policy            absent
header-missing  permissions-policy         absent
header-missing  strict-transport-security  absent"""
DNS = """dns   tezaur.chiuta.ro      -> 185.199.108.153 185.199.110.153 185.199.111.153 185.199.109.153
dns   www.tezaur.chiuta.ro  -> [Errno -2] Name or service not known
www   www subdomain answers 0"""
WK = """webkit:track-switch  ['fr', 'fr']        <- two 'fr' tracks for one choice
webkit:video-12  tracks: ro en(showing) fr it es pt de hu  +  en(showing) ro fr it es pt de hu
chromium:track-switch ['fr']   firefox:track-switch ['fr']"""
V = []
def vid(i, title, scenes): V.append(dict(id=i, title=title, kind="term", scenes=scenes))

vid("R1-numere-cifre-vs-cuvinte", "Number written as digits is flagged although the source says it in words", [
  Scene(L("Source: 'four months', 'three years'. Draft: '4 months', '3 years', and one real error: '5 years'.", "Sursa: 'four months', 'three years'. Ciorna: '4 months', '3 years' și o greșeală reală: '5 years'."),
        lambda c: (c.cmd("cat lib/period.txt work/draft.md", n=12))),
  Scene(L("v0.4.0: the two CORRECT citations are flagged number-mismatch. A tool that cries wolf trains users to ignore it.", "v0.4.0: cele două citări CORECTE sunt marcate number-mismatch. Un instrument care dă alarme false învață utilizatorul să le ignore."),
        lambda c: c.cmd("tezaur-v040 audit work/draft.md --bib lib/refs.bib", n=16)),
  Scene(L("Fixed in this commit: figures are compared by value (4 = four = patru). Only '5 years' is flagged now.", "Reparat în acest commit: cifrele se compară ca valoare (4 = four = patru). Acum doar '5 years' e marcat."),
        lambda c: c.cmd("tezaur audit work/draft.md --bib lib/refs.bib", n=16)),
])
vid("R2-antete-securitate", "No security headers on the live site", [
  Scene(L("Live site, checked from GitHub Actions. All six hardening headers are absent. " + EV, "Site-ul live, verificat din GitHub Actions. Toate cele șase antete de securitate lipsesc. " + EV),
        lambda c: (c.cmd("python3 scripts/audit/live_audit.py   # job 'http'", out=HDR, n=12))),
  Scene(L("GitHub Pages cannot set response headers. Anyone can frame the page; there is no CSP. Only a meta referrer was added.", "GitHub Pages nu permite antete proprii. Pagina poate fi pusă în iframe de oricine; nu există CSP. Am adăugat doar meta referrer."),
        lambda c: c.wait(500)),
])
vid("R3-www-si-metadate", "www does not resolve; no canonical/og/favicon", [
  Scene(L("www.tezaur.chiuta.ro does not resolve at all. " + EV, "www.tezaur.chiuta.ro nu se rezolvă deloc. " + EV),
        lambda c: (c.cmd("python3 scripts/audit/live_audit.py   # DNS part", out=DNS, n=8))),
  Scene(L("The deck before the fix had no canonical, no Open Graph, no favicon: a shared link shows a blank card.", "Prezentarea dinainte de reparație nu avea canonical, Open Graph, favicon: un link distribuit arată un card gol."),
        lambda c: c.cmd(f"git -C {R} show HEAD~1:index.html | grep -cE 'og:title|rel=\"canonical\"|rel=\"icon\"'", shown="git show <înainte>:index.html | grep -cE 'og:title|canonical|icon'", n=4)),
  Scene(L("After the fix: present.", "După reparație: prezente."),
        lambda c: c.cmd(f"grep -cE 'og:title|rel=\"canonical\"|rel=\"icon\"' {R}/index.html", shown="grep -cE 'og:title|canonical|icon' index.html", n=4)),
])
vid("R4-safari-subtitrari-duble", "Safari showed every caption twice", [
  Scene(L("In WebKit the page has two 'fr' tracks for one choice: the mp4 carried its own embedded tracks and the page also loaded .vtt files. " + EV, "În WebKit pagina are două piste 'fr' pentru o alegere: mp4-ul avea piste încorporate, iar pagina încărca și fișiere .vtt. " + EV),
        lambda c: (c.cmd("python3 scripts/audit/live_browsers.py   # webkit", out=WK, n=6))),
  Scene(L("Cause, reproduced: the published mp4 contained 8 subtitle streams.", "Cauza, reprodusă: mp4-ul publicat conținea 8 fluxuri de subtitrare."),
        lambda c: c.cmd(f"git -C {R} show HEAD~1:assets/tutorials/01-first-question.mp4 > /tmp/old.mp4; ffprobe -v error -show_entries stream=codec_type -of csv=p=0 /tmp/old.mp4 | sort | uniq -c", shown="ffprobe old.mp4 | uniq -c", n=4)),
  Scene(L("Fixed: the published mp4 now has video only.", "Reparat: mp4-ul publicat are acum doar video."),
        lambda c: c.cmd(f"ffprobe -v error -show_entries stream=codec_type -of csv=p=0 {R}/assets/tutorials/01-first-question.mp4 | sort | uniq -c", shown="ffprobe new.mp4 | uniq -c", n=4)),
])
vid("R5-contor-16-vs-20", "Counter said 01 / 16 but the deck has 20 slides", [
  Scene(L("Before JavaScript runs (or with it disabled) the deck says '01 / 16'. It has 20 slides.", "Înainte să ruleze JavaScript (sau cu el oprit) prezentarea spune '01 / 16'. Are 20 de slide-uri."),
        lambda c: (c.cmd(f"git -C {R} show HEAD~1:index.html | grep -o 'aria-live=\"polite\">[^<]*'", shown="git show <înainte>:index.html | grep -o 'aria-live=...'", n=3),
                   c.cmd(f"git -C {R} show HEAD~1:index.html | grep -c '<section class=\"slide\"'", shown="grep -c '<section class=\"slide\"'", n=3))),
  Scene(L("Fixed to 01 / 20 (the script still updates it at load).", "Reparat la 01 / 20 (scriptul îl actualizează oricum la încărcare)."),
        lambda c: c.cmd(f"grep -o 'aria-live=\"polite\">[^<]*' {R}/index.html", shown="grep -o 'aria-live=...' index.html", n=3)),
])

if __name__ == "__main__":
    from playwright.sync_api import sync_playwright
    os.makedirs(OUT, exist_ok=True); meta = []
    with sync_playwright() as p:
        br = p.chromium.launch()
        for t in V:
            print("recording", t["id"], flush=True)
            vd, cues = record(br, t, OUT, sh, "")
            dur = build(vd, cues, t["id"], OUT); meta.append(dict(id=t["id"], title=t["title"], seconds=round(dur, 1)))
        br.close()
    shutil.rmtree(os.path.join(OUT, "_rec"), ignore_errors=True)
    json.dump(meta, open(os.path.join(OUT, "videos.json"), "w"), indent=1)
    print("done", len(meta))
