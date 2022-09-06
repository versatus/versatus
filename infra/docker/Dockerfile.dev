# Build stage
FROM rust:latest AS builder

RUN update-ca-certificates

WORKDIR /vrrb

COPY ./ .

RUN cargo build --release

# Runner
FROM gcr.io/distroless/cc

WORKDIR /vrrb

ENV RUST_BACKTRACE 1

COPY --from=builder /vrrb/target/release/vrrb ./

USER vrrb

CMD ["/vrrb/vrrb"]
