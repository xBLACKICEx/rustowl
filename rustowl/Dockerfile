FROM rust:1-slim-bookworm AS builder

WORKDIR /app
COPY . .

RUN cargo build --release --locked

ENTRYPOINT ["cargo", "run", "--release"]
