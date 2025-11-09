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

    // Ensure file is provided (unless we already handled setup)
    let file = args.file.clone().unwrap_or_else(|| {
        eprintln!("Error: markdown file argument is required");
        eprintln!("\nUsage: treemd <FILE>\n");
        eprintln!("For shell completion setup, use:");
        eprintln!("  treemd --setup-completions");
        std::process::exit(1);
    });

    // Parse the markdown file
    let doc = match parser::parse_file(&file) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        }
    };

    // If no flags, launch TUI
    if !args.list
        && !args.tree
        && !args.count
        && args.section.is_none()
        && args.command.is_none()
        && !args.setup_completions
    {
        let mut terminal = ratatui::init();
        let app = treemd::App::new(doc);
        let result = treemd::tui::run(&mut terminal, app);
        ratatui::restore();
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

    match format {
        OutputFormat::Tree | OutputFormat::Plain => {
            for (i, node) in tree.iter().enumerate() {
                let is_last = i == tree.len() - 1;
                print!("{}", node.render_box_tree("", is_last));
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
