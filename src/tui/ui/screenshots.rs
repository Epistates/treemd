//! Screenshots of the TUI, for reviewing how a change looks.
//!
//! `TREEMD_SCREENSHOTS=dir cargo test screenshots -- --ignored` draws a set
//! of screens in every theme and writes each as an SVG with its colors. Turn
//! them into PNGs with `rsvg-convert`. The test is ignored by default: it
//! checks nothing and writes files.

use super::render;
use crate::tui::app::App;
use crate::tui::terminal_compat::ColorMode;
use crate::tui::theme::ThemeName;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use std::fmt::Write as _;

const CELL_W: f64 = 8.4;
const CELL_H: f64 = 18.0;
/// What `Color::Reset` stands for: a dark terminal's own colors.
const TERMINAL_FG: &str = "#d0d0d0";
const TERMINAL_BG: &str = "#16181d";

/// The 16 ANSI colors as a typical dark terminal draws them.
const ANSI: [&str; 16] = [
    "#1d1f21", "#cc6666", "#b5bd68", "#f0c674", "#81a2be", "#b294bb", "#8abeb7", "#c5c8c6",
    "#666666", "#d54e53", "#b9ca4a", "#e7c547", "#7aa6da", "#c397d8", "#70c0b1", "#eaeaea",
];

fn hex(color: Color, default: &str) -> String {
    let indexed = |i: u8| -> String {
        match i {
            0..=15 => ANSI[i as usize].to_string(),
            16..=231 => {
                let i = i - 16;
                let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
                format!(
                    "#{:02x}{:02x}{:02x}",
                    level(i / 36),
                    level((i / 6) % 6),
                    level(i % 6)
                )
            }
            _ => {
                let v = 8 + (i - 232) * 10;
                format!("#{v:02x}{v:02x}{v:02x}")
            }
        }
    };
    match color {
        Color::Reset => default.to_string(),
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Indexed(i) => indexed(i),
        Color::Black => indexed(0),
        Color::Red => indexed(1),
        Color::Green => indexed(2),
        Color::Yellow => indexed(3),
        Color::Blue => indexed(4),
        Color::Magenta => indexed(5),
        Color::Cyan => indexed(6),
        Color::Gray => indexed(7),
        Color::DarkGray => indexed(8),
        Color::LightRed => indexed(9),
        Color::LightGreen => indexed(10),
        Color::LightYellow => indexed(11),
        Color::LightBlue => indexed(12),
        Color::LightMagenta => indexed(13),
        Color::LightCyan => indexed(14),
        Color::White => indexed(15),
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `buffer` as an SVG, one rect per background run and one text per cell.
pub(crate) fn svg(buffer: &Buffer) -> String {
    let area = buffer.area;
    let width = f64::from(area.width) * CELL_W;
    let height = f64::from(area.height) * CELL_H;
    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" font-family="Menlo, monospace" font-size="14" shape-rendering="crispEdges">"#
    );
    let _ = writeln!(
        out,
        r#"<rect width="{width}" height="{height}" fill="{TERMINAL_BG}"/>"#
    );
    for y in 0..area.height {
        for x in 0..area.width {
            let cell = &buffer[(x, y)];
            let reversed = cell.modifier.contains(Modifier::REVERSED);
            let (fg, bg) = if reversed {
                (hex(cell.bg, TERMINAL_BG), hex(cell.fg, TERMINAL_FG))
            } else {
                (hex(cell.fg, TERMINAL_FG), hex(cell.bg, TERMINAL_BG))
            };
            let (px, py) = (f64::from(x) * CELL_W, f64::from(y) * CELL_H);
            if bg != TERMINAL_BG {
                let _ = writeln!(
                    out,
                    r#"<rect x="{px}" y="{py}" width="{CELL_W}" height="{CELL_H}" fill="{bg}"/>"#
                );
            }
            let symbol = cell.symbol();
            if symbol.trim().is_empty() {
                continue;
            }
            let mut attrs = String::new();
            if cell.modifier.contains(Modifier::BOLD) {
                attrs.push_str(r#" font-weight="bold""#);
            }
            if cell.modifier.contains(Modifier::ITALIC) {
                attrs.push_str(r#" font-style="italic""#);
            }
            if cell.modifier.contains(Modifier::UNDERLINED) {
                attrs.push_str(r#" text-decoration="underline""#);
            }
            if cell.modifier.contains(Modifier::DIM) {
                attrs.push_str(r#" opacity="0.6""#);
            }
            let _ = writeln!(
                out,
                r#"<text x="{px}" y="{}" fill="{fg}"{attrs}>{}</text>"#,
                py + CELL_H * 0.75,
                escape(symbol)
            );
        }
    }
    out.push_str("</svg>\n");
    out
}

fn shot(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| render(frame, app)).unwrap();
    svg(terminal.backend().buffer())
}

const SAMPLE: &str = "# treemd\n\nA markdown navigator with **tree-based** structural navigation.\n\n\
## Features\n\n- Two panes: an *outline* and the content\n- [x] Queries with `tql`\n- [ ] Images in the terminal\n\n\
> Read documents the way you think about them: by structure.\n\n\
```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n\n\
| Key | Action |\n|-----|--------|\n| `j` | Next |\n| `k` | Previous |\n\n\
See the [README](https://github.com/epistates/treemd) and [Install](#install).\n\n\
### Navigation\n\nMove with `j` and `k`.\n\n### Search\n\nPress `/`.\n\n\
## Install\n\n```sh\ncargo install treemd\n```\n\n## License\n\nMIT\n";

fn app(theme: ThemeName) -> App {
    app_in(theme, ColorMode::Rgb)
}

fn app_in(theme: ThemeName, mode: ColorMode) -> App {
    let mut config = crate::Config::default();
    config.ui.theme = format!("{theme:?}");
    let mut app = App::new(
        crate::parser::parse_markdown(SAMPLE),
        "README.md".into(),
        std::path::PathBuf::from("README.md"),
        config,
        mode,
        false,
    );
    app.jump_to_heading(1);
    app
}

#[test]
#[ignore = "writes screenshots; run with TREEMD_SCREENSHOTS=dir"]
fn screenshots() {
    use crate::keybindings::Action;
    use strum::IntoEnumIterator;

    let Some(dir) = std::env::var_os("TREEMD_SCREENSHOTS") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();

    type Scene = (&'static str, fn(&mut App));
    let scenes: [Scene; 8] = [
        ("normal", |_| {}),
        ("help", |app| {
            app.execute_action(Action::ToggleHelp);
        }),
        ("themes", |app| {
            app.execute_action(Action::ToggleThemePicker);
        }),
        ("palette", |app| {
            app.execute_action(Action::OpenCommandPalette);
        }),
        ("search", |app| {
            app.execute_action(Action::EnterSearchMode);
            for c in "nav".chars() {
                app.apply_text_input_edit(crate::tui::app::TextInputEdit::Insert(c));
            }
        }),
        ("interactive", |app| {
            app.execute_action(Action::EnterInteractiveMode);
            app.execute_action(Action::InteractiveNext);
        }),
        ("links", |app| {
            app.jump_to_heading(0);
            app.execute_action(Action::EnterLinkFollowMode);
        }),
        ("files", |app| {
            app.execute_action(Action::OpenFilePicker);
        }),
    ];

    for theme in ThemeName::iter() {
        for (name, setup) in scenes {
            let mut app = app(theme);
            setup(&mut app);
            let file = dir.join(format!("{theme:?}-{name}.svg"));
            std::fs::write(file, shot(&mut app, 120, 34)).unwrap();
        }
    }
    // NO_COLOR: no color at all.
    for (name, setup) in scenes {
        let mut app = app_in(ThemeName::OceanDark, ColorMode::Mono);
        setup(&mut app);
        let file = dir.join(format!("mono-{name}.svg"));
        std::fs::write(file, shot(&mut app, 120, 34)).unwrap();
    }
    // A narrow terminal, where the footer has to choose.
    for (name, setup) in scenes {
        let mut app = app(ThemeName::OceanDark);
        setup(&mut app);
        let file = dir.join(format!("narrow-{name}.svg"));
        std::fs::write(file, shot(&mut app, 60, 20)).unwrap();
    }
}
