#!/bin/bash

cargo run -- node run \
    --id abcdef \
    --idx 1 \
    --data-dir .vrrb \
    --db-path .vrrb/node/node.db \
    --udp-gossip-address 127.0.0.1:8081 \
    --raptorq-gossip-address 127.0.0.1:8082 \
    --http-api-address 127.0.0.1:8080 \
    --http-api-version 1.0.1 \
    --bootstrap-node-addresses 127.0.0.1:8081 \

