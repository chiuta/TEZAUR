# Signing

Two independent layers, both optional to enable and both tested in CI:

1. **Keyless build provenance (works now, no secret).** `release.yml` attests every release archive and `.deb` with Sigstore
   (`actions/attest-build-provenance`). Check a download: `gh attestation verify tezaur-*.tar.gz -R chiuta/TEZAUR`.
   This proves *which workflow run on which commit built the file*; it does not depend on any key you hold.
2. **GPG-signed APT and RPM repositories (needs your key).** `pages.yml` signs the repositories when the repository secret
   `GPG_PRIVATE_KEY` exists, and publishes `apt/tezaur.gpg`, `apt/InRelease`, `apt/Release.gpg`, `rpm/tezaur.asc`,
   `rpm/repodata/repomd.xml.asc`. Without the secret the repositories stay unsigned (`trusted=yes` / `gpgcheck=0`) and the workflow says so.

## Create the key (on your own computer — the private key never goes into a chat)

```bash
gpg --quick-generate-key "TEZAUR packages <you@example.org>" rsa4096 sign 2y   # asks for a passphrase: leave it EMPTY for a CI-only key
gpg --list-secret-keys --keyid-format long                                      # note the key id
gpg --armor --export-secret-keys KEYID | gh secret set GPG_PRIVATE_KEY -R chiuta/TEZAUR
gpg --armor --export KEYID > tezaur-public.asc                                  # keep a copy of the public key somewhere you trust
```

Use a key made only for this repository (the CI signs non-interactively, so it cannot have a passphrase); revoke it if the secret ever leaks.
Re-run the `pages` workflow; `verify-live` then installs from the published site with signature checking on.

## What the test proves

`test-signing.sh` (workflow `signing-test`) uses a throw-away key passed exactly like the real secret, and checks that a signed repository
installs with checking ON, that a tampered `Packages` file is refused by apt, and that dnf refuses the repository when the key does not match.
It does **not** prove anything about your real key (which only you hold) or about third-party mirrors.
