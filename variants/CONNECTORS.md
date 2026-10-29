# Connecting TEZAUR to AI assistants

Two routes, depending on where the assistant runs.

## 1. Assistant on your own computer → MCP over stdio (`tezaur mcp`)
`python3 tezaur-setup.py install` writes the entry for every client it finds:

| Client | What it edits | Status |
|---|---|---|
| Claude Desktop | `claude_desktop_config.json` → `mcpServers.tezaur` | standard format; tested only against a fake config |
| Claude Code | `claude mcp add --scope user` | command checked through a shim; real `claude` not run |
| Cursor · Windsurf · Cline · Roo Code · Gemini CLI · LM Studio | their `mcp.json`-style files, `mcpServers.tezaur` | same JSON shape; paths from each tool's documentation; none run |
| VS Code (MCP) | `Code/User/mcp.json` → `servers.tezaur` (`type: stdio`) | documented shape; not run |
| Continue | `~/.continue/mcpServers/tezaur.json` | documented shape; not run |
| Zed | `settings.json` → `context_servers.tezaur` | Zed settings usually contain comments → the installer then only prints what to add |
| OpenAI Codex CLI | marked block in `~/.codex/config.toml` (`[mcp_servers.tezaur]`) | TOML written by hand; not run |
| Goose · Jan · AnythingLLM · Open WebUI | UI-only: the installer prints the exact fields | — |

Tools offered: `tezaur_search`, `tezaur_sources`, `tezaur_read`, `tezaur_audit`, `tezaur_coverage`. They are read-only; the assistant's own model
does the writing, so no local model is needed.

## 2. Assistant in the cloud (ChatGPT, Claude.ai, Gemini, Perplexity…) → remote connector
Cloud assistants cannot reach `127.0.0.1`. `tezaur serve` now speaks MCP over HTTP at `POST /mcp` (one JSON-RPC message per request, JSON replies,
no SSE stream, no sessions) and publishes its API description at `GET /openapi.json`.

```
tezaur serve --bind 127.0.0.1 --port 8787 --token LONG-RANDOM-SECRET
# expose it yourself over HTTPS: a Cloudflare Tunnel, Tailscale Funnel, your YunoHost reverse proxy …
#   connector URL:  https://your-host.example/mcp      auth: header  Authorization: Bearer LONG-RANDOM-SECRET
```
**Think before doing this**: it hands whoever holds the token — and the AI provider — passages of your library. That is the opposite of the "nothing
leaves your machine" default, so TEZAUR never does it on its own. Share only a workspace (`--workspaces`) that holds what you are willing to expose.
Clients that insist on SSE streaming or OAuth will not work with this minimal endpoint. I tested `/mcp` with `curl` only, not with ChatGPT or Claude.ai.

OpenAPI-based tools: Open WebUI (Settings → Tools → add `https://…/openapi.json`) and Custom GPT Actions can import the spec directly; untested.

## Not an AI connector, but related
Ollama / Jan / LM Studio are the *model* side (`--api`, `--host`); they are what TEZAUR talks to, not clients of it.
