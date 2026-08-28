# Contributing

Run the complete local check before opening a pull request:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
cargo package --locked --allow-dirty --no-verify
```

Keep the dependency list empty unless a proposed dependency is explicitly justified by a portability or safety requirement.
