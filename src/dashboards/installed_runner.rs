/// Installed dashboard runner — spawns third-party TUI apps in a PTY and
/// renders their output inside the TUIX main container.
///
/// Uses `portable-pty` to create a pseudo-terminal sized to the container area,
/// and `vt100` to parse the terminal output into cells we can render with ratatui.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use ratatui::{
    layout::{Alignment, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use vt100::Parser;

use super::super::tuix::models::Action;

/// Manages a running installed dashboard subprocess.
pub struct InstalledDashboard {
    pub name: String,
    label: String,
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    parser: Arc<Mutex<Parser>>,
    /// Last known size so we can detect resizes
    last_cols: u16,
    last_rows: u16,
    _reader_handle: thread::JoinHandle<()>,
}

impl InstalledDashboard {
    /// Spawn the dashboard command in a PTY sized to `cols` x `rows`.
    pub fn start(name: &str, label: &str, cmd: &[String], cols: u16, rows: u16) -> Option<Self> {
        if cmd.is_empty() {
            return None;
        }

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .ok()?;

        let mut command = CommandBuilder::new(&cmd[0]);
        if cmd.len() > 1 {
            command.args(&cmd[1..]);
        }
        // Set TERM so the child knows it's in a terminal
        command.env("TERM", "xterm-256color");
        // Set columns/rows env vars some apps check
        command.env("COLUMNS", cols.to_string());
        command.env("LINES", rows.to_string());

        let _child = pair.slave.spawn_command(command).ok()?;
        drop(pair.slave); // Close slave side in parent

        let parser = Arc::new(Mutex::new(Parser::new(rows, cols, 0)));
        let parser_clone = Arc::clone(&parser);

        let mut reader = pair.master.try_clone_reader().ok()?;
        let writer = pair.master.take_writer().ok()?;

        // Background thread reads PTY output and feeds it to the vt100 parser
        let reader_handle = thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut p) = parser_clone.lock() {
                            p.process(&buf[..n]);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Some(Self {
            name: name.to_string(),
            label: label.to_string(),
            writer,
            master: pair.master,
            parser,
            last_cols: cols,
            last_rows: rows,
            _reader_handle: reader_handle,
        })
    }

    /// Resize the PTY if the container area changed.
    pub fn resize_if_needed(&mut self, cols: u16, rows: u16) {
        if cols != self.last_cols || rows != self.last_rows {
            self.last_cols = cols;
            self.last_rows = rows;
            let _ = self.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
            if let Ok(mut p) = self.parser.lock() {
                p.set_size(rows, cols);
            }
        }
    }

    /// Render the PTY screen contents into the given ratatui area.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, border_style: Style) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", self.label))
            .title_alignment(Alignment::Right)
            .style(border_style);
        frame.render_widget(block, area);

        let inner = area.inner(Margin {
            horizontal: 1,
            vertical: 1,
        });

        // Resize PTY to match inner area
        self.resize_if_needed(inner.width, inner.height);

        // Read the virtual terminal screen
        let screen = if let Ok(p) = self.parser.lock() {
            p.screen().clone()
        } else {
            return;
        };

        // Convert vt100 screen rows to ratatui Lines, clipping to inner area
        let mut lines: Vec<Line> = Vec::with_capacity(inner.height as usize);

        for row in 0..inner.height {
            let mut spans: Vec<Span> = Vec::new();
            let mut col = 0u16;

            while col < inner.width {
                let cell = screen.cell(row, col);
                if let Some(cell) = cell {
                    let ch = cell.contents();
                    let ch_display = if ch.is_empty() { " " } else { &ch };

                    // Truncate: if the character would overflow, skip it
                    let char_width = unicode_display_width(ch_display) as u16;
                    if col + char_width > inner.width {
                        break;
                    }

                    let fg = convert_vt100_color(cell.fgcolor());
                    let bg = convert_vt100_color(cell.bgcolor());

                    let mut style = Style::default();
                    if fg != Color::Reset {
                        style = style.fg(fg);
                    }
                    if bg != Color::Reset {
                        style = style.bg(bg);
                    }
                    if cell.bold() {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if cell.italic() {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    if cell.underline() {
                        style = style.add_modifier(Modifier::UNDERLINED);
                    }
                    if cell.inverse() {
                        style = style.add_modifier(Modifier::REVERSED);
                    }

                    spans.push(Span::styled(ch_display.to_string(), style));
                    col += char_width.max(1);
                } else {
                    spans.push(Span::raw(" "));
                    col += 1;
                }
            }

            lines.push(Line::from(spans));
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }

    /// Forward a key action to the PTY.
    pub fn send_key(&mut self, action: Action) {
        let bytes: &[u8] = match action {
            Action::Up => b"\x1b[A",
            Action::Down => b"\x1b[B",
            Action::Right => b"\x1b[C",
            Action::Left => b"\x1b[D",
            Action::Enter => b"\r",
            Action::Back => b"q", // Send 'q' which sigye uses to quit
            Action::Tab => b"\t",
            Action::Quit => b"\x1b", // ESC
        };
        let _ = self.writer.write_all(bytes);
    }

    /// Send a raw character to the PTY (for passthrough keys).
    #[allow(dead_code)]
    pub fn send_raw(&mut self, ch: char) {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        let _ = self.writer.write_all(s.as_bytes());
    }
}

impl Drop for InstalledDashboard {
    fn drop(&mut self) {
        // Closing master will signal EOF to the child
        // The reader thread will exit when it gets EOF
    }
}

/// Convert a vt100 Color to a ratatui Color.
fn convert_vt100_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

/// Calculate display width of a string (approximate; counts chars).
fn unicode_display_width(s: &str) -> usize {
    s.chars().count()
}
