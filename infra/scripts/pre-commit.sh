#!/bin/bash

set -e

# Formats the code for consistency
echo 'Formatting...'

 for rust_file in $(git diff --name-only --cached | grep ".*\.rs$"); do
     cargo +nightly fmt -- $rust_file
     git add $rust_file
 done
 echo "Done"


