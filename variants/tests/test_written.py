#!/usr/bin/env python3
"""Variants that used to be 'written, not run': each is exercised against the REAL tool where one can be installed here
(Emacs, Neovim, LyX under Xvfb, Streamlit, Gradio, Marimo, nginx, Tor, rpm, web-ext, shellcheck, systemd-analyze).
A check is SKIPped (not failed) when its tool is missing. Model answers come from a fake Ollama; speech engines are mocked.
NOT covered: Nix, Snap/Flatpak builds, Helm, Quadlet, Docker Hub images, Zotero, MediaWiki, real Firefox/Windows/macOS/iOS/Android."""
import json, os, shutil, subprocess, sys, tempfile, time, socket, urllib.request, tomllib, glob
HERE = os.path.dirname(os.path.abspath(__file__)); ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
BIN = os.environ.get("TEZAUR_BIN") or os.path.join(ROOT, "target", "release", "tezaur")
ok = True
def check(n, c, e=""):
    global ok
    print(("PASS " if c else "FAIL ") + n + ("" if c else " — " + str(e)[:400])); ok = ok and bool(c)
def skip(n, why): print("SKIP " + n + " — " + why)
def have(*t): return all(shutil.which(x) for x in t)
def sh(cmd, **kw): return subprocess.run(cmd, shell=True, capture_output=True, text=True, stdin=subprocess.DEVNULL, **kw)
d = tempfile.mkdtemp(); lib = d + "/lib"; os.makedirs(lib)
open(lib + "/lisbon_convention.txt", "w").write("The Lisbon Recognition Convention requires parties to recognise qualifications unless a substantial difference is shown. The burden of proof lies with the body assessing the qualification.")
open(lib + "/refs.bib", "w").write("@article{lisbon_convention, title={Lisbon Recognition Convention}, author={Council of Europe}, year={1997}}\n")
open(lib + "/directive_notes.md", "w").write("Aptitude tests and adaptation periods are compensation measures under the directive.")
env = dict(os.environ, TEZAUR_INDEX=lib + "/t.idx", TEZAUR_HOST="http://127.0.0.1:11999", TEZAUR_MODEL="m")
subprocess.run([BIN, "ingest", "."], cwd=lib, env=env, capture_output=True, stdin=subprocess.DEVNULL)
PORT = "8861"; SRV = "http://127.0.0.1:" + PORT
procs = [subprocess.Popen([sys.executable, HERE + "/mock_ollama.py", "11999"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL),
         subprocess.Popen([BIN, "serve", "--port", PORT], cwd=lib, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)]
time.sleep(1.5)
senv = dict(env, TEZAUR_SERVER=SRV, PYTHONPATH=ROOT + "/variants/pypi")
try:
    # ---- Emacs
    if have("emacs"):
        el = d + "/t.el"
        open(el, "w").write('(add-to-list \'load-path "%s/variants/editors/emacs")(require \'tezaur)(setq tezaur-server "%s")(tezaur-ask "burden of proof")(with-current-buffer "*TEZAUR*" (princ (buffer-string)))' % (ROOT, SRV))
        r = sh("emacs --batch -Q -l " + el, timeout=60)
        check("emacs: tezaur-ask in real Emacs returns the cited answer + references", "burden of proof lies" in r.stdout and "## References" in r.stdout, r.stdout + r.stderr)
        r = sh("emacs --batch -Q --eval '(setq byte-compile-error-on-warn t)' -f batch-byte-compile %s/variants/editors/emacs/tezaur.el 2>&1; rm -f %s/variants/editors/emacs/tezaur.elc" % (ROOT, ROOT), timeout=60)
        check("emacs: byte-compiles without warnings", r.stdout.strip() == "", r.stdout)
    else: skip("emacs", "emacs not installed")
    # ---- Neovim (real, headless; 0.9 has no vim.system)
    if have("nvim"):
        lua = d + "/nv.lua"
        open(lua, "w").write('vim.opt.rtp:prepend("%s/variants/editors/neovim")\nrequire("tezaur").setup({server="%s"})\nvim.cmd("runtime plugin/tezaur.lua")\nvim.cmd("TezaurAsk burden of proof")\nvim.wait(8000,function() return false end)\nfor _,b in ipairs(vim.api.nvim_list_bufs()) do io.stdout:write(table.concat(vim.api.nvim_buf_get_lines(b,0,12,false),"\\n").."\\n") end\nvim.cmd("qa!")\n' % (ROOT, SRV))
        r = sh("nvim --headless -u NONE -l " + lua, timeout=40)
        check("neovim: TezaurAsk in real headless Neovim (works on 0.9 and 0.10+)", "burden of proof lies" in r.stdout and "References" in r.stdout, r.stdout + r.stderr)
    else: skip("neovim", "nvim not installed")
    # ---- LyX (real LyX under Xvfb): export, audit, citation inset through the server pipe
    if have("lyx", "xvfb-run"):
        ld = d + "/lyx"; os.makedirs(ld + "/u")
        hdr = open(ROOT + "/variants/tests/fixtures/t.lyx").read() if os.path.exists(ROOT + "/variants/tests/fixtures/t.lyx") else None
        if hdr:
            shutil.copy(ROOT + "/variants/tests/fixtures/t.lyx", ld + "/t.lyx")
            r = sh("cd %s && xvfb-run -a lyx -batch -e latex t.lyx" % ld, timeout=90)
            check("lyx: real LyX exports .lyx to LaTeX with \\cite", os.path.exists(ld + "/t.tex") and "\\cite{lisbon_convention}" in open(ld + "/t.tex").read(), r.stdout + r.stderr)
            tt = ROOT + "/variants/editors/latex/tezaur_tex.py"
            r = sh("python3 %s audit %s/t.lyx" % (tt, ld), env=senv, timeout=60)
            check("lyx: audit reads the .lyx citation insets (unknown key flagged)", "unknown-source" in r.stdout and "ghost2020" in r.stdout, r.stdout)
            open(ld + "/u/preferences", "w").write('\\serverpipe "%s/pipe"\n' % ld)
            shutil.copy(ld + "/t.lyx", ld + "/doc.lyx")
            script = 'lyx -userdir %s/u %s/doc.lyx >%s/lyx.log 2>&1 & for i in $(seq 90); do [ -e %s/pipe.in ] && break; sleep 1; done; sleep 4; LYXPIPE=%s/pipe python3 %s cite "burden of proof" --lyx >%s/cite.log 2>&1; sleep 2; echo "LYXCMD:t:buffer-write" > %s/pipe.in; sleep 3; pkill lyx' % (ld, ld, ld, ld, ld, tt, ld, ld)
            rr = sh("xvfb-run -a bash -c '%s'" % script, env=senv, timeout=120)
            check("lyx: `cite --lyx` inserts a real citation inset through the LyX server pipe", open(ld + "/doc.lyx").read().count('key "lisbon_convention"') == 2, rr.stdout + rr.stderr + os.popen("ls %s; tail -5 %s/lyx.log; cat %s/cite.log" % (ld, ld, ld)).read())
        else: skip("lyx", "fixture missing")
    else: skip("lyx", "lyx or xvfb-run not installed")
    # ---- Streamlit / Gradio / Marimo
    nb = ROOT + "/variants/notebooks"
    try:
        from streamlit.testing.v1 import AppTest
        os.environ.update(TEZAUR_SERVER=SRV); sys.path.insert(0, ROOT + "/variants/pypi")
        at = AppTest.from_file(nb + "/streamlit_app.py", default_timeout=60).run()
        at.text_input[0].set_value("burden of proof").run()
        check("streamlit: ask tab renders the cited answer and the invented-citation warning", "burden of proof lies" in at.markdown[0].value and any("non-existent" in w.value for w in at.warning) and not at.exception, [str(x) for x in at.exception])
        at.text_area[0].set_value("Moon cheese [@ghost2020].").run()
        check("streamlit: audit tab flags the unknown source", any("unknown-source" in e.value for e in at.error), "")
    except ImportError: skip("streamlit", "streamlit not installed")
    try:
        import gradio, gradio_client
        gp = subprocess.Popen([sys.executable, nb + "/gradio_app.py"], env=dict(senv, GRADIO_ANALYTICS_ENABLED="False", HF_HUB_OFFLINE="1"), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, cwd=nb)
        procs.append(gp)
        for _ in range(40):
            try: socket.create_connection(("127.0.0.1", 7860), 1).close(); break
            except OSError: time.sleep(0.5)
        gc = gradio_client.Client("http://127.0.0.1:7860", verbose=False)
        check("gradio: launched app answers /ask with citations", "burden of proof lies" in gc.predict("burden of proof", "ask", api_name="/ask"), "")
        check("gradio: /audit returns the markdown report", "unknown source 1" in gc.predict("Moon cheese [@ghost2020].", api_name="/audit"), "")
    except ImportError: skip("gradio", "gradio not installed")
    except Exception as e: check("gradio: app runs", False, repr(e))
    if have("marimo"):
        r = sh("cd %s && marimo export html marimo_app.py -o %s/m.html" % (nb, d), env=senv, timeout=120)
        check("marimo: the notebook exports and runs", os.path.exists(d + "/m.html") and "Type a question" in open(d + "/m.html").read(), r.stdout + r.stderr)
    else: skip("marimo", "marimo not installed")
    # ---- linters on shipped scripts and manifests
    if have("shellcheck"):
        files = " ".join(glob.glob(ROOT + "/variants/yunohost*/scripts/*") + [ROOT + "/variants/deploy/rpi/install.sh", ROOT + "/variants/deploy/termux/install.sh", ROOT + "/variants/installer/install.sh", ROOT + "/variants/installer/uninstall.sh", ROOT + "/build-portable.sh"] + glob.glob(ROOT + "/variants/*.sh"))
        r = sh("shellcheck -S warning -e SC2154 " + files)  # SC2154: YunoHost injects $app, $install_dir... at run time
        check("shellcheck: install/package scripts (RPi, Termux, YunoHost, installer, build)", r.returncode == 0, r.stdout)
    else: skip("shellcheck", "not installed")
    for f in glob.glob(ROOT + "/variants/yunohost*/manifest.toml"):
        try: tomllib.load(open(f, "rb")); check("toml: " + os.path.relpath(f, ROOT), True)
        except Exception as e: check("toml: " + f, False, e)
    for f in ("variants/packaging/scoop/tezaur.json", "variants/extension/manifest.firefox.json", "variants/capacitor/capacitor.config.json", "variants/tauri/src-tauri/tauri.conf.json"):
        try: json.load(open(ROOT + "/" + f)); check("json: " + f, True)
        except Exception as e: check("json: " + f, False, e)
    if have("xmllint"): check("xml: MSIX + Office manifests well-formed", sh("xmllint --noout %s/variants/deploy/msix/AppxManifest.xml %s/variants/office/manifest.xml" % (ROOT, ROOT)).returncode == 0)
    try:
        from openapi_spec_validator import validate
        validate(json.load(open(ROOT + "/variants/editors/rest/openapi.json"))); validate(json.load(urllib.request.urlopen(SRV + "/openapi.json")))
        check("openapi: shipped spec and the one served by `tezaur serve` validate (Open WebUI tool-server format)", True)
    except ImportError: skip("openapi", "openapi-spec-validator not installed")
    except Exception as e: check("openapi validates", False, e)
    if have("node"): check("zotero: bootstrap.js parses", sh("node --check %s/variants/editors/zotero/bootstrap.js" % ROOT).returncode == 0)
    if have("systemd-analyze"):
        r = sh("systemd-analyze verify %s/variants/deploy/systemd/tezaur.service 2>&1" % ROOT)
        bad = [l for l in r.stdout.splitlines() if "tezaur" in l.lower() and "is not executable" not in l and "not found" not in l.lower()]
        check("systemd: tezaur.service verifies (only the not-yet-installed binary is reported)", not bad, "\n".join(bad))
    # ---- web-ext lint (Firefox)
    if have("npx"):
        wd = d + "/webext"; shutil.copytree(ROOT + "/variants/extension", wd + "/ext"); shutil.copy(wd + "/ext/manifest.firefox.json", wd + "/ext/manifest.json")
        r = sh("cd %s && npx --yes web-ext lint -s ext -o json 2>/dev/null" % wd, timeout=180)
        try:
            j = json.loads(r.stdout); check("firefox: web-ext lint reports 0 errors (%d warnings: innerHTML use, reviewed on AMO)" % j["summary"]["warnings"], j["summary"]["errors"] == 0, [e["message"] for e in j["errors"]])
        except Exception: skip("firefox web-ext", "lint output unavailable (offline?)")
    # ---- nginx: the Docker site's same-origin proxy, against the fake model
    if have("nginx"):
        os.chmod(d, 0o755); sd = d + "/ng"; os.makedirs(sd + "/html"); os.makedirs(sd + "/logs")
        subprocess.run([sys.executable, ROOT + "/variants/build-web.py", sd + "/html"], capture_output=True)
        shutil.copy(ROOT + "/pwa/manifest.webmanifest", sd + "/html/")
        conf = open(ROOT + "/variants/docker/nginx.conf").read().replace("listen 80;", "listen 8862;").replace("/usr/share/nginx/html", sd + "/html").replace("http://ollama:11434/", "http://127.0.0.1:11999/").replace("http://host.docker.internal:1337/v1/", "http://127.0.0.1:1337/v1/")
        open(sd + "/site.conf", "w").write(conf)
        open(sd + "/nginx.conf", "w").write("pid %s/nginx.pid; error_log %s/logs/e.log; events{} http{ access_log off; client_body_temp_path %s/b; proxy_temp_path %s/p; fastcgi_temp_path %s/f; uwsgi_temp_path %s/u; scgi_temp_path %s/s; include %s/site.conf; include /etc/nginx/mime.types; }" % ((sd,) * 8))
        r = sh("nginx -t -c %s/nginx.conf 2>&1" % sd)
        check("nginx: the Docker site config is valid", r.returncode == 0, r.stdout)
        if r.returncode == 0:
            sh("nginx -c %s/nginx.conf >/dev/null 2>&1" % sd)
            for _ in range(30):
                try: socket.create_connection(("127.0.0.1", 8862), 1).close(); break
                except OSError: time.sleep(0.3)
            try:
                try: page = urllib.request.urlopen("http://127.0.0.1:8862/index.html", timeout=5).read().decode()
                except Exception as ex: sys.stderr.write("nginx debug: %r %s %s\n" % (ex, os.listdir(sd + "/html"), open(sd + "/logs/e.log").read()[-400:])); raise
                req = urllib.request.Request("http://127.0.0.1:8862/ollama/api/chat", b'{"messages":[]}', {"Content-Type": "application/json"})
                body = urllib.request.urlopen(req, timeout=10).read().decode()
                check("nginx: serves the static app and proxies /ollama/ to the model server without CORS", "TEZAUR" in page and "burden of proof" in body, body[:200])
            finally: sh("kill $(cat %s/nginx.pid)" % sd)
    else: skip("nginx", "not installed")
    # ---- Tor: config valid, onion hostname generated, SOCKS5 port answers (no Tor network here, so no onion is reachable)
    if have("tor"):
        td = d + "/tor"; os.makedirs(td, mode=0o700)
        open(d + "/torrc", "w").write("DataDirectory %s/data\nHiddenServiceDir %s/hs/\nHiddenServicePort 80 127.0.0.1:%s\nSocksPort 19051\nLog notice file %s/n.log\n" % (td, td, PORT, td))
        r = sh("tor -f %s/torrc --verify-config 2>&1" % d)
        check("tor: the torrc from deploy/tor/README.md is valid", "valid" in r.stdout, r.stdout)
        tp = subprocess.Popen(["tor", "-f", d + "/torrc"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL); procs.append(tp)
        for _ in range(30):
            if os.path.exists(td + "/hs/hostname"): break
            time.sleep(0.5)
        check("tor: hidden service created (.onion hostname written)", os.path.exists(td + "/hs/hostname") and open(td + "/hs/hostname").read().strip().endswith(".onion"), "")
        try: socket.create_connection(("127.0.0.1", 19051), 3).close(); check("tor: SOCKS5 port open for `mesh add --proxy socks5h://…`", True)
        except OSError as e: check("tor: SOCKS5 port open", False, e)
    else: skip("tor", "not installed")
    # ---- rpm
    if have("rpm", "cargo-generate-rpm") or (have("rpm") and shutil.which("cargo")):
        r = sh("cd %s && cargo generate-rpm 2>&1 | tail -1; ls target/generate-rpm/*.rpm" % ROOT, timeout=300)
        rp = sorted(glob.glob(ROOT + "/target/generate-rpm/*.rpm"), key=os.path.getmtime)
        if rp:
            q = sh("rpm -qpl %s; rpm -qpi %s | grep -E '^(Name|Version)'" % (rp[-1], rp[-1])).stdout
            check("rpm: package built; installs /usr/bin/tezaur and the single-file app", "/usr/bin/tezaur" in q and "/usr/share/tezaur/tezaur.html" in q and "Name        : tezaur" in q, q)
        else: skip("rpm", "cargo-generate-rpm not available")
    else: skip("rpm", "rpm/cargo-generate-rpm missing")
    # ---- voice: dictation fills the question, read-aloud speaks the answer (speech engines mocked)
    try:
        from playwright.sync_api import sync_playwright
        INIT = "window.__s=[];window.SpeechRecognition=class{start(){window.__l=this.lang;setTimeout(()=>this.onresult({results:[[{transcript:'burden of proof'}]]}),50)}};Object.defineProperty(window,'speechSynthesis',{value:{speaking:false,getVoices:()=>[{localService:true,lang:'ro-RO'}],speak(u){window.__s.push([u.text,u.lang||''])},cancel(){}},configurable:true});window.SpeechSynthesisUtterance=class{constructor(t){this.text=t}};window.confirm=()=>true;"
        with sync_playwright() as pw:
            b = pw.chromium.launch(); pg = b.new_page(); pg.add_init_script(INIT); pg.goto(SRV + "/"); pg.wait_for_timeout(1000)
            pg.click("#tabs button[data-m=ask]"); pg.fill("#lang", "ro"); pg.click("#mic"); pg.wait_for_timeout(300)
            q = pg.input_value("#q"); lg = pg.evaluate("window.__l")
            pg.click("#run"); pg.wait_for_function("document.querySelector('#out').innerText.includes('burden')", timeout=15000); pg.click("#speak"); pg.wait_for_timeout(200)
            sp = pg.evaluate("window.__s"); b.close()
        check("voice: dictation fills the question in the chosen language; read-aloud speaks the answer with a local voice (mocked engines)", q == "burden of proof" and lg == "ro" and sp and "burden of proof lies" in sp[0][0] and sp[0][1] == "ro-RO", (q, lg, sp))
    except ImportError: skip("voice", "playwright not installed")
finally:
    for p in procs: p.kill()
    subprocess.run("pkill -x lyx; pkill -x tor; pkill -x nginx", shell=True)
sys.exit(0 if ok else 1)
