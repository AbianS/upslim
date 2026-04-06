# ──────────────────────────────────────────────
# Stage 1: Build — Alpine con musl para static binary
# Build context: workspace root (upslim/)
# ──────────────────────────────────────────────
FROM rust:1.86-alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app

# Cachear dependencias: copiamos los manifiestos del workspace
# y creamos stubs de source para que cargo descargue las deps
COPY Cargo.toml Cargo.lock ./
COPY packages/server/Cargo.toml packages/server/Cargo.toml
RUN mkdir -p packages/server/src && \
    echo "fn main() {}" > packages/server/src/main.rs && \
    echo "" > packages/server/src/lib.rs
RUN cargo build --release -p upslim-server 2>/dev/null; true

# Build real con el source completo
COPY packages/server/src packages/server/src
RUN touch packages/server/src/main.rs packages/server/src/lib.rs && \
    cargo build --release -p upslim-server && \
    strip target/release/upslim-server

# ──────────────────────────────────────────────
# Stage 2: Runtime — scratch (solo binario + CA certs)
# ──────────────────────────────────────────────
FROM scratch

# CA certificates para HTTPS checks
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Solo el binario
COPY --from=builder /app/target/release/upslim-server /upslim-server

# Volúmenes esperados
VOLUME ["/config"]
VOLUME ["/data"]

ENTRYPOINT ["/upslim-server", "--config", "/config"]
