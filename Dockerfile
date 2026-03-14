FROM rust:1.94-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
    && cargo build --release \
    && rm -rf src

# Build actual source
COPY src/ src/
RUN touch src/main.rs src/lib.rs && cargo build --release

FROM gcr.io/distroless/cc-debian13

WORKDIR /opt/echoip

COPY --from=builder /build/target/release/echoip ./echoip
COPY html/ html/

EXPOSE 8080

ENTRYPOINT ["/opt/echoip/echoip"]
