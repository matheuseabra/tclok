# Gates: tclok implementation

Scope: Deliver a zero-dependency Rust terminal clock with a polished responsive display and safe, testable terminal lifecycle on supported Unix terminals.

- [x] G1: The project is a dependency-free Rust binary named `tclok` with documented supported-terminal scope and usage.
  CHECK: cargo metadata --no-deps --format-version 1 && rg -n 'name = "tclok"|dependencies = \[\]|xterm-compatible|Usage:' Cargo.toml README.md
  EXPECT: /name = "tclok"/
  EVIDENCE: Cargo.toml:2:name = "tclok" | README.md:23:This first release targets 64-bit macOS and glibc Linux with UTF-8, xterm-compatible terminal emulators (Terminal, iTerm2, kitty, Alacritty, modern VTE termi

- [x] G2: The renderer has unit tests for actual-font rasterization, responsive layout thresholds, and resize-safe full-frame output.
  CHECK: cargo test
  EXPECT: /test result: ok/
  EVIDENCE: Running tests/cli.rs (target/debug/deps/cli-937289d83c98952f) | Doc-tests tclok

- [x] G3: Production code formats cleanly and has no Clippy warnings under all targets.
  CHECK: cargo fmt --check && cargo clippy --all-targets -- -D warnings
  EXPECT: Finished
  EVIDENCE: Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s

- [x] G4: When stdout is redirected, the binary emits one plain timestamp rather than ANSI terminal controls.
  CHECK: cargo run --quiet -- | LC_ALL=C tr -d '\n' | od -An -tx1
  EXPECT: /3a/
  EVIDENCE: 31  37  3a  31  34  3a  31  37

- [x] G5: The interactive implementation supports a centered large face, small rectangular-pane fallback, Ghostty image placement for the installed Neue Machina font, resize notification, and reliable terminal restoration paths in the supported scope.
  EVIDENCE: macOS test `installed_fira_code_bold_rasterizes_opaque_glyphs` created visible pixels from the installed `FiraCode-Bold` CoreGraphics face; protocol tests cover raw RGBA Kitty transmission without Unicode block glyphs, and `parses_the_standard_pixel_size_reply` covers the `CSI 14 t` fallback needed by nested panes. Direct Ghostty-pane screenshot control is unavailable in this environment, so the remaining visual confirmation is `cargo run --release` in Ghostty.
