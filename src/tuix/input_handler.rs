/// TUIX — normalize raw crossterm key codes into Action enum values.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::models::Action;

/// Return the Action for a raw key event, or None if not mapped.
pub fn map_key(key: KeyEvent) -> Option<Action> {
    // Ignore key events with Ctrl/Alt modifiers (except for plain keys)
    if key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
        return None;
    }

    match key.code {
        // Navigation — arrows
        KeyCode::Up => Some(Action::Up),
        KeyCode::Down => Some(Action::Down),
        KeyCode::Left => Some(Action::Left),
        KeyCode::Right => Some(Action::Right),
        // Navigation — WASD
        KeyCode::Char('w') => Some(Action::Up),
        KeyCode::Char('s') => Some(Action::Down),
        KeyCode::Char('a') => Some(Action::Left),
        KeyCode::Char('d') => Some(Action::Right),
        // Enter / activate
        KeyCode::Enter => Some(Action::Enter),
        KeyCode::Char('e') => Some(Action::Enter),
        // Back
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(Action::Back),
        KeyCode::Backspace => Some(Action::Back),
        // Tab — toggle focus
        KeyCode::Tab => Some(Action::Tab),
        // Quit
        KeyCode::Esc => Some(Action::Quit),
        _ => None,
    }
}

/// Convert an Action to a string key name for forwarding to internal apps.
pub fn action_to_key_name(action: Action) -> Option<&'static str> {
    match action {
        Action::Up => Some("Up"),
        Action::Down => Some("Down"),
        Action::Left => Some("Left"),
        Action::Right => Some("Right"),
        Action::Enter => Some("Enter"),
        Action::Back => Some("q"),
        _ => None,
    }
}
