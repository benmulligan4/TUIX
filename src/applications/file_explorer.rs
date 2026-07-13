/// File Explorer — internal TUIX app.
///
/// A file explorer built on ratatui-explorer. Navigate directories
/// with arrow keys / WASD. Enter opens directories, Q/Backspace
/// closes the app.
///
/// TUIX Internal App API:
///     start()               — called when the app is opened
///     stop()                — called when the app is closed
///     render(frame, area)   — draw the app inside the main container
///     handle_key(code)      — receive raw key codes; return action

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::Style,
    Frame,
};
use ratatui_explorer::{FileExplorer, Theme};

/// Action returned by the file explorer to the TUIX shell.
pub enum FileExplorerAction {
    Back,
}

pub struct FileExplorerApp {
    explorer: Option<FileExplorer>,
}

impl FileExplorerApp {
    pub fn new() -> Self {
        Self { explorer: None }
    }

    pub fn start(&mut self) {
        let theme = Theme::default().add_default_title();
        self.explorer = FileExplorer::with_theme(theme).ok();
    }

    pub fn stop(&mut self) {
        self.explorer = None;
    }

    /// Handle a key code string from the TUIX shell.
    /// Returns Some(FileExplorerAction::Back) to close the app.
    pub fn handle_key(&mut self, code: &str) -> Option<FileExplorerAction> {
        // Q / Backspace closes the app (Back action from TUIX)
        if code == "q" || code == "Q" || code == "Backspace" {
            return Some(FileExplorerAction::Back);
        }

        // Map TUIX key names to crossterm KeyCode for the explorer
        let key_code = match code {
            "Up" => KeyCode::Up,
            "Down" => KeyCode::Down,
            "Left" => KeyCode::Left,
            "Right" => KeyCode::Right,
            "Enter" => KeyCode::Enter,
            _ => return None,
        };

        let key_event = KeyEvent {
            code: key_code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };

        if let Some(ref mut explorer) = self.explorer {
            let event = Event::Key(key_event);
            let _ = explorer.handle(&event);
        }

        None
    }

    /// Render the file explorer in the given area.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, _border_style: Style) {
        if let Some(ref explorer) = self.explorer {
            frame.render_widget(&explorer.widget(), area);
        }
    }
}
