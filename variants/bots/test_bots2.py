"""Tests for the Slack/Discord/WhatsApp/e-mail bots with signed requests and mock services (no network)."""
import hashlib, hmac, http.server, json, os, sys, threading, time, urllib.request, urllib.parse
os.environ["TEZAUR_BIN"] = "/bin/echo"   # stands in for tezaur: echoes its arguments
sys.path.insert(0, os.path.dirname(__file__))
fails = []
def check(c, m):
    print(("ok   " if c else "FAIL ") + m)
    if not c: fails.append(m)

# a mock "other side": records everything posted to it
got = []
class Sink(http.server.BaseHTTPRequestHandler):
    def do_POST(self): self._r()
    def do_PATCH(self): self._r()
    def _r(self):
        got.append((self.command, self.path, self.headers.get("Authorization"), json.loads(self.rfile.read(int(self.headers["Content-Length"])) or b"{}")))
        self.send_response(200); self.send_header("Content-Length", "2"); self.end_headers(); self.wfile.write(b"{}")
    def log_message(self, *a): pass
sink = http.server.HTTPServer(("127.0.0.1", 0), Sink); threading.Thread(target=sink.serve_forever, daemon=True).start()
SINK = "http://127.0.0.1:%d" % sink.server_port

def serve(handler):
    s = http.server.HTTPServer(("127.0.0.1", 0), handler); threading.Thread(target=s.serve_forever, daemon=True).start()
    return "http://127.0.0.1:%d" % s.server_port

def post(url, body, headers):
    req = urllib.request.Request(url, body, headers)
    try: r = urllib.request.urlopen(req, timeout=10); return r.status, r.read()
    except urllib.error.HTTPError as e: return e.code, b""

def wait(n, t=5):
    end = time.time() + t
    while len(got) < n and time.time() < end: time.sleep(0.05)

# ---- Slack
os.environ.update(SLACK_SIGNING_SECRET="s3cret", ALLOWED_SLACK_USERS="U1")
import slack_bot
body = urllib.parse.urlencode({"user_id": "U1", "text": "ask burden", "response_url": SINK + "/slack-resp"}).encode()
ts = str(int(time.time())); sig = "v0=" + hmac.new(b"s3cret", b"v0:" + ts.encode() + b":" + body, hashlib.sha256).hexdigest()
u = serve(slack_bot.H)
st, _ = post(u, body, {"X-Slack-Request-Timestamp": ts, "X-Slack-Signature": sig}); wait(1)
check(st == 200 and got and got[-1][1] == "/slack-resp" and "ask burden" in got[-1][3]["text"], "slack: signed request answered via response_url")
check(post(u, body, {"X-Slack-Request-Timestamp": ts, "X-Slack-Signature": "v0=bad"})[0] == 401, "slack: bad signature rejected")
old = str(int(time.time()) - 4000); sig2 = "v0=" + hmac.new(b"s3cret", b"v0:" + old.encode() + b":" + body, hashlib.sha256).hexdigest()
check(post(u, body, {"X-Slack-Request-Timestamp": old, "X-Slack-Signature": sig2})[0] == 401, "slack: replayed (old) request rejected")
body2 = urllib.parse.urlencode({"user_id": "U9", "text": "x", "response_url": SINK + "/no"}).encode()
sig3 = "v0=" + hmac.new(b"s3cret", b"v0:" + ts.encode() + b":" + body2, hashlib.sha256).hexdigest()
st, r = post(u, body2, {"X-Slack-Request-Timestamp": ts, "X-Slack-Signature": sig3})
check(st == 200 and b"Not allowed" in r, "slack: unlisted user refused")

# ---- Discord
from nacl.signing import SigningKey
sk = SigningKey.generate()
os.environ.update(DISCORD_PUBLIC_KEY=sk.verify_key.encode().hex(), DISCORD_APP_ID="APP", ALLOWED_DISCORD_USERS="42", DISCORD_API=SINK)
import discord_bot
def dsign(b): t = str(int(time.time())); return {"X-Signature-Ed25519": sk.sign(t.encode() + b).signature.hex(), "X-Signature-Timestamp": t}
u = serve(discord_bot.H)
b = json.dumps({"type": 1}).encode()
st, r = post(u, b, dsign(b)); check(st == 200 and json.loads(r) == {"type": 1}, "discord: PING answered (signature verified)")
check(post(u, b, {"X-Signature-Ed25519": "00" * 64, "X-Signature-Timestamp": "1"})[0] == 401, "discord: bad signature rejected")
n = len(got)
b = json.dumps({"type": 2, "token": "TOK", "member": {"user": {"id": "42"}}, "data": {"options": [{"name": "question", "value": "burden"}]}}).encode()
st, r = post(u, b, dsign(b)); wait(n + 1)
check(json.loads(r)["type"] == 5 and got[-1][0] == "PATCH" and got[-1][1] == "/webhooks/APP/TOK/messages/@original" and "ask burden" in got[-1][3]["content"], "discord: deferred reply then PATCH @original")
b = json.dumps({"type": 2, "token": "T", "member": {"user": {"id": "7"}}, "data": {}}).encode()
check(json.loads(post(u, b, dsign(b))[1])["data"]["content"] == "Not allowed.", "discord: unlisted user refused")

# ---- WhatsApp
os.environ.update(WA_VERIFY_TOKEN="vt", WA_APP_SECRET="as", WA_ACCESS_TOKEN="AT", WA_PHONE_ID="PH", ALLOWED_WA_NUMBERS="4071", WA_GRAPH=SINK)
import whatsapp_bot
u = serve(whatsapp_bot.H)
r = urllib.request.urlopen(u + "/?hub.mode=subscribe&hub.verify_token=vt&hub.challenge=12345").read()
check(r == b"12345", "whatsapp: webhook verification handshake")
payload = json.dumps({"entry": [{"changes": [{"value": {"messages": [{"from": "4071", "type": "text", "text": {"body": "burden"}}, {"from": "999", "type": "text", "text": {"body": "spam"}}]}}]}]}).encode()
n = len(got); sig = "sha256=" + hmac.new(b"as", payload, hashlib.sha256).hexdigest()
st, _ = post(u, payload, {"X-Hub-Signature-256": sig}); wait(n + 1); time.sleep(0.3)
sent = got[n:]
check(st == 200 and len(sent) == 1 and sent[0][1] == "/PH/messages" and sent[0][2] == "Bearer AT" and sent[0][3]["to"] == "4071", "whatsapp: replies only to the allowed number")
check(post(u, payload, {"X-Hub-Signature-256": "sha256=00"})[0] == 401, "whatsapp: bad signature rejected")

# ---- e-mail
import email_bot
from email.message import EmailMessage
m = EmailMessage(); m["From"] = "Ana <ana@uni.ro>"; m["Subject"] = "burden of proof?"; m["Message-ID"] = "<1@x>"; m.set_content("Who bears it?\n> quoted\n-- \nsig")
r = email_bot.reply_for(bytes(m), {"ana@uni.ro"}, answer=lambda q: "ANSWER:" + q)
check(r is not None and r["To"] == "ana@uni.ro" and r["In-Reply-To"] == "<1@x>" and "ANSWER:Who bears it?" in r.get_content() and "quoted" not in r.get_content(), "email: reply threads, quotes and signature stripped")
check(email_bot.reply_for(bytes(m), {"bob@x.org"}) is None, "email: unknown sender ignored")
sent_mail = []
class FakeIMAP:
    def __init__(s, host): pass
    def __enter__(s): return s
    def __exit__(s, *a): pass
    def login(s, *a): pass
    def select(s, *a): pass
    def search(s, *a): return "OK", [b"1"]
    def fetch(s, i, spec): return "OK", [(b"1 (RFC822)", bytes(m))]
class FakeSMTP(FakeIMAP):
    def send_message(s, msg): sent_mail.append(msg)
os.environ["TEZAUR_BIN"] = "/bin/echo"
email_bot.poll(FakeIMAP, FakeSMTP, {"IMAP_HOST": "i", "SMTP_HOST": "s", "MAIL_USER": "bot@x.org", "MAIL_PASSWORD": "p", "ALLOWED_SENDERS": "ana@uni.ro"})
check(len(sent_mail) == 1 and sent_mail[0]["From"] == "bot@x.org", "email: poll() fetches, answers and sends via SMTP")

print("\n%d failed" % len(fails) if fails else "\nall bot tests passed")
sys.exit(1 if fails else 0)
