FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    chafa \
    catimg \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash app

RUN mkdir /data && chown app:app /data

COPY --from=builder /app/target/release/cli-red-social /usr/local/bin/cli-red-social

USER app
WORKDIR /data

EXPOSE 2222

CMD ["cli-red-social", "--port", "2222", "--db", "postgres://social:social@db/social"]
