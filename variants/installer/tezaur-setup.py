#!/usr/bin/env python3
"""tezaur-setup — the TEZAUR meta-installer (Python 3.8+, standard library only; Linux, macOS, Windows).

Looks at what is installed on this computer (and what is plugged in or reachable), shows a plan, and installs the matching
TEZAUR integration into each application it finds. Nothing is changed unless you say `install` (or --yes).

    python3 tezaur-setup.py                      # scan: what was found and what would be done (changes nothing)
    python3 tezaur-setup.py install              # ask before each group
    python3 tezaur-setup.py install --yes        # do everything that is ready
    python3 tezaur-setup.py install --only mcp,editors --skip emacs
    python3 tezaur-setup.py install --all        # also the opt-in extras: background service, mesh peers
    python3 tezaur-setup.py status               # what this tool installed earlier
    python3 tezaur-setup.py uninstall            # undo exactly what it did (files removed, config keys restored)
    python3 tezaur-setup.py install --remote me@raspberrypi     # push the same installer to another machine over ssh

Where the files come from: a folder or zip made by variants/build-plugins.sh (--source, or next to this script, or ../../dist/plugins).
Everything it writes is recorded in ~/.tezaur/installed.json, which is what `uninstall` reads.
Config files are backed up once as <name>.tezaur-backup before the first edit."""
import argparse
import json
import os
import platform
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
import zipfile
from pathlib import Path

VERSION = "1.0"
SYS = os.environ.get("TEZAUR_SETUP_OS") or {"Linux": "linux", "Darwin": "mac", "Windows": "win"}.get(platform.system(), "linux")
HOME = Path(os.environ.get("TEZAUR_SETUP_HOME") or Path.home())
FAKE = bool(os.environ.get("TEZAUR_SETUP_HOME"))  # tests: never look outside the fake home
MANIFEST = HOME / ".tezaur" / "installed.json"
HERE = Path(__file__).resolve().parent


def cfg_dir():
    if SYS == "mac":
        return HOME / "Library" / "Application Support"
    if SYS == "win":
        return Path(os.environ.get("APPDATA") or HOME / "AppData" / "Roaming") if not FAKE else HOME / "AppData" / "Roaming"
    return Path(os.environ.get("XDG_CONFIG_HOME") or HOME / ".config") if not FAKE else HOME / ".config"


def data_dir():
    if SYS == "mac":
        return HOME / "Library" / "Application Support"
    if SYS == "win":
        return Path(os.environ.get("LOCALAPPDATA") or HOME / "AppData" / "Local") if not FAKE else HOME / "AppData" / "Local"
    return Path(os.environ.get("XDG_DATA_HOME") or HOME / ".local" / "share") if not FAKE else HOME / ".local" / "share"


def which(name):
    if FAKE:  # only the PATH the test set up
        return shutil.which(name, path=os.environ.get("PATH"))
    return shutil.which(name)


def say(*a):
    print(*a, flush=True)


# --------------------------------------------------------------------------- bundle (where artifacts come from)

class Bundle:
    def __init__(self, source=None):
        self.tmp = None
        cands = [Path(source)] if source else [HERE, HERE / "plugins", HERE.parent.parent / "dist" / "plugins", HERE.parent.parent / "dist"]
        self.dirs = []
        for c in cands:
            if c.is_dir():
                self.dirs.append(c)
            elif c.is_file() and c.suffix == ".zip":
                self.tmp = Path(tempfile.mkdtemp(prefix="tezaur-bundle-"))
                zipfile.ZipFile(c).extractall(self.tmp)
                self.dirs.append(self.tmp)
        if not source:
            z = HERE / "tezaur-plugins.zip"
            if z.exists():
                self.tmp = Path(tempfile.mkdtemp(prefix="tezaur-bundle-"))
                zipfile.ZipFile(z).extractall(self.tmp)
                self.dirs.append(self.tmp)

    def find(self, name):
        for d in self.dirs:
            p = d / name
            if p.exists():
                return p
        return None

    def glob(self, pattern):
        for d in self.dirs:
            hit = sorted(d.glob(pattern))
            if hit:
                return hit[0]
        return None

    def cleanup(self):
        if self.tmp:
            shutil.rmtree(self.tmp, ignore_errors=True)


def find_binary(explicit, bundle):
    if explicit:
        return str(Path(explicit).resolve())
    exe = "tezaur.exe" if SYS == "win" else "tezaur"
    for c in [which("tezaur"), *(str(d / exe) for d in bundle.dirs), str(HERE.parent.parent / "target" / "release" / exe), str(HERE / exe)]:
        if c and Path(c).exists():
            return str(Path(c).resolve())
    return None


# --------------------------------------------------------------------------- recording what we did

class Ledger:
    """Everything written goes here so `uninstall` can reverse it exactly."""

    def __init__(self):
        self.data = {"version": VERSION, "items": {}}
        if MANIFEST.exists():
            try:
                self.data = json.loads(MANIFEST.read_text("utf-8"))
            except ValueError:
                pass

    def add(self, item_id, rec):
        self.data["items"].setdefault(item_id, []).append(rec)
        self.save()

    def drop(self, item_id):
        self.data["items"].pop(item_id, None)
        self.save()

    def save(self):
        MANIFEST.parent.mkdir(parents=True, exist_ok=True)
        MANIFEST.write_text(json.dumps(self.data, indent=1), "utf-8")


def backup_once(p):
    b = p.with_name(p.name + ".tezaur-backup")
    if p.exists() and not b.exists():
        shutil.copy2(p, b)


def read_json(p):
    if not p.exists():
        return {}
    txt = p.read_text("utf-8").strip()
    return json.loads(txt) if txt else {}


def json_set(item, led, path, keys, value):
    """Set keys[0][keys[1]]… = value in a JSON file; remember what was there before."""
    obj = read_json(path)  # raises ValueError for JSON-with-comments: caller reports it as manual
    cur = obj
    for k in keys[:-1]:
        cur = cur.setdefault(k, {})
    existed, old = keys[-1] in cur, cur.get(keys[-1])
    backup_once(path)
    cur[keys[-1]] = value
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, ensure_ascii=False), "utf-8")
    led.add(item, {"t": "json-key", "path": str(path), "keys": keys, "existed": existed, "old": old})


def json_list_add(item, led, path, keys, value):
    obj = read_json(path)
    cur = obj
    for k in keys[:-1]:
        cur = cur.setdefault(k, {})
    lst = cur.setdefault(keys[-1], [])
    if value not in lst:
        backup_once(path)
        lst.append(value)
        path.write_text(json.dumps(obj, indent=2, ensure_ascii=False), "utf-8")
        led.add(item, {"t": "json-list", "path": str(path), "keys": keys, "value": value})


def mkdirs(path, item, led):
    """Create path; record every directory that did not exist so uninstall can prune exactly those."""
    missing = []
    q = path
    while not q.exists() and q != q.parent:
        missing.append(q)
        q = q.parent
    path.mkdir(parents=True, exist_ok=True)
    for d in reversed(missing):
        led.add(item, {"t": "dir", "path": str(d)})


def extract(zpath, dest, item, led, prefix=None):
    """Unzip into dest (a directory that did not exist) and record it as one removable unit."""
    mkdirs(dest, item, led)
    with zipfile.ZipFile(zpath) as z:
        for n in z.namelist():
            if n.endswith("/") or "__pycache__" in n:
                continue
            rel = n[len(prefix):] if prefix and n.startswith(prefix) else n
            if not rel or ".." in Path(rel).parts:
                continue
            out = dest / rel
            mkdirs(out.parent, item, led)
            if out.exists():
                backup_once(out)
            out.write_bytes(z.read(n))
            led.add(item, {"t": "file", "path": str(out)})


def copy_file(src, dest, item, led):
    mkdirs(dest.parent, item, led)
    if dest.exists():
        backup_once(dest)
    shutil.copy2(src, dest)
    led.add(item, {"t": "file", "path": str(dest)})


MARK_A, MARK_B = "tezaur >>>", "<<< tezaur"


def copy_tree(src, dest, item, led):
    mkdirs(dest.parent, item, led)
    fresh = not dest.exists()
    shutil.copytree(src, dest, dirs_exist_ok=True)
    if fresh:
        led.add(item, {"t": "tree", "path": str(dest)})


def text_block(item, led, path, comment, body):
    path.parent.mkdir(parents=True, exist_ok=True)
    cur = path.read_text("utf-8") if path.exists() else ""
    if MARK_A in cur:
        return
    backup_once(path)
    block = "\n%s %s\n%s\n%s %s\n" % (comment, MARK_A, body, comment, MARK_B)
    path.write_text(cur + block, "utf-8")
    led.add(item, {"t": "text-block", "path": str(path), "comment": comment})


def latex_tools(c, zpath):
    """Unpack the LaTeX-world helper scripts/snippets to ~/.tezaur/tools/latex, with TEZAUR_TOOLS → the real path in LSP snippets."""
    dest = HOME / ".tezaur" / "tools" / "latex"
    if (dest / "tezaur_tex.py").exists():
        return
    extract(zpath, dest, "latex-tools", c.led)
    lsp = B_find_lsp(c)
    if lsp:
        copy_file(lsp, HOME / ".tezaur" / "tools" / "tezaur_lsp.py", "latex-tools", c.led)
    for f in (dest / "lsp-configs").glob("*"):
        f.write_text(f.read_text("utf-8").replace("TEZAUR_TOOLS", (HOME / ".tezaur" / "tools").as_posix()), "utf-8")


def B_find_lsp(c):
    return c.bundle.find("tezaur_lsp.py")


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=kw.pop("timeout", 120), **kw)


# --------------------------------------------------------------------------- items

class Item:
    def __init__(self, id, group, title, state, detail="", apply=None, hint="", optin=False):
        # state: ready | manual | missing | root | absent
        self.id, self.group, self.title, self.state, self.detail, self.apply, self.hint, self.optin = id, group, title, state, detail, apply, hint, optin

    def row(self):
        return {"id": self.id, "group": self.group, "title": self.title, "state": self.state, "detail": self.detail, "hint": self.hint, "optin": self.optin}


class Ctx:
    def __init__(self, bundle, binary, index, led, args):
        self.bundle, self.bin, self.index, self.led, self.args = bundle, binary, index, led, args


def writable_dir(p):
    q = p
    while not q.exists() and q != q.parent:
        q = q.parent
    return os.access(q, os.W_OK)


def mcp_entry(ctx, flavour="std"):
    e = {"command": ctx.bin or "tezaur", "args": ["mcp"]}
    if ctx.index:
        e["env"] = {"TEZAUR_INDEX": str(ctx.index)}
    if flavour == "vscode":
        e = dict({"type": "stdio"}, **e)
    if flavour == "zed":
        e = dict({"source": "custom"}, **e)
    return e


def codex_toml(ctx):
    t = "[mcp_servers.tezaur]\ncommand = %s\nargs = [\"mcp\"]" % json.dumps(ctx.bin or "tezaur")
    if ctx.index:
        t += "\n[mcp_servers.tezaur.env]\nTEZAUR_INDEX = %s" % json.dumps(str(ctx.index))
    return t


def mcp_targets():
    """(id, title, config file, key path, application present?, entry flavour, note) for every MCP client whose config is a JSON file."""
    code_user = cfg_dir() / "Code" / "User"
    gs = code_user / "globalStorage"
    return [
        ("claude-desktop", "Claude Desktop (MCP)", cfg_dir() / "Claude" / "claude_desktop_config.json", ["mcpServers", "tezaur"], (cfg_dir() / "Claude").exists(), "std", "restart Claude Desktop afterwards"),
        ("cursor", "Cursor (MCP)", HOME / ".cursor" / "mcp.json", ["mcpServers", "tezaur"], (HOME / ".cursor").exists(), "std", ""),
        ("windsurf", "Windsurf (MCP)", HOME / ".codeium" / "windsurf" / "mcp_config.json", ["mcpServers", "tezaur"], (HOME / ".codeium" / "windsurf").exists(), "std", ""),
        ("vscode-mcp", "VS Code (MCP)", code_user / "mcp.json", ["servers", "tezaur"], code_user.exists(), "vscode", "VS Code ≥ 1.99; format from its documentation, not tested here"),
        ("cline", "Cline (VS Code)", gs / "saoudrizwan.claude-dev" / "settings" / "cline_mcp_settings.json", ["mcpServers", "tezaur"], (gs / "saoudrizwan.claude-dev").exists(), "std", ""),
        ("roo", "Roo Code (VS Code)", gs / "rooveterinaryinc.roo-cline" / "settings" / "mcp_settings.json", ["mcpServers", "tezaur"], (gs / "rooveterinaryinc.roo-cline").exists(), "std", ""),
        ("continue", "Continue", HOME / ".continue" / "mcpServers" / "tezaur.json", ["mcpServers", "tezaur"], (HOME / ".continue").exists(), "std", "format from Continue's documentation, not tested here"),
        ("gemini-cli", "Gemini CLI", HOME / ".gemini" / "settings.json", ["mcpServers", "tezaur"], (HOME / ".gemini").exists(), "std", ""),
        ("lmstudio", "LM Studio (MCP)", HOME / ".lmstudio" / "mcp.json", ["mcpServers", "tezaur"], (HOME / ".lmstudio").exists(), "std", ""),
        ("zed", "Zed (MCP)", cfg_dir() / "zed" / "settings.json", ["context_servers", "tezaur"], (cfg_dir() / "zed").exists(), "zed", "Zed's settings usually contain comments; then this is shown as manual"),
    ]


def scan(ctx):
    items = []
    B = ctx.bundle

    # --- the program itself
    if ctx.bin:
        target = (HOME / ".local" / "bin" / "tezaur") if SYS != "win" else (data_dir() / "Programs" / "Tezaur" / "tezaur.exe")
        on_path = which("tezaur")
        if on_path and Path(on_path).resolve() == Path(ctx.bin).resolve() or target.exists():
            items.append(Item("binary", "core", "tezaur program", "absent", "already installed at " + (on_path or str(target))))
        else:
            def ap(c, target=target):
                copy_file(c.bin, target, "binary", c.led)
                target.chmod(0o755)
                c.bin = str(target)
            items.append(Item("binary", "core", "Install the tezaur program", "ready", "%s → %s" % (ctx.bin, target), ap,
                              "add %s to PATH if it is not already" % target.parent))
    else:
        items.append(Item("binary", "core", "tezaur program", "missing", "no tezaur binary found (use --bin, put it on PATH, or build: cargo build --release)"))

    # --- model servers (information only)
    models = []
    for name, port, path in (("Ollama", 11434, "/api/tags"), ("Jan", 1337, "/v1/models"), ("LM Studio", 1234, "/v1/models")):
        if FAKE:
            break
        try:
            with urllib.request.urlopen("http://127.0.0.1:%d%s" % (port, path), timeout=0.6) as r:
                if r.status == 200:
                    models.append("%s on :%d" % (name, port))
        except Exception:
            pass
    ctx.models = models

    # --- AI assistants via MCP (the library as tools)
    def mcp_json(id, title, path, keys, present, flavour="std", note=""):
        if not present:
            return
        def ap(c, path=path, keys=keys, flavour=flavour, id=id):
            json_set(id, c.led, path, keys, mcp_entry(c, flavour))
        try:
            read_json(path)
            items.append(Item(id, "mcp", title, "ready", "add tezaur to " + str(path), ap, note))
        except ValueError:
            items.append(Item(id, "mcp", title, "manual", "%s is not plain JSON (comments?)" % path, None, 'add: "tezaur": %s' % json.dumps(mcp_entry(ctx))))

    for id, title, path, keys, present, flavour, note in mcp_targets():
        mcp_json(id, title, path, keys, present, flavour, note)
    # Codex CLI keeps its servers in TOML: a marked block, no TOML parser needed
    codex = HOME / ".codex" / "config.toml"
    if (HOME / ".codex").exists():
        def ap(c, codex=codex):
            text_block("codex", c.led, codex, "#", codex_toml(c))
        items.append(Item("codex", "mcp", "OpenAI Codex CLI (MCP)", "ready", "marked block in " + str(codex), ap, "format from Codex's documentation; not tested here"))
    # clients that only accept a server through their own UI
    for id, title, present, how in (
        ("goose", "Goose", which("goose") or (cfg_dir() / "goose").exists(), "goose configure → Add Extension → Command-line extension → command: %s mcp" % (ctx.bin or "tezaur")),
        ("jan", "Jan (as MCP client)", (cfg_dir() / "Jan").exists(), "Settings → MCP Servers → add: command %s, args: mcp" % (ctx.bin or "tezaur")),
        ("open-webui", "Open WebUI", which("open-webui") or (HOME / ".open-webui").exists(), "Settings → Tools → add a tool server: http://127.0.0.1:8787/openapi.json (run `tezaur serve`; add the token as Bearer)"),
        ("anythingllm", "AnythingLLM", (cfg_dir() / "anythingllm-desktop").exists(), "Agent skills → MCP servers: command %s, args: mcp" % (ctx.bin or "tezaur")),
    ):
        if present:
            items.append(Item(id, "mcp", title, "manual", "configured in its own UI", None, how))
    claude = which("claude")
    if claude:
        def ap(c, claude=claude):
            r = run([claude, "mcp", "add", "--scope", "user", "tezaur", "--", c.bin or "tezaur", "mcp"])
            if r.returncode != 0:
                raise RuntimeError((r.stderr or r.stdout).strip()[:300])
            c.led.add("claude-code", {"t": "cmd", "undo": [claude, "mcp", "remove", "--scope", "user", "tezaur"]})
        items.append(Item("claude-code", "mcp", "Claude Code (MCP)", "ready", "claude mcp add --scope user tezaur", ap))

    # --- editors
    def pack_target(vim):
        if vim:
            return (HOME / ("vimfiles" if SYS == "win" else ".vim") / "pack" / "tezaur" / "start" / "tezaur")
        base = (data_dir() / "nvim-data") if SYS == "win" else (data_dir() / "nvim")
        return base / "site" / "pack" / "tezaur" / "start" / "tezaur"

    for id, title, exe, art, vim, dirs in (
        ("vim", "Vim", "vim", "tezaur-vim.zip", True, [HOME / ".vim", HOME / "vimfiles"]),
        ("neovim", "Neovim", "nvim", "tezaur-neovim.zip", False, [cfg_dir() / "nvim", HOME / ".config" / "nvim"]),
    ):
        if not (which(exe) or any(d.exists() for d in dirs)):
            continue
        z = B.find(art)
        if not z:
            items.append(Item(id, "editors", title, "missing", art + " not in the bundle"))
            continue
        items.append(Item(id, "editors", title, "ready", "→ " + str(pack_target(vim)), lambda c, z=z, v=vim, i=id: extract(z, pack_target(v), i, c.led)))

    emacs_inits = [HOME / ".emacs.d" / "init.el", HOME / ".config" / "emacs" / "init.el", HOME / ".emacs"]
    if which("emacs") or any(p.exists() for p in emacs_inits) or (HOME / ".emacs.d").exists():
        el = B.find("tezaur.el")
        if el:
            init = next((p for p in emacs_inits if p.exists()), emacs_inits[0])
            lispdir = init.parent / "lisp" if init.name == "init.el" else HOME / ".emacs.d" / "lisp"
            def ap(c, el=el, init=init, lispdir=lispdir):
                copy_file(el, lispdir / "tezaur.el", "emacs", c.led)
                text_block("emacs", c.led, init, ";;", '(add-to-list \'load-path "%s")\n(require \'tezaur)' % lispdir.as_posix())
            items.append(Item("emacs", "editors", "Emacs", "ready", "tezaur.el → %s and a marked block in %s" % (lispdir, init), ap))
        else:
            items.append(Item("emacs", "editors", "Emacs", "missing", "tezaur.el not in the bundle"))

    sub_dirs = {"linux": cfg_dir() / "sublime-text" / "Packages", "mac": cfg_dir() / "Sublime Text" / "Packages", "win": cfg_dir() / "Sublime Text" / "Packages"}
    if sub_dirs[SYS].parent.exists():
        z = B.find("tezaur-sublime.zip")
        if z:
            items.append(Item("sublime", "editors", "Sublime Text", "ready", "→ " + str(sub_dirs[SYS] / "TEZAUR"), lambda c, z=z: extract(z, sub_dirs[SYS], "sublime", c.led)))

    npp = cfg_dir() / "Notepad++" if SYS == "win" else None
    if npp and npp.exists():
        z = B.find("tezaur-notepadpp.zip")
        if z:
            dest = npp / "plugins" / "config" / "PythonScript" / "scripts"
            items.append(Item("notepadpp", "editors", "Notepad++", "ready", "scripts → " + str(dest), lambda c, z=z, d=dest: extract(z, d, "notepadpp", c.led),
                              "needs the PythonScript plugin (Plugins Admin) or use the bundled .nppexec file"))

    code = which("code") or which("codium")
    vs_ext = HOME / (".vscode-oss" if (code and "codium" in code) else ".vscode") / "extensions"
    if code or vs_ext.exists():
        z = B.find("tezaur-vscode.zip")
        if z:
            items.append(Item("vscode", "editors", "VS Code / VSCodium extension", "ready", "→ " + str(vs_ext / "trom.tezaur-1.0.0"), lambda c, z=z, d=vs_ext / "trom.tezaur-1.0.0": extract(z, d, "vscode", c.led), "reload the window"))

    # --- note-taking, reference managers
    ob_json = cfg_dir() / "obsidian" / "obsidian.json"
    if ob_json.exists():
        z = B.find("tezaur-obsidian.zip")
        try:
            vaults = [Path(v["path"]) for v in read_json(ob_json).get("vaults", {}).values() if Path(v.get("path", "")).exists()]
        except Exception:
            vaults = []
        for v in vaults:
            if z:
                def ap(c, z=z, v=v):
                    extract(z, v / ".obsidian" / "plugins" / "tezaur", "obsidian:" + v.name, c.led)
                    cp = v / ".obsidian" / "community-plugins.json"
                    lst = json.loads(cp.read_text("utf-8")) if cp.exists() else []
                    if "tezaur" not in lst:
                        backup_once(cp)
                        cp.write_text(json.dumps(lst + ["tezaur"]), "utf-8")
                        c.led.add("obsidian:" + v.name, {"t": "json-array-file", "path": str(cp), "value": "tezaur"})
                items.append(Item("obsidian:" + v.name, "notes", "Obsidian vault “%s”" % v.name, "ready", "→ %s/.obsidian/plugins/tezaur" % v, ap, "turn off Restricted mode in Settings → Community plugins if it is on"))
    for id, title, art, hint in (
        ("joplin", "Joplin", "tezaur-joplin.jpl", "Settings → Plugins → ⚙ Install from file → tezaur-joplin.jpl"),
        ("logseq", "Logseq", "tezaur-logseq.zip", "Settings → Advanced → Developer mode, then Plugins → Load unpacked plugin"),
        ("zotero", "Zotero", "tezaur-zotero.xpi", "Tools → Plugins → ⚙ → Install Plugin From File"),
    ):
        present = {"joplin": (cfg_dir() / "joplin-desktop").exists(), "logseq": (HOME / ".logseq").exists() or (cfg_dir() / "Logseq").exists(),
                   "zotero": (HOME / "Zotero").exists() or bool(which("zotero"))}[id]
        if present and B.find(art):
            def ap(c, art=art, id=id):
                d = HOME / ".tezaur" / "to-install"
                copy_file(B.find(art), d / art, id, c.led)
            items.append(Item(id, "notes", title, "manual", "these apps load plugins only from their own UI; the file is staged for you", ap, hint))

    # --- browsers
    browsers = [(n, any(which(x) for x in exes) or any(p.exists() for p in dirs)) for n, exes, dirs in (
        ("Chrome / Edge / Brave", ("google-chrome", "chromium", "chromium-browser", "brave-browser", "microsoft-edge"), [cfg_dir() / "google-chrome", cfg_dir() / "chromium", cfg_dir() / "BraveSoftware", cfg_dir() / "Google" / "Chrome"]),
        ("Firefox", ("firefox",), [HOME / ".mozilla", cfg_dir() / "Firefox"]),
    )]
    for (n, ok), art, sub in zip(browsers, ("tezaur-chrome.zip", "tezaur-firefox.zip"), ("chrome", "firefox")):
        if ok and B.find(art):
            dest = HOME / ".tezaur" / "extension" / sub
            hint = "chrome://extensions → Developer mode → Load unpacked → " + str(dest) if sub == "chrome" else "about:debugging#/runtime/this-firefox → Load Temporary Add-on → manifest.json in " + str(dest)
            items.append(Item("browser-" + sub, "browsers", n + " extension", "manual", "unpacked to " + str(dest) + " (browsers do not allow silent installs)", lambda c, art=art, dest=dest, sub=sub: extract(B.find(art), dest, "browser-" + sub, c.led), hint))

    # --- writing tools
    if which("pandoc"):
        z = B.find("tezaur-pandoc-quarto.zip")
        if z:
            dest = (cfg_dir() / "pandoc" / "filters") if SYS == "win" else (HOME / ".local" / "share" / "pandoc" / "filters")
            def ap(c, z=z, dest=dest):
                with zipfile.ZipFile(z) as zz:
                    tmp = Path(tempfile.mkdtemp())
                    zz.extract("tezaur.lua", tmp)
                    copy_file(tmp / "tezaur.lua", dest / "tezaur.lua", "pandoc", c.led)
            items.append(Item("pandoc", "writing", "Pandoc / Quarto filter", "ready", "tezaur.lua → " + str(dest), ap, "use: pandoc --lua-filter=tezaur.lua"))
    if which("calibredb") or which("calibre"):
        s = B.find("tezaur_calibre.py")
        if s:
            items.append(Item("calibre", "writing", "Calibre library sync script", "ready", "→ ~/.tezaur/tools/tezaur_calibre.py", lambda c, s=s: copy_file(s, HOME / ".tezaur" / "tools" / "tezaur_calibre.py", "calibre", c.led),
                              "run: python3 ~/.tezaur/tools/tezaur_calibre.py --library ~/Calibre"))
    lsp = B.find("tezaur_lsp.py")
    if lsp and any(which(x) for x in ("nvim", "hx", "helix", "zed", "emacs", "code")):
        items.append(Item("lsp", "editors", "TEZAUR language server (citation completion + audit on save)", "ready", "→ ~/.tezaur/tools/tezaur_lsp.py", lambda c, s=lsp: copy_file(s, HOME / ".tezaur" / "tools" / "tezaur_lsp.py", "lsp", c.led),
                          "point your editor's LSP client at: python3 ~/.tezaur/tools/tezaur_lsp.py"))

    # --- LaTeX world: LyX, TeXstudio, Geany, Kate and any LSP editor
    tex = B.find("tezaur-latex.zip")
    tools = HOME / ".tezaur" / "tools"
    lyxdir = next((d for d in (HOME / ".lyx", cfg_dir() / "lyx", cfg_dir() / "LyX") if d.exists()), None)
    if tex and (which("lyx") or lyxdir):
        lyxdir = lyxdir or HOME / ".lyx"
        def ap(c, tex=tex, lyxdir=lyxdir):
            latex_tools(c, tex)
            snip = (tools / "latex" / "lyx-preferences.snippet").read_text("utf-8").replace("TEZAUR_TOOLS", (tools / "latex").as_posix())
            text_block("lyx", c.led, lyxdir / "preferences", "#", snip)
        items.append(Item("lyx", "editors", "LyX", "ready", "File → Export → “TEZAUR audit” via a converter block in %s" % (lyxdir / "preferences"), ap,
                          "restart LyX; the converter syntax follows LyX's documentation, not tested here. Citations: python3 ~/.tezaur/tools/latex/tezaur_tex.py cite \"words\" [--lyx]"))
    geany = HOME / ".config" / "geany"
    if tex and (which("geany") or geany.exists()):
        def ap(c, tex=tex, geany=geany):
            latex_tools(c, tex)
            snip = (tools / "latex" / "geany-filetypes.latex.conf.snippet").read_text("utf-8").replace("TEZAUR_TOOLS", (tools / "latex").as_posix())
            text_block("geany", c.led, geany / "filedefs" / "filetypes.latex.conf", "#", snip)
        items.append(Item("geany", "editors", "Geany", "ready", "build-menu entry in %s" % (geany / "filedefs" / "filetypes.latex.conf"), ap, "Build → TEZAUR audit"))
    if tex and (which("texstudio") or (cfg_dir() / "texstudio").exists()):
        items.append(Item("texstudio", "editors", "TeXstudio", "manual", "user command to add by hand", lambda c, tex=tex: latex_tools(c, tex), "see ~/.tezaur/tools/latex/texstudio-user-tools.txt"))
    if tex and (which("kate") or (cfg_dir() / "kate").exists()):
        items.append(Item("kate", "editors", "Kate", "manual", "LSP client settings to merge by hand", lambda c, tex=tex: latex_tools(c, tex), "merge ~/.tezaur/tools/latex/lsp-configs/kate-lspclient.json into Kate's LSP Client settings"))
    for exe, cfgname, fname in (("hx", "Helix", "helix-languages.toml"), ("zed", "Zed (language server)", "zed-settings.json")):
        if tex and (which(exe) or which("helix")) and exe == "hx":
            items.append(Item("helix", "editors", "Helix (language server)", "manual", "append to ~/.config/helix/languages.toml", lambda c, tex=tex: latex_tools(c, tex), "snippet: ~/.tezaur/tools/latex/lsp-configs/helix-languages.toml"))
            break

    # --- notebooks
    py = which("python3") or which("python")
    if py and (which("jupyter") or which("jupyter-lab") or which("ipython") or (HOME / ".jupyter").exists() or (HOME / ".ipython").exists()):
        whl = B.glob("tezaur_client-*.whl")
        if whl:
            def ap(c, py=py, whl=whl):
                r = run([py, "-m", "pip", "install", "--user", str(whl)], timeout=300)
                if r.returncode != 0 and "externally-managed" in (r.stderr + r.stdout):
                    raise RuntimeError("this Python is managed by the system (PEP 668); install into a virtualenv or pipx instead:  pip install %s" % whl)
                if r.returncode != 0:
                    r = run([py, "-m", "pip", "install", str(whl)], timeout=300)
                if r.returncode != 0:
                    raise RuntimeError((r.stderr or r.stdout).strip()[-300:])
                c.led.add("jupyter", {"t": "cmd", "undo": [py, "-m", "pip", "uninstall", "-y", "tezaur-client"]})
            items.append(Item("jupyter", "notebooks", "Jupyter / JupyterLab / IPython / VS Code notebooks", "ready", "pip install " + whl.name, ap, "in a notebook: %load_ext tezaur_client.magics"))

    # --- other places
    wp = [p for p in ([Path(a) for a in ctx.args.wordpress] + ([] if FAKE else [Path("/var/www/html"), Path("/var/www/wordpress"), Path("/srv/www/wordpress")])) if (p / "wp-content" / "plugins").is_dir()]
    for p in wp:
        z = B.find("tezaur-wordpress.zip")
        if z:
            st = "ready" if writable_dir(p / "wp-content" / "plugins") else "root"
            items.append(Item("wordpress:" + str(p), "cms", "WordPress at " + str(p), st, "→ %s/wp-content/plugins/tezaur-research" % p, lambda c, z=z, p=p: extract(z, p / "wp-content" / "plugins", "wordpress:" + str(p), c.led),
                              "activate it under Plugins" if st == "ready" else "not writable: re-run with sudo"))
    dw = [p for p in ([Path(a) for a in ctx.args.dokuwiki] + ([] if FAKE else [Path("/var/www/dokuwiki"), Path("/usr/share/dokuwiki"), Path("/var/lib/dokuwiki")])) if (p / "lib" / "plugins").is_dir()]
    for p in dw:
        z = B.find("tezaur-dokuwiki.zip")
        if z:
            st = "ready" if writable_dir(p / "lib" / "plugins") else "root"
            items.append(Item("dokuwiki:" + str(p), "cms", "DokuWiki at " + str(p), st, "→ %s/lib/plugins/tezaur" % p, lambda c, z=z, p=p: extract(z, p / "lib" / "plugins", "dokuwiki:" + str(p), c.led),
                              "" if st == "ready" else "not writable: re-run with sudo"))
    mw = [p for p in [Path(a) for a in ctx.args.mediawiki] if (p / "extensions").is_dir()]
    for p in mw:
        z = B.find("tezaur-mediawiki.zip")
        if z:
            items.append(Item("mediawiki:" + str(p), "cms", "MediaWiki at " + str(p), "ready", "→ %s/extensions/Tezaur" % p, lambda c, z=z, p=p: extract(z, p / "extensions", "mediawiki:" + str(p), c.led),
                              'then add to LocalSettings.php: wfLoadExtension( "Tezaur" );'))

    # --- background service (opt-in)
    if SYS == "linux" and which("systemctl"):
        unit = HOME / ".config" / "systemd" / "user" / "tezaur.service"
        def ap(c, unit=unit):
            unit.parent.mkdir(parents=True, exist_ok=True)
            unit.write_text("[Unit]\nDescription=TEZAUR (local only)\n\n[Service]\nExecStart=%s serve\nRestart=on-failure\n%s\n[Install]\nWantedBy=default.target\n" % (c.bin or "tezaur", ("Environment=TEZAUR_INDEX=%s\n" % c.index) if c.index else ""), "utf-8")
            c.led.add("service", {"t": "file", "path": str(unit)})
            if not FAKE:
                run(["systemctl", "--user", "daemon-reload"])
                run(["systemctl", "--user", "enable", "--now", "tezaur"])
                c.led.add("service", {"t": "cmd", "undo": ["systemctl", "--user", "disable", "--now", "tezaur"]})
        items.append(Item("service", "extras", "Run TEZAUR in the background (systemd user service)", "ready", "→ " + str(unit), ap, "", optin=True))
    if SYS == "mac":
        pl = HOME / "Library" / "LaunchAgents" / "org.tezaur.serve.plist"
        def ap(c, pl=pl):
            pl.parent.mkdir(parents=True, exist_ok=True)
            pl.write_text('<?xml version="1.0" encoding="UTF-8"?>\n<plist version="1.0"><dict><key>Label</key><string>org.tezaur.serve</string><key>ProgramArguments</key><array><string>%s</string><string>serve</string></array><key>RunAtLoad</key><true/><key>KeepAlive</key><true/></dict></plist>\n' % (c.bin or "tezaur"), "utf-8")
            c.led.add("service", {"t": "file", "path": str(pl)})
            if not FAKE:
                run(["launchctl", "load", str(pl)])
                c.led.add("service", {"t": "cmd", "undo": ["launchctl", "unload", str(pl)]})
        items.append(Item("service", "extras", "Run TEZAUR in the background (launchd)", "ready", "→ " + str(pl), ap, "", optin=True))

    # --- devices
    adb = which("adb")
    if adb:
        r = run([adb, "devices"])
        serials = [l.split()[0] for l in r.stdout.splitlines()[1:] if l.strip().endswith("device")]
        apk = B.glob("*.apk")
        for s in serials:
            if apk:
                def ap(c, s=s, apk=apk):
                    r = run([adb, "-s", s, "install", "-r", str(apk)], timeout=300)
                    if r.returncode != 0 or "Success" not in r.stdout:
                        raise RuntimeError((r.stdout + r.stderr).strip()[-300:])
                    c.led.add("android:" + s, {"t": "cmd", "undo": [adb, "-s", s, "uninstall", "tf.trom.tezaur"]})
                items.append(Item("android:" + s, "devices", "Android phone " + s, "ready", "adb install " + apk.name, ap))
            else:
                items.append(Item("android:" + s, "devices", "Android phone " + s, "missing", "no .apk in the bundle (build it: GitHub Actions → android-apk)", None, "or open the PWA in Chrome → Install app"))
    for root in ([] if FAKE else [Path("/media") / os.environ.get("USER", ""), Path("/run/media") / os.environ.get("USER", ""), Path("/mnt"), Path("/Volumes")]) + [Path(a) for a in ctx.args.usb]:
        if not root.is_dir():
            continue
        cands = [root] if (root / "PortableApps").is_dir() else [d for d in root.iterdir() if d.is_dir() and (d / "PortableApps").is_dir()]
        for d in cands:
            src = B.find("TezaurPortable") or B.find("TezaurPortable.zip")
            dest = d / "PortableApps" / "TezaurPortable"
            if src and src.is_dir():
                items.append(Item("usb:" + str(d), "devices", "PortableApps drive " + str(d), "ready", "→ " + str(dest), lambda c, src=src, dest=dest, d=d: copy_tree(src, dest, "usb:" + str(d), c.led)))
            elif src:
                items.append(Item("usb:" + str(d), "devices", "PortableApps drive " + str(d), "ready", "→ " + str(dest), lambda c, src=src, dest=dest, d=d: extract(src, dest.parent, "usb:" + str(d), c.led)))
            else:
                items.append(Item("usb:" + str(d), "devices", "PortableApps drive " + str(d), "missing", "TezaurPortable is not in the bundle (build-portable)"))

    ssh_cfg = HOME / ".ssh" / "config"
    hosts = []
    if ssh_cfg.exists():
        for l in ssh_cfg.read_text("utf-8", "replace").splitlines():
            m = re.match(r"\s*Host\s+(.+)", l, re.I)
            if m:
                hosts += [h for h in m.group(1).split() if not re.search(r"[*?!]", h)]
    for h in hosts:
        items.append(Item("ssh:" + h, "remote", "ssh host " + h, "manual", "not touched unless you ask", None, "python3 tezaur-setup.py install --yes --remote " + h))

    if ctx.bin and not FAKE:
        try:
            r = run([ctx.bin, "mesh", "discover", "--seconds", "2"], timeout=10)
            for line in r.stdout.splitlines():
                m = re.match(r"(\S+)\s+(https?://\S+)\s", line)
                if m:
                    name, url = m.groups()
                    items.append(Item("peer:" + name, "peers", "TEZAUR node “%s” on the network" % name, "ready", "mesh add %s %s" % (name, url),
                                      lambda c, name=name, url=url: (run([c.bin, "mesh", "add", name, url]), c.led.add("peer:" + name, {"t": "cmd", "undo": [c.bin, "mesh", "rm", name]})), "sharing stays opt-in on both sides", optin=True))
        except Exception:
            pass
    return items


# --------------------------------------------------------------------------- uninstall

def drop_backup(p):
    """The one-time backup is no longer needed once our edit is reversed, and leaving it would litter the user's config folders."""
    b = p.with_name(p.name + ".tezaur-backup")
    if b.exists():
        b.unlink()


def undo_record(rec):
    t = rec["t"]
    if t == "file":
        p = Path(rec["path"])
        b = p.with_name(p.name + ".tezaur-backup")
        if b.exists():
            shutil.move(str(b), str(p))
        elif p.exists():
            p.unlink()
    elif t == "dir":
        p = Path(rec["path"])
        if p.is_dir() and not any(p.iterdir()):
            p.rmdir()
    elif t == "tree":
        shutil.rmtree(rec["path"], ignore_errors=True)
    elif t == "json-key":
        p = Path(rec["path"])
        if p.exists():
            obj = read_json(p)
            cur = obj
            for k in rec["keys"][:-1]:
                cur = cur.get(k, {})
            if rec["existed"]:
                cur[rec["keys"][-1]] = rec["old"]
            else:
                cur.pop(rec["keys"][-1], None)
            p.write_text(json.dumps(obj, indent=2, ensure_ascii=False), "utf-8")
        drop_backup(p)
    elif t in ("json-list", "json-array-file"):
        p = Path(rec["path"])
        if p.exists():
            obj = json.loads(p.read_text("utf-8"))
            if t == "json-list":
                cur = obj
                for k in rec["keys"][:-1]:
                    cur = cur.get(k, {})
                lst = cur.get(rec["keys"][-1], [])
            else:
                lst = obj
            if rec["value"] in lst:
                lst.remove(rec["value"])
            p.write_text(json.dumps(obj, indent=2), "utf-8")
    elif t == "text-block":
        p = Path(rec["path"])
        if p.exists():
            txt = p.read_text("utf-8")
            c = re.escape(rec["comment"])
            p.write_text(re.sub(r"\n?%s %s.*?%s %s\n?" % (c, re.escape(MARK_A), c, re.escape(MARK_B)), "", txt, flags=re.S), "utf-8")
        drop_backup(p)
    elif t == "cmd":
        try:
            run(rec["undo"], timeout=60)
        except Exception:
            pass


def uninstall(led, only):
    n = 0
    for item in list(led.data["items"]):
        if only and not any(item == o or item.startswith(o) for o in only):
            continue
        recs = led.data["items"][item]
        for rec in reversed(recs):  # directories after their files; JSON edits in reverse order
            if rec["t"] != "dir":
                undo_record(rec)
        for rec in reversed(recs):
            if rec["t"] == "dir":
                undo_record(rec)
        led.drop(item)
        n += 1
        say("  removed", item)
    return n


# --------------------------------------------------------------------------- complete removal

def _ours(entry):
    """True if a config value is a TEZAUR MCP entry (so a sweep never deletes somebody else's 'tezaur' key)."""
    if not isinstance(entry, dict):
        return False
    cmd = str(entry.get("command", "")) + " " + " ".join(map(str, entry.get("args", [])))
    return "tezaur" in cmd.lower() or entry.get("args") == ["mcp"]


def sweep_plan(extra_roots, purge, purge_data):
    """Everything TEZAUR-shaped that this tool (or a hand install following the docs) could have left, found without the ledger."""
    acts = []  # (description, callable)
    add = lambda d, f: acts.append((d, f))
    for id, title, path, keys, _, _, _ in mcp_targets():
        if path.exists():
            try:
                cur = read_json(path)
                for k in keys[:-1]:
                    cur = cur.get(k, {})
                if _ours(cur.get(keys[-1])):
                    def rm(path=path, keys=keys):
                        obj = read_json(path)
                        cur = obj
                        for k in keys[:-1]:
                            cur = cur[k]
                        cur.pop(keys[-1], None)
                        if path.parent.name == "mcpServers" and not obj.get(keys[0]):
                            path.unlink()
                        else:
                            path.write_text(json.dumps(obj, indent=2, ensure_ascii=False), "utf-8")
                    add("MCP entry “tezaur” in %s" % path, rm)
            except (ValueError, AttributeError):
                pass
    for blockfile, comment in ((HOME / ".codex" / "config.toml", "#"), (HOME / ".lyx" / "preferences", "#"), (cfg_dir() / "lyx" / "preferences", "#"),
                               (HOME / ".config" / "geany" / "filedefs" / "filetypes.latex.conf", "#"), (HOME / ".emacs.d" / "init.el", ";;"),
                               (HOME / ".config" / "emacs" / "init.el", ";;"), (HOME / ".emacs", ";;")):
        if blockfile.exists() and MARK_A in blockfile.read_text("utf-8", "replace"):
            add("marked TEZAUR block in %s" % blockfile, lambda f=blockfile, c=comment: undo_record({"t": "text-block", "path": str(f), "comment": c}))
    dirs = [HOME / ".vim" / "pack" / "tezaur", HOME / "vimfiles" / "pack" / "tezaur", data_dir() / "nvim" / "site" / "pack" / "tezaur", data_dir() / "nvim-data" / "site" / "pack" / "tezaur",
            cfg_dir() / "sublime-text" / "Packages" / "TEZAUR", cfg_dir() / "Sublime Text" / "Packages" / "TEZAUR",
            HOME / ".vscode" / "extensions" / "trom.tezaur-1.0.0", HOME / ".vscode-oss" / "extensions" / "trom.tezaur-1.0.0",
            HOME / ".emacs.d" / "lisp" / "tezaur.el", HOME / ".config" / "emacs" / "lisp" / "tezaur.el",
            HOME / ".local" / "share" / "pandoc" / "filters" / "tezaur.lua", cfg_dir() / "pandoc" / "filters" / "tezaur.lua",
            cfg_dir() / "Notepad++" / "plugins" / "config" / "PythonScript" / "scripts" / "tezaur_npp.py",
            HOME / ".config" / "systemd" / "user" / "tezaur.service", HOME / "Library" / "LaunchAgents" / "org.tezaur.serve.plist"]
    for root in extra_roots["wordpress"]:
        dirs.append(Path(root) / "wp-content" / "plugins" / "tezaur-research")
    for root in extra_roots["dokuwiki"]:
        dirs.append(Path(root) / "lib" / "plugins" / "tezaur")
    for root in extra_roots["mediawiki"]:
        dirs.append(Path(root) / "extensions" / "Tezaur")
    ob = cfg_dir() / "obsidian" / "obsidian.json"
    if ob.exists():
        try:
            for v in read_json(ob).get("vaults", {}).values():
                dirs.append(Path(v["path"]) / ".obsidian" / "plugins" / "tezaur")
        except Exception:
            pass
    for d in dirs:
        if d.exists() or d.is_symlink():
            def rm(d=d):
                if d.is_dir():
                    shutil.rmtree(d, ignore_errors=True)
                else:
                    d.unlink()
                if d.name == "tezaur.service" and SYS == "linux" and not FAKE:
                    run(["systemctl", "--user", "disable", "--now", "tezaur"])
            add("remove %s" % d, rm)
    for vault_cp in ([Path(v["path"]) / ".obsidian" / "community-plugins.json" for v in read_json(ob).get("vaults", {}).values()] if ob.exists() else []):
        if vault_cp.exists() and "tezaur" in vault_cp.read_text("utf-8", "replace"):
            def rm(f=vault_cp):
                lst = [x for x in json.loads(f.read_text("utf-8")) if x != "tezaur"]
                f.write_text(json.dumps(lst), "utf-8")
            add("“tezaur” in %s" % vault_cp, rm)
    claude = which("claude")
    if claude:
        r = run([claude, "mcp", "get", "tezaur"])
        if r.returncode == 0:
            add("Claude Code MCP server “tezaur”", lambda: run([claude, "mcp", "remove", "--scope", "user", "tezaur"]))
    py = which("python3") or which("python")
    if py and not FAKE:
        r = run([py, "-m", "pip", "show", "tezaur-client"])
        if r.returncode == 0:
            add("Python package tezaur-client", lambda: run([py, "-m", "pip", "uninstall", "-y", "tezaur-client"]))
    for d in (HOME / ".tezaur" / "extension", HOME / ".tezaur" / "tools", HOME / ".tezaur" / "to-install"):
        if d.exists():
            add("remove %s" % d, lambda d=d: shutil.rmtree(d, ignore_errors=True))
    if purge:
        for b in (HOME / ".local" / "bin" / "tezaur", data_dir() / "Programs" / "Tezaur"):
            if b.exists():
                add("remove the program %s" % b, lambda b=b: shutil.rmtree(b) if b.is_dir() else b.unlink())
        if (HOME / ".tezaur").exists():
            add("remove %s (including this tool's record)" % (HOME / ".tezaur"), lambda: shutil.rmtree(HOME / ".tezaur", ignore_errors=True))
    if purge_data:
        for d in (data_dir() / "tezaur", data_dir() / "TEZAUR", cfg_dir() / "tezaur"):
            if d.exists():
                add("DELETE YOUR DATA in %s (index, settings, mesh keys)" % d, lambda d=d: shutil.rmtree(d, ignore_errors=True))
    touched = [t[2] for t in mcp_targets()] + [HOME / ".codex" / "config.toml", HOME / ".lyx" / "preferences", cfg_dir() / "lyx" / "preferences",
               HOME / ".config" / "geany" / "filedefs" / "filetypes.latex.conf", HOME / ".emacs.d" / "init.el", HOME / ".config" / "emacs" / "init.el", HOME / ".emacs"]
    if ob.exists():
        try:
            touched += [Path(v["path"]) / ".obsidian" / "community-plugins.json" for v in read_json(ob).get("vaults", {}).values()]
        except Exception:
            pass
    if any(drop for drop in (p.with_name(p.name + ".tezaur-backup").exists() for p in touched)):
        add("delete the one-time config backups (*.tezaur-backup) next to files TEZAUR edited", lambda: [drop_backup(p) for p in touched])
    return acts


# --------------------------------------------------------------------------- remote

def remote_install(host, source, only, yes):
    me = Path(__file__).resolve()
    zip_src = None
    for d in ([Path(source)] if source else [HERE, HERE / "plugins", HERE.parent.parent / "dist"]):
        if d.is_file() and d.suffix == ".zip":
            zip_src = d
        elif (d / "tezaur-plugins.zip").exists():
            zip_src = d / "tezaur-plugins.zip"
        if zip_src:
            break
    if not zip_src:
        raise SystemExit("remote install needs tezaur-plugins.zip (run variants/build-plugins.sh, or pass --source)")
    tmp = "/tmp/tezaur-setup-%d" % int(time.time())
    for cmd in (["ssh", host, "mkdir -p " + tmp], ["scp", "-q", str(me), str(zip_src), "%s:%s/" % (host, tmp)]):
        r = run(cmd, timeout=300)
        if r.returncode != 0:
            raise SystemExit("%s failed: %s" % (cmd[0], (r.stderr or r.stdout).strip()))
    rest = "install --source %s/tezaur-plugins.zip%s%s" % (tmp, " --yes" if yes else "", " --only " + only if only else "")
    r = run(["ssh", host, "python3 %s/tezaur-setup.py %s" % (tmp, rest)], timeout=900)
    say(r.stdout.strip() or r.stderr.strip())
    return r.returncode


# --------------------------------------------------------------------------- main

GROUP_ORDER = ["core", "mcp", "editors", "notes", "notebooks", "writing", "browsers", "cms", "devices", "remote", "peers", "extras"]
GROUP_NAMES = {"core": "Program", "mcp": "AI assistants (MCP)", "editors": "Editors", "notes": "Notes & reference managers", "notebooks": "Notebooks",
               "writing": "Writing tools", "browsers": "Browsers", "cms": "Wikis & CMS", "devices": "Devices", "remote": "Remote machines (ssh)",
               "peers": "Mesh peers found on the network", "extras": "Extras"}
MARK = {"ready": "●", "manual": "○", "missing": "✗", "root": "!", "absent": "✓"}


def main(argv=None):
    ap = argparse.ArgumentParser(prog="tezaur-setup", description="TEZAUR meta-installer: detect applications and devices, install the matching integrations.")
    ap.add_argument("command", nargs="?", default="scan", choices=["scan", "install", "status", "uninstall"])
    ap.add_argument("--yes", "-y", action="store_true", help="do everything that is ready without asking")
    ap.add_argument("--all", action="store_true", help="include opt-in extras (background service, mesh peers)")
    ap.add_argument("--only", default="", help="comma list of item ids or group names")
    ap.add_argument("--skip", default="", help="comma list of item ids or group names to leave alone")
    ap.add_argument("--source", help="folder or zip made by build-plugins.sh")
    ap.add_argument("--bin", help="path to the tezaur binary")
    ap.add_argument("--index", help="index file to put into MCP configs (TEZAUR_INDEX)")
    ap.add_argument("--json", action="store_true", help="machine-readable scan")
    ap.add_argument("--wordpress", action="append", default=[], metavar="DIR", help="a WordPress root (repeatable)")
    ap.add_argument("--dokuwiki", action="append", default=[], metavar="DIR")
    ap.add_argument("--mediawiki", action="append", default=[], metavar="DIR")
    ap.add_argument("--usb", action="append", default=[], metavar="DIR", help="a drive or folder to treat as a PortableApps drive")
    ap.add_argument("--sweep", action="store_true", help="uninstall: also find and remove leftovers without relying on the record (hand installs, a lost record)")
    ap.add_argument("--purge", action="store_true", help="uninstall: also remove the tezaur program and ~/.tezaur (this tool's own folder)")
    ap.add_argument("--purge-data", action="store_true", help="uninstall: ALSO delete TEZAUR's data folders (index, settings, mesh identity). Your Library of PDFs is never touched.")
    ap.add_argument("--dry-run", action="store_true", help="uninstall: list what would be removed")
    ap.add_argument("--remote", action="append", default=[], metavar="HOST", help="install on another machine over ssh (user@host)")
    a = ap.parse_args(argv)
    only = [x for x in a.only.split(",") if x]
    skip = [x for x in a.skip.split(",") if x]

    led = Ledger()
    if a.command == "status":
        if not led.data["items"]:
            say("Nothing installed by tezaur-setup.")
        for k, v in led.data["items"].items():
            say("%s  (%d change%s)" % (k, len(v), "" if len(v) == 1 else "s"))
        return 0
    if a.command == "uninstall":
        extra = {"wordpress": a.wordpress, "dokuwiki": a.dokuwiki, "mediawiki": a.mediawiki}
        if a.dry_run:
            for item, recs in led.data["items"].items():
                if not only or any(item == o or item.startswith(o) for o in only):
                    say("  would undo %s (%d change%s)" % (item, len(recs), "" if len(recs) == 1 else "s"))
            if a.sweep or a.purge or a.purge_data:
                for d, _ in sweep_plan(extra, a.purge, a.purge_data):
                    say("  would " + d)
            return 0
        if a.purge_data and not a.yes:
            say("--purge-data deletes your TEZAUR index and settings. Add --yes to confirm.")
            return 1
        n = uninstall(led, only)
        say("%d recorded item(s) removed." % n)
        if a.sweep or a.purge or a.purge_data:
            m = 0
            for d, f in sweep_plan(extra, a.purge, a.purge_data):
                try:
                    f()
                    m += 1
                    say("  swept: " + d)
                except Exception as e:  # keep going: a half-removed state is worse than a reported failure
                    say("  could not %s: %s" % (d, e))
            say("%d leftover(s) swept." % m)
        return 0
    if a.remote:
        rc = 0
        for h in a.remote:
            say("→ " + h)
            rc |= remote_install(h, a.source, a.only, a.yes) or 0
        return rc

    bundle = Bundle(a.source)
    try:
        binary = find_binary(a.bin, bundle)
        ctx = Ctx(bundle, binary, a.index, led, a)
        items = scan(ctx)
        if a.json:
            print(json.dumps({"os": SYS, "binary": binary, "model_servers": ctx.models, "items": [i.row() for i in items]}, indent=1))
            return 0
        say("TEZAUR setup %s — %s, home %s" % (VERSION, SYS, HOME))
        say("tezaur binary: %s" % (binary or "NOT FOUND"))
        say("model servers: %s" % (", ".join(ctx.models) if ctx.models else "none answering (start Ollama / Jan / LM Studio to get answers; search works without)"))
        say("bundle: %s" % (", ".join(str(d) for d in bundle.dirs) or "NOT FOUND — pass --source"))
        for g in GROUP_ORDER:
            rows = [i for i in items if i.group == g]
            if not rows:
                continue
            say("\n" + GROUP_NAMES[g])
            for i in rows:
                say("  %s %-52s %s%s" % (MARK[i.state], i.title, i.detail, "   [opt-in]" if i.optin else ""))
                if i.hint and a.command == "scan":
                    say("      ↳ " + i.hint)
        if not items:
            say("\nNothing to install here.")
        say("\n● ready  ○ needs a click of yours  ! needs sudo  ✗ artifact missing  ✓ already done")
        if a.command == "scan":
            n = sum(1 for i in items if i.state == "ready" and not i.optin)
            say("%d ready. Run `tezaur-setup install` to apply (nothing has been changed)." % n)
            return 0

        done, failed, manual = [], [], []
        for g in GROUP_ORDER:
            rows = [i for i in items if i.group == g and i.state in ("ready", "manual") and i.apply]
            rows = [i for i in rows if (not only or any(i.id == o or i.id.startswith(o + ":") or i.group == o for o in only))
                    and not any(i.id == s or i.id.startswith(s + ":") or i.group == s for s in skip)
                    and (not i.optin or a.all or only)]
            if not rows:
                continue
            if not a.yes:
                ans = input("\nInstall %s (%d)? [Y/n] " % (GROUP_NAMES[g], len(rows))).strip().lower() if sys.stdin.isatty() else "n"
                if ans not in ("", "y", "yes"):
                    continue
            for i in rows:
                try:
                    i.apply(ctx)
                    (manual if i.state == "manual" else done).append(i)
                    say("  %s %s" % ("→" if i.state == "manual" else "✓", i.title))
                except (RuntimeError, OSError, ValueError, zipfile.BadZipFile) as e:
                    failed.append((i, str(e)))
                    say("  ✗ %s: %s" % (i.title, e))
        say("\n%d installed, %d staged for you, %d failed." % (len(done), len(manual), len(failed)))
        for i in manual:
            say("  ○ %s — %s" % (i.title, i.hint))
        for i in done:
            if i.hint:
                say("  • %s — %s" % (i.title, i.hint))
        say("Undo everything with: tezaur-setup uninstall")
        return 1 if failed else 0
    finally:
        bundle.cleanup()


if __name__ == "__main__":
    sys.exit(main())
