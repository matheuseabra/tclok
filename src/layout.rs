use crate::clock::ClockSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub columns: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Hero,
    Standard,
    Compact,
    Minimal,
    Hidden,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionedLine {
    pub row: u16,
    pub column: u16,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub mode: LayoutMode,
    pub lines: Vec<PositionedLine>,
}

pub fn select_layout(size: TerminalSize, clock: &ClockSnapshot) -> Layout {
    if size.columns == 0 || size.rows == 0 {
        return Layout {
            mode: LayoutMode::Hidden,
            lines: Vec::new(),
        };
    }
    if let Some(time) = largest_fitting_time(size, &clock.face_text) {
        if size.rows >= 10 {
            let mut lines = Vec::new();
            let time_row = (size.rows / 2).max(1);
            push_centered(&mut lines, size, time_row, time);
            push_centered(
                &mut lines,
                size,
                time_row.saturating_add(2),
                &clock.date_text,
            );
            if let Some(period) = clock.meridiem {
                push_centered(&mut lines, size, 1, period);
            }
            return Layout {
                mode: LayoutMode::Hero,
                lines,
            };
        }
        if size.rows >= 7 {
            return Layout {
                mode: LayoutMode::Standard,
                lines: vec![PositionedLine {
                    row: (size.rows / 2).max(1),
                    column: centered_column(size.columns, cell_width(time)),
                    text: time.to_owned(),
                }],
            };
        }
    }
    if size.columns >= 5 {
        let mut lines = Vec::new();
        let time_row = (size.rows / 2).max(1);
        push_centered(&mut lines, size, time_row, &clock.compact_text);
        if size.rows >= 3 {
            push_centered(
                &mut lines,
                size,
                time_row.saturating_add(1),
                &clock.date_text,
            );
        }
        return Layout {
            mode: LayoutMode::Compact,
            lines,
        };
    }
    let mut lines = Vec::new();
    let text = truncate_cells(&clock.compact_text, usize::from(size.columns));
    push_centered(&mut lines, size, (size.rows / 2).max(1), &text);
    Layout {
        mode: LayoutMode::Minimal,
        lines,
    }
}

fn largest_fitting_time(size: TerminalSize, full_time: &str) -> Option<&str> {
    let mut candidates = std::iter::once(full_time).chain(
        full_time
            .rsplit_once(':')
            .map(|(head, _)| head)
            .filter(|head| head.contains(':')),
    );
    candidates.find(|time| usize::from(size.columns) >= cell_width(time) + 2)
}

fn push_centered(lines: &mut Vec<PositionedLine>, size: TerminalSize, row: u16, text: &str) {
    if row <= size.rows {
        let text = truncate_cells(text, usize::from(size.columns));
        lines.push(PositionedLine {
            row,
            column: centered_column(size.columns, cell_width(&text)),
            text,
        });
    }
}

fn centered_column(columns: u16, width: usize) -> u16 {
    ((usize::from(columns).saturating_sub(width.min(usize::from(columns))) / 2) + 1) as u16
}

pub fn cell_width(text: &str) -> usize {
    text.chars().count()
}
fn truncate_cells(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot() -> ClockSnapshot {
        ClockSnapshot {
            face_text: "12:34:56".into(),
            compact_text: "12:34:56".into(),
            date_text: "Wed, 27 Aug 2026".into(),
            meridiem: None,
        }
    }

    #[test]
    fn hiding_seconds_keeps_the_native_face_in_a_narrower_pane() {
        let layout = select_layout(
            TerminalSize {
                columns: 7,
                rows: 7,
            },
            &snapshot(),
        );
        assert_eq!(layout.mode, LayoutMode::Standard);
        assert_eq!(layout.lines[0].text, "12:34");
    }
    #[test]
    fn responsive_layout_selects_every_mode() {
        let clock = snapshot();
        assert_eq!(
            select_layout(
                TerminalSize {
                    columns: 90,
                    rows: 25
                },
                &clock
            )
            .mode,
            LayoutMode::Hero
        );
        assert_eq!(
            select_layout(
                TerminalSize {
                    columns: 50,
                    rows: 7
                },
                &clock
            )
            .mode,
            LayoutMode::Standard
        );
        assert_eq!(
            select_layout(
                TerminalSize {
                    columns: 12,
                    rows: 4
                },
                &clock
            )
            .mode,
            LayoutMode::Compact
        );
        assert_eq!(
            select_layout(
                TerminalSize {
                    columns: 4,
                    rows: 1
                },
                &clock
            )
            .mode,
            LayoutMode::Minimal
        );
    }
    #[test]
    fn every_line_fits_across_pane_sizes() {
        for columns in 1..100 {
            for rows in 1..30 {
                let size = TerminalSize { columns, rows };
                assert!(
                    select_layout(size, &snapshot())
                        .lines
                        .iter()
                        .all(|line| line.row >= 1
                            && line.row <= rows
                            && line.column >= 1
                            && line.column <= columns
                            && cell_width(&line.text) <= usize::from(columns))
                );
            }
        }
    }
}
