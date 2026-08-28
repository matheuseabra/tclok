use crate::layout::{Layout, PositionedLine};
use crate::{Gradient, Rgb};
use std::fmt::Write as _;

pub fn render_frame(layout: &Layout, color: Option<Rgb>, gradient: Option<Gradient>) -> String {
    let mut frame = String::from("\x1b[2J\x1b[H");
    if let Some(color) = color.filter(|_| gradient.is_none()) {
        let _ = write!(
            frame,
            "\x1b[38;2;{};{};{}m",
            color.red, color.green, color.blue
        );
    }
    let top = layout.lines.iter().map(|line| line.row).min().unwrap_or(1);
    let bottom = layout
        .lines
        .iter()
        .map(|line| line.row)
        .max()
        .unwrap_or(top);
    for line in &layout.lines {
        let line_color = gradient.map(|gradient| {
            let position = f64::from(line.row.saturating_sub(top))
                / f64::from(bottom.saturating_sub(top).max(1));
            gradient.color_at(position)
        });
        if let Some(color) = line_color {
            let _ = write!(
                frame,
                "\x1b[38;2;{};{};{}m",
                color.red, color.green, color.blue
            );
        }
        render_line(&mut frame, line);
    }
    if color.is_some() || gradient.is_some() {
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
            None,
        );
        assert!(frame.contains("\x1b[38;2;18;52;86m"));
        assert!(frame.ends_with("\x1b[0m"));
    }

    #[test]
    fn gradient_colors_each_rendered_line() {
        let frame = render_frame(
            &Layout {
                mode: LayoutMode::Hero,
                lines: vec![
                    PositionedLine {
                        row: 1,
                        column: 1,
                        text: "12".into(),
                    },
                    PositionedLine {
                        row: 3,
                        column: 1,
                        text: "date".into(),
                    },
                ],
            },
            None,
            Some(Gradient {
                top: Rgb::new(255, 0, 0),
                bottom: Rgb::new(0, 0, 255),
            }),
        );
        assert!(frame.contains("\x1b[38;2;255;0;0m"));
        assert!(frame.contains("\x1b[38;2;0;0;255m"));
    }
}
