#!/usr/bin/env bash
# Sign the APT and RPM repositories built by CI.
#   GPG_PRIVATE_KEY   ASCII-armored private key (a repository secret). No key -> nothing is signed, the script exits 0 and the
#                     repositories stay unsigned (see README). The key must have NO passphrase (a CI-only signing subkey is ideal).
# usage: sign.sh deb <repo/apt>      writes Release.gpg, InRelease, tezaur.gpg (binary key), tezaur.asc (armored key)
#        sign.sh rpm-pkgs <dir>      signs every .rpm in <dir> (run BEFORE createrepo_c)
#        sign.sh rpm-meta <repo/rpm> signs repodata/repomd.xml and writes tezaur.asc next to it
# Prints "signed" on success, "unsigned" when there is no key.
set -euo pipefail
cmd=${1:?usage}; dir=${2:?dir}
export GNUPGHOME; GNUPGHOME=$(mktemp -d); chmod 700 "$GNUPGHOME"; trap 'gpgconf --kill gpg-agent 2>/dev/null || true; rm -rf "$GNUPGHOME"' EXIT
if [ -n "${GPG_PRIVATE_KEY:-}" ]; then
  printf '%s\n' "$GPG_PRIVATE_KEY" | gpg --batch --quiet --import
else
  echo unsigned; exit 0
fi
FPR=$(gpg --batch --list-secret-keys --with-colons | awk -F: '/^fpr:/{print $10; exit}')
case $cmd in
  deb)
    cd "$dir"
    gpg --batch --yes --armor --detach-sign -u "$FPR" -o Release.gpg Release
    gpg --batch --yes --clearsign -u "$FPR" -o InRelease Release
    gpg --batch --export "$FPR" > tezaur.gpg; gpg --batch --armor --export "$FPR" > tezaur.asc ;;
  rpm-pkgs)
    for f in "$dir"/*.rpm; do rpmsign --define "_gpg_name $FPR" --define "_gpg_path $GNUPGHOME" --addsign "$f" >/dev/null; done ;;
  rpm-meta)
    gpg --batch --yes --armor --detach-sign -u "$FPR" -o "$dir/repodata/repomd.xml.asc" "$dir/repodata/repomd.xml"
    gpg --batch --armor --export "$FPR" > "$dir/tezaur.asc" ;;
  *) echo "unknown command $cmd" >&2; exit 1 ;;
esac
echo signed
