# The Rust server image: `tezaur serve` with PDF/OCR tools. Build from the repository root.
#   docker build -t tezaur-server .
#   docker run -p 8787:8787 -v tezaur-data:/data -e TEZAUR_TOKEN=secret -e TEZAUR_HOST=http://host.docker.internal:11434 tezaur-server
FROM rust:1-slim AS build
WORKDIR /src
COPY . .
RUN cargo build --release --bin tezaur

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends poppler-utils tesseract-ocr tesseract-ocr-eng tesseract-ocr-ron ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/tezaur /usr/local/bin/tezaur
ENV TEZAUR_INDEX=/data/tezaur.idx
VOLUME /data
EXPOSE 8787
# --bind 0.0.0.0 requires a token: set TEZAUR_TOKEN
ENTRYPOINT ["tezaur", "serve", "--bind", "0.0.0.0", "--port", "8787", "--workspaces", "/data/workspaces"]
