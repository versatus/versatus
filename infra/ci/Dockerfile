# Build stage
FROM rust:latest AS builder

RUN update-ca-certificates

RUN apt update
RUN apt -y install make clang pkg-config libssl-dev llvm

WORKDIR /vrrb

COPY ./ .

RUN cargo build --release

# Runner
FROM gcr.io/distroless/cc

WORKDIR /vrrb

ENV RUST_BACKTRACE 1

COPY --from=builder /vrrb/target/release/vrrb ./

CMD ["/vrrb/vrrb", "node", "run"]
