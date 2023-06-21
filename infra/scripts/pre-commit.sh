#!/bin/bash

set -e

# Formats the code for consistency
echo 'Formatting...'

 for rust_file in $(git diff --name-only --cached | grep ".*\.rs$"); do

    if test -e "$rust_file"; then
        rustup run nightly cargo fmt -- "$rust_file"
        git add "$rust_file"
    fi
 done
 echo "Done"


