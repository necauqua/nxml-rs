#!/usr/bin/env bash
set -eux

# not a check, technically, but eh whatever
rustfmt +nightly **/src/**.rs

cargo check --quiet --workspace --all-targets
cargo clippy --quiet --workspace --all-targets --all-features -- -D warnings -W clippy::all
RUSTDOCFLAGS='-D warnings' cargo doc --quiet --workspace --all-features

cargo test --quiet --workspace --all-targets --all-features
cargo test --quiet --workspace --doc

