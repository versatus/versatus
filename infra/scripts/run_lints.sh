#!/bin/bash

cargo +nightly fmt --all 
cargo +nightly clippy --all -- -D clippy::all

