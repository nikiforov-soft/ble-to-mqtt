FROM rust:1.68.0-bookworm as builder

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN apt-get update && \
    apt-get install -y libdbus-1-dev pkg-config libssl-dev cmake

WORKDIR /work

COPY . .

RUN cargo build --release

FROM debian:bookworm

RUN apt-get update && \
    apt-get install -y openssl bluez

COPY --from="builder" /work/target/release/ble-to-mqtt /app

CMD ["/app", "-v"]
