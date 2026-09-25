/// Boot intro animation, shown on startup unless disabled in Settings → Appearance.
/// The two styles share the loading bar, stage text and skip handling.

pub mod modern;
pub mod retro;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{
    backend::Backend,
    style::{Color, Style},
    text::Span,
    Terminal,
};

/// Partial-block characters for sub-cell progress bar resolution.
const PARTIALS: [char; 7] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉'];

const STAGES: &[&str] = &[
    "INITIALISING KERNEL",
    "MOUNTING CONFIGURATION",
    "LOADING DASHBOARDS",
    "REGISTERING APPLICATIONS",
    "STARTING SHELL",
];

const DURATION_MS: u64 = 2600;
const FRAME_MS: u64 = 33;
const HOLD_MS: u64 = 600;

/// Play the intro animation in the configured style. Returns early if the user
/// presses a key.
pub fn play<B: Backend>(terminal: &mut Terminal<B>, accent: Color, style: &str) -> io::Result<()> {
    if style.eq_ignore_ascii_case("Retro") {
        retro::play(terminal, accent)
    } else {
        modern::play(terminal, accent)
    }
}

fn skip_requested() -> io::Result<bool> {
    if event::poll(Duration::from_millis(FRAME_MS))? {
        if let Event::Key(key) = event::read()? {
            return Ok(key.kind == KeyEventKind::Press);
        }
    }
    Ok(false)
}

/// Nudge the linear ratio so the bar stutters between stages like a real boot.
fn eased(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    let wobble = (t * STAGES.len() as f64 * std::f64::consts::PI).sin() * 0.035;
    (t + wobble).clamp(0.0, 1.0)
}

fn center(span: Span<'_>, width: u16) -> Vec<Span<'_>> {
    let pad = (width as usize).saturating_sub(span.content.chars().count()) / 2;
    vec![Span::raw(" ".repeat(pad)), span]
}

fn bar_spans(width: usize, ratio: f64, accent: Color, trough: Color) -> Vec<Span<'static>> {
    let eighths = (width as f64 * 8.0 * ratio).round() as usize;
    let full = eighths / 8;
    let remainder = eighths % 8;

    let mut filled = "█".repeat(full.min(width));
    let mut used = full.min(width);
    if remainder > 0 && used < width {
        filled.push(PARTIALS[remainder - 1]);
        used += 1;
    }

    let mut spans = vec![Span::styled(filled, Style::default().fg(accent))];
    if used < width {
        spans.push(Span::styled(
            "░".repeat(width - used),
            Style::default().fg(trough),
        ));
    }
    spans
}
