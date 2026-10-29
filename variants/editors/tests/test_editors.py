"""Runs every editor integration against a live `tezaur serve` (TEZAUR_SERVER, default http://127.0.0.1:8810) with a mock model."""
import importlib.util, json, os, subprocess, sys, tempfile, types, zipfile, plistlib
from pathlib import Path

HERE = Path(__file__).resolve().parent.parent
SERVER = os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8810")
os.environ["TEZAUR_SERVER"] = SERVER
fails = []


def check(cond, msg):
    print(("ok   " if cond else "FAIL ") + msg)
    if not cond:
        fails.append(msg)


def load(path, name, mocks=None):
    for k, v in (mocks or {}).items():
        sys.modules[k] = v
    spec = importlib.util.spec_from_file_location(name, path)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


# ---- Vim (real vim, headless)
def vim_run(mode, q):
    out = tempfile.mktemp()
    cmd = ["vim", "-es", "-u", "NONE", "-N", "--cmd", "set rtp+=%s" % (HERE / "vim"), "--cmd", "let g:tezaur_server='%s'" % SERVER,
           "-c", "runtime plugin/tezaur.vim", "-c", "call tezaur#run('%s','%s')" % (mode, q),
           "-c", "call writefile(tezaur#last(), '%s')" % out, "-c", "q!"]
    subprocess.run(cmd, timeout=60, capture_output=True, stdin=subprocess.DEVNULL)
    return Path(out).read_text() if Path(out).exists() else ""
txt = vim_run("ask", "burden of proof")
check("burden of proof lies" in txt and "## References" in txt and "\u26a0" in txt, "vim: TezaurAsk answer + references + warning")
check("[1]" in vim_run("search", "burden"), "vim: TezaurSearch lists passages")

# ---- Neovim Lua through lupa (mock of the vim API; vim.system really calls curl)
try:
    import lupa
    L = lupa.LuaRuntime(unpack_returned_tuples=True)
    notes = []

    def system(cmd, opts, cb):
        cmd = [cmd[i] for i in sorted(cmd.keys())]
        p = subprocess.run(cmd, input=opts["stdin"], capture_output=True, text=True)
        cb(L.table(code=p.returncode, stdout=p.stdout))
    L.globals().PYSYS = system
    L.globals().PYNOTE = lambda *a: notes.append(a)
    L.execute("""
      vim = { log={levels={WARN=1}}, notify=function(...) PYNOTE(...) end,
        tbl_extend=function(_, a, b) local r={} for k,v in pairs(a) do r[k]=v end for k,v in pairs(b or {}) do r[k]=v end return r end,
        trim=function(s) return (s:gsub('^%s+',''):gsub('%s+$','')) end,
        list_extend=function(a,b) for _,v in ipairs(b) do a[#a+1]=v end return a end,
        schedule_wrap=function(f) return f end, system=function(c,o,cb) PYSYS(c,o,cb) end,
        json={ encode=function(t) return PYJSON(t) end, decode=function(s) return PYDEC(s) end } }
    """)
    def enc(t):
        def conv(x):
            if lupa.lua_type(x) == "table":
                return {k: conv(v) for k, v in x.items()}
            return x
        return json.dumps(conv(t))
    def dec(s):
        def conv(x):
            if isinstance(x, dict):
                return L.table_from({k: conv(v) for k, v in x.items()})
            if isinstance(x, list):
                return L.table_from([conv(v) for v in x])
            return x
        return conv(json.loads(s))
    L.globals().PYJSON, L.globals().PYDEC = enc, dec
    L.execute("package.path = '%s/neovim/lua/?/init.lua;' .. package.path" % HERE)
    M = L.eval("(require('tezaur'))")
    M.setup(L.table(server=SERVER))
    got = {}
    M.run("ask", "burden of proof", lambda lines, d: got.update(lines=[lines[i] for i in sorted(lines.keys())]))
    joined = "\n".join(got.get("lines", []))
    check("burden of proof lies" in joined and "## References" in joined, "neovim: run() via vim.system -> curl -> formatted lines")
    f = M.format("search", dec(json.dumps({"hits": [{"n": 1, "label": "L", "text": "a   b"}]})))
    check(f[1] == "[1] L" and f[2].strip() == "a b", "neovim: format(search)")
except ImportError:
    print("skip neovim (pip install lupa)")

# ---- Sublime
sub = types.ModuleType("sublime"); sp = types.ModuleType("sublime_plugin")
sp.TextCommand = type("TextCommand", (), {})
m = load(HERE / "sublime/tezaur.py", "tz_sublime", {"sublime": sub, "sublime_plugin": sp})
d = m.call(SERVER, None, "ask", "burden of proof", 4)
check("burden of proof lies" in m.format_result("ask", d), "sublime: call + format_result")
check(m.format_result("ask", m.call("http://127.0.0.1:1", None, "ask", "x", 1)).startswith("Error: cannot reach"), "sublime: unreachable server message")

# ---- Notepad++ PythonScript
m = load(HERE / "notepadpp/tezaur_npp.py", "tz_npp")
class Ed:
    def __init__(s, sel=""): s.sel, s.buf = sel, ""
    def getSelText(s): return s.sel
    def documentEnd(s): pass
    def setCurrentPos(s, p): pass
    def getSelectionEnd(s): return 0
    def addText(s, t): s.buf += t
class Np:
    def prompt(s, *a): return "burden of proof"
e = Ed(); m.main(e, Np())
check("burden of proof lies" in e.buf, "notepad++: script inserts answer (prompt path)")
e = Ed("burden"); m.main(e, Np(), "search")
check("[1]" in e.buf, "notepad++: selection used as the question, search mode")

# ---- Joplin + Logseq (node)
js = f"""
const j = require('{HERE}/joplin/src/index.js'), l = require('{HERE}/logseq/index.js');
(async () => {{
  const S = {{server: '{SERVER}', token: '', k: 4}};
  const d = await j.call(S, 'ask', 'burden of proof');
  const t = j.format('ask', d);
  if (!/burden of proof lies/.test(t) || !/References/.test(t)) throw new Error('joplin');
  const b = l.toBlocks(await l.call(S, 'ask', 'burden of proof'));
  if (b.length < 3) throw new Error('logseq blocks ' + b.length);
  const h = l.hitsToBlocks(await l.call(S, 'search', 'burden'));
  if (!/^\\[1\\]/.test(h[0])) throw new Error('logseq search');
  const bad = j.format('ask', await j.call({{server:'http://127.0.0.1:1',k:1}}, 'ask', 'x'));
  if (!/cannot reach/.test(bad)) throw new Error('unreachable');
  console.log('node ok');
}})().catch(e => {{ console.error('NODEFAIL', e.message); process.exit(1); }});
"""
r = subprocess.run(["node", "-e", js], capture_output=True, text=True)
check(r.returncode == 0, "joplin + logseq: call/format against live server " + r.stderr.strip()[:80])

# ---- Raycast
env = dict(os.environ, TEZAUR_SERVER=SERVER)
r = subprocess.run([str(HERE / "raycast/tezaur-ask.sh"), "burden of proof"], capture_output=True, text=True, env=env, timeout=60)
check("burden of proof lies" in r.stdout, "raycast: ask script")
r = subprocess.run([str(HERE / "raycast/tezaur-search.sh"), "burden"], capture_output=True, text=True, env=env, timeout=60)
check(r.stdout.startswith("[1]"), "raycast: search script")

# ---- Alfred
r = subprocess.run([sys.executable, str(HERE / "alfred/tezaur_filter.py"), "burden", "of", "proof"], capture_output=True, text=True, env=env)
items = json.loads(r.stdout)["items"]
check(len(items) >= 1 and "arg" in items[0], "alfred: script filter JSON")
z = tempfile.mktemp(suffix=".alfredworkflow")
subprocess.run([sys.executable, str(HERE / "alfred/build.py"), z], check=True, capture_output=True)
with zipfile.ZipFile(z) as zf:
    pl = plistlib.loads(zf.read("info.plist"))
    check("tezaur_filter.py" in zf.namelist() and pl["bundleid"] == "org.tezaur.alfred" and len(pl["objects"]) == 2, "alfred: workflow bundle is a valid zip + plist")

# ---- LSP
def rpc(p, msg):
    b = json.dumps(msg).encode()
    p.stdin.write(b"Content-Length: %d\r\n\r\n" % len(b) + b); p.stdin.flush()
def rd(p):
    n = 0
    while True:
        h = p.stdout.readline().strip()
        if not h: break
        if h.lower().startswith(b"content-length:"): n = int(h.split(b":")[1])
    return json.loads(p.stdout.read(n))
p = subprocess.Popen([sys.executable, str(HERE / "lsp/tezaur_lsp.py")], stdin=subprocess.PIPE, stdout=subprocess.PIPE, env=env)
rpc(p, {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}); init = rd(p)
check(init["result"]["capabilities"]["completionProvider"]["triggerCharacters"] == ["@"], "lsp: initialize")
uri = "file:///t.md"
rpc(p, {"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument": {"uri": uri, "text": "See [@lisbon_convention; @nope].\nAlso [@dir"}}})
diag = rd(p)["params"]["diagnostics"]
check(len(diag) == 1 and "nope" in diag[0]["message"], "lsp: flags unknown key only")
rpc(p, {"jsonrpc": "2.0", "id": 2, "method": "textDocument/completion", "params": {"textDocument": {"uri": uri}, "position": {"line": 1, "character": 10}}})
items = rd(p)["result"]["items"]
check([i["label"] for i in items] == ["directive_notes"] and "Smith" in items[0]["detail"], "lsp: completes `[@dir` -> directive_notes with citation")
rpc(p, {"jsonrpc": "2.0", "id": 3, "method": "textDocument/hover", "params": {"textDocument": {"uri": uri}, "position": {"line": 0, "character": 10}}})
hv = rd(p)["result"]
check(hv and "Council of Europe" in hv["contents"]["value"], "lsp: hover shows the citation")
rpc(p, {"jsonrpc": "2.0", "method": "textDocument/didSave", "params": {"textDocument": {"uri": uri}, "text": "Fine intro.\nPenguins sing opera in Antarctica every single winter morning loudly [@lisbon_convention]."}})
sd = rd(p)["params"]["diagnostics"]
check(any(d["source"] == "tezaur-audit" and d["range"]["start"]["line"] == 1 and "unsupported" in d["message"] for d in sd), "lsp: audit on save flags the unsupported sentence on its line")
p.stdin.close(); p.wait(timeout=5)

print("\n%d failed" % len(fails) if fails else "\nall editor tests passed")
sys.exit(1 if fails else 0)
