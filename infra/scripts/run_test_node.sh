#!/bin/bash

cargo run -- node run \
    --id abcdef \
    --node-idx 1 \
    --data-dir .vrrb \
    --db-path .vrrb/node/node.db \
    --address 127.0.0.1:8080 \
    --bootstrap-node-addr 127.0.0.1:8081 \

