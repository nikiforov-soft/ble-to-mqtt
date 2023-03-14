FROM rust:1.68.0-bookworm as builder

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN apt-get update && \
    apt-get install -y libdbus-1-dev pkg-config libssl-dev cmake

WORKDIR /work

COPY . .

RUN cargo build --release

FROM debian:bookworm

ARG VERSION=0.0.0

RUN apt-get update && \
    apt-get install -y openssl bluez

COPY --from="builder" /work/target/release/ble-to-mqtt /app

CMD ["/app", "-v"]

# Metadata
LABEL org.opencontainers.image.vendor="Rumen Nikiforov" \
    org.opencontainers.image.url="https://github.com/nikiforov-soft/ble-to-mqtt" \
    org.opencontainers.image.title="BLE to MQTT bridge" \
    org.opencontainers.image.description="Send all Bluetooth Low Energy events to MQTT topic" \
    org.opencontainers.image.version="$VERSION" \
    org.opencontainers.image.documentation="https://github.com/nikiforov-soft/ble-to-mqtt"
