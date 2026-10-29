# TEZAUR — all the ways to run it

| Variant | Where | Status |
|---|---|---|
| **CLI** (Linux/macOS/Windows) | repo root, `cargo build --release` | built + 50 tests |
| **MCP server** — your library as tools inside Claude Desktop / Claude Code / Cursor | `tezaur mcp` | tested over stdio (initialize, tools/list, tools/call, errors) |
| **PortableApps.com** folder | `build-portable.sh/.bat`, `packaging/` | Linux relocation test only; Windows exe not built |
| **Single-file HTML** | `tezaur.html` | tested headless against a mock server |
| **PWA** (install from Chrome/Edge/Safari) | `pwa/` | service worker + manifest verified |
| **Browser extension** Chrome/Edge/Brave + Firefox | `variants/extension`, `build-extension.sh` | Chromium: loaded, ingested, answered from a server **without CORS**. Firefox: manifest only |
| **Desktop app** Electron (Win nsis + portable, macOS dmg, Linux AppImage + deb) | `variants/electron` | app runs under Xvfb (self-test); installers only via CI |
| **Desktop app** Tauri 2 (small, native webview) | `variants/tauri` | dependency resolution only; needs CORS on the model server; downloads may need a plugin |
| **Android APK** | `android/` + `.github/workflows/android.yml` | not compiled (SDK unreachable here) |
| **.deb** (Debian/Ubuntu/Mint) | `variants/build-deb.sh` | built, listed, extracted and executed |
| **Docker / Compose** (with Ollama and a same-origin proxy, no CORS setup) | `variants/docker` | files written; image not built here |
| **YunoHost** app (packaging v2) | `variants/yunohost` | syntax-checked; not linted, not installed |
| **`tezaur serve`** — local web server: the web app + JSON API, PDF/EPUB/OCR on the server, embeddings, workspaces, index sync, model proxy (no CORS) | `tezaur serve --token X --workspaces DIR` | all endpoints tested with curl; web app server mode tested end to end in Chromium (upload incl. OCR'd scan, search, ask, summarize, workspaces) |
| **OCR** for scanned PDFs (poppler + tesseract, automatic) | `ingest` | tested on an image-only PDF (English). Romanian needs the `tesseract-ocr-ron` language pack |
| **`--rerank`** / **`--verify`** (model re-orders passages; model checks each cited sentence) | global flags | unit tests for the parsers + runs against a mock model; usefulness depends on the model |
| **`tezaur export`** Markdown → LaTeX (`\cite{}` + BibTeX), Word, HTML | `tezaur export d.md --to tex` | LaTeX compiled with pdflatex + bibtex (citations resolved); docx opened by pandoc; HTML escaped |
| **`tezaur zotero`** — BibTeX from Zotero's local API + its storage folder | `tezaur zotero` | against a mock of the API (items matched to PDFs through the `file` field); not against a real Zotero |
| **Telegram / Matrix bots** | `variants/bots` | against mock services: allow-lists, self-message and other-room filtering verified; not the real services |
| **Jupyter / JupyterLab / IPython / VS Code notebooks / Colab / Kaggle** — magics (`%tezaur_ask`, `%%tezaur_audit`, `%%tezaur_coverage`…), notebook auditor, JupyterLab launcher tile (jupyter-server-proxy), Streamlit · Gradio · Marimo front-ends | `variants/pypi` (`tezaur_client.magics`, `.notebook`, `.proxy`), `variants/notebooks`; older `variants/jupyter` kept | a notebook **executed in a real Jupyter kernel** against the live server (search, ask, audit, coverage, receipt check, no kernel errors); notebook auditor run; **the JupyterLab tile and the Streamlit/Gradio/Marimo apps are written but not run** (compiled only); Colab/Kaggle can reach only a publicly reachable server |
| **LyX · TeXstudio · Geany · Kate · Helix · Zed · Neovim · Emacs-eglot · Sublime-LSP** (LaTeX world) — audit of `.tex`/`.lyx` (`\\cite`, `\\parencite[p. 3]{}`, LyX citation insets), `\\cite` suggestions, optional insertion into the open LyX document through its server pipe | `variants/editors/latex` (`tezaur_tex.py`, config snippets) | `.tex` and `.lyx` audited through the live server, `cite` run, the LyX pipe write checked against a FIFO; **LyX, TeXstudio, Geany, Kate and Helix themselves not run** — the converter, build-menu and user-command syntaxes are from their documentation; the LyX `self-insert` LFUN inserts the text, not a citation inset |
| **Connectors to AI assistants** — Claude Desktop/Code, Cursor, Windsurf, Cline, Roo, Continue, Gemini CLI, Codex, LM Studio, Zed, VS Code over stdio; Goose/Jan/AnythingLLM/Open WebUI as printed instructions; cloud assistants through `POST /mcp` + `/openapi.json` | `CONNECTORS.md`, installer, `tezaur serve` | config files written and removed against a fake home; `/mcp` and `/openapi.json` tested with HTTP; **no real AI client was connected** |
| **Meta-installer** `tezaur-setup` — detects Claude Desktop / Claude Code / Cursor / Windsurf / VS Code, Vim, Neovim, Emacs, Sublime, Notepad++, Obsidian vaults, Joplin, Logseq, Zotero, browsers, pandoc, Jupyter, WordPress / DokuWiki / MediaWiki folders, Android phones (adb), PortableApps USB drives, ssh hosts and TEZAUR nodes on the LAN; scans first, installs on `--yes`, remembers every change, `uninstall` reverses it (`--sweep` finds leftovers without the record, `--purge` removes the program and `~/.tezaur`, `--purge-data --yes` also the data folders, `--dry-run` previews; `uninstall.sh` / `Uninstall-Tezaur.bat`), `--remote host` pushes itself over ssh | `variants/installer/` (also `install.sh`, `Install-Tezaur.bat`; ships inside `tezaur-plugins.zip`) | run against a **fake home directory full of fake applications**: scan, install, second run idempotent, status, uninstall restores configs byte-for-byte; adb/ssh/claude/pip/systemctl calls checked through shims. **Never run on real Windows or macOS; the real applications' own config formats are as documented, not exercised** |
| **`audit` · `coverage` · `novelty` · `ask --receipt` · `receipt` · `declare`** — check every `[@key, p. N]` citation of a draft against the cited page, quotations word-for-word, which outline sections have sources, what a text adds to the library, tamper-evident receipts for answers, an AI-use statement | `tezaur audit/coverage/novelty/receipt/declare`, `/api/audit` `/api/coverage` `/api/novelty` `/api/receipt`, MCP `tezaur_audit` `tezaur_coverage`, LSP diagnostics on save | unit tests (SHA-256/HMAC vectors, verdicts, tamper detection) + end-to-end through CLI, HTTP API, MCP and LSP. These are **mechanical term-overlap checks, not proof**; a receipt shows what the tool saw and that it was not edited, not that the claim is true |
| **`audit --semantic` · `refs` · `eval` · `lit` · `refs --online`** — local-model second opinion on each citation (paraphrases, negations), reference hygiene, retrieval metrics on your own questions, opt-in literature search and DOI/retraction check against OpenAlex/Crossref, an *Audit* / *References* panel in the web app, on-disk BM25 cache | `tezaur audit --semantic/refs/eval/lit`, `/api/refs`, `/api/audit` (`semantic`), MCP `tezaur_refs`, `tezaur.html` (server mode) | Rust unit tests + `test_round6.py` (CLI against a **scripted stand-in model** and **fake OpenAlex/Crossref servers**) + `test_panel.py` (real Chromium). **No real model and no real OpenAlex/Crossref was ever contacted**; semantic verdicts from a real model are unmeasured |
| **LibreOffice** macros (find / insert references) | `variants/libreoffice` | logic run with a mocked UNO document; not loaded in LibreOffice |
| **Windows installer** (Inno Setup) · **rpm** · **Nix flake** · **server Dockerfile** · **GHCR publishing** · **Capacitor** (Android + iOS) | `variants/windows`, `Cargo.toml` metadata, `flake.nix`, `Dockerfile`, `.github/workflows/docker.yml`, `variants/capacitor` | flake parses; Capacitor project generation tested (`cap add android` + sync); the rest never built here |
| **Obsidian plugin** — asks your vault, citations are `[[links]]` to the exact heading | `variants/obsidian`, `build-obsidian.sh` | engine unit-tested (6 tests); plugin run against a mocked Obsidian API (index, ask, search, insert); not loaded in real Obsidian |
| **VS Code extension** — search/ask your Markdown/LaTeX workspace, answers cite file:line | `variants/vscode`, `build-vscode.sh` | run against a mocked VS Code API; not loaded in real VS Code |
| **Word add-in** (task pane + “insert into document”) | `variants/office`, `build-office.sh` | button + insertion verified with a mocked Office.js; needs HTTPS hosting and sideloading |
| **iPhone / iPad** (WKWebView shell) | `variants/ios` | written, never compiled (needs a Mac; CI job on a macOS runner) |
| **Snap, Flatpak, Homebrew, Scoop, winget, AUR** | `variants/packaging` | manifests with `OWNER`/hash placeholders; syntax-checked only |
| **Mesh P2P** — federated search, pull shared documents, offline `.tzb` bundles, LAN discovery, peers over Tor/SOCKS; browsers connect by WebRTC with pasted codes (no server) | `tezaur mesh …`, "Mesh P2P" panel in the web app, [MESH.md](MESH.md) | two real servers (search, pull, token refusal, no leak of unshared docs, dedupe, bundles, UDP discovery, SOCKS proxy); two isolated Chromium browsers connected over WebRTC with no signalling server; browser↔Rust bundle interchange both ways; federated *ask* in server mode. Not tested across real machines, NAT or the internet |
| **`/api/ask`, `/api/bib`, OpenAPI** — one JSON API for every plugin below | `tezaur serve`, `variants/editors/rest/openapi.yaml` | spec validated; all endpoints used by the tests |
| **Exports**: LaTeX/Word/HTML, **PDF** (pdflatex), **ODT/EPUB** (pandoc), bibliography as **RIS / CSL JSON / BibTeX**, flashcards as Anki **.apkg** | `tezaur export --to pdf|odt|epub`, `tezaur bib --to ris|csl-json|bibtex`, `tezaur-apkg` | PDF compiled with citations resolved; ODT/EPUB produced by pandoc; .apkg read back from its database; **not imported into Anki or Zotero** |
| **DokuWiki · WordPress · MediaWiki** plugins | `variants/cms` | DokuWiki and WordPress run against stubs of their APIs and the live server (ask, search, token kept server-side, per-IP rate limit, HTML escaping); MediaWiki only `php -l`; none installed in the real CMS |
| **Vim · Neovim · Emacs · Sublime Text · Notepad++** (PythonScript + NppExec) | `variants/editors/{vim,neovim,emacs,sublime,notepadpp}` | Vim: real headless Vim; Neovim: Lua run in a mock `vim` (real curl); Sublime/Notepad++: mocked APIs; **Emacs: not run (no Emacs here)**. A native Notepad++ DLL is not built |
| **Joplin · Logseq · Zotero 7 · Raycast · Alfred · pandoc/Quarto filter** | `variants/editors/…` | Joplin/Logseq logic, Raycast scripts, Alfred filter, and the pandoc Lua filter run against the live server; Alfred bundle is a valid zip+plist but not loaded in Alfred; **Zotero plugin untested** |
| **Language server** (citation key completion, hover, unknown-key warnings) for any LSP editor | `variants/editors/lsp/tezaur_lsp.py` | driven over stdio by a test client |
| **Slack · Discord · WhatsApp · e-mail** bots | `variants/bots` | signature checks (HMAC, Ed25519), replay rejection, allow-lists, threading, against mock services; not the real services |
| **Google Docs** add-on (Apps Script) | `variants/gdocs` | logic run with mocked Apps Script services; needs an https server and short answers (Google's ~30–60 s fetch limit) |
| **Calibre** sync (uploads books, writes BibTeX with matching keys) | `variants/calibre` | against the live server with a fixture; `calibredb` itself not run |
| **PyPI** `tezaur-client` · **npm** `tezaur-client` · crates.io metadata | `variants/pypi`, `variants/npm`, `Cargo.toml` | wheel installed in a venv and npm tarball installed, both run against the server; **not published** (names may be taken) |
| **Browser extension**: side panel (Chrome/Edge) / sidebar (Firefox) + right-click "Ask TEZAUR" · **PWA share target** · **Android share** | `variants/extension`, `pwa/`, `android/` | Chromium: APIs present and `?q=` opens the app; PWA share parameters tested; Android share intent only syntax-checked |
| **Voice**: dictation (browser speech recognition) and read-aloud | web app | buttons present; **in Chrome/Edge dictation is cloud-based** (the app asks first); not exercised with a microphone |
| **YunoHost package for `tezaur serve`** | `variants/yunohost-serve` | manifest parses, scripts `bash -n`; never installed or linted |
| **systemd · launchd · Raspberry Pi · Helm · Docker Desktop · Termux · F-Droid · Homebrew Cask · MSIX · NixOS module · Podman Quadlet · Tor** | `variants/deploy` | well-formedness checks (plist, YAML, XML, `ruby -c`, `nix-instantiate --parse`, `bash -n`); nothing deployed; Helm not rendered |
| **GitHub Pages** (public PWA URL) | `.github/workflows/pages.yml` | YAML valid; never run |
| **CI for everything** | `.github/workflows/*.yml` | YAML valid; never run |

## One command to install everywhere
```
python3 tezaur-setup.py            # scan: lists what it found, changes nothing
python3 tezaur-setup.py install --yes
python3 tezaur-setup.py uninstall  # exact undo
```
See the docstring of `variants/installer/tezaur-setup.py` for `--only`, `--skip`, `--all` (background service, mesh peers) and `--remote user@host`.
Browsers, Joplin, Logseq and Zotero only accept plugins through their own UI, so for those it unpacks/stages the file and prints the click path.

## MCP in Claude Desktop / Claude Code

```json
{ "mcpServers": { "tezaur": { "command": "/path/to/tezaur", "args": ["--index", "/path/to/tezaur.idx", "mcp"] } } }
```
Tools: `tezaur_search` (passages with citation labels, optional `only`), `tezaur_sources`, `tezaur_read`
(passages of one document in order), `tezaur_audit` (check a draft's citations), `tezaur_coverage` (which outline sections have sources). Read-only; the client's own model does the writing, so no local
model server is needed. In Claude Code: `claude mcp add tezaur -- /path/to/tezaur --index /path/to/tezaur.idx mcp`.

## Editors: shared engine
`variants/core/tezaur-core.js` (BM25 with diacritic folding, sentence-aware chunking with line numbers and headings, the same
grounded prompts and citation check; `node --test variants/core/test.js`) is bundled into the Obsidian plugin and the
VS Code extension. These talk to the model non-streaming (Obsidian's `requestUrl`, Node `fetch`), so CORS is not an issue there.

## `tezaur serve` in one minute
```
tezaur --index ~/thesis/tezaur.idx --api ollama --model llama3.1:8b serve --workspaces ~/thesis/ws
# open http://127.0.0.1:8787  — add files in the page; they are read by the server (PDF, OCR, EPUB)
tezaur serve --bind 0.0.0.0 --token SECRET     # LAN/VPN access: the token is mandatory off localhost
```
Workspaces (`?ws=name`, one index + Library per chapter/student), `GET/PUT /api/index` (copy an index between machines;
a corrupt upload is refused), `/llm/*` forwards to the configured model server. Run the same thing in Docker: root `Dockerfile`.

## Run every automated check
`variants/tests/run-all.sh` — Rust tests, then a mock model + real `tezaur serve` against which the CMS plugins, editors, LSP, bots and the
Google Docs logic run; then audit/receipts through CLI/API/MCP, a notebook in a real Jupyter kernel, and the meta-installer against a fake home. (Browser/mesh tests use Playwright and are listed in `MESH.md`.)

## Still not built (and why)
- **Excel / PowerPoint / Outlook add-ins, JetBrains and Zed plugins, Nextcloud / Moodle / Discourse apps, Safari extension, Siri Shortcuts file**:
  each needs its own toolchain or an Apple/IDE runtime that I could not run; `variants/editors/rest/tezaur.http` and the OpenAPI spec
  give JetBrains/Postman/Insomnia users the API directly, and a Siri/Android "Shortcut" is one "Get contents of URL" action against `/api/ask`.
- **Local speech-to-text (Whisper) on the server**: not built (no whisper binary to test with).
- **WebAssembly build of the Rust engine**: the wasm target cannot be installed in my sandbox, so I could not compile or test it;
  `tezaur serve` already gives the web app the native engine.
- **Zotero 7 plugin** is written (`variants/editors/zotero`) but untested for lack of a Zotero to load it in.
- **LibreOffice *extension package* (.oxt)** with a toolbar: the macros work as scripts; packaging needs LibreOffice to validate.
- **Signed installers (Windows/macOS) and store submissions** (Play, App Store, Chrome Web Store, Flathub): need certificates/accounts.
- **MSIX**: requires signing; the Inno Setup installer covers Windows without it.
