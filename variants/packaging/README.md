# Package-manager manifests

These need a **published release** before they can work: replace `OWNER` with your GitHub account/organisation, the version,
and every `SHA256-OF-…` value (`sha256sum file`). They were written from the packaging formats' documentation and have **not**
been run through the managers' own linters/builders — treat each as a starting point.

| Manager | File | Builds from |
|---|---|---|
| Snap (Ubuntu Store) | `snap/snapcraft.yaml` | source (Rust plugin) |
| Flatpak (Flathub) | `flatpak/tf.trom.Tezaur.yml` | source (needs `cargo-sources.json`, see header) |
| Homebrew (macOS/Linux) | `homebrew/tezaur.rb` | source |
| Scoop (Windows) | `scoop/tezaur.json` | Windows zip from `release.yml` |
| winget (Windows) | `winget/*.yaml` | Windows zip from `release.yml` |
| AUR (Arch) | `aur/PKGBUILD` | source |
