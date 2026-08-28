use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use tclok::clock::{ClockSnapshot, next_second_delay};
use tclok::layout::select_layout;
use tclok::neue_machina;
use tclok::render::render_frame;
use tclok::terminal;
use tclok::{HourFormat, Options};

const USAGE: &str = "Usage: tclok [--12h|--24h] [--seconds|--no-seconds]\n\nA large, resize-responsive clock for modern UTF-8 xterm-compatible terminals.\nUses your terminal's foreground color. Use Ctrl-C to exit.";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tclok: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let options = parse_options(env::args().skip(1))?;
    if !terminal::stdout_is_terminal() {
        println!("{}", ClockSnapshot::now(options).compact_text);
        return Ok(());
    }

    if !matches!(env::var("TERM"), Ok(term) if !term.is_empty() && term != "dumb") {
        return Err("interactive mode requires an xterm-compatible TERM value".to_owned());
    }

    let _session = terminal::TerminalSession::enter().map_err(|error| error.to_string())?;
    while !terminal::stop_requested() {
        if let Some(size) = terminal::terminal_size() {
            let clock = ClockSnapshot::now(options);
            let frame = if env::var("TERM_PROGRAM")
                .is_ok_and(|program| program.eq_ignore_ascii_case("ghostty"))
            {
                neue_machina::render(size, terminal::terminal_pixel_size(), &clock)
                    .unwrap_or_else(|| render_frame(&select_layout(size, &clock)))
            } else {
                render_frame(&select_layout(size, &clock))
            };
            let mut stdout = io::stdout().lock();
            stdout
                .write_all(frame.as_bytes())
                .map_err(|error| error.to_string())?;
            stdout.flush().map_err(|error| error.to_string())?;
        }
        let delay = if terminal::take_resize_notification() {
            std::time::Duration::ZERO
        } else {
            next_second_delay()
        };
        terminal::sleep_until_signal_or_timeout(delay);
    }
    Ok(())
}

fn parse_options(arguments: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options::default();
    for argument in arguments {
        match argument.as_str() {
            "--12h" => options.hour_format = HourFormat::H12,
            "--24h" => options.hour_format = HourFormat::H24,
            "--seconds" => options.show_seconds = true,
            "--no-seconds" => options.show_seconds = false,
            "--help" | "-h" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("tclok {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown option: {unknown}\n\n{USAGE}")),
        }
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_parse_without_dependencies() {
        let options = parse_options(["--12h", "--no-seconds"].map(str::to_owned).into_iter())
            .expect("valid options");
        assert_eq!(options.hour_format, HourFormat::H12);
        assert!(!options.show_seconds);
    }

    #[test]
    fn removed_color_override_is_an_error() {
        assert!(parse_options(["--color=always"].map(str::to_owned).into_iter()).is_err());
    }
}
