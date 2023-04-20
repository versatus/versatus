# Stage 1: Build the VRRB project
FROM rust:1.57 as builder
WORKDIR /usr/src/vrrb

# Install the necessary system dependencies
RUN apt-get update && \
    apt-get install -y libclang-dev llvm-dev

# Copy the source code and compile the project
COPY . .
RUN make build

# Stage 2: Create the runtime container with Alpine Linux
FROM alpine:latest

# Install required dependencies
RUN apk --no-cache add ca-certificates && \
    apk add --no-cache libgcc libstdc++

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/vrrb/target/release/vrrb /usr/local/bin/vrrb

# Expose required ports (customize these according to your network requirements)
EXPOSE 8080 9293

# Run the VRRB node
CMD ["/usr/local/bin/vrrb", "node", "--help"]
