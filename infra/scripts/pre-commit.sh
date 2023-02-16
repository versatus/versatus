#!/bin/bash

set -e

# Formats the code for consistency
echo 'Formatting...'

 for rust_file in $(git diff --name-only --cached | grep ".*\.rs$"); do

    if test -f "$rust_file"; then
         cargo +nightly fmt -- $rust_file
    fi

     git add $rust_file
 done
 echo "Done"


