FROM rust:1.87.0 as builder

WORKDIR /app
COPY . .

WORKDIR /app/webserver
RUN cargo build --release --bin webserver

FROM debian:bookworm-slim

COPY --from=builder /app/target/release/webserver /usr/local/bin/webserver

EXPOSE 8080
CMD ["webserver"]