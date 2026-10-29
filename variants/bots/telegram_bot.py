#!/usr/bin/env python3
"""Telegram front-end for TEZAUR (long polling, standard library only). The library and the model stay on your machine;
only questions and answers pass through Telegram. Set:
  TELEGRAM_TOKEN      from @BotFather
  ALLOWED_CHAT_IDS    comma-separated chat ids that may use the bot (required — the bot answers nobody else)
  TEZAUR_BIN / TEZAUR_ARGS   path to tezaur and its global flags
Send /id to the bot to learn your chat id."""
import json, os, sys, time, urllib.parse, urllib.request
from common import route, chunks

TOKEN = os.environ.get("TELEGRAM_TOKEN", "")
API = os.environ.get("TELEGRAM_API", "https://api.telegram.org")
ALLOWED = {x.strip() for x in os.environ.get("ALLOWED_CHAT_IDS", "").split(",") if x.strip()}


def call(method, **params):
    data = urllib.parse.urlencode(params).encode()
    with urllib.request.urlopen(f"{API}/bot{TOKEN}/{method}", data, timeout=70) as r:
        return json.load(r)


def handle(update):
    msg = update.get("message") or {}
    text, chat = msg.get("text"), str((msg.get("chat") or {}).get("id", ""))
    if not text or not chat:
        return
    if text.strip().startswith("/id"):
        return call("sendMessage", chat_id=chat, text=f"chat id: {chat}")
    if chat not in ALLOWED:
        return  # silent: never reveal the bot serves a private library
    call("sendChatAction", chat_id=chat, action="typing")
    for part in chunks(route(text)):
        call("sendMessage", chat_id=chat, text=part)


def main(once=False):
    if not TOKEN or not ALLOWED:
        sys.exit("Set TELEGRAM_TOKEN and ALLOWED_CHAT_IDS (send /id to the bot once to see your id).")
    offset = 0
    while True:
        try:
            for u in call("getUpdates", offset=offset, timeout=30).get("result", []):
                offset = u["update_id"] + 1
                handle(u)
        except Exception as e:  # network hiccup: wait and retry
            print("telegram:", e, file=sys.stderr)
            time.sleep(5)
        if once:
            return


if __name__ == "__main__":
    main()
