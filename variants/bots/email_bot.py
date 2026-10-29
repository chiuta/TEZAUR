#!/usr/bin/env python3
"""E-mail front-end (stdlib): write a question to a dedicated mailbox, get a cited answer back. Works offline-first with any IMAP/SMTP
server, e.g. your own. Only senders in ALLOWED_SENDERS are answered (the From header can be forged: use a mailbox that enforces SPF/DKIM/DMARC).
Env: IMAP_HOST, SMTP_HOST, MAIL_USER, MAIL_PASSWORD, ALLOWED_SENDERS (comma-separated addresses), POLL_SECONDS (60)."""
import email, imaplib, os, smtplib, sys, time
from email.message import EmailMessage
from email.utils import parseaddr
import common


def question_of(msg):
    subject = (msg.get("Subject") or "").strip()
    body = ""
    for part in msg.walk() if msg.is_multipart() else [msg]:
        if part.get_content_type() == "text/plain":
            body = part.get_payload(decode=True).decode(part.get_content_charset() or "utf-8", "replace")
            break
    lines = [l for l in body.splitlines() if not l.startswith(">")]
    text = "\n".join(lines).split("\n-- \n")[0].strip()
    return (text or subject).strip()


def reply_for(raw, allowed, answer=common.route):
    """raw bytes of an incoming message -> EmailMessage reply, or None when the sender is not allowed / there is no question."""
    msg = email.message_from_bytes(raw)
    sender = parseaddr(msg.get("From", ""))[1].lower()
    if sender not in allowed:
        return None
    q = question_of(msg)
    if not q:
        return None
    r = EmailMessage()
    r["To"], r["Subject"] = sender, "Re: " + (msg.get("Subject") or "your question")
    if msg.get("Message-ID"):
        r["In-Reply-To"] = r["References"] = msg["Message-ID"]
    r.set_content(answer(q))
    return r


def poll(imap_cls=imaplib.IMAP4_SSL, smtp_cls=smtplib.SMTP_SSL, env=os.environ):
    allowed = {a.strip().lower() for a in env.get("ALLOWED_SENDERS", "").split(",") if a.strip()}
    with imap_cls(env["IMAP_HOST"]) as im:
        im.login(env["MAIL_USER"], env["MAIL_PASSWORD"]); im.select("INBOX")
        _, ids = im.search(None, "UNSEEN")
        for i in ids[0].split():
            _, data = im.fetch(i, "(RFC822)")  # marks the message as seen
            r = reply_for(data[0][1], allowed)
            if r is not None:
                r["From"] = env["MAIL_USER"]
                with smtp_cls(env["SMTP_HOST"]) as s:
                    s.login(env["MAIL_USER"], env["MAIL_PASSWORD"]); s.send_message(r)


if __name__ == "__main__":
    need = ["IMAP_HOST", "SMTP_HOST", "MAIL_USER", "MAIL_PASSWORD", "ALLOWED_SENDERS"]
    if any(not os.environ.get(k) for k in need):
        sys.exit("Set " + ", ".join(need))
    while True:
        try:
            poll()
        except Exception as e:  # keep running through network hiccups
            print("poll failed:", e, file=sys.stderr)
        time.sleep(int(os.environ.get("POLL_SECONDS", "60")))
