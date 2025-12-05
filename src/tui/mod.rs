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

use crate::keybindings::{Action, KeybindingMode};
use color_eyre::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{Event, KeyCode, KeyEventKind};
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
                // Get the current mode for keybinding lookup
                let mode = app.current_keybinding_mode();

                // Handle text input modes specially - they need character input
                match mode {
                    KeybindingMode::CellEdit => {
                        if let KeyCode::Char(c) = key.code {
                            app.cell_edit_value.push(c);
                            continue;
                        }
                    }
                    KeybindingMode::Search => {
                        if let KeyCode::Char(c) = key.code {
                            app.search_input(c);
                            continue;
                        }
                    }
                    KeybindingMode::LinkSearch => {
                        if let KeyCode::Char(c) = key.code {
                            app.link_search_push(c);
                            continue;
                        }
                    }
                    _ => {}
                }

                // Look up action for this key
                let action = app.get_action_for_key(key.code, key.modifiers);

                // Handle direct number jumps in LinkFollow mode (not bound to actions)
                if mode == KeybindingMode::LinkFollow {
                    if let KeyCode::Char(c @ '1'..='9') = key.code {
                        let idx = c.to_digit(10).unwrap() as usize - 1;
                        if let Some(display_idx) =
                            app.filtered_link_indices.iter().position(|&i| i == idx)
                        {
                            app.selected_link_idx = Some(display_idx);
                        }
                        continue;
                    }
                }

                // Process the action
                if let Some(action) = action {
                    if handle_action(&mut app, terminal, action)? {
                        return Ok(()); // Quit requested
                    }
                }
            }
        }
    }
}

/// Handle a keybinding action, returning true if quit is requested
fn handle_action(
    app: &mut App,
    terminal: &mut DefaultTerminal,
    action: Action,
) -> Result<bool> {
    // Clear status message on most actions (except some special cases)
    let should_clear_status = !matches!(
        action,
        Action::EnterLinkFollowMode | Action::InteractiveNextLink
    );
    if should_clear_status && app.status_message.is_some() {
        app.status_message = None;
    }

    match action {
        // Application
        Action::Quit => return Ok(true),

        // Navigation
        Action::Next => app.next(),
        Action::Previous => app.previous(),
        Action::First => app.first(),
        Action::Last => app.last(),
        Action::PageDown => app.scroll_page_down(),
        Action::PageUp => app.scroll_page_up(),
        Action::JumpToParent => app.jump_to_parent(),

        // Outline
        Action::Expand => app.expand(),
        Action::Collapse => app.collapse(),
        Action::ToggleExpand => app.toggle_expand(),
        Action::ToggleFocus => app.toggle_focus(),
        Action::ToggleOutline => app.toggle_outline(),
        Action::OutlineWidthIncrease => app.cycle_outline_width(true),
        Action::OutlineWidthDecrease => app.cycle_outline_width(false),

        // View
        Action::ToggleHelp => app.toggle_help(),
        Action::ToggleThemePicker => app.toggle_theme_picker(),
        Action::ToggleRawSource => app.toggle_raw_source(),

        // Theme picker
        Action::ThemePickerNext => app.theme_picker_next(),
        Action::ThemePickerPrevious => app.theme_picker_previous(),
        Action::ApplyTheme => app.apply_selected_theme(),

        // Help navigation
        Action::HelpScrollDown => app.scroll_help_down(),
        Action::HelpScrollUp => app.scroll_help_up(),

        // Mode transitions
        Action::EnterInteractiveMode => app.enter_interactive_mode(),
        Action::ExitInteractiveMode => app.exit_interactive_mode(),
        Action::EnterLinkFollowMode => app.enter_link_follow_mode(),
        Action::EnterSearchMode => app.toggle_search(),
        Action::ExitMode => {
            // Context-dependent exit
            let mode = app.current_keybinding_mode();
            match mode {
                KeybindingMode::LinkFollow => {
                    if !app.link_search_query.is_empty() {
                        app.clear_link_search();
                    } else {
                        app.exit_link_follow_mode();
                    }
                }
                KeybindingMode::LinkSearch => app.stop_link_search(),
                KeybindingMode::InteractiveTable => {
                    app.interactive_state.exit_table_mode();
                    app.status_message = Some(app.interactive_state.status_text());
                }
                KeybindingMode::Search => app.toggle_search(),
                _ => {}
            }
        }

        // Search
        Action::SearchBackspace => {
            let mode = app.current_keybinding_mode();
            match mode {
                KeybindingMode::Search => app.search_backspace(),
                KeybindingMode::LinkSearch => app.link_search_pop(),
                KeybindingMode::CellEdit => {
                    app.cell_edit_value.pop();
                }
                _ => {}
            }
        }
        Action::ConfirmAction => {
            let mode = app.current_keybinding_mode();
            match mode {
                KeybindingMode::Search => app.toggle_search(),
                KeybindingMode::LinkSearch => {
                    app.stop_link_search();
                    if let Err(e) = app.follow_selected_link() {
                        app.status_message = Some(format!("✗ Error: {}", e));
                    }
                    app.update_content_metrics();
                }
                KeybindingMode::CellEdit => {
                    match app.save_edited_cell() {
                        Ok(()) => app.mode = app::AppMode::Interactive,
                        Err(e) => app.status_message = Some(format!("✗ Error saving: {}", e)),
                    }
                }
                KeybindingMode::ConfirmDialog => {
                    if let Err(e) = app.confirm_file_create() {
                        app.status_message = Some(format!("✗ Error: {}", e));
                    }
                }
                _ => {}
            }
        }
        Action::CancelAction => {
            let mode = app.current_keybinding_mode();
            match mode {
                KeybindingMode::CellEdit => {
                    app.mode = app::AppMode::Interactive;
                    app.status_message = Some("Editing cancelled".to_string());
                }
                KeybindingMode::ConfirmDialog => app.cancel_file_create(),
                _ => {}
            }
        }

        // Link following
        Action::NextLink => app.next_link(),
        Action::PreviousLink => app.previous_link(),
        Action::FollowLink => {
            if let Err(e) = app.follow_selected_link() {
                app.status_message = Some(format!("✗ Error: {}", e));
            }
            app.update_content_metrics();
        }
        Action::LinkSearch => app.start_link_search(),
        Action::JumpToLink1 => select_link_by_number(app, 0),
        Action::JumpToLink2 => select_link_by_number(app, 1),
        Action::JumpToLink3 => select_link_by_number(app, 2),
        Action::JumpToLink4 => select_link_by_number(app, 3),
        Action::JumpToLink5 => select_link_by_number(app, 4),
        Action::JumpToLink6 => select_link_by_number(app, 5),
        Action::JumpToLink7 => select_link_by_number(app, 6),
        Action::JumpToLink8 => select_link_by_number(app, 7),
        Action::JumpToLink9 => select_link_by_number(app, 8),

        // Interactive mode
        Action::InteractiveNext => {
            app.interactive_state.next();
            app.scroll_to_interactive_element(20);
            app.status_message = Some(app.interactive_state.status_text());
        }
        Action::InteractivePrevious => {
            app.interactive_state.previous();
            app.scroll_to_interactive_element(20);
            app.status_message = Some(app.interactive_state.status_text());
        }
        Action::InteractiveNextLink => {
            app.interactive_state.next();
            app.scroll_to_interactive_element(20);
            app.status_message = Some(app.interactive_state.status_text());
        }
        Action::InteractivePreviousLink => {
            app.interactive_state.previous();
            app.scroll_to_interactive_element(20);
            app.status_message = Some(app.interactive_state.status_text());
        }
        Action::InteractiveActivate => {
            if let Err(e) = app.activate_interactive_element() {
                app.status_message = Some(format!("✗ Error: {}", e));
            }
            app.update_content_metrics();
        }
        Action::InteractiveLeft => handle_table_navigation(app, TableDirection::Left),
        Action::InteractiveRight => handle_table_navigation(app, TableDirection::Right),

        // Clipboard
        Action::CopyContent => app.copy_content(),
        Action::CopyAnchor => app.copy_anchor(),

        // Bookmarks
        Action::SetBookmark => app.set_bookmark(),
        Action::JumpToBookmark => app.jump_to_bookmark(),

        // Jump to heading by number
        Action::JumpToHeading1 => app.jump_to_heading(0),
        Action::JumpToHeading2 => app.jump_to_heading(1),
        Action::JumpToHeading3 => app.jump_to_heading(2),
        Action::JumpToHeading4 => app.jump_to_heading(3),
        Action::JumpToHeading5 => app.jump_to_heading(4),
        Action::JumpToHeading6 => app.jump_to_heading(5),
        Action::JumpToHeading7 => app.jump_to_heading(6),
        Action::JumpToHeading8 => app.jump_to_heading(7),
        Action::JumpToHeading9 => app.jump_to_heading(8),

        // File operations
        Action::OpenInEditor => {
            match run_editor(terminal, &app.current_file_path) {
                Ok(_) => {
                    if let Err(e) = app.reload_current_file() {
                        app.status_message = Some(format!("✗ Failed to reload: {}", e));
                    } else {
                        app.status_message = Some("✓ File reloaded after editing".to_string());
                    }
                    app.update_content_metrics();
                }
                Err(e) => {
                    app.status_message = Some(format!("✗ Editor failed: {}", e));
                }
            }
        }
        Action::GoBack => {
            if app.go_back().is_ok() {
                app.update_content_metrics();
            }
        }
        Action::GoForward => {
            if app.go_forward().is_ok() {
                app.update_content_metrics();
            }
        }

        // Content scrolling (same as next/previous in content focus)
        Action::ScrollDown => app.next(),
        Action::ScrollUp => app.previous(),
    }

    Ok(false)
}

/// Select a link by its index
fn select_link_by_number(app: &mut App, idx: usize) {
    if let Some(display_idx) = app.filtered_link_indices.iter().position(|&i| i == idx) {
        app.selected_link_idx = Some(display_idx);
    }
}

/// Direction for table navigation
enum TableDirection {
    Left,
    Right,
}

/// Handle table cell navigation
fn handle_table_navigation(app: &mut App, direction: TableDirection) {
    let (rows, cols) = if let Some(element) = app.interactive_state.current_element() {
        if let crate::tui::interactive::ElementType::Table { rows, cols, .. } =
            &element.element_type
        {
            Some((*rows, *cols))
        } else {
            None
        }
    } else {
        None
    }
    .unwrap_or((0, 0));

    match direction {
        TableDirection::Left => {
            if cols > 0 {
                app.interactive_state.table_move_left();
                app.status_message = Some(app.interactive_state.table_status_text(rows + 1, cols));
            }
        }
        TableDirection::Right => {
            if cols > 0 {
                app.interactive_state.table_move_right(cols);
                app.status_message = Some(app.interactive_state.table_status_text(rows + 1, cols));
            }
        }
    }
}
