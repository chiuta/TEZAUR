# TEZAUR Mesh — independent research nodes that exchange material directly

No central server, no account, no cloud. Each node is a normal TEZAUR (CLI/`tezaur serve`, a browser tab, a phone) that *chooses* what it offers.

```
   ┌── laptop A (tezaur serve) ──── LAN broadcast / mDNS ──── laptop B ──┐
   │        ▲  │ HTTP + token                                   │        │
   │ USB .tzb  └─── Tor onion / Tailscale / VPN ─── lab server ─┘        │
   └── phone browser ◄── WebRTC data channel (codes pasted by hand) ──► colleague's browser
```

## Four ways to exchange (all opt-in)
| Mode | What moves | Needs |
|---|---|---|
| **Federated search** `tezaur mesh search …`, `GET /api/search?mesh=1`, "include peers" box in server mode | only the matching *snippets*, with the peer's citation label | the peer's `tezaur serve` + token |
| **Pull** `tezaur mesh pull [peer]` | the peer's shared documents as extracted text + page numbers, stored under `mesh/<peer>/…` and re-embedded locally | same |
| **Bundle** `tezaur mesh export x.tzb` / `import` (also in the browser) | the same, in a zip: USB stick, e-mail, AirDrop — fully offline | nothing |
| **WebRTC browser↔browser** ("Mesh P2P" panel) | search and pull over a direct data channel; the invitation/answer codes are copied through any channel (chat, QR printed by you, paper) | two browsers; on one LAN no internet at all; across the internet tick "STUN" (asks a Google server for your public address) |

`tezaur mesh discover` finds servers on the LAN (UDP beacon, enabled when a server is bound to a network address or `TEZAUR_MESH_BEACON=1`).
Online without opening ports: a Tor onion service or Tailscale (`mesh add NAME URL --proxy socks5h://127.0.0.1:9050`; see `deploy/tor/README.md`).

## Trust model (read this)
- **Nothing is offered until you say so**: `tezaur mesh share smith lisbon` (substrings of path/title) or `*`. Documents received from others are **not re-offered** unless `--relay`.
- A peer sees only snippets of *shared* documents; it never sees your questions' answers, your other documents or your model.
- Content-addressed: a document whose hash you already hold is never stored twice, however many peers offer it. Received paths are namespaced (`mesh/<peer>/`) and stripped of `..`.
- **Received text is untrusted.** It enters your prompts like any excerpt; a hostile peer could plant instructions in a document (prompt injection) or false facts. TEZAUR labels such passages `[via peer]` so the citation shows where they came from; verify before citing.
- **Copyright**: sharing PDFs/books with colleagues may be restricted by licences. Mesh exchanges extracted text, not the original files, but that does not make it free of rights. Share your own notes, open-access and licensed-for-sharing material.
- Transport is plain HTTP with a bearer token: use a VPN/Tor/HTTPS reverse proxy off the local network. WebRTC data channels are encrypted (DTLS) but the manual codes are only as private as the channel you send them through.
- Browser ↔ Rust document hashes differ (the browser hashes extracted text, Rust hashes file bytes), so the same book ingested on both sides may be copied once more.

## What was tested
Two real `tezaur serve` nodes (token, share rules, no leak of unshared documents, pull, dedupe on second pull, wrong-token refusal, UDP discovery, SOCKS5 proxy path),
bundle round-trips in both directions between Chromium and the Rust CLI, and two isolated Chromium contexts connected over WebRTC with only pasted codes
(search, pull, no-op second pull, unshared document not returned), and a federated *ask* in server mode that cites a peer's passage.
**Not tested**: different machines, NAT traversal over the internet (STUN/TURN), Tor itself, mDNS across real Wi-Fi, large libraries.

## Protocol sketch (for implementers)
`GET /api/mesh/manifest` → `{node:{id,name}, docs:[{path,title,hash,pages,chunks}]}` · `GET /api/mesh/doc?path=` → `{path,title,hash,pages,chunks:[{page,text}]}` ·
`GET /api/mesh/search?q=&k=` → `{hits:[{label,text,page,path,title,score}]}` · bundle = zip(`manifest.json {format:"tezaur-bundle/1"}`, `docs/NNNNN.json`, optional `refs.bib`) ·
UDP probe `TEZAUR-MESH?` on port 8788 → `{"tezaur":1,"id","name","port"}`. Full list: `editors/rest/openapi.yaml`.
