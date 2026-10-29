#!/usr/bin/env bash
# Runs every automated check that needs no special hardware: Rust unit tests, then a mock model + a real `tezaur serve` against which
# the CMS plugins, editor integrations, bots, Google Docs logic and the clients are exercised.  Usage: variants/tests/run-all.sh
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"; T="$(mktemp -d)"; BIN="$ROOT/target/release/tezaur"
cleanup() { kill $MOCK $SRV 2>/dev/null; rm -rf "$T"; }; trap cleanup EXIT
fail=0; step() { echo; echo "== $*"; }
step "cargo test"; (cd "$ROOT" && cargo test --release 2>&1 | grep -E "^test result" ) || fail=1
[ -x "$BIN" ] || (cd "$ROOT" && cargo build --release) || exit 1
python3 "$ROOT/variants/tests/mock_ollama.py" 11999 & MOCK=$!
mkdir -p "$T/lib"; printf 'The Lisbon Recognition Convention requires parties to recognise qualifications unless a substantial difference is shown. The burden of proof lies with the body assessing the qualification.' > "$T/lib/lisbon_convention.txt"
printf 'Aptitude tests and adaptation periods are compensation measures under the directive.' > "$T/lib/directive_notes.md"
printf '@article{lisbon_convention, author={Council of Europe}, title={Convention on the Recognition of Qualifications}, year={1997}}\n@book{directive_notes, author={Smith, Jane and Doe, John}, title={Notes on the Directive}, publisher={Press}, year={2019}}\n' > "$T/lib/refs.bib"
export TEZAUR_INDEX="$T/lib/t.idx" TEZAUR_HOST=http://127.0.0.1:11999 TEZAUR_MODEL=m TEZAUR_SERVER=http://127.0.0.1:8810
(cd "$T/lib" && "$BIN" ingest . >/dev/null 2>&1); "$BIN" serve --port 8810 >"$T/serve.log" 2>&1 & SRV=$!; sleep 1.5
step "CMS plugins (DokuWiki, WordPress)"; php "$ROOT/variants/cms/tests/test.php" || fail=1
step "Editors, LSP, Raycast, Alfred, Joplin, Logseq"; python3 "$ROOT/variants/editors/tests/test_editors.py" 2>&1 | grep -v WARNING || fail=1
step "LyX / LaTeX tooling"; python3 "$ROOT/variants/editors/tests/test_latex.py" | grep -v ^PASS; [ "${PIPESTATUS[0]}" = 0 ] || fail=1
step "Bots (Slack, Discord, WhatsApp, e-mail)"; (cd "$ROOT/variants/bots" && python3 test_bots2.py 2>&1 | grep -v WARNING) || fail=1
step "Google Docs add-on logic"; node "$ROOT/variants/gdocs/test.js" "$ROOT/variants/gdocs" || fail=1
step "Audit, coverage, novelty, receipts (CLI, API, MCP)"; TEZAUR_BIN="$BIN" python3 "$ROOT/variants/tests/test_insight.py" | grep -v ^PASS || fail=1
step "Round 6: BM25 cache, eval, refs, semantic audit, OpenAlex/Crossref (mock services)"; TEZAUR_BIN="$BIN" python3 "$ROOT/variants/tests/test_round6.py" | grep -v ^PASS; [ "${PIPESTATUS[0]}" = 0 ] || fail=1
step "HTML audit panel in a real browser"; TEZAUR_BIN="$BIN" python3 "$ROOT/variants/tests/test_panel.py" | grep -v ^PASS; [ "${PIPESTATUS[0]}" = 0 ] || fail=1
step "Jupyter: magics executed in a real kernel"; python3 "$ROOT/variants/notebooks/test_notebook.py" 2>&1 | grep -E "^(PASS|FAIL)" | grep -v ^PASS; [ "${PIPESTATUS[0]}" = 0 ] || fail=1
step "Meta-installer (fake home with fake applications)"; [ -f "$ROOT/dist/plugins/tezaur-setup.py" ] || bash "$ROOT/variants/build-plugins.sh" >/dev/null 2>&1; python3 "$ROOT/variants/installer/test_installer.py" | grep -v ^PASS; [ "${PIPESTATUS[0]}" = 0 ] || fail=1
echo; [ $fail = 0 ] && echo "ALL PASSED" || { echo "SOME FAILED"; exit 1; }
