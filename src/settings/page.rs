/// Settings page — rendered in the TUIX main container.
///
/// Left pane: settings categories with emoji icons.
/// Right pane: selected category's settings content.

use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::pages::{
    about, appearance, audio, button_mapping, display, git, hotkeys, power, utilities,
    wifi_bluetooth,
};
use super::state::{SettingsCategory, SettingsState};

pub fn render(frame: &mut Frame, area: Rect, border_style: Style, ss: &mut SettingsState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Settings ")
        .title_alignment(ratatui::layout::Alignment::Right)
        .style(border_style);
    frame.render_widget(block, area);

    let inner = area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });

    // Split into left (categories) and right (content) panes
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(26), Constraint::Fill(1)])
        .split(inner);

    let left_area = panes[0];
    let right_area = panes[1];

    // --- Left pane: category list ---
    let categories = SettingsCategory::ALL;
    let mut left_lines: Vec<Line> = Vec::new();

    for (i, cat) in categories.iter().enumerate() {
        let is_selected = i == ss.category_cursor;
        let available = cat.is_available();
        let in_left = !ss.in_right_pane;

        let prefix = if is_selected && in_left { " » " } else { "   " };

        let style = if is_selected && in_left {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else if is_selected && ss.in_right_pane {
            // Selected but focus is in right pane — highlight but no bg
            Style::default().fg(Color::Cyan)
        } else if !available {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        left_lines.push(Line::from(Span::styled(
            format!("{}{}", prefix, cat.label()),
            style,
        )));
    }

    frame.render_widget(Paragraph::new(left_lines), left_area);

    // --- Vertical separator ---
    // Draw a thin border between panes using the right pane's block
    let right_block = Block::default()
        .borders(Borders::LEFT)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(right_block, right_area);

    let right_inner = right_area.inner(Margin {
        horizontal: 1,
        vertical: 0,
    });

    // --- Right pane: category content ---
    let cat = ss.selected_category();
    let cursor = ss.right_cursor;
    let scroll = ss.right_scroll;

    // Category title
    let title_area = Rect {
        x: right_inner.x,
        y: right_inner.y,
        width: right_inner.width,
        height: 2.min(right_inner.height),
    };
    let content_area = Rect {
        x: right_inner.x,
        y: right_inner.y + 2,
        width: right_inner.width,
        height: right_inner.height.saturating_sub(2),
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}", cat.label()),
            Style::default().fg(Color::White),
        ))),
        title_area,
    );

    // Render the selected category's content
    match cat {
        SettingsCategory::About => about::render(frame, content_area, cursor, scroll),
        SettingsCategory::WifiBluetooth => {
            wifi_bluetooth::render(frame, content_area, cursor, scroll)
        }
        SettingsCategory::Audio => audio::render(frame, content_area, cursor, scroll),
        SettingsCategory::Display => display::render(frame, content_area, cursor, scroll),
        SettingsCategory::Appearance => appearance::render(frame, content_area, cursor, scroll),
        SettingsCategory::ButtonMapping => {
            button_mapping::render(frame, content_area, cursor, scroll)
        }
        SettingsCategory::Hotkeys => hotkeys::render(frame, content_area, cursor, scroll),
        SettingsCategory::Git => git::render(frame, content_area, cursor, scroll),
        SettingsCategory::Utilities => utilities::render(frame, content_area, cursor, scroll),
        SettingsCategory::Power => power::render(frame, content_area, cursor, scroll),
    }
}

/// Get the item count for the currently selected category.
pub fn current_item_count(ss: &SettingsState) -> usize {
    match ss.selected_category() {
        SettingsCategory::About => about::item_count(),
        SettingsCategory::WifiBluetooth => wifi_bluetooth::item_count(),
        SettingsCategory::Audio => audio::item_count(),
        SettingsCategory::Display => display::item_count(),
        SettingsCategory::Appearance => appearance::item_count(),
        SettingsCategory::ButtonMapping => button_mapping::item_count(),
        SettingsCategory::Hotkeys => hotkeys::item_count(),
        SettingsCategory::Git => git::item_count(),
        SettingsCategory::Utilities => utilities::item_count(),
        SettingsCategory::Power => power::item_count(),
    }
}

/// Result from a settings Enter action.
pub enum SettingsAction {
    None,
    Quit,
    Restart,
}

/// Handle Enter press in the right pane.
pub fn handle_right_pane_enter(ss: &SettingsState) -> SettingsAction {
    let cursor = ss.right_cursor;
    match ss.selected_category() {
        SettingsCategory::Display => { display::handle_enter(cursor); SettingsAction::None }
        SettingsCategory::Appearance => { appearance::handle_enter(cursor); SettingsAction::None }
        SettingsCategory::ButtonMapping => { button_mapping::handle_enter(cursor); SettingsAction::None }
        SettingsCategory::Hotkeys => { hotkeys::handle_enter(cursor); SettingsAction::None }
        SettingsCategory::Git => { git::handle_enter(cursor); SettingsAction::None }
        SettingsCategory::Utilities => { utilities::handle_enter(cursor); SettingsAction::None }
        SettingsCategory::Power => power::handle_enter(cursor),
        _ => SettingsAction::None,
    }
}
