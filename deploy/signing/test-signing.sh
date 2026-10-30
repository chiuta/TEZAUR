#!/usr/bin/env bash
# End-to-end test of sign.sh with a THROW-AWAY key passed exactly like the real secret: build both repos, sign them, install from them in
# clean containers with signature checking ON, and prove that a tampered repository is REJECTED.
# Needs docker, dpkg-dev, apt-utils, createrepo-c, rpm, gnupg. usage: test-signing.sh <file.deb> <file.rpm>
set -euo pipefail
deb=$(realpath "$1"); rpm=$(realpath "$2"); here=$(cd "$(dirname "$0")" && pwd)
W=$(mktemp -d); export GNUPGHOME=$W/gh; mkdir -m 700 "$GNUPGHOME"
gpg --batch --quiet --passphrase '' --quick-generate-key "TEZAUR test key <test@invalid>" rsa3072 sign never
export GPG_PRIVATE_KEY; GPG_PRIVATE_KEY=$(gpg --batch --armor --export-secret-keys); unset GNUPGHOME
mkdir -p "$W/r/apt" "$W/r/rpm"; cp "$deb" "$W/r/apt/"; cp "$rpm" "$W/r/rpm/"
( cd "$W/r/apt" && dpkg-scanpackages --multiversion . > Packages && gzip -k Packages && apt-ftparchive release . > Release )
[ "$("$here/sign.sh" deb "$W/r/apt")" = signed ]
[ "$("$here/sign.sh" rpm-pkgs "$W/r/rpm")" = signed ]
createrepo_c -q "$W/r/rpm"
[ "$("$here/sign.sh" rpm-meta "$W/r/rpm")" = signed ]
# without a key the script must do nothing and say so
[ "$(env -u GPG_PRIVATE_KEY "$here/sign.sh" deb "$W/r/apt")" = unsigned ]
cp -r "$W/r" "$W/bad"; echo "tampered" >> "$W"/bad/apt/*.deb   # Packages (signed through Release) lists the .deb hash, so apt must refuse this file
serve() { (cd "$1" && python3 -m http.server "$2" >/dev/null 2>&1 &); }
serve "$W/r" 8090; serve "$W/bad" 8091; sleep 2
ok=1
say() { echo "$1: $2"; case $2 in PASS) echo "::notice::$1 PASS";; *) echo "::error::$1 $2";; esac; }
docker run --rm --network host debian:stable-slim bash -c "apt-get update -qq >/dev/null; apt-get install -y -qq gnupg >/dev/null; \
  curl -sf http://127.0.0.1:8090/apt/tezaur.gpg -o /usr/share/keyrings/tezaur.gpg 2>/dev/null || { apt-get install -y -qq curl >/dev/null; curl -sf http://127.0.0.1:8090/apt/tezaur.gpg -o /usr/share/keyrings/tezaur.gpg; }; \
  echo 'deb [signed-by=/usr/share/keyrings/tezaur.gpg] http://127.0.0.1:8090/apt ./' > /etc/apt/sources.list.d/t.list && apt-get update -qq && apt-get install -y -qq tezaur && tezaur --version" && say apt-signed-install PASS || { say apt-signed-install FAIL; ok=0; }
# repository signed with the right key but with a tampered Packages file: apt must refuse
if docker run --rm --network host debian:stable-slim bash -c "apt-get update -qq >/dev/null; apt-get install -y -qq curl >/dev/null; curl -sf http://127.0.0.1:8091/apt/tezaur.gpg -o /usr/share/keyrings/tezaur.gpg; \
  echo 'deb [signed-by=/usr/share/keyrings/tezaur.gpg] http://127.0.0.1:8091/apt ./' > /etc/apt/sources.list.d/t.list && apt-get update 2>&1 | tee /tmp/o; apt-get install -y tezaur"; then say apt-tampered-rejected FAIL; ok=0; else say apt-tampered-rejected PASS; fi
docker run --rm --network host fedora:latest bash -c "rpm --import http://127.0.0.1:8090/rpm/tezaur.asc && printf '[t]\nname=t\nbaseurl=http://127.0.0.1:8090/rpm/\nenabled=1\ngpgcheck=1\nrepo_gpgcheck=1\ngpgkey=http://127.0.0.1:8090/rpm/tezaur.asc\n' > /etc/yum.repos.d/t.repo && dnf install -y tezaur && tezaur --version && rpm -qi tezaur | grep -i signature" && say rpm-signed-install PASS || { say rpm-signed-install FAIL; ok=0; }
# an unsigned/other-key repo must be refused when gpgcheck=1: use the signed repo but with a DIFFERENT key
if docker run --rm --network host fedora:latest bash -c "printf '[t]\nname=t\nbaseurl=http://127.0.0.1:8090/rpm/\nenabled=1\ngpgcheck=1\nrepo_gpgcheck=1\ngpgkey=file:///etc/hostname\n' > /etc/yum.repos.d/t.repo && dnf install -y tezaur"; then say rpm-without-key-rejected FAIL; ok=0; else say rpm-without-key-rejected PASS; fi
[ $ok = 1 ]
