#!/usr/bin/env python3
"""Executes the demo notebook in a real Jupyter kernel against a running `tezaur serve` ($TEZAUR_SERVER)."""
import os, sys
import nbformat
from nbclient import NotebookClient
sys.path.insert(0, os.path.dirname(__file__))
from make_demo import build

here = os.path.dirname(os.path.abspath(__file__))
os.environ["PYTHONPATH"] = os.path.join(here, "..", "pypi") + os.pathsep + os.environ.get("PYTHONPATH", "")
nb = build()
NotebookClient(nb, timeout=120, kernel_name="python3").execute()
def out(i):
    o = nb.cells[i].get("outputs", [])
    return " ".join(str(x.get("data", {}).get("text/html", "")) + str(x.get("data", {}).get("text/plain", "")) + str(x.get("text", "")) + str(x.get("ename", "")) for x in o)
ok = True
def check(name, cond):
    global ok; print(("PASS " if cond else "FAIL ") + name); ok = ok and cond
check("search renders passages", "burden" in out(2).lower())
check("ask renders cited answer + excerpts", "excerpts" in out(3) and "<div" in out(3))
check("audit flags the unsupported sentence", "unsupported" in out(4) and "Penguins" in out(4) and "1 problem" in out(4))
check("coverage shows empty section", "Quantum gravity" in out(5) and "empty" in out(5))
check("receipt verifies from the notebook", "True" in out(6))
check("no kernel errors", not any(c.get("outputs") and any(o.get("output_type") == "error" for o in c["outputs"]) for c in nb.cells))
sys.exit(0 if ok else 1)
