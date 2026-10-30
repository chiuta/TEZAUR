#!/usr/bin/env python3
"""Checks for: persisted BM25 cache, eval, refs hygiene, audit --semantic (mock judge model), lit / refs --online (mock OpenAlex+Crossref).
Everything online or model-based talks to local fakes: this verifies the plumbing and the verdict logic, NOT real-model or real-service quality."""
import json, os, subprocess, sys, tempfile, time

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.environ.get("TEZAUR_BIN") or os.path.join(HERE, "..", "..", "target", "release", "tezaur")
ok = True
def check(name, cond, extra=""):
    global ok
    print(("PASS " if cond else "FAIL ") + name + (" — " + str(extra)[:400] if not cond else ""))
    ok = ok and bool(cond)

d = tempfile.mkdtemp(); lib = os.path.join(d, "lib"); os.makedirs(lib)
open(f"{lib}/lisbon_convention.txt", "w").write("The Lisbon Recognition Convention requires parties to recognise qualifications unless a substantial difference is shown. The burden of proof lies with the body assessing the qualification.")
open(f"{lib}/directive_notes.md", "w").write("Aptitude tests and adaptation periods are compensation measures under the directive.")
open(f"{lib}/refs.bib", "w").write(
 "@article{lisbon_convention, author={Council of Europe}, title={Convention on the Recognition of Qualifications}, year={1997}}\n"
 "@book{directive_notes, author={Smith, Jane}, title={Notes on the Directive}, year={2019}}\n"
 "@article{popescu2019, author={Popescu, Ana}, title={Mutual recognition of professional qualifications}, year={2019}, doi={10.1000/real}}\n"
 "@article{quack2018, author={Quack, Zed}, title={A miracle cure for everything}, year={2018}, doi={10.1000/retr}}\n"
 "@article{rossi2012, author={Rossi, Maria}, title={Free movement of workers in the single market}, year={2012}, doi={10.1000/year}}\n"
 "@article{ghost2020, author={Nobody, N}, title={Entirely invented study}, year={2020}, doi={10.9999/fake}}\n"
 "@article{crossonly, author={Rossi, C}, title={Only in Crossref}, year={2020}, doi={10.2000/cr}}\n")
env = dict(os.environ, TEZAUR_INDEX=f"{lib}/t.idx", TEZAUR_HOST="http://127.0.0.1:11998", TEZAUR_MODEL="m",
           TEZAUR_OPENALEX="http://127.0.0.1:11997", TEZAUR_CROSSREF="http://127.0.0.1:11997")
def run(*a): return subprocess.run([BIN, *a], capture_output=True, text=True, stdin=subprocess.DEVNULL, env=env, cwd=lib)

procs = [subprocess.Popen([sys.executable, os.path.join(HERE, f)] + ([p] if p else []), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) for f, p in (("mock_judge.py", "11998"), ("mock_scholar.py", "11997"))]
try:
    time.sleep(1.0)
    run("ingest", ".")

    # --- BM25 cache on disk
    r = run("search", "burden of proof"); check("search works", "lisbon" in r.stdout.lower(), r.stderr)
    check("bm25 cache written next to the index", os.path.exists(f"{lib}/t.idx.bm25"))
    r2 = run("search", "burden of proof"); check("same result from cache", r.stdout == r2.stdout)
    open(f"{lib}/extra.txt", "w").write("Penguins live in the southern hemisphere and eat fish."); run("ingest", ".")
    r3 = run("search", "penguins fish"); check("cache invalidated after ingest", "extra" in r3.stdout.lower(), r3.stdout[:200])

    # --- eval
    cases = f"{d}/cases.jsonl"
    open(cases, "w").write('{"q":"who has the burden of proof","doc":"lisbon_convention"}\n{"q":"aptitude tests adaptation periods","doc":"directive_notes"}\n{"q":"penguins fish","doc":"lisbon"}\n{"q":"zzz","doc":"missing doc"}\n')
    j = json.loads(run("eval", cases, "--json").stdout)
    check("eval counts only resolvable cases", j["cases"] == 3 and len(j["unresolved_docs"]) == 1, j)
    check("eval recall@1 = 2/3", abs(j["recall@1"] - 0.667) < 0.001 and len(j["misses"]) == 1, j)

    # --- refs hygiene
    draft = f"{d}/draft.md"
    open(draft, "w").write("The burden of proof lies with the body [@lisbon_convention]. Another claim [@ghost2020]. A third [@nowhere].\n")
    j = json.loads(run("refs", draft, "--json").stdout)
    check("refs: key in neither bib nor library", j["missing_everywhere"] == ["nowhere"], j)
    check("refs: bib entry cited but no file", "ghost2020" in j["in_bib_but_not_in_library"], j)
    check("refs: bib never cited", "directive_notes" in j["bib_never_cited"] and "popescu2019" in j["bib_never_cited"], j)
    check("refs: library never cited", any("Notes on the Directive" in t or "directive" in t.lower() for t in j["library_never_cited"]), j)

    # --- semantic audit
    sem = f"{d}/sem.md"
    open(sem, "w").write(
        "Recognition is mandatory for qualifications unless the competent authority establishes a substantial difference [@lisbon_convention]. "
        "The burden of proof does not lie with the body assessing the qualification [@lisbon_convention]. "
        "The burden of proof lies with the body assessing the qualification [@lisbon_convention].\n")
    base = json.loads(run("audit", sem, "--json").stdout)
    check("baseline audit flags the paraphrase", base[0]["verdict"] in ("weak", "unsupported"), base[0])
    check("baseline audit accepts the negation (known weakness)", base[1]["verdict"] == "supported", base[1])
    s = run("audit", sem, "--json", "--semantic"); sj = json.loads(s.stdout)
    check("semantic: paraphrase rescued", sj[0]["verdict"] == "supported" and "[model]" in sj[0]["note"], sj[0])
    check("semantic: negation caught", sj[1]["verdict"] == "contradicted", sj[1])
    check("semantic: plain agreement unchanged", sj[2]["verdict"] == "supported", sj[2])
    check("semantic: announces itself on stderr", "semantic pass" in s.stderr, s.stderr)
    check("--fail counts contradicted", run("audit", sem, "--semantic", "--fail").returncode == 2)

    # --- online (mock OpenAlex + Crossref)
    j = json.loads(run("lit", "recognition qualifications", "--json").stdout)
    check("lit returns works from OpenAlex", j and j[0]["doi"] == "10.1000/real" and j[0]["authors"][0] == "Ana Popescu", j)
    bt = run("lit", "recognition qualifications", "--bibtex").stdout
    check("lit --bibtex emits an entry", "@" in bt and "10.1000/real" in bt, bt)
    check("lit falls back to Crossref", "Only in Crossref" in run("lit", "zzzunknownterm").stdout)
    draft2 = f"{d}/d2.md"
    open(draft2, "w").write("[@popescu2019] [@quack2018] [@rossi2012] [@ghost2020] [@crossonly] [@lisbon_convention]\n")
    on = json.loads(run("refs", draft2, "--json", "--online").stdout)["online"]
    st = {x["key"]: x["status"] for x in on}
    check("online: real DOI ok", st.get("popescu2019") == "ok", st)
    check("online: retraction found", st.get("quack2018") == "retracted", st)
    check("online: year mismatch found", st.get("rossi2012") == "metadata-mismatch", st)
    check("online: invented DOI not found", st.get("ghost2020") == "not-found", st)
    check("online: Crossref-only work ok", st.get("crossonly") == "ok", st)
    check("online: entry without DOI/title is checked by title or unchecked", st.get("lisbon_convention") in ("not-found", "ok", "unchecked"), st)
    # audit --online: same checks on the citations of a draft, JSON shape and exit code
    r = run("audit", draft2, "--json", "--online", "--fail")
    ja = json.loads(r.stdout)
    sta = {x["key"]: x["status"] for x in ja["online"]}
    check("audit --online: findings + online in JSON", isinstance(ja["findings"], list) and sta.get("quack2018") == "retracted" and sta.get("ghost2020") == "not-found", sta)
    check("audit --online: announces what is sent", "sending" in r.stderr and "OpenAlex" in r.stderr, r.stderr)
    check("audit --online --fail: non-zero on retracted/not-found", r.returncode == 2, r.returncode)
    r = run("audit", draft2, "--json")
    check("audit without --online: plain list, nothing sent", isinstance(json.loads(r.stdout), list) and "sending" not in r.stderr, r.stderr)
    # unreachable service must degrade to a clear error, not a crash
    e2 = dict(env, TEZAUR_OPENALEX="http://127.0.0.1:1", TEZAUR_CROSSREF="http://127.0.0.1:1")
    r = subprocess.run([BIN, "lit", "anything"], capture_output=True, text=True, stdin=subprocess.DEVNULL, env=e2, cwd=lib)
    check("unreachable services: clean error", r.returncode != 0 and "panicked" not in r.stderr and "OpenAlex" in r.stderr, r.stderr)
finally:
    for p in procs: p.kill()
sys.exit(0 if ok else 1)
