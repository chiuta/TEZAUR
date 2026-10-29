"""Tutorial video engine: records real UI / real command output with Playwright,
then builds per-language subtitled mp4 (burned-in), SRT/VTT, and one multi-track mp4.

No voice-over: the narration is the subtitles. Terminal output is produced by really
running the commands; the web UI is the real `tezaur serve` page.
"""
import html, json, os, shutil, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor

LANGS = ["en", "ro", "fr", "it", "es", "pt", "de", "hu"]
ISO3 = dict(en="eng", ro="ron", fr="fra", it="ita", es="spa", pt="por", de="deu", hu="hun")
NAMES = dict(en="English", ro="Română", fr="Français", it="Italiano", es="Español", pt="Português", de="Deutsch", hu="Magyar")
W, H, OUTH = 1280, 640, 768        # page viewport; the video is padded to 1280x768 and subtitles live in the bottom bar
BG = "#101418"
CPS = 15.0                          # reading speed used to hold a scene long enough (chars/second)

def L(en, ro, fr, it, es, pt, de, hu):
    return dict(en=en, ro=ro, fr=fr, it=it, es=es, pt=pt, de=de, hu=hu)

# ------------------------------------------------------------------ shell (real execution)
class Env:
    def __init__(self, cwd, env):
        self.cwd, self.env = cwd, env
    def run(self, cmd, t=60):
        try:
            r = subprocess.run(["bash", "-c", cmd], cwd=self.cwd, env=self.env, capture_output=True, text=True, timeout=t, stdin=subprocess.DEVNULL)
            return r.stdout + r.stderr
        except subprocess.TimeoutExpired as x:
            return "[timeout]"

def clip(txt, n=22, w=118):
    ls = [l[:w] + ("…" if len(l) > w else "") for l in txt.rstrip().splitlines()]
    return "\n".join(ls[:n] + (["…"] if len(ls) > n else []))

TERM_HTML = """<html><body style="margin:0;background:#0d1117;height:100vh;overflow:hidden">
<div style="display:flex;flex-direction:column;height:100vh;font:15px/1.45 'DejaVu Sans Mono',ui-monospace,monospace;color:#c9d1d9">
<div style="background:#161b22;padding:9px 14px;border-bottom:1px solid #30363d;display:flex;gap:7px;align-items:center;flex:none">
<i style="width:11px;height:11px;border-radius:6px;background:#ff5f56"></i><i style="width:11px;height:11px;border-radius:6px;background:#ffbd2e"></i><i style="width:11px;height:11px;border-radius:6px;background:#27c93f"></i>
<span style="margin-left:12px;color:#8b949e">~/thesis — bash</span></div>
<div style="flex:1;display:flex;flex-direction:column;justify-content:flex-end;overflow:hidden"><pre id="t" style="margin:0;padding:12px 18px;white-space:pre-wrap;word-break:break-word"></pre></div></div>
<script>
const T=document.getElementById('t');
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
window.typeCmd=async(c,ms)=>{const p=document.createElement('span');T.appendChild(p);
  const pr=document.createElement('span');pr.style.color='#7ee787';pr.textContent='$ ';p.appendChild(pr);
  const b=document.createElement('b');p.appendChild(b);const cur=document.createElement('span');cur.textContent='\\u2588';cur.style.color='#8b949e';p.appendChild(cur);
  for(const ch of c){b.textContent+=ch;await sleep(ms)} await sleep(350);cur.remove();T.appendChild(document.createTextNode('\\n'))};
window.out=async(s,ms)=>{const ls=s.split('\\n');for(let i=0;i<ls.length;i++){T.appendChild(document.createTextNode(ls[i]+'\\n'));if(i%2==1)await sleep(ms)} T.appendChild(document.createTextNode('\\n'))};
window.note=(s)=>{const e=document.createElement('span');e.style.color='#8b949e';e.textContent=s+'\\n\\n';T.appendChild(e)};
</script></body></html>"""

CURSOR_JS = """(()=>{const d=document.createElement('div');d.style.cssText='position:fixed;z-index:99999;width:18px;height:18px;border-radius:9px;background:rgba(220,40,40,.8);border:2px solid #fff;pointer-events:none;left:-40px;top:-40px;transform:translate(-50%,-50%);box-shadow:0 0 6px #000';
addEventListener('DOMContentLoaded',()=>document.body.appendChild(d));addEventListener('mousemove',e=>{d.style.left=e.clientX+'px';d.style.top=e.clientY+'px'},true);})();"""

class Scene:
    def __init__(self, cap, fn, hold=0.0):
        self.cap, self.fn, self.hold = cap, fn, hold

class Ctx:
    """handed to scene functions"""
    def __init__(self, pg, env, base):
        self.pg, self.env, self.base = pg, env, base
    # --- terminal
    def term_open(self):
        self.pg.set_content(TERM_HTML); self.pg.wait_for_timeout(300)
    def cmd(self, c, shown=None, out=None, n=22, ms=28, t=60):
        """type a command and show its REAL output (or `out` if given, for commands that must not run here)"""
        o = clip(self.env.run(c, t) if out is None else out, n)
        self.pg.evaluate("([c,ms])=>window.typeCmd(c,ms)", [shown or c, ms])
        self.pg.evaluate("([s,ms])=>window.out(s,ms)", [o, 90])
        self.pg.wait_for_timeout(500)
    def note(self, s): self.pg.evaluate("s=>window.note(s)", s)
    # --- web
    def goto(self, path="/"):
        self.pg.goto(self.base + path); self.pg.wait_for_timeout(1200)
    def move(self, sel):
        b = self.pg.locator(sel).first.bounding_box()
        self.pg.mouse.move(b["x"] + b["width"] / 2, b["y"] + min(b["height"] / 2, 20), steps=25)
    def click(self, sel):
        self.move(sel); self.pg.wait_for_timeout(250); self.pg.locator(sel).first.click(); self.pg.wait_for_timeout(350)
    def type(self, sel, text, ms=30):
        self.click(sel); self.pg.locator(sel).first.fill(""); self.pg.locator(sel).first.press_sequentially(text, delay=ms)
    def tab(self, name):
        self.click(f'#tabs button[data-m="{name}"]')
    def scroll(self, y, steps=12):
        for _ in range(steps):
            self.pg.mouse.wheel(0, y / steps); self.pg.wait_for_timeout(40)
    def wait(self, ms): self.pg.wait_for_timeout(ms)

# ------------------------------------------------------------------ recording
def record(browser, tut, outdir, env, base):
    vdir = os.path.join(outdir, "_rec", tut["id"]); shutil.rmtree(vdir, ignore_errors=True); os.makedirs(vdir)
    ctx = browser.new_context(viewport={"width": W, "height": H}, record_video_dir=vdir, record_video_size={"width": W, "height": H})
    ctx.add_init_script(CURSOR_JS)
    t0 = time.time(); pg = ctx.new_page()
    c = Ctx(pg, env, base)
    if tut["kind"] == "term": c.term_open()
    else: c.goto("/")
    pg.wait_for_timeout(700)
    cues = []
    for sc in tut["scenes"]:
        s = time.time() - t0
        sc.fn(c)
        need = max(len(sc.cap[l]) for l in LANGS) / CPS + 1.2 + sc.hold
        el = time.time() - t0 - s
        if el < need: pg.wait_for_timeout(int((need - el) * 1000))
        cues.append((s, time.time() - t0, sc.cap))
    pg.wait_for_timeout(1500)
    pg.close(); vid = pg.video.path(); ctx.close()
    return vid, cues

# ------------------------------------------------------------------ subtitles
def ts(x, vtt=False):
    h, m, s = int(x // 3600), int(x % 3600 // 60), x % 60
    return f"{h:02d}:{m:02d}:{s:06.3f}".replace(".", "." if vtt else ",")

def write_subs(cues, lang, base):
    srt, vtt = [], ["WEBVTT", ""]
    for i, (a, b, cap) in enumerate(cues, 1):
        txt = cap[lang].replace("`", "")
        srt += [str(i), f"{ts(a)} --> {ts(b - 0.05)}", txt, ""]
        vtt += [f"{ts(a, True)} --> {ts(b - 0.05, True)}", txt, ""]
    open(f"{base}.{lang}.srt", "w", encoding="utf-8").write("\n".join(srt))
    open(f"{base}.{lang}.vtt", "w", encoding="utf-8").write("\n".join(vtt))

STYLE = "FontName=DejaVu Sans,FontSize=11,PrimaryColour=&H00FFFFFF,OutlineColour=&H00000000,Outline=0,Shadow=0,Alignment=2,MarginV=16,MarginL=40,MarginR=40"
def ff(*a):
    r = subprocess.run(["ffmpeg", "-y", "-loglevel", "error", *a], capture_output=True, text=True)
    if r.returncode: raise RuntimeError(r.stderr[-800:])

def build(vid, cues, tid, outdir):
    d = os.path.join(outdir, tid); os.makedirs(d, exist_ok=True); base = os.path.join(d, tid)
    pad = f"pad={W}:{OUTH}:0:0:{BG},fps=25"
    clean = base + ".clean.mp4"
    ff("-i", vid, "-vf", pad, "-c:v", "libx264", "-crf", "27", "-preset", "veryfast", "-pix_fmt", "yuv420p", "-movflags", "+faststart", clean)
    for l in LANGS: write_subs(cues, l, base)
    def burn(l):
        ff("-i", vid, "-vf", f"{pad},subtitles={os.path.basename(base)}.{l}.srt:original_size={W}x{OUTH}:force_style='{STYLE}'",
           "-c:v", "libx264", "-crf", "27", "-preset", "veryfast", "-pix_fmt", "yuv420p", "-movflags", "+faststart", f"{base}.{l}.mp4")
    cwd = os.getcwd(); os.chdir(d)
    try:
        with ThreadPoolExecutor(2) as ex: list(ex.map(burn, LANGS))
        args = ["-i", clean]
        for l in LANGS: args += ["-i", f"{tid}.{l}.srt"]
        maps = ["-map", "0:v"]
        for i, l in enumerate(LANGS): maps += ["-map", str(i + 1)]
        meta = []
        for i, l in enumerate(LANGS): meta += [f"-metadata:s:s:{i}", f"language={ISO3[l]}", f"-metadata:s:s:{i}", f"title={NAMES[l]}"]
        ff(*args, *maps, "-c:v", "copy", "-c:s", "mov_text", *meta, f"{tid}.multi.mp4")
    finally:
        os.chdir(cwd)
    dur = cues[-1][1] + 1.5
    return dur
