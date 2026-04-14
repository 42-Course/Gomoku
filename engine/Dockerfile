FROM rust:1.83-slim AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/engine /usr/local/bin/engine

CMD ["cp", "/usr/local/bin/engine", "/out/engine"]
