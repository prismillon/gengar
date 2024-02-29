FROM rust:latest AS builder
RUN cargo new --bin app
WORKDIR /app
COPY Cargo.* ./
RUN cargo build --release
COPY src/*.rs ./src/.
RUN touch -a -m ./src/main.rs
RUN cargo build --release

FROM debian
WORKDIR /app
COPY --from=builder /app/target/release/gengar /app/gengar
CMD "/app/gengar"