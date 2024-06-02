#!/usr/bin/env bash
set -e

# not a check, technically, but eh whatever
rustfmt +nightly **/src/**.rs

cargo clippy --all-targets --all-features -- -D warnings
cargo doc --all-features
cargo test --all-features
