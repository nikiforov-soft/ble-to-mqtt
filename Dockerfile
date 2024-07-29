FROM rust:1.80-bookworm AS builder

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN apt-get update && \
    apt-get install -y libdbus-1-dev pkg-config cmake

WORKDIR /src

COPY . .

RUN cargo build --release

FROM debian:bookworm

RUN apt-get update && \
    apt-get install -y libdbus-1-3

COPY --from="builder" /src/target/release/ble-to-mqtt /app

CMD ["/app", "-v"]
