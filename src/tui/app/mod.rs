use crate::config::Config;
use crate::keybindings::{Action, KeybindingMode, Keybindings};
use crate::parser::{Document, HeadingNode, Link, extract_links};
use crate::tui::image_cache::{
    AsyncProtocol, DecodedStill, FileStamp, ImageWorker, StreamingGif, file_stamp,
};
use crate::tui::interactive::{ElementType, InteractiveState};
use crate::tui::iterm_animation;
use crate::tui::syntax::{CodeBlockBackground, SyntaxHighlighter};
use crate::tui::terminal_compat::ColorMode;
use crate::tui::theme::{Theme, ThemeName};
use crossterm::event::{KeyCode, KeyModifiers};
use indexmap::IndexMap;
use ratatui::layout::Rect;
use ratatui::widgets::{ListState, ScrollbarState};
#[cfg(all(feature = "mermaid", unix))]
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod actions;
mod editing;
mod elements;
mod file_picker;
mod images;
mod navigation;
mod outline;
mod palette;
mod search;

pub use editing::PendingEdit;
pub use file_picker::FilePickerState;
pub use images::ImageModalState;
pub(crate) use images::ItermAnimationSurface;
use images::{ItermAnimationPlacement, ItermAnimationTarget};
pub use navigation::{FileState, LinkPickerState};
use navigation::{is_openable_url, link_root_for};
pub use palette::{CommandPaletteState, PALETTE_COMMANDS};
pub use search::DocSearchState;

/// Which heading this is, in a way that survives a reload: its text, and how
/// many headings before it have the same text.
///
/// By text alone, headings that share a name were one heading: collapsing
/// one `### Added` in a changelog collapsed every one, the open-todo filter
/// kept them all, and a bookmark marked them all and went to the first.
type HeadingKey = (String, usize);

/// The [`HeadingKey`] of every heading, in heading order.
fn heading_keys(document: &Document) -> Vec<HeadingKey> {
    let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    document
        .headings
        .iter()
        .map(|heading| {
            let count = seen.entry(heading.text.as_str()).or_insert(0);
            let key = (heading.text.clone(), *count);
            *count += 1;
            key
        })
        .collect()
}

/// Special marker for the document overview entry (shows entire file content)
pub const DOCUMENT_OVERVIEW: &str = "(Document)";

/// Result of executing an action
#[derive(Debug)]
pub enum ActionResult {
    /// Continue the main loop
    Continue,
    /// Exit the application
    Quit,
    /// Run an editor on a file, optionally at a specific line
    RunEditor(PathBuf, Option<u32>),
    /// Clear and redraw the whole screen
    Redraw,
    /// Hand the terminal back to the shell until resumed
    Suspend,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Outline,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    Interactive,
    LinkFollow,
    CellEdit,
    ConfirmFileCreate,
    DocSearch,             // In-document search mode (n/N navigation)
    CommandPalette,        // Fuzzy-searchable command palette
    ConfirmSaveWidth,      // Modal confirmation for saving outline width
    ConfirmSaveBeforeQuit, // Prompt to save unsaved changes before quitting
    ConfirmSaveBeforeNav,  // Prompt to save unsaved changes before navigating
    FilePicker,            // File picker modal for switching files
}

/// Type of pending navigation when user has unsaved changes
#[derive(Debug, Clone)]
pub enum PendingNavigation {
    /// Navigate back in file history
    Back,
    /// Navigate forward in file history
    Forward,
    /// Load a file (relative path, optional anchor)
    LoadFile(PathBuf, Option<String>),
    /// Open a file chosen in the file picker (absolute path)
    OpenPicked(PathBuf),
}

/// Fingerprint of a file's bytes, compared within one session only, so
/// `DefaultHasher` being unstable across Rust releases does not matter.
fn content_hash(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// A high-level edit applied to whichever text-input field is active.
/// Keys are translated to edits in one place so every input field gets the
/// same editing behavior (and modifier chords never insert literal chars).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputEdit {
    Insert(char),
    /// Ctrl+U — clear the whole input
    Clear,
    /// Ctrl+W — delete the trailing word
    DeleteWord,
}

pub struct App {
    pub document: Document,
    pub filename: String,
    pub tree: Vec<HeadingNode>,
    pub outline_state: ListState,
    pub outline_scroll_state: ScrollbarState,
    pub focus: Focus,
    pub outline_items: Vec<OutlineItem>,
    pub content_scroll: usize,
    pub content_scroll_state: ScrollbarState,
    pub content_height: usize,
    pub content_viewport_height: u16, // Actual viewport height for scroll calculations
    pub show_help: bool,
    pub help_scroll: u16,
    /// The help popup's last scroll position and page height, in rows, as
    /// laid out by the frame that drew it.
    pub(crate) help_max_scroll: u16,
    pub(crate) help_page: u16,
    pub show_search: bool,
    pub outline_search_active: bool, // Whether search input is active (cursor visible)
    pub search_query: String,
    pub highlighter: SyntaxHighlighter,
    pub show_outline: bool,
    pub show_heading_markers: bool, // Show # prefixes in outline sidebar
    pub code_fences: crate::config::CodeFences, // How to delimit fenced code blocks
    /// Whether terminal mouse capture is active. When on, the scroll wheel drives
    /// navigation but the terminal's native click-drag text selection is disabled.
    /// Toggling it off hands the mouse back to the terminal so text can be selected
    /// and copied. Enabled at startup to match `main.rs`'s best-effort capture.
    pub mouse_capture: bool,
    pub outline_width: u16, // Percentage: 20, 30, or 40
    /// Whether the config file had a custom (non-standard) outline width at startup.
    /// Used to protect power users' custom config values from being overwritten.
    /// Standard values are 20, 30, 40; anything else is considered custom.
    config_has_custom_outline_width: bool,
    /// The bookmarked heading (or the document overview, by its text).
    pub bookmark_position: Option<HeadingKey>,
    /// The headings whose children are hidden in the outline.
    collapsed_headings: HashSet<HeadingKey>,
    pub filter_by_todos: bool, // Filter outline to show only headings with open todos
    pub current_theme: ThemeName,
    pub theme: Theme,
    pub show_theme_picker: bool,
    pub theme_picker_selected: usize,
    pub theme_picker_original: Option<ThemeName>, // Original theme before picker opened (for cancel)
    previous_selection: Option<Option<usize>>,    // Heading index for change detection
    /// True when the cached content_height/scrollbar may be stale and need
    /// recomputation on the next `update_content_metrics`. Set by file
    /// reload, raw-source toggle, etc.
    metrics_dirty: bool,

    // Link following state
    pub mode: AppMode,
    /// Vim-style count prefix for motion commands (e.g., 5j moves down 5)
    pub count_prefix: Option<usize>,
    pub current_file_path: PathBuf, // Path to current file for resolving relative links
    pub file_path_changed: bool,    // Flag to signal file watcher needs update
    pub suppress_file_watch: bool,  // Skip next file watch check (after internal save)
    /// The displayed text, rendered lines and wrapped rows of the content
    /// pane, each kept until something it was built from changes.
    pub(crate) content_cache: crate::tui::content_view::ContentCache,
    /// The layout of the last frame's content pane, where every element,
    /// image and search match is found on screen.
    pub(crate) content_view: std::sync::Arc<crate::tui::content_view::ContentView>,
    /// Set when the selected element should be scrolled into view once the
    /// frame being drawn has laid the section out.
    pending_element_scroll: bool,
    /// Hash of the file as treemd last read or wrote it. A save checks the
    /// disk against this first, so an edit made elsewhere since then is not
    /// silently overwritten by the in-memory copy.
    disk_hash: u64,
    /// Where links may lead: the document's git work tree, or the directory
    /// treemd was opened on. Set when a document is opened deliberately (at
    /// startup or from the file picker), not when a link is followed.
    link_root: PathBuf,
    pub links_in_view: Vec<Link>, // Links in currently displayed content
    pub link_picker: LinkPickerState,

    // File picker state
    pub file_picker: FilePickerState,
    pub startup_needs_file_picker: bool, // True if started without file arg
    pub file_picker_dir: Option<PathBuf>, // Custom directory for file picker
    pub show_hidden: bool,               // Whether to show hidden (dot) files and directories

    pub file_history: Vec<FileState>,   // Back navigation stack
    pub file_future: Vec<FileState>,    // Forward navigation stack (for undo back)
    pub status_message: Option<String>, // Temporary status message to display
    pub status_message_time: Option<Instant>, // When the status message was set

    // Interactive element navigation
    pub interactive_state: InteractiveState,

    // Cell editing state
    pub cell_edit_value: String,          // Current value being edited
    pub cell_edit_original_value: String, // Original value before editing (for undo)
    pub cell_edit_row: usize,             // Row being edited
    pub cell_edit_col: usize,             // Column being edited
    /// The cell exactly as written between its pipes, which undo restores.
    cell_edit_original_source: String,

    // Pending edits buffer (for safe editing with explicit save)
    pub pending_edits: Vec<PendingEdit>, // Stack of uncommitted edits

    // Persistent clipboard for Linux X11 compatibility
    // On Linux, the clipboard instance must stay alive to serve paste requests
    clipboard: Option<arboard::Clipboard>,

    // Configuration persistence
    config: Config,
    color_mode: ColorMode,

    // Pending file to open in external editor (set by link following, consumed by main loop)
    pub pending_editor_file: Option<PathBuf>,

    // Raw source view toggle
    pub show_raw_source: bool,

    // Pending file creation (for confirm dialog)
    pub pending_file_create: Option<PathBuf>,
    pub pending_file_create_message: Option<String>,

    /// In-document search (/ + n/N).
    pub doc_search: DocSearchState,

    /// Command palette (`:`).
    pub command_palette: CommandPaletteState,

    // Customizable keybindings
    pub keybindings: Keybindings,

    // Pending navigation (for confirm save dialog when navigating with unsaved changes)
    pub pending_navigation: Option<PendingNavigation>,

    // Terminal graphics protocol picker (with fallback font size)
    pub picker: Option<ratatui_image::picker::Picker>,

    // Shared resize/encode worker. StatefulImage rendering stays non-blocking.
    image_worker: Option<ImageWorker>,

    // Cached image protocols keyed by resolved path (avoids re-decoding from disk every frame)
    /// Decoded image protocols. Bounded LRU cache: access-ordered, with
    /// the oldest entries evicted when capacity is reached. Keeping recently
    /// seen images survives navigation between sections so the user doesn't
    /// pay the decode cost on every back-and-forth.
    pub(crate) image_protocol_cache: IndexMap<PathBuf, AsyncProtocol>,
    /// The file each cached image was decoded from, as it was then, so a
    /// replaced image is decoded again rather than shown as it used to be.
    image_stamps: std::collections::HashMap<PathBuf, FileStamp>,
    /// Images that failed to decode, and the file as it was then, so a bad
    /// image is not decoded again on every section change.
    image_failures: std::collections::HashMap<PathBuf, FileStamp>,
    /// Images queued on the worker to decode, and the file as it was when
    /// asked, so each is asked for once.
    pub(crate) image_decoding: std::collections::HashMap<PathBuf, FileStamp>,
    /// The images the current section shows, which eviction leaves alone.
    image_section: HashSet<PathBuf>,

    // Streaming inline GIF state, keyed with the corresponding protocol cache.
    // Unlike the modal viewer this retains only the compositing canvas, keeping
    // long README recordings memory-bounded.

    // Image modal viewing state (path, current frame, GIF playback).
    pub image_modal: ImageModalState,

    // Native iTerm2 GIF playback. The original file is transferred after the
    // Ratatui frame is flushed, so no large OSC payload enters a buffer cell.
    iterm_animation_target: Option<ItermAnimationTarget>,
    iterm_animation_staged: Option<ItermAnimationPlacement>,
    iterm_animation_sent: Option<ItermAnimationPlacement>,
    iterm_animation_failed: bool,

    // Image rendering control (can be disabled via config or CLI)
    pub images_enabled: bool,

    // Mermaid diagram rendering cache: source hash → rendered protocol
    #[cfg(all(feature = "mermaid", unix))]
    pub(crate) mermaid_protocol_cache: IndexMap<u64, AsyncProtocol>,
    #[cfg(all(feature = "mermaid", unix))]
    pub mermaid_render_errors: HashMap<u64, String>,
    /// Lays diagrams out off the UI thread; started by the first diagram.
    #[cfg(all(feature = "mermaid", unix))]
    mermaid_worker: Option<crate::tui::mermaid::MermaidWorker>,
    /// Diagrams queued on the worker for the current width, by source hash.
    #[cfg(all(feature = "mermaid", unix))]
    pub(crate) mermaid_pending: HashSet<u64>,
    #[cfg(all(feature = "mermaid", unix))]
    pub mermaid_last_render_width: u32,
    /// Pixel dimensions (width, height) of each rendered mermaid image, keyed by source hash.
    #[cfg(all(feature = "mermaid", unix))]
    pub mermaid_image_dims: HashMap<u64, (u32, u32)>,
    /// Terminal row count for each rendered mermaid image, derived from pixel height / font height.
    #[cfg(all(feature = "mermaid", unix))]
    pub mermaid_placeholder_rows: HashMap<u64, usize>,
    /// Set when new mermaid dims are stored; triggers a re-index on the next frame.
    #[cfg(all(feature = "mermaid", unix))]
    pub mermaid_needs_reindex: bool,

    // LaTeX detection state
    pub latex_detected: bool,
    pub latex_hint_shown: bool,
}

#[derive(Debug, Clone)]
pub struct OutlineItem {
    pub level: usize,
    pub text: String,
    pub expanded: bool,
    pub has_children: bool, // Track if this heading has children in the tree
    pub heading_index: Option<usize>, // Index into Document::headings
}

impl App {
    pub fn new(
        document: Document,
        filename: String,
        file_path: PathBuf,
        config: Config,
        color_mode: ColorMode,
        images_enabled: bool,
    ) -> Self {
        let tree = document.build_tree();
        let collapsed_headings = HashSet::new();
        let mut outline_items =
            Self::flatten_tree(&tree, &collapsed_headings, &heading_keys(&document));

        // Add document overview entry if there's preamble content or no headings
        let has_preamble = Self::has_preamble_content(&document);
        if has_preamble || document.headings.is_empty() {
            outline_items.insert(
                0,
                OutlineItem {
                    level: 0, // Level 0 for document overview (renders without # prefix)
                    text: DOCUMENT_OVERVIEW.to_string(),
                    expanded: true,
                    has_children: !outline_items.is_empty(),
                    heading_index: None,
                },
            );
        }

        let mut outline_state = ListState::default();
        if !outline_items.is_empty() {
            outline_state.select(Some(0));
        }

        let content_lines = document.content.lines().count();

        // Load theme from config, apply color mode, then apply custom colors
        let current_theme = config.theme_name();
        let theme = Theme::from_name(current_theme)
            .with_color_mode(color_mode, current_theme)
            .with_custom_colors(&config.theme, color_mode);

        // Load sublime color scheme directory
        let code_theme_dir = config.code_theme_dir_path();
        // Load sublime color scheme name (for code highlighting)
        let code_theme = config.ui.code_theme.as_str();
        // Code block background: unset falls back to the code theme's own, and
        // the literal "none" turns it off for anyone who prefers code to sit on
        // the terminal background. Quantizing for 256-color terminals is left
        // to the highlighter, which does the same for every token color.
        let code_block_bg = match config.theme.code_block_bg.as_ref() {
            None => CodeBlockBackground::FromTheme,
            Some(crate::config::ColorValue::Named(name)) if name.eq_ignore_ascii_case("none") => {
                CodeBlockBackground::Off
            }
            Some(value) => match value.to_color() {
                Some(c) => CodeBlockBackground::Color(c),
                None => CodeBlockBackground::FromTheme,
            },
        };

        // Load outline width from config
        let outline_width = config.ui.outline_width;

        // Detect if config has a custom (non-standard) outline width
        // Standard values: 20, 30, 40 - anything else is a custom power-user setting
        let config_has_custom_outline_width =
            outline_width != 20 && outline_width != 30 && outline_width != 40;

        // Load keybindings from config (before config is moved)
        let keybindings = config.keybindings();
        // Anything wrong with the config file. It was printed too, but before
        // the alternate screen, where nobody saw it.
        let config_warning = config
            .warnings
            .first()
            .map(|first| match config.warnings.len() {
                1 => format!("⚠ config: {first}"),
                n => format!("⚠ config: {first} (and {} more)", n - 1),
            });

        // Picker detection must happen after entering the alternate screen.
        // A single worker then services every protocol without blocking draws.
        let picker = images_enabled.then(Self::init_picker).flatten();
        let image_worker = picker.as_ref().map(|_| ImageWorker::new());
        let disk_hash = content_hash(document.content.as_bytes());
        let link_root = link_root_for(&file_path, config.security.trust_piped_documents);
        let latex_detected = crate::tui::ui::util::contains_latex(&document.content);

        Self {
            document,
            filename,
            tree,
            outline_state,
            outline_scroll_state: ScrollbarState::new(outline_items.len()),
            focus: Focus::Outline,
            outline_items,
            content_scroll: 0,
            content_scroll_state: ScrollbarState::new(content_lines),
            content_height: content_lines,
            content_viewport_height: 20, // Default, will be updated by UI on first render
            show_help: false,
            help_scroll: 0,
            help_max_scroll: u16::MAX,
            help_page: 1,
            show_search: false,
            outline_search_active: false,
            search_query: String::new(),
            highlighter: SyntaxHighlighter::new(
                code_theme,
                code_theme_dir,
                code_block_bg,
                color_mode,
            ),
            show_outline: true,
            show_heading_markers: config.ui.outline_heading_markers,
            code_fences: config.content.code_fences,
            mouse_capture: true,
            outline_width,
            config_has_custom_outline_width,
            bookmark_position: None,
            collapsed_headings,
            filter_by_todos: false,
            current_theme,
            theme,
            show_theme_picker: false,
            theme_picker_selected: 0,
            theme_picker_original: None,
            previous_selection: None,
            metrics_dirty: true,

            // Link following state
            mode: AppMode::Normal,
            count_prefix: None,
            current_file_path: file_path,
            file_path_changed: false,
            suppress_file_watch: false,
            content_cache: Default::default(),
            content_view: Default::default(),
            pending_element_scroll: false,
            disk_hash,
            link_root,
            links_in_view: Vec::new(),
            link_picker: LinkPickerState::default(),

            // File picker state
            file_picker: FilePickerState::default(),
            startup_needs_file_picker: false,
            file_picker_dir: None,
            show_hidden: false,

            file_history: Vec::new(),
            file_future: Vec::new(),
            status_message: config_warning,
            status_message_time: None,

            // Interactive element navigation
            interactive_state: InteractiveState::new(),

            // Cell editing state
            cell_edit_value: String::new(),
            cell_edit_original_value: String::new(),
            cell_edit_original_source: String::new(),
            cell_edit_row: 0,
            cell_edit_col: 0,

            // Pending edits buffer
            pending_edits: Vec::new(),

            // Initialize persistent clipboard (None if unavailable)
            clipboard: arboard::Clipboard::new().ok(),

            // Configuration persistence
            config,
            color_mode,

            // Pending editor file
            pending_editor_file: None,

            // Raw source view (off by default)
            show_raw_source: false,

            // Pending file creation (for confirm dialog)
            pending_file_create: None,
            pending_file_create_message: None,

            // Document search state
            doc_search: DocSearchState::default(),

            // Command palette state
            command_palette: CommandPaletteState::default(),

            // Customizable keybindings (loaded from config)
            // Note: keybindings() called before config is moved into struct
            keybindings,

            // Pending navigation (for confirm save dialog)
            pending_navigation: None,

            // Terminal graphics protocol picker with fallback (like figif)
            // Only initialize if images are enabled
            picker,

            image_worker,

            // Cached image protocols (populated after index_elements)
            image_protocol_cache: IndexMap::new(),
            image_stamps: std::collections::HashMap::new(),
            image_failures: std::collections::HashMap::new(),
            image_decoding: std::collections::HashMap::new(),
            image_section: HashSet::new(),

            // Image modal viewing state (path, GIF playback, etc.)
            image_modal: ImageModalState::default(),

            iterm_animation_target: None,
            iterm_animation_staged: None,
            iterm_animation_sent: None,
            iterm_animation_failed: false,

            // Image rendering control
            images_enabled,

            // Mermaid diagram cache
            #[cfg(all(feature = "mermaid", unix))]
            mermaid_protocol_cache: IndexMap::new(),
            #[cfg(all(feature = "mermaid", unix))]
            mermaid_render_errors: HashMap::new(),
            #[cfg(all(feature = "mermaid", unix))]
            mermaid_worker: None,
            #[cfg(all(feature = "mermaid", unix))]
            mermaid_pending: HashSet::new(),
            #[cfg(all(feature = "mermaid", unix))]
            mermaid_last_render_width: 0,
            #[cfg(all(feature = "mermaid", unix))]
            mermaid_image_dims: HashMap::new(),
            #[cfg(all(feature = "mermaid", unix))]
            mermaid_placeholder_rows: HashMap::new(),
            #[cfg(all(feature = "mermaid", unix))]
            mermaid_needs_reindex: false,

            // LaTeX detection
            latex_detected,
            latex_hint_shown: false,
        }
    }

    /// Update the content viewport height (called by UI when terminal size is known)
    pub fn set_viewport_height(&mut self, height: u16) {
        self.content_viewport_height = height.max(1); // Ensure at least 1 to avoid divide-by-zero
    }

    /// Maximum scroll offset: stops when last line is at bottom of viewport
    pub fn max_content_scroll(&self) -> usize {
        self.content_height
            .saturating_sub(self.content_viewport_height as usize)
    }

    /// Scroll content down by one line
    fn scroll_content_down(&mut self) {
        let max_scroll = self.max_content_scroll();
        let new_scroll = self.content_scroll.saturating_add(1);
        if new_scroll <= max_scroll {
            self.content_scroll = new_scroll;
            self.content_scroll_state = self.content_scroll_state.position(new_scroll);
        }
    }

    /// Scroll content up by one line
    fn scroll_content_up(&mut self) {
        let new_scroll = self.content_scroll.saturating_sub(1);
        self.content_scroll = new_scroll;
        self.content_scroll_state = self.content_scroll_state.position(new_scroll);
    }

    /// Toggle between raw source view and rendered markdown view
    pub fn toggle_raw_source(&mut self) {
        self.show_raw_source = !self.show_raw_source;
        // Raw vs rendered changes the line count of the displayed content.
        self.metrics_dirty = true;
        let msg = if self.show_raw_source {
            "Raw source view enabled"
        } else {
            "Rendered view enabled"
        };
        self.set_status_message(msg);
    }

    /// Set an informational status message that auto-expires after a few
    /// seconds. Messages assigned directly to `status_message` (without a
    /// timestamp) are treated as sticky and dismissed on the next keypress.
    pub fn set_status_message(&mut self, msg: &str) {
        self.status_message = Some(msg.to_string());
        self.status_message_time = Some(Instant::now());
    }

    /// How long a timed status message has left, zero once it has expired,
    /// or `None` when there is no timed message.
    pub fn status_message_expires_in(&self) -> Option<Duration> {
        const STATUS_MESSAGE_TIMEOUT: Duration = Duration::from_secs(3);

        self.status_message.as_ref()?;
        self.status_message_time
            .map(|time| STATUS_MESSAGE_TIMEOUT.saturating_sub(time.elapsed()))
    }

    /// Clear status message if it has expired
    pub fn clear_expired_status_message(&mut self) {
        if self.status_message_expires_in() == Some(Duration::ZERO) {
            self.status_message = None;
            self.status_message_time = None;
        }
    }

    /// Dismiss a sticky status message (one set without a timestamp, e.g.
    /// errors). Called on the next keypress so errors stay visible until the
    /// user acts, while timed info messages expire on their own.
    pub fn dismiss_sticky_status_message(&mut self) {
        if self.status_message.is_some() && self.status_message_time.is_none() {
            self.status_message = None;
        }
    }

    /// Delete the trailing whitespace-delimited word from a string (Ctrl+W).
    fn delete_last_word(s: &mut String) {
        while s.chars().last().is_some_and(char::is_whitespace) {
            s.pop();
        }
        while s.chars().last().is_some_and(|c| !c.is_whitespace()) {
            s.pop();
        }
    }

    /// Route a text edit to whichever input field is currently active
    /// (outline search, doc search, link search, file search, command
    /// palette, cell edit). Returns true if an active field consumed it.
    pub fn apply_text_input_edit(&mut self, edit: TextInputEdit) -> bool {
        use TextInputEdit::*;

        // The image viewer is in front of any search that opened it.
        if self.is_image_modal_open() {
            return false;
        }

        // Outline search
        if self.show_search && self.outline_search_active {
            match edit {
                Insert(c) => self.search_input(c),
                Clear => {
                    self.search_query.clear();
                    self.filter_outline();
                }
                DeleteWord => {
                    Self::delete_last_word(&mut self.search_query);
                    self.filter_outline();
                }
            }
            return true;
        }

        // Doc search
        if self.mode == AppMode::DocSearch && self.doc_search.active {
            match edit {
                Insert(c) => self.doc_search_input(c),
                Clear => {
                    self.doc_search.query.clear();
                    self.update_doc_search_matches();
                }
                DeleteWord => {
                    Self::delete_last_word(&mut self.doc_search.query);
                    self.update_doc_search_matches();
                }
            }
            return true;
        }

        // Link search
        if self.mode == AppMode::LinkFollow && self.link_picker.active {
            match edit {
                Insert(c) => self.link_search_push(c),
                Clear => {
                    self.link_picker.query.clear();
                    self.update_link_filter();
                }
                DeleteWord => {
                    Self::delete_last_word(&mut self.link_picker.query);
                    self.update_link_filter();
                }
            }
            return true;
        }

        // File search
        if self.mode == AppMode::FilePicker && self.file_picker.active {
            match edit {
                Insert(c) => self.file_search_push(c),
                Clear => {
                    self.file_picker.query.clear();
                    self.update_file_filter();
                }
                DeleteWord => {
                    Self::delete_last_word(&mut self.file_picker.query);
                    self.update_file_filter();
                }
            }
            return true;
        }

        // Command palette
        if self.mode == AppMode::CommandPalette {
            match edit {
                Insert(c) => self.command_palette_input(c),
                Clear => {
                    self.command_palette.query.clear();
                    self.filter_commands();
                }
                DeleteWord => {
                    Self::delete_last_word(&mut self.command_palette.query);
                    self.filter_commands();
                }
            }
            return true;
        }

        // Cell edit
        if self.mode == AppMode::CellEdit {
            match edit {
                Insert(c) => self.cell_edit_value.push(c),
                Clear => self.cell_edit_value.clear(),
                DeleteWord => Self::delete_last_word(&mut self.cell_edit_value),
            }
            return true;
        }

        false
    }

    /// Insert a bracketed paste into the active single-line input.
    ///
    /// Newlines and tabs become spaces and other control characters are
    /// dropped, so pasted terminal escape sequences can never become commands.
    /// Filters are recomputed once for the complete paste rather than once per
    /// character.
    pub fn apply_text_input_paste(&mut self, pasted: &str) -> bool {
        let text: String = pasted
            .chars()
            .filter_map(|character| match character {
                '\r' | '\n' | '\t' => Some(' '),
                character if character.is_control() => None,
                character => Some(character),
            })
            .collect();

        if self.show_search && self.outline_search_active {
            self.search_query.push_str(&text);
            self.filter_outline();
            return true;
        }
        if self.mode == AppMode::DocSearch && self.doc_search.active {
            self.doc_search.query.push_str(&text);
            self.update_doc_search_matches();
            return true;
        }
        if self.mode == AppMode::LinkFollow && self.link_picker.active {
            self.link_picker.query.push_str(&text);
            self.update_link_filter();
            return true;
        }
        if self.mode == AppMode::FilePicker && self.file_picker.active {
            self.file_picker.query.push_str(&text);
            self.update_file_filter();
            return true;
        }
        if self.mode == AppMode::CommandPalette {
            self.command_palette.query.push_str(&text);
            self.filter_commands();
            return true;
        }
        if self.mode == AppMode::CellEdit {
            self.cell_edit_value.push_str(&text);
            return true;
        }

        false
    }

    /// Accumulate a digit into the vim-style count prefix
    /// Returns true if the digit was handled as a count prefix
    pub fn accumulate_count_digit(&mut self, digit: char) -> bool {
        if let Some(d) = digit.to_digit(10) {
            let current = self.count_prefix.unwrap_or(0);
            // Limit to reasonable count (max 9999)
            let new_count = current
                .saturating_mul(10)
                .saturating_add(d as usize)
                .min(9999);
            self.count_prefix = Some(new_count);
            true
        } else {
            false
        }
    }

    /// Get and consume the count prefix, returning at least 1
    pub fn take_count(&mut self) -> usize {
        self.count_prefix.take().unwrap_or(1)
    }

    /// Clear the count prefix without consuming it
    pub fn clear_count(&mut self) {
        self.count_prefix = None;
    }

    /// Check if there's an active count prefix
    pub fn has_count(&self) -> bool {
        self.count_prefix.is_some()
    }

    /// Update content height based on current selection and reset scroll if selection changed.
    ///
    /// The expensive line-counting only runs when the selection or the
    /// underlying content has changed (signalled by `metrics_dirty`). The
    /// render loop calls this every frame; without gating, a large section
    /// would be re-scanned 60+ times per second during animations.
    pub fn update_content_metrics(&mut self) {
        let current_selection = self.outline_state.selected().map(|i| {
            self.outline_items
                .get(i)
                .and_then(|item| item.heading_index)
        });
        let selection_changed = current_selection != self.previous_selection;

        if selection_changed {
            // Reset content scroll when selection changes
            self.content_scroll = 0;
            self.previous_selection = current_selection;

            // Per-element state (a table's cursor, an expanded `<details>`)
            // is keyed by the element's place in its section, so another
            // section's element in the same place inherited it: a cursor on
            // a cell its table does not have, a block shown expanded.
            self.interactive_state.element_states.clear();

            // Reindex interactive elements for the new section
            let content_text = self.displayed_section_content().to_string();

            use crate::parser::content::parse_content;
            let blocks = parse_content(&content_text, 0);
            self.index_interactive_elements(&blocks);
            self.populate_image_cache();
        }

        if selection_changed || self.metrics_dirty {
            let content_text = self.current_section_content();
            let content_lines = content_text.lines().count();
            self.content_height = content_lines;
            self.content_scroll_state =
                ScrollbarState::new(content_lines).position(self.content_scroll);
            self.metrics_dirty = false;
        } else {
            // Cheap path: just keep the scrollbar position in sync with scroll.
            self.content_scroll_state = self.content_scroll_state.position(self.content_scroll);
        }
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        if self.show_help {
            self.help_scroll = 0; // Reset scroll when opening help
        }
    }

    pub fn scroll_help_down(&mut self) {
        self.help_scroll = self.help_scroll.saturating_add(1).min(self.help_max_scroll);
    }

    pub fn scroll_help_up(&mut self) {
        self.help_scroll = self.help_scroll.saturating_sub(1);
    }

    /// Scroll help popup down by a page
    pub fn scroll_help_page_down(&mut self) {
        let page_size = self.help_page.saturating_sub(1).max(1);
        self.help_scroll = self
            .help_scroll
            .saturating_add(page_size)
            .min(self.help_max_scroll);
    }

    /// Scroll help popup up by a page
    pub fn scroll_help_page_up(&mut self) {
        let page_size = self.help_page.saturating_sub(1).max(1);
        self.help_scroll = self.help_scroll.saturating_sub(page_size);
    }

    /// Maximum search query length to prevent performance issues
    const MAX_SEARCH_LEN: usize = 256;

    // ========== Document Search Methods ==========

    pub fn scroll_page_down(&mut self) {
        let page = usize::from(self.content_viewport_height.saturating_sub(2).max(1));
        if self.focus == Focus::Content {
            let new_scroll = self.content_scroll.saturating_add(page);
            self.content_scroll = new_scroll.min(self.max_content_scroll());
            self.content_scroll_state = self.content_scroll_state.position(self.content_scroll);
        } else {
            self.page_outline(page as isize);
        }
    }

    pub fn scroll_page_up(&mut self) {
        let page = usize::from(self.content_viewport_height.saturating_sub(2).max(1));
        if self.focus == Focus::Content {
            self.content_scroll = self.content_scroll.saturating_sub(page);
            self.content_scroll_state = self.content_scroll_state.position(self.content_scroll);
        } else {
            self.page_outline(-(page as isize));
        }
    }

    pub fn toggle_focus(&mut self) {
        // If in locked-in outline search state, Tab cycles to next filtered item
        if self.show_search && !self.outline_search_active && !self.outline_items.is_empty() {
            let current = self.outline_state.selected().unwrap_or(0);
            let next = (current + 1) % self.outline_items.len();
            self.select_outline_index(next);
            return;
        }

        if self.show_outline {
            self.focus = match self.focus {
                Focus::Outline => Focus::Content,
                Focus::Content => Focus::Outline,
            };
        }
    }

    /// Toggle focus backwards (Shift+Tab) - cycles to previous item when search is locked in
    pub fn toggle_focus_back(&mut self) {
        // If in locked-in outline search state, Shift+Tab cycles to previous filtered item
        if self.show_search && !self.outline_search_active && !self.outline_items.is_empty() {
            let current = self.outline_state.selected().unwrap_or(0);
            let len = self.outline_items.len();
            let prev = (current + len - 1) % len;
            self.select_outline_index(prev);
            return;
        }

        // Same as toggle_focus when not in locked search state
        if self.show_outline {
            self.focus = match self.focus {
                Focus::Outline => Focus::Content,
                Focus::Content => Focus::Outline,
            };
        }
    }

    /// Toggle terminal mouse capture.
    ///
    /// With capture on, the scroll wheel drives navigation but the terminal
    /// cannot perform native click-drag text selection. Turning it off releases
    /// the mouse to the terminal so the user can select and copy arbitrary text;
    /// turning it back on restores scroll-wheel navigation. Capture changes are
    /// best-effort — terminals that reject the command simply keep their state.
    pub fn toggle_mouse_capture(&mut self) {
        use crossterm::ExecutableCommand;
        use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
        use std::io::stdout;

        self.mouse_capture = !self.mouse_capture;
        if self.mouse_capture {
            let _ = stdout().execute(EnableMouseCapture);
            self.set_status_message("Mouse capture: ON (scroll navigation)");
        } else {
            let _ = stdout().execute(DisableMouseCapture);
            self.set_status_message("Mouse capture: OFF (drag to select text)");
        }
    }

    // ========== Command Palette ==========

    /// The current section as the content pane shows it: with frontmatter and
    /// LaTeX hidden as configured, unless the raw source is on.
    ///
    /// Everything that refers to a block by its index, the interactive
    /// index, the renderer, and the edits that act on a selected element,
    /// has to parse this same text. The index used to parse the unfiltered
    /// section while the renderer drew the filtered one, so with frontmatter
    /// hidden the pointer marked one link while Enter followed another.
    pub(crate) fn displayed_section_content(&mut self) -> std::sync::Arc<String> {
        use std::hash::{Hash, Hasher};

        let raw = self.current_section_content();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (
            &raw,
            self.show_raw_source,
            self.frontmatter_filter_active(),
            self.latex_filter_active(),
            self.should_latex_aggressive(),
        )
            .hash(&mut hasher);
        let key = hasher.finish();
        if let Some((cached, text)) = &self.content_cache.displayed
            && *cached == key
        {
            return std::sync::Arc::clone(text);
        }
        let text = std::sync::Arc::new(self.filter_for_display(raw));
        self.content_cache.displayed = Some((key, std::sync::Arc::clone(&text)));
        text
    }

    /// [`Self::displayed_section_content`] without the cache, for callers
    /// that only borrow the app.
    fn compute_displayed_section(&self) -> String {
        self.filter_for_display(self.current_section_content())
    }

    fn filter_for_display(&self, raw: String) -> String {
        if self.show_raw_source {
            raw
        } else {
            crate::tui::ui::util::filter_content(
                &raw,
                self.frontmatter_filter_active(),
                self.latex_filter_active(),
                self.should_latex_aggressive(),
            )
        }
    }

    /// Get the content for the currently selected section, or the full document if no heading is selected.
    fn current_section_content(&self) -> String {
        self.selected_heading_index()
            .and_then(|idx| self.document.extract_section_at_index(idx))
            .unwrap_or_else(|| self.document.content.clone())
    }

    /// Get the source line number (1-indexed) for the currently selected heading.
    ///
    /// Returns None if no heading is selected or if the selection is the document overview.
    pub fn selected_heading_source_line(&self) -> Option<u32> {
        let selected_text = self.selected_heading_text()?;

        // Document overview doesn't have a source line
        if selected_text == DOCUMENT_OVERVIEW {
            return Some(1); // Return line 1 for document overview
        }

        let heading = self.document.headings.get(self.selected_heading_index()?)?;
        let offset = heading.offset.min(self.document.content.len());
        let before = &self.document.content[..offset];
        let line = before.chars().filter(|&c| c == '\n').count() + 1;
        Some(line as u32)
    }

    /// The file line, from 1, that the displayed section's text starts on.
    ///
    /// Elements count lines in the displayed text, which starts at the
    /// section's first non-blank line, past hidden frontmatter in the
    /// overview. Counting from the line after the heading put the editor one
    /// line or more above the element.
    fn displayed_text_first_line(&self) -> usize {
        let content = &self.document.content;
        let start = match self.selected_heading_index() {
            Some(index) => {
                let end = self.document.section_end(index);
                let body = self.document.body_start(index).min(end);
                let text = &content[body..end];
                body + (text.len() - text.trim_start().len())
            }
            None if self.frontmatter_filter_active() => {
                crate::parser::frontmatter_range(content).map_or(0, |range| range.end)
            }
            None => 0,
        };
        1 + content[..start].matches('\n').count()
    }

    /// Sync previous_selection to current selection (prevents spurious scroll resets)
    pub fn sync_previous_selection(&mut self) {
        self.previous_selection = self.outline_state.selected().map(|i| {
            self.outline_items
                .get(i)
                .and_then(|item| item.heading_index)
        });
    }

    pub fn toggle_theme_picker(&mut self) {
        if self.show_theme_picker {
            // Closing picker - restore original theme if set (user pressed Esc)
            if let Some(original) = self.theme_picker_original.take() {
                self.apply_theme_preview(original);
            }
            self.show_theme_picker = false;
        } else {
            // Opening picker - store current theme and set selection
            self.theme_picker_original = Some(self.current_theme);
            self.theme_picker_selected = match self.current_theme {
                ThemeName::OceanDark => 0,
                ThemeName::Nord => 1,
                ThemeName::Dracula => 2,
                ThemeName::Solarized => 3,
                ThemeName::Monokai => 4,
                ThemeName::Gruvbox => 5,
                ThemeName::TokyoNight => 6,
                ThemeName::CatppuccinMocha => 7,
            };
            self.show_theme_picker = true;
        }
    }

    /// Convert theme picker selection index to ThemeName
    fn theme_name_from_index(idx: usize) -> ThemeName {
        match idx {
            0 => ThemeName::OceanDark,
            1 => ThemeName::Nord,
            2 => ThemeName::Dracula,
            3 => ThemeName::Solarized,
            4 => ThemeName::Monokai,
            5 => ThemeName::Gruvbox,
            6 => ThemeName::TokyoNight,
            7 => ThemeName::CatppuccinMocha,
            _ => ThemeName::OceanDark,
        }
    }

    /// Apply a theme preview (doesn't save to config)
    fn apply_theme_preview(&mut self, theme_name: ThemeName) {
        self.current_theme = theme_name;
        self.theme = Theme::from_name(theme_name)
            .with_color_mode(self.color_mode, theme_name)
            .with_custom_colors(&self.config.theme, self.color_mode);
    }

    pub fn theme_picker_next(&mut self) {
        if self.theme_picker_selected < 7 {
            self.theme_picker_selected += 1;
            // Apply theme preview immediately
            let theme_name = Self::theme_name_from_index(self.theme_picker_selected);
            self.apply_theme_preview(theme_name);
        }
    }

    pub fn theme_picker_previous(&mut self) {
        if self.theme_picker_selected > 0 {
            self.theme_picker_selected -= 1;
            // Apply theme preview immediately
            let theme_name = Self::theme_name_from_index(self.theme_picker_selected);
            self.apply_theme_preview(theme_name);
        }
    }

    pub fn apply_selected_theme(&mut self) {
        // Theme is already applied via preview, just save to config and close
        self.theme_picker_original = None; // Clear so toggle doesn't restore
        self.show_theme_picker = false;

        // Save to config (silently ignore errors)
        let _ = self.config.set_theme(self.current_theme);
    }

    /// Get the editor configuration for external file editing
    pub fn editor_config(&self) -> opensesame::EditorConfig {
        self.config.editor.clone()
    }

    // ===== File Picker Methods =====

    /// Check if frontmatter should be hidden (from config)
    pub fn should_hide_frontmatter(&self) -> bool {
        self.config.content.hide_frontmatter
    }

    /// Check if LaTeX should be hidden (from config)
    pub fn should_hide_latex(&self) -> bool {
        self.config.content.hide_latex
    }

    /// Whether frontmatter is hidden from what is displayed: `hide_frontmatter`
    /// is on and the whole document is shown, the only text that can start
    /// with frontmatter. Applied to a section, it took a section opening
    /// with a `---` rule for frontmatter and hid it up to the next rule.
    fn frontmatter_filter_active(&self) -> bool {
        self.should_hide_frontmatter() && self.selected_heading_index().is_none()
    }

    /// Whether LaTeX is filtered from this document: `hide_latex` is on and
    /// the document has LaTeX in it. Run on an ordinary document, the filter
    /// ate dollar signs, backslashed paths and indentation.
    pub fn latex_filter_active(&self) -> bool {
        self.should_hide_latex() && self.latex_detected
    }

    /// Check if aggressive LaTeX filtering is enabled (from config)
    pub fn should_latex_aggressive(&self) -> bool {
        self.config.content.latex_aggressive
    }
}

#[cfg(test)]
mod heading_identity_tests {
    use super::*;
    use crate::keybindings::Action;

    const CHANGELOG: &str = "# One\n\n## Added\n\n- a\n\n# Two\n\n## Added\n\n- b\n\n# Three\n\n## Child\n\n### Deep\n\ntext\n";

    fn app() -> App {
        App::new(
            crate::parser::parse_markdown(CHANGELOG),
            "CHANGELOG.md".into(),
            PathBuf::from("CHANGELOG.md"),
            Config::default(),
            ColorMode::Rgb,
            false,
        )
    }

    fn outline(app: &App) -> Vec<&str> {
        app.outline_items
            .iter()
            .map(|item| item.text.as_str())
            .collect()
    }

    /// Collapse was keyed by heading text, so collapsing one `# One` made
    /// every heading with its name collapse: here the two `## Added` share
    /// a name, and collapsing the first `# One`'s does not touch `# Two`'s.
    #[test]
    fn collapsing_a_heading_leaves_its_namesakes_alone() {
        let mut app = app();
        app.focus = Focus::Outline;
        let first_one = outline(&app).iter().position(|t| *t == "One").unwrap();
        app.select_outline_index(first_one);
        app.collapse();
        assert_eq!(
            outline(&app),
            ["One", "Two", "Added", "Three", "Child", "Deep"]
        );
    }

    /// Anchors were searched in the visible outline rows only, so a heading
    /// under a collapsed one was not found, and `#added-1` never reached
    /// the second `## Added`.
    #[test]
    fn anchors_reach_collapsed_and_repeated_headings() {
        let mut app = app();
        app.collapse_all();
        app.jump_to_anchor("deep").unwrap();
        assert_eq!(app.selected_heading_text(), Some("Deep"));

        app.jump_to_anchor("added-1").unwrap();
        assert_eq!(app.selected_heading_index(), Some(3));
    }

    /// A link copied from GitHub keeps `_` (`#snake_case`), which treemd's
    /// own slugs drop, so it was "not found" whatever the config said.
    #[test]
    fn github_anchors_are_followed_in_either_style() {
        let mut app = App::new(
            crate::parser::parse_markdown("# Intro\n## snake_case\n## Other\n"),
            "t.md".into(),
            PathBuf::from("t.md"),
            Config::default(),
            ColorMode::Rgb,
            false,
        );
        app.jump_to_anchor("snake_case").unwrap();
        assert_eq!(app.selected_heading_text(), Some("snake_case"));
        app.jump_to_anchor("other").unwrap();
        app.jump_to_anchor("snakecase").unwrap();
        assert_eq!(app.selected_heading_text(), Some("snake_case"));
    }

    /// In interactive mode `g` and `G` moved the outline, switching section
    /// under the selected element; they move between elements now.
    #[test]
    fn first_and_last_in_interactive_mode_pick_elements() {
        let mut app = App::new(
            crate::parser::parse_markdown(
                "# A\n\n[one](a.md)\n\n[two](b.md)\n\n[three](c.md)\n\n# B\n",
            ),
            "t.md".into(),
            PathBuf::from("t.md"),
            Config::default(),
            ColorMode::Rgb,
            false,
        );
        app.select_outline_index(0);
        app.enter_interactive_mode();
        let count = app.interactive_state.elements.len();
        assert!(count >= 3);

        app.execute_action(Action::Last);
        assert_eq!(app.interactive_state.current_index, Some(count - 1));
        assert_eq!(app.selected_heading_text(), Some("A"));
        app.execute_action(Action::First);
        assert_eq!(app.interactive_state.current_index, Some(0));
        assert_eq!(app.selected_heading_text(), Some("A"));
    }

    /// Element lines count from the first line of the displayed text, after
    /// the blank lines below the heading; counted from the heading's next
    /// line, `e` opened the editor above the element.
    #[test]
    fn the_editor_opens_on_the_element_line() {
        let mut app = App::new(
            crate::parser::parse_markdown("# A\n\n\n[link](a.md)\n"),
            "t.md".into(),
            PathBuf::from("t.md"),
            Config::default(),
            ColorMode::Rgb,
            false,
        );
        app.select_outline_index(0);
        app.enter_interactive_mode();
        assert_eq!(app.interactive_element_source_line(), Some(4));
    }

    /// In help, `G` moved the outline behind the popup.
    #[test]
    fn last_in_help_scrolls_help() {
        let mut app = app();
        app.select_outline_index(0);
        app.show_help = true;
        app.help_max_scroll = 40;
        app.execute_action(Action::Last);
        assert_eq!(app.help_scroll, 40);
        assert_eq!(app.selected_heading_text(), Some("One"));
    }
}

#[cfg(test)]
mod display_filter_tests {
    use super::*;

    fn app_with(content: &str) -> App {
        App::new(
            crate::parser::parse_markdown(content),
            "note.md".into(),
            PathBuf::from("note.md"),
            Config::default(),
            ColorMode::Rgb,
            false,
        )
    }

    /// A section opening with a `---` rule was taken for frontmatter and
    /// hidden up to the next rule.
    #[test]
    fn a_rule_at_the_top_of_a_section_is_content() {
        let mut app = app_with("# Title\n\n---\n\nImportant text here.\n\n---\n\nMore text.\n");
        app.select_outline_index(0);
        assert!(
            app.displayed_section_content()
                .contains("Important text here.")
        );
    }

    /// With LaTeX hidden by default, the filter ran on every document and
    /// collapsed indentation, so a nested list displayed flat and a checkbox
    /// toggle mapped to the wrong item.
    #[test]
    fn a_document_without_latex_is_displayed_as_written() {
        let body = "- [ ] top one\n  - [ ] sub a\n- [ ] costs $5 and $10";
        let mut app = app_with(&format!("# Tasks\n\n{body}\n"));
        app.select_outline_index(0);
        assert!(app.config.content.hide_latex);
        assert_eq!(app.displayed_section_content().as_str(), body);
    }
}
