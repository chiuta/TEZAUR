"""Shared by the chat bots: runs the tezaur CLI and shapes its output for a chat message."""
import os, shlex, subprocess

TEZAUR = os.environ.get("TEZAUR_BIN", "tezaur")
EXTRA = shlex.split(os.environ.get("TEZAUR_ARGS", ""))  # e.g. --index /data/tezaur.idx --api ollama --model llama3.1:8b
TIMEOUT = int(os.environ.get("TEZAUR_TIMEOUT", "900"))
HELP = ("TEZAUR — întreabă biblioteca ta.\n"
        "/ask <întrebare> — răspuns cu citări\n"
        "/search <termeni> — doar pasaje\n"
        "/sources — documentele indexate\n"
        "Orice alt text = /ask.")


def run(cmd: str, arg: str = "") -> str:
    args = [TEZAUR, *EXTRA, cmd] + ([arg] if arg else [])
    try:
        p = subprocess.run(args, capture_output=True, text=True, timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        return "Timp depășit: modelul local e prea lent pentru această întrebare."
    except FileNotFoundError:
        return f"Nu găsesc programul „{TEZAUR}” (setează TEZAUR_BIN)."
    out = p.stdout.strip()
    warn = [l for l in p.stderr.splitlines() if l.startswith(("⚠", "error:"))]
    if p.returncode != 0 and not out:
        return "Eroare: " + (warn[0] if warn else p.stderr.strip()[-300:] or "necunoscută")
    return out + ("\n\n" + "\n".join(warn) if warn else "")


def route(text: str) -> str:
    """'/ask q', '/search q', '/sources', '/help', or free text -> reply."""
    text = text.strip()
    if not text or text in ("/start", "/help"):
        return HELP
    head, _, rest = text.partition(" ")
    head = head.split("@")[0]
    if head == "/sources":
        return run("sources")
    if head == "/search":
        return run("search", rest.strip()) if rest.strip() else "Scrie și ce cauți: /search termeni"
    if head == "/ask":
        return run("ask", rest.strip()) if rest.strip() else "Scrie și întrebarea: /ask ..."
    if head.startswith("/"):
        return HELP
    return run("ask", text)


def chunks(s: str, n: int = 3800):
    while s:
        cut = s.rfind("\n", 0, n) if len(s) > n else len(s)
        cut = cut if cut > n // 2 or len(s) <= n else n
        yield s[:cut]
        s = s[cut:].lstrip("\n")
