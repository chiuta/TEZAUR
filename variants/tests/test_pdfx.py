#!/usr/bin/env python3
"""Advanced PDF: tables and display equations from real PDFs built with pdflatex (skipped when pdflatex/pdftotext are missing).
Heuristic extraction: this checks the sample documents below, NOT arbitrary PDFs."""
import json, os, shutil, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.environ.get("TEZAUR_BIN") or os.path.join(HERE, "..", "..", "target", "release", "tezaur")
if not (shutil.which("pdflatex") and shutil.which("pdftotext")):
    print("SKIP pdflatex/pdftotext not installed"); sys.exit(0)
ok = True
def check(n, c, e=""):
    global ok
    print(("PASS " if c else "FAIL ") + n + ("" if c else " — " + str(e)[:400])); ok = ok and bool(c)
d = tempfile.mkdtemp()
def mk(name, body, pre=""):
    open(f"{d}/{name}.tex", "w").write("\\documentclass{article}\\usepackage{amsmath}" + pre + "\\begin{document}" + body + "\\end{document}")
    subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", f"{name}.tex"], cwd=d, capture_output=True, stdin=subprocess.DEVNULL)
mk("one", "Some introductory prose about methods and results in this paper.\n\n"
 "\\begin{table}[h]\\centering\\setlength{\\tabcolsep}{16pt}\\caption{Accuracy of three methods.}\\begin{tabular}{lrr}Method & Acc. & F1 \\\\ Baseline & 71.2 & 69.8 \\\\ Ours & 84.5 & 83.1 \\\\ Oracle & 93.0 & 92.4 \\end{tabular}\\end{table}\n\n"
 "More prose follows here to separate the objects on the page.\n\n"
 "\\begin{equation} S = \\sum_{i=1}^{N} p_i t_i \\end{equation}\n\nClosing words of the paragraph.")
env = dict(os.environ, TEZAUR_INDEX=f"{d}/t.idx")
def run(*a): return subprocess.run([BIN, *a], capture_output=True, text=True, stdin=subprocess.DEVNULL, env=env, cwd=d)
t = run("tables", f"{d}/one.pdf")
check("tables: Markdown header", "| Method | Acc. | F1 |" in t.stdout, t.stdout + t.stderr)
check("tables: rows with numbers", "| Ours | 84.5 | 83.1 |" in t.stdout, t.stdout)
j = run("tables", f"{d}/one.pdf", "--json")
try: tj = json.loads(j.stdout)
except Exception: tj = None
check("tables: JSON parses", isinstance(tj, (list, dict)), j.stdout)
c = run("tables", f"{d}/one.pdf", "--csv")
check("tables: CSV", "Baseline,71.2,69.8" in c.stdout, c.stdout)
e = run("equations", f"{d}/one.pdf")
check("equations: sum with limits and number", "\\sum_{i=1}^{N}" in e.stdout and "eq. 1" in e.stdout, e.stdout)
run("ingest", f"{d}/one.pdf")
s = run("search", "Baseline Oracle accuracy")
check("search: whole table is one passage", "| Oracle | 93.0 | 92.4 |" in s.stdout and "| Baseline | 71.2 | 69.8 |" in s.stdout, s.stdout)
s2 = run("search", "sum equation")
check("search: equation text indexed", "[equation]" in s2.stdout or "\\sum" in s2.stdout, s2.stdout)
sys.exit(0 if ok else 1)
