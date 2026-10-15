use crate::tui::terminal_compat::ColorMode;
use ratatui::style::{Color, Modifier, Style};
use strum::IntoEnumIterator;

/// The built-in themes. The config names one by its variant name
/// (`theme = "Nord"`), case-insensitively.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter, strum::EnumString, strum::IntoStaticStr,
)]
#[strum(ascii_case_insensitive)]
pub enum ThemeName {
    OceanDark,
    Nord,
    Dracula,
    Solarized,
    Monokai,
    Gruvbox,
    TokyoNight,
    CatppuccinMocha,
    OneDark,
    RosePine,
    Kanagawa,
    Everforest,
    GitHubDark,
    CatppuccinLatte,
    SolarizedLight,
    GitHubLight,
    /// The terminal's own 16 colors and background.
    Terminal,
}

impl ThemeName {
    /// Every theme, in the order the picker lists them.
    pub fn all() -> impl Iterator<Item = ThemeName> {
        ThemeName::iter()
    }

    /// The name the config uses: `OceanDark`, `CatppuccinLatte`.
    pub fn config_name(self) -> &'static str {
        self.into()
    }

    /// The name to show: `Ocean Dark`, `Catppuccin Latte`.
    pub fn display_name(self) -> &'static str {
        match self {
            ThemeName::OceanDark => "Ocean Dark",
            ThemeName::Nord => "Nord",
            ThemeName::Dracula => "Dracula",
            ThemeName::Solarized => "Solarized Dark",
            ThemeName::Monokai => "Monokai",
            ThemeName::Gruvbox => "Gruvbox",
            ThemeName::TokyoNight => "Tokyo Night",
            ThemeName::CatppuccinMocha => "Catppuccin Mocha",
            ThemeName::OneDark => "One Dark",
            ThemeName::RosePine => "Rose Pine",
            ThemeName::Kanagawa => "Kanagawa",
            ThemeName::Everforest => "Everforest",
            ThemeName::GitHubDark => "GitHub Dark",
            ThemeName::CatppuccinLatte => "Catppuccin Latte",
            ThemeName::SolarizedLight => "Solarized Light",
            ThemeName::GitHubLight => "GitHub Light",
            ThemeName::Terminal => "Terminal",
        }
    }

    /// One line on what the theme is, for the picker.
    pub fn description(self) -> &'static str {
        match self {
            ThemeName::OceanDark => "Base16 Ocean, cool blues (default)",
            ThemeName::Nord => "Arctic, north-bluish palette",
            ThemeName::Dracula => "Dark, with vibrant accents",
            ThemeName::Solarized => "Precision colors, dark",
            ThemeName::Monokai => "The classic editor palette",
            ThemeName::Gruvbox => "Retro groove, warm and earthy",
            ThemeName::TokyoNight => "Downtown Tokyo at night",
            ThemeName::CatppuccinMocha => "Soothing pastels, dark",
            ThemeName::OneDark => "Atom's One Dark",
            ThemeName::RosePine => "Soho vibes, muted and warm",
            ThemeName::Kanagawa => "After Hokusai's Great Wave",
            ThemeName::Everforest => "Green, soft, easy on the eyes",
            ThemeName::GitHubDark => "GitHub's dark default",
            ThemeName::CatppuccinLatte => "Soothing pastels, light",
            ThemeName::SolarizedLight => "Precision colors, light",
            ThemeName::GitHubLight => "GitHub's light default",
            ThemeName::Terminal => "Your terminal's own colors",
        }
    }

    /// Whether the theme has a light background.
    pub fn is_light(self) -> bool {
        matches!(
            self,
            ThemeName::CatppuccinLatte | ThemeName::SolarizedLight | ThemeName::GitHubLight
        )
    }

    /// The syntect theme code blocks use when `code_theme` is `auto`.
    pub fn code_theme(self) -> &'static str {
        match self {
            ThemeName::Solarized => "Solarized (dark)",
            ThemeName::SolarizedLight => "Solarized (light)",
            ThemeName::CatppuccinLatte | ThemeName::GitHubLight => "InspiredGitHub",
            ThemeName::Monokai => "base16-mocha.dark",
            _ => "base16-ocean.dark",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub background: Color,
    pub foreground: Color,
    pub heading_1: Color,
    pub heading_2: Color,
    pub heading_3: Color,
    pub heading_4: Color,
    pub heading_5: Color,
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub inline_code_fg: Color,
    pub inline_code_bg: Color,
    pub bold_fg: Color,
    pub italic_fg: Color,
    pub list_bullet: Color,
    pub blockquote_border: Color,
    pub blockquote_fg: Color,
    pub code_fence: Color,
    pub title_bar_fg: Color,
    pub scrollbar_fg: Color,
    pub selection_indicator_fg: Color,
    pub selection_indicator_bg: Color,
    pub link_fg: Color,
    pub link_selected_bg: Color,
    pub link_selected_fg: Color,
    pub table_border: Color,
    // Search highlighting
    pub search_match_bg: Color,
    pub search_match_fg: Color,
    pub search_current_bg: Color,
    pub search_current_fg: Color,
    // Footer keybinding hints
    pub help_key_bg: Color,
    pub help_key_fg: Color,
    pub help_desc_fg: Color,
    pub footer_bg: Color,
    /// No color at all: what color marked is drawn with bold, underline
    /// and reverse video instead.
    pub monochrome: bool,
}

/// A theme's colors by role. Every field of [`Theme`] is derived from these
/// in [`Theme::from_palette`], so a theme is a dozen colors rather than
/// forty, and every theme uses each role the same way.
struct Palette {
    name: &'static str,
    /// The page.
    bg: Color,
    /// Raised: the status bar, popups, the selected row, inline code.
    surface: Color,
    /// Lines: unfocused borders, table rules, the scrollbar.
    overlay: Color,
    /// Secondary text: quotes, hints, descriptions.
    muted: Color,
    text: Color,
    /// Bold text and the selected row's text.
    bright: Color,
    /// Focus, titles, keys.
    accent: Color,
    /// Heading colors, `#` to `#####`.
    headings: [Color; 5],
    code: Color,
    italic: Color,
    link: Color,
    /// Search matches.
    yellow: Color,
}

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// `a` mixed into `b`, `amount` of the way from `b` to `a`.
fn mix(a: Color, b: Color, amount: f32) -> Color {
    match (a, b) {
        (Color::Rgb(ar, ag, ab), Color::Rgb(br, bg, bb)) => {
            let ch = |x: u8, y: u8| {
                (f32::from(y) + (f32::from(x) - f32::from(y)) * amount).round() as u8
            };
            Color::Rgb(ch(ar, br), ch(ag, bg), ch(ab, bb))
        }
        _ => b,
    }
}

/// `color` as a terminal with `mode` colors shows it.
fn quantize(color: Color, mode: ColorMode) -> Color {
    match mode {
        ColorMode::Rgb => color,
        ColorMode::Indexed256 => rgb_to_256(color),
        ColorMode::Mono => Color::Reset,
    }
}

/// The sRGB value a color stands for: itself, or for an indexed color its
/// value in the xterm palette. The 16 named colors vary by terminal.
pub(crate) fn rgb_value(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Indexed(i @ 16..=231) => {
            let i = i - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            Some((level(i / 36), level((i / 6) % 6), level(i % 6)))
        }
        Color::Indexed(i @ 232..=255) => {
            let v = 8 + (i - 232) * 10;
            Some((v, v, v))
        }
        _ => None,
    }
}

/// Relative luminance, as WCAG defines it.
fn luminance((r, g, b): (u8, u8, u8)) -> f64 {
    let channel = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

/// WCAG contrast ratio between two colors, 1 to 21, when both are known.
pub(crate) fn contrast(a: Color, b: Color) -> Option<f64> {
    let (la, lb) = (luminance(rgb_value(a)?), luminance(rgb_value(b)?));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    Some((hi + 0.05) / (lo + 0.05))
}

/// `fg`, moved toward white on a dark `on` or black on a light one just
/// far enough to have `min` contrast with it as `mode` shows them.
fn ensure_contrast(fg: Color, on: Color, min: f64, mode: ColorMode) -> Color {
    let dark = rgb_value(on).is_some_and(|rgb| luminance(rgb) < 0.18);
    let toward = if dark {
        Color::Rgb(255, 255, 255)
    } else {
        Color::Rgb(0, 0, 0)
    };
    (0..=20)
        .map(|step| quantize(mix(toward, fg, step as f32 / 20.0), mode))
        .find(|color| contrast(*color, on).is_some_and(|ratio| ratio >= min))
        .unwrap_or_else(|| quantize(toward, mode))
}

/// The color for text on a colored marker `on`: whichever of the theme's
/// background, its brightest text, black and white reads best on it.
fn text_on(on: Color, p: &Palette) -> Color {
    [
        p.bg,
        p.bright,
        Color::Rgb(0, 0, 0),
        Color::Rgb(255, 255, 255),
    ]
    .into_iter()
    .max_by(|a, b| {
        let ratio = |c: &Color| contrast(*c, on).unwrap_or(0.0);
        ratio(a).total_cmp(&ratio(b))
    })
    .unwrap_or(p.bg)
}

impl Theme {
    /// The theme in 24-bit color.
    pub fn from_name(name: ThemeName) -> Self {
        Self::for_terminal(name, ColorMode::Rgb)
    }

    /// The theme as a terminal with `mode` colors can show it.
    pub fn for_terminal(name: ThemeName, mode: ColorMode) -> Self {
        match (name, mode) {
            (_, ColorMode::Mono) => Self::monochrome(),
            (ThemeName::Terminal, _) => Self::terminal(),
            _ => Self::from_palette(&palette(name), mode),
        }
    }

    /// Every field from the palette's roles.
    ///
    /// Text is held to the contrast it needs against what it is drawn on
    /// (see `theme_contrast_tests`), after rounding to the terminal's
    /// colors: a color that falls short is moved toward white on a dark
    /// background or black on a light one until it reads. A palette color
    /// that already reads is left exactly as published.
    fn from_palette(p: &Palette, mode: ColorMode) -> Self {
        let q = |color: Color| quantize(color, mode);
        let bg = q(p.bg);
        let surface = q(p.surface);
        let readable = |fg: Color, on: Color, min: f64| ensure_contrast(fg, on, min, mode);
        let text = readable(p.text, bg, 4.5);
        let bright = readable(p.bright, surface, 4.5);
        let muted = readable(readable(p.muted, bg, 4.5), surface, 4.0);
        let accent = readable(p.accent, bg, 3.0);
        let key = readable(p.accent, surface, 4.5);
        let yellow = q(p.yellow);
        let headings = p.headings.map(|color| readable(color, bg, 3.0));
        Self {
            name: p.name,
            background: bg,
            foreground: text,
            heading_1: headings[0],
            heading_2: headings[1],
            heading_3: headings[2],
            heading_4: headings[3],
            heading_5: headings[4],
            border_focused: accent,
            border_unfocused: q(p.overlay),
            selection_bg: surface,
            selection_fg: bright,
            status_bar_bg: surface,
            status_bar_fg: readable(p.text, surface, 4.5),
            inline_code_fg: readable(p.code, surface, 4.5),
            inline_code_bg: surface,
            bold_fg: readable(p.bright, bg, 4.5),
            italic_fg: readable(p.italic, bg, 4.5),
            list_bullet: accent,
            blockquote_border: q(p.overlay),
            blockquote_fg: muted,
            code_fence: muted,
            title_bar_fg: accent,
            scrollbar_fg: q(p.overlay),
            selection_indicator_fg: text_on(accent, p),
            selection_indicator_bg: accent,
            link_fg: readable(p.link, bg, 4.5),
            link_selected_bg: accent,
            link_selected_fg: text_on(accent, p),
            table_border: q(p.overlay),
            search_match_bg: q(mix(p.yellow, p.bg, 0.3)),
            search_match_fg: readable(p.bright, q(mix(p.yellow, p.bg, 0.3)), 4.5),
            search_current_bg: yellow,
            search_current_fg: text_on(yellow, p),
            help_key_bg: surface,
            help_key_fg: key,
            help_desc_fg: muted,
            footer_bg: bg,
            monochrome: false,
        }
    }

    /// The terminal's own palette: its background, its foreground and its
    /// 16 colors, whatever it has set them to. Works in any color mode.
    fn terminal() -> Self {
        Self {
            name: "Terminal",
            background: Color::Reset,
            foreground: Color::Reset,
            heading_1: Color::Cyan,
            heading_2: Color::Blue,
            heading_3: Color::Green,
            heading_4: Color::Yellow,
            heading_5: Color::Magenta,
            border_focused: Color::Cyan,
            border_unfocused: Color::DarkGray,
            selection_bg: Color::DarkGray,
            selection_fg: Color::White,
            status_bar_bg: Color::DarkGray,
            status_bar_fg: Color::White,
            inline_code_fg: Color::Yellow,
            inline_code_bg: Color::Reset,
            bold_fg: Color::Reset,
            italic_fg: Color::Magenta,
            list_bullet: Color::Cyan,
            blockquote_border: Color::DarkGray,
            blockquote_fg: Color::Gray,
            code_fence: Color::Gray,
            title_bar_fg: Color::Cyan,
            scrollbar_fg: Color::DarkGray,
            selection_indicator_fg: Color::Black,
            selection_indicator_bg: Color::Cyan,
            link_fg: Color::Blue,
            link_selected_bg: Color::Cyan,
            link_selected_fg: Color::Black,
            table_border: Color::DarkGray,
            search_match_bg: Color::DarkGray,
            search_match_fg: Color::Yellow,
            search_current_bg: Color::Yellow,
            search_current_fg: Color::Black,
            help_key_bg: Color::DarkGray,
            help_key_fg: Color::Cyan,
            help_desc_fg: Color::Gray,
            footer_bg: Color::Reset,
            monochrome: false,
        }
    }

    /// No color: every color the terminal's own, and [`Theme::monochrome`]
    /// set so the style helpers mark with bold, underline and reverse video.
    fn monochrome() -> Self {
        Self {
            name: "Monochrome",
            monochrome: true,
            ..Self::all(Color::Reset)
        }
    }

    /// Every color `color`.
    fn all(color: Color) -> Self {
        Self {
            name: "",
            background: color,
            foreground: color,
            heading_1: color,
            heading_2: color,
            heading_3: color,
            heading_4: color,
            heading_5: color,
            border_focused: color,
            border_unfocused: color,
            selection_bg: color,
            selection_fg: color,
            status_bar_bg: color,
            status_bar_fg: color,
            inline_code_fg: color,
            inline_code_bg: color,
            bold_fg: color,
            italic_fg: color,
            list_bullet: color,
            blockquote_border: color,
            blockquote_fg: color,
            code_fence: color,
            title_bar_fg: color,
            scrollbar_fg: color,
            selection_indicator_fg: color,
            selection_indicator_bg: color,
            link_fg: color,
            link_selected_bg: color,
            link_selected_fg: color,
            table_border: color,
            search_match_bg: color,
            search_match_fg: color,
            search_current_bg: color,
            search_current_fg: color,
            help_key_bg: color,
            help_key_fg: color,
            help_desc_fg: color,
            footer_bg: color,
            monochrome: false,
        }
    }

    pub fn heading_color(&self, level: usize) -> Color {
        match level {
            1 => self.heading_1,
            2 => self.heading_2,
            3 => self.heading_3,
            4 => self.heading_4,
            _ => self.heading_5,
        }
    }

    pub fn border_style(&self, focused: bool) -> Style {
        if focused {
            Style::default().fg(self.border_focused)
        } else {
            Style::default().fg(self.border_unfocused)
        }
    }

    /// `style`, or in monochrome reverse video and bold, which is how a
    /// colored highlight shows without color.
    fn or_reversed(&self, style: Style) -> Style {
        if self.monochrome {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            style
        }
    }

    pub fn selection_style(&self) -> Style {
        self.or_reversed(
            Style::default()
                .bg(self.selection_bg)
                .fg(self.selection_fg)
                .add_modifier(Modifier::BOLD),
        )
    }

    pub fn status_bar_style(&self) -> Style {
        if self.monochrome {
            return Style::default().add_modifier(Modifier::REVERSED);
        }
        Style::default()
            .bg(self.status_bar_bg)
            .fg(self.status_bar_fg)
    }

    /// The marker beside a selected element: the arrow, a selected image's
    /// frame.
    pub fn marker_style(&self) -> Style {
        self.or_reversed(
            Style::default()
                .fg(self.selection_indicator_fg)
                .bg(self.selection_indicator_bg)
                .add_modifier(Modifier::BOLD),
        )
    }

    /// A selected link or table cell.
    pub fn selected_link_style(&self) -> Style {
        self.or_reversed(
            Style::default()
                .fg(self.link_selected_fg)
                .bg(self.link_selected_bg)
                .add_modifier(Modifier::BOLD),
        )
    }

    /// A chip: a short label on a block of `color`, such as the mode in the
    /// status bar or the focused pane's title.
    pub fn chip_style(&self, color: Color) -> Style {
        self.or_reversed(
            Style::default()
                .bg(color)
                .fg(self.text_on(color))
                .add_modifier(Modifier::BOLD),
        )
    }

    pub fn inline_code_style(&self) -> Style {
        Style::default()
            .fg(self.inline_code_fg)
            .bg(self.inline_code_bg)
    }

    pub fn bold_style(&self) -> Style {
        Style::default()
            .fg(self.bold_fg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn italic_style(&self) -> Style {
        Style::default()
            .fg(self.italic_fg)
            .add_modifier(Modifier::ITALIC)
    }

    pub fn text_style(&self) -> Style {
        Style::default().fg(self.foreground)
    }

    pub fn content_style(&self) -> Style {
        Style::default().fg(self.foreground).bg(self.background)
    }

    pub fn code_fence_style(&self) -> Style {
        Style::default().fg(self.code_fence)
    }

    // Modal/popup color helpers (already respects color mode since theme is converted)
    pub fn modal_bg(&self) -> Color {
        self.selection_bg
    }

    /// The readable text color on `bg`: whichever of the theme's
    /// background, its text, black and white contrasts most with it. For a
    /// terminal color, whose value is unknown, the theme's background.
    pub fn text_on(&self, bg: Color) -> Color {
        [
            self.background,
            self.foreground,
            Color::Rgb(0, 0, 0),
            Color::Rgb(255, 255, 255),
        ]
        .into_iter()
        .filter_map(|fg| contrast(fg, bg).map(|ratio| (ratio, fg)))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map_or(self.background, |(_, fg)| fg)
    }

    /// A popup's own colors: its background and its text. Setting only the
    /// background left unstyled text in the terminal's default color, light
    /// gray on a light theme's popup.
    pub fn modal_style(&self) -> Style {
        Style::default().bg(self.modal_bg()).fg(self.modal_text())
    }

    pub fn modal_border(&self) -> Color {
        self.border_focused
    }

    pub fn modal_title(&self) -> Color {
        self.border_focused
    }

    /// Style for search matches (non-current)
    pub fn search_match_style(&self) -> Style {
        if self.monochrome {
            return Style::default().add_modifier(Modifier::UNDERLINED);
        }
        Style::default()
            .bg(self.search_match_bg)
            .fg(self.search_match_fg)
    }

    /// Style for the current/focused search match
    pub fn search_current_style(&self) -> Style {
        self.or_reversed(
            Style::default()
                .bg(self.search_current_bg)
                .fg(self.search_current_fg)
                .add_modifier(Modifier::BOLD),
        )
    }

    pub fn modal_text(&self) -> Color {
        self.foreground
    }

    pub fn modal_selected_fg(&self) -> Color {
        self.help_key_fg
    }

    pub fn modal_selected_marker(&self) -> Color {
        self.list_bullet
    }

    pub fn modal_key_fg(&self) -> Color {
        self.help_key_fg
    }

    pub fn modal_description(&self) -> Color {
        self.blockquote_fg
    }

    /// Style for keybinding hints keys in footer
    pub fn help_key_style(&self) -> Style {
        Style::default()
            .bg(self.help_key_bg)
            .fg(self.help_key_fg)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for keybinding hints descriptions in footer
    pub fn help_desc_style(&self) -> Style {
        Style::default().fg(self.help_desc_fg)
    }

    /// Style for footer background
    pub fn footer_style(&self) -> Style {
        Style::default().bg(self.footer_bg)
    }

    /// Apply custom color overrides from config
    pub fn with_custom_colors(
        mut self,
        custom: &crate::config::CustomThemeConfig,
        mode: ColorMode,
    ) -> Self {
        // No color means none, custom colors included.
        if mode == ColorMode::Mono {
            return self;
        }
        // Helper macro to apply color override if present
        macro_rules! apply_color {
            ($field:ident) => {
                if let Some(ref color_value) = custom.$field {
                    if let Some(color) = color_value.to_color() {
                        // Quantize custom RGB colors if in 256-color mode
                        self.$field = if matches!(mode, ColorMode::Indexed256) {
                            rgb_to_256(color)
                        } else {
                            color
                        };
                    }
                }
            };
        }

        apply_color!(background);
        apply_color!(foreground);
        apply_color!(heading_1);
        apply_color!(heading_2);
        apply_color!(heading_3);
        apply_color!(heading_4);
        apply_color!(heading_5);
        apply_color!(border_focused);
        apply_color!(border_unfocused);
        apply_color!(selection_bg);
        apply_color!(selection_fg);
        apply_color!(status_bar_bg);
        apply_color!(status_bar_fg);
        apply_color!(inline_code_fg);
        apply_color!(inline_code_bg);
        apply_color!(bold_fg);
        apply_color!(italic_fg);
        apply_color!(list_bullet);
        apply_color!(blockquote_border);
        apply_color!(blockquote_fg);
        apply_color!(code_fence);
        apply_color!(title_bar_fg);
        apply_color!(scrollbar_fg);
        apply_color!(selection_indicator_fg);
        apply_color!(selection_indicator_bg);
        apply_color!(link_fg);
        apply_color!(link_selected_bg);
        apply_color!(link_selected_fg);
        apply_color!(table_border);
        // Search highlighting
        apply_color!(search_match_bg);
        apply_color!(search_match_fg);
        apply_color!(search_current_bg);
        apply_color!(search_current_fg);
        // Footer keybinding hints
        apply_color!(help_key_bg);
        apply_color!(help_key_fg);
        apply_color!(help_desc_fg);
        apply_color!(footer_bg);

        self
    }
}

/// Each theme's palette. Colors are the theme's own published values where
/// it has them; a few secondary-text colors are lifted until they meet the
/// contrast `theme_contrast_tests` holds every theme to.
fn palette(name: ThemeName) -> Palette {
    match name {
        ThemeName::OceanDark => Palette {
            name: "Ocean Dark",
            bg: rgb(0x2b303b),
            surface: rgb(0x343d46),
            overlay: rgb(0x4f5b66),
            muted: rgb(0xa7adba),
            text: rgb(0xc0c5ce),
            bright: rgb(0xeff1f5),
            accent: rgb(0x64c8ff),
            headings: [
                rgb(0x64c8ff),
                rgb(0x96c8ff),
                rgb(0x96ffc8),
                rgb(0xc8ff96),
                rgb(0xc8c8c8),
            ],
            code: rgb(0xffc864),
            italic: rgb(0xc08fff),
            link: rgb(0x7aa6ff),
            yellow: rgb(0xebcb8b),
        },
        ThemeName::Nord => Palette {
            name: "Nord",
            bg: rgb(0x2e3440),
            surface: rgb(0x3b4252),
            overlay: rgb(0x4c566a),
            muted: rgb(0xaab3c5),
            text: rgb(0xd8dee9),
            bright: rgb(0xeceff4),
            accent: rgb(0x88c0d0),
            headings: [
                rgb(0x88c0d0),
                rgb(0x8fbcbb),
                rgb(0xa3be8c),
                rgb(0xebcb8b),
                rgb(0xb48ead),
            ],
            code: rgb(0xebcb8b),
            italic: rgb(0xb48ead),
            link: rgb(0x88c0d0),
            yellow: rgb(0xebcb8b),
        },
        ThemeName::Dracula => Palette {
            name: "Dracula",
            bg: rgb(0x282a36),
            surface: rgb(0x44475a),
            overlay: rgb(0x6272a4),
            muted: rgb(0xb4b9d6),
            text: rgb(0xf8f8f2),
            bright: rgb(0xffffff),
            accent: rgb(0xbd93f9),
            headings: [
                rgb(0x8be9fd),
                rgb(0x50fa7b),
                rgb(0xffb86c),
                rgb(0xff79c6),
                rgb(0xbd93f9),
            ],
            code: rgb(0xf1fa8c),
            italic: rgb(0xbd93f9),
            link: rgb(0x8be9fd),
            yellow: rgb(0xf1fa8c),
        },
        ThemeName::Solarized => Palette {
            name: "Solarized Dark",
            bg: rgb(0x002b36),
            surface: rgb(0x073642),
            overlay: rgb(0x586e75),
            muted: rgb(0x93a1a1),
            text: rgb(0xb2bdbd),
            bright: rgb(0xeee8d5),
            accent: rgb(0x268bd2),
            headings: [
                rgb(0x268bd2),
                rgb(0x2aa198),
                rgb(0x859900),
                rgb(0xb58900),
                rgb(0xcb4b16),
            ],
            code: rgb(0xb58900),
            italic: rgb(0x9a9fe0),
            link: rgb(0x2aa198),
            yellow: rgb(0xb58900),
        },
        ThemeName::Monokai => Palette {
            name: "Monokai",
            bg: rgb(0x272822),
            surface: rgb(0x3e3d32),
            overlay: rgb(0x75715e),
            muted: rgb(0xb3ae94),
            text: rgb(0xf8f8f2),
            bright: rgb(0xffffff),
            accent: rgb(0x66d9ef),
            headings: [
                rgb(0x66d9ef),
                rgb(0xa6e22e),
                rgb(0xfd971f),
                rgb(0xf92672),
                rgb(0xae81ff),
            ],
            code: rgb(0xe6db74),
            italic: rgb(0xae81ff),
            link: rgb(0x66d9ef),
            yellow: rgb(0xe6db74),
        },
        ThemeName::Gruvbox => Palette {
            name: "Gruvbox",
            bg: rgb(0x282828),
            surface: rgb(0x3c3836),
            overlay: rgb(0x665c54),
            muted: rgb(0xbdae93),
            text: rgb(0xebdbb2),
            bright: rgb(0xfbf1c7),
            accent: rgb(0xb8bb26),
            headings: [
                rgb(0x83a598),
                rgb(0xb8bb26),
                rgb(0xfabd2f),
                rgb(0xfe8019),
                rgb(0xd3869b),
            ],
            code: rgb(0xfabd2f),
            italic: rgb(0xd3869b),
            link: rgb(0x83a598),
            yellow: rgb(0xfabd2f),
        },
        ThemeName::TokyoNight => Palette {
            name: "Tokyo Night",
            bg: rgb(0x1a1b26),
            surface: rgb(0x292e42),
            overlay: rgb(0x414868),
            muted: rgb(0xa9b1d6),
            text: rgb(0xc0caf5),
            bright: rgb(0xe0e6ff),
            accent: rgb(0x7aa2f7),
            headings: [
                rgb(0x7aa2f7),
                rgb(0x7dcfff),
                rgb(0x9ece6a),
                rgb(0xe0af68),
                rgb(0xbb9af7),
            ],
            code: rgb(0xff9e64),
            italic: rgb(0xbb9af7),
            link: rgb(0x7aa2f7),
            yellow: rgb(0xe0af68),
        },
        ThemeName::CatppuccinMocha => Palette {
            name: "Catppuccin Mocha",
            bg: rgb(0x1e1e2e),
            surface: rgb(0x313244),
            overlay: rgb(0x585b70),
            muted: rgb(0xa6adc8),
            text: rgb(0xcdd6f4),
            bright: rgb(0xf5e0dc),
            accent: rgb(0x89b4fa),
            headings: [
                rgb(0x89b4fa),
                rgb(0x89dceb),
                rgb(0xa6e3a1),
                rgb(0xf9e2af),
                rgb(0xcba6f7),
            ],
            code: rgb(0xfab387),
            italic: rgb(0xf5c2e7),
            link: rgb(0x89b4fa),
            yellow: rgb(0xf9e2af),
        },
        ThemeName::OneDark => Palette {
            name: "One Dark",
            bg: rgb(0x282c34),
            surface: rgb(0x3e4451),
            overlay: rgb(0x5c6370),
            muted: rgb(0xa7adba),
            text: rgb(0xc8ccd4),
            bright: rgb(0xe6e9ef),
            accent: rgb(0x61afef),
            headings: [
                rgb(0x61afef),
                rgb(0xc678dd),
                rgb(0x98c379),
                rgb(0xe5c07b),
                rgb(0x56b6c2),
            ],
            code: rgb(0xe5c07b),
            italic: rgb(0xc678dd),
            link: rgb(0x61afef),
            yellow: rgb(0xe5c07b),
        },
        ThemeName::RosePine => Palette {
            name: "Rose Pine",
            bg: rgb(0x191724),
            surface: rgb(0x26233a),
            overlay: rgb(0x524f67),
            muted: rgb(0x908caa),
            text: rgb(0xe0def4),
            bright: rgb(0xfaf8ff),
            accent: rgb(0xc4a7e7),
            headings: [
                rgb(0xebbcba),
                rgb(0xc4a7e7),
                rgb(0x9ccfd8),
                rgb(0xf6c177),
                rgb(0xeb6f92),
            ],
            code: rgb(0xf6c177),
            italic: rgb(0xc4a7e7),
            link: rgb(0x9ccfd8),
            yellow: rgb(0xf6c177),
        },
        ThemeName::Kanagawa => Palette {
            name: "Kanagawa",
            bg: rgb(0x1f1f28),
            surface: rgb(0x2a2a37),
            overlay: rgb(0x54546d),
            muted: rgb(0xa8a69a),
            text: rgb(0xdcd7ba),
            bright: rgb(0xf2ecbc),
            accent: rgb(0x7e9cd8),
            headings: [
                rgb(0x7e9cd8),
                rgb(0x957fb8),
                rgb(0x98bb6c),
                rgb(0xe6c384),
                rgb(0x7fb4ca),
            ],
            code: rgb(0xe6c384),
            italic: rgb(0xa992cd),
            link: rgb(0x7fb4ca),
            yellow: rgb(0xe6c384),
        },
        ThemeName::Everforest => Palette {
            name: "Everforest",
            bg: rgb(0x2d353b),
            surface: rgb(0x3d484d),
            overlay: rgb(0x56635f),
            muted: rgb(0xaab5ad),
            text: rgb(0xd3c6aa),
            bright: rgb(0xe9e0c8),
            accent: rgb(0xa7c080),
            headings: [
                rgb(0xa7c080),
                rgb(0x83c092),
                rgb(0x7fbbb3),
                rgb(0xdbbc7f),
                rgb(0xd699b6),
            ],
            code: rgb(0xdbbc7f),
            italic: rgb(0xd699b6),
            link: rgb(0x7fbbb3),
            yellow: rgb(0xdbbc7f),
        },
        ThemeName::GitHubDark => Palette {
            name: "GitHub Dark",
            bg: rgb(0x0d1117),
            surface: rgb(0x21262d),
            overlay: rgb(0x30363d),
            muted: rgb(0x9198a1),
            text: rgb(0xd1d9e0),
            bright: rgb(0xf0f6fc),
            accent: rgb(0x58a6ff),
            headings: [
                rgb(0x58a6ff),
                rgb(0xd2a8ff),
                rgb(0x7ee787),
                rgb(0xe3b341),
                rgb(0xffa657),
            ],
            code: rgb(0xffa657),
            italic: rgb(0xd2a8ff),
            link: rgb(0x58a6ff),
            yellow: rgb(0xe3b341),
        },
        ThemeName::CatppuccinLatte => Palette {
            name: "Catppuccin Latte",
            bg: rgb(0xeff1f5),
            surface: rgb(0xdce0e8),
            overlay: rgb(0x9ca0b0),
            muted: rgb(0x5c5f77),
            text: rgb(0x4c4f69),
            bright: rgb(0x1e1e2e),
            accent: rgb(0x1e66f5),
            headings: [
                rgb(0x1e66f5),
                rgb(0x8839ef),
                rgb(0x2f7d20),
                rgb(0xa15c00),
                rgb(0xd20f39),
            ],
            code: rgb(0xb5440a),
            italic: rgb(0x8839ef),
            link: rgb(0x1e5fd6),
            yellow: rgb(0xdf8e1d),
        },
        ThemeName::SolarizedLight => Palette {
            name: "Solarized Light",
            bg: rgb(0xfdf6e3),
            surface: rgb(0xeee8d5),
            overlay: rgb(0x93a1a1),
            muted: rgb(0x586e75),
            text: rgb(0x073642),
            bright: rgb(0x002b36),
            accent: rgb(0x268bd2),
            headings: [
                rgb(0x1f74b0),
                rgb(0x1e7a73),
                rgb(0x667500),
                rgb(0x8c6a00),
                rgb(0xb2400f),
            ],
            code: rgb(0x8c6a00),
            italic: rgb(0x5b5fb8),
            link: rgb(0x1f6ea8),
            yellow: rgb(0xb58900),
        },
        ThemeName::GitHubLight => Palette {
            name: "GitHub Light",
            bg: rgb(0xffffff),
            surface: rgb(0xeff2f5),
            overlay: rgb(0xd1d9e0),
            muted: rgb(0x59636e),
            text: rgb(0x1f2328),
            bright: rgb(0x010409),
            accent: rgb(0x0969da),
            headings: [
                rgb(0x0969da),
                rgb(0x8250df),
                rgb(0x1a7f37),
                rgb(0x9a6700),
                rgb(0xbc4c00),
            ],
            code: rgb(0x953800),
            italic: rgb(0x8250df),
            link: rgb(0x0969da),
            yellow: rgb(0x9a6700),
        },
        ThemeName::Terminal => unreachable!("the terminal theme has no palette"),
    }
}

/// The nearest of the 256 colors to `color`: the closer of its nearest
/// point on the 6x6x6 color cube and its nearest gray on the 24-step ramp.
///
/// The cube's levels are 0, 95, 135, 175, 215 and 255. Rounding each channel
/// to a multiple of 51 picked the wrong level for most colors, so a dark
/// background came out a shade lighter or a different hue.
pub(crate) fn rgb_to_256(color: Color) -> Color {
    const LEVELS: [i32; 6] = [0, 95, 135, 175, 215, 255];
    let Color::Rgb(r, g, b) = color else {
        // Already indexed, named or the terminal's own: nothing to map.
        return color;
    };
    let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
    let distance = |(x, y, z): (i32, i32, i32)| (x - r).pow(2) + (y - g).pow(2) + (z - b).pow(2);

    let level = |c: i32| (0..6).min_by_key(|&i| (LEVELS[i] - c).abs()).unwrap_or(0);
    let (ri, gi, bi) = (level(r), level(g), level(b));
    let cube = (LEVELS[ri], LEVELS[gi], LEVELS[bi]);
    let cube_index = 16 + 36 * ri + 6 * gi + bi;

    let gray_step = ((r + g + b) / 3 - 8).clamp(0, 230) / 10;
    let gray_step = (gray_step..=(gray_step + 1).min(23))
        .min_by_key(|&step| {
            let v = 8 + step * 10;
            distance((v, v, v))
        })
        .unwrap_or(gray_step);
    let gray = 8 + gray_step * 10;

    if distance((gray, gray, gray)) < distance(cube) {
        Color::Indexed(232 + gray_step as u8)
    } else {
        Color::Indexed(cube_index as u8)
    }
}

#[cfg(test)]
mod contrast_tests {
    use super::*;

    /// Every pair of colors drawn on top of each other, and the contrast it
    /// needs: 4.5:1 for text (WCAG AA), 4:1 for secondary text on a raised
    /// surface, and 3:1 for bold headings and for lines that mark focus.
    fn requirements(t: &Theme) -> Vec<(&'static str, Color, Color, f64)> {
        let mut pairs = vec![
            ("text", t.foreground, t.background, 4.5),
            ("bold", t.bold_fg, t.background, 4.5),
            ("italic", t.italic_fg, t.background, 4.5),
            ("link", t.link_fg, t.background, 4.5),
            ("quote", t.blockquote_fg, t.background, 4.5),
            ("inline code", t.inline_code_fg, t.inline_code_bg, 4.5),
            ("selection", t.selection_fg, t.selection_bg, 4.5),
            ("status bar", t.status_bar_fg, t.status_bar_bg, 4.5),
            ("footer key", t.help_key_fg, t.help_key_bg, 4.5),
            ("footer hint", t.help_desc_fg, t.footer_bg, 4.5),
            ("popup text", t.modal_text(), t.modal_bg(), 4.5),
            ("popup key", t.modal_key_fg(), t.modal_bg(), 4.5),
            ("popup selection", t.modal_selected_fg(), t.modal_bg(), 4.5),
            ("popup hint", t.modal_description(), t.modal_bg(), 4.0),
            ("selected link", t.link_selected_fg, t.link_selected_bg, 4.5),
            (
                "selection marker",
                t.selection_indicator_fg,
                t.selection_indicator_bg,
                4.5,
            ),
            ("search match", t.search_match_fg, t.search_match_bg, 4.5),
            (
                "current match",
                t.search_current_fg,
                t.search_current_bg,
                4.5,
            ),
            ("focused border", t.border_focused, t.background, 3.0),
            ("title", t.title_bar_fg, t.background, 3.0),
            ("bullet", t.list_bullet, t.background, 3.0),
            ("code fence", t.code_fence, t.background, 3.0),
        ];
        for (level, color) in [
            t.heading_1,
            t.heading_2,
            t.heading_3,
            t.heading_4,
            t.heading_5,
        ]
        .into_iter()
        .enumerate()
        {
            pairs.push((
                ["h1", "h2", "h3", "h4", "h5"][level],
                color,
                t.background,
                3.0,
            ));
        }
        pairs
    }

    /// Text in every theme has the contrast to read, in 24-bit color and in
    /// a 256-color terminal. The review that prompted this found popup hints
    /// at 1.4:1 in Nord and footer hints under 3:1 in four themes.
    #[test]
    fn every_theme_is_readable() {
        let mut failures = Vec::new();
        for name in ThemeName::all().filter(|name| *name != ThemeName::Terminal) {
            for mode in [ColorMode::Rgb, ColorMode::Indexed256] {
                let theme = Theme::for_terminal(name, mode);
                for (what, fg, bg, min) in requirements(&theme) {
                    let ratio = contrast(fg, bg).expect("built-in themes are RGB");
                    if ratio < min {
                        failures.push(format!("{name:?} {mode:?} {what}: {ratio:.2} < {min}"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    /// Every theme has a distinct name, and the config reads back the name
    /// it writes.
    #[test]
    fn theme_names_round_trip() {
        for name in ThemeName::all() {
            assert_eq!(name.config_name().parse::<ThemeName>().ok(), Some(name));
            assert_eq!(
                name.config_name().to_lowercase().parse::<ThemeName>().ok(),
                Some(name)
            );
        }
    }

    /// Without color, what color marked still shows: the selection, chips
    /// and the current match in reverse video, other matches underlined, and
    /// no color left anywhere.
    #[test]
    fn monochrome_marks_without_color() {
        let theme = Theme::for_terminal(ThemeName::Dracula, ColorMode::Mono);
        for style in [
            theme.selection_style(),
            theme.marker_style(),
            theme.chip_style(theme.border_focused),
            theme.search_current_style(),
            theme.status_bar_style(),
        ] {
            assert!(style.add_modifier.contains(Modifier::REVERSED), "{style:?}");
        }
        assert!(
            theme
                .search_match_style()
                .add_modifier
                .contains(Modifier::UNDERLINED)
        );
        assert_eq!(theme.foreground, Color::Reset);
        assert_eq!(theme.heading_1, Color::Reset);
    }

    /// The nearest of the 256 colors: the cube's levels are 0, 95, 135...,
    /// not multiples of 51, and a gray goes to the gray ramp.
    #[test]
    fn colors_map_to_the_nearest_of_256() {
        assert_eq!(rgb_to_256(Color::Rgb(0, 0, 0)), Color::Indexed(16));
        assert_eq!(rgb_to_256(Color::Rgb(255, 255, 255)), Color::Indexed(231));
        assert_eq!(rgb_to_256(Color::Rgb(95, 135, 175)), Color::Indexed(67));
        assert_eq!(rgb_to_256(Color::Rgb(128, 128, 128)), Color::Indexed(244));
        assert_eq!(rgb_to_256(Color::Reset), Color::Reset);
    }
}
