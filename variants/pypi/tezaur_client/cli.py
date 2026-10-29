import argparse
import os
import sys

from . import Client, TezaurError


def main(argv=None):
    ap = argparse.ArgumentParser(prog="tezaur-py", description="Talk to a running `tezaur serve`.")
    ap.add_argument("command", choices=["ask", "compare", "gaps", "search", "sources"])
    ap.add_argument("text", nargs="*")
    ap.add_argument("-k", type=int, default=8)
    ap.add_argument("--server", default=os.environ.get("TEZAUR_SERVER", "http://127.0.0.1:8787"))
    ap.add_argument("--token", default=os.environ.get("TEZAUR_TOKEN"))
    a = ap.parse_args(argv)
    c, q = Client(a.server, a.token), " ".join(a.text)
    try:
        if a.command == "sources":
            for d in c.sources()["docs"]:
                print("%s%s  (%d chunks)" % (d["title"], " — " + d["cite"] if d.get("cite") else "", d["chunks"]))
        elif a.command == "search":
            for h in c.search(q, a.k):
                print("[%d] %s\n    %s" % (h["n"], h["label"], " ".join(h["text"][:300].split())))
        else:
            r = c.ask(q, a.command, a.k)
            print(r["answer"])
            if r.get("references"):
                print("\n## References\n" + r["references"])
            for w in r.get("warnings", []):
                print("⚠", w, file=sys.stderr)
    except TezaurError as e:
        sys.exit("tezaur-py: %s" % e)


if __name__ == "__main__":
    main()
