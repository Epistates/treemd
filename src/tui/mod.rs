mod app;
mod edits;
mod help_text;
mod image_cache;
mod interactive;
mod kitty_animation;
#[cfg(all(feature = "mermaid", unix))]
mod mermaid;
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
use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};
use crossterm::terminal::{
    BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen,
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
    mouse_captured: bool,
) -> Result<()> {
    use crossterm::event::{DisableMouseCapture, EnableMouseCapture};

    // Leave alternate screen, disable raw mode and release the mouse so the
    // editor gets full terminal control (a child editor inheriting
    // mouse-reporting mode behaves erratically)
    stdout().execute(LeaveAlternateScreen)?;
    if mouse_captured {
        let _ = stdout().execute(DisableMouseCapture);
    }
    tty::suspend_raw_mode()?;

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
    if mouse_captured {
        let _ = stdout().execute(EnableMouseCapture);
    }
    tty::resume_raw_mode()?;
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

    // Create file watcher for live reload
    let mut file_watcher = watcher::FileWatcher::new().ok();
    if let Some(ref mut watcher) = file_watcher {
        let _ = watcher.watch(&app.current_file_path);
    }

    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            // Use synchronized output when animating GIFs (reduces flicker).
            let use_sync = app.is_image_modal_open()
                && app.image_modal.gif_frames.len() > 1
                && !app.has_kitty_animation();

            if use_sync {
                let _ = stdout().execute(BeginSynchronizedUpdate);
            }

            terminal.draw(|frame| ui::render(frame, &mut app))?;

            if use_sync {
                let _ = stdout().execute(EndSynchronizedUpdate);
            }

            // If mermaid image dims arrived during this frame, schedule an immediate
            // second draw so the corrected placeholder sizes take effect without
            // requiring a keypress.
            #[cfg(all(feature = "mermaid", unix))]
            if app.mermaid_needs_reindex {
                needs_redraw = true;
                continue;
            }

            needs_redraw = false;
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
            match run_editor(
                terminal,
                &file_path,
                None,
                &editor_config,
                app.mouse_capture,
            ) {
                Ok(_) => {
                    app.status_message = Some(format!("✓ Opened {} in editor", filename));
                }
                Err(e) => {
                    app.status_message = Some(format!("✗ Failed to open {}: {}", filename, e));
                }
            }
            needs_redraw = true;
            continue; // Redraw after returning from editor
        }

        // Poll for events with dynamic timeout:
        // - When GIF is animating: use time until next frame (for smooth playback)
        // - Otherwise: 100ms for responsive UI updates
        let poll_timeout = app
            .time_until_next_frame()
            .unwrap_or(Duration::from_millis(100));
        let event_ready = tty::poll_event(poll_timeout)?;

        // Check the file watcher every iteration, regardless of whether there
        // was a keyboard event. Otherwise a debounced reload could stall
        // behind a stream of keypresses indefinitely.
        if app.suppress_file_watch {
            app.suppress_file_watch = false;
            if let Some(ref mut watcher) = file_watcher {
                watcher.check_for_changes(); // Drain events, ignore result
            }
        } else if let Some(ref mut watcher) = file_watcher
            && watcher.check_for_changes()
        {
            if app.has_unsaved_changes {
                // Never clobber buffered edits with an external reload; the
                // user must save or undo first, then the file can be reloaded.
                app.status_message = Some(
                    "⚠ File changed on disk — reload skipped to protect unsaved edits (save or undo first)"
                        .to_string(),
                );
                needs_redraw = true;
            } else {
                // Save state before reload
                let was_interactive = app.mode == app::AppMode::Interactive;
                let saved_scroll = app.content_scroll;
                let saved_element_idx = app.interactive_state.current_index;

                // File changed externally - reload with state preservation
                match app.reload_current_file() {
                    Err(e) => {
                        app.status_message = Some(format!("✗ Reload failed: {}", e));
                        needs_redraw = true;
                    }
                    // Disk content matches what we have (e.g. the echo of our
                    // own save arriving after the suppression window) — ignore.
                    Ok(false) => {}
                    Ok(true) => {
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
                        app.content_scroll = saved_scroll.min(app.max_content_scroll());
                        app.content_scroll_state = app
                            .content_scroll_state
                            .position(app.content_scroll as usize);
                        // Sync previous_selection to prevent update_content_metrics() from resetting scroll
                        app.sync_previous_selection();

                        app.status_message = Some("↻ File reloaded (external change)".to_string());
                        needs_redraw = true;
                    }
                }
            }
        }

        if !event_ready {
            // GIF animation needs redraw on timeout
            if app.is_image_modal_open() && app.image_modal.gif_frames.len() > 1 {
                needs_redraw = true;
            }
            continue;
        }

        // Any input event requires a redraw
        needs_redraw = true;

        let event = tty::read_event()?;

        // Mouse handling — keep simple: scroll wheel scrolls content / outline.
        if let Some(mouse) = event.as_mouse_event() {
            handle_mouse(&mut app, mouse);
            continue;
        }

        if let Some(key) = event.as_key_press_event() {
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

            // A sticky message (error set without a timestamp) stays visible
            // until the user presses a key; dismiss it now. Track the message
            // state so anything set directly during this keypress is marked
            // sticky for the next one.
            app.dismiss_sticky_status_message();
            let prev_status = app.status_message.clone();
            let prev_status_time = app.status_message_time;

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
                        {
                            // CommandPalette confirm routes through the palette
                            // executor; everything else through the action dispatch.
                            // Both produce an ActionResult handled uniformly below.
                            let result = if action == Action::ConfirmAction
                                && app.mode == app::AppMode::CommandPalette
                            {
                                app.execute_selected_command()
                            } else {
                                app.execute_action(action)
                            };
                            match result {
                                ActionResult::Quit => return Ok(()),
                                ActionResult::RunEditor(path, line) => {
                                    let editor_config = app.editor_config();
                                    match run_editor(
                                        terminal,
                                        &path,
                                        line,
                                        &editor_config,
                                        app.mouse_capture,
                                    ) {
                                        Ok(_) => {
                                            // The user deliberately edited the file externally,
                                            // so the on-disk version wins over any buffered edits.
                                            let had_pending = app.has_unsaved_changes;
                                            match app.reload_current_file() {
                                                Err(e) => {
                                                    app.status_message =
                                                        Some(format!("✗ Failed to reload: {}", e));
                                                }
                                                Ok(reloaded) => {
                                                    if reloaded && had_pending {
                                                        app.discard_pending_edits();
                                                        app.status_message = Some(
                                                            "✓ File reloaded after editing (buffered edits discarded)"
                                                                .to_string(),
                                                        );
                                                    } else if reloaded {
                                                        app.status_message = Some(
                                                            "✓ File reloaded after editing"
                                                                .to_string(),
                                                        );
                                                    }
                                                }
                                            }
                                            app.update_content_metrics();
                                        }
                                        Err(e) => {
                                            app.status_message =
                                                Some(format!("✗ Editor failed: {}", e));
                                        }
                                    }
                                }
                                ActionResult::Redraw => {
                                    terminal.clear()?;
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

            // Anything assigned directly to status_message during this
            // keypress (without refreshing the timestamp) is treated as
            // sticky: it stays visible until the next keypress.
            if app.status_message != prev_status && app.status_message_time == prev_status_time {
                app.status_message_time = None;
            }
        }
    }
}

/// Handle a mouse event. Currently scroll-only — pointer-click routing into
/// outline/content panes would require knowing the laid-out rects, which the
/// renderer hasn't published yet.
fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    use crate::keybindings::Action;

    // Scroll wheel maps to existing actions so keybindings stay authoritative.
    let action = match mouse.kind {
        MouseEventKind::ScrollDown => Some(Action::Next),
        MouseEventKind::ScrollUp => Some(Action::Previous),
        _ => None,
    };

    if let Some(a) = action {
        // Bypass quit/editor/redraw return paths — scroll never produces them.
        let _ = app.execute_action(a);
    }
}

/// Handle text input for search/edit modes.
/// Returns true if the key was handled by an active input field.
///
/// All six input fields share the same editing behavior: plain characters
/// insert (Shift allowed for capitals), Ctrl+U clears, Ctrl+W deletes the
/// trailing word, and any other modifier chord falls through to the
/// keybinding system instead of inserting a literal character.
fn handle_text_input(
    app: &mut App,
    code: KeyCode,
    modifiers: crossterm::event::KeyModifiers,
) -> bool {
    use crate::tui::app::TextInputEdit;
    use crossterm::event::KeyModifiers;

    let chorded =
        modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER);

    let edit = match code {
        KeyCode::Char('u') if modifiers == KeyModifiers::CONTROL => TextInputEdit::Clear,
        KeyCode::Char('w') if modifiers == KeyModifiers::CONTROL => TextInputEdit::DeleteWord,
        KeyCode::Char(c) if !chorded => TextInputEdit::Insert(c),
        _ => return false,
    };

    app.apply_text_input_edit(edit)
}
