# TTY Clock: zero-dependency Rust implementation plan

## Goal

Build a polished terminal clock in Rust with no crate dependencies. It must redraw cleanly when its terminal pane changes size, remain responsive while idle, and present the time as a centered, attractive digital display.

## Product decisions

- Ship a single binary crate named `tclok` using only the Rust standard library plus direct Unix/macOS FFI declarations where terminal APIs are necessary.
- Treat a resize as a normal render event. The next frame must use the current terminal dimensions and must not leave stale glyphs behind.
- Use a large seven-segment-style clock as the primary display, with a small date and contextual footer when space permits.
- Degrade deliberately for constrained panes: compact digital time first, then a minimal centered time. Never emit malformed ANSI sequences or crop a large clock.
- Target Unix-like terminals initially (macOS and Linux). Document Windows as out of scope for the first release rather than silently claiming portability.

## Proposed crate structure

```text
tclok/
├── Cargo.toml
├── src/
│   ├── main.rs          # CLI parsing, startup/shutdown, event loop
│   ├── terminal.rs      # RAII terminal session and Unix FFI wrappers
│   ├── layout.rs        # chooses render mode and calculates coordinates
│   ├── clock.rs         # time/date formatting
│   └── render.rs        # ANSI output construction and frame drawing
├── tests/
│   ├── layout_test.rs   # mode and centering boundary cases
│   └── render_test.rs   # snapshot-like string assertions for frames
└── docs/
    └── tty-clock-rust-plan.md
```

Keep terminal control, layout decisions, and rendering separate. This lets layout and rendering logic be exercised without an interactive TTY, while the unsafe platform boundary remains small and auditable.

## Implementation phases

### 1. Bootstrap the crate and command contract

Create a binary crate with `edition = "2024"` and no `[dependencies]`. Parse a deliberately small set of arguments with `std::env::args`:

- `--24h` / `--12h` to choose time format (default: locale-neutral 24-hour).
- `--seconds` / `--no-seconds` to control seconds (default: show seconds).
- `--help` and `--version`.

Reject unknown or conflicting flags with a concise usage message and a non-zero exit status. If stdout is not a terminal, print one formatted time value and exit, preventing raw escape sequences in pipelines.

### 2. Build the terminal session layer

In `terminal.rs`, create `TerminalSession` that owns all terminal mutations and restores state in `Drop`:

- Verify whether stdout is a TTY using `isatty`.
- Query rows and columns using `ioctl(TIOCGWINSZ)` and a local `winsize` `#[repr(C)]` definition.
- Enter the alternate screen, hide the cursor, reset style, and enable line wrapping safety as appropriate.
- On drop or explicit shutdown, reset attributes, clear no more than needed, show the cursor, and leave the alternate screen.

Use `extern "C"` declarations gated by `#[cfg(unix)]`; define only the constants and C-compatible structures required by the implementation. Keep every `unsafe` call in this module and explain its preconditions beside the call.

Install a lightweight `SIGWINCH` handler through `signal(2)` that only sets an `AtomicBool`. The render loop consumes and clears that flag, then obtains fresh dimensions. Avoid allocating, locking, formatting, or writing from the signal handler.

Also handle `SIGINT` and `SIGTERM` by setting a separate atomic shutdown flag. This ensures a normal loop exit and `Drop`-based terminal restoration. Install signal handlers transactionally and restore the prior dispositions during shutdown.

### 3. Model time and display text

In `clock.rs`, obtain wall-clock values via `std::time::SystemTime` and convert local calendar data using minimal Unix FFI (`localtime_r`, `strftime`) or a documented UTC fallback if local conversion cannot be supported without a dependency. Isolate this in a `ClockSnapshot` containing:

- `time_text`, such as `14:05:09` or `2:05:09 PM`.
- `date_text`, such as `Wednesday, 27 August 2026`.
- A one-second update boundary calculated from `SystemTime`.

For the large display on macOS Ghostty, rasterize the user-installed `FiraCode-Bold` face through CoreGraphics and place it using the Kitty graphics protocol. This renders the real locally installed typeface without bundling it or changing the user's terminal configuration. Other terminals use ordinary text; do not substitute block glyph art.

Read terminal pixel dimensions from `TIOCGWINSZ`; if a nested pane reports zeros, issue the standard `CSI 14 t` window-pixel query and parse Ghostty's `CSI 4;height;width t` reply before selecting the image rectangle.

Reserve the final row below the seven-row image for a centered `DD/MM/YYYY` date when at least ten rows are available. Below the full-time width threshold, render `HH:MM` in the same image rectangle before falling back to ordinary compact text.

Do not impose an application palette. The clock inherits the terminal's foreground color and remains legible in the user's ANSI theme.

### 4. Create responsive layout selection

In `layout.rs`, expose a pure function:

```rust
fn select_layout(size: TerminalSize, clock: &ClockSnapshot, options: &Options) -> Layout
```

It returns a mode plus exact origins for every block. Use these modes:

| Mode | Use when | Rendered content |
| --- | --- | --- |
| `Hero` | Double-height time fits and 4+ rows | Big clock with date |
| `Standard` | Double-height time fits and 2+ rows | Big clock only; optimized for wide, short panes |
| `Compact` | At least 5 columns | Centered one-line digital time, optional date if another row fits |
| `Minimal` | At least 1 cell | Truncated/centered plain time within available width |

Calculate dimensions from rendered cell widths, clamp every origin to non-negative coordinates, and omit optional content before reducing the core time display. When a full large `HH:MM:SS` face no longer fits horizontally, drop seconds and retain the large `HH:MM` treatment before selecting `Compact`. For terminal dimensions of zero or failed queries, skip drawing and wait for a valid resize.

### 5. Render full frames without resize artifacts

In `render.rs`, write a `Renderer` that takes a `Layout` and builds a single buffered frame:

1. Move the cursor home and erase the full display (`CSI 2J`, `CSI H`).
2. Paint the current frame at its calculated positions.
3. Reset styles and flush stdout once.

Full-frame clearing is intentionally preferred for the initial version: it guarantees that shrinking and expanding panes cannot retain old clock pixels. The clock is small enough that redraw cost is negligible at one frame per second. Later optimization may add line diffing only after profiling shows a real need.

Center both horizontally and vertically. For `Hero`, use a quiet single-line frame treatment only when it has enough surrounding space; do not draw borders that compete with the clock in narrow panes. Use ANSI cursor positioning with one-based row/column coordinates and cap all writes to the known terminal bounds.

### 6. Run an interruptible, deadline-based loop

In `main.rs`, drive a loop that renders immediately, then waits in short bounded intervals until either:

- the next one-second boundary is reached;
- `SIGWINCH` marks a resize pending; or
- shutdown is requested.

Do not use a blind `sleep(Duration::from_secs(1))`, which drifts relative to wall-clock second changes and delays resize handling. Use `nanosleep` to block until the next second boundary; a handled `SIGWINCH` interrupts that wait and triggers an immediate redraw. Re-query terminal size before every render, including scheduled time renders, because terminal emulators may not reliably deliver every resize signal.

Quit cleanly on Ctrl-C. Avoid a raw-input layer in the first release: this keeps the zero-dependency FFI boundary narrow and avoids mutating stdin state solely for an optional quit key.

### 7. Test and manually verify

Add focused unit/integration tests that require no terminal:

- Glyph composition produces equal-width rows for all supported characters.
- Layout selection chooses each mode at its documented boundaries.
- Every calculated coordinate is inside the available terminal dimensions.
- Compact and minimal layouts preserve a readable time for narrow widths.
- Frames inherit the terminal foreground without emitting palette-setting SGR color codes.
- Frame output begins by clearing/home positioning, preventing stale content after a resize.

Manually validate in Terminal, iTerm2, and a tmux or Zellij split pane:

1. Start at a normal size and confirm a centered, stable hero clock.
2. Drag the pane repeatedly across every layout threshold; verify no ghost glyphs, wrapping, flicker beyond the expected once-per-second update, or panic.
3. Expand after shrinking; confirm hero layout returns and re-centers.
4. Test the inherited terminal theme, 12/24-hour formatting, seconds on/off, Ctrl-C, and an induced panic/early error; verify the cursor and alternate screen are restored.
5. Pipe output (`tclok | cat`) and verify it emits one plain timestamp with no ANSI control bytes.

## Definition of done

- `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` pass with an empty dependency tree beyond the standard Rust toolchain.
- The binary works in an interactive Unix terminal and restores terminal state for normal exits, interrupts, and panics.
- The clock stays centered and switches layouts correctly through interactive pane resizes.
- A small pane still shows a coherent time rather than wrapped or clipped art.
- The README documents installation, flags, platform scope, and the no-dependency rationale.

## Deferred work

- Windows Console API support.
- User-configurable themes, fonts, and 12/24-hour auto-detection from locale.
- Terminal capability detection beyond TTY/color heuristics.
- Render diffing, animations, and alternative display treatments.
