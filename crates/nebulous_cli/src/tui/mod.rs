pub mod render;
pub mod state;
pub mod theme;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use backend_core::{Result, TrackerError, TrackerStorage};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::{CrosstermBackend, TestBackend};
use ratatui::buffer::Buffer;
use ratatui::Terminal;

pub use render::{
    locked_centered_rect, render_close_modal, render_create_modal, render_frame, render_move_modal,
    render_scaling_lock_guard, MIN_TERM_HEIGHT, MIN_TERM_WIDTH, WIDE_LAYOUT_MIN_WIDTH,
};
pub use state::{
    input_scroll_window, ActiveModal, CloseField, CloseModalState, CreateField, CreateModalState,
    ItemRow, MoveModalState, PaneFocus, ScopeRow, TuiApp,
};
pub use theme::{
    select_nebula_banner, BannerTier, NebulaBanner, AMBER_ALERT, AMBER_BRIGHT, AMBER_DARK,
    AMBER_DIM, AMBER_PRIMARY, BG_BLACK, PANEL_BG,
};

/// Renders the current [`TuiApp`] state into an off-screen [`ratatui::buffer::Buffer`]
/// at the specified `width` and `height` using [`TestBackend`].
pub fn render_to_buffer(app: &mut TuiApp, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("TestBackend terminal initialization");
    terminal
        .draw(|frame| render_frame(frame, app))
        .expect("TestBackend draw");
    terminal.backend().buffer().clone()
}

/// Renders the current [`TuiApp`] state into a newline-separated UTF-8 string
/// representation of the terminal buffer at `(width, height)` using [`TestBackend`].
pub fn render_to_string(app: &mut TuiApp, width: u16, height: u16) -> String {
    let buf = render_to_buffer(app, width, height);
    let area = buf.area;
    let mut out = String::new();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell((x, y)) {
                out.push_str(cell.symbol());
            }
        }
        out.push('\n');
    }
    out
}

struct TerminalCleanupGuard;

impl Drop for TerminalCleanupGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

/// Launches the interactive retro Amber-on-Black TUI in the active terminal.
pub fn run_tui(storage: TrackerStorage, cwd: Option<PathBuf>) -> Result<()> {
    let mut app = TuiApp::new(storage, cwd)?;

    enable_raw_mode().map_err(|source| TrackerError::Io {
        path: PathBuf::from("/dev/tty"),
        source,
    })?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|source| TrackerError::Io {
        path: PathBuf::from("/dev/tty"),
        source,
    })?;
    let _guard = TerminalCleanupGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|source| TrackerError::Io {
        path: PathBuf::from("/dev/tty"),
        source,
    })?;

    loop {
        terminal
            .draw(|frame| render_frame(frame, &mut app))
            .map_err(|source| TrackerError::Io {
                path: PathBuf::from("/dev/tty"),
                source,
            })?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(250)).map_err(|source| TrackerError::Io {
            path: PathBuf::from("/dev/tty"),
            source,
        })? {
            if let Event::Key(key) = event::read().map_err(|source| TrackerError::Io {
                path: PathBuf::from("/dev/tty"),
                source,
            })? {
                if key.kind == KeyEventKind::Press {
                    if let Err(err) = app.handle_key(key) {
                        app.status_message = Some(format!("Error: {err}"));
                    }
                }
            }
        }
    }

    Ok(())
}
