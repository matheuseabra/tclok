#!/usr/bin/env sh
set -eu

release_version=${1:?expected release version}
manifest_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)

test "$release_version" = "$manifest_version"
cargo fmt --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
cargo package --locked --allow-dirty --no-verify
