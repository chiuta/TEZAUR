#!/usr/bin/env python3
"""End-to-end checks of audit / coverage / novelty / receipts / declare, through the CLI, the HTTP API and MCP.
Needs: a running `tezaur serve` ($TEZAUR_SERVER), the mock model, and $TEZAUR_INDEX pointing at the same index."""
import json, os, subprocess, sys, tempfile, urllib.request

BIN = os.environ.get("TEZAUR_BIN") or os.path.join(os.path.dirname(__file__), "..", "..", "target", "release", "tezaur")
SRV = os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8810")
ok = True
def check(name, cond, extra=""):
    global ok
    print(("PASS " if cond else "FAIL ") + name + (" — " + str(extra) if not cond else ""))
    ok = ok and bool(cond)
def run(*a, **k):
    return subprocess.run([BIN, *a], capture_output=True, text=True, stdin=subprocess.DEVNULL, **k)
def post(path, body):
    r = urllib.request.Request(SRV + path, json.dumps(body).encode(), {"Content-Type": "application/json"})
    return json.load(urllib.request.urlopen(r))

d = tempfile.mkdtemp()
draft = os.path.join(d, "draft.md")
open(draft, "w").write(
    "# Intro\nThe burden of proof lies with the body assessing the qualification [@lisbon_convention]. "
    "Penguins sing opera in Antarctica every single winter morning loudly [@lisbon_convention]. "
    "Someone cited a ghost [@nobody2020]. The convention says \"The burden of proof lies with the body assessing the qualification\" [@lisbon_convention]. "
    "It also says \"zebras may vote in every national election held\" [@lisbon_convention].\n")

# CLI audit
r = run("audit", draft, "--json")
f = json.loads(r.stdout)
v = [x["verdict"] for x in f]
for want in ("supported", "unsupported", "unknown-source", "quote-verbatim", "quote-not-found"):
    check("audit verdict " + want, want in v, v)
check("audit --fail exits 2 on problems", run("audit", draft, "--fail").returncode == 2)

# CLI ask --receipt, verify, tamper, declare
rc = os.path.join(d, "r.json"); ans = os.path.join(d, "a.txt")
r = run("ask", "Who bears the burden of proof?", "--receipt", rc)
check("ask writes receipt", os.path.exists(rc), r.stderr[-300:])
if os.path.exists(rc):
    rec = json.load(open(rc))
    open(ans, "w").write(r.stdout.split("\n## References")[0])
    check("receipt verifies", run("receipt", rc).returncode == 0, run("receipt", rc).stdout)
    rec2 = dict(rec); rec2["question"] = "forged"; bad = os.path.join(d, "bad.json"); json.dump(rec2, open(bad, "w"))
    check("tampered receipt fails", run("receipt", bad).returncode == 2)
    dec = run("declare", rc, "--lang", "ro").stdout
    check("declare (ro) mentions receipts", "Declarație" in dec and rec["digest"] in dec, dec[:200])
    os.environ["TEZAUR_RECEIPT_KEY"] = "secret"
    r2 = os.path.join(d, "r2.json"); run("ask", "burden of proof", "--receipt", r2)
    check("HMAC present when key set", "hmac" in json.load(open(r2)))
    check("HMAC verifies with key", run("receipt", r2).returncode == 0)
    os.environ["TEZAUR_RECEIPT_KEY"] = "other"
    check("HMAC fails with wrong key", run("receipt", r2).returncode == 2)
    del os.environ["TEZAUR_RECEIPT_KEY"]

# coverage + novelty CLI
o = os.path.join(d, "o.md"); open(o, "w").write("# Recognition of qualifications\n# Quantum gravity\n")
cv = json.loads(run("coverage", o, "--json").stdout)
st = {x["heading"]: x["status"] for x in cv["rows"]}
check("coverage statuses", st.get("Quantum gravity") == "empty" and st.get("Recognition of qualifications") in ("thin", "well-covered"), st)
nv = json.loads(run("novelty", draft, "--json").stdout)
check("novelty returns passages", len(nv["passages"]) >= 1)

# HTTP API
a = post("/api/audit", {"text": open(draft).read()})
check("API audit", a["problems"] >= 3 and "markdown" in a, a.get("problems"))
c = post("/api/coverage", {"text": "# Quantum gravity\n# Recognition of qualifications"})
check("API coverage", {x["status"] for x in c["rows"]} >= {"empty"})
n = post("/api/novelty", {"text": "Penguins sing opera in Antarctica every single winter morning loudly."})
check("API novelty", n["new"] >= 1)
q = post("/api/ask", {"q": "Who bears the burden of proof?"})
check("API ask carries a receipt", q.get("receipt", {}).get("format") == "tezaur-receipt/1")
vr = post("/api/receipt", {"receipt": q["receipt"], "answer": q["answer"]})
check("API receipt verifies", vr["ok"], vr)
vr2 = post("/api/receipt", {"receipt": q["receipt"], "answer": q["answer"] + "!"})
check("API receipt rejects edited answer", not vr2["ok"])

# MCP
msgs = "\n".join(json.dumps(m) for m in [
    {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
    {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
    {"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "tezaur_audit", "arguments": {"text": open(draft).read()}}},
    {"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "tezaur_coverage", "arguments": {"outline": "# Quantum gravity"}}},
]) + "\n"
out = [json.loads(l) for l in subprocess.run([BIN, "mcp"], input=msgs, capture_output=True, text=True).stdout.splitlines()]
names = [t["name"] for t in out[1]["result"]["tools"]]
check("MCP lists new tools", "tezaur_audit" in names and "tezaur_coverage" in names, names)
check("MCP audit text", "unsupported" in out[2]["result"]["content"][0]["text"])
check("MCP coverage text", "empty" in out[3]["result"]["content"][0]["text"])
# MCP over HTTP, OpenAPI, DNS-rebinding guard
import http.client
u = SRV.split("//")[1]; h, pt = u.split(":")
def raw(method, path, body=None, host=None):
    c = http.client.HTTPConnection(h, int(pt), timeout=30); c.putrequest(method, path, skip_host=bool(host))
    if host: c.putheader("Host", host)
    if body is not None: c.putheader("Content-Length", str(len(body)))
    c.endheaders(body); r = c.getresponse(); return r.status, r.read()
st, b = raw("POST", "/mcp", json.dumps({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}).encode())
check("HTTP MCP lists tools", st == 200 and b"tezaur_audit" in b, b[:100])
st, b = raw("POST", "/mcp", json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "tezaur_coverage", "arguments": {"outline": "# Quantum gravity"}}}).encode())
check("HTTP MCP tool call", st == 200 and b"empty" in b, b[:200])
st, b = raw("POST", "/mcp", json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}).encode())
check("HTTP MCP notification gets 202", st == 202)
st, b = raw("GET", "/openapi.json")
check("openapi.json served", st == 200 and json.loads(b)["openapi"].startswith("3."))
st, b = raw("GET", "/api/health", host="evil.example.com:8810")
check("foreign Host header refused without a token (DNS rebinding)", st == 403, st)
st, b = raw("GET", "/api/health", host="localhost:8810")
check("localhost Host accepted", st == 200)
print("OK" if ok else "FAILED"); sys.exit(0 if ok else 1)
