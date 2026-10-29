"""Adversarial regression tests: CSRF on the token-less local server, DNS rebinding, zip bombs, figure swaps."""
import os, subprocess, sys, tempfile, time, urllib.request, zipfile
B = os.environ.get("TEZAUR_BIN", os.path.join(os.path.dirname(__file__), "..", "..", "target", "release", "tezaur"))
tmp = tempfile.mkdtemp(); env = dict(os.environ, TEZAUR_INDEX=tmp + "/t.idx", HOME=tmp, NO_COLOR="1")
fails = []
def check(name, ok):
    print(("ok   " if ok else "FAIL ") + name); (None if ok else fails.append(name))
def req(url, method="GET", headers=None, data=None):
    r = urllib.request.Request(url, data=data, method=method, headers=headers or {})
    try: return urllib.request.urlopen(r, timeout=10).status
    except urllib.error.HTTPError as e: return e.code
os.makedirs(tmp + "/lib")
open(tmp + "/lib/a.txt", "w").write("The mean processing time was 71.3 days in Germany.")
subprocess.run([B, "ingest", tmp + "/lib"], env=env, capture_output=True)
p = subprocess.Popen([B, "serve", "--port", "8899"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL); time.sleep(1.5)
try:
    u = "http://127.0.0.1:8899"; body = b'{"name":"x","url":"http://a.example"}'
    check("cross-site POST refused", req(u + "/api/mesh/peers", "POST", {"Origin": "http://evil.example"}, body) == 403)
    check("Origin: null POST refused", req(u + "/api/mesh/peers", "POST", {"Origin": "null"}, body) == 403)
    check("same-origin POST allowed", req(u + "/api/mesh/peers", "POST", {"Origin": u}, body) == 200)
    check("non-browser POST (no Origin) allowed", req(u + "/api/mesh/peers", "POST", {}, body) == 200)
    check("rebinding Host refused", req(u + "/api/health", "GET", {"Host": "evil.example"}) == 403)
finally:
    p.kill()
z = zipfile.ZipFile(tmp + "/bomb.docx", "w", zipfile.ZIP_DEFLATED); z.writestr("word/document.xml", "<w:t>" + "a" * (300 << 20) + "</w:t>"); z.close()
os.makedirs(tmp + "/b"); os.rename(tmp + "/bomb.docx", tmp + "/b/bomb.docx")
r = subprocess.run([B, "ingest", tmp + "/b"], env=dict(env, TEZAUR_INDEX=tmp + "/b.idx"), capture_output=True, text=True, timeout=120)
check("zip bomb refused, not loaded", "zip bomb" in (r.stdout + r.stderr) and r.returncode in (0, 1))
open(tmp + "/d.md", "w").write("It took 3.71 days [@a, p. 1].\n")
r = subprocess.run([B, "audit", tmp + "/d.md"], env=env, capture_output=True, text=True)
check("re-ordered figure 3.71 vs 71.3 flagged", "number-mismatch" in r.stdout)
sys.exit(1 if fails else 0)
