#!/usr/bin/env python3
"""LyX / LaTeX tooling against a live `tezaur serve` ($TEZAUR_SERVER): .tex and .lyx audit, \\cite suggestions, LyX server pipe (a FIFO stands in for LyX)."""
import os, subprocess, sys, tempfile, threading
T = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "latex", "tezaur_tex.py")
ok = True
def check(n, c, x=""):
    global ok; print(("PASS " if c else "FAIL ") + n + ("" if c else " — " + str(x))); ok = ok and bool(c)
def run(*a, env=None):
    return subprocess.run([sys.executable, T, *a], capture_output=True, text=True, env=dict(os.environ, **(env or {})), stdin=subprocess.DEVNULL, timeout=60)
d = tempfile.mkdtemp()
tex = os.path.join(d, "a.tex"); open(tex, "w").write(r"""\documentclass{article}\begin{document}
\section{Intro}
The burden of proof lies with the body assessing the qualification \parencite[p. 1]{lisbon_convention}.
Penguins sing opera in Antarctica every single winter morning loudly \cite{lisbon_convention}. % \cite{ignored}
\end{document}""")
r = run("audit", tex)
check("tex: unsupported sentence flagged, exit 2", r.returncode == 2 and "Penguins" in r.stdout and "1 checks" not in r.stdout, r.stdout + r.stderr)
check("tex: supported sentence not flagged", "burden of proof" not in r.stdout.split("Open the page")[1])
lyx = os.path.join(d, "a.lyx"); open(lyx, "w").write("#LyX 2.3\n\\begin_header\n\\end_header\n\n\\begin_body\n\n\\begin_layout Standard\nPenguins sing opera in Antarctica every single winter morning loudly\n\\begin_inset CommandInset citation\nLatexCommand cite\nkey \"lisbon_convention\"\nliteral \"false\"\n\n\\end_inset\n\n.\n\\end_layout\n\n\\end_body\n\\end_document\n")
r = run("audit", lyx)
check("lyx: citation inset audited", r.returncode == 2 and "Penguins" in r.stdout, r.stdout)
r = run("cite", "burden of proof")
check("cite suggests keys", "\\cite{lisbon_convention}" in r.stdout, r.stdout)
pipe = os.path.join(d, "lyxpipe"); os.mkfifo(pipe + ".in")
got = []
t = threading.Thread(target=lambda: got.append(open(pipe + ".in").read())); t.start()
r = run("cite", "burden of proof", "--lyx", env={"LYXPIPE": pipe}); t.join(10)
check("cite --lyx writes a citation-insert LYXCMD line to the pipe", got and got[0].startswith("LYXCMD:tezaur:citation-insert:"), got)
sys.exit(0 if ok else 1)
