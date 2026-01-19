//! # treemd
//!
//! A markdown navigator with tree-based structural navigation and syntax highlighting.
//!
//! ## Features
//!
//! - Interactive TUI with dual-pane interface (outline + content)
//! - CLI mode for scripting and automation
//! - Syntax-highlighted code blocks (50+ languages)
//! - Tree-based navigation with expand/collapse
//! - Search and filter headings
//! - Multiple output formats (plain, JSON, tree)
//!
//! ## Usage
//!
//! Launch the interactive TUI:
//! ```sh
//! treemd README.md
//! ```
//!
//! List all headings:
//! ```sh
//! treemd -l README.md
//! ```
//!
//! Show heading tree:
//! ```sh
//! treemd --tree README.md
//! ```

mod cli;

use clap::Parser as ClapParser;
use cli::{Cli, OutputFormat};
use color_eyre::Result;
use std::collections::HashMap;
use std::process;
use treemd::{Document, parser};

fn main() -> Result<()> {
    color_eyre::install()?;

    // Handle dynamic shell completions
    #[cfg(feature = "unstable-dynamic")]
    clap_complete::CompleteEnv::with_factory(|| {
        use clap::CommandFactory;
        Cli::command()
    })
    .complete();

    let args = Cli::parse();

    // Handle completion setup
    #[cfg(feature = "unstable-dynamic")]
    if args.setup_completions {
        match cli::setup::setup_completions_interactive("treemd") {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!("Error setting up completions: {}", e);
                cli::setup::print_completion_instructions("treemd");
                process::exit(1);
            }
        }
    }

    // Handle --query-help (doesn't require input)
    if args.query_help {
        print_query_help();
        return Ok(());
    }

    // For TUI mode with piped stdin, we'll read stdin first, then open TUI
    // This allows elegant piping: tree | treemd
    //
    // How it works:
    // 1. Read all of stdin into memory (tree output, markdown, etc.)
    // 2. Process and parse the content
    // 3. ratatui/crossterm opens /dev/tty for keyboard input (not stdin)
    // 4. TUI displays the processed content with full interactivity
    //
    // This is the standard pattern used by: less, fzf, bat, etc.

    // Determine input source - check for file picker case first
    let (input_source, needs_file_picker, file_picker_dir) = match args.file.len() {
        0 => {
            // No file provided - check for .md files in cwd
            use std::fs;
            let cwd = std::env::current_dir().unwrap_or_default();
            let md_files: Vec<_> = fs::read_dir(&cwd)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    let path = entry.path();
                    path.is_file()
                        && path
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext == "md" || ext == "markdown")
                            .unwrap_or(false)
                })
                .collect();

            if md_files.is_empty() {
                eprintln!("No markdown files found in current directory.");
                eprintln!("\nUsage: treemd [OPTIONS] <FILE>");
                eprintln!("       treemd [OPTIONS] -");
                eprintln!("       treemd [OPTIONS] .           # Open file picker");
                eprintln!("       tree | treemd [OPTIONS]\n");
                eprintln!("Tip: Navigate to a directory with .md files, or specify a file path.");
                eprintln!("\nFor shell completion setup, use:");
                eprintln!("  treemd --setup-completions");
                std::process::exit(0);
            }

            // Create dummy document to show file picker
            (
                treemd::input::InputSource::Stdin(
                    "# Select a file\n\nPress Enter to select a markdown file.".to_string(),
                ),
                true,
                None,
            )
        }
        1 => {
            let file_path = &args.file[0];
            // Check if it's a directory
            if file_path.is_dir() {
                // Directory provided - open file picker in that directory
                (
                    treemd::input::InputSource::Stdin(
                        "# Select a file\n\nPress Enter to select a markdown file.".to_string(),
                    ),
                    true,
                    Some(file_path.clone()),
                )
            } else {
                // Single file path was provided - use existing logic
                match treemd::input::determine_input_source(Some(file_path.as_path())) {
                    Ok(source) => (source, false, None),
                    Err(treemd::input::InputError::NoTty) => {
                        eprintln!("Error: markdown file argument is required");
                        eprintln!("\nUsage: treemd [OPTIONS] <FILE>");
                        eprintln!("       treemd [OPTIONS] -");
                        eprintln!("       treemd [OPTIONS] .           # Open file picker");
                        eprintln!("       tree | treemd [OPTIONS]\n");
                        eprintln!(
                            "Use '-' to explicitly read from stdin, or pipe input with CLI flags."
                        );
                        eprintln!("\nFor shell completion setup, use:");
                        eprintln!("  treemd --setup-completions");
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Error reading input: {}", e);
                        process::exit(1);
                    }
                }
            }
        }
        _ => {
            // Multiple files provided - open first file, others available in file picker
            // For now, just open the first file (file picker will show all files in same dir)
            let file_path = &args.file[0];
            if file_path.is_dir() {
                // If first arg is a directory, use it for file picker
                (
                    treemd::input::InputSource::Stdin(
                        "# Select a file\n\nPress Enter to select a markdown file.".to_string(),
                    ),
                    true,
                    Some(file_path.clone()),
                )
            } else {
                match treemd::input::determine_input_source(Some(file_path.as_path())) {
                    Ok(source) => (source, false, None),
                    Err(treemd::input::InputError::NoTty) => {
                        eprintln!("Error: markdown file argument is required");
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Error reading input: {}", e);
                        process::exit(1);
                    }
                }
            }
        }
    };

    // Check if stdin was piped (before consuming input_source)
    let stdin_was_piped = matches!(input_source, treemd::input::InputSource::Stdin(_));

    // Process input (handles tree format conversion, markdown passthrough, etc.)
    let markdown_content = match treemd::input::process_input(input_source) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error processing input: {}", e);
            process::exit(1);
        }
    };

    // Parse the markdown content
    let doc = parser::parse_markdown(&markdown_content);

    // Handle query mode
    if let Some(ref query_str) = args.query {
        return handle_query_mode(&doc, query_str, args.query_output.as_deref());
    }

    // If no flags, launch TUI
    if !args.list
        && !args.tree
        && !args.count
        && args.section.is_none()
        && args.command.is_none()
        && !args.setup_completions
    {
        // Load configuration
        let mut config = treemd::Config::load();

        // Apply theme override from CLI if provided
        if let Some(ref theme_name) = args.theme {
            config.ui.theme = theme_name.clone();
        }

        // Detect terminal capabilities and determine color mode
        // Priority: CLI args > config file > auto-detection
        let caps = treemd::tui::TerminalCapabilities::detect();
        let color_mode = if let Some(ref mode_arg) = args.color_mode {
            // CLI flag takes highest priority
            use cli::ColorModeArg;
            use treemd::tui::ColorMode;
            match mode_arg {
                ColorModeArg::Auto => caps.recommended_color_mode,
                ColorModeArg::Rgb => ColorMode::Rgb,
                ColorModeArg::Color256 => ColorMode::Indexed256,
            }
        } else {
            // Check config file setting before falling back to auto-detection
            use treemd::tui::ColorMode;
            match config.terminal.color_mode.as_str() {
                "rgb" => ColorMode::Rgb,
                "256" => ColorMode::Indexed256,
                // "auto" or any other value falls back to detection
                _ => caps.recommended_color_mode,
            }
        };

        // Show compatibility warning if needed (before TUI init)
        // Skip the warning prompt if stdin was piped (already consumed)
        if caps.should_warn && !config.terminal.warned_terminal_app {
            if let Some(warning) = caps.warning_message() {
                eprintln!("\n{}\n", warning);
                // Only wait for keypress if stdin is still available (not piped)
                if !stdin_was_piped {
                    use std::io::{Read, stdin};
                    let _ = stdin().read(&mut [0u8]).unwrap();
                } else {
                    eprintln!("Press any key in the TUI to continue...");
                }
            }
            // Mark that we've warned the user
            let _ = config.set_warned_terminal_app();
        }

        // Initialize terminal with explicit error handling
        // When stdin is piped, we use /dev/tty for input (handled by tui::tty module)
        use crossterm::ExecutableCommand;
        use crossterm::terminal::EnterAlternateScreen;
        use std::io::stdout;

        // Manually initialize to get better error messages
        // Use our custom enable_raw_mode that handles piped stdin
        treemd::tui::tty::enable_raw_mode().inspect_err(|e| {
            eprintln!("Failed to enable raw mode: {}", e);
            eprintln!("Note: When piping input, ensure you have a controlling terminal.");
        })?;

        stdout().execute(EnterAlternateScreen).inspect_err(|_| {
            treemd::tui::tty::disable_raw_mode().ok();
        })?;

        let backend = ratatui::backend::CrosstermBackend::new(stdout());
        let mut terminal = ratatui::Terminal::new(backend).inspect_err(|_| {
            treemd::tui::tty::disable_raw_mode().ok();
        })?;

        // Get filename and path (use placeholders for stdin)
        let (filename, file_path) = if !args.file.is_empty() && !args.file[0].is_dir() {
            let file = &args.file[0];
            let name = file
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("stdin")
                .to_string();
            let path = file.canonicalize().unwrap_or_else(|_| file.clone());
            (name, path)
        } else {
            // Stdin input or directory
            ("stdin".to_string(), std::path::PathBuf::from("<stdin>"))
        };

        // Determine if images are enabled
        // Priority: CLI flags > config file > default (true)
        let images_enabled = if args.no_images {
            false
        } else if args.images {
            true
        } else {
            config.images.enabled
        };

        let mut app =
            treemd::App::new(doc, filename, file_path, config, color_mode, images_enabled);
        if needs_file_picker {
            app.startup_needs_file_picker = true;
        }
        if let Some(dir) = file_picker_dir {
            app.file_picker_dir = Some(dir.canonicalize().unwrap_or(dir));
        }
        let result = treemd::tui::run(&mut terminal, app);

        // Cleanup terminal state
        use crossterm::terminal::LeaveAlternateScreen;
        stdout().execute(LeaveAlternateScreen).ok();
        treemd::tui::tty::disable_raw_mode().ok();

        return result;
    }

    // Handle CLI commands
    handle_cli_mode(&args, &doc);
    Ok(())
}

fn handle_cli_mode(args: &Cli, doc: &Document) {
    // Apply filters
    let headings: Vec<_> = if let Some(level) = args.level {
        doc.headings_at_level(level)
    } else if let Some(ref filter) = args.filter {
        doc.filter_headings(filter)
    } else {
        doc.headings.iter().collect()
    };

    // Handle different modes
    if args.count {
        print_heading_counts(doc);
    } else if args.tree {
        print_tree(doc, &args.output);
    } else if let Some(ref section_name) = args.section {
        extract_section(doc, section_name);
    } else if args.list {
        print_headings(&headings, &args.output, doc);
    }
}

fn print_headings(headings: &[&parser::Heading], format: &OutputFormat, doc: &Document) {
    match format {
        OutputFormat::Plain => {
            for heading in headings {
                let prefix = "#".repeat(heading.level);
                println!("{} {}", prefix, heading.text);
            }
        }
        OutputFormat::Json => {
            // Use new nested JSON output with markdown intelligence
            let json_output = parser::build_json_output(doc, None);
            let json = serde_json::to_string_pretty(&json_output).unwrap();
            println!("{}", json);
        }
        OutputFormat::Tree => {
            eprintln!("Use --tree for tree output");
            process::exit(1);
        }
    }
}

fn print_tree(doc: &Document, format: &OutputFormat) {
    let tree = doc.build_tree();
    let config = treemd::Config::load();
    let compact = config.is_compact_tree();

    match format {
        OutputFormat::Tree | OutputFormat::Plain => {
            for (i, node) in tree.iter().enumerate() {
                let is_last = i == tree.len() - 1;
                print!("{}", node.render_box_tree_styled("", is_last, compact));
            }
        }
        OutputFormat::Json => {
            // For JSON, we'll serialize the flat headings list
            // (Tree serialization would need custom implementation)
            let json = serde_json::to_string_pretty(&doc.headings).unwrap();
            println!("{}", json);
        }
    }
}

fn print_heading_counts(doc: &Document) {
    let mut counts: HashMap<usize, usize> = HashMap::new();

    for heading in &doc.headings {
        *counts.entry(heading.level).or_insert(0) += 1;
    }

    println!("Heading counts:");
    for level in 1..=6 {
        if let Some(count) = counts.get(&level) {
            let prefix = "#".repeat(level);
            println!("  {}: {}", prefix, count);
        }
    }
    println!("\nTotal: {}", doc.headings.len());
}

fn extract_section(doc: &Document, section_name: &str) {
    let heading = match doc.find_heading(section_name) {
        Some(h) => h,
        None => {
            eprintln!("Section '{}' not found", section_name);
            process::exit(1);
        }
    };

    // Find the section in content
    // This is a simple implementation - could be improved
    let search = format!("{} {}", "#".repeat(heading.level), heading.text);
    if let Some(start) = doc.content.find(&search) {
        // Find next heading at same or higher level
        let after = &doc.content[start..];
        let section_level = heading.level;

        // Find end of section
        let end_pos = doc
            .headings
            .iter()
            .skip_while(|h| h.text != heading.text)
            .skip(1)
            .find(|h| h.level <= section_level)
            .and_then(|next_heading| {
                let search = format!("{} {}", "#".repeat(next_heading.level), next_heading.text);
                after.find(&search)
            })
            .unwrap_or(after.len());

        println!("{}", &after[..end_pos].trim());
    }
}

fn handle_query_mode(doc: &Document, query_str: &str, output_format: Option<&str>) -> Result<()> {
    use treemd::query::{self, OutputFormat};

    // Parse output format
    let format = output_format
        .map(|s| s.parse::<OutputFormat>())
        .transpose()
        .map_err(|e| {
            eprintln!("Error: {}", e);
            process::exit(1);
        })?
        .unwrap_or(OutputFormat::Plain);

    // Execute query
    match query::execute(doc, query_str) {
        Ok(results) => {
            if results.is_empty() {
                // No results - exit silently like jq
                return Ok(());
            }
            let output = query::format_output(&results, format);
            println!("{}", output);
            Ok(())
        }
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

fn print_query_help() {
    let help = r#"
treemd Query Language (tql)

A jq-like query language for navigating and extracting markdown structure.

ELEMENT SELECTORS
    .h, .heading    All headings (any level)
    .h1 - .h6       Headings by level
    .code           All code blocks
    .code[rust]     Code blocks by language
    .link, .a       All links
    .link[external] External links only
    .img            All images
    .table          All tables
    .list           All lists
    .blockquote     All blockquotes

FILTERS & INDEXING
    .h2[Features]       Heading containing "Features" (fuzzy)
    .h2["Installation"] Heading with exact text
    .h2[0]              First h2
    .h2[-1]             Last h2
    .h2[1:3]            h2s at index 1 and 2
    .h2[:3]             First 3 h2s

HIERARCHY
    .h1 > .h2           Direct child h2s under h1s
    .h1 >> .code        Code blocks anywhere under h1s

PIPES
    .h2 | text          Get heading text (strips ##)
    [.h2] | count       Count all h2s
    .code | lang        Get code block languages
    .link | url         Get link URLs

COLLECTION FUNCTIONS
    count, length       Count elements (alias: len, size)
    first, last         First/last element (alias: head)
    limit(n), take(n)   First n elements
    skip(n), drop(n)    Skip first n elements
    nth(n)              Get element at index
    reverse             Reverse order
    sort                Sort alphabetically
    sort_by(key)        Sort by property
    unique              Remove duplicates
    flatten             Flatten nested arrays
    group_by(key)       Group elements by key
    min, max            Min/max numeric value
    add                 Sum numbers or concat strings

STRING FUNCTIONS
    text                Get text representation
    upper, lower        Case conversion
    trim                Strip whitespace
    split(sep)          Split by separator
    join(sep)           Join with separator
    replace(a, b)       Replace substring
    slugify             URL-friendly slug
    lines, words, chars Count lines/words/chars

FILTER FUNCTIONS
    select(cond)        Keep if condition true (alias: where, filter)
    contains(s)         Contains substring (alias: includes)
    startswith(s)       Starts with prefix
    endswith(s)         Ends with suffix
    matches(regex)      Matches regex pattern
    any, all            Check if any/all truthy
    not                 Negate boolean

CONTENT FUNCTIONS
    content             Section content (for headings)
    md                  Raw markdown
    url, href, src      Get URL/link/image source
    lang                Code block language

AGGREGATION FUNCTIONS
    stats               Document statistics
    levels              Heading count by level
    langs               Code block count by language
    types               Link types count

EXAMPLES
    # List all h2 headings
    treemd -q '.h2' doc.md

    # Get heading text only
    treemd -q '.h2 | text' doc.md

    # Count headings
    treemd -q '[.h2] | count' doc.md

    # First 5 headings
    treemd -q '[.h] | limit(5)' doc.md

    # Filter headings (three equivalent ways)
    treemd -q '.h | select(contains("API"))' doc.md
    treemd -q '.h | where(contains("API"))' doc.md
    treemd -q '.h[API]' doc.md

    # All Rust code blocks
    treemd -q '.code[rust]' doc.md

    # External link URLs
    treemd -q '.link[external] | url' doc.md

    # h2s under "Features" section
    treemd -q '.h1[Features] > .h2' doc.md

    # Group headings by level
    treemd -q '[.h] | group_by("level")' doc.md

    # Document statistics
    treemd -q '. | stats' doc.md

    # JSON output
    treemd -q '.h2' --query-output json doc.md

OUTPUT FORMATS (--query-output)
    plain       Human-readable text (default)
    json        Compact JSON
    json-pretty Pretty-printed JSON (alias: jsonp)
    jsonl       Line-delimited JSON (one per line)
    md          Raw markdown
    tree        Tree structure

For more details, see: https://github.com/epistates/treemd
"#;
    println!("{}", help.trim());
}
