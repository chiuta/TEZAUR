# Video tutorials (8 subtitle languages)

`python3 scripts/tutorials/make.py [--bin target/release/tezaur] [--out DIR] [ids…]` records 13 short tutorials
and builds, for each: `<id>.<lang>.mp4` (subtitles burned in), `<id>.<lang>.srt|vtt`, `<id>.clean.mp4`,
`<id>.multi.mp4` (clean video + 8 selectable subtitle tracks) and an `index.html` player.

Languages: en, ro, fr, it, es, pt, de, hu. No voice-over: the narration is the subtitles.

What is real: the web UI is the real `tezaur serve` page driven by Playwright; every terminal tutorial runs the real
`tezaur` binary on the demo library in `demo/` and shows its actual output (the install tutorial only *shows* the package
manager commands, which CI runs on every push). What is not: the language model is `variants/tests/mock_ollama.py`, a stand-in
with a canned answer (it deliberately includes one invented citation to show the verifier), the web page itself is in Romanian,
and the translations were machine-written and have not been reviewed by native speakers.

Needs python3 + playwright (chromium), ffmpeg (libx264, libass), poppler-utils. The videos are not committed (regenerate them).
