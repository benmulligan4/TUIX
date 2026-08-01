/// Boot animation — displays a TUIX splash screen with progress bar on startup.

use std::io;
use std::thread;
use std::time::Duration;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame, Terminal,
};

use crate::settings::persistence;

const TUIX_LOGO: &[&str] = &[
    "████████╗██╗   ██╗██╗██╗  ██╗",
    "╚══██╔══╝██║   ██║██║╚██╗██╔╝",
    "   ██║   ██║   ██║██║ ╚███╔╝ ",
    "   ██║   ██║   ██║██║ ██╔██╗ ",
    "   ██║   ╚██████╔╝██║██╔╝ ██╗",
    "   ╚═╝    ╚═════╝ ╚═╝╚═╝  ╚═╝",
];

const STEPS: &[&str] = &[
    "Loading kernel...",
    "Starting services...",
    "Loading applications...",
    "Initialising interface...",
    "Ready.",
];

/// Run the boot animation if enabled in settings. Returns when complete.
pub fn run_if_enabled() {
    let settings = persistence::load();
    let enabled = persistence::get_bool(&settings, "display.boot_animation", true);
    if !enabled {
        return;
    }

    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).expect("Failed to enter alternate screen");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let total_steps = STEPS.len();
    let step_delay = Duration::from_millis(400);

    for step in 0..=total_steps {
        terminal
            .draw(|frame| render_splash(frame, step, total_steps))
            .ok();
        if step < total_steps {
            thread::sleep(step_delay);
        }
    }

    // Brief pause on the completed screen
    thread::sleep(Duration::from_millis(300));

    // Clean up — the main app will re-enter alternate screen
    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
}

fn render_splash(frame: &mut Frame, completed_steps: usize, total_steps: usize) {
    let area = frame.area();

    // Vertical centering: logo (6) + spacing (1) + subtitle (1) + spacing (2) + steps (5) + spacing (1) + bar (1) = 17
    let content_height: u16 = 17;
    let v_pad = area.height.saturating_sub(content_height) / 2;

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(v_pad),
            Constraint::Length(content_height),
            Constraint::Min(0),
        ])
        .split(area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // logo
            Constraint::Length(1),  // spacing
            Constraint::Length(1),  // subtitle
            Constraint::Length(2),  // spacing
            Constraint::Length(5),  // steps
            Constraint::Length(1),  // spacing
            Constraint::Length(1),  // progress bar
        ])
        .split(outer[1]);

    // Logo
    let logo_lines: Vec<Line> = TUIX_LOGO
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(Color::Cyan))))
        .collect();
    frame.render_widget(
        Paragraph::new(logo_lines).alignment(Alignment::Center),
        inner[0],
    );

    // Subtitle
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "created by Ben Mulligan",
            Style::default().fg(Color::DarkGray),
        )))
        .alignment(Alignment::Center),
        inner[2],
    );

    // Steps
    let step_lines: Vec<Line> = STEPS
        .iter()
        .enumerate()
        .map(|(i, text)| {
            if i < completed_steps {
                Line::from(vec![
                    Span::styled(" [✓] ", Style::default().fg(Color::Green)),
                    Span::styled(*text, Style::default().fg(Color::White)),
                ])
            } else if i == completed_steps {
                Line::from(vec![
                    Span::styled(" [·] ", Style::default().fg(Color::Yellow)),
                    Span::styled(*text, Style::default().fg(Color::Yellow)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(" [ ] ", Style::default().fg(Color::DarkGray)),
                    Span::styled(*text, Style::default().fg(Color::DarkGray)),
                ])
            }
        })
        .collect();

    // Center the steps horizontally
    let steps_width: u16 = 40;
    let steps_pad = inner[4].width.saturating_sub(steps_width) / 2;
    let steps_area = Rect {
        x: inner[4].x + steps_pad,
        y: inner[4].y,
        width: steps_width.min(inner[4].width),
        height: inner[4].height,
    };
    frame.render_widget(Paragraph::new(step_lines), steps_area);

    // Progress bar
    let bar_width: u16 = 30;
    let bar_pad = inner[6].width.saturating_sub(bar_width) / 2;
    let bar_area = Rect {
        x: inner[6].x + bar_pad,
        y: inner[6].y,
        width: bar_width.min(inner[6].width),
        height: 1,
    };

    let filled = if total_steps > 0 {
        (completed_steps as u16 * bar_width) / total_steps as u16
    } else {
        0
    };
    let empty = bar_width.saturating_sub(filled);

    let bar_line = Line::from(vec![
        Span::styled(
            "█".repeat(filled as usize),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            "░".repeat(empty as usize),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(bar_line).alignment(Alignment::Left), bar_area);
}
