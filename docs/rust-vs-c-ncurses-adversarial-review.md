# Can Rust beat C/ncurses for `tclok`? An adversarial review

## Verdict

**Yes, but not across every dimension, and not with the plan as currently written.** A Rust implementation can credibly beat the original `tty-clock` in memory safety outside the FFI boundary, separation of concerns, automated testing, resize/layout behavior, and narrow-pane usability. It can match human-perceived rendering speed because a one-frame-per-second clock is not computationally demanding.

It is unlikely to beat the dynamically linked C/ncurses program in executable size or terminal coverage. The current proposal also loses the idle-efficiency contest: polling every 25–50 ms means 20–40 wake checks per second, whereas the clock only needs a one-second deadline and event-driven input/resize wakeups. Unconditional `CSI 2J` full-screen redraws may send more output and flicker more than ncurses' virtual-to-physical screen update machinery.

The defensible goal is therefore: **beat the original as a product on supported xterm-compatible terminals, while matching its steady-state resource use within measured limits**. “Rust beats C” should remain a benchmark result, not a premise.

## What the baseline actually provides

The original is not merely C drawing characters. ncurses owns terminal initialization, cbreak input, key decoding, color setup, cursor visibility, windows, movement, resizing, and terminal-specific output. The source calls `initscr`/`newterm`, `cbreak`, `keypad`, `start_color`, `curs_set`, and the window APIs directly ([upstream initialization](https://github.com/xorg62/tty-clock/blob/f2f847cf2cc2949c8a8b7779a778f366d3743474/ttyclock.c#L36-L76)). Its event path uses nonblocking `wgetch`, receives ncurses' `KEY_RESIZE`, and waits with `pselect` or `nanosleep` ([upstream input loop](https://github.com/xorg62/tty-clock/blob/f2f847cf2cc2949c8a8b7779a778f366d3743474/ttyclock.c#L423-L461)).

ncurses also selects behavior from terminal descriptions and several possible size sources; its documentation notes that size may come from the operating system, environment, or terminfo ([ncurses manual](https://man7.org/linux/man-pages/man3/ncurses.3x.html)). A raw ANSI implementation replaces that library dependency with application-owned policy and platform code. That can be worthwhile, but it is a transfer of complexity, not its removal.

## Scorecard before implementation

| Dimension | Likely winner | Adversarial assessment |
| --- | --- | --- |
| Clock calculation and layout speed | Tie | Both are far below any meaningful CPU limit at 1 Hz. Language choice is immaterial here. |
| Idle wakeups | C plan baseline | Rust's proposed 25–50 ms polling performs 20–40 checks/s. An event-driven Rust loop can recover a tie or win. |
| Output efficiency | ncurses | ncurses can update the physical screen from its virtual model; clearing and repainting every cell is deliberately coarser. |
| Resize and small-pane UX | Rust | Pure layout modes and immediate reflow are a real product advantage over reinitializing the original UI on `KEY_RESIZE`. |
| Memory safety | Rust, conditionally | Layout/rendering can be safe Rust. Hand-declared `termios`, `ioctl`, signal, and time ABIs remain unsafe and platform-sensitive. |
| Terminal compatibility | ncurses | terminfo adapts to terminal capabilities; hard-coded CSI and xterm alternate-screen sequences assume a narrower terminal family. |
| Binary size | Probably C | A small dynamically linked C executable normally excludes ncurses from its file size. Rust size must be measured after equivalent stripping/optimization. |
| Startup latency | Unknown, likely indistinguishable to users | Neither implementation does substantial work, so only controlled measurement can support a winner claim. |
| Maintainability | Split decision | Rust modules and tests win; bespoke libc ABI declarations and terminal capability policy lose. |
| Feature breadth | Original C | The original already exposes UTC, date formatting, target TTY, rebound/screensaver, box, live key controls, and timing options ([upstream CLI](https://github.com/xorg62/tty-clock#usage)). `tclok` may intentionally choose a smaller product, but should not call that feature parity. |

## Findings that challenge the current plan

### 1. “Zero dependency” is underspecified

No Cargo dependencies does not mean no dependencies: the program still depends on a Unix libc ABI, an xterm-like control-sequence vocabulary, Unicode width behavior, and terminal-emulator conventions. Direct declarations of `winsize`, `termios`, `time_t`, `tm`, variadic `ioctl`, and signal functions must be correct for each supported target.

This makes “macOS and Linux” too broad unless the plan names architectures and CI targets. A wrong C structure or constant is not a graceful compatibility failure; it can be undefined behavior. The safe Rust core does not neutralize an incorrect FFI boundary.

**Change:** choose explicitly between:

- strict zero-crate support for named targets such as `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, and `aarch64-apple-darwin`, backed by target-specific modules and CI; or
- one small, audited `libc` build dependency to remove duplicated ABI declarations while still shipping one self-contained executable with no Rust runtime package requirement.

### 2. The proposed wait loop wastes the clearest performance opportunity

A 25–50 ms sleep slice wakes 20–40 times per second even when nothing happens. That is unnecessary for a clock and can matter on laptops more than the few microseconds spent rendering.

**Change:** block until the earliest monotonic deadline or file-descriptor event using `poll`, `ppoll`, or `pselect`. Use `sigaction`, handle `EINTR`, and re-query time and dimensions after every wake. If immediate, race-free signal notification is required, use a self-pipe whose handler performs only an async-signal-safe `write`. POSIX restricts handlers to async-signal-safe operations ([POSIX signal rules](https://pubs.opengroup.org/onlinepubs/9799919799/functions/V2_chap02.html); [Linux signal-safety](https://man7.org/linux/man-pages/man7/signal-safety.7.html)). A simple atomic flag is acceptable only when it communicates no accompanying non-atomic state; document the ordering and lost-event semantics.

### 3. ANSI is a compatibility budget, not a universal terminal API

`CSI H`, `CSI 2J`, SGR, cursor hiding, wrapping control, and alternate-screen entry are common on modern xterm-compatible terminals, but ncurses uses terminfo precisely because terminals vary. Even the `clear` utility obtains its sequence from the terminal database ([ncurses `clear(1)`](https://manpages.debian.org/trixie/ncurses-bin/clear.1.en.html)). `isatty` only establishes that a descriptor is associated with a terminal, not that it accepts the chosen sequences or that a human controls it ([POSIX `isatty`](https://pubs.opengroup.org/onlinepubs/009695299/functions/isatty.html)).

**Change:** state a minimum compatibility contract: UTF-8 plus ECMA-48/xterm-style cursor addressing, SGR, erase, and optionally alternate screen. Treat empty/unknown `TERM` and `TERM=dumb` as plain-output or explicit-error cases. Honor `NO_COLOR`; make alternate-screen use disableable. Define interactive mode from both streams: render capability belongs to stdout, while keyboard capability belongs to stdin.

### 4. Full-screen clearing maximizes simplicity, not performance

The original calls `wrefresh` on windows and lets ncurses manage screen state ([drawing code](https://github.com/xorg62/tty-clock/blob/f2f847cf2cc2949c8a8b7779a778f366d3743474/ttyclock.c#L217-L295)). `tclok`'s full clear guarantees stale content is removed, but over SSH, serial links, multiplexers, or slow emulators it may produce avoidable bytes and visible flashes.

**Change:** keep full repaint as the correctness-first implementation, but do not describe its cost as negligible without evidence. Retain the previous logical frame and either diff changed rows or clear only the union of old and new occupied regions. Force one full repaint after resize or detected desynchronization. Benchmark both strategies by output bytes, render time, and visual behavior.

### 5. Terminal restoration needs transactional setup

The plan says `Drop` restores terminal state, but setup can fail after the first mutation and before the fully constructed guard exists. It also proposes nonblocking stdin without requiring restoration of the original `fcntl` flags. POSIX recommends saving the exact `tcgetattr` result and deriving changes from it ([`tcgetattr`](https://man7.org/linux/man-pages/man3/tcgetattr.3p.html); [`tcsetattr`](https://www.man7.org/linux/man-pages/man3/tcsetattr.3p.html)).

Raw mode also changes signal behavior. If `ISIG` is cleared, Ctrl-C becomes byte `0x03` rather than kernel-generated `SIGINT`; `cfmakeraw` is not a portable POSIX abstraction ([Linux termios](https://www.man7.org/linux/man-pages/man3/termios.3.html)).

**Change:** specify cbreak-like mode, preserving `ISIG`, unless byte-level Ctrl-C handling is intentional. Snapshot termios and descriptor flags before mutation, arm an idempotent rollback guard immediately, and restore only what was successfully changed. Save and restore prior signal dispositions. Cleanup should be best-effort and must not panic. A panic hook may write reset sequences during ordinary panic unwinding, but signal handlers must never invoke formatting, allocation, terminal cleanup, or `Drop`.

### 6. Unicode-safe rendering is overclaimed

Rust byte length is not terminal cell width, and the standard library has no general terminal-width algorithm. Even a formally assigned Unicode width does not guarantee identical font rendering across terminals. A fixed table of block characters can be tested on supported terminals, but it is not generically “Unicode-safe.” Locale and malformed/non-UTF-8 environments also need a policy.

**Change:** either use ASCII glyphs/control sequences whose cell widths are known under the compatibility contract, or declare the selected block characters to be one-cell requirements and provide an ASCII fallback. Store glyph widths as validated metadata rather than calculating them from UTF-8 length.

### 7. Local time FFI is another portability surface

The original uses global-state `localtime` and fixed buffers, so `localtime_r` plus bounded output is an improvement ([upstream time formatting](https://github.com/xorg62/tty-clock/blob/f2f847cf2cc2949c8a8b7779a778f366d3743474/ttyclock.c#L171-L214)). It still requires correct platform definitions for `time_t` and `struct tm`, handling `strftime` returning zero, timezone changes, and wall-clock jumps.

**Change:** separate monotonic scheduling from wall-clock display. Recompute the next wall-clock boundary after every render instead of trusting a previous deadline across clock adjustments. Add UTC as a reliable fallback and test DST transitions, invalid/out-of-range time conversion, and a timezone change while running.

### 8. The test plan stops short of the unsafe behavior

Pure layout and string tests are valuable but cannot prove termios restoration, signal behavior, resize races, output clipping, or compatibility. The riskiest module currently gets only manual review.

**Change:** add PTY integration tests that launch the binary, resize the pseudo-terminal, send input/signals, inspect emitted bytes, and verify termios/descriptor restoration after normal exit, initialization failure, and panic. Run rapid resize storms, redirected stdin with TTY stdout and the reverse, `TERM=dumb`, missing `TERM`, tmux, and Linux/macOS CI. Unit-test every FFI return-code and `EINTR` branch through a narrow system-call abstraction.

## Required benchmark before claiming a win

Build release artifacts for both programs on the same machine and report commands, compiler versions, linking mode, and whether binaries are stripped. Compare equivalent behavior—centered clock, seconds, color, same terminal—rather than default versus default.

Record at least:

1. stripped executable bytes and dynamic-library dependencies;
2. median and tail startup-to-first-frame latency over many runs, separating warm and cold-cache claims;
3. steady-state CPU time, resident memory, wakeups/context switches, and system calls over five minutes;
4. bytes written for 60 stable frames and during a scripted resize storm;
5. resize-to-correct-frame latency; and
6. a terminal matrix covering macOS Terminal, iTerm2, common Linux VTE/Kitty/Alacritty terminals, tmux, SSH, `TERM=dumb`, redirected streams, and panes from 1×1 upward.

Cargo provides release-profile controls for LTO, code generation units, optimization level, stripping, and panic strategy ([Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)). Test a size-oriented profile such as LTO, one codegen unit, stripping, and `panic = "abort"`, but disclose the correctness tradeoff: aborting bypasses unwinding and `Drop`, so terminal restoration then depends on the panic hook and cannot be guaranteed for all fatal exits.

No implementation can restore a terminal after `SIGKILL`, power loss, or a killed emulator. The documentation should promise restoration only for handled exits and explicitly give users a recovery command such as `reset`.

## Recommended edits to the implementation plan

1. Replace the 25–50 ms sleep loop with an event-driven monotonic-deadline design.
2. Add a supported-target and terminal-capability matrix; describe raw ANSI as a deliberate xterm-compatible scope reduction from ncurses.
3. Decide `libc` crate versus hand-maintained per-target ABI declarations, and revise “zero dependency” to say exactly which kind of dependency is being avoided.
4. Define cbreak/`ISIG`, stdin-versus-stdout TTY behavior, prior signal-handler restoration, `EINTR`, and transactional cleanup.
5. Make full repaint the fallback, then measure row diffing or bounded-region clearing.
6. Replace generic Unicode-width claims with a tested one-cell glyph contract plus ASCII fallback.
7. Separate monotonic scheduling from wall-clock/timezone conversion.
8. Add PTY integration, resize-storm, cleanup, stream-redirection, and per-target FFI tests.
9. Add the benchmark protocol above to the definition of done; prohibit unmeasured speed, startup, or size claims.
10. Reframe the product goal: win on reliable responsive UX and maintainable safe core, seek parity on runtime cost, and explicitly accept narrower terminal compatibility unless terminfo support is added.

## Final assessment

Rust can win where the original is weakest: global mutable state, fixed buffers, limited layout behavior, and difficult unit testing. The original source demonstrates those opportunities, including non-reentrant time conversion and tightly coupled draw/update logic ([time update](https://github.com/xorg62/tty-clock/blob/f2f847cf2cc2949c8a8b7779a778f366d3743474/ttyclock.c#L171-L214)).

Rust does not automatically win merely by removing ncurses. ncurses is doing useful compatibility and rendering work. A disciplined, event-driven, target-explicit Rust program can be the better clock on modern supported terminals; a polling, always-clear, hand-rolled-ABI implementation cannot honestly claim a general performance, size, or compatibility victory over C/ncurses.
