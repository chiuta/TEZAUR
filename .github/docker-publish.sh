#!/usr/bin/env bash
set -euo pipefail
OWNER=$(echo "$1" | tr 'A-Z' 'a-z')
docker build -t ghcr.io/$OWNER/tezaur-server:latest .
python3 variants/build-web.py variants/docker/site
cp pwa/manifest.webmanifest pwa/sw.js pwa/icon-*.png variants/docker/site/
docker build -t ghcr.io/$OWNER/tezaur-web:latest variants/docker
docker push ghcr.io/$OWNER/tezaur-server:latest
docker push ghcr.io/$OWNER/tezaur-web:latest
