# Adversarial checks (kept from the 0.5.1 review)

* `evil_oai.py MODE PORT` is a hostile OAI-PMH archive (modes: bomb, deep, loop, endless, emptypages, big, tags, ctrl, slow, redirect, cdata).
  `oai_run.py TEZAUR WORKDIR MODE...` starts one per mode and runs `tezaur oai` against it (set `NO_PROXY=127.0.0.1`).
  Expected: no crash, no hang beyond the 300 s request limit, no request to any other host.
* `serve_content_length.py TEZAUR WORKDIR` checks that a huge `Content-Length` does not kill `tezaur serve` (it did in 0.5.0).
* The parser and exporter cases (hostile `.bib`/RIS/CSL, LaTeX injection in `export`) are unit tests in `src/sanitize.rs`, `src/export.rs` and `src/oaipmh.rs`.
