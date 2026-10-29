#!/usr/bin/env python3
"""WhatsApp Business Cloud API webhook (stdlib only). Meta calls YOUR https endpoint; questions and answers pass through Meta's servers.
Env: WA_VERIFY_TOKEN (any string you also enter in Meta's webhook setup), WA_APP_SECRET (signature check), WA_ACCESS_TOKEN,
WA_PHONE_ID, ALLOWED_WA_NUMBERS (digits only, comma-separated, required), PORT (8792)."""
import hashlib, hmac, json, os, sys, threading, urllib.parse, urllib.request
from http.server import BaseHTTPRequestHandler, HTTPServer
import common

GRAPH = os.environ.get("WA_GRAPH", "https://graph.facebook.com/v20.0")
ALLOWED = {x.strip() for x in os.environ.get("ALLOWED_WA_NUMBERS", "").split(",") if x.strip()}


def cfg(k): return os.environ.get(k, "")


def verify(app_secret, body, header):
    mine = "sha256=" + hmac.new(app_secret.encode(), body, hashlib.sha256).hexdigest()
    return hmac.compare_digest(mine, header or "")


def send(to, text):
    for part in common.chunks(text, 3800):
        req = urllib.request.Request(f"{GRAPH}/{cfg('WA_PHONE_ID')}/messages", json.dumps({"messaging_product": "whatsapp", "to": to, "type": "text", "text": {"body": part}}).encode(),
                                     {"Content-Type": "application/json", "Authorization": "Bearer " + cfg("WA_ACCESS_TOKEN")})
        urllib.request.urlopen(req, timeout=30).read()


def messages(payload):
    for e in payload.get("entry", []):
        for c in e.get("changes", []):
            for m in c.get("value", {}).get("messages", []):
                if m.get("type") == "text":
                    yield m["from"], m["text"]["body"]


class H(BaseHTTPRequestHandler):
    def do_GET(self):  # webhook verification handshake
        q = {k: v[0] for k, v in urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query).items()}
        if q.get("hub.mode") == "subscribe" and hmac.compare_digest(q.get("hub.verify_token", ""), cfg("WA_VERIFY_TOKEN")):
            b = q.get("hub.challenge", "").encode(); self.send_response(200); self.end_headers(); self.wfile.write(b)
        else:
            self.send_response(403); self.end_headers()

    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        if not verify(cfg("WA_APP_SECRET"), body, self.headers.get("X-Hub-Signature-256")):
            self.send_response(401); self.end_headers(); return
        for sender, text in messages(json.loads(body)):
            if sender in ALLOWED:  # silent for everyone else
                threading.Thread(target=lambda s=sender, t=text: send(s, common.route(t)), daemon=True).start()
        self.send_response(200); self.end_headers()

    def log_message(self, *a): pass


if __name__ == "__main__":
    if not (cfg("WA_VERIFY_TOKEN") and cfg("WA_APP_SECRET") and cfg("WA_ACCESS_TOKEN") and cfg("WA_PHONE_ID") and ALLOWED):
        sys.exit("Set WA_VERIFY_TOKEN, WA_APP_SECRET, WA_ACCESS_TOKEN, WA_PHONE_ID and ALLOWED_WA_NUMBERS.")
    HTTPServer(("0.0.0.0", int(os.environ.get("PORT", "8792"))), H).serve_forever()
