#!/usr/bin/env python3
"""Build all tutorial videos.   usage: make.py [--bin PATH_TO_tezaur] [--out DIR] [ids...]

Needs: python3 + playwright (chromium), ffmpeg with libx264 and libass, poppler-utils (pdftotext).
Output per tutorial: <id>.<lang>.mp4 (subtitles burned in), <id>.<lang>.srt/.vtt, <id>.clean.mp4 and
<id>.multi.mp4 (clean video + 8 selectable subtitle tracks). Plus index.html (player) and tutorials.json.
"""
import argparse, html, json, os, shutil, subprocess, sys, time
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from engine import LANGS, NAMES, Env, record, build
from tutorials import TUTS

ap = argparse.ArgumentParser()
ap.add_argument("--bin", default=os.path.join(HERE, "..", "..", "target", "release", "tezaur"))
ap.add_argument("--out", default=os.path.join(HERE, "out"))
ap.add_argument("ids", nargs="*")
a = ap.parse_args()
BIN = os.path.abspath(a.bin); OUT = os.path.abspath(a.out)
os.makedirs(OUT, exist_ok=True)

WORK = "/tmp/thesis"
shutil.rmtree(WORK, ignore_errors=True); shutil.copytree(os.path.join(HERE, "demo"), WORK)
binroot = os.path.join(WORK, "_bin"); os.makedirs(binroot); os.symlink(BIN, os.path.join(binroot, "tezaur"))
env = dict(os.environ, PATH=binroot + ":" + os.environ["PATH"], TEZAUR_INDEX=WORK + "/t.idx", TEZAUR_HOST="http://127.0.0.1:11999",
           TEZAUR_MODEL="demo-model", HOME=WORK + "/_home", NO_COLOR="1", TERM="dumb", PS1="$ ")
os.makedirs(WORK + "/_home", exist_ok=True)
sh = Env(WORK, env)
procs = []
def bg(args, e=None):
    procs.append(subprocess.Popen(args, cwd=WORK, env=e or env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
bg([sys.executable, os.path.join(HERE, "..", "..", "variants", "tests", "mock_ollama.py"), "11999"])
web_env = dict(env, TEZAUR_INDEX=WORK + "/web.idx")
Env(WORK, web_env).run("tezaur ingest lib >/dev/null; true")
bg(["tezaur", "serve", "--port", "8851"], web_env)
time.sleep(2)

from playwright.sync_api import sync_playwright
meta = []
try:
    with sync_playwright() as p:
        br = p.chromium.launch()
        for tut in TUTS:
            if a.ids and not any(tut["id"].startswith(i) for i in a.ids): continue
            print("recording", tut["id"], flush=True)
            if tut["kind"] == "term":
                sh.run("rm -rf ana bob out.tex r1.json; tezaur ingest lib >/dev/null 2>&1")
            vid, cues = record(br, tut, OUT, sh, "http://127.0.0.1:8851")
            dur = build(vid, cues, tut["id"], OUT)
            meta.append(dict(id=tut["id"], title=tut["title"], kind=tut["kind"], seconds=round(dur, 1), scenes=len(cues)))
            print("  built", tut["id"], round(dur), "s", flush=True)
        br.close()
finally:
    for pr in procs: pr.kill()
shutil.rmtree(os.path.join(OUT, "_rec"), ignore_errors=True)
json.dump(meta, open(os.path.join(OUT, "tutorials.json"), "w"), ensure_ascii=False, indent=1)

# player page: one <video> with 8 <track>s
rows = []
for m in meta:
    i = m["id"]
    tr = "".join(f'<track kind="subtitles" src="{i}/{i}.{l}.vtt" srclang="{l}" label="{NAMES[l]}"{" default" if l == "en" else ""}>' for l in LANGS)
    t = json.dumps(m["title"], ensure_ascii=False)
    rows.append(f'<section><h2>{html.escape(i)} <small data-t=\'{html.escape(t, quote=True)}\'></small></h2><video controls preload="metadata" src="{i}/{i}.clean.mp4">{tr}</video></section>')
page = """<!doctype html><meta charset="utf-8"><title>TEZAUR video tutorials</title><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{font:16px system-ui;margin:0;background:#f6f4ef;color:#222}main{max-width:900px;margin:auto;padding:16px}video{width:100%;background:#000;border-radius:8px}
h2 small{font-weight:400;color:#555}select{font:inherit;padding:4px}</style><main><h1>TEZAUR — video tutorials</h1>
<p>Language: <select id="l">""" + "".join(f'<option value="{l}">{NAMES[l]}</option>' for l in LANGS) + """</select> (titles and the subtitle track you pick in the player's CC menu)</p>
""" + "\n".join(rows) + """</main><script>
const sel=document.getElementById('l');
function apply(){const l=sel.value;document.querySelectorAll('[data-t]').forEach(e=>e.textContent='— '+JSON.parse(e.dataset.t)[l]);
 document.querySelectorAll('video').forEach(v=>{for(const t of v.textTracks)t.mode=(t.language===l?'showing':'disabled')})}
sel.onchange=apply;document.querySelectorAll('video').forEach(v=>v.addEventListener('loadedmetadata',apply));apply();</script>"""
open(os.path.join(OUT, "index.html"), "w", encoding="utf-8").write(page)
print("done:", len(meta), "tutorials ->", OUT)
