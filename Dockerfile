FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev libcurl4-openssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copiar solo manifiestos primero para cachear dependencias
COPY Cargo.toml Cargo.lock ./

# Crear un src/main.rs dummy para compilar dependencias sin el código real
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Ahora copiar el código real y recompilar (solo lo que cambió)
COPY . .
RUN touch src/main.rs && cargo build --release

FROM node:26-bookworm-slim AS ui-builder

WORKDIR /opt/agora-ui
COPY ui-opentui/package.json ui-opentui/package-lock.json ./
RUN npm ci
COPY ui-opentui/tsconfig.json ./
COPY ui-opentui/src ./src

FROM node:26-bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    chafa \
    fim \
    libcurl4 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash app

RUN mkdir /data && chown app:app /data

COPY --from=builder /app/target/release/agora /usr/local/bin/agora
COPY --from=ui-builder /opt/agora-ui /opt/agora-ui

ENV AGORA_OPENTUI_ENTRY=/opt/agora-ui/src/index.tsx
ENV AGORA_BACKEND_BIN=/usr/local/bin/agora

WORKDIR /data
EXPOSE 2222

CMD ["agora", "--port", "2222", "--db", "postgres://social:agora@db/social"]
