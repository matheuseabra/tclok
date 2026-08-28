use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use tclok::clock::{ClockSnapshot, next_second_delay};
use tclok::layout::select_layout;
use tclok::neue_machina;
use tclok::render::render_frame;
use tclok::terminal;
use tclok::{HourFormat, Options, Rgb};

const USAGE: &str = "Usage: tclok [--12h|--24h] [--seconds|--no-seconds] [--color <#RRGGBB>]\n\nA large, resize-responsive clock for modern UTF-8 xterm-compatible terminals.\nUses your terminal's foreground color unless --color is provided. Use Ctrl-C to exit.";

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
    let foreground = options.color.or_else(terminal::terminal_foreground_color);
    while !terminal::stop_requested() {
        if let Some(size) = terminal::terminal_size() {
            let clock = ClockSnapshot::now(options);
            let frame = if env::var("TERM_PROGRAM")
                .is_ok_and(|program| program.eq_ignore_ascii_case("ghostty"))
            {
                neue_machina::render(size, terminal::terminal_pixel_size(), foreground, &clock)
                    .unwrap_or_else(|| render_frame(&select_layout(size, &clock), options.color))
            } else {
                render_frame(&select_layout(size, &clock), options.color)
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
    let mut arguments = arguments;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--12h" => options.hour_format = HourFormat::H12,
            "--24h" => options.hour_format = HourFormat::H24,
            "--seconds" => options.show_seconds = true,
            "--no-seconds" => options.show_seconds = false,
            "--color" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--color expects a hex value such as #7aa2f7".to_owned())?;
                options.color = Some(parse_color(&value)?);
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("tclok {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            value if value.starts_with("--color=") => {
                options.color = Some(parse_color(&value[8..])?);
            }
            unknown => return Err(format!("unknown option: {unknown}\n\n{USAGE}")),
        }
    }
    Ok(options)
}

fn parse_color(value: &str) -> Result<Rgb, String> {
    Rgb::from_hex(value)
        .ok_or_else(|| format!("invalid color `{value}`; use #RRGGBB or #RGB\n\n{USAGE}"))
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
        assert_eq!(options.color, None);
    }

    #[test]
    fn parses_hex_color_in_both_forms() {
        let equals = parse_options(["--color=#7aa2f7"].map(str::to_owned).into_iter())
            .expect("valid hex color");
        let separate = parse_options(["--color", "#abc"].map(str::to_owned).into_iter())
            .expect("valid short hex color");
        assert_eq!(equals.color, Some(Rgb::new(0x7a, 0xa2, 0xf7)));
        assert_eq!(separate.color, Some(Rgb::new(0xaa, 0xbb, 0xcc)));
    }

    #[test]
    fn rejects_invalid_color() {
        assert!(parse_options(["--color=always"].map(str::to_owned).into_iter()).is_err());
    }
}
