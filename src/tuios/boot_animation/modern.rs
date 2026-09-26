/// Modern intro — the tuiOS logo rendered as a splash screen above the loading bar.

use std::io::{self, Cursor};
use std::time::{Duration, Instant};

use image::{imageops, imageops::FilterType, DynamicImage, ImageFormat, Rgb, RgbImage};
use ratatui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame, Terminal,
};
use ratatui_splash_screen::{SplashConfig, SplashScreen};

use crate::settings::pages::appearance::hue_to_rgb;

use super::{bar_spans, center, eased, skip_requested, DURATION_MS, HOLD_MS, STAGES};

const LOGO_FADED: &[u8] = include_bytes!("../tuios_logo_faded.png");
const LOGO_STATIC: &[u8] = include_bytes!("../tuios_logo_static.png");

/// Frames the splash takes to resolve from blurred/dark to the full logo.
const SPLASH_STEPS: i32 = 12;

/// Widest the centred text block under the logo is allowed to get.
const CONTENT_MAX_W: u16 = 84;
/// Detail is capped at one cell per pixel, so the logo takes most of the width it can get.
const SPLASH_MAX_W: u16 = 200;
/// Percentage of the screen width the logo is allowed to span.
const SPLASH_WIDTH_PCT: u32 = 80;
/// Below this the logo turns to mush, so the text wordmark is used instead.
const SPLASH_MIN_W: u16 = 40;
const SPLASH_MIN_ROWS: u16 = 3;
/// Cells are roughly twice as tall as they are wide.
const CELL_ASPECT: u32 = 2;
/// Anything brighter than this counts as logo rather than background padding.
const TRIM_LUMA: u16 = 96;
/// Subtitle, spacers, bar and status line rendered under the logo.
const TEXT_ROWS: u16 = 7;

/// Degrees of hue the rainbow tint spans across the logo.
const RAINBOW_SPAN: f32 = 300.0;
/// Degrees of hue the rainbow tint drifts per millisecond.
const RAINBOW_DRIFT: f32 = 0.12;

/// How the logo and loading bar are coloured.
#[derive(Clone, Copy, PartialEq)]
pub enum Rainbow {
    /// Accent colour from Settings.
    Off,
    /// Fixed hue gradient across the logo.
    Static,
    /// Hue gradient that drifts through the spectrum.
    Dynamic,
}

impl Rainbow {
    pub fn from_name(name: &str) -> Self {
        if name.eq_ignore_ascii_case("Rainbow Static") {
            Self::Static
        } else if name.to_ascii_lowercase().starts_with("rainbow") {
            Self::Dynamic
        } else {
            Self::Off
        }
    }

    /// Hue the logo's left edge sits at for this frame.
    fn phase(self, tick: u64) -> f32 {
        match self {
            Self::Dynamic => (tick as f32 * RAINBOW_DRIFT) % 360.0,
            _ => 0.0,
        }
    }
}

/// Which artwork the splash is built from.
#[derive(Clone, Copy, PartialEq)]
pub enum Logo {
    /// Badge that fades out to the left.
    Faded,
    /// Solid badge.
    Static,
}

impl Logo {
    pub fn from_name(name: &str) -> Self {
        if name.eq_ignore_ascii_case("Faded") { Self::Faded } else { Self::Static }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Faded => LOGO_FADED,
            Self::Static => LOGO_STATIC,
        }
    }
}

/// The logo plus the cell box it was built for.
struct Splash {
    screen: SplashScreen,
    /// One pixel per terminal cell, kept for re-tinting.
    cells: RgbImage,
    cells_w: u16,
    cells_h: u16,
}

/// Drop the black padding so the artwork itself fills the space it is given.
fn trim_padding(logo: DynamicImage) -> DynamicImage {
    let rgb = logo.to_rgb8();
    let (mut x0, mut y0, mut x1, mut y1) = (rgb.width(), rgb.height(), 0, 0);
    for (x, y, px) in rgb.enumerate_pixels() {
        if px[0] as u16 + px[1] as u16 + px[2] as u16 > TRIM_LUMA {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    if x1 <= x0 || y1 <= y0 {
        return logo;
    }
    logo.crop_imm(x0, y0, x1 - x0 + 1, y1 - y0 + 1)
}

/// Resample the logo so one source pixel lands on exactly one braille dot.
///
/// The widget colours a whole cell from the last dot drawn in it, so the logo is
/// first resolved down to one pixel per cell and then blown back up to the dot
/// grid — every dot in a cell then shares a colour instead of fighting for it.
fn cell_image(logo: &DynamicImage, cells_w: u16, cells_h: u16) -> RgbImage {
    let cells = logo
        .resize_exact(cells_w as u32, cells_h as u32, FilterType::Lanczos3)
        .to_rgb8();
    // Downscaling smears the small lettering into grey, so pull the edges back
    imageops::unsharpen(&cells, 0.8, 1)
}

fn splash_screen(cells: &RgbImage, steps: i32) -> Option<SplashScreen> {
    let mut data = Vec::new();
    DynamicImage::ImageRgb8(cells.clone())
        .resize_exact(cells.width() * 2, cells.height() * 4, FilterType::Nearest)
        .write_to(&mut Cursor::new(&mut data), ImageFormat::Png)
        .ok()?;
    SplashScreen::new(SplashConfig {
        image_data: &data,
        sha256sum: None,
        render_steps: steps,
        use_colors: true,
    })
    .ok()
}

/// Lay a hue gradient over the mark, keeping each pixel's brightness so the
/// background stays black.
fn rainbow_tint(cells: &RgbImage, phase: f32) -> RgbImage {
    let width = cells.width().max(1) as f32;
    let mut out = cells.clone();
    for (x, _, px) in out.enumerate_pixels_mut() {
        let luma = (px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000;
        let (r, g, b) = hue_to_rgb((phase + x as f32 / width * RAINBOW_SPAN) % 360.0);
        *px = Rgb([
            (r as u32 * luma / 255) as u8,
            (g as u32 * luma / 255) as u8,
            (b as u32 * luma / 255) as u8,
        ]);
    }
    out
}

/// Drift the tint once the blur-in has finished — a one-step screen paints at
/// full colour on its first render, so the reveal is not restarted.
fn drift_tint(splash: &mut Splash, phase: f32) {
    if !splash.screen.is_rendered() {
        return;
    }
    if let Some(screen) = splash_screen(&rainbow_tint(&splash.cells, phase), 1) {
        splash.screen = screen;
    }
}

/// Fit the logo to the screen at its own aspect, leaving room for the text below.
fn splash(width: u16, height: u16, rainbow: Rainbow, logo: Logo) -> Option<Splash> {
    let logo = trim_padding(image::load_from_memory_with_format(logo.bytes(), ImageFormat::Png).ok()?);
    let (px_w, px_h) = (logo.width().max(1), logo.height().max(1));

    let room_w = (width as u32 * SPLASH_WIDTH_PCT / 100).min(SPLASH_MAX_W as u32);
    let room_h = height.saturating_sub(TEXT_ROWS + 2) as u32;
    let cells_w = room_w.min(room_h * CELL_ASPECT * px_w / px_h) as u16;
    let cells_h = (cells_w as u32 * px_h / (px_w * CELL_ASPECT)).max(1) as u16;
    if cells_w < SPLASH_MIN_W || cells_h < SPLASH_MIN_ROWS {
        return None;
    }

    let cells = cell_image(&logo, cells_w, cells_h);
    let screen = if rainbow == Rainbow::Off {
        splash_screen(&cells, SPLASH_STEPS)?
    } else {
        splash_screen(&rainbow_tint(&cells, 0.0), SPLASH_STEPS)?
    };
    Some(Splash {
        screen,
        cells,
        cells_w,
        cells_h,
    })
}

/// Play the intro animation. Returns early if the user presses a key.
pub fn play<B: Backend>(
    terminal: &mut Terminal<B>,
    accent: Color,
    rainbow: Rainbow,
    logo: Logo,
) -> io::Result<()> {
    let start = Instant::now();
    let total = Duration::from_millis(DURATION_MS);

    let size = terminal.size()?;
    let mut splash = splash(size.width, size.height, rainbow, logo);

    loop {
        let elapsed = start.elapsed();
        if elapsed >= total {
            break;
        }
        let tick = elapsed.as_millis() as u64;
        let progress = eased(tick as f64 / DURATION_MS as f64);
        let accent = frame_accent(accent, rainbow, tick);
        if rainbow == Rainbow::Dynamic {
            if let Some(splash) = splash.as_mut() {
                drift_tint(splash, rainbow.phase(tick));
            }
        }

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
        let accent = frame_accent(accent, rainbow, tick);
        if rainbow == Rainbow::Dynamic {
            if let Some(splash) = splash.as_mut() {
                drift_tint(splash, rainbow.phase(tick));
            }
        }
        terminal.draw(|frame| render(frame, splash.as_mut(), 1.0, tick, accent, true))?;
        if skip_requested()? {
            break;
        }
    }

    Ok(())
}

/// In rainbow mode the bar and status text ride the same hue as the logo's left edge.
fn frame_accent(accent: Color, rainbow: Rainbow, tick: u64) -> Color {
    if rainbow == Rainbow::Off {
        return accent;
    }
    let (r, g, b) = hue_to_rgb(rainbow.phase(tick));
    Color::Rgb(r, g, b)
}

fn render(
    frame: &mut Frame,
    splash: Option<&mut Splash>,
    progress: f64,
    tick: u64,
    accent: Color,
    ready: bool,
) {
    let area = frame.area();
    let progress = progress.clamp(0.0, 1.0);

    let dim = Color::Rgb(70, 70, 70);
    let text = Color::Rgb(170, 170, 170);

    let content_w = area.width.saturating_sub(4).clamp(12, CONTENT_MAX_W);
    let (logo_w, logo_rows) = splash
        .as_ref()
        .map_or((0, 0), |s| (s.cells_w, s.cells_h));

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

    let subtitle = if content_w >= 24 {
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

    let text_h = lines.len() as u16;
    let top = area.y + area.height.saturating_sub(logo_rows + text_h) / 2;

    if let Some(splash) = splash {
        frame.render_widget(
            &mut splash.screen,
            Rect {
                x: area.x + area.width.saturating_sub(logo_w) / 2,
                y: top,
                width: logo_w.min(area.width),
                height: logo_rows.min(area.height),
            },
        );
    }

    frame.render_widget(
        Paragraph::new(lines),
        Rect {
            x: area.x + area.width.saturating_sub(content_w) / 2,
            y: top + logo_rows,
            width: content_w.min(area.width),
            height: text_h.min(area.height.saturating_sub(logo_rows)),
        },
    );
}



