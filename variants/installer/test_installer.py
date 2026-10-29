#!/usr/bin/env python3
"""Runs tezaur-setup against a fake home directory full of fake applications and checks scan / install / idempotence / uninstall.
Needs the bundle from variants/build-plugins.sh (dist/plugins)."""
import json, os, shutil, stat, subprocess, sys, tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
SETUP = HERE / "tezaur-setup.py"
BUNDLE = HERE.parent.parent / "dist" / "plugins"
BIN = HERE.parent.parent / "target" / "release" / "tezaur"
ok = True
def check(name, cond, extra=""):
    global ok
    print(("PASS " if cond else "FAIL ") + name + ("" if cond else " — " + str(extra))); ok = ok and bool(cond)

home = Path(tempfile.mkdtemp()); shims = home / "shims"; shims.mkdir(); log = home / "calls.log"
def shim(name, body=""):
    p = shims / name
    p.write_text('#!/bin/sh\necho "%s $@" >> %s\n%s\n' % (name, log, body)); p.chmod(p.stat().st_mode | stat.S_IEXEC)
for n in ("nvim", "emacs", "pandoc", "systemctl", "jupyter"): shim(n)
shim("claude")
shim("python3", 'exit 0')
shim("adb", 'if [ "$1" = devices ]; then echo "List of devices attached"; echo "ABC123\tdevice"; fi')
env = dict(os.environ, TEZAUR_SETUP_HOME=str(home), PATH=str(shims) + os.pathsep + "/usr/bin:/bin", HOME=str(home))

# a lived-in machine
(home / ".config/Claude").mkdir(parents=True)
(home / ".config/Claude/claude_desktop_config.json").write_text(json.dumps({"mcpServers": {"other": {"command": "x"}}, "theme": "dark"}))
(home / ".cursor").mkdir(); (home / ".vim").mkdir()
(home / ".config/nvim").mkdir(parents=True)
(home / ".emacs.d").mkdir(); (home / ".emacs.d/init.el").write_text(";; my config\n(setq x 1)\n")
(home / ".config/sublime-text/Packages").mkdir(parents=True)
vault = home / "Notes"; (vault / ".obsidian").mkdir(parents=True)
(vault / ".obsidian/community-plugins.json").write_text('["dataview"]')
(home / ".config/obsidian").mkdir(parents=True)
(home / ".config/obsidian/obsidian.json").write_text(json.dumps({"vaults": {"a": {"path": str(vault)}}}))
(home / ".mozilla").mkdir()
wp = home / "site"; (wp / "wp-content/plugins").mkdir(parents=True)
usb = home / "usb"; (usb / "PortableApps").mkdir(parents=True)
(home / ".ssh").mkdir(); (home / ".ssh/config").write_text("Host pi\n  HostName 10.0.0.5\nHost *\n  ForwardAgent no\n")
(home / "Zotero").mkdir()
for d in (".gemini", ".lmstudio", ".codex", ".continue", ".lyx", ".config/geany", ".config/goose", ".config/zed", ".config/texstudio"):
    (home / d).mkdir(parents=True, exist_ok=True)
(home / ".config/zed/settings.json").write_text('// my zed settings\n{ "theme": "One" }\n')
(home / ".codex/config.toml").write_text('model = "gpt-5"\n')
(home / ".lyx/preferences").write_text('\\screen_dpi 96\n')
cl = home / ".config/Code/User/globalStorage/saoudrizwan.claude-dev/settings"; cl.mkdir(parents=True)

def setup(*a):
    return subprocess.run([sys.executable, str(SETUP), "--source", str(BUNDLE), "--bin", str(BIN), *a, "--wordpress", str(wp), "--usb", str(usb)], env=env, capture_output=True, text=True, stdin=subprocess.DEVNULL)

r = setup("scan", "--json")
scan = json.loads(r.stdout)
ids = {i["id"]: i for i in scan["items"]}
for want in ("gemini-cli", "lmstudio", "codex", "continue", "cline", "zed", "goose", "lyx", "geany", "texstudio", "claude-desktop", "cursor", "claude-code", "vim", "neovim", "emacs", "sublime", "obsidian:Notes", "browser-firefox", "pandoc", "jupyter", "service", "zotero", "ssh:pi", "android:ABC123"):
    check("scan finds " + want, want in ids, sorted(ids))
check("Zed with comments is manual, not clobbered", ids["zed"]["state"] == "manual")
check("scan finds the WordPress folder", any(k.startswith("wordpress:") for k in ids))
check("scan finds the PortableApps drive", any(k.startswith("usb:") for k in ids))
check("scan changed nothing", not (home / ".tezaur/installed.json").exists() and not (home / ".cursor/mcp.json").exists())
check("Chrome absent -> no Chrome item", "browser-chrome" not in ids)
check("android without apk is reported missing", ids["android:ABC123"]["state"] == "missing")

r = setup("install", "--yes")
check("install exits 0", r.returncode == 0, r.stdout[-600:] + r.stderr[-300:])
cd = json.loads((home / ".config/Claude/claude_desktop_config.json").read_text())
check("Claude Desktop: tezaur added, others kept", cd["mcpServers"]["tezaur"]["args"] == ["mcp"] and "other" in cd["mcpServers"] and cd["theme"] == "dark")
check("Claude Desktop: backup written", (home / ".config/Claude/claude_desktop_config.json.tezaur-backup").exists())
check("Cursor config created", json.loads((home / ".cursor/mcp.json").read_text())["mcpServers"]["tezaur"]["command"].endswith("tezaur"))
check("Vim pack installed", (home / ".vim/pack/tezaur/start/tezaur/plugin").exists())
check("Neovim pack installed", (home / ".local/share/nvim/site/pack/tezaur/start/tezaur/lua").exists())
init = (home / ".emacs.d/init.el").read_text()
check("Emacs: block appended, own config kept", "(require 'tezaur)" in init and "(setq x 1)" in init and (home / ".emacs.d/lisp/tezaur.el").exists())
check("Sublime package", (home / ".config/sublime-text/Packages/TEZAUR/tezaur.py").exists())
check("Obsidian plugin + enabled", (vault / ".obsidian/plugins/tezaur/main.js").exists() and "tezaur" in json.loads((vault / ".obsidian/community-plugins.json").read_text()))
check("Firefox extension unpacked", (home / ".tezaur/extension/firefox/manifest.json").exists())
check("Pandoc filter", (home / ".local/share/pandoc/filters/tezaur.lua").exists())
check("WordPress plugin copied", (wp / "wp-content/plugins/tezaur-research").is_dir())
check("PortableApps copy", (usb / "PortableApps/TezaurPortable").is_dir())
check("Zotero xpi staged", (home / ".tezaur/to-install/tezaur-zotero.xpi").exists())
check("Gemini CLI entry", json.loads((home / ".gemini/settings.json").read_text())["mcpServers"]["tezaur"]["args"] == ["mcp"])
check("LM Studio entry", "tezaur" in json.loads((home / ".lmstudio/mcp.json").read_text())["mcpServers"])
check("Continue file", "tezaur" in json.loads((home / ".continue/mcpServers/tezaur.json").read_text())["mcpServers"])
check("Cline entry", "tezaur" in json.loads((cl / "cline_mcp_settings.json").read_text())["mcpServers"])
cx = (home / ".codex/config.toml").read_text()
import tomllib
tc = tomllib.loads(cx)
check("Codex TOML block parses and keeps model", tc["model"] == "gpt-5" and tc["mcp_servers"]["tezaur"]["args"] == ["mcp"], cx)
check("Zed settings untouched", (home / ".config/zed/settings.json").read_text().startswith("// my zed settings"))
lp = (home / ".lyx/preferences").read_text()
check("LyX converter block, own settings kept", "tezaur_tex.py audit" in lp and "\\screen_dpi 96" in lp and (home / ".tezaur/tools/latex/tezaur_tex.py").exists(), lp)
check("Geany build entry", "TEZAUR audit" in (home / ".config/geany/filedefs/filetypes.latex.conf").read_text())
check("LSP snippets point at the real tools dir", str(home) in (home / ".tezaur/tools/latex/lsp-configs/helix-languages.toml").read_text())
calls = log.read_text()
check("claude mcp add was called", "claude mcp add --scope user tezaur" in calls, calls)
check("pip install of the wheel was called", "-m pip install" in calls and "tezaur_client" in calls, calls)
check("service NOT installed without --all", not (home / ".config/systemd/user/tezaur.service").exists())

before = (home / ".emacs.d/init.el").read_text()
r2 = setup("install", "--yes")
check("second run is idempotent", (home / ".emacs.d/init.el").read_text() == before and r2.returncode == 0)

r = setup("install", "--yes", "--all", "--only", "service")
check("--all --only service installs the unit", (home / ".config/systemd/user/tezaur.service").exists(), r.stdout)

r = setup("status")
check("status lists items", "claude-desktop" in r.stdout and "emacs" in r.stdout)
r = setup("uninstall")
check("uninstall exits 0", r.returncode == 0, r.stdout + r.stderr)
cd = json.loads((home / ".config/Claude/claude_desktop_config.json").read_text())
check("Claude Desktop restored", "tezaur" not in cd["mcpServers"] and "other" in cd["mcpServers"])
check("Cursor entry removed", "tezaur" not in json.loads((home / ".cursor/mcp.json").read_text()).get("mcpServers", {}))
check("Emacs init restored exactly", (home / ".emacs.d/init.el").read_text().strip() == ";; my config\n(setq x 1)".strip(), (home / ".emacs.d/init.el").read_text())
check("Vim pack removed", not (home / ".vim/pack/tezaur").exists())
check("Obsidian plugin removed and list restored", not (vault / ".obsidian/plugins/tezaur").exists() and json.loads((vault / ".obsidian/community-plugins.json").read_text()) == ["dataview"])
check("WordPress plugin removed", not (wp / "wp-content/plugins/tezaur-research").exists())
check("PortableApps copy removed", not (usb / "PortableApps/TezaurPortable").exists())
check("service unit removed", not (home / ".config/systemd/user/tezaur.service").exists())
calls = log.read_text()
check("claude mcp remove + pip uninstall were called", "claude mcp remove" in calls and "pip uninstall" in calls, calls)

# --- complete removal: lose the record, then sweep
setup("install", "--yes")
(home / ".tezaur/installed.json").unlink()
r = setup("uninstall", "--dry-run", "--sweep")
check("dry-run lists leftovers and changes nothing", "would" in r.stdout and (home / ".gemini/settings.json").exists() and "tezaur" in (home / ".gemini/settings.json").read_text(), r.stdout)
(home / ".gemini/settings.json").write_text(json.dumps({"mcpServers": {"tezaur": {"command": "somebody-elses-tool"}, "other": {"command": "x"}}}))
r = setup("uninstall", "--sweep", "--purge")
check("sweep exits 0", r.returncode == 0, r.stdout + r.stderr)
check("sweep spares a 'tezaur' key that is not ours", "somebody-elses-tool" in (home / ".gemini/settings.json").read_text())
check("sweep removed MCP entries", "tezaur" not in json.loads((home / ".lmstudio/mcp.json").read_text()).get("mcpServers", {}) and not (home / ".continue/mcpServers/tezaur.json").exists())
check("sweep removed Codex block, kept the rest", "tezaur" not in (home / ".codex/config.toml").read_text() and 'model = "gpt-5"' in (home / ".codex/config.toml").read_text())
check("sweep removed LyX/Emacs blocks", "tezaur" not in (home / ".lyx/preferences").read_text() and "tezaur" not in (home / ".emacs.d/init.el").read_text())
check("sweep removed packs, plugins, tools", not (home / ".vim/pack/tezaur").exists() and not (wp / "wp-content/plugins/tezaur-research").exists() and not (vault / ".obsidian/plugins/tezaur").exists())
check("purge removed the program and ~/.tezaur", not (home / ".local/bin/tezaur").exists() and not (home / ".tezaur").exists())
check("no .tezaur-backup litter left behind", not list(home.rglob("*.tezaur-backup")), list(home.rglob("*.tezaur-backup")))
check("user's own settings still there", (home / ".config/zed/settings.json").read_text().startswith("// my zed") and "screen_dpi" in (home / ".lyx/preferences").read_text())
os.makedirs(home / ".local/share/tezaur"); (home / ".local/share/tezaur/tezaur.idx").write_text("x")
r = setup("uninstall", "--purge-data")
check("--purge-data without --yes refuses", r.returncode == 1 and (home / ".local/share/tezaur/tezaur.idx").exists())
r = setup("uninstall", "--purge-data", "--yes")
check("--purge-data --yes deletes data folder", not (home / ".local/share/tezaur").exists(), r.stdout)

# remote: ssh/scp shims record what would run
shim("ssh"); shim("scp")
log.write_text("")
r = subprocess.run([sys.executable, str(SETUP), "--source", str(BUNDLE / ".." / "tezaur-plugins.zip"), "install", "--yes", "--remote", "me@pi"], env=env, capture_output=True, text=True, stdin=subprocess.DEVNULL)
c = log.read_text()
check("remote: scp then ssh run installer", "scp -q" in c and "tezaur-setup.py install --source" in c and "me@pi" in c, c + r.stdout + r.stderr)
shutil.rmtree(home, ignore_errors=True)
print("OK" if ok else "FAILED"); sys.exit(0 if ok else 1)
