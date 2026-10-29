#!/usr/bin/env python3
"""Local dictation with Whisper inside the page (real Chromium, a fake microphone fed with a spoken sentence).
Needs: espeak-ng, ffmpeg, playwright+chromium, network access to cdn.jsdelivr.net and huggingface.co (model download, ~40 MB). Run in CI (real-variants-2.yml)."""
import os, subprocess, sys, tempfile, time
from playwright.sync_api import sync_playwright
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
d = tempfile.mkdtemp(); wav = d + "/speech.wav"
subprocess.run(["espeak-ng", "-s", "150", "-v", "en-us+f3", "-w", d + "/raw.wav", "The burden of proof lies with the body assessing the qualification"], check=True)
subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", d + "/raw.wav", "-ar", "48000", "-ac", "1", wav], check=True)
ok = False
with sync_playwright() as pw:
    b = pw.chromium.launch(args=["--use-fake-ui-for-media-stream", "--use-fake-device-for-media-stream", "--use-file-for-fake-audio-capture=" + wav, "--autoplay-policy=no-user-gesture-required"])
    ctx = b.new_context(permissions=["microphone"]); pg = ctx.new_page(); logs = []
    pg.on("console", lambda m: logs.append(m.text)); pg.on("pageerror", lambda e: logs.append("ERR " + str(e)))
    pg.goto("file://" + ROOT + "/tezaur.html"); pg.add_init_script("window.__whisperModel='Xenova/whisper-base'"); pg.reload(); pg.wait_for_timeout(1000)
    pg.click("#tabs button[data-m=ask]") if pg.query_selector("#tabs button[data-m=ask]") else None
    pg.fill("#lang", "en")
    assert pg.is_visible("#mic2"), "local dictation button hidden"
    pg.click("#mic2"); pg.wait_for_timeout(5500); pg.click("#mic2")
    try:
        pg.wait_for_function("document.querySelector('#q').value.length > 5", timeout=240000)
    except Exception: pass
    q = pg.input_value("#q"); st = pg.inner_text("#dictst"); print("question field:", repr(q), "| status:", st)
    ok = "burden" in q.lower() and "proof" in q.lower()
    if not ok: print("console:", logs[-8:])
    b.close()
print("PASS whisper: spoken sentence transcribed locally in the page" if ok else "FAIL whisper"); sys.exit(0 if ok else 1)
