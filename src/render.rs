use crate::layout::{Layout, PositionedLine};
use std::fmt::Write as _;

pub fn render_frame(layout: &Layout) -> String {
    let mut frame = String::from("\x1b[2J\x1b[H");
    for line in &layout.lines {
        render_line(&mut frame, line);
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
        let frame = render_frame(&Layout {
            mode: LayoutMode::Compact,
            lines: vec![PositionedLine {
                row: 1,
                column: 1,
                text: "12".into(),
            }],
        });
        assert!(frame.starts_with("\x1b[2J\x1b[H"));
        assert!(!frame.contains("38;5"));
        assert!(!frame.contains("\x1b#"));
    }
}
