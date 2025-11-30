use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[cfg(feature = "unstable-dynamic")]
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate, ValueCompleter};

#[derive(Parser, Debug)]
#[command(name = "treemd")]
#[command(version)]
#[command(about = "A markdown navigator with tree-based structural navigation")]
#[command(
    long_about = "treemd - A modern markdown viewer combining tree-based navigation with interactive TUI.\n\n\
    Launch without flags for interactive mode with dual-pane interface, vim-style navigation,\n\
    syntax highlighting, and real-time search. Use flags for CLI mode to extract, filter,\n\
    and analyze markdown structure.\n\n\
    Examples:\n  \
    treemd README.md              # Interactive TUI mode\n  \
    treemd -l README.md           # List all headings\n  \
    treemd --tree README.md       # Show heading tree\n  \
    treemd -s Installation doc.md # Extract section\n  \
    treemd --setup-completions    # Set up shell completions"
)]
pub struct Cli {
    /// Markdown file to view (.md or .markdown), or '-' for stdin
    ///
    /// Path to the markdown file to open. Use '-' to read from stdin.
    /// If no file is specified and stdin is piped, input is read from stdin.
    ///
    /// Examples:
    ///   treemd README.md         # Open file
    ///   treemd -                 # Read from stdin
    ///   cat doc.md | treemd -l   # Pipe markdown
    #[arg(add = markdown_file_completer())]
    pub file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// List all headings in the document (non-interactive)
    ///
    /// Displays all headings with their level indicators (# for h1, ## for h2, etc.).
    /// Combine with --filter or --level to narrow results.
    #[arg(short = 'l', long = "list")]
    pub list: bool,

    /// Show heading tree structure with box-drawing characters (non-interactive)
    ///
    /// Renders the document structure as a visual tree using Unicode box-drawing.
    /// Shows parent-child relationships between headings hierarchically.
    #[arg(long = "tree")]
    pub tree: bool,

    /// Filter headings by text pattern (case-insensitive)
    ///
    /// Only shows headings containing the specified text.
    /// Works with --list or --tree modes.
    ///
    /// Example: --filter "install" matches "Installation" and "Installing"
    #[arg(long = "filter", value_name = "PATTERN")]
    pub filter: Option<String>,

    /// Show only headings at specific level (1-6)
    ///
    /// Filters headings by their level:
    ///   1 = # (h1), 2 = ## (h2), 3 = ### (h3), etc.
    ///
    /// Example: -L 2 shows only ## headings
    #[arg(short = 'L', long = "level", value_name = "LEVEL")]
    pub level: Option<usize>,

    /// Output format for --list and --tree modes
    ///
    /// Controls how headings are displayed:
    ///   plain - Human-readable text (default)
    ///   json  - JSON array for scripting/parsing
    ///   tree  - Box-drawing tree structure
    #[arg(short = 'o', long = "output", default_value = "plain")]
    pub output: OutputFormat,

    /// Extract specific section by heading name
    ///
    /// Extracts content from a heading until the next heading of same or higher level.
    /// Useful for pulling specific sections from large documents.
    ///
    /// Example: -s "Usage" extracts the Usage section
    #[arg(short = 's', long = "section", value_name = "HEADING")]
    pub section: Option<String>,

    /// Count headings by level (shows statistics)
    ///
    /// Displays a summary showing how many headings exist at each level (h1-h6)
    /// and the total count.
    #[arg(long = "count")]
    pub count: bool,

    /// Set up shell completions interactively
    ///
    /// Interactive helper to configure tab completion for your shell (bash/zsh/fish).
    /// Detects your shell, finds the config file, and offers to add completion setup.
    /// Completions intelligently filter to show only .md/.markdown files.
    #[arg(long = "setup-completions")]
    pub setup_completions: bool,

    /// Set theme for TUI mode
    ///
    /// Override the saved theme preference. Available themes:
    /// OceanDark, Nord, Dracula, Solarized, Monokai, Gruvbox, TokyoNight, CatppuccinMocha
    ///
    /// Example: --theme Nord
    #[arg(long = "theme", value_name = "THEME")]
    pub theme: Option<String>,

    /// Force color mode (auto, rgb, 256)
    ///
    /// Override automatic terminal detection:
    ///   auto - Detect terminal capabilities (default)
    ///   rgb  - Force true color (16M colors)
    ///   256  - Force 256-color palette
    ///
    /// Example: --color-mode 256
    #[arg(long = "color-mode", value_name = "MODE")]
    pub color_mode: Option<ColorModeArg>,

    /// Query expression for selecting/filtering document elements
    ///
    /// Uses a jq-like syntax for navigating and extracting markdown structure.
    /// Supports element selectors (.h2, .code, .link), filters, pipes, and more.
    ///
    /// Examples:
    ///   -q '.h2'                    # All h2 headings
    ///   -q '.code[rust]'            # Rust code blocks
    ///   -q '.h1[Features] > .h2'    # h2s under "Features"
    ///   -q '.link | url'            # All link URLs
    ///   -q '.h | select(contains("API"))' # Headings with "API"
    ///
    /// See --query-help for full documentation.
    #[arg(short = 'q', long = "query", value_name = "EXPR")]
    pub query: Option<String>,

    /// Show query language documentation and examples
    ///
    /// Displays comprehensive help for the query language including:
    /// - Element selectors (.h1-6, .code, .link, etc.)
    /// - Filters and indexing
    /// - Built-in functions
    /// - Pipe composition
    /// - Output formats
    #[arg(long = "query-help")]
    pub query_help: bool,

    /// Output format for query results
    ///
    /// Controls how query results are displayed:
    ///   plain  - Human-readable text (default)
    ///   json   - Compact JSON
    ///   jsonp  - Pretty-printed JSON
    ///   jsonl  - Line-delimited JSON
    ///   md     - Raw markdown
    ///   tree   - Tree structure
    ///
    /// Example: -q '.h2' --query-output json
    #[arg(long = "query-output", value_name = "FORMAT")]
    pub query_output: Option<String>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ColorModeArg {
    /// Automatically detect terminal capabilities
    Auto,
    /// Force RGB/true color mode
    Rgb,
    /// Force 256-color mode
    #[value(name = "256")]
    Color256,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// Show heading at specific line number
    ///
    /// Finds and displays the heading that appears at or before the given line number.
    /// Useful for jumping to a specific location in the document structure.
    AtLine {
        /// Line number in the markdown file
        ///
        /// The line number to search for. Returns the heading at or immediately
        /// before this line.
        line: usize,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    /// Plain text output
    Plain,
    /// JSON output
    Json,
    /// Tree format with box-drawing
    Tree,
}

#[cfg(feature = "unstable-dynamic")]
fn markdown_file_completer() -> ArgValueCompleter {
    use std::ffi::OsStr;
    use std::path::Path;

    struct MarkdownCompleter;

    impl ValueCompleter for MarkdownCompleter {
        fn complete(&self, current: &OsStr) -> Vec<CompletionCandidate> {
            // Parse the input to extract the directory being completed
            // e.g., "../docs/README" -> directory="../docs", prefix="README"
            let input_str = current.to_string_lossy();
            let input_path = Path::new(input_str.as_ref());

            // Determine which directory to search
            let search_dir: &Path;
            let prefix: String;

            if input_str.is_empty() {
                // No input yet, show current directory
                search_dir = Path::new(".");
                prefix = String::new();
            } else if input_str.ends_with('/') || input_str.ends_with('\\') {
                // Ends with separator, show contents of that directory
                search_dir = input_path;
                prefix = String::new();
            } else {
                // Partial path, show completions in parent directory
                // NOTE: parent() returns Some("") for simple filenames like "R"
                // We need to normalize empty paths to "." for correct completion
                let parent = input_path.parent().unwrap_or(Path::new("."));
                search_dir = if parent.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    parent
                };
                prefix = input_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
            };

            // Read the target directory
            let entries = match std::fs::read_dir(search_dir) {
                Ok(entries) => entries,
                Err(_) => return vec![],
            };

            entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let path = entry.path();
                    let is_dir = path.is_dir();
                    let file_name = path.file_name()?.to_string_lossy().to_string();

                    // Filter by prefix if provided
                    if !prefix.is_empty()
                        && !file_name.to_lowercase().starts_with(&prefix.to_lowercase())
                    {
                        return None;
                    }

                    // Build the completion value relative to the original input
                    let completion_value = if search_dir == Path::new(".") {
                        file_name.clone()
                    } else {
                        search_dir.join(&file_name).to_string_lossy().to_string()
                    };

                    // Include directories and .md/.markdown files
                    if is_dir {
                        // Append trailing slash to directories for easier navigation
                        let mut dir_completion = completion_value;
                        if !dir_completion.ends_with('/') {
                            dir_completion.push('/');
                        }
                        Some(
                            CompletionCandidate::new(dir_completion).help(Some("directory".into())),
                        )
                    } else if let Some(ext) = path.extension() {
                        let ext_lower = ext.to_string_lossy().to_lowercase();
                        if ext_lower == "md" || ext_lower == "markdown" {
                            Some(CompletionCandidate::new(completion_value))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        }
    }

    ArgValueCompleter::new(MarkdownCompleter)
}

#[cfg(not(feature = "unstable-dynamic"))]
fn markdown_file_completer() -> clap::builder::ValueHint {
    clap::ValueHint::FilePath
}
