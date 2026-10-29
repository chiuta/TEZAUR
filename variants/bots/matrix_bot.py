#!/usr/bin/env python3
"""Matrix front-end for TEZAUR (standard library only). Unencrypted rooms only — the bot cannot read end-to-end
encrypted rooms. It answers messages that start with "!tezaur" (e.g. "!tezaur who bears the burden of proof?",
"!tezaur search burden", "!tezaur sources"). Set:
  MATRIX_HOMESERVER   https://matrix.example.org
  MATRIX_TOKEN        access token of the bot account
  MATRIX_ROOMS        comma-separated room ids it may answer in (required)
  TEZAUR_BIN / TEZAUR_ARGS"""
import json, os, sys, time, urllib.parse, urllib.request, uuid
from common import route, chunks

HS = os.environ.get("MATRIX_HOMESERVER", "").rstrip("/")
TOKEN = os.environ.get("MATRIX_TOKEN", "")
ROOMS = {x.strip() for x in os.environ.get("MATRIX_ROOMS", "").split(",") if x.strip()}
PREFIX = "!tezaur"


def req(method, path, body=None):
    r = urllib.request.Request(HS + "/_matrix/client/v3" + path, method=method,
                               data=None if body is None else json.dumps(body).encode(),
                               headers={"Authorization": "Bearer " + TOKEN, "Content-Type": "application/json"})
    with urllib.request.urlopen(r, timeout=90) as resp:
        return json.load(resp)


def say(room, text):
    for part in chunks(text):
        req("PUT", f"/rooms/{urllib.parse.quote(room)}/send/m.room.message/{uuid.uuid4().hex}", {"msgtype": "m.text", "body": part})


def handle_sync(data, me):
    for room, info in (data.get("rooms", {}).get("join", {})).items():
        if room not in ROOMS:
            continue
        for ev in info.get("timeline", {}).get("events", []):
            body = (ev.get("content") or {}).get("body", "")
            if ev.get("type") == "m.room.message" and ev.get("sender") != me and body.startswith(PREFIX):
                q = body[len(PREFIX):].strip()
                # "search x" / "sources" / "ask x" map onto the shared router; free text = ask
                if q.split(" ")[0] in ("search", "sources", "ask", "help"):
                    q = "/" + q
                say(room, route(q))


def main(once=False):
    if not (HS and TOKEN and ROOMS):
        sys.exit("Set MATRIX_HOMESERVER, MATRIX_TOKEN and MATRIX_ROOMS.")
    me = req("GET", "/account/whoami")["user_id"]
    since = req("GET", "/sync?timeout=0").get("next_batch")  # ignore history: only new messages
    while True:
        try:
            d = req("GET", f"/sync?timeout=30000&since={urllib.parse.quote(since)}")
            since = d["next_batch"]
            handle_sync(d, me)
        except Exception as e:
            print("matrix:", e, file=sys.stderr)
            time.sleep(5)
        if once:
            return


if __name__ == "__main__":
    main()
