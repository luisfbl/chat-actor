FROM rust:1.87.0 as builder

WORKDIR /app
COPY . .

WORKDIR /app/websocket
RUN cargo build --release --bin websocket

FROM debian:bookworm-slim

COPY --from=builder /app/target/release/websocket /usr/local/bin/websocket

EXPOSE 9002
CMD ["websocket"]