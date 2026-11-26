# syntax=docker/dockerfile:1
# Pinned tags: Rust 1.85 / Alpine 3.20 (edition2024 estable en Rust 1.85; ver CI).
# Revisit pins with every toolchain upgrade; digests are preferred for reproducible builds.
FROM rust:1.85-alpine AS builder

# musl-dev for the target libc; perl is required by `ring` (rustls crypto backend).
RUN apk add --no-cache musl-dev perl

WORKDIR /app

# Manifests first: dependency fetching is cached until Cargo.toml/Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch --locked

COPY src ./src
RUN cargo build --release --locked --offline

# ---------------------------------------------------------------------------
FROM alpine:3.20 AS runtime

# ca-certificates for outbound HTTPS to Navidrome; wget for the HEALTHCHECK.
# UID/GID 1000 is created explicitly so files and volume mounts stay predictable.
RUN apk add --no-cache ca-certificates wget \
 && addgroup -g 1000 -S bridge \
 && adduser -u 1000 -S -G bridge bridge

WORKDIR /app

# Static musl binary: no OpenSSL, no toolchain, no source in the final image.
COPY --from=builder --chown=1000:1000 \
    /app/target/release/alexa_navidrome_bridge \
    /usr/local/bin/alexa_navidrome_bridge

USER 1000:1000
ENV RUST_LOG=info
EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -q --spider http://127.0.0.1:3000/health || exit 1

ENTRYPOINT ["/usr/local/bin/alexa_navidrome_bridge"]
