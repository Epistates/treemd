mod app;
mod help_text;
mod interactive;
mod syntax;
pub mod terminal_compat;
pub mod theme;
pub mod tty; // Public module for TTY handling
mod ui;

pub use app::App;
pub use interactive::InteractiveState;
pub use terminal_compat::{ColorMode, TerminalCapabilities};
pub use theme::ThemeName;

use color_eyre::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::DefaultTerminal;
use std::io::stdout;
use std::time::Duration;

/// Suspend the TUI, run an external editor, then restore the TUI
fn run_editor(terminal: &mut DefaultTerminal, file_path: &std::path::PathBuf) -> Result<()> {
    // Leave alternate screen and disable raw mode to give editor full terminal control
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    // Open file in editor (blocks until editor closes)
    let result = edit::edit_file(file_path);

    // Restore terminal state
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    terminal.clear()?;

    // Return editor result
    result.map_err(|e| e.into())
}

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

        // Handle pending editor file open (from link following non-markdown files)
        if let Some(file_path) = app.pending_editor_file.take() {
            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");
            match run_editor(terminal, &file_path) {
                Ok(_) => {
                    app.status_message = Some(format!("✓ Opened {} in editor", filename));
                }
                Err(e) => {
                    app.status_message = Some(format!("✗ Failed to open {}: {}", filename, e));
                }
            }
            continue; // Redraw after returning from editor
        }

        // Poll for events with timeout to allow status message expiration
        // Use 100ms timeout for responsive UI updates
        if !tty::poll_event(Duration::from_millis(100))? {
            // No event, just continue loop to redraw (handles status message timeout)
            continue;
        }

        if let Event::Key(key) = tty::read_event()? {
            if key.kind == KeyEventKind::Press {
                // Handle help mode scrolling
                if app.show_help {
                    match key.code {
                        KeyCode::Char('?') | KeyCode::Esc => app.toggle_help(),
                        KeyCode::Char('j') | KeyCode::Down => app.scroll_help_down(),
                        KeyCode::Char('k') | KeyCode::Up => app.scroll_help_up(),
                        // Copy operations work in help mode too
                        KeyCode::Char('y') => app.copy_content(),
                        KeyCode::Char('Y') => app.copy_anchor(),
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
                        // Copy operations work in theme picker too
                        KeyCode::Char('y') => app.copy_content(),
                        KeyCode::Char('Y') => app.copy_anchor(),
                        KeyCode::Char('q') => return Ok(()),
                        _ => {}
                    }
                }
                // Handle interactive mode
                else if app.mode == app::AppMode::Interactive {
                    // Check if we're in table navigation mode
                    if app.interactive_state.is_in_table_mode() {
                        // Table navigation mode - handle hjkl navigation
                        match key.code {
                            KeyCode::Esc => {
                                app.interactive_state.exit_table_mode();
                                app.status_message = Some(app.interactive_state.status_text());
                            }
                            KeyCode::Char('h') | KeyCode::Left => {
                                // Extract table dimensions first
                                let (rows, cols) = if let Some(element) =
                                    app.interactive_state.current_element()
                                {
                                    if let crate::tui::interactive::ElementType::Table {
                                        rows,
                                        cols,
                                        ..
                                    } = &element.element_type
                                    {
                                        Some((*rows, *cols))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                                .unwrap_or((0, 0));

                                if cols > 0 {
                                    app.interactive_state.table_move_left();
                                    app.status_message = Some(
                                        app.interactive_state.table_status_text(rows + 1, cols),
                                    );
                                }
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                // Extract table dimensions first
                                let (rows, cols) = if let Some(element) =
                                    app.interactive_state.current_element()
                                {
                                    if let crate::tui::interactive::ElementType::Table {
                                        rows,
                                        cols,
                                        ..
                                    } = &element.element_type
                                    {
                                        Some((*rows, *cols))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                                .unwrap_or((0, 0));

                                if rows > 0 {
                                    app.interactive_state.table_move_down(rows + 1);
                                    app.status_message = Some(
                                        app.interactive_state.table_status_text(rows + 1, cols),
                                    );
                                }
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                // Extract table dimensions first
                                let (rows, cols) = if let Some(element) =
                                    app.interactive_state.current_element()
                                {
                                    if let crate::tui::interactive::ElementType::Table {
                                        rows,
                                        cols,
                                        ..
                                    } = &element.element_type
                                    {
                                        Some((*rows, *cols))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                                .unwrap_or((0, 0));

                                if rows > 0 {
                                    app.interactive_state.table_move_up();
                                    app.status_message = Some(
                                        app.interactive_state.table_status_text(rows + 1, cols),
                                    );
                                }
                            }
                            KeyCode::Char('l') | KeyCode::Right => {
                                // Extract table dimensions first
                                let (rows, cols) = if let Some(element) =
                                    app.interactive_state.current_element()
                                {
                                    if let crate::tui::interactive::ElementType::Table {
                                        rows,
                                        cols,
                                        ..
                                    } = &element.element_type
                                    {
                                        Some((*rows, *cols))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                                .unwrap_or((0, 0));

                                if cols > 0 {
                                    app.interactive_state.table_move_right(cols);
                                    app.status_message = Some(
                                        app.interactive_state.table_status_text(rows + 1, cols),
                                    );
                                }
                            }
                            KeyCode::Char('y') => {
                                // Copy cell
                                if let Err(e) = app.copy_table_cell() {
                                    app.status_message = Some(format!("✗ Error: {}", e));
                                }
                            }
                            KeyCode::Char('Y') => {
                                // Copy row
                                if let Err(e) = app.copy_table_row() {
                                    app.status_message = Some(format!("✗ Error: {}", e));
                                }
                            }
                            KeyCode::Char('r') => {
                                // Copy table as markdown
                                if let Err(e) = app.copy_table_markdown() {
                                    app.status_message = Some(format!("✗ Error: {}", e));
                                }
                            }
                            KeyCode::Enter => {
                                // Enter cell edit mode
                                if let Err(e) = app.enter_cell_edit_mode() {
                                    app.status_message = Some(format!("✗ Error: {}", e));
                                }
                            }
                            KeyCode::Char('q') => return Ok(()),
                            _ => {}
                        }
                    } else {
                        // Regular interactive mode
                        // Clear status message on most key presses
                        if key.code != KeyCode::Tab {
                            app.status_message = None;
                        }

                        match key.code {
                            KeyCode::Esc | KeyCode::Char('i') => app.exit_interactive_mode(),
                            KeyCode::Tab => {
                                if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    app.interactive_state.previous();
                                } else {
                                    app.interactive_state.next();
                                }
                                // Auto-scroll to keep element in view
                                app.scroll_to_interactive_element(20);
                                // Update status bar
                                app.status_message = Some(app.interactive_state.status_text());
                            }
                            KeyCode::Char('j') | KeyCode::Down => {
                                app.interactive_state.next();
                                // Auto-scroll to keep element in view
                                app.scroll_to_interactive_element(20);
                                app.status_message = Some(app.interactive_state.status_text());
                            }
                            KeyCode::Char('k') | KeyCode::Up => {
                                app.interactive_state.previous();
                                // Auto-scroll to keep element in view
                                app.scroll_to_interactive_element(20);
                                app.status_message = Some(app.interactive_state.status_text());
                            }
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                // Activate the selected element
                                if let Err(e) = app.activate_interactive_element() {
                                    app.status_message = Some(format!("✗ Error: {}", e));
                                }
                                // Update content metrics after actions that might change content
                                app.update_content_metrics();
                            }
                            KeyCode::Char('y') => {
                                // Copy action - delegate to activate for code/image elements
                                if let Err(e) = app.activate_interactive_element() {
                                    app.status_message = Some(format!("✗ Error: {}", e));
                                }
                            }
                            KeyCode::Char('q') => return Ok(()),
                            _ => {}
                        }
                    }
                }
                // Handle cell edit mode
                else if app.mode == app::AppMode::CellEdit {
                    match key.code {
                        KeyCode::Esc => {
                            // Cancel editing
                            app.mode = app::AppMode::Interactive;
                            app.status_message = Some("Editing cancelled".to_string());
                        }
                        KeyCode::Enter => {
                            // Save the edited cell
                            match app.save_edited_cell() {
                                Ok(()) => {
                                    app.mode = app::AppMode::Interactive;
                                }
                                Err(e) => {
                                    app.status_message = Some(format!("✗ Error saving: {}", e));
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            app.cell_edit_value.pop();
                        }
                        KeyCode::Char(c) => {
                            app.cell_edit_value.push(c);
                        }
                        _ => {}
                    }
                }
                // Handle link follow mode
                else if app.mode == app::AppMode::LinkFollow {
                    // Clear status message on any key press in link mode
                    app.status_message = None;

                    // Handle search input mode
                    if app.link_search_active {
                        match key.code {
                            KeyCode::Esc => {
                                // Stop search but keep filter
                                app.stop_link_search();
                            }
                            KeyCode::Enter => {
                                // Stop search and follow selected link
                                app.stop_link_search();
                                if let Err(e) = app.follow_selected_link() {
                                    app.status_message = Some(format!("✗ Error: {}", e));
                                }
                                app.update_content_metrics();
                            }
                            KeyCode::Backspace => {
                                app.link_search_pop();
                            }
                            KeyCode::Char(c) => {
                                app.link_search_push(c);
                            }
                            KeyCode::Down => app.next_link(),
                            KeyCode::Up => app.previous_link(),
                            _ => {}
                        }
                    } else {
                        // Normal link follow mode
                        match key.code {
                            KeyCode::Esc => {
                                if !app.link_search_query.is_empty() {
                                    // First Esc clears the search
                                    app.clear_link_search();
                                } else {
                                    app.exit_link_follow_mode();
                                }
                            }
                            KeyCode::Enter => {
                                if let Err(e) = app.follow_selected_link() {
                                    // Show error in status message
                                    app.status_message = Some(format!("✗ Error: {}", e));
                                }
                                app.update_content_metrics();
                            }
                            KeyCode::Tab => {
                                if key.modifiers.contains(KeyModifiers::SHIFT) {
                                    app.previous_link();
                                } else {
                                    app.next_link();
                                }
                            }
                            KeyCode::Char('/') => {
                                // Start search mode
                                app.start_link_search();
                            }
                            KeyCode::Char('j') | KeyCode::Down => app.next_link(),
                            KeyCode::Char('k') | KeyCode::Up => app.previous_link(),
                            KeyCode::Char(c @ '1'..='9') => {
                                // Direct link selection by number (searches original indices)
                                let idx = c.to_digit(10).unwrap() as usize - 1;
                                // Find this index in the filtered list
                                if let Some(display_idx) =
                                    app.filtered_link_indices.iter().position(|&i| i == idx)
                                {
                                    app.selected_link_idx = Some(display_idx);
                                }
                            }
                            KeyCode::Char('p') => {
                                // Jump to parent heading while staying in link mode
                                app.jump_to_parent_links();
                            }
                            // Copy operations work in link mode too
                            KeyCode::Char('y') => app.copy_content(),
                            KeyCode::Char('Y') => app.copy_anchor(),
                            KeyCode::Char('q') => return Ok(()),
                            _ => {}
                        }
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
                    // Clear status message on any key press in normal mode
                    if app.status_message.is_some() && key.code != KeyCode::Char('f') {
                        app.status_message = None;
                    }

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
                        KeyCode::Char('p') => app.jump_to_parent(),
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
                        // Edit file
                        KeyCode::Char('e') => {
                            // Run editor with proper terminal suspend/restore
                            match run_editor(terminal, &app.current_file_path) {
                                Ok(_) => {
                                    // Reload file after successful edit
                                    if let Err(e) = app.reload_current_file() {
                                        app.status_message =
                                            Some(format!("✗ Failed to reload: {}", e));
                                    } else {
                                        app.status_message =
                                            Some("✓ File reloaded after editing".to_string());
                                    }
                                    app.update_content_metrics();
                                }
                                Err(e) => {
                                    app.status_message = Some(format!("✗ Editor failed: {}", e));
                                }
                            }
                        }
                        // Interactive element navigation
                        KeyCode::Char('i') => app.enter_interactive_mode(),
                        // Raw source toggle
                        KeyCode::Char('r') => app.toggle_raw_source(),
                        // Link following
                        KeyCode::Char('f') => app.enter_link_follow_mode(),
                        KeyCode::Char('b') | KeyCode::Backspace => {
                            if app.go_back().is_ok() {
                                app.update_content_metrics();
                            }
                        }
                        KeyCode::Char('F') => {
                            // Forward navigation (Shift+F)
                            if app.go_forward().is_ok() {
                                app.update_content_metrics();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
