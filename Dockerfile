# ── Build stage ──────────────────────────────────────────────────────────────
FROM rust:1.82-slim-bookworm AS builder

WORKDIR /app

# Cache dependencies separately from your source code.
# Copy only the manifest files first, build a dummy lib to warm the cache,
# then copy the real source. This avoids a full recompile on every code change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Now copy the real source and build
COPY src ./src
COPY static ./static
RUN touch src/main.rs \
    && cargo build --release

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Run as a non-root user
RUN useradd -ms /bin/bash appuser
USER appuser

WORKDIR /app

COPY --from=builder /app/target/release/rPC ./rpc

EXPOSE 8088

CMD ["./rpc"]
