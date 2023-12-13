#!/bin/bash

build_with_tool() {
    local tool=$1
    if ! "$tool" build -t ghcr.io/vrrb-io/vrrb -f infra/ci/Dockerfile .; then
        echo "Build failed with $tool"
        exit 1
    fi
}

find_container_tool() {
    for tool in docker nerdctl podman; do
        if command -v "$tool" &> /dev/null; then
            echo "$tool"
            return 0
        fi
    done
    echo "No container runtime found" >&2
    return 1
}

main() {
    if [[ -n $1 ]] && command -v "$1" &> /dev/null; then
        build_with_tool "$1"
    else
        echo "Attempting to auto-detect suitable container runtime..."
        local tool
        tool=$(find_container_tool)
        if [[ $? -eq 0 ]]; then
            build_with_tool "$tool"
        else
            exit 1
        fi
    fi
}

main "$@"
