use crate::tui::terminal_compat::ColorMode;
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeName {
    OceanDark,
    Nord,
    Dracula,
    Solarized,
    Monokai,
    Gruvbox,
    TokyoNight,
    CatppuccinMocha,
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
}

impl Theme {
    pub fn from_name(name: ThemeName) -> Self {
        match name {
            ThemeName::OceanDark => Self::ocean_dark(),
            ThemeName::Nord => Self::nord(),
            ThemeName::Dracula => Self::dracula(),
            ThemeName::Solarized => Self::solarized(),
            ThemeName::Monokai => Self::monokai(),
            ThemeName::Gruvbox => Self::gruvbox(),
            ThemeName::TokyoNight => Self::tokyo_night(),
            ThemeName::CatppuccinMocha => Self::catppuccin_mocha(),
        }
    }

    pub fn from_name_256(name: ThemeName) -> Self {
        match name {
            ThemeName::OceanDark => Self::ocean_dark_256(),
            ThemeName::Nord => Self::nord_256(),
            ThemeName::Dracula => Self::dracula_256(),
            ThemeName::Solarized => Self::solarized_256(),
            ThemeName::Monokai => Self::monokai_256(),
            ThemeName::Gruvbox => Self::gruvbox_256(),
            ThemeName::TokyoNight => Self::tokyo_night_256(),
            ThemeName::CatppuccinMocha => Self::catppuccin_mocha_256(),
        }
    }

    /// Base16 Ocean Dark - Default theme
    pub fn ocean_dark() -> Self {
        Self {
            name: "Ocean Dark",
            background: Color::Rgb(43, 48, 59),
            foreground: Color::Rgb(192, 197, 206),
            heading_1: Color::Rgb(100, 200, 255),
            heading_2: Color::Rgb(150, 200, 255),
            heading_3: Color::Rgb(150, 255, 200),
            heading_4: Color::Rgb(200, 255, 150),
            heading_5: Color::Rgb(200, 200, 200),
            border_focused: Color::Cyan,
            border_unfocused: Color::DarkGray,
            selection_bg: Color::Rgb(40, 40, 60),
            selection_fg: Color::White,
            status_bar_bg: Color::Rgb(52, 61, 70),
            status_bar_fg: Color::Rgb(192, 197, 206),
            inline_code_fg: Color::Rgb(255, 200, 100),
            inline_code_bg: Color::Rgb(50, 50, 50),
            bold_fg: Color::White,
            italic_fg: Color::Rgb(192, 143, 255),
            list_bullet: Color::Cyan,
            blockquote_border: Color::Rgb(150, 150, 150),
            blockquote_fg: Color::Rgb(150, 150, 150),
            code_fence: Color::Rgb(150, 180, 200),
            title_bar_fg: Color::Rgb(100, 200, 255),
            scrollbar_fg: Color::Rgb(80, 80, 100),
            selection_indicator_fg: Color::Rgb(43, 48, 59),
            selection_indicator_bg: Color::Rgb(100, 200, 255),
            link_fg: Color::Rgb(100, 150, 255),
            link_selected_bg: Color::Rgb(100, 200, 255),
            link_selected_fg: Color::Rgb(43, 48, 59),
            table_border: Color::Rgb(100, 100, 120),
        }
    }

    /// Nord theme - Arctic, north-bluish color palette
    pub fn nord() -> Self {
        Self {
            name: "Nord",
            background: Color::Rgb(46, 52, 64),
            foreground: Color::Rgb(216, 222, 233),
            heading_1: Color::Rgb(136, 192, 208), // Nord Frost
            heading_2: Color::Rgb(143, 188, 187), // Nord Frost
            heading_3: Color::Rgb(163, 190, 140), // Nord Aurora Green
            heading_4: Color::Rgb(235, 203, 139), // Nord Aurora Yellow
            heading_5: Color::Rgb(180, 142, 173), // Nord Aurora Purple
            border_focused: Color::Rgb(136, 192, 208),
            border_unfocused: Color::Rgb(76, 86, 106),
            selection_bg: Color::Rgb(59, 66, 82),
            selection_fg: Color::Rgb(236, 239, 244),
            status_bar_bg: Color::Rgb(59, 66, 82),
            status_bar_fg: Color::Rgb(216, 222, 233),
            inline_code_fg: Color::Rgb(235, 203, 139),
            inline_code_bg: Color::Rgb(59, 66, 82),
            bold_fg: Color::Rgb(236, 239, 244),
            italic_fg: Color::Rgb(180, 142, 173),
            list_bullet: Color::Rgb(136, 192, 208),
            blockquote_border: Color::Rgb(76, 86, 106),
            blockquote_fg: Color::Rgb(76, 86, 106),
            code_fence: Color::Rgb(143, 188, 187),
            title_bar_fg: Color::Rgb(136, 192, 208),
            scrollbar_fg: Color::Rgb(76, 86, 106),
            selection_indicator_fg: Color::Rgb(46, 52, 64),
            selection_indicator_bg: Color::Rgb(136, 192, 208),
            link_fg: Color::Rgb(129, 161, 193),
            link_selected_bg: Color::Rgb(136, 192, 208),
            link_selected_fg: Color::Rgb(46, 52, 64),
            table_border: Color::Rgb(76, 86, 106),
        }
    }

    /// Dracula theme - Dark theme with vibrant colors
    pub fn dracula() -> Self {
        Self {
            name: "Dracula",
            background: Color::Rgb(40, 42, 54),
            foreground: Color::Rgb(248, 248, 242),
            heading_1: Color::Rgb(139, 233, 253), // Cyan
            heading_2: Color::Rgb(80, 250, 123),  // Green
            heading_3: Color::Rgb(255, 184, 108), // Orange
            heading_4: Color::Rgb(255, 121, 198), // Pink
            heading_5: Color::Rgb(189, 147, 249), // Purple
            border_focused: Color::Rgb(189, 147, 249),
            border_unfocused: Color::Rgb(68, 71, 90),
            selection_bg: Color::Rgb(68, 71, 90),
            selection_fg: Color::Rgb(248, 248, 242),
            status_bar_bg: Color::Rgb(68, 71, 90),
            status_bar_fg: Color::Rgb(248, 248, 242),
            inline_code_fg: Color::Rgb(241, 250, 140),
            inline_code_bg: Color::Rgb(68, 71, 90),
            bold_fg: Color::Rgb(255, 255, 255),
            italic_fg: Color::Rgb(189, 147, 249),
            list_bullet: Color::Rgb(139, 233, 253),
            blockquote_border: Color::Rgb(98, 114, 164),
            blockquote_fg: Color::Rgb(98, 114, 164),
            code_fence: Color::Rgb(189, 147, 249),
            title_bar_fg: Color::Rgb(139, 233, 253),
            scrollbar_fg: Color::Rgb(68, 71, 90),
            selection_indicator_fg: Color::Rgb(40, 42, 54),
            selection_indicator_bg: Color::Rgb(139, 233, 253),
            link_fg: Color::Rgb(139, 233, 253),
            link_selected_bg: Color::Rgb(139, 233, 253),
            link_selected_fg: Color::Rgb(40, 42, 54),
            table_border: Color::Rgb(98, 114, 164),
        }
    }

    /// Solarized Dark - Precision colors for machines and people
    pub fn solarized() -> Self {
        Self {
            name: "Solarized",
            background: Color::Rgb(0, 43, 54),
            foreground: Color::Rgb(131, 148, 150),
            heading_1: Color::Rgb(38, 139, 210), // Blue
            heading_2: Color::Rgb(42, 161, 152), // Cyan
            heading_3: Color::Rgb(133, 153, 0),  // Green
            heading_4: Color::Rgb(181, 137, 0),  // Yellow
            heading_5: Color::Rgb(203, 75, 22),  // Orange
            border_focused: Color::Rgb(38, 139, 210),
            border_unfocused: Color::Rgb(7, 54, 66),
            selection_bg: Color::Rgb(7, 54, 66),
            selection_fg: Color::Rgb(147, 161, 161),
            status_bar_bg: Color::Rgb(7, 54, 66),
            status_bar_fg: Color::Rgb(131, 148, 150),
            inline_code_fg: Color::Rgb(181, 137, 0),
            inline_code_bg: Color::Rgb(7, 54, 66),
            bold_fg: Color::Rgb(147, 161, 161),
            italic_fg: Color::Rgb(108, 113, 196),
            list_bullet: Color::Rgb(42, 161, 152),
            blockquote_border: Color::Rgb(88, 110, 117),
            blockquote_fg: Color::Rgb(88, 110, 117),
            code_fence: Color::Rgb(42, 161, 152),
            title_bar_fg: Color::Rgb(38, 139, 210),
            scrollbar_fg: Color::Rgb(88, 110, 117),
            selection_indicator_fg: Color::Rgb(0, 43, 54),
            selection_indicator_bg: Color::Rgb(38, 139, 210),
            link_fg: Color::Rgb(38, 139, 210),
            link_selected_bg: Color::Rgb(38, 139, 210),
            link_selected_fg: Color::Rgb(0, 43, 54),
            table_border: Color::Rgb(88, 110, 117),
        }
    }

    /// Monokai - Sublime Text's iconic color scheme
    pub fn monokai() -> Self {
        Self {
            name: "Monokai",
            background: Color::Rgb(39, 40, 34),
            foreground: Color::Rgb(248, 248, 242),
            heading_1: Color::Rgb(102, 217, 239), // Cyan
            heading_2: Color::Rgb(166, 226, 46),  // Green
            heading_3: Color::Rgb(253, 151, 31),  // Orange
            heading_4: Color::Rgb(249, 38, 114),  // Pink
            heading_5: Color::Rgb(174, 129, 255), // Purple
            border_focused: Color::Rgb(102, 217, 239),
            border_unfocused: Color::Rgb(73, 72, 62),
            selection_bg: Color::Rgb(73, 72, 62),
            selection_fg: Color::Rgb(248, 248, 242),
            status_bar_bg: Color::Rgb(73, 72, 62),
            status_bar_fg: Color::Rgb(248, 248, 242),
            inline_code_fg: Color::Rgb(230, 219, 116),
            inline_code_bg: Color::Rgb(73, 72, 62),
            bold_fg: Color::Rgb(255, 255, 255),
            italic_fg: Color::Rgb(102, 217, 239),
            list_bullet: Color::Rgb(102, 217, 239),
            blockquote_border: Color::Rgb(117, 113, 94),
            blockquote_fg: Color::Rgb(117, 113, 94),
            code_fence: Color::Rgb(102, 217, 239),
            title_bar_fg: Color::Rgb(102, 217, 239),
            scrollbar_fg: Color::Rgb(117, 113, 94),
            selection_indicator_fg: Color::Rgb(39, 40, 34),
            selection_indicator_bg: Color::Rgb(102, 217, 239),
            link_fg: Color::Rgb(102, 217, 239),
            link_selected_bg: Color::Rgb(102, 217, 239),
            link_selected_fg: Color::Rgb(39, 40, 34),
            table_border: Color::Rgb(117, 113, 94),
        }
    }

    /// Gruvbox Dark - Retro groove color scheme
    pub fn gruvbox() -> Self {
        Self {
            name: "Gruvbox",
            background: Color::Rgb(40, 40, 40),
            foreground: Color::Rgb(235, 219, 178),
            heading_1: Color::Rgb(131, 165, 152), // Aqua
            heading_2: Color::Rgb(184, 187, 38),  // Green
            heading_3: Color::Rgb(250, 189, 47),  // Yellow
            heading_4: Color::Rgb(254, 128, 25),  // Orange
            heading_5: Color::Rgb(211, 134, 155), // Purple
            border_focused: Color::Rgb(184, 187, 38),
            border_unfocused: Color::Rgb(60, 56, 54),
            selection_bg: Color::Rgb(60, 56, 54),
            selection_fg: Color::Rgb(235, 219, 178),
            status_bar_bg: Color::Rgb(60, 56, 54),
            status_bar_fg: Color::Rgb(235, 219, 178),
            inline_code_fg: Color::Rgb(250, 189, 47),
            inline_code_bg: Color::Rgb(60, 56, 54),
            bold_fg: Color::Rgb(251, 241, 199),
            italic_fg: Color::Rgb(211, 134, 155),
            list_bullet: Color::Rgb(131, 165, 152),
            blockquote_border: Color::Rgb(146, 131, 116),
            blockquote_fg: Color::Rgb(146, 131, 116),
            code_fence: Color::Rgb(131, 165, 152),
            title_bar_fg: Color::Rgb(131, 165, 152),
            scrollbar_fg: Color::Rgb(146, 131, 116),
            selection_indicator_fg: Color::Rgb(40, 40, 40),
            selection_indicator_bg: Color::Rgb(131, 165, 152),
            link_fg: Color::Rgb(131, 165, 152),
            link_selected_bg: Color::Rgb(131, 165, 152),
            link_selected_fg: Color::Rgb(40, 40, 40),
            table_border: Color::Rgb(146, 131, 116),
        }
    }

    /// Tokyo Night - Modern dark theme celebrating Tokyo's neon lights at night
    pub fn tokyo_night() -> Self {
        Self {
            name: "Tokyo Night",
            background: Color::Rgb(26, 27, 38), // Very dark blue-black
            foreground: Color::Rgb(192, 202, 245), // Soft blue-white
            heading_1: Color::Rgb(122, 162, 247), // Blue
            heading_2: Color::Rgb(125, 207, 255), // Cyan
            heading_3: Color::Rgb(158, 206, 106), // Green
            heading_4: Color::Rgb(224, 175, 104), // Yellow
            heading_5: Color::Rgb(187, 154, 247), // Purple
            border_focused: Color::Rgb(122, 162, 247),
            border_unfocused: Color::Rgb(41, 46, 66),
            selection_bg: Color::Rgb(41, 46, 66),
            selection_fg: Color::Rgb(192, 202, 245),
            status_bar_bg: Color::Rgb(31, 35, 53),
            status_bar_fg: Color::Rgb(192, 202, 245),
            inline_code_fg: Color::Rgb(255, 158, 100), // Orange
            inline_code_bg: Color::Rgb(41, 46, 66),
            bold_fg: Color::Rgb(255, 255, 255),
            italic_fg: Color::Rgb(187, 154, 247),       // Purple
            list_bullet: Color::Rgb(125, 207, 255),     // Cyan
            blockquote_border: Color::Rgb(86, 95, 137), // Comment
            blockquote_fg: Color::Rgb(169, 177, 214),   // Fg dark
            code_fence: Color::Rgb(125, 207, 255),      // Cyan
            title_bar_fg: Color::Rgb(122, 162, 247),
            scrollbar_fg: Color::Rgb(86, 95, 137),
            selection_indicator_fg: Color::Rgb(26, 27, 38),
            selection_indicator_bg: Color::Rgb(122, 162, 247),
            link_fg: Color::Rgb(122, 162, 247),
            link_selected_bg: Color::Rgb(122, 162, 247),
            link_selected_fg: Color::Rgb(26, 27, 38),
            table_border: Color::Rgb(86, 95, 137),
        }
    }

    /// Catppuccin Mocha - Soothing pastel theme for cozy night coding
    pub fn catppuccin_mocha() -> Self {
        Self {
            name: "Catppuccin Mocha",
            background: Color::Rgb(30, 30, 46),    // Base
            foreground: Color::Rgb(205, 214, 244), // Text
            heading_1: Color::Rgb(137, 180, 250),  // Blue
            heading_2: Color::Rgb(137, 220, 235),  // Sky
            heading_3: Color::Rgb(166, 227, 161),  // Green
            heading_4: Color::Rgb(249, 226, 175),  // Yellow
            heading_5: Color::Rgb(203, 166, 247),  // Mauve
            border_focused: Color::Rgb(137, 180, 250),
            border_unfocused: Color::Rgb(69, 71, 90), // Surface 1
            selection_bg: Color::Rgb(69, 71, 90),     // Surface 1
            selection_fg: Color::Rgb(205, 214, 244),  // Text
            status_bar_bg: Color::Rgb(24, 24, 37),    // Mantle
            status_bar_fg: Color::Rgb(205, 214, 244), // Text
            inline_code_fg: Color::Rgb(250, 179, 135), // Peach
            inline_code_bg: Color::Rgb(49, 50, 68),   // Surface 0
            bold_fg: Color::Rgb(255, 255, 255),
            italic_fg: Color::Rgb(245, 194, 231),         // Pink
            list_bullet: Color::Rgb(148, 226, 213),       // Teal
            blockquote_border: Color::Rgb(108, 112, 134), // Overlay 0
            blockquote_fg: Color::Rgb(147, 153, 178),     // Overlay 2
            code_fence: Color::Rgb(116, 199, 236),        // Sapphire
            title_bar_fg: Color::Rgb(137, 180, 250),      // Blue
            scrollbar_fg: Color::Rgb(108, 112, 134),      // Overlay 0
            selection_indicator_fg: Color::Rgb(30, 30, 46), // Base
            selection_indicator_bg: Color::Rgb(137, 180, 250), // Blue
            link_fg: Color::Rgb(137, 180, 250),           // Blue
            link_selected_bg: Color::Rgb(137, 180, 250),  // Blue
            link_selected_fg: Color::Rgb(30, 30, 46),     // Base
            table_border: Color::Rgb(108, 112, 134),      // Overlay 0
        }
    }

    // ========== 256-Color Optimized Variants ==========

    /// Ocean Dark - 256-color optimized variant
    pub fn ocean_dark_256() -> Self {
        Self {
            name: "Ocean Dark",
            background: Color::Indexed(236), // ~(43, 48, 59)
            foreground: Color::Indexed(188), // ~(192, 197, 206)
            heading_1: Color::Indexed(117),  // Bright blue
            heading_2: Color::Indexed(153),  // Light blue
            heading_3: Color::Indexed(121),  // Cyan-green
            heading_4: Color::Indexed(192),  // Light green-yellow
            heading_5: Color::Indexed(250),  // Light gray
            border_focused: Color::Cyan,
            border_unfocused: Color::DarkGray,
            selection_bg: Color::Indexed(237),
            selection_fg: Color::White,
            status_bar_bg: Color::Indexed(238),
            status_bar_fg: Color::Indexed(188),
            inline_code_fg: Color::Indexed(222), // Light orange
            inline_code_bg: Color::Indexed(235),
            bold_fg: Color::White,
            italic_fg: Color::Indexed(177), // Light purple
            list_bullet: Color::Cyan,
            blockquote_border: Color::Indexed(246),
            blockquote_fg: Color::Indexed(246),
            code_fence: Color::Indexed(152),
            title_bar_fg: Color::Indexed(117),
            scrollbar_fg: Color::Indexed(240),
            selection_indicator_fg: Color::Indexed(236),
            selection_indicator_bg: Color::Indexed(117),
            link_fg: Color::Indexed(111),
            link_selected_bg: Color::Indexed(117),
            link_selected_fg: Color::Indexed(236),
            table_border: Color::Indexed(241),
        }
    }

    /// Nord - 256-color optimized variant based on official Nord palette
    pub fn nord_256() -> Self {
        Self {
            name: "Nord",
            background: Color::Indexed(236), // nord0 approximation
            foreground: Color::Indexed(252), // nord4 approximation
            heading_1: Color::Indexed(109),  // nord8 Frost cyan
            heading_2: Color::Indexed(109),  // nord7 Frost teal
            heading_3: Color::Indexed(150),  // nord14 Aurora green
            heading_4: Color::Indexed(222),  // nord13 Aurora yellow
            heading_5: Color::Indexed(139),  // nord15 Aurora purple
            border_focused: Color::Indexed(109), // Frost cyan
            border_unfocused: Color::Indexed(238),
            selection_bg: Color::Indexed(238),
            selection_fg: Color::Indexed(253),
            status_bar_bg: Color::Indexed(238),
            status_bar_fg: Color::Indexed(252),
            inline_code_fg: Color::Indexed(222), // Aurora yellow
            inline_code_bg: Color::Indexed(238),
            bold_fg: Color::Indexed(253),
            italic_fg: Color::Indexed(139),   // Aurora purple
            list_bullet: Color::Indexed(109), // Frost cyan
            blockquote_border: Color::Indexed(240),
            blockquote_fg: Color::Indexed(240),
            code_fence: Color::Indexed(109),
            title_bar_fg: Color::Indexed(109),
            scrollbar_fg: Color::Indexed(240),
            selection_indicator_fg: Color::Indexed(236),
            selection_indicator_bg: Color::Indexed(109),
            link_fg: Color::Indexed(110),
            link_selected_bg: Color::Indexed(109),
            link_selected_fg: Color::Indexed(236),
            table_border: Color::Indexed(240),
        }
    }

    /// Dracula - 256-color optimized variant based on official palette
    pub fn dracula_256() -> Self {
        Self {
            name: "Dracula",
            background: Color::Indexed(236),     // Background
            foreground: Color::Indexed(231),     // Foreground
            heading_1: Color::Indexed(117),      // Cyan
            heading_2: Color::Indexed(84),       // Green
            heading_3: Color::Indexed(215),      // Orange
            heading_4: Color::Indexed(212),      // Pink
            heading_5: Color::Indexed(141),      // Purple
            border_focused: Color::Indexed(141), // Purple
            border_unfocused: Color::Indexed(238),
            selection_bg: Color::Indexed(238),
            selection_fg: Color::Indexed(231),
            status_bar_bg: Color::Indexed(238),
            status_bar_fg: Color::Indexed(231),
            inline_code_fg: Color::Indexed(228), // Yellow
            inline_code_bg: Color::Indexed(238),
            bold_fg: Color::White,
            italic_fg: Color::Indexed(141),   // Purple
            list_bullet: Color::Indexed(117), // Cyan
            blockquote_border: Color::Indexed(61),
            blockquote_fg: Color::Indexed(61),
            code_fence: Color::Indexed(141), // Purple
            title_bar_fg: Color::Indexed(117),
            scrollbar_fg: Color::Indexed(238),
            selection_indicator_fg: Color::Indexed(236),
            selection_indicator_bg: Color::Indexed(117),
            link_fg: Color::Indexed(117),
            link_selected_bg: Color::Indexed(117),
            link_selected_fg: Color::Indexed(236),
            table_border: Color::Indexed(61),
        }
    }

    /// Solarized - 256-color degraded variant
    pub fn solarized_256() -> Self {
        Self {
            name: "Solarized",
            background: Color::Indexed(234),    // Base03
            foreground: Color::Indexed(244),    // Base0
            heading_1: Color::Indexed(33),      // Blue
            heading_2: Color::Indexed(37),      // Cyan
            heading_3: Color::Indexed(64),      // Green
            heading_4: Color::Indexed(136),     // Yellow
            heading_5: Color::Indexed(166),     // Orange
            border_focused: Color::Indexed(33), // Blue
            border_unfocused: Color::Indexed(235),
            selection_bg: Color::Indexed(235), // Base02
            selection_fg: Color::Indexed(246), // Base1
            status_bar_bg: Color::Indexed(235),
            status_bar_fg: Color::Indexed(244),
            inline_code_fg: Color::Indexed(136), // Yellow
            inline_code_bg: Color::Indexed(235),
            bold_fg: Color::Indexed(246),
            italic_fg: Color::Indexed(61),   // Violet
            list_bullet: Color::Indexed(37), // Cyan
            blockquote_border: Color::Indexed(240),
            blockquote_fg: Color::Indexed(240),
            code_fence: Color::Indexed(37), // Cyan
            title_bar_fg: Color::Indexed(33),
            scrollbar_fg: Color::Indexed(240),
            selection_indicator_fg: Color::Indexed(234),
            selection_indicator_bg: Color::Indexed(33),
            link_fg: Color::Indexed(33),
            link_selected_bg: Color::Indexed(33),
            link_selected_fg: Color::Indexed(234),
            table_border: Color::Indexed(240),
        }
    }

    /// Monokai - 256-color optimized variant
    pub fn monokai_256() -> Self {
        Self {
            name: "Monokai",
            background: Color::Indexed(235),    // ~(39, 40, 34)
            foreground: Color::Indexed(231),    // ~(248, 248, 242)
            heading_1: Color::Indexed(81),      // Cyan
            heading_2: Color::Indexed(148),     // Green
            heading_3: Color::Indexed(208),     // Orange
            heading_4: Color::Indexed(197),     // Pink
            heading_5: Color::Indexed(141),     // Purple
            border_focused: Color::Indexed(81), // Cyan
            border_unfocused: Color::Indexed(237),
            selection_bg: Color::Indexed(237),
            selection_fg: Color::Indexed(231),
            status_bar_bg: Color::Indexed(237),
            status_bar_fg: Color::Indexed(231),
            inline_code_fg: Color::Indexed(186), // Yellow
            inline_code_bg: Color::Indexed(237),
            bold_fg: Color::White,
            italic_fg: Color::Indexed(81),   // Cyan
            list_bullet: Color::Indexed(81), // Cyan
            blockquote_border: Color::Indexed(241),
            blockquote_fg: Color::Indexed(241),
            code_fence: Color::Indexed(81), // Cyan
            title_bar_fg: Color::Indexed(81),
            scrollbar_fg: Color::Indexed(241),
            selection_indicator_fg: Color::Indexed(235),
            selection_indicator_bg: Color::Indexed(81),
            link_fg: Color::Indexed(81),
            link_selected_bg: Color::Indexed(81),
            link_selected_fg: Color::Indexed(235),
            table_border: Color::Indexed(241),
        }
    }

    /// Gruvbox - 256-color optimized variant (already looks good, refined further)
    pub fn gruvbox_256() -> Self {
        Self {
            name: "Gruvbox",
            background: Color::Indexed(235),     // Dark background
            foreground: Color::Indexed(223),     // ~(235, 219, 178)
            heading_1: Color::Indexed(108),      // Aqua
            heading_2: Color::Indexed(142),      // Green
            heading_3: Color::Indexed(214),      // Yellow
            heading_4: Color::Indexed(208),      // Orange
            heading_5: Color::Indexed(175),      // Purple
            border_focused: Color::Indexed(142), // Green
            border_unfocused: Color::Indexed(237),
            selection_bg: Color::Indexed(237),
            selection_fg: Color::Indexed(223),
            status_bar_bg: Color::Indexed(237),
            status_bar_fg: Color::Indexed(223),
            inline_code_fg: Color::Indexed(214), // Yellow
            inline_code_bg: Color::Indexed(237),
            bold_fg: Color::Indexed(229),     // Light
            italic_fg: Color::Indexed(175),   // Purple
            list_bullet: Color::Indexed(108), // Aqua
            blockquote_border: Color::Indexed(243),
            blockquote_fg: Color::Indexed(243),
            code_fence: Color::Indexed(108), // Aqua
            title_bar_fg: Color::Indexed(108),
            scrollbar_fg: Color::Indexed(243),
            selection_indicator_fg: Color::Indexed(235),
            selection_indicator_bg: Color::Indexed(108),
            link_fg: Color::Indexed(108),
            link_selected_bg: Color::Indexed(108),
            link_selected_fg: Color::Indexed(235),
            table_border: Color::Indexed(243),
        }
    }

    /// Tokyo Night - 256-color optimized variant
    pub fn tokyo_night_256() -> Self {
        Self {
            name: "Tokyo Night",
            background: Color::Indexed(234), // Very dark blue-black
            foreground: Color::Indexed(189), // Soft blue-white
            heading_1: Color::Indexed(110),  // Blue
            heading_2: Color::Indexed(117),  // Bright cyan
            heading_3: Color::Indexed(150),  // Green
            heading_4: Color::Indexed(179),  // Yellow
            heading_5: Color::Indexed(141),  // Purple
            border_focused: Color::Indexed(110), // Blue
            border_unfocused: Color::Indexed(237),
            selection_bg: Color::Indexed(237),
            selection_fg: Color::Indexed(189),
            status_bar_bg: Color::Indexed(236),
            status_bar_fg: Color::Indexed(189),
            inline_code_fg: Color::Indexed(215), // Orange
            inline_code_bg: Color::Indexed(237),
            bold_fg: Color::White,
            italic_fg: Color::Indexed(141),   // Purple
            list_bullet: Color::Indexed(117), // Cyan
            blockquote_border: Color::Indexed(243),
            blockquote_fg: Color::Indexed(189),
            code_fence: Color::Indexed(117), // Cyan
            title_bar_fg: Color::Indexed(110),
            scrollbar_fg: Color::Indexed(243),
            selection_indicator_fg: Color::Indexed(234),
            selection_indicator_bg: Color::Indexed(110),
            link_fg: Color::Indexed(110),
            link_selected_bg: Color::Indexed(110),
            link_selected_fg: Color::Indexed(234),
            table_border: Color::Indexed(243),
        }
    }

    /// Catppuccin Mocha - 256-color optimized variant
    pub fn catppuccin_mocha_256() -> Self {
        Self {
            name: "Catppuccin Mocha",
            background: Color::Indexed(235),     // Base
            foreground: Color::Indexed(189),     // Text
            heading_1: Color::Indexed(117),      // Blue
            heading_2: Color::Indexed(153),      // Sky
            heading_3: Color::Indexed(151),      // Green
            heading_4: Color::Indexed(223),      // Yellow
            heading_5: Color::Indexed(183),      // Mauve
            border_focused: Color::Indexed(117), // Blue
            border_unfocused: Color::Indexed(238),
            selection_bg: Color::Indexed(238),
            selection_fg: Color::Indexed(189),
            status_bar_bg: Color::Indexed(234), // Mantle
            status_bar_fg: Color::Indexed(189),
            inline_code_fg: Color::Indexed(216), // Peach
            inline_code_bg: Color::Indexed(237),
            bold_fg: Color::White,
            italic_fg: Color::Indexed(218),   // Pink
            list_bullet: Color::Indexed(116), // Teal
            blockquote_border: Color::Indexed(242),
            blockquote_fg: Color::Indexed(245),
            code_fence: Color::Indexed(116), // Sapphire
            title_bar_fg: Color::Indexed(117),
            scrollbar_fg: Color::Indexed(242),
            selection_indicator_fg: Color::Indexed(235),
            selection_indicator_bg: Color::Indexed(117),
            link_fg: Color::Indexed(117),
            link_selected_bg: Color::Indexed(117),
            link_selected_fg: Color::Indexed(235),
            table_border: Color::Indexed(242),
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

    pub fn selection_style(&self) -> Style {
        Style::default()
            .bg(self.selection_bg)
            .fg(self.selection_fg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn status_bar_style(&self) -> Style {
        Style::default()
            .bg(self.status_bar_bg)
            .fg(self.status_bar_fg)
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

    pub fn modal_border(&self) -> Color {
        self.border_focused
    }

    pub fn modal_title(&self) -> Color {
        self.border_focused
    }

    pub fn modal_text(&self) -> Color {
        self.foreground
    }

    pub fn modal_selected_fg(&self) -> Color {
        self.border_focused
    }

    pub fn modal_selected_marker(&self) -> Color {
        self.list_bullet
    }

    pub fn modal_key_fg(&self) -> Color {
        self.heading_3
    }

    pub fn modal_description(&self) -> Color {
        self.blockquote_fg
    }

    /// Apply custom color overrides from config
    pub fn with_custom_colors(
        mut self,
        custom: &crate::config::CustomThemeConfig,
        mode: ColorMode,
    ) -> Self {
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

        self
    }

    /// Apply color mode to theme (use optimized 256-color variants or convert RGB)
    pub fn with_color_mode(self, mode: ColorMode, theme_name: ThemeName) -> Self {
        match mode {
            ColorMode::Rgb => self,
            ColorMode::Indexed256 => {
                // Use optimized 256-color variants for built-in themes
                Theme::from_name_256(theme_name)
            }
        }
    }

    /// Apply color mode to custom theme (convert RGB to 256-color if needed)
    /// This is used for custom themes that don't have optimized variants
    pub fn with_color_mode_custom(mut self, mode: ColorMode) -> Self {
        match mode {
            ColorMode::Rgb => self,
            ColorMode::Indexed256 => {
                self.background = rgb_to_256(self.background);
                self.foreground = rgb_to_256(self.foreground);
                self.heading_1 = rgb_to_256(self.heading_1);
                self.heading_2 = rgb_to_256(self.heading_2);
                self.heading_3 = rgb_to_256(self.heading_3);
                self.heading_4 = rgb_to_256(self.heading_4);
                self.heading_5 = rgb_to_256(self.heading_5);
                self.border_focused = rgb_to_256(self.border_focused);
                self.border_unfocused = rgb_to_256(self.border_unfocused);
                self.selection_bg = rgb_to_256(self.selection_bg);
                self.selection_fg = rgb_to_256(self.selection_fg);
                self.status_bar_bg = rgb_to_256(self.status_bar_bg);
                self.status_bar_fg = rgb_to_256(self.status_bar_fg);
                self.inline_code_fg = rgb_to_256(self.inline_code_fg);
                self.inline_code_bg = rgb_to_256(self.inline_code_bg);
                self.bold_fg = rgb_to_256(self.bold_fg);
                self.italic_fg = rgb_to_256(self.italic_fg);
                self.list_bullet = rgb_to_256(self.list_bullet);
                self.blockquote_border = rgb_to_256(self.blockquote_border);
                self.blockquote_fg = rgb_to_256(self.blockquote_fg);
                self.code_fence = rgb_to_256(self.code_fence);
                self.title_bar_fg = rgb_to_256(self.title_bar_fg);
                self.scrollbar_fg = rgb_to_256(self.scrollbar_fg);
                self.selection_indicator_fg = rgb_to_256(self.selection_indicator_fg);
                self.selection_indicator_bg = rgb_to_256(self.selection_indicator_bg);
                self.link_fg = rgb_to_256(self.link_fg);
                self.link_selected_bg = rgb_to_256(self.link_selected_bg);
                self.link_selected_fg = rgb_to_256(self.link_selected_fg);
                self.table_border = rgb_to_256(self.table_border);
                self
            }
        }
    }
}

/// Convert RGB color to nearest 256-color palette entry
fn rgb_to_256(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            // Check if it's grayscale
            if r == g && g == b {
                // Map to grayscale ramp (232-255)
                if r < 8 {
                    return Color::Indexed(16); // Black
                }
                if r > 247 {
                    return Color::Indexed(231); // White
                }
                let gray_index = ((r as f32 - 8.0) / 10.0).round() as u8;
                return Color::Indexed(232 + gray_index);
            }

            // Map to 6x6x6 RGB cube (16-231)
            let r_index = (r as f32 / 51.0).round() as u8;
            let g_index = (g as f32 / 51.0).round() as u8;
            let b_index = (b as f32 / 51.0).round() as u8;

            Color::Indexed(16 + 36 * r_index + 6 * g_index + b_index)
        }
        // Already indexed or named color - pass through
        other => other,
    }
}
