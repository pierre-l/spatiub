# Build
FROM rust:1.30 as build

RUN USER=root cargo new --bin spatiub
WORKDIR /spatiub
RUN rm src/*.rs

COPY ./demo_client ./demo_client
COPY ./demo_core ./demo_core
COPY ./demo_server ./demo_server
COPY ./lib ./lib
COPY ./Cargo.toml ./Cargo.toml

ENV RUSTFLAGS "-C target-cpu=native"
RUN cargo build --release

# Runtime
FROM debian:stretch-slim

COPY --from=build /spatiub/target/release/spatiub* ./
CMD ["./spatiub"]