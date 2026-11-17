# Package-manager manifests

Scoop, winget and Homebrew point at the published release 0.5.3 with its real checksums (computed from the downloaded assets). For a newer release, change the version and the checksum (`sha256sum file`, or the `SHA256SUMS-*.txt` attached to each release); Scoop's `autoupdate` does this by itself. They were written from the packaging formats' documentation and have **not**
been run through the managers' own linters/builders — treat each as a starting point.

| Manager | File | Builds from |
|---|---|---|
| Snap (Ubuntu Store) | `snap/snapcraft.yaml` | source (Rust plugin) |
| Flatpak (Flathub) | `flatpak/tf.trom.Tezaur.yml` | source (needs `cargo-sources.json`, see header) |
| Homebrew (macOS/Linux) | `homebrew/tezaur.rb` | source |
| Scoop (Windows) | `scoop/tezaur.json` | Windows zip from `release.yml` |
| winget (Windows) | `winget/*.yaml` | Windows zip from `release.yml` |
| AUR (Arch) | `aur/PKGBUILD` | source |
