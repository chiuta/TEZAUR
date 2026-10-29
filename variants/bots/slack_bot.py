#!/usr/bin/env python3
"""Slack slash command `/tezaur <question>` (stdlib only). Slack calls YOUR https endpoint, so it must be reachable from the internet
(reverse proxy / tunnel to this port); the library and the model stay on your machine, only questions and answers cross Slack.
Env: SLACK_SIGNING_SECRET (required), ALLOWED_SLACK_USERS (comma-separated user ids, required), PORT (default 8790)."""
import hashlib, hmac, json, os, sys, threading, time, urllib.parse, urllib.request
from http.server import BaseHTTPRequestHandler, HTTPServer
import common

SECRET = os.environ.get("SLACK_SIGNING_SECRET", "")
ALLOWED = {x.strip() for x in os.environ.get("ALLOWED_SLACK_USERS", "").split(",") if x.strip()}


def verify(secret, timestamp, body, signature, now=None):
    """Slack's v0 signature: HMAC-SHA256 over 'v0:<ts>:<body>', rejecting requests older than 5 minutes."""
    try:
        if abs((now or time.time()) - int(timestamp)) > 300:
            return False
    except (TypeError, ValueError):
        return False
    mine = "v0=" + hmac.new(secret.encode(), b"v0:" + timestamp.encode() + b":" + body, hashlib.sha256).hexdigest()
    return hmac.compare_digest(mine, signature or "")


def delayed(url, text):
    for part in common.chunks(text, 2900):
        req = urllib.request.Request(url, json.dumps({"response_type": "ephemeral", "text": part}).encode(), {"Content-Type": "application/json"})
        urllib.request.urlopen(req, timeout=30).read()


class H(BaseHTTPRequestHandler):
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        if not verify(SECRET, self.headers.get("X-Slack-Request-Timestamp", ""), body, self.headers.get("X-Slack-Signature", "")):
            self.send_response(401); self.end_headers(); return
        f = {k: v[0] for k, v in urllib.parse.parse_qs(body.decode()).items()}
        if f.get("user_id") not in ALLOWED:
            return self._json({"response_type": "ephemeral", "text": "Not allowed."})
        threading.Thread(target=lambda: delayed(f["response_url"], common.route(f.get("text", "") or "/help")), daemon=True).start()
        self._json({"response_type": "ephemeral", "text": "Searching your library… (a local model can take a while)"})

    def _json(self, d):
        b = json.dumps(d).encode()
        self.send_response(200); self.send_header("Content-Type", "application/json"); self.send_header("Content-Length", str(len(b))); self.end_headers(); self.wfile.write(b)

    def log_message(self, *a): pass


if __name__ == "__main__":
    if not SECRET or not ALLOWED:
        sys.exit("Set SLACK_SIGNING_SECRET and ALLOWED_SLACK_USERS.")
    HTTPServer(("0.0.0.0", int(os.environ.get("PORT", "8790"))), H).serve_forever()
