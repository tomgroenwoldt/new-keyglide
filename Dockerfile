FROM rust:latest AS builder

WORKDIR /usr/src/keyglide
COPY . .
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --target=x86_64-unknown-linux-musl --release --bin backend

FROM alpine:latest
COPY --from=builder \
    /usr/src/keyglide/target/x86_64-unknown-linux-musl/release/backend \
    /bin/backend
ENTRYPOINT ["/bin/backend"]
