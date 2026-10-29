ARG SERVER_IMAGE=ghcr.io/OWNER/tezaur:latest
FROM ${SERVER_IMAGE} AS server
FROM alpine:3.20
LABEL org.opencontainers.image.title="TEZAUR" \
      org.opencontainers.image.description="Local citation-grounded research assistant" \
      com.docker.desktop.extension.api.version=">= 0.3.4" \
      com.docker.desktop.extension.icon="https://raw.githubusercontent.com/OWNER/tezaur/main/pwa/icon-192.png"
COPY metadata.json docker-compose.yaml /
COPY ui /ui
