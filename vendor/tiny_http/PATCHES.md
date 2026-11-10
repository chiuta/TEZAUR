# Local patch to tiny_http 0.12.0

This directory is `tiny_http` 0.12.0 from crates.io (MIT OR Apache-2.0, see LICENSE-MIT and LICENSE-APACHE) with one change:

* `src/util/equal_reader.rs`, `Drop for EqualReader`: upstream allocates `vec![0; remaining_to_read]`, where
  `remaining_to_read` is the Content-Length the client announced for a request body that nobody read. A single
  request with `Content-Length: 99999999999` made `tezaur serve` abort with "memory allocation failed" (the server
  answered 413 and then died; the same happens for every early answer such as 401 or 403). The patch drains the unread
  body in 8 KiB pieces and, when more than 1 MiB is left unread, does not drain at all: the connection is reported as
  failed and closed.

Found by the adversarial review of 2026-11-10. 0.12.0 is the latest upstream release at the time of writing; remove
this directory and the `[patch.crates-io]` entry in `/Cargo.toml` when upstream fixes it.
* `src/lib.rs`: `#![allow(unused_imports, dead_code)]` added on top (upstream warnings only).
