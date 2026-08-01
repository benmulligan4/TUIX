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

use crate::settings::pages::appearance::color_from_name;
use crate::settings::persistence;

const TUIX_LOGO: &[&str] = &[
    "████████ ██    ██ ██ ██   ██",
    "   ██    ██    ██ ██  ██ ██ ",
    "   ██    ██    ██ ██   ███  ",
    "   ██    ██    ██ ██  ██ ██ ",
    "   ██     ██████  ██ ██   ██",
    "",
    "   ▀█▀   █  █   █   █ █  █",
    "    █    █  █   █    ██   ",
    "    █    █  █   █    ██   ",
    "    █    █  █   █   █ █  █",
    "    █     ██    █  █   █ █",
];

struct BootStep {
    label: &'static str,
    duration_ms: u64,
}

const BOOT_STEPS: &[BootStep] = &[
    BootStep { label: "Starting TUIX...", duration_ms: 350 },
    BootStep { label: "Starting services...", duration_ms: 500 },
    BootStep { label: "Loading applications...", duration_ms: 600 },
    BootStep { label: "Initialising interface...", duration_ms: 300 },
    BootStep { label: "Ready.", duration_ms: 200 },
];

/// Run the boot animation if enabled in settings.
pub fn run_if_enabled() {
    let settings = persistence::load();
    let enabled = persistence::get_bool(&settings, "appearance.boot_animation", true);
    if !enabled {
        return;
    }

    let color_name = persistence::get_str(&settings, "appearance.boot_animation_color", "White");
    let logo_color = color_from_name(&color_name);

    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).expect("Failed to enter alternate screen");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let total_duration: u64 = BOOT_STEPS.iter().map(|s| s.duration_ms).sum();
    let mut elapsed: u64 = 0;

    for step in BOOT_STEPS {
        // Animate the progress bar within this step with sub-increments
        let increments: u64 = 4;
        let increment_delay = step.duration_ms / increments;
        for i in 1..=increments {
            let sub_elapsed = elapsed + (step.duration_ms * i / increments);
            let progress = sub_elapsed as f32 / total_duration as f32;
            thread::sleep(Duration::from_millis(increment_delay));
            terminal
                .draw(|frame| render_splash(frame, step.label, progress.min(1.0), logo_color))
                .ok();
        }
        elapsed += step.duration_ms;
    }

    // Brief pause on completed screen
    thread::sleep(Duration::from_millis(250));

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
}

fn render_splash(frame: &mut Frame, status: &str, progress: f32, logo_color: Color) {
    let area = frame.area();

    // logo (11) + spacing (2) + status (1) + spacing (1) + bar (1) + spacing (1) + subtitle (1) = 18
    let content_height: u16 = 18;
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
            Constraint::Length(11), // logo
            Constraint::Length(2),  // spacing
            Constraint::Length(1),  // status line
            Constraint::Length(1),  // spacing
            Constraint::Length(1),  // progress bar
            Constraint::Length(1),  // spacing
            Constraint::Length(1),  // subtitle
        ])
        .split(outer[1]);

    // Logo
    let logo_lines: Vec<Line> = TUIX_LOGO
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(logo_color))))
        .collect();
    frame.render_widget(
        Paragraph::new(logo_lines).alignment(Alignment::Center),
        inner[0],
    );

    // Status line
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            status,
            Style::default().fg(Color::Gray),
        )))
        .alignment(Alignment::Center),
        inner[2],
    );

    // Progress bar
    let bar_width: u16 = 36;
    let bar_pad = inner[4].width.saturating_sub(bar_width) / 2;
    let bar_area = Rect {
        x: inner[4].x + bar_pad,
        y: inner[4].y,
        width: bar_width.min(inner[4].width),
        height: 1,
    };

    let filled = (progress * bar_width as f32) as u16;
    let empty = bar_width.saturating_sub(filled);

    let bar_line = Line::from(vec![
        Span::styled(
            "█".repeat(filled as usize),
            Style::default().fg(logo_color),
        ),
        Span::styled(
            "░".repeat(empty as usize),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(bar_line), bar_area);

    // Subtitle
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "created by Ben Mulligan",
            Style::default().fg(Color::DarkGray),
        )))
        .alignment(Alignment::Center),
        inner[6],
    );
}
