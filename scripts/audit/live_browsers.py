#!/usr/bin/env python3
"""Cross-browser behaviour of the LIVE site (Chromium, Firefox, WebKit). Emits GitHub annotations."""
import sys, time
from playwright.sync_api import sync_playwright
H, BR = sys.argv[1], sys.argv[2]
BASE = "https://" + H
def note(t, m): print(f"::notice title={BR}:{t}::{str(m)[:1400]}".replace("\n", "%0A"), flush=True)
def bad(t, m): print(f"::error title={BR}:{t}::{str(m)[:1400]}".replace("\n", "%0A"), flush=True)
with sync_playwright() as p:
    b = getattr(p, BR).launch()
    ctx = b.new_context(viewport={"width": 1280, "height": 800}); pg = ctx.new_page(); errs = []; fails = []
    pg.on("console", lambda m: errs.append(m.text) if m.type in ("error", "warning") else None)
    pg.on("pageerror", lambda e: errs.append("PAGEERROR " + str(e)))
    pg.on("requestfailed", lambda r: fails.append(r.url + " " + str(r.failure)))
    t0 = time.time(); pg.goto(BASE + "/", wait_until="load", timeout=90000); load = time.time() - t0
    pg.wait_for_timeout(2500)
    note("load", f"load event {load:.1f}s; slides={pg.evaluate('document.querySelectorAll(\".slide\").length')}; counter='{pg.evaluate('document.querySelector(\"#count\").textContent')}'; title='{pg.title()}'")
    (bad if errs else note)("console", errs[:6] or "clean"); (bad if fails else note)("failed-requests", fails[:6] or "none")
    pg.evaluate("document.getElementById('vid') && document.getElementById('vid').scrollIntoView()"); pg.wait_for_timeout(1500)
    info = pg.evaluate("""()=>{const v=document.querySelector('#vgrid video');if(!v)return null;return {n:document.querySelectorAll('#vgrid video').length,mp4:v.canPlayType('video/mp4; codecs="avc1.42E01E"'),tracks:v.textTracks.length}}""")
    if not info: bad("video-section", "no #vgrid video on the live page"); sys.exit(0)
    note("video-support", info)
    for k in range(13):
        r = pg.evaluate("""async k=>{const v=document.querySelectorAll('#vgrid video')[k];v.muted=true;v.load();
            await new Promise(r=>{v.onloadedmetadata=r;v.onerror=r;setTimeout(r,15000)});
            let pl='';try{await v.play();await new Promise(r=>setTimeout(r,2500))}catch(e){pl=String(e)}
            const st={rs:v.readyState,ns:v.networkState,paused:v.paused,dur:v.duration,buf:v.buffered.length?v.buffered.end(0):0};
            const tr=[...v.textTracks].map(t=>t.language+':'+t.mode+':'+(t.cues?t.cues.length:'null'));
            const o={k,id:v.dataset.id,st,t:v.currentTime,err:v.error&&v.error.code,play:pl,tracks:tr.join(' ')};v.pause();return o}""", k)
        (note if r["t"] > 0.5 and not r["err"] else bad)("video-%d" % k, r)
    pg.click("#vlang button[data-l=fr]"); pg.wait_for_timeout(600)
    r = pg.evaluate("[...document.querySelector('#vgrid video').textTracks].filter(t=>t.mode==='showing').map(t=>t.language)")
    (note if r == ["fr"] else bad)("track-switch", r)
    c = b.new_context(viewport={"width": 360, "height": 740}, is_mobile=(BR != "firefox"), has_touch=(BR != "firefox")); m = c.new_page(); m.goto(BASE + "/", wait_until="load", timeout=90000); m.wait_for_timeout(1500)
    o = m.evaluate("({sw:document.documentElement.scrollWidth,cw:document.documentElement.clientWidth})")
    (bad if o["sw"] > o["cw"] + 2 else note)("mobile-360", o)
    if BR == "chromium":
        cdp = ctx.new_cdp_session(pg); cdp.send("Network.enable")
        cdp.send("Network.emulateNetworkConditions", dict(offline=False, latency=150, downloadThroughput=int(1.6e6 / 8), uploadThroughput=int(750e3 / 8)))
        p2 = ctx.new_page(); t0 = time.time(); p2.goto(BASE + "/", wait_until="domcontentloaded", timeout=120000); d1 = time.time() - t0
        try: p2.wait_for_selector("#s1.in", timeout=120000)
        except Exception: pass
        note("slow-3g", f"Fast-3G: DOMContentLoaded {d1:.1f}s, hero revealed {time.time()-t0:.1f}s")
    b.close()
