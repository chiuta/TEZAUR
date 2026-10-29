#!/usr/bin/env python3
"""Discord slash command `/tezaur question:<text>` via the Interactions Endpoint (HTTP, no gateway/websocket).
Needs `pip install pynacl` (Discord requires Ed25519 signature checks). Register the command once (see README).
Env: DISCORD_PUBLIC_KEY, DISCORD_APP_ID (both from the developer portal), ALLOWED_DISCORD_USERS (user ids, required), PORT (8791)."""
import json, os, sys, threading, urllib.request
from http.server import BaseHTTPRequestHandler, HTTPServer
import common

PUBKEY = os.environ.get("DISCORD_PUBLIC_KEY", "")
APP_ID = os.environ.get("DISCORD_APP_ID", "")
API = os.environ.get("DISCORD_API", "https://discord.com/api/v10")
ALLOWED = {x.strip() for x in os.environ.get("ALLOWED_DISCORD_USERS", "").split(",") if x.strip()}


def verify(pubkey_hex, signature_hex, timestamp, body):
    from nacl.signing import VerifyKey
    from nacl.exceptions import BadSignatureError
    try:
        VerifyKey(bytes.fromhex(pubkey_hex)).verify(timestamp.encode() + body, bytes.fromhex(signature_hex))
        return True
    except (BadSignatureError, ValueError):
        return False


def finish(token, text):
    parts = list(common.chunks(text, 1900)) or ["(empty)"]
    for i, part in enumerate(parts):
        url = f"{API}/webhooks/{APP_ID}/{token}" + ("/messages/@original" if i == 0 else "")
        req = urllib.request.Request(url, json.dumps({"content": part}).encode(), {"Content-Type": "application/json"}, method="PATCH" if i == 0 else "POST")
        urllib.request.urlopen(req, timeout=30).read()


def handle(body):
    """Returns the immediate JSON reply for an interaction; long work is finished in a thread."""
    i = json.loads(body)
    if i.get("type") == 1:
        return {"type": 1}
    user = (i.get("member", {}).get("user") or i.get("user") or {}).get("id")
    if user not in ALLOWED:
        return {"type": 4, "data": {"content": "Not allowed.", "flags": 64}}
    q = next((o["value"] for o in i.get("data", {}).get("options", []) if o["name"] == "question"), "/help")
    threading.Thread(target=lambda: finish(i["token"], common.route(q)), daemon=True).start()
    return {"type": 5, "data": {"flags": 64}}  # "thinking…", visible only to the asker


class H(BaseHTTPRequestHandler):
    def do_POST(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        if not verify(PUBKEY, self.headers.get("X-Signature-Ed25519", ""), self.headers.get("X-Signature-Timestamp", ""), body):
            self.send_response(401); self.end_headers(); return
        b = json.dumps(handle(body)).encode()
        self.send_response(200); self.send_header("Content-Type", "application/json"); self.send_header("Content-Length", str(len(b))); self.end_headers(); self.wfile.write(b)

    def log_message(self, *a): pass


if __name__ == "__main__":
    if not (PUBKEY and APP_ID and ALLOWED):
        sys.exit("Set DISCORD_PUBLIC_KEY, DISCORD_APP_ID and ALLOWED_DISCORD_USERS.")
    HTTPServer(("0.0.0.0", int(os.environ.get("PORT", "8791"))), H).serve_forever()
