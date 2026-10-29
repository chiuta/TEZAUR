# TEZAUR server on YunoHost (packaging v2.1)

Installs the real Rust server (OCR, workspaces, editors' API, mesh) as a systemd service behind nginx, on its own (sub)domain.
The web app is protected by the YunoHost portal; `/api` and `/llm` are protected by an access token (so Obsidian/VS Code/WordPress/bots/peers can call
them): `sudo yunohost app setting tezaur_server token`. The model server (Ollama…) is not installed by this package.

Install: `sudo yunohost app install https://github.com/chiuta/tezaur_server_ynh` (after replacing OWNER and the sha256 in `manifest.toml`
and publishing the release binary). **Written against the packaging-v2 documentation; not linted with `package_linter` and never installed on a
YunoHost instance.** The first install will probably need small fixes: treat it as a template.
