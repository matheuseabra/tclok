use crate::Rgb;
use crate::layout::{Layout, PositionedLine};
use std::fmt::Write as _;

pub fn render_frame(layout: &Layout, color: Option<Rgb>) -> String {
    let mut frame = String::from("\x1b[2J\x1b[H");
    if let Some(color) = color {
        let _ = write!(
            frame,
            "\x1b[38;2;{};{};{}m",
            color.red, color.green, color.blue
        );
    }
    for line in &layout.lines {
        render_line(&mut frame, line);
    }
    if color.is_some() {
        frame.push_str("\x1b[0m");
    }
    frame
}
fn render_line(frame: &mut String, line: &PositionedLine) {
    let _ = write!(frame, "\x1b[{};{}H", line.row, line.column);
    frame.push_str(&line.text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutMode;
    #[test]
    fn frame_clears_without_overriding_terminal_color() {
        let frame = render_frame(
            &Layout {
                mode: LayoutMode::Compact,
                lines: vec![PositionedLine {
                    row: 1,
                    column: 1,
                    text: "12".into(),
                }],
            },
            None,
        );
        assert!(frame.starts_with("\x1b[2J\x1b[H"));
        assert!(!frame.contains("38;5"));
        assert!(!frame.contains("\x1b#"));
    }

    #[test]
    fn explicit_color_wraps_the_frame_in_truecolor_sgr() {
        let frame = render_frame(
            &Layout {
                mode: LayoutMode::Compact,
                lines: vec![PositionedLine {
                    row: 1,
                    column: 1,
                    text: "12".into(),
                }],
            },
            Some(Rgb::new(18, 52, 86)),
        );
        assert!(frame.contains("\x1b[38;2;18;52;86m"));
        assert!(frame.ends_with("\x1b[0m"));
    }
}
