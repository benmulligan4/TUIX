/// Boot intro animation — the tuiOS logo splash screen with a loading bar.
/// Shown on startup unless disabled in Settings → Appearance.

use std::io::{self, Cursor};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyEventKind};
use image::{imageops::FilterType, ImageFormat};
use ratatui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame, Terminal,
};
use ratatui_splash_screen::{SplashConfig, SplashScreen};

const LOGO_PNG: &[u8] = include_bytes!("tuios_logo.png");

/// Frames the splash takes to resolve from blurred/dark to the full logo.
const SPLASH_STEPS: i32 = 12;

/// Widest the centred content block is allowed to get.
const CONTENT_MAX_W: u16 = 84;
/// Below this the logo turns to mush, so the text wordmark is used instead.
const CONTENT_MIN_SPLASH_W: u16 = 24;
/// Logo aspect (~3:1) halved again because terminal cells are twice as tall as wide.
const SPLASH_CELL_ASPECT: u16 = 6;
const SPLASH_MIN_ROWS: u16 = 3;
/// Subtitle, spacers, bar and status line rendered under the logo.
const TEXT_ROWS: u16 = 7;

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

/// Resample the logo so one source pixel lands on exactly one braille dot.
///
/// The widget colours a whole cell from the last dot drawn in it, so the logo is
/// first averaged down to one pixel per cell and then blown back up to the dot
/// grid — every dot in a cell then shares a colour instead of fighting for it.
fn splash_image_data(cells_w: u16, cells_h: u16) -> Option<Vec<u8>> {
    let logo = image::load_from_memory_with_format(LOGO_PNG, ImageFormat::Png).ok()?;
    let mut data = Vec::new();
    logo.resize_exact(cells_w as u32, cells_h as u32, FilterType::Triangle)
        .resize_exact(cells_w as u32 * 2, cells_h as u32 * 4, FilterType::Nearest)
        .write_to(&mut Cursor::new(&mut data), ImageFormat::Png)
        .ok()?;
    Some(data)
}

fn splash_screen(cells_w: u16, cells_h: u16) -> Option<SplashScreen> {
    if cells_w < CONTENT_MIN_SPLASH_W || cells_h < SPLASH_MIN_ROWS {
        return None;
    }
    let data = splash_image_data(cells_w, cells_h)?;
    SplashScreen::new(SplashConfig {
        image_data: &data,
        sha256sum: None,
        render_steps: SPLASH_STEPS,
        use_colors: true,
    })
    .ok()
}

/// Width of the centred content block and height of the logo above it, in cells.
fn dims(width: u16, height: u16) -> (u16, u16) {
    let content_w = width.saturating_sub(4).clamp(12, CONTENT_MAX_W);
    let logo_h = (content_w / SPLASH_CELL_ASPECT).min(height.saturating_sub(TEXT_ROWS));
    (content_w, logo_h)
}

/// Play the intro animation. Returns early if the user presses a key.
pub fn play<B: Backend>(terminal: &mut Terminal<B>, accent: Color) -> io::Result<()> {
    let start = Instant::now();
    let total = Duration::from_millis(DURATION_MS);

    let size = terminal.size()?;
    let (content_w, logo_h) = dims(size.width, size.height);
    let mut splash = splash_screen(content_w, logo_h);

    loop {
        let elapsed = start.elapsed();
        if elapsed >= total {
            break;
        }
        let tick = elapsed.as_millis() as u64;
        let progress = eased(tick as f64 / DURATION_MS as f64);

        terminal.draw(|frame| render(frame, splash.as_mut(), progress, tick, accent, false))?;

        if skip_requested()? {
            return Ok(());
        }
    }

    // Hold on "SYSTEM READY" so the boot reads as finished
    let hold = Instant::now();
    loop {
        let elapsed = hold.elapsed();
        if elapsed >= Duration::from_millis(HOLD_MS) {
            break;
        }
        let tick = DURATION_MS + elapsed.as_millis() as u64;
        terminal.draw(|frame| render(frame, splash.as_mut(), 1.0, tick, accent, true))?;
        if skip_requested()? {
            break;
        }
    }

    Ok(())
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

fn render(
    frame: &mut Frame,
    splash: Option<&mut SplashScreen>,
    progress: f64,
    tick: u64,
    accent: Color,
    ready: bool,
) {
    let area = frame.area();
    let progress = progress.clamp(0.0, 1.0);

    let dim = Color::Rgb(70, 70, 70);
    let text = Color::Rgb(170, 170, 170);

    let (content_w, splash_h) = dims(area.width, area.height);
    let splash = splash.filter(|_| splash_h >= SPLASH_MIN_ROWS && content_w >= CONTENT_MIN_SPLASH_W);
    let logo_h = if splash.is_some() { splash_h } else { 1 };

    let mut lines: Vec<Line> = Vec::new();

    if splash.is_none() {
        let pad = (content_w as usize).saturating_sub(7) / 2;
        lines.push(Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled(
                " tui ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "OS",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    lines.push(Line::default());

    let subtitle = if content_w >= CONTENT_MIN_SPLASH_W {
        "Created by Ben Mulligan"
    } else {
        "Ben Mulligan"
    };
    lines.push(Line::from(center(
        Span::styled(subtitle, Style::default().fg(dim)),
        content_w,
    )));

    lines.push(Line::default());
    lines.push(Line::default());

    // Progress bar: solid accent fill, dark trough, framed by dim half-blocks
    let bar_w = content_w.saturating_sub(2) as usize;
    let mut bar: Vec<Span> = vec![Span::styled("▐", Style::default().fg(dim))];
    bar.extend(bar_spans(bar_w, progress, accent, dim));
    bar.push(Span::styled("▌", Style::default().fg(dim)));
    lines.push(Line::from(bar));

    lines.push(Line::default());

    // Status line: stage on the left with a blinking cursor, percentage on the right
    let stage = if ready {
        "SYSTEM READY"
    } else {
        STAGES[((progress * STAGES.len() as f64) as usize).min(STAGES.len() - 1)]
    };
    let percent = format!("{:>3}%", (progress * 100.0).round() as u16);
    let cursor = if (tick / 400) % 2 == 0 { "█" } else { " " };
    let used = 2 + stage.chars().count() + 1 + percent.chars().count();
    let gap = (content_w as usize).saturating_sub(used);
    lines.push(Line::from(vec![
        Span::styled("> ", Style::default().fg(accent)),
        Span::styled(stage, Style::default().fg(text)),
        Span::styled(cursor, Style::default().fg(accent)),
        Span::raw(" ".repeat(gap)),
        Span::styled(percent, Style::default().fg(accent)),
    ]));

    let content_h = logo_h + lines.len() as u16;
    let rect = Rect {
        x: area.x + area.width.saturating_sub(content_w) / 2,
        y: area.y + area.height.saturating_sub(content_h) / 2,
        width: content_w.min(area.width),
        height: content_h.min(area.height),
    };

    if let Some(splash) = splash {
        let logo_rect = Rect {
            height: logo_h.min(rect.height),
            ..rect
        };
        frame.render_widget(splash, logo_rect);
    }

    let text_rect = Rect {
        y: rect.y + logo_h,
        height: rect.height.saturating_sub(logo_h),
        ..rect
    };
    frame.render_widget(Paragraph::new(lines), text_rect);
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
