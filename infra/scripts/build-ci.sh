#!/bin/bash

build_with_tool() {
    $1 build -t ghcr.io/vrrb-io/vrrb -f infra/ci/Dockerfile .
}

auto_detect_tool() {
    if command -v docker &> /dev/null
    then
        build_with_tool docker
        exit
    elif command -v nerdctl &> /dev/null
    then
        build_with_tool nerdctl
        exit
    elif command -v podman &> /dev/null
    then
        build_with_tool podman
        exit
    else
        echo "No container runtime found"
        exit 1
    fi
}

if command -v "$1" &> /dev/null
then
    build_with_tool "$1"
else
    echo "Attempting to auto detect suitable container runtime..."
    auto_detect_tool
fi
