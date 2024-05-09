FROM rust:latest AS builder
RUN cargo new --bin app
WORKDIR /app
COPY Cargo.* ./
RUN cargo build --release --target x86_64-unknown-linux-gnu
COPY src/*.rs ./src/.
RUN touch -a -m ./src/main.rs
RUN cargo build --release --bins --target x86_64-unknown-linux-gnu

FROM scratch
USER 1000
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/gengar /gengar
CMD ["./gengar"]