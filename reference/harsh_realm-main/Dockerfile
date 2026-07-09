# ─── Harsh Realm — Backend ────────────────────────────────────────────────────
# Multi-stage build: build the Vue frontend, build the Rust web host
# (harsh-web), then assemble a slim runtime image that serves both.
#
# Build:  docker build -t harsh-realm .
# Run:    docker run -p 8080:8080 -v ./worlds:/app/worlds harsh-realm
# ──────────────────────────────────────────────────────────────────────────────

# ── Stage 1: build the Vue frontend → frontend/dist ───────────────────────────
FROM node:20-slim AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build


# ── Stage 2: build the Rust web host (harsh-web) ──────────────────────────────
FROM rust:1-slim-bookworm AS backend
# rusqlite is built with the bundled SQLite (compiled from C), so a C toolchain
# is required.
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY crates/ ./crates/
RUN cargo build --release --manifest-path crates/harsh-web/Cargo.toml --bin harsh-web


# ── Stage 3: runtime image ────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# The compiled server binary.
COPY --from=backend /app/crates/harsh-web/target/release/harsh-web /usr/local/bin/harsh-web
# The built frontend (served as static assets) + authored content packs.
COPY --from=frontend /app/frontend/dist ./frontend/dist
COPY content/ ./content/

# Worlds directory is typically a bind-mounted volume.
RUN mkdir -p /app/worlds

ENV HARSH_REALM_HOST=0.0.0.0 \
    HARSH_REALM_PORT=8080 \
    HARSH_REALM_BASE_DIR=/app

EXPOSE 8080

CMD ["harsh-web"]
