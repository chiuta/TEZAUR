"""Audit the prose of a whole notebook (.ipynb): every Markdown cell is checked against the library.
    python -m tezaur_client.notebook thesis.ipynb          # prints problems per cell, exit 2 if any"""
import json
import sys

from . import Client


def audit_notebook(path, client=None):
    c = client or Client()
    nb = json.load(open(path, encoding="utf-8"))
    out = []
    for i, cell in enumerate(nb.get("cells", [])):
        if cell.get("cell_type") != "markdown":
            continue
        text = "".join(cell.get("source", []))
        if "@" not in text and len(text.split()) < 14:
            continue
        for f in c.audit(text)["findings"]:
            if f["verdict"] not in ("supported", "quote-verbatim"):
                out.append(dict(f, cell=i))
    return out


def main(argv=None):
    argv = argv or sys.argv[1:]
    if not argv:
        print(__doc__)
        return 1
    bad = audit_notebook(argv[0])
    for f in bad:
        print("cell %d: %s — %s\n    %s" % (f["cell"], f["verdict"], f["note"], f["sentence"][:140]))
    print("%d problem(s)" % len(bad))
    return 2 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
