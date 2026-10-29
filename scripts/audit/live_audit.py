#!/usr/bin/env python3
"""Adversarial audit of the LIVE presentation site. Run on a GitHub runner (the dev sandbox cannot reach the domain).
Emits one GitHub annotation per observation: ::notice for facts, ::error for defects (so results are readable without log access)."""
import json, re, socket, ssl, subprocess, sys, time, urllib.request, urllib.error, datetime
H = sys.argv[1] if len(sys.argv) > 1 else "tezaur.chiuta.ro"
BASE = "https://" + H
def note(t, m): print(f"::notice title={t}::{str(m)[:1500]}".replace("\n", "%0A"), flush=True)
def warn(t, m): print(f"::warning title={t}::{str(m)[:1500]}".replace("\n", "%0A"), flush=True)
def bad(t, m): print(f"::error title={t}::{str(m)[:1500]}".replace("\n", "%0A"), flush=True)
def get(url, headers=None, method="GET", timeout=40, follow=True):
    class NoRedir(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, *a, **k): return None
    op = urllib.request.build_opener() if follow else urllib.request.build_opener(NoRedir)
    r = urllib.request.Request(url, headers=headers or {}, method=method)
    t0 = time.time()
    try:
        with op.open(r, timeout=timeout) as x:
            body = x.read() if method == "GET" else b""
            return x.status, dict((k.lower(), v) for k, v in x.headers.items()), body, time.time() - t0
    except urllib.error.HTTPError as e:
        return e.code, dict((k.lower(), v) for k, v in e.headers.items()), e.read() if method == "GET" else b"", time.time() - t0
    except Exception as e:
        return 0, {}, str(e).encode(), time.time() - t0
# ---- TLS / DNS
try:
    ctx = ssl.create_default_context()
    with socket.create_connection((H, 443), timeout=15) as s, ctx.wrap_socket(s, server_hostname=H) as ss:
        c = ss.getpeercert(); exp = datetime.datetime.strptime(c["notAfter"], "%b %d %H:%M:%S %Y %Z")
        note("tls", f"valid, expires {exp:%Y-%m-%d} ({(exp-datetime.datetime.utcnow()).days} days); SAN={[v for k,v in c['subjectAltName']]}; proto={ss.version()}")
except Exception as e: bad("tls", f"TLS handshake to {H} failed: {e}")
for h in (H, "www." + H):
    try: note("dns", f"{h} -> {socket.gethostbyname_ex(h)[2]}")
    except Exception as e: (warn if h.startswith("www.") else bad)("dns", f"{h}: {e}")
st, hd, _, _ = get("http://" + H + "/", method="GET", follow=False)
note("http-redirect", f"http://{H}/ -> {st} {hd.get('location')}") if st in (301, 302, 308) else bad("http-redirect", f"plain HTTP not redirected to HTTPS (status {st})")
st, hd, _, _ = get("https://www." + H + "/", follow=False)
note("www", f"https://www.{H}/ -> {st} {hd.get('location')}") if st in (200, 301, 308) else warn("www", f"www subdomain answers {st} (needs a DNS CNAME owned by the domain admin)")
# ---- main page
st, hd, body, dt = get(BASE + "/", headers={"Accept-Encoding": "gzip, br"})
import gzip, zlib
if hd.get("content-encoding") == "gzip": body = gzip.decompress(body)
elif hd.get("content-encoding") == "deflate": body = zlib.decompress(body)
html = body.decode("utf-8", "replace")
note("index", f"status {st}, {len(body)/1e6:.2f} MB transferred ({hd.get('content-encoding')}), {dt:.2f}s, content-type={hd.get('content-type')}, cache-control={hd.get('cache-control')}")
for k in ("strict-transport-security", "content-security-policy", "x-content-type-options", "x-frame-options", "referrer-policy", "permissions-policy"):
    if k in hd: continue
    # GitHub Pages cannot set response headers: a <meta> equivalent counts where one exists, the rest is a standing warning
    meta = {"content-security-policy": 'http-equiv="Content-Security-Policy"', "referrer-policy": 'name="referrer"'}.get(k)
    if meta and meta in html: note("header-meta", f"{k} absent as header, present as <meta>")
    else: warn("header-missing", f"{k} absent on / (GitHub Pages cannot set it; needs a proxy/CDN)")
n_sl = len(re.findall(r'<section class="slide"', html)); ctr = re.findall(r'id="count"[^>]*>([^<]*)', html)
if not n_sl or not ctr or not ctr[0].strip().endswith(f"/ {n_sl:02d}"): bad("counter", f"static counter {ctr} does not match {n_sl} slides")
for need in ('rel="canonical"', 'property="og:title"', 'rel="icon"', 'id="vid"'):
    if need not in html: bad("metadata", f"missing {need}")
note("slides", f"live HTML: {len(re.findall(r'<section class=\"slide\"', html))} slides, counter literal={re.findall(r'id=\"count\"[^>]*>([^<]*)', html)}, has video section={'id=\"vid\"' in html}")
# ---- assets
for path in ["assets/tutorials/05-cli-ingest-search.mp4", "assets/tutorials/05-cli-ingest-search.hu.vtt", "assets/tutorials/05-cli-ingest-search.jpg", "app/", "app/manifest.webmanifest", "app/sw.js", "favicon.ico", "robots.txt", "sitemap.xml", "404.html", "this-page-does-not-exist-xyz", "CNAME", "apt/Packages", "rpm/tezaur.repo", "flatpak/tezaur.flatpakrepo", "fdroid/repo/index-v1.json"]:
    st, hd, b, dt = get(f"{BASE}/{path}", method="GET" if not path.endswith(".mp4") else "HEAD")
    msg = f"{path}: {st} type={hd.get('content-type')} len={hd.get('content-length')} ranges={hd.get('accept-ranges')} cache={hd.get('cache-control')}"
    (note if st == 200 or path in ("404.html", "this-page-does-not-exist-xyz", "CNAME", "favicon.ico", "robots.txt", "sitemap.xml") else bad)("asset", msg)
st, hd, b, _ = get(f"{BASE}/assets/tutorials/05-cli-ingest-search.mp4", headers={"Range": "bytes=0-99"})
(note if st == 206 else bad)("range-request", f"video Range request -> {st}, content-range={hd.get('content-range')}")
st, hd, b, _ = get(f"{BASE}/assets/tutorials/05-cli-ingest-search.hu.vtt")
ct = hd.get("content-type", "")
(note if "text/vtt" in ct else bad)("vtt-mime", f"subtitle MIME is '{ct}' (Firefox/Safari refuse tracks that are not text/vtt); first line={b[:8]!r}")
# ---- every link and asset referenced by the page
urls = set(re.findall(r'(?:href|src)="(https?://[^"#]+)"', html)) | set(re.findall(r"https?://[A-Za-z0-9./_\-?=&%~]+", re.sub(r"data:[^\"')]+", "", html)))
urls = {u.rstrip(".,;)") for u in urls if "192.168" not in u and "127.0.0.1" not in u and "example" not in u and "localhost" not in u}
for u in sorted(urls)[:60]:
    st, hd, _, dt = get(u, method="HEAD" if "github.com" not in u else "GET", timeout=25)
    (note if 200 <= st < 400 else bad)("link", f"{u} -> {st}")
# ---- all tutorial files exist live (13 x 10)
ids = [m["id"] for m in json.loads(get(f"{BASE}/assets/tutorials/tutorials.json")[2] or b"[]")]
miss = []
for i in ids:
    for ext in ["mp4", "jpg"] + [l + ".vtt" for l in "ro en fr it es pt de hu".split()]:
        st = get(f"{BASE}/assets/tutorials/{i}.{ext}", method="HEAD")[0]
        if st != 200: miss.append(f"{i}.{ext}:{st}")
(bad if miss else note)("tutorial-files", f"{len(ids)} tutorials; missing/broken: {miss[:10]}")
