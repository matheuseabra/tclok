# tclok

[![CI](https://github.com/matheuseabra/tclok/actions/workflows/ci.yml/badge.svg)](https://github.com/matheuseabra/tclok/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/tclok.svg)](https://crates.io/crates/tclok)
[![License](https://img.shields.io/crates/l/tclok.svg)](LICENSE)

A dependency-free, resize-responsive terminal clock for modern panes.

```sh
cargo install tclok
tclok
```

## Install

```sh
cargo install tclok
# After the Homebrew tap is published:
brew install matheuseabra/tclok/tclok
```

## Usage

```text
tclok [--12h|--24h] [--seconds|--no-seconds]
```

`tclok` redraws on resize, uses the alternate screen, and prints one plain timestamp when stdout is redirected.

On macOS Ghostty with an installed `FiraCode-Bold` font, the large clock is rendered through Kitty graphics. At 10+ rows it includes a `DD/MM/YYYY` date; narrower panes remove seconds before using the text fallback.

## Support

64-bit macOS and glibc Linux, UTF-8, xterm-compatible terminals. `TERM=dumb`, Windows, and legacy terminals are out of scope.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo package --allow-dirty
```

MIT licensed. See [release notes and packaging](docs/releasing.md).
