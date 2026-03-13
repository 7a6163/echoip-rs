FROM rust:1.85-slim AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/echoip

COPY --from=builder /build/target/release/echoip ./echoip
COPY html/ html/

EXPOSE 8080

ENTRYPOINT ["/opt/echoip/echoip"]
