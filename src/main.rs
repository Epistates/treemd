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
    // Rust ignores SIGPIPE, so writing into `treemd -l doc.md | head -1`
    // after `head` exits panicked with "failed printing to stdout". The
    // default disposition ends the process quietly, as for any other filter.
    // Remote fetching is unaffected: std's TCP sockets write with
    // MSG_NOSIGNAL or SO_NOSIGPIPE, so a dropped server is still an error.
    #[cfg(unix)]
    // SAFETY: called first in main, before any other thread exists, and
    // SIG_DFL is a valid disposition for SIGPIPE.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

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

    // Handle --man-page (doesn't require input)
    if args.man_page {
        use clap::CommandFactory;
        use std::io::Write;
        let man = clap_mangen::Man::new(Cli::command());
        let mut buf: Vec<u8> = Vec::new();
        man.render(&mut buf)?;
        std::io::stdout().write_all(&buf)?;
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

    // Remote documents: http(s) URLs are fetched directly, `github:owner/repo`
    // opens that repository's README.
    let remote_spec = args
        .file
        .first()
        .map(|p| p.to_string_lossy().to_string())
        .filter(|s| treemd::input::is_remote_spec(s));

    // Determine input source - check for remote and file picker cases first
    let (input_source, needs_file_picker, file_picker_dir) = if let Some(ref spec) = remote_spec {
        match fetch_remote(spec) {
            Ok(content) => (treemd::input::InputSource::Stdin(content), false, None),
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    } else {
        let picker_placeholder = || {
            treemd::input::InputSource::Stdin(
                "# Select a file\n\nPress o to open a markdown file.".to_string(),
            )
        };
        let cli_mode = !launches_tui(&args);
        if args.file.len() > 1 {
            eprintln!(
                "warning: only the first file is opened; {} more ignored",
                args.file.len() - 1
            );
        }

        match args.file.first() {
            // No file, but something is piped in: that is the document, as in
            // `cat doc.md | treemd -q '.h'` or `tree | treemd`. This has to
            // come before the file picker, which would otherwise take over
            // whenever the working directory holds a markdown file.
            None if treemd::input::is_stdin_piped() => {
                match treemd::input::determine_input_source(None) {
                    Ok(source) => (source, false, None),
                    Err(e) => {
                        eprintln!("Error reading input: {}", e);
                        process::exit(1);
                    }
                }
            }
            // Nothing to read. The TUI can offer the directory's markdown
            // files; a listing or a query has no document, which is a usage
            // error rather than an empty success.
            None if cli_mode => {
                eprintln!("error: no input: name a markdown file, or pipe one in");
                print_usage_hint();
                process::exit(2);
            }
            None => {
                let cwd = std::env::current_dir().unwrap_or_default();
                let has_markdown = std::fs::read_dir(&cwd)
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| entry.ok())
                    .any(|entry| {
                        let path = entry.path();
                        path.is_file()
                            && path
                                .extension()
                                .and_then(|ext| ext.to_str())
                                .is_some_and(|ext| ext == "md" || ext == "markdown")
                    });
                if !has_markdown {
                    eprintln!("No markdown files found in current directory.");
                    print_usage_hint();
                    process::exit(2);
                }
                (picker_placeholder(), true, None)
            }
            Some(path) if path.is_dir() => {
                if cli_mode {
                    eprintln!(
                        "error: {} is a directory; name a markdown file",
                        path.display()
                    );
                    process::exit(1);
                }
                (picker_placeholder(), true, Some(path.clone()))
            }
            Some(path) => match treemd::input::determine_input_source(Some(path.as_path())) {
                Ok(source) => (source, false, None),
                Err(e) => {
                    eprintln!("Error reading {}: {}", path.display(), e);
                    process::exit(1);
                }
            },
        }
    };

    // No terminal to draw on, as in `treemd doc.md > out` or `| cat`: print
    // the document, as bat and less do, instead of writing the TUI's escape
    // sequences into a file or a pipe.
    if launches_tui(&args) && !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        if needs_file_picker {
            eprintln!("error: stdout is not a terminal, and there is no document to print");
            process::exit(2);
        }
        let (treemd::input::InputSource::File(content)
        | treemd::input::InputSource::Stdin(content)) = input_source;
        print!("{content}");
        return Ok(());
    }

    // Check if stdin was piped (before consuming input_source)
    let stdin_was_piped = matches!(input_source, treemd::input::InputSource::Stdin(_));

    // Process input (handles tree format conversion, markdown passthrough,
    // etc.). Only for the TUI: the heading it gives a heading-less document is
    // there so the outline has something to select, and in CLI output it is a
    // heading the document does not have, shifting every line number by two.
    let markdown_content = if launches_tui(&args) {
        match treemd::input::process_input(input_source) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error processing input: {}", e);
                process::exit(1);
            }
        }
    } else {
        match input_source {
            treemd::input::InputSource::File(c) | treemd::input::InputSource::Stdin(c) => c,
        }
    };

    // Parse the markdown content
    let doc = parser::parse_markdown(&markdown_content);

    // Handle query mode
    if let Some(ref query_str) = args.query {
        return handle_query_mode(
            &doc,
            query_str,
            args.query_output.as_deref(),
            args.exit_status,
        );
    }

    // If no flags, launch TUI
    if launches_tui(&args) {
        // Load configuration
        let mut config = treemd::Config::load();

        // Apply theme override from CLI if provided
        if let Some(ref theme_name) = args.theme {
            // A misspelled name used to fall back to Ocean Dark in silence.
            if theme_name.parse::<treemd::tui::theme::ThemeName>().is_err() {
                let names: Vec<&str> = treemd::tui::theme::ThemeName::all()
                    .map(|name| name.config_name())
                    .collect();
                eprintln!(
                    "error: unknown theme '{theme_name}'; themes are {}",
                    names.join(", ")
                );
                process::exit(2);
            }
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
                ColorModeArg::None => ColorMode::Mono,
            }
        } else {
            // Check config file setting before falling back to auto-detection
            use treemd::tui::ColorMode;
            match config.terminal.color_mode.as_str() {
                "rgb" => ColorMode::Rgb,
                "256" => ColorMode::Indexed256,
                "none" => ColorMode::Mono,
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
                    let _ = stdin().read(&mut [0u8]);
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
        use crossterm::event::{EnableBracketedPaste, EnableMouseCapture};
        use crossterm::terminal::EnterAlternateScreen;
        use std::io::stdout;

        // Install panic hook before entering raw mode so the terminal is
        // restored if anything below panics.
        treemd::tui::tty::install_panic_hook();
        // And signals, so one arriving while the terminal is set up (the image
        // probe can take seconds) does not leave it in raw mode.
        treemd::tui::signals::install();

        // Manually initialize to get better error messages
        // Use our custom enable_raw_mode that handles piped stdin
        treemd::tui::tty::enable_raw_mode().inspect_err(|e| {
            eprintln!("Failed to enable raw mode: {}", e);
            eprintln!("Note: When piping input, ensure you have a controlling terminal.");
        })?;

        stdout().execute(EnterAlternateScreen).inspect_err(|_| {
            treemd::tui::tty::disable_raw_mode().ok();
        })?;

        // Mouse capture: best-effort. Some terminals don't support it; that's
        // fine — keyboard navigation still works.
        let _ = stdout().execute(EnableMouseCapture);
        // Treat pasted text as one event so it can never execute keybindings.
        let _ = stdout().execute(EnableBracketedPaste);

        let backend = ratatui::backend::CrosstermBackend::new(stdout());
        let mut terminal = ratatui::Terminal::new(backend).inspect_err(|_| {
            treemd::tui::tty::restore();
        })?;

        // Get filename and path (use placeholders for stdin)
        let (filename, file_path) = if !args.file.is_empty() && !args.file[0].is_dir() {
            let file = &args.file[0];
            let name = file
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("stdin")
                .to_string();
            // A URL or `github:` spec is kept as given: canonicalized, it
            // resolved against a directory named `http:` in the working
            // directory, if there was one, and the fetched page was then
            // treated as a local file, its links and images included.
            let path = if remote_spec.is_some() {
                file.clone()
            } else {
                file.canonicalize().unwrap_or_else(|_| file.clone())
            };
            (name, path)
        } else {
            // Stdin input or directory
            (
                "stdin".to_string(),
                std::path::PathBuf::from(treemd::input::STDIN_PATH),
            )
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

        // Cleanup terminal state, the same way a panic or a signal does
        treemd::tui::tty::restore();

        return result;
    }

    // Handle CLI commands
    handle_cli_mode(&args, &doc);
    Ok(())
}

/// Fetch a remote document. http(s) URLs are fetched directly;
/// `github:owner/repo` resolves to the repository's README on the default
/// branch via raw.githubusercontent.com.
/// How to give treemd input, for the errors that mean there was none.
fn print_usage_hint() {
    eprintln!("\nUsage: treemd [OPTIONS] <FILE>");
    eprintln!("       treemd [OPTIONS] -");
    eprintln!("       treemd [OPTIONS] .           # Open file picker");
    eprintln!("       tree | treemd [OPTIONS]\n");
    eprintln!("Tip: Navigate to a directory with .md files, or specify a file path.");
    eprintln!("\nFor shell completion setup, use:");
    eprintln!("  treemd --setup-completions");
}

/// Whether these arguments open the TUI rather than printing and exiting.
fn launches_tui(args: &Cli) -> bool {
    args.query.is_none()
        && !args.list
        && !args.tree
        && !args.count
        && args.section.is_none()
        && args.at_line.is_none()
}

fn fetch_remote(spec: &str) -> Result<String, String> {
    // ureq sets no timeouts of its own, so a server that accepts and never
    // answers hung treemd for good. An https request also stays on https
    // through its redirects rather than following one down to plain http.
    let agent = |url: &str| -> ureq::Agent {
        ureq::Agent::config_builder()
            .timeout_connect(Some(std::time::Duration::from_secs(10)))
            .timeout_global(Some(std::time::Duration::from_secs(30)))
            .https_only(url.starts_with("https://"))
            .build()
            .into()
    };
    let fetch = |url: &str, accept: Option<&str>| -> Result<String, String> {
        let mut request = agent(url).get(url);
        if let Some(accept) = accept {
            request = request.header("Accept", accept);
        }
        let mut response = request
            .call()
            .map_err(|e| format!("Failed to fetch {}: {}", url, e))?;
        // ureq's limit counts the bytes received, before decompression, so
        // a 40 KB gzip body expanding to gigabytes passed it. The text is
        // held to the same size after decoding.
        const MAX_FETCH_BYTES: u64 = 10 * 1024 * 1024;
        let mut text = String::new();
        std::io::Read::read_to_string(
            &mut std::io::Read::take(
                response
                    .body_mut()
                    .with_config()
                    .limit(MAX_FETCH_BYTES)
                    .reader(),
                MAX_FETCH_BYTES + 1,
            ),
            &mut text,
        )
        .map_err(|e| format!("Failed to read response from {}: {}", url, e))?;
        if text.len() as u64 > MAX_FETCH_BYTES {
            return Err(format!("{url} is larger than the 10 MB limit"));
        }
        Ok(text)
    };

    let Some(repo) = spec.strip_prefix("github:") else {
        return fetch(spec, None);
    };
    let repo = repo.trim_matches('/');
    if repo.split('/').filter(|s| !s.is_empty()).count() != 2 {
        return Err(format!(
            "Invalid GitHub spec '{}': expected github:owner/repo",
            spec
        ));
    }

    // GitHub's own answer to "what is this repository's README", whatever
    // it is named: readme.md, README.markdown, docs/README.md. Asking for
    // README.md at the root by name missed all of those. The API allows 60
    // unauthenticated requests an hour, so past that the name is tried.
    let api = format!("https://api.github.com/repos/{repo}/readme");
    fetch(&api, Some("application/vnd.github.raw+json")).or_else(|api_error| {
        let raw = format!("https://raw.githubusercontent.com/{repo}/HEAD/README.md");
        fetch(&raw, None).map_err(|raw_error| format!("{api_error}; then {raw_error}"))
    })
}

fn handle_cli_mode(args: &Cli, doc: &Document) {
    // Apply filters. Both apply when both are given: `-L 2 --filter api` is
    // the level-2 headings mentioning api, not every level-2 heading.
    let mut headings: Vec<_> = match args.filter {
        Some(ref filter) => doc.filter_headings(filter),
        None => doc.headings.iter().collect(),
    };
    if let Some(level) = args.level {
        headings.retain(|h| h.level == level);
    }

    // --at-line takes precedence over the other flag-based modes.
    if let Some(line) = args.at_line {
        print_heading_at_line(doc, line);
        return;
    }

    // Handle different modes
    if args.count {
        print_heading_counts(doc);
    } else if args.tree {
        print_tree(doc, &args.output, &headings, args.line_numbers);
    } else if let Some(ref section_name) = args.section {
        extract_section(doc, section_name);
    } else if args.list {
        // `-o json` is the nested document, always whole. Say so rather than
        // print every heading as if the filters had been applied.
        if matches!(args.output, OutputFormat::Json)
            && (args.level.is_some() || args.filter.is_some())
        {
            eprintln!(
                "note: -L and --filter do not apply to -o json, which is the whole document; \
                 for filtered JSON use -q, e.g. -q '.h2' --query-output json"
            );
        }
        print_headings(&headings, &args.output, doc, args.line_numbers);
    }
}

/// Print the heading at or immediately before `target_line` (1-indexed).
///
/// Used by the `at-line` subcommand. Walks the heading list once, counting
/// newlines between consecutive offsets so the whole call is O(content_len)
/// rather than O(headings * content_len).
fn print_heading_at_line(doc: &Document, target_line: usize) {
    if target_line == 0 {
        eprintln!("Error: line number must be >= 1");
        process::exit(1);
    }

    let mut best: Option<&parser::Heading> = None;
    let mut prev_line = 1usize;
    let mut prev_offset = 0usize;

    for h in &doc.headings {
        let h_line = prev_line + doc.content[prev_offset..h.offset].matches('\n').count();
        if h_line <= target_line {
            best = Some(h);
        } else {
            break;
        }
        prev_line = h_line;
        prev_offset = h.offset;
    }

    match best {
        Some(h) => {
            let prefix = "#".repeat(h.level);
            println!("{} {}", prefix, cli::terminal::display(&h.text));
        }
        None => {
            eprintln!("No heading at or before line {}", target_line);
            process::exit(1);
        }
    }
}

fn print_headings(
    headings: &[&parser::Heading],
    format: &OutputFormat,
    doc: &Document,
    line_numbers: bool,
) {
    match format {
        OutputFormat::Plain => {
            let ranges = line_numbers.then(|| doc.heading_line_ranges());
            for heading in headings {
                let prefix = "#".repeat(heading.level);
                match ranges.as_ref().and_then(|r| r.get(&heading.offset)) {
                    Some(&(start, end)) => {
                        println!(
                            "{} {} [{}-{}]",
                            prefix,
                            cli::terminal::display(&heading.text),
                            start,
                            end
                        )
                    }
                    None => println!("{} {}", prefix, cli::terminal::display(&heading.text)),
                }
            }
        }
        OutputFormat::Json => {
            // Use new nested JSON output with markdown intelligence
            let slug_style = treemd::Config::load().content.slug_style;
            let json_output = parser::build_json_output_with(doc, None, slug_style);
            let json = serde_json::to_string_pretty(&json_output)
                .expect("JSON serialization of document output should not fail");
            println!("{}", json);
        }
        OutputFormat::Tree => print_tree(doc, format, headings, line_numbers),
    }
}

fn print_tree(
    doc: &Document,
    format: &OutputFormat,
    headings: &[&parser::Heading],
    line_numbers: bool,
) {
    // Build the tree from the (possibly filtered) heading subset so that
    // --tree --filter / --tree --level honor the docstring. When no filter
    // is in play, `headings` is the full list and we get the same tree as
    // doc.build_tree().
    let tree = if headings.len() == doc.headings.len() {
        doc.build_tree()
    } else {
        let owned: Vec<parser::Heading> = headings.iter().map(|&h| h.clone()).collect();
        // build_tree only inspects self.headings, so empty content is fine.
        Document::new(String::new(), owned).build_tree()
    };

    let config = treemd::Config::load();
    let compact = config.is_compact_tree();

    match format {
        OutputFormat::Tree | OutputFormat::Plain => {
            let ranges = line_numbers.then(|| doc.heading_line_ranges());
            for (i, node) in tree.iter().enumerate() {
                let is_last = i == tree.len() - 1;
                let rendered = match ranges.as_ref() {
                    Some(ranges) => node.render_box_tree_with_lines("", is_last, compact, ranges),
                    None => node.render_box_tree_styled("", is_last, compact),
                };
                print!("{}", cli::terminal::display(&rendered));
            }
        }
        OutputFormat::Json => {
            // For JSON, serialize the (filtered) flat headings list.
            // Tree serialization would need custom implementation.
            let owned: Vec<parser::Heading> = headings.iter().map(|&h| h.clone()).collect();
            let json = serde_json::to_string_pretty(&owned)
                .expect("JSON serialization of headings should not fail");
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

    // Use stored byte offsets instead of string-searching the rendered heading.
    // String search is wrong when the heading source has inline markdown
    // (e.g. `## **Bold** Section`) since heading.text is the stripped form.
    let start = heading.offset;
    let index = doc
        .headings
        .iter()
        .position(|h| h.offset == start)
        .expect("a found heading is one of the document's");
    let end = doc.section_end(index);

    println!("{}", cli::terminal::display(doc.content[start..end].trim()));
}

fn handle_query_mode(
    doc: &Document,
    query_str: &str,
    output_format: Option<&str>,
    exit_status: bool,
) -> Result<()> {
    use treemd::query::{self, OutputFormat};

    // Parse output format
    let format = output_format
        .map(|s| s.parse::<OutputFormat>())
        .transpose()
        .map_err(|e| {
            eprintln!("Error: {}", e);
            process::exit(2);
        })?
        .unwrap_or(OutputFormat::Plain);

    // jq's exit codes: 3 for a query that does not parse, 5 for one that
    // fails while running. Both were 1, which `--exit-status` also uses for
    // no results, so a script could not tell a typo from an empty match.
    let parsed = query::parse(query_str).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(3);
    });
    let slug_style = treemd::Config::load().content.slug_style;
    match query::engine(doc)
        .with_slug_style(slug_style)
        .execute(&parsed)
    {
        Ok(results) => {
            // No results prints nothing, like jq, except as JSON, which is
            // always one array so a consumer can parse it the same way
            // whatever the count: `[]` when nothing matched.
            if !results.is_empty()
                || matches!(format, OutputFormat::Json | OutputFormat::JsonPretty)
            {
                let output = query::format_output(&results, format);
                println!("{}", cli::terminal::display(&output));
            }
            if exit_status && results.is_empty() {
                process::exit(1);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("{}", e);
            process::exit(5);
        }
    }
}

fn print_query_help() {
    let help = r#"
treemd Query Language (tql)

A jq-like query language for navigating and extracting markdown structure.
Results print one per line; a selector that matches nothing prints nothing
(`[]` with --query-output json) and exits 0. A query that does not parse
exits 3, and one that fails while running exits 5.

ELEMENT SELECTORS
    .h, .heading        All headings (any level)
    .h1 - .h6           Headings by level
    .code               All code blocks
    .link, .a           All links
    .img, .image        All images
    .table              All tables
    .list               All lists (a nested list is its own entry)
    .blockquote, .quote All blockquotes
    .para, .p           All paragraphs
    .frontmatter, .fm   YAML front matter, as an object
    .                   The whole document

    After a pipe, a selector applied to a heading looks only inside that
    heading's section: `.h2[0] | .code` is the code under the first h2.
    On an input that has a key or property of the same name, `.name` reads
    it instead: `.fm | .image` is the front matter's `image`.

FILTERS & INDEXING
    .h2[Features]       h2s whose text contains "Features" (case-insensitive)
    .h2[Getting Started] A bare filter runs to the ], spaces included
    .h2["Install"]      h2s whose whole text is "Install" (case-insensitive)
    .code[rust]         Code blocks by language (whole name, case-insensitive)
    .code[c++]          Also .code[c#], .code[objective-c]
    .link[external]     Links by type: external, anchor, relative, wikilink
    .h2[0]              First h2
    .h2[-1]             Last h2
    .h2[1:3]            h2s at index 1 and 2
    .h2[:3]             First 3 h2s
    .h2[]               Every h2
    .table[0].rows[][0] After a stream of arrays, an index applies to each
    .fm.tags[1:]        A slice of an array is an array, as in jq
    .fm["draft-of"]     A key with a dash; also .fm."draft-of"

HIERARCHY
    .h1 > .h2           h2s directly under each h1
    .h1 > .code         Code in each h1's own body, before its first subheading
    .h1 >> .code        Code anywhere in each h1's section
    Works for every element kind except front matter, which is document-wide.

PROPERTIES
    headings    .text .level .line .slug .content .md .children .parent
                (.content and .md include subsections; .line is the
                heading's own line)
    code        .lang .text .lines .start_line .end_line
    links       .text .url .type
    images      .alt .src .title
    tables      .headers .rows .cols .alignments
    lists       .items .ordered
    paragraphs  .text        blockquotes  .text
    document    .heading_count .word_count .text
    objects     .key, for any key (also front matter); a missing key is null

PIPES
    .h2 | text          Heading text (strips ##)
    .h2 | count         Number of h2s
    .code | lang        Code block languages
    .link | url         Link URLs

    A collection function after a selector (or anything else that yields
    several values) runs once on all of them: count, length, first, last,
    nth, limit, skip, sort, sort_by, unique, reverse, group_by, join, min,
    max, add, any, all, levels, langs, types. Given a single array, as in
    `[.h2] | count`, they work on that array. Ordering functions (sort,
    sort_by, unique, reverse, limit, skip) hand back a stream. A stage that
    starts with one works on the whole stream too: `.h | count > 3`.

COLLECTION FUNCTIONS
    count, length       Count values (alias: len, size)
    first, last         First/last value (alias: head)
    nth(n)              Value at index n (negative counts from the end)
    limit(n), take(n)   First n values
    skip(n), drop(n)    All but the first n values
    reverse             Reverse order
    sort                Sort (null, booleans, numbers, strings, arrays, objects)
    sort_by(f)          Sort by f, e.g. sort_by(.level) or sort_by("level")
    unique              Drop repeated values, keeping the first
    group_by(f)         Object of arrays keyed by f (alias: group)
    flatten             Flatten one level of nested arrays
    min, max            Smallest/largest number
    add                 Sum numbers, or concatenate as text
    keys, values        Keys/values of an object (indices/items of an array)

STRING FUNCTIONS
    text                Text of a value (a heading's text, a code body)
    upper, lower        Case conversion
    trim                Strip surrounding whitespace
    split(sep)          Split into an array
    join(sep)           Join values with sep
    replace(a, b)       Replace every a with b
    slugify             URL-friendly slug
    lines, words, chars Count lines/words/characters

TESTS
    select(cond)        Keep the input when cond is true (alias: where, filter)
    contains(s)         Text contains s, case-insensitive (alias: includes)
    startswith(s)       Text starts with s, case-insensitive
    endswith(s)         Text ends with s, case-insensitive
    matches(regex)      Text matches a regex (case-sensitive)
    has(key)            Object has key
    any(cond), all(cond) Whether cond holds for any/every value
    empty               Whether the input is null or an empty string,
                        array or object
    type                Type name: heading, code, string, number, ...
    not                 Negation: `not .x` or `.x | not`

CONTENT FUNCTIONS
    content             Section body under a heading
    md                  Raw markdown of a heading's section
    url, href, src      Link URL or image source
    lang                Code block language

AGGREGATION FUNCTIONS
    stats               Document statistics
    levels              Heading count by level
    langs               Code block count by language
    types               Link count by type
    levels, langs and types count the elements they are given (`.h2 |
    levels`), or the whole document's when given none; stats is always
    the whole document.

OPERATORS
    ==, !=              Equality; values of different types are never equal
    <, <=, >, >=        Comparison; across types, jq's order: null <
                        booleans < numbers < strings < arrays < objects
    and, or, not        Logic; `not .level == 2` is `not (.level == 2)`
    +, -, *, /, %       Arithmetic as in jq: + joins strings, arrays and
                        objects, - removes array items, / splits a string,
                        "ab" * 3 repeats it. Other types are an error:
                        "x" + (.level | tostring)
    a // b              a, or b when a is null, false, 0 or empty
    ( )                 Grouping

CONSTRUCTION & CONDITIONALS
    {title: .text, n: (.h | count)}     Object
    [.h2 | .text]                       Array
    if .level == 1 then "top" elif .level == 2 then "sub" else "deep" end
    With no else, a false condition passes the input through unchanged.
    `,` separates whole queries (`.h1, .h2`); it is not allowed inside ( ).

EXAMPLES
    # List all h2 headings
    treemd -q '.h2' doc.md

    # Count headings
    treemd -q '.h2 | count' doc.md

    # Filter headings (three equivalent ways)
    treemd -q '.h | select(contains("API"))' doc.md
    treemd -q '.h | where(contains("API"))' doc.md
    treemd -q '.h[API]' doc.md

    # Rust code under the "Install" section
    treemd -q '.h2[Install] | .code[rust]' doc.md

    # External link URLs
    treemd -q '.link[external] | url' doc.md

    # Headings sorted by level, and grouped by level
    treemd -q '.h | sort_by(.level) | text' doc.md
    treemd -q '.h | group_by(.level)' doc.md

    # Outline with each heading's direct children
    treemd -q '.h1 | {title: .text, sections: [.children[] | .text]}' doc.md

    # JSON output
    treemd -q '.h2' --query-output json doc.md

OUTPUT FORMATS (--query-output)
    plain       Human-readable text (default)
    json        Compact JSON, always an array
    json-pretty Pretty-printed JSON array (alias: jsonp)
    jsonl       One JSON value per line
    md          Raw markdown
    tree        Tree structure

For more details, see: https://github.com/epistates/treemd
"#;
    println!("{}", help.trim());
}
