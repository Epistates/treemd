mod app;
mod syntax;
mod theme;
mod ui;

pub use app::App;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;

/// Run the TUI application.
///
/// This function handles the main event loop for the interactive terminal interface.
/// It processes keyboard events and renders the UI until the user quits.
///
/// # Arguments
///
/// * `terminal` - A mutable reference to a ratatui terminal
/// * `app` - The App instance to run
///
/// # Returns
///
/// Returns `Ok(())` on successful exit, or an error if something goes wrong.
pub fn run(terminal: &mut DefaultTerminal, app: App) -> Result<()> {
    let mut app = app;

    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                // Handle help mode scrolling
                if app.show_help {
                    match key.code {
                        KeyCode::Char('?') | KeyCode::Esc => app.toggle_help(),
                        KeyCode::Char('j') | KeyCode::Down => app.scroll_help_down(),
                        KeyCode::Char('k') | KeyCode::Up => app.scroll_help_up(),
                        KeyCode::Char('q') => return Ok(()),
                        _ => {}
                    }
                }
                // Handle theme picker mode
                else if app.show_theme_picker {
                    match key.code {
                        KeyCode::Esc => app.toggle_theme_picker(),
                        KeyCode::Enter => app.apply_selected_theme(),
                        KeyCode::Char('j') | KeyCode::Down => app.theme_picker_next(),
                        KeyCode::Char('k') | KeyCode::Up => app.theme_picker_previous(),
                        KeyCode::Char('q') => return Ok(()),
                        _ => {}
                    }
                }
                // Handle search mode separately
                else if app.show_search {
                    match key.code {
                        KeyCode::Esc => app.toggle_search(),
                        KeyCode::Enter => {
                            app.toggle_search();
                            // Keep the filtered results
                        }
                        KeyCode::Char(c) => app.search_input(c),
                        KeyCode::Backspace => app.search_backspace(),
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc if !app.show_help => return Ok(()),
                        KeyCode::Char('?') => app.toggle_help(),
                        KeyCode::Char('/') => app.toggle_search(),
                        KeyCode::Esc if app.show_help => app.toggle_help(),
                        KeyCode::Char('j') | KeyCode::Down => app.next(),
                        KeyCode::Char('k') | KeyCode::Up => app.previous(),
                        KeyCode::Char('d') => app.scroll_page_down(),
                        KeyCode::Char('u') => app.scroll_page_up(),
                        KeyCode::Char('g') => app.first(),
                        KeyCode::Char('G') => app.last(),
                        KeyCode::Enter | KeyCode::Char(' ') => app.toggle_expand(),
                        KeyCode::Tab => app.toggle_focus(),
                        KeyCode::Char('h') | KeyCode::Left => app.collapse(),
                        KeyCode::Char('l') | KeyCode::Right => app.expand(),
                        // New UX features
                        KeyCode::Char('w') => app.toggle_outline(),
                        KeyCode::Char('[') => app.cycle_outline_width(false),
                        KeyCode::Char(']') => app.cycle_outline_width(true),
                        KeyCode::Char('m') => app.set_bookmark(),
                        KeyCode::Char('\'') => app.jump_to_bookmark(),
                        KeyCode::Char('1') => app.jump_to_heading(0),
                        KeyCode::Char('2') => app.jump_to_heading(1),
                        KeyCode::Char('3') => app.jump_to_heading(2),
                        KeyCode::Char('4') => app.jump_to_heading(3),
                        KeyCode::Char('5') => app.jump_to_heading(4),
                        KeyCode::Char('6') => app.jump_to_heading(5),
                        KeyCode::Char('7') => app.jump_to_heading(6),
                        KeyCode::Char('8') => app.jump_to_heading(7),
                        KeyCode::Char('9') => app.jump_to_heading(8),
                        // Theme and clipboard
                        KeyCode::Char('t') => app.toggle_theme_picker(),
                        KeyCode::Char('y') => app.copy_content(),
                        KeyCode::Char('Y') => app.copy_anchor(),
                        _ => {}
                    }
                }
            }
        }
    }
}
