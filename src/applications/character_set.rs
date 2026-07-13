/// Character Set — internal TUIX app.
///
/// Displays a scrollable grid of printable characters with their
/// Unicode code points. No external code or binary required — the
/// character data is generated at runtime.
///
/// TUIX Internal App API:
///     start()               — called when the app is opened
///     stop()                — called when the app is closed
///     render(frame, area)   — draw the app inside the main container
///     handle_key(code)      — receive raw key codes; return action

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Character ranges to display
const RANGES: &[(&str, u32, u32)] = &[
    ("English Characters", 0x0020, 0x007E),
    ("Arrows",               0x2190, 0x21FF),
    ("Math Operators",       0x2200, 0x22FF),
    ("Geometric Shapes",     0x25A0, 0x25FF),
    ("Block Elements",       0x2580, 0x259F),
    ("Box Drawing",          0x2500, 0x257F),
];

pub struct CharacterSetApp {
    pub scroll_offset: usize,
    lines: Vec<String>,
}

impl CharacterSetApp {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            lines: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.scroll_offset = 0;
        self.lines = Self::build_lines();
    }

    pub fn stop(&mut self) {}

    fn build_lines() -> Vec<String> {
        let mut lines = Vec::new();

        for (name, start, end) in RANGES {
            lines.push(String::new());
            lines.push(format!("  ── {} (U+{:04X}–U+{:04X}) ──", name, start, end));
            lines.push(String::new());

            let mut row = String::from("  ");
            let mut count = 0;
            for cp in *start..=*end {
                if let Some(ch) = char::from_u32(cp) {
                    row.push_str(&format!(" {:04X}:{}", cp, ch));
                    count += 1;
                    if count % 8 == 0 {
                        lines.push(row);
                        row = String::from("  ");
                    }
                }
            }
            if count % 8 != 0 {
                lines.push(row);
            }
        }

        lines
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, border_style: Style) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Character Set — ↑↓ scroll, Q to close ")
            .title_alignment(Alignment::Right)
            .style(border_style);
        frame.render_widget(block, area);
        let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
        let visible_rows = inner.height as usize;

        if visible_rows == 0 || self.lines.is_empty() {
            return;
        }

        // Clamp scroll
        let max_offset = self.lines.len().saturating_sub(visible_rows);
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }

        let end = (self.scroll_offset + visible_rows).min(self.lines.len());
        let visible = &self.lines[self.scroll_offset..end];

        let constraints: Vec<Constraint> = visible
            .iter()
            .map(|_| Constraint::Length(1))
            .collect();

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (i, line) in visible.iter().enumerate() {
            let style = if line.starts_with("  ──") {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            frame.render_widget(
                Paragraph::new(Text::raw(line.clone())).style(style),
                rows[i],
            );
        }
    }

    pub fn handle_key(&mut self, code: &str) -> Option<AppAction> {
        match code {
            "q" | "Q" | "Backspace" | "Esc" | "Escape" => Some(AppAction::Back),
            "Up" | "w" => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
                None
            }
            "Down" | "s" => {
                self.scroll_offset += 1;
                None
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppAction {
    Back,
}
