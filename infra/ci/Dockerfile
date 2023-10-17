# Build stage
FROM rust:latest AS builder

RUN update-ca-certificates

RUN apt update
RUN apt -y install make clang pkg-config libssl-dev llvm

WORKDIR /versatus

COPY ./ .

RUN cargo build --release

# Runner
FROM gcr.io/distroless/cc

WORKDIR /versatus

ENV RUST_BACKTRACE 1

COPY --from=builder /versatus/target/release/versatus ./

CMD ["/versatus/versatus", "node", "run"]
