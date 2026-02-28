mod app;
mod help_text;
mod image_cache;
mod interactive;
mod kitty_animation;
mod syntax;
pub mod terminal_compat;
pub mod theme;
pub mod tty; // Public module for TTY handling
mod ui;
mod watcher;

pub use app::{ActionResult, App};
pub use interactive::InteractiveState;
pub use terminal_compat::{ColorMode, TerminalCapabilities};
pub use theme::ThemeName;

use crate::keybindings::Action;
use color_eyre::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen,
    disable_raw_mode, enable_raw_mode,
};
use opensesame::{Editor, EditorConfig};
use ratatui::DefaultTerminal;
use std::io::stdout;
use std::path::Path;
use std::time::Duration;

/// Suspend the TUI, run an external editor, then restore the TUI.
///
/// If line is provided and the editor supports it, the file will be opened at that line.
/// Uses the provided EditorConfig for editor selection and arguments.
fn run_editor(
    terminal: &mut DefaultTerminal,
    file: &Path,
    line: Option<u32>,
    editor_config: &EditorConfig,
) -> Result<()> {
    // Leave alternate screen and disable raw mode to give editor full terminal control
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    // Build editor command with config
    let mut builder = Editor::builder()
        .file(file)
        .with_config(editor_config.clone());

    if let Some(l) = line {
        builder = builder.line(l);
    }

    let result = builder.open();

    // Restore terminal state
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    terminal.clear()?;

    result.map_err(|e| color_eyre::eyre::eyre!("{}", e))
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

    // Handle startup file picker if needed (scans directory)
    if app.startup_needs_file_picker {
        app.enter_file_picker();
    }

    // Auto-hide outline when only 0-1 markdown files in directory
    // (reuses scan data from enter_file_picker if it ran; otherwise scans itself)
    app.auto_hide_outline_if_single_file();

    // Create file watcher for live reload
    let mut file_watcher = watcher::FileWatcher::new().ok();
    if let Some(ref mut watcher) = file_watcher {
        let _ = watcher.watch(&app.current_file_path);
    }

    // Load first image from the document if picker is available
    // (Picker is already initialized in App::new with fallback font size)
    if app.picker.is_some() {
        let content = app.document.content.clone();
        app.load_first_image(&content);
    }

    loop {
        // Use synchronized output when animating GIFs (reduces flicker on iTerm2, etc.)
        // This makes the entire frame update atomic from the terminal's perspective.
        let use_sync = app.is_image_modal_open()
            && app.modal_gif_frames.len() > 1
            && !app.has_kitty_animation(); // Kitty uses native animation, no sync needed

        if use_sync {
            let _ = stdout().execute(BeginSynchronizedUpdate);
        }

        terminal.draw(|frame| ui::render(frame, &mut app))?;

        if use_sync {
            let _ = stdout().execute(EndSynchronizedUpdate);
        }

        // Update file watcher if the current file changed (e.g., via navigation)
        if app.file_path_changed {
            app.file_path_changed = false;
            if let Some(ref mut watcher) = file_watcher {
                let _ = watcher.watch(&app.current_file_path);
            }
        }

        // Handle pending editor file open (from link following non-markdown files)
        if let Some(file_path) = app.pending_editor_file.take() {
            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");
            let editor_config = app.editor_config();
            match run_editor(terminal, &file_path, None, &editor_config) {
                Ok(_) => {
                    app.status_message = Some(format!("✓ Opened {} in editor", filename));
                }
                Err(e) => {
                    app.status_message = Some(format!("✗ Failed to open {}: {}", filename, e));
                }
            }
            continue; // Redraw after returning from editor
        }

        // Poll for events with dynamic timeout:
        // - When GIF is animating: use time until next frame (for smooth playback)
        // - Otherwise: 100ms for responsive UI updates
        let poll_timeout = app
            .time_until_next_frame()
            .unwrap_or(Duration::from_millis(100));
        if !tty::poll_event(poll_timeout)? {
            // No keyboard event - check for file changes (unless suppressed after internal save)
            if app.suppress_file_watch {
                // Clear suppression and drain any pending file events
                app.suppress_file_watch = false;
                if let Some(ref mut watcher) = file_watcher {
                    watcher.check_for_changes(); // Drain events, ignore result
                }
            } else if let Some(ref mut watcher) = file_watcher
                && watcher.check_for_changes()
            {
                // Save state before reload
                let was_interactive = app.mode == app::AppMode::Interactive;
                let saved_scroll = app.content_scroll;
                let saved_element_idx = app.interactive_state.current_index;

                // File changed externally - reload with state preservation
                if let Err(e) = app.reload_current_file() {
                    app.status_message = Some(format!("✗ Reload failed: {}", e));
                } else {
                    // Re-index interactive elements if in interactive mode
                    if was_interactive {
                        app.reindex_interactive_elements();
                        // Restore element selection if still valid
                        if let Some(idx) = saved_element_idx
                            && idx < app.interactive_state.elements.len()
                        {
                            app.interactive_state.current_index = Some(idx);
                        }
                    }
                    // Restore scroll position
                    app.content_scroll = saved_scroll.min(app.content_height.saturating_sub(1));
                    app.content_scroll_state = app
                        .content_scroll_state
                        .position(app.content_scroll as usize);
                    // Sync previous_selection to prevent update_content_metrics() from resetting scroll
                    app.sync_previous_selection();

                    app.status_message = Some("↻ File reloaded (external change)".to_string());
                }
            }
            continue;
        }

        if let Event::Key(key) = tty::read_event()?
            && key.kind == KeyEventKind::Press
        {
            // When image modal is open, handle modal-specific keys
            if app.is_image_modal_open() {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        app.close_image_modal();
                    }
                    // Manual frame stepping for GIFs
                    KeyCode::Left | KeyCode::Char('h') => {
                        app.modal_prev_frame();
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        app.modal_next_frame();
                    }
                    // Toggle play/pause for GIF animation
                    KeyCode::Char(' ') => {
                        app.modal_toggle_animation();
                    }
                    _ => {}
                }
                continue;
            }

            // Handle text input modes separately - these need raw character input
            let handled = handle_text_input(&mut app, key.code, key.modifiers);

            if !handled {
                // Handle vim-style count prefix (digits before motion commands)
                // Only in modes where count makes sense (Normal, Interactive)
                // Skip in LinkFollow mode where 1-9 jump to links
                let digit_handled = if let KeyCode::Char(c) = key.code {
                    if c.is_ascii_digit()
                        && key.modifiers.is_empty()
                        && matches!(app.mode, app::AppMode::Normal | app::AppMode::Interactive)
                    {
                        // Special case: '0' without existing count goes to start (like vim)
                        if c == '0' && !app.has_count() {
                            false // Let '0' be handled as a motion (go to first)
                        } else {
                            app.accumulate_count_digit(c)
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if !digit_handled {
                    // Try to get an action from the keybinding system
                    if let Some(action) = app.get_action_for_key(key.code, key.modifiers) {
                        // Special handling for CommandPalette confirm - it may return Quit
                        if action == Action::ConfirmAction
                            && app.mode == app::AppMode::CommandPalette
                        {
                            if app.execute_selected_command() {
                                return Ok(()); // Quit command executed
                            }
                        } else {
                            match app.execute_action(action) {
                                ActionResult::Quit => return Ok(()),
                                ActionResult::RunEditor(path, line) => {
                                    let editor_config = app.editor_config();
                                    match run_editor(terminal, &path, line, &editor_config) {
                                        Ok(_) => {
                                            if let Err(e) = app.reload_current_file() {
                                                app.status_message =
                                                    Some(format!("✗ Failed to reload: {}", e));
                                            } else {
                                                app.status_message = Some(
                                                    "✓ File reloaded after editing".to_string(),
                                                );
                                            }
                                            app.update_content_metrics();
                                        }
                                        Err(e) => {
                                            app.status_message =
                                                Some(format!("✗ Editor failed: {}", e));
                                        }
                                    }
                                }
                                ActionResult::Continue => {}
                            }
                        }
                    } else {
                        // No action found - clear count prefix (invalid key cancels count)
                        app.clear_count();
                    }
                }
            }
        }
    }
}

/// Handle text input for search/edit modes
/// Returns true if the key was handled
fn handle_text_input(
    app: &mut App,
    code: KeyCode,
    modifiers: crossterm::event::KeyModifiers,
) -> bool {
    // Text input modes: outline search, doc search, link search, command palette, cell edit

    // Outline search mode - only handle input when active
    if app.show_search && app.outline_search_active {
        match code {
            KeyCode::Char('u') if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                app.search_query.clear();
                app.filter_outline();
                return true;
            }
            KeyCode::Char(c) => {
                app.search_input(c);
                return true;
            }
            _ => {}
        }
    }

    // Doc search input mode
    if app.mode == app::AppMode::DocSearch && app.doc_search_active {
        match code {
            KeyCode::Char('u') if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                app.doc_search_query.clear();
                app.update_doc_search_matches();
                return true;
            }
            KeyCode::Char(c) => {
                app.doc_search_input(c);
                return true;
            }
            _ => {}
        }
    }

    // Link search input mode
    if app.mode == app::AppMode::LinkFollow
        && app.link_search_active
        && let KeyCode::Char(c) = code
    {
        app.link_search_push(c);
        return true;
    }

    // File search input mode (FilePicker with file_search_active flag)
    if ((app.mode == app::AppMode::FilePicker && app.file_search_active)
        || app.mode == app::AppMode::FileSearch)
        && let KeyCode::Char(c) = code
    {
        app.file_search_push(c);
        return true;
    }

    // Command palette input mode
    if app.mode == app::AppMode::CommandPalette
        && let KeyCode::Char(c) = code
    {
        app.command_palette_input(c);
        return true;
    }

    // Cell edit mode
    if app.mode == app::AppMode::CellEdit
        && let KeyCode::Char(c) = code
    {
        app.cell_edit_value.push(c);
        return true;
    }

    false
}
