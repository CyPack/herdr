use crate::config::{Keybinds, NewTerminalCwdConfig, SoundConfig, ToastConfig, ToastDelivery};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Direction, Rect};
use ratatui::style::Color;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use crate::detect::AgentState;
use crate::layout::{PaneId, PaneInfo, SplitBorder};
use crate::selection::Selection;

pub(crate) type InstalledPluginRegistry =
    std::collections::HashMap<String, crate::api::schema::InstalledPluginInfo>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PluginPaneRecord {
    pub plugin_id: String,
    pub entrypoint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PaneGraphicsLayer {
    pub format: crate::api::schema::PaneGraphicsFormat,
    pub image_width: u32,
    pub image_height: u32,
    pub data: Vec<u8>,
    pub data_fingerprint: u64,
    pub render: crate::api::schema::PaneGraphicsPlacementParams,
}

impl PaneGraphicsLayer {
    pub(crate) fn new(
        format: crate::api::schema::PaneGraphicsFormat,
        image_width: u32,
        image_height: u32,
        data: Vec<u8>,
        render: crate::api::schema::PaneGraphicsPlacementParams,
    ) -> Self {
        let data_fingerprint = pane_graphics_data_fingerprint(&data);
        Self {
            format,
            image_width,
            image_height,
            data,
            data_fingerprint,
            render,
        }
    }
}

fn pane_graphics_data_fingerprint(data: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PopupPaneState {
    pub pane_id: PaneId,
    pub terminal_id: crate::terminal::TerminalId,
    pub width: Option<crate::popup_size::PopupSize>,
    pub height: Option<crate::popup_size::PopupSize>,
}

// ---------------------------------------------------------------------------
// Selection autoscroll types
// ---------------------------------------------------------------------------

/// Direction of automatic scrolling during text selection drag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectionAutoscrollDirection {
    Up,
    Down,
}

/// State for automatic scrolling during text selection drag.
///
/// When the cursor hovers in the 1-row hot zone at the top or bottom edge
/// of a pane (or outside the pane), this struct captures the direction and
/// last known mouse position so a recurring 30ms tick can continue scrolling
/// and extending the selection even when the mouse is not moving.
#[derive(Clone, Debug)]
pub(crate) struct SelectionAutoscroll {
    pub direction: SelectionAutoscrollDirection,
    pub last_mouse_screen_col: u16,
    pub last_mouse_screen_row: u16,
    pub inner_rect: Rect,
}

#[derive(Clone)]
pub(crate) struct RightClickPassthroughGesture {
    pub pane_info: PaneInfo,
    pub modifiers: KeyModifiers,
}
use crate::terminal_theme::{HostAppearance, TerminalTheme};
use crate::workspace::Workspace;

// ---------------------------------------------------------------------------
// Theme palette — all UI colors in one place, ready for theming
// ---------------------------------------------------------------------------

/// All colors used by the UI. Derived from a base accent color for now,
/// but structured so a full theme system can replace it later.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // all fields defined for theming — some used later
pub struct Palette {
    /// Primary accent (highlight, active borders).
    pub accent: Color,
    /// Background for floating panels, overlays, and modals.
    pub panel_bg: Color,
    /// Subtle surface background for selected/focused items.
    pub surface0: Color,
    /// Slightly lighter surface for hover/active states.
    pub surface1: Color,
    /// Very dim surface for separators.
    pub surface_dim: Color,
    /// Muted text (secondary info, numbers).
    pub overlay0: Color,
    /// Slightly brighter overlay text.
    pub overlay1: Color,
    /// Main text color — soft white.
    pub text: Color,
    /// Subdued text (workspace numbers, dim labels).
    pub subtext0: Color,
    /// Branch name / special label color.
    pub mauve: Color,
    /// Done / idle states.
    pub green: Color,
    /// Working / running states.
    pub yellow: Color,
    /// Needs attention / blocked states.
    pub red: Color,
    /// Unseen / done notification accent.
    pub blue: Color,
    /// Notification accent / unseen markers.
    pub teal: Color,
    /// Interrupted / warning states.
    pub peach: Color,
}

impl Palette {
    /// Catppuccin Mocha — the default.
    pub fn catppuccin() -> Self {
        Self {
            accent: Color::Rgb(137, 180, 250), // blue
            panel_bg: Color::Rgb(24, 24, 37),
            surface0: Color::Rgb(49, 50, 68),
            surface1: Color::Rgb(69, 71, 90),
            surface_dim: Color::Rgb(30, 30, 46),
            overlay0: Color::Rgb(108, 112, 134),
            overlay1: Color::Rgb(127, 132, 156),
            text: Color::Rgb(205, 214, 244),
            subtext0: Color::Rgb(166, 173, 200),
            mauve: Color::Rgb(203, 166, 247),
            green: Color::Rgb(166, 227, 161),
            yellow: Color::Rgb(249, 226, 175),
            red: Color::Rgb(243, 139, 168),
            blue: Color::Rgb(137, 180, 250),
            teal: Color::Rgb(148, 226, 213),
            peach: Color::Rgb(250, 179, 135),
        }
    }

    /// Catppuccin Latte — the light Catppuccin flavor.
    pub fn catppuccin_latte() -> Self {
        Self {
            accent: Color::Rgb(30, 102, 245),
            panel_bg: Color::Rgb(239, 241, 245),
            surface0: Color::Rgb(204, 208, 218),
            surface1: Color::Rgb(188, 192, 204),
            surface_dim: Color::Rgb(230, 233, 239),
            overlay0: Color::Rgb(156, 160, 176),
            overlay1: Color::Rgb(140, 143, 161),
            text: Color::Rgb(76, 79, 105),
            subtext0: Color::Rgb(108, 111, 133),
            mauve: Color::Rgb(136, 57, 239),
            green: Color::Rgb(64, 160, 43),
            yellow: Color::Rgb(223, 142, 29),
            red: Color::Rgb(210, 15, 57),
            blue: Color::Rgb(30, 102, 245),
            teal: Color::Rgb(23, 146, 153),
            peach: Color::Rgb(254, 100, 11),
        }
    }

    /// Terminal 16-color theme.
    pub fn terminal() -> Self {
        Self {
            accent: Color::Blue,
            panel_bg: Color::Reset,
            surface0: Color::Reset,
            surface1: Color::DarkGray,
            surface_dim: Color::DarkGray,
            overlay0: Color::Gray,
            overlay1: Color::White,
            text: Color::Reset,
            subtext0: Color::Gray,
            mauve: Color::Gray,
            green: Color::Green,
            yellow: Color::Yellow,
            red: Color::LightRed,
            blue: Color::Blue,
            teal: Color::Cyan,
            peach: Color::Yellow,
        }
    }

    /// Tokyo Night — blue-purple aesthetic.
    pub fn tokyo_night() -> Self {
        Self {
            accent: Color::Rgb(122, 162, 247), // blue
            panel_bg: Color::Rgb(26, 27, 38),
            surface0: Color::Rgb(36, 40, 59),
            surface1: Color::Rgb(65, 72, 104),
            surface_dim: Color::Rgb(26, 27, 38),
            overlay0: Color::Rgb(86, 95, 137),
            overlay1: Color::Rgb(105, 113, 150),
            text: Color::Rgb(192, 202, 245),
            subtext0: Color::Rgb(169, 177, 214),
            mauve: Color::Rgb(187, 154, 247),
            green: Color::Rgb(158, 206, 106),
            yellow: Color::Rgb(224, 175, 104),
            red: Color::Rgb(247, 118, 142),
            blue: Color::Rgb(122, 162, 247),
            teal: Color::Rgb(125, 207, 255),
            peach: Color::Rgb(255, 158, 100),
        }
    }

    /// Tokyo Night Day — the light Tokyo Night style.
    pub fn tokyo_night_day() -> Self {
        Self {
            accent: Color::Rgb(46, 125, 233),
            panel_bg: Color::Rgb(225, 226, 231),
            surface0: Color::Rgb(196, 200, 218),
            surface1: Color::Rgb(168, 174, 203),
            surface_dim: Color::Rgb(210, 211, 218),
            overlay0: Color::Rgb(137, 144, 179),
            overlay1: Color::Rgb(104, 112, 154),
            text: Color::Rgb(55, 96, 191),
            subtext0: Color::Rgb(97, 114, 176),
            mauve: Color::Rgb(120, 71, 189),
            green: Color::Rgb(88, 117, 57),
            yellow: Color::Rgb(140, 108, 62),
            red: Color::Rgb(245, 42, 101),
            blue: Color::Rgb(46, 125, 233),
            teal: Color::Rgb(17, 140, 116),
            peach: Color::Rgb(177, 92, 0),
        }
    }

    /// Dracula — purple/pink/green.
    pub fn dracula() -> Self {
        Self {
            accent: Color::Rgb(189, 147, 249), // purple
            panel_bg: Color::Rgb(40, 42, 54),
            surface0: Color::Rgb(68, 71, 90),
            surface1: Color::Rgb(98, 114, 164),
            surface_dim: Color::Rgb(40, 42, 54),
            overlay0: Color::Rgb(98, 114, 164),
            overlay1: Color::Rgb(130, 140, 180),
            text: Color::Rgb(248, 248, 242),
            subtext0: Color::Rgb(210, 210, 220),
            mauve: Color::Rgb(255, 121, 198), // pink
            green: Color::Rgb(80, 250, 123),
            yellow: Color::Rgb(241, 250, 140),
            red: Color::Rgb(255, 85, 85),
            blue: Color::Rgb(139, 233, 253), // cyan-ish
            teal: Color::Rgb(139, 233, 253),
            peach: Color::Rgb(255, 184, 108),
        }
    }

    /// Nord — frosty blue palette.
    pub fn nord() -> Self {
        Self {
            accent: Color::Rgb(136, 192, 208), // frost
            panel_bg: Color::Rgb(46, 52, 64),
            surface0: Color::Rgb(59, 66, 82),
            surface1: Color::Rgb(67, 76, 94),
            surface_dim: Color::Rgb(46, 52, 64),
            overlay0: Color::Rgb(76, 86, 106),
            overlay1: Color::Rgb(100, 110, 130),
            text: Color::Rgb(236, 239, 244),
            subtext0: Color::Rgb(216, 222, 233),
            mauve: Color::Rgb(180, 142, 173),
            green: Color::Rgb(163, 190, 140),
            yellow: Color::Rgb(235, 203, 139),
            red: Color::Rgb(191, 97, 106),
            blue: Color::Rgb(129, 161, 193),
            teal: Color::Rgb(143, 188, 187),
            peach: Color::Rgb(208, 135, 112),
        }
    }

    /// Gruvbox Dark — warm retro palette.
    pub fn gruvbox() -> Self {
        Self {
            accent: Color::Rgb(215, 153, 33), // yellow
            panel_bg: Color::Rgb(40, 40, 40),
            surface0: Color::Rgb(60, 56, 54),
            surface1: Color::Rgb(80, 73, 69),
            surface_dim: Color::Rgb(40, 40, 40),
            overlay0: Color::Rgb(146, 131, 116),
            overlay1: Color::Rgb(168, 153, 132),
            text: Color::Rgb(235, 219, 178),
            subtext0: Color::Rgb(213, 196, 161),
            mauve: Color::Rgb(211, 134, 155),
            green: Color::Rgb(184, 187, 38),
            yellow: Color::Rgb(250, 189, 47),
            red: Color::Rgb(251, 73, 52),
            blue: Color::Rgb(131, 165, 152),
            teal: Color::Rgb(142, 192, 124),
            peach: Color::Rgb(254, 128, 25),
        }
    }

    /// Gruvbox Light — the light retro palette.
    pub fn gruvbox_light() -> Self {
        Self {
            accent: Color::Rgb(7, 102, 120),
            panel_bg: Color::Rgb(251, 241, 199),
            surface0: Color::Rgb(235, 219, 178),
            surface1: Color::Rgb(213, 196, 161),
            surface_dim: Color::Rgb(242, 229, 188),
            overlay0: Color::Rgb(146, 131, 116),
            overlay1: Color::Rgb(124, 111, 100),
            text: Color::Rgb(60, 56, 54),
            subtext0: Color::Rgb(80, 73, 69),
            mauve: Color::Rgb(143, 63, 113),
            green: Color::Rgb(121, 116, 14),
            yellow: Color::Rgb(181, 118, 20),
            red: Color::Rgb(157, 0, 6),
            blue: Color::Rgb(7, 102, 120),
            teal: Color::Rgb(66, 123, 88),
            peach: Color::Rgb(175, 58, 3),
        }
    }

    /// One Dark — Atom's classic dark theme.
    pub fn one_dark() -> Self {
        Self {
            accent: Color::Rgb(97, 175, 239), // blue
            panel_bg: Color::Rgb(40, 44, 52),
            surface0: Color::Rgb(44, 49, 58),
            surface1: Color::Rgb(62, 68, 81),
            surface_dim: Color::Rgb(40, 44, 52),
            overlay0: Color::Rgb(92, 99, 112),
            overlay1: Color::Rgb(115, 122, 135),
            text: Color::Rgb(171, 178, 191),
            subtext0: Color::Rgb(150, 156, 168),
            mauve: Color::Rgb(198, 120, 221),
            green: Color::Rgb(152, 195, 121),
            yellow: Color::Rgb(229, 192, 123),
            red: Color::Rgb(224, 108, 117),
            blue: Color::Rgb(97, 175, 239),
            teal: Color::Rgb(86, 182, 194),
            peach: Color::Rgb(209, 154, 102),
        }
    }

    /// One Light — Atom's classic light theme.
    pub fn one_light() -> Self {
        Self {
            accent: Color::Rgb(64, 120, 242),
            panel_bg: Color::Rgb(250, 250, 250),
            surface0: Color::Rgb(240, 240, 241),
            surface1: Color::Rgb(229, 229, 230),
            surface_dim: Color::Rgb(245, 245, 246),
            overlay0: Color::Rgb(160, 161, 167),
            overlay1: Color::Rgb(104, 107, 119),
            text: Color::Rgb(56, 58, 66),
            subtext0: Color::Rgb(104, 107, 119),
            mauve: Color::Rgb(166, 38, 164),
            green: Color::Rgb(80, 161, 79),
            yellow: Color::Rgb(193, 132, 1),
            red: Color::Rgb(228, 86, 73),
            blue: Color::Rgb(64, 120, 242),
            teal: Color::Rgb(1, 132, 188),
            peach: Color::Rgb(152, 104, 1),
        }
    }

    /// Solarized Dark — Ethan Schoonover's classic.
    pub fn solarized() -> Self {
        Self {
            accent: Color::Rgb(38, 139, 210), // blue
            panel_bg: Color::Rgb(0, 43, 54),
            surface0: Color::Rgb(7, 54, 66),
            surface1: Color::Rgb(88, 110, 117),
            surface_dim: Color::Rgb(0, 43, 54),
            overlay0: Color::Rgb(88, 110, 117),
            overlay1: Color::Rgb(101, 123, 131),
            text: Color::Rgb(147, 161, 161),
            subtext0: Color::Rgb(131, 148, 150),
            mauve: Color::Rgb(211, 54, 130),
            green: Color::Rgb(133, 153, 0),
            yellow: Color::Rgb(181, 137, 0),
            red: Color::Rgb(220, 50, 47),
            blue: Color::Rgb(38, 139, 210),
            teal: Color::Rgb(42, 161, 152),
            peach: Color::Rgb(203, 75, 22),
        }
    }

    /// Solarized Light — Ethan Schoonover's light variant.
    pub fn solarized_light() -> Self {
        Self {
            accent: Color::Rgb(38, 139, 210),
            panel_bg: Color::Rgb(253, 246, 227),
            surface0: Color::Rgb(238, 232, 213),
            surface1: Color::Rgb(147, 161, 161),
            surface_dim: Color::Rgb(238, 232, 213),
            overlay0: Color::Rgb(147, 161, 161),
            overlay1: Color::Rgb(88, 110, 117),
            text: Color::Rgb(101, 123, 131),
            subtext0: Color::Rgb(131, 148, 150),
            mauve: Color::Rgb(211, 54, 130),
            green: Color::Rgb(133, 153, 0),
            yellow: Color::Rgb(181, 137, 0),
            red: Color::Rgb(220, 50, 47),
            blue: Color::Rgb(38, 139, 210),
            teal: Color::Rgb(42, 161, 152),
            peach: Color::Rgb(203, 75, 22),
        }
    }

    /// Kanagawa — inspired by Katsushika Hokusai.
    pub fn kanagawa() -> Self {
        Self {
            accent: Color::Rgb(126, 156, 216), // blue
            panel_bg: Color::Rgb(31, 31, 40),
            surface0: Color::Rgb(42, 42, 55),
            surface1: Color::Rgb(54, 54, 70),
            surface_dim: Color::Rgb(31, 31, 40),
            overlay0: Color::Rgb(114, 113, 105),
            overlay1: Color::Rgb(135, 134, 125),
            text: Color::Rgb(220, 215, 186),
            subtext0: Color::Rgb(200, 195, 170),
            mauve: Color::Rgb(149, 127, 184),
            green: Color::Rgb(118, 148, 106),
            yellow: Color::Rgb(192, 163, 110),
            red: Color::Rgb(195, 64, 67),
            blue: Color::Rgb(126, 156, 216),
            teal: Color::Rgb(127, 180, 202),
            peach: Color::Rgb(255, 160, 102),
        }
    }

    /// Kanagawa Lotus — the light Kanagawa variant.
    pub fn kanagawa_lotus() -> Self {
        Self {
            accent: Color::Rgb(77, 105, 155),
            panel_bg: Color::Rgb(242, 236, 188),
            surface0: Color::Rgb(220, 213, 172),
            surface1: Color::Rgb(201, 203, 209),
            surface_dim: Color::Rgb(213, 206, 163),
            overlay0: Color::Rgb(160, 156, 172),
            overlay1: Color::Rgb(138, 137, 128),
            text: Color::Rgb(84, 84, 100),
            subtext0: Color::Rgb(67, 67, 108),
            mauve: Color::Rgb(98, 76, 131),
            green: Color::Rgb(111, 137, 78),
            yellow: Color::Rgb(119, 113, 63),
            red: Color::Rgb(200, 64, 83),
            blue: Color::Rgb(77, 105, 155),
            teal: Color::Rgb(78, 140, 162),
            peach: Color::Rgb(204, 109, 0),
        }
    }

    /// Rosé Pine — muted, elegant.
    pub fn rose_pine() -> Self {
        Self {
            accent: Color::Rgb(196, 167, 231), // iris
            panel_bg: Color::Rgb(25, 23, 36),
            surface0: Color::Rgb(31, 29, 46),
            surface1: Color::Rgb(38, 35, 58),
            surface_dim: Color::Rgb(25, 23, 36),
            overlay0: Color::Rgb(110, 106, 134),
            overlay1: Color::Rgb(144, 140, 170),
            text: Color::Rgb(224, 222, 244),
            subtext0: Color::Rgb(200, 197, 220),
            mauve: Color::Rgb(196, 167, 231),  // iris
            green: Color::Rgb(49, 116, 143),   // pine
            yellow: Color::Rgb(246, 193, 119), // gold
            red: Color::Rgb(235, 111, 146),    // love
            blue: Color::Rgb(49, 116, 143),    // pine
            teal: Color::Rgb(156, 207, 216),   // foam
            peach: Color::Rgb(234, 154, 151),  // rose
        }
    }

    /// Rosé Pine Dawn — the light Rosé Pine variant.
    pub fn rose_pine_dawn() -> Self {
        Self {
            accent: Color::Rgb(144, 122, 169),
            panel_bg: Color::Rgb(250, 244, 237),
            surface0: Color::Rgb(242, 233, 225),
            surface1: Color::Rgb(255, 250, 243),
            surface_dim: Color::Rgb(242, 233, 225),
            overlay0: Color::Rgb(152, 147, 165),
            overlay1: Color::Rgb(121, 117, 147),
            text: Color::Rgb(70, 66, 97),
            subtext0: Color::Rgb(121, 117, 147),
            mauve: Color::Rgb(144, 122, 169),
            green: Color::Rgb(40, 105, 131),
            yellow: Color::Rgb(234, 157, 52),
            red: Color::Rgb(180, 99, 122),
            blue: Color::Rgb(40, 105, 131),
            teal: Color::Rgb(86, 148, 159),
            peach: Color::Rgb(215, 130, 126),
        }
    }

    /// Vesper — minimal high-contrast monochrome with peach and mint accents.
    pub fn vesper() -> Self {
        Self {
            accent: Color::Rgb(255, 199, 153),
            panel_bg: Color::Rgb(26, 26, 26),
            surface0: Color::Rgb(35, 35, 35),
            surface1: Color::Rgb(40, 40, 40),
            surface_dim: Color::Rgb(16, 16, 16),
            overlay0: Color::Rgb(92, 92, 92),
            overlay1: Color::Rgb(126, 126, 126),
            text: Color::Rgb(255, 255, 255),
            subtext0: Color::Rgb(160, 160, 160),
            mauve: Color::Rgb(255, 209, 168),
            green: Color::Rgb(153, 255, 228),
            yellow: Color::Rgb(255, 199, 153),
            red: Color::Rgb(255, 128, 128),
            blue: Color::Rgb(176, 176, 176),
            teal: Color::Rgb(102, 221, 204),
            peach: Color::Rgb(255, 199, 153),
        }
    }

    /// Resolve a theme by name. Returns None for unknown names.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().replace([' ', '_'], "-").as_str() {
            "catppuccin" | "catppuccin-mocha" => Some(Self::catppuccin()),
            "catppuccin-latte" | "latte" | "light" => Some(Self::catppuccin_latte()),
            "terminal" => Some(Self::terminal()),
            "tokyo-night" | "tokyonight" => Some(Self::tokyo_night()),
            "tokyo-night-day" | "tokyo-day" | "tokyonight-day" => Some(Self::tokyo_night_day()),
            "dracula" => Some(Self::dracula()),
            "nord" => Some(Self::nord()),
            "gruvbox" | "gruvbox-dark" => Some(Self::gruvbox()),
            "gruvbox-light" => Some(Self::gruvbox_light()),
            "one-dark" | "onedark" => Some(Self::one_dark()),
            "one-light" | "onelight" => Some(Self::one_light()),
            "solarized" | "solarized-dark" => Some(Self::solarized()),
            "solarized-light" => Some(Self::solarized_light()),
            "kanagawa" => Some(Self::kanagawa()),
            "kanagawa-lotus" | "lotus" => Some(Self::kanagawa_lotus()),
            "rose-pine" | "rosepine" => Some(Self::rose_pine()),
            "rose-pine-dawn" | "rosepine-dawn" | "dawn" => Some(Self::rose_pine_dawn()),
            "vesper" => Some(Self::vesper()),
            _ => None,
        }
    }

    /// Apply custom color overrides on top of this palette.
    pub fn with_overrides(mut self, custom: &crate::config::CustomThemeColors) -> Self {
        use crate::config::parse_color;
        if let Some(c) = &custom.accent {
            self.accent = parse_color(c);
        }
        if let Some(c) = &custom.panel_bg {
            self.panel_bg = parse_color(c);
        }
        if let Some(c) = &custom.surface0 {
            self.surface0 = parse_color(c);
        }
        if let Some(c) = &custom.surface1 {
            self.surface1 = parse_color(c);
        }
        if let Some(c) = &custom.surface_dim {
            self.surface_dim = parse_color(c);
        }
        if let Some(c) = &custom.overlay0 {
            self.overlay0 = parse_color(c);
        }
        if let Some(c) = &custom.overlay1 {
            self.overlay1 = parse_color(c);
        }
        if let Some(c) = &custom.text {
            self.text = parse_color(c);
        }
        if let Some(c) = &custom.subtext0 {
            self.subtext0 = parse_color(c);
        }
        if let Some(c) = &custom.mauve {
            self.mauve = parse_color(c);
        }
        if let Some(c) = &custom.green {
            self.green = parse_color(c);
        }
        if let Some(c) = &custom.yellow {
            self.yellow = parse_color(c);
        }
        if let Some(c) = &custom.red {
            self.red = parse_color(c);
        }
        if let Some(c) = &custom.blue {
            self.blue = parse_color(c);
        }
        if let Some(c) = &custom.teal {
            self.teal = parse_color(c);
        }
        if let Some(c) = &custom.peach {
            self.peach = parse_color(c);
        }
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceCardArea {
    pub ws_idx: usize,
    pub rect: Rect,
    pub indented: bool,
}

/// One remembered chat under a workspace in the Spaces tab.
///
/// The title is optional on purpose: the ledger records a session id, and the
/// title lives in the agent's own store, which is keyed by the directory the
/// agent was launched in. A chat started elsewhere and wired to this workspace
/// therefore has no resolvable title — the row degrades to a short id rather
/// than disappearing, because the association is the information that matters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceChatRow {
    pub session_id: String,
    pub agent: String,
    pub title: Option<String>,
    pub last_seen_ms: u64,
    /// Transcript mtime, when the chat's own file could be found. Drives the
    /// relative-age column; absent for a chat whose store location is unknown,
    /// in which case the row simply carries no age rather than a wrong one.
    pub last_modified: Option<std::time::SystemTime>,
}

/// Milliseconds since the epoch, saturating at 0 for times before it.
///
/// The ledger and the transcript store answer "when" in different units, and
/// the drawer has to order them together; this is the one place they meet.
pub(crate) fn system_time_to_ms(time: std::time::SystemTime) -> u64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

impl WorkspaceChatRow {
    /// Label for the row: the resolved title, else a short id plus the agent.
    ///
    /// The fallback is not a placeholder for missing data — for a chat started
    /// outside this workspace's directory there IS no resolvable title, and the
    /// association is still worth showing. Degrading beats disappearing.
    /// Milliseconds since the epoch of this chat's last activity.
    ///
    /// TP-DRAW-04/05: the transcript's own mtime when it was found, otherwise
    /// the moment the ledger last saw the chat. Both are "when did this last
    /// move", so one column can order and date every row — a drawer where only
    /// some rows carry an age reads as broken rather than partial.
    pub fn last_activity_ms(&self) -> u64 {
        self.last_modified
            .map(system_time_to_ms)
            .unwrap_or(self.last_seen_ms)
    }

    /// Ordering key: last activity, with the id breaking ties so the order is
    /// stable across refreshes rather than shuffling equal timestamps.
    pub(crate) fn sort_key(&self) -> (u64, &str) {
        (self.last_activity_ms(), self.session_id.as_str())
    }

    pub fn display_label(&self) -> String {
        match self.title.as_deref() {
            Some(title) if !title.is_empty() => title.to_string(),
            _ => {
                let short: String = self.session_id.chars().take(8).collect();
                format!("{short} · {}", self.agent)
            }
        }
    }
}

/// One laid-out chat row in the Spaces tab.
///
/// Kept in its own vector rather than appended to `workspace_card_areas`,
/// which is index-aligned with workspaces: appending here would make a chat
/// click resolve as a workspace (the reasoning already proven for the stage
/// entries on the tab strip, TP-FTAB-ENTRY-05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceChatRowArea {
    pub rect: Rect,
    pub ws_idx: usize,
    pub chat_idx: usize,
}

/// One laid-out chat row of the daily section.
///
/// It carries an index into the daily rows and nothing else: the section has
/// no workspace, and giving it a `ws_idx` would be inventing one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyChatRowArea {
    pub rect: Rect,
    pub chat_idx: usize,
}

/// One laid-out chat row under a declared container.
///
/// TP-CHAT-MOVE-06: it names its container and its position in that
/// container's list, and carries no `ws_idx` for the same reason the daily row
/// carries none — a container is not a workspace and may have no directory at
/// all, so a workspace index here would be invented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleChatRowArea {
    pub rect: Rect,
    pub node_key: String,
    pub chat_idx: usize,
}

/// The laid-out "… N older" / "… fewer" row of one drawer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMoreChatsArea {
    pub rect: Rect,
    pub ws_idx: usize,
}

/// One laid-out "this container is empty" note in the Spaces tab.
///
/// TP-MOD-25: it gets a vector of its own for the opposite of the usual
/// reason. The other vectors exist so a click resolves to the right thing;
/// this one exists so the row is *painted at all*. A row that is emitted,
/// takes its line, and is never drawn leaves exactly the blank gap the note
/// was added to explain — and this fork has shipped that gap twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEmptyModuleArea {
    pub rect: Rect,
    pub node_key: String,
}

/// One laid-out worktree-group header row in the Spaces tab.
///
/// TP-TREE-05: a third vector, for the same reason the chat rows got a second
/// one. A header is not a workspace — it has no `ws_idx` — so putting it in
/// the workspace-indexed vector would make a header click resolve as whichever
/// workspace happened to share its position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceGroupHeaderArea {
    pub rect: Rect,
    /// The worktree-space key this header folds and unfolds.
    pub space_key: String,
}

/// One `[[spaces.project]]` header row, kept in its own vector for the same
/// reason as [`WorkspaceGroupHeaderArea`]: a project header is neither a
/// workspace nor a space, so clicks on it must resolve through its own key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceProjectHeaderArea {
    pub rect: Rect,
    /// The project key this header folds and unfolds.
    pub project_key: String,
}

/// Cached Claude Code chat sessions for one pinned project directory. This is
/// TUI/client-layer presentation state: the reader ([`crate::claude_sessions`])
/// fills it on demand, never during render (CLAUDE.md render-purity boundary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSessions {
    /// Expanded, absolute project directory (matches `projects_pinned`).
    pub path: std::path::PathBuf,
    /// The newest chat sessions for this project (up to the fetch limit),
    /// newest first (empty is a normal state).
    pub sessions: Vec<crate::claude_sessions::ClaudeSession>,
    /// TOTAL session count in the store — busy projects hold far more chats
    /// than are parsed/listed; the surplus renders as "… N older".
    pub total_count: usize,
}

/// What a single laid-out row in the Projects tab points at. Rows reference the
/// `projects_sessions` cache by index (mirroring [`WorkspaceCardArea`], which
/// stores `ws_idx`) so the pure render and the mouse handler resolve content
/// and targets from the same source of truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectRowKind {
    /// A pinned project header row (collapse/expand chevron + name).
    Project { proj_idx: usize },
    /// A chat session row under an expanded project. Task #5 resumes it.
    Chat { proj_idx: usize, chat_idx: usize },
    /// The "(no chats)" placeholder under an expanded project with no sessions.
    Empty { proj_idx: usize },
    /// The " +" button at the right edge of a project header row: opens a new
    /// chat in that project with the default agent (left click) or the agent
    /// selector menu (shift+left click / right click).
    NewChat { proj_idx: usize },
    /// The inert "… N older" row shown when a project has more chats than the
    /// per-project display limit.
    More { proj_idx: usize },
}

/// A laid-out Projects-tab row: its screen rect plus what it points at. Computed
/// by `compute_view` (geometry) and consumed by the pure render and the mouse
/// hit-testing path, exactly like [`WorkspaceCardArea`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectRowArea {
    pub rect: Rect,
    pub kind: ProjectRowKind,
}

/// One visible CURRENT row in the native file manager. `compute_view` stores
/// these shared render/input coordinates so mouse hit-testing never recreates
/// responsive Miller geometry independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerRowArea {
    pub rect: Rect,
    pub entry_idx: usize,
    pub entry_path: PathBuf,
}

/// Client-local actions exposed at the right edge of a native file-manager
/// CURRENT row. The order is also the responsive visibility priority: narrow
/// layouts retain the earliest complete actions and drop the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerRowAction {
    SendAgent,
    Rename,
    Delete,
}

impl FileManagerRowAction {
    pub const ALL: [Self; 3] = [Self::SendAgent, Self::Rename, Self::Delete];

    pub const fn label(self) -> &'static str {
        match self {
            Self::SendAgent => ">",
            Self::Rename => "r",
            Self::Delete => "x",
        }
    }
}

/// One exact row-action hit target. The absolute entry index is resolved while
/// synchronizing the viewport so later input never reconstructs scroll math.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerRowActionArea {
    pub rect: Rect,
    pub entry_idx: usize,
    pub entry_path: std::path::PathBuf,
    pub action: FileManagerRowAction,
}

/// Client-local actions exposed by the native file-manager header. These are
/// presentation/input tags only; they are not server or wire-protocol state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerHeaderAction {
    Copy,
    Paste,
    NewFolder,
    Delete,
}

/// Client-local native-FM operation kind. Runtime execution stays in the
/// App-owned worker; this pure projection is safe for render and unit tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerOperationKind {
    Copy,
    Move,
    Trash,
    PermanentDelete,
    Rename,
    BulkRename,
}

/// One explicit lifecycle state for a bounded native-FM operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerOperationStatus {
    Running,
    Completed,
    Cancelled,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerOperationItemStatus {
    Pending,
    Running,
    Completed,
    Retained,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerOperationItemState {
    pub path: PathBuf,
    /// Exact surviving path when rollback cannot prove restoration. Normal
    /// terminal states leave this empty.
    pub recovery_path: Option<PathBuf>,
    pub status: FileManagerOperationItemStatus,
}

/// Destructive native-FM action selected by the user after confirmation.
/// This is client-local authority only; the App-owned worker performs any
/// eventual filesystem mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerDeleteKind {
    Trash,
    Permanent,
}

/// Explicit phases keep reversible trash and irreversible deletion from
/// sharing a single ambiguous confirmation action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerDeleteConfirmationStage {
    ChooseAction,
    ConfirmPermanent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerDeleteConfirmation {
    pub paths: Vec<PathBuf>,
    pub stage: FileManagerDeleteConfirmationStage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerDeleteRequest {
    pub kind: FileManagerDeleteKind,
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileManagerLocationNavigationIntent {
    FollowPreview,
    EnterTrail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileManagerLocationNavigationRequest {
    pub(crate) path: PathBuf,
    pub(crate) intent: FileManagerLocationNavigationIntent,
}

impl FileManagerLocationNavigationRequest {
    pub(crate) fn new(path: PathBuf, intent: FileManagerLocationNavigationIntent) -> Self {
        Self { path, intent }
    }

    pub(crate) fn follow(path: PathBuf) -> Self {
        Self::new(path, FileManagerLocationNavigationIntent::FollowPreview)
    }
}

impl From<PathBuf> for FileManagerLocationNavigationRequest {
    fn from(path: PathBuf) -> Self {
        Self::follow(path)
    }
}

impl PartialEq<PathBuf> for FileManagerLocationNavigationRequest {
    fn eq(&self, other: &PathBuf) -> bool {
        self.path == *other
    }
}

/// Exact client-local native-FM identities owned by the Rename text modal.
/// Opening or rendering this state performs no filesystem work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerRenameState {
    pub paths: Vec<PathBuf>,
    pub validation_error: Option<FileManagerRenameValidationError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerRenameValidationError {
    Empty,
    CurrentDirectory,
    ParentDirectory,
    Absolute,
    Separator,
    ContainsNul,
    NameTooLong,
    WindowsReservedName,
    WindowsReservedCharacter,
    WindowsTrailingDotOrSpace,
    SourceUnavailable,
}

impl FileManagerRenameValidationError {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Empty => "name cannot be empty",
            Self::CurrentDirectory | Self::ParentDirectory => "name cannot be . or ..",
            Self::Absolute | Self::Separator => "name must be one path component",
            Self::ContainsNul => "name contains a null byte",
            Self::NameTooLong => "name is too long",
            Self::WindowsReservedName => "name is reserved on Windows",
            Self::WindowsReservedCharacter => "name contains a Windows-reserved character",
            Self::WindowsTrailingDotOrSpace => "name cannot end with dot or space on Windows",
            Self::SourceUnavailable => "source changed; reopen Rename",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerRenameRequest {
    pub source_path: PathBuf,
    pub new_name: String,
}

/// Fully edited bulk mapping awaiting operation-time revalidation. The
/// current single-name modal does not synthesize this request; it is a typed
/// worker boundary for the bulk editor surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerBulkRenameRequest {
    pub mappings: Vec<(PathBuf, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerOperationState {
    pub generation: u64,
    pub kind: FileManagerOperationKind,
    pub destination_directory: PathBuf,
    pub total_items: usize,
    pub completed_items: usize,
    pub failed_items: usize,
    pub status: FileManagerOperationStatus,
    /// Ordered exact source identities and their latest terminal projection.
    pub items: Vec<FileManagerOperationItemState>,
}

impl FileManagerOperationState {
    pub fn is_running(&self) -> bool {
        self.status == FileManagerOperationStatus::Running
    }
}

impl FileManagerHeaderAction {
    pub const ALL: [Self; 4] = [Self::Copy, Self::Paste, Self::NewFolder, Self::Delete];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Copy => "[copy]",
            Self::Paste => "[paste]",
            Self::NewFolder => "[new folder]",
            Self::Delete => "[delete]",
        }
    }
}

/// Named header-action rectangle shared by pure view computation and future
/// render/input consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileManagerHeaderActionArea {
    pub rect: Rect,
    pub action: FileManagerHeaderAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerActionBarSelectionKind {
    File,
    Directory,
    Multiple,
    Unavailable,
}

/// Prepared client-local identity for the current native-FM selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerActionBarSelection {
    pub paths: Vec<PathBuf>,
    pub label: String,
    pub kind: FileManagerActionBarSelectionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerActionDisabledReason {
    InactiveFocusOwner,
    NoSelection,
    EmptyClipboard,
    ReadOnlyTarget,
    MultipleSelection,
    StaleSelection,
    UnsupportedSelection,
    UnsupportedAction,
    OperationInFlight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileManagerActionState {
    pub action: FileManagerHeaderAction,
    pub enabled: bool,
    pub disabled_reason: Option<FileManagerActionDisabledReason>,
}

/// Pure presentation model for the persistent native-FM action bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerActionBarModel {
    pub selection: Option<FileManagerActionBarSelection>,
    pub clipboard_count: usize,
    pub actions: [FileManagerActionState; 4],
}

impl FileManagerActionBarModel {
    pub fn action_state(&self, action: FileManagerHeaderAction) -> Option<&FileManagerActionState> {
        self.actions.iter().find(|state| state.action == action)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerContextMenuTargetKind {
    File,
    Directory,
    Multiple,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileManagerContextMenuAction {
    Open,
    /// Open the selection's picture in the full-frame viewer.
    ///
    /// The built-in `Open` only descends into directories, so without this a
    /// PDF or an image has no working menu entry at all — and a plugin that
    /// does not handle the type is now correctly absent from the menu.
    Enlarge,
    Copy,
    Rename,
    Delete,
    Compress,
    SendAgent,
    /// Hand the selection to another machine on the tailnet.
    ///
    /// Built in rather than left to a plugin because the destination has to be
    /// chosen, and a plugin action has nowhere to ask: it runs headless and
    /// cannot put a picker on the screen.
    SendTailscale,
    Plugin {
        plugin_id: String,
        action_id: String,
    },
}

impl FileManagerContextMenuAction {
    pub const ALL: [Self; 8] = [
        Self::Open,
        Self::Enlarge,
        Self::Copy,
        Self::Rename,
        Self::Delete,
        Self::Compress,
        Self::SendAgent,
        Self::SendTailscale,
    ];

    pub fn label(&self) -> &str {
        match self {
            Self::Open => "Open",
            Self::Enlarge => "Enlarge",
            Self::Copy => "Copy",
            Self::Rename => "Rename",
            Self::Delete => "Delete",
            Self::Compress => "Compress",
            Self::SendAgent => "Add Reference to Agent...",
            Self::SendTailscale => "Send with Tailscale...",
            Self::Plugin { action_id, .. } => action_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerContextMenuItem {
    pub action: FileManagerContextMenuAction,
    pub label: String,
    pub enabled: bool,
    pub disabled_reason: Option<FileManagerActionDisabledReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerContextMenuModel {
    pub target_kind: FileManagerContextMenuTargetKind,
    pub paths: Vec<PathBuf>,
    pub items: Vec<FileManagerContextMenuItem>,
}

/// Client-local file action intent emitted by C3 after current-state
/// revalidation. C4/C5 own all eventual filesystem and agent side effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerContextActionIntent {
    pub action: FileManagerContextMenuAction,
    pub paths: Vec<PathBuf>,
}

impl FileManagerContextMenuModel {
    /// Derive file-menu presentation authority only from the already-prepared
    /// N4.2 action-bar snapshot. This performs no cursor or filesystem reads.
    #[cfg(test)]
    pub fn from_action_bar(action_bar: &FileManagerActionBarModel) -> Option<Self> {
        Self::from_action_bar_with_plugins(action_bar, &[])
    }

    /// Append neutral, already-discovered plugin actions after the built-ins.
    /// The caller may pass an untrusted superset; context and exact path
    /// representability are checked again here before anything is exposed.
    pub fn from_action_bar_with_plugins(
        action_bar: &FileManagerActionBarModel,
        plugin_actions: &[crate::api::schema::PluginActionInfo],
    ) -> Option<Self> {
        let selection = action_bar.selection.as_ref()?;
        if selection.paths.is_empty() {
            return None;
        }

        let target_kind = match selection.kind {
            FileManagerActionBarSelectionKind::File => FileManagerContextMenuTargetKind::File,
            FileManagerActionBarSelectionKind::Directory => {
                FileManagerContextMenuTargetKind::Directory
            }
            FileManagerActionBarSelectionKind::Multiple => {
                FileManagerContextMenuTargetKind::Multiple
            }
            FileManagerActionBarSelectionKind::Unavailable => {
                FileManagerContextMenuTargetKind::Unavailable
            }
        };
        let copy_reason =
            prepared_action_disabled_reason(action_bar.action_state(FileManagerHeaderAction::Copy));
        let write_reason = prepared_action_disabled_reason(
            action_bar.action_state(FileManagerHeaderAction::Delete),
        );
        let selection_reasons = [copy_reason, write_reason];
        let selection_failure = [
            FileManagerActionDisabledReason::InactiveFocusOwner,
            FileManagerActionDisabledReason::OperationInFlight,
            FileManagerActionDisabledReason::StaleSelection,
            FileManagerActionDisabledReason::UnsupportedSelection,
        ]
        .into_iter()
        .find(|reason| selection_reasons.contains(&Some(*reason)))
        .or_else(|| {
            matches!(target_kind, FileManagerContextMenuTargetKind::Unavailable)
                .then_some(FileManagerActionDisabledReason::StaleSelection)
        });

        let mut items = FileManagerContextMenuAction::ALL
            .into_iter()
            .map(|action| {
                let disabled_reason = if let Some(reason) = selection_failure {
                    Some(reason)
                } else if matches!(target_kind, FileManagerContextMenuTargetKind::Multiple)
                    && matches!(
                        &action,
                        FileManagerContextMenuAction::Open
                            | FileManagerContextMenuAction::Enlarge
                            | FileManagerContextMenuAction::Rename
                            | FileManagerContextMenuAction::SendAgent
                    )
                {
                    Some(FileManagerActionDisabledReason::MultipleSelection)
                } else {
                    match &action {
                        FileManagerContextMenuAction::Open
                        | FileManagerContextMenuAction::Copy
                        | FileManagerContextMenuAction::SendAgent => copy_reason,
                        FileManagerContextMenuAction::Rename
                        | FileManagerContextMenuAction::Delete => write_reason,
                        // Decided from the name, which is the only evidence
                        // this projection has and the same evidence the
                        // preview classifier uses. The viewer checks the live
                        // preview again before it opens, so a name that
                        // promises a picture herdr cannot decode is refused
                        // there rather than opening onto an empty frame.
                        FileManagerContextMenuAction::Enlarge => selection
                            .paths
                            .first()
                            .filter(|path| {
                                crate::fm::entry_kind::path_looks_like_image(path)
                                    || crate::fm::pdf_preview::is_pdf_path(path)
                            })
                            .map_or(
                                Some(FileManagerActionDisabledReason::UnsupportedSelection),
                                |_| copy_reason,
                            ),
                        FileManagerContextMenuAction::Compress => {
                            Some(FileManagerActionDisabledReason::UnsupportedAction)
                        }
                        // Taildrop takes files. A folder is offered but
                        // disabled rather than hidden: the entry is the answer
                        // to "can I send this?", and a menu that silently drops
                        // it leaves the reader unsure whether the feature
                        // exists at all. Several files at once are fine —
                        // `tailscale file cp` takes a list.
                        FileManagerContextMenuAction::SendTailscale => {
                            if selection.paths.is_empty()
                                || matches!(
                                    target_kind,
                                    FileManagerContextMenuTargetKind::Directory
                                )
                            {
                                Some(FileManagerActionDisabledReason::UnsupportedSelection)
                            } else {
                                copy_reason
                            }
                        }
                        FileManagerContextMenuAction::Plugin { .. } => {
                            Some(FileManagerActionDisabledReason::UnsupportedSelection)
                        }
                    }
                };
                let label = action.label().to_string();
                FileManagerContextMenuItem {
                    action,
                    label,
                    enabled: disabled_reason.is_none(),
                    disabled_reason,
                }
            })
            .collect::<Vec<_>>();

        if selection.paths.iter().all(|path| path.to_str().is_some()) {
            let mut plugin_actions = plugin_actions
                .iter()
                .filter(|action| {
                    action
                        .contexts
                        .contains(&crate::api::schema::PluginActionContext::File)
                })
                // An action that does not handle this selection is absent, not
                // disabled. A greyed-out entry says "not right now"; this one
                // does not apply to these files at all, and offering it ran
                // the wrong program — a spreadsheet editor on a PDF, which
                // opened an empty tab with nothing to explain it.
                .filter(|action| action.matches_paths(&selection.paths))
                .collect::<Vec<_>>();
            plugin_actions.sort_by_key(|action| action.qualified_id());
            plugin_actions.dedup_by(|left, right| left.qualified_id() == right.qualified_id());
            items.extend(plugin_actions.into_iter().map(|action| {
                let disabled_reason = selection_failure;
                FileManagerContextMenuItem {
                    action: FileManagerContextMenuAction::Plugin {
                        plugin_id: action.plugin_id.clone(),
                        action_id: action.action_id.clone(),
                    },
                    label: action.title.clone(),
                    enabled: disabled_reason.is_none(),
                    disabled_reason,
                }
            }));
        }

        Some(Self {
            target_kind,
            paths: selection.paths.clone(),
            items,
        })
    }
}

impl FileManagerContextActionIntent {
    /// Convert a client-local plugin file intent into the existing public API
    /// request model without running the plugin command.
    pub fn plugin_invocation_params(&self) -> Option<crate::api::schema::PluginActionInvokeParams> {
        let FileManagerContextMenuAction::Plugin {
            plugin_id,
            action_id,
        } = &self.action
        else {
            return None;
        };
        let file_paths = self
            .paths
            .iter()
            .map(|path| path.to_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()?;
        Some(crate::api::schema::PluginActionInvokeParams {
            plugin_id: Some(plugin_id.clone()),
            action_id: action_id.clone(),
            context: Some(crate::api::schema::PluginInvocationContext {
                file_paths,
                invocation_source: Some("file_manager".into()),
                ..Default::default()
            }),
        })
    }
}

fn prepared_action_disabled_reason(
    state: Option<&FileManagerActionState>,
) -> Option<FileManagerActionDisabledReason> {
    match state {
        Some(state) if state.enabled && state.disabled_reason.is_none() => None,
        Some(state) => state
            .disabled_reason
            .or(Some(FileManagerActionDisabledReason::StaleSelection)),
        None => Some(FileManagerActionDisabledReason::StaleSelection),
    }
}

/// Deferred request to open a Claude Code chat as a new tab in a project
/// directory (Projects tab, Task #5). `session_id` `Some` resumes that
/// session, `None` starts a fresh chat. Set by the mouse handler and consumed
/// by the event loop like the other `request_*` fields, because spawning a
/// tab needs the runtime (`App`), not just `AppState`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectChatTabRequest {
    /// Expanded, absolute project directory; becomes the new tab's cwd.
    pub project_path: std::path::PathBuf,
    /// Claude Code session id to resume, or `None` for a new chat.
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeCreateState {
    pub source_workspace_id: String,
    pub source_checkout_path: std::path::PathBuf,
    pub source_existing_membership: Option<crate::workspace::WorktreeSpaceMembership>,
    pub source_repo_root: std::path::PathBuf,
    pub repo_key: String,
    pub repo_name: String,
    pub branch: String,
    pub checkout_path: std::path::PathBuf,
    pub error: Option<String>,
    pub creating: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeRemoveState {
    pub workspace_id: String,
    pub repo_root: std::path::PathBuf,
    pub path: std::path::PathBuf,
    pub error: Option<String>,
    pub removing: bool,
    pub force_confirmation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeOpenEntry {
    pub path: std::path::PathBuf,
    pub branch: Option<String>,
    pub is_linked_worktree: bool,
    pub already_open_ws_idx: Option<usize>,
}

impl WorktreeOpenEntry {
    pub(crate) fn display_name(&self) -> String {
        self.branch.clone().unwrap_or_else(|| {
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
                .unwrap_or_else(|| self.path.display().to_string())
        })
    }

    pub(crate) fn status_label(&self) -> &'static str {
        if self.already_open_ws_idx.is_some() {
            "open"
        } else if self.branch.is_some() {
            ""
        } else if self.is_linked_worktree {
            "detached"
        } else {
            "root"
        }
    }

    fn search_text(&self) -> String {
        format!(
            "{} {} {} {}",
            self.display_name(),
            self.path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default(),
            self.path.display(),
            self.status_label()
        )
        .to_lowercase()
    }

    fn matches_query(&self, query: &str) -> bool {
        text_matches_query(query, &self.search_text())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeOpenState {
    pub source_workspace_id: String,
    pub source_existing_membership: Option<crate::workspace::WorktreeSpaceMembership>,
    pub source_checkout_path: std::path::PathBuf,
    pub source_repo_root: std::path::PathBuf,
    pub repo_key: String,
    pub repo_name: String,
    pub entries: Vec<WorktreeOpenEntry>,
    pub selected: usize,
    pub query: String,
    pub search_focused: bool,
    pub error: Option<String>,
}

impl WorktreeOpenState {
    pub(crate) fn filtered_indices(&self) -> Vec<usize> {
        let query = self.query.trim();
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                (query.is_empty() || entry.matches_query(query)).then_some(idx)
            })
            .collect()
    }

    pub(crate) fn selected_entry_index(&self) -> Option<usize> {
        let indices = self.filtered_indices();
        if indices.contains(&self.selected) {
            Some(self.selected)
        } else {
            indices.first().copied()
        }
    }

    pub(crate) fn normalize_selection(&mut self) {
        if let Some(selected) = self.selected_entry_index() {
            self.selected = selected;
        }
    }

    pub(crate) fn select_previous_filtered(&mut self) {
        let indices = self.filtered_indices();
        let Some(current) = self.selected_entry_index() else {
            return;
        };
        let pos = indices.iter().position(|idx| *idx == current).unwrap_or(0);
        self.selected = indices[pos.saturating_sub(1)];
    }

    pub(crate) fn select_next_filtered(&mut self) {
        let indices = self.filtered_indices();
        let Some(current) = self.selected_entry_index() else {
            return;
        };
        let pos = indices.iter().position(|idx| *idx == current).unwrap_or(0);
        self.selected = indices[(pos + 1).min(indices.len().saturating_sub(1))];
    }
}

pub(crate) fn text_matches_query(query: &str, text: &str) -> bool {
    let haystack = text.to_lowercase();
    query
        .to_lowercase()
        .split_whitespace()
        .all(|needle| haystack.contains(needle))
}

/// Computed view geometry — derived from AppState + terminal size.
/// Updated before each render, consumed by render and mouse handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewLayout {
    Desktop,
    Mobile,
}

/// Which content the sidebar's top section shows: the workspace list
/// (`Spaces`, the default and Herdr's core navigation), pinned project chats
/// (`Projects`), or the file tree (`Files`). Sidebar presentation state that
/// lives in the TUI/client layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarTab {
    #[default]
    Spaces,
    Projects,
    Files,
}

impl SidebarTab {
    /// All tabs in left-to-right display order.
    pub const ALL: [SidebarTab; 3] = [SidebarTab::Spaces, SidebarTab::Projects, SidebarTab::Files];

    /// Short header label shown in the tab bar.
    pub fn label(self) -> &'static str {
        match self {
            SidebarTab::Spaces => "Spaces",
            SidebarTab::Projects => "Projects",
            SidebarTab::Files => "Files",
        }
    }
}

pub use super::file_manager_locations_model::{
    FileManagerLocationIcon, FileManagerLocationItem, FileManagerLocationsModel,
};
#[cfg(test)]
pub use super::file_manager_locations_model::{
    FileManagerLocationSectionKind, FILE_MANAGER_LOCATIONS_MAX_ITEMS,
};

/// Client-local, computed hit geometry for the focused agent's attachment
/// affordance. The stable pane and terminal identities travel with the rect so
/// input never has to infer authority from a border coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAttachmentActionArea {
    pub rect: Rect,
    pub pane_id: PaneId,
    pub terminal_id: crate::terminal::TerminalId,
}

/// Client-local, computed hit geometry for the focused agent's existing-
/// worktree launcher. Stable identities travel with the rect so input can
/// reject stale frame snapshots before emitting the existing intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentWorktreeActionArea {
    pub rect: Rect,
    pub workspace_id: String,
    pub pane_id: PaneId,
    pub terminal_id: crate::terminal::TerminalId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAttachmentTarget {
    pub workspace_id: String,
    pub pane_id: PaneId,
    pub terminal_id: crate::terminal::TerminalId,
}

#[derive(Debug, Clone)]
pub struct AgentAttachmentPickerState {
    pub file_manager: crate::fm::FmState,
    pub target: AgentAttachmentTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentAttachmentDeliveryRequest {
    pub path: PathBuf,
    pub target: AgentAttachmentTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentAttachmentOpenError {
    InsufficientSpace,
    Unavailable,
}

pub struct ViewState {
    pub layout: ViewLayout,
    /// One cached named-region projection with generation-safe flattened hits.
    /// Mobile keeps an empty projection with its own revision so transitions
    /// cannot reactivate a stale desktop hit token.
    pub shell: crate::ui::shell::ShellView,
    pub sidebar_rect: Rect,
    pub workspace_card_areas: Vec<WorkspaceCardArea>,
    /// Chat drawer rows, kept apart from `workspace_card_areas` because that
    /// vector is workspace-indexed: a chat folded into it would resolve as a
    /// workspace switch on click.
    pub workspace_chat_row_areas: Vec<WorkspaceChatRowArea>,
    /// The "older chats" rows, in their own vector so a press there can never
    /// resolve as the chat above it (TP-DRAW-11).
    pub workspace_more_chats_areas: Vec<WorkspaceMoreChatsArea>,
    /// The daily section's own rows, in vectors of their own.
    ///
    /// TP-DAILY-03/07: the section carries no `ws_idx` at all, so folding its
    /// rows into the workspace-indexed vectors would make every press resolve
    /// as some other workspace's chat. The header, the chats and the "older"
    /// switch answer three different gestures and so keep three vectors.
    pub daily_header_area: Option<Rect>,
    pub daily_chat_row_areas: Vec<DailyChatRowArea>,
    /// TP-CHAT-MOVE-06: the laid-out chat rows of declared containers.
    pub module_chat_row_areas: Vec<ModuleChatRowArea>,
    pub daily_more_area: Option<Rect>,
    /// Worktree-group header rows, kept apart for the same reason: a header is
    /// not a workspace, so it must never be resolvable through a ws_idx.
    pub workspace_group_header_areas: Vec<WorkspaceGroupHeaderArea>,
    /// Project header rows, one per `[[spaces.project]]` umbrella on screen.
    pub workspace_project_header_areas: Vec<WorkspaceProjectHeaderArea>,
    /// "Nothing in here yet" notes, one per drawn container that has nothing
    /// beneath it. Laid out so the row can be painted, not so it can be hit.
    pub workspace_empty_module_areas: Vec<WorkspaceEmptyModuleArea>,
    /// Hit areas for the Spaces/Projects/Files header tabs (one per
    /// `SidebarTab::ALL`, in order). Empty when the sidebar is collapsed.
    pub sidebar_tab_hit_areas: Vec<Rect>,
    /// Laid-out rows for the Projects tab (project headers + chat sessions).
    /// Empty on every non-Projects tab and when the sidebar is collapsed.
    pub project_row_areas: Vec<ProjectRowArea>,
    /// Complete AppDock entry targets for the current frame. Empty whenever
    /// the live shell projects no dock region.
    pub app_dock_entry_areas: Vec<crate::ui::app_dock::AppDockEntryArea>,
    /// One current-frame projection for the Files-local locations rail,
    /// compact action, exact row identities, and remaining Trail viewport.
    pub(crate) file_manager_locations: crate::ui::FileManagerLocationsView,
    /// Bounded logical Miller columns and dividers projected for the current
    /// Files frame. Empty while Files is closed or its body cannot fit one
    /// complete minimum-width column.
    #[allow(dead_code)] // P1 establishes compute ownership; P2 removes this
    // once render/input consume the snapshot.
    pub(crate) file_manager_miller: crate::ui::MillerViewSnapshot,
    /// Canonical Trail columns, rows, dividers, and optional detail panel for
    /// the current Files frame. Render consumes this exact projection; T7.4
    /// input will hit-test the same immutable geometry.
    pub(crate) file_manager_trail: crate::ui::TrailViewSnapshot,
    /// Visible CURRENT rows for the native file manager. Empty while FM is
    /// closed or when its content area has no drawable rows.
    pub file_manager_row_areas: Vec<FileManagerRowArea>,
    /// Exact, disjoint action targets at the right edge of visible CURRENT
    /// rows. Empty while FM is closed or the row cannot fit a complete action.
    pub file_manager_row_action_areas: Vec<FileManagerRowActionArea>,
    /// Named native-FM header actions for this frame. Empty while FM is closed
    /// or when the header cannot preserve its minimum identity width.
    pub file_manager_header_action_areas: Vec<FileManagerHeaderActionArea>,
    /// The rect the enlarged preview draws into, when the viewer is open.
    ///
    /// Published here rather than derived twice: the decode target, the Kitty
    /// placement and the renderer must agree on one rect, and the two that run
    /// outside render have no frame to measure.
    pub preview_viewer_content_area: Option<Rect>,
    /// Selection-sensitive persistent action-bar content for this frame.
    /// `None` while the native FM is closed.
    pub file_manager_action_bar: Option<FileManagerActionBarModel>,
    /// Exact complete `[+]` target for the focused agent pane. `None` for
    /// non-agent, non-terminal, file-manager, mobile, borderless, or too-small
    /// layouts.
    pub agent_attachment_action_area: Option<AgentAttachmentActionArea>,
    /// Exact complete `[w]` target beside `[+]` for an eligible focused agent.
    /// `None` when cached Git/worktree capability is absent or linked-child.
    pub agent_worktree_action_area: Option<AgentWorktreeActionArea>,
    /// Exact visible CURRENT rows inside the blocking attachment picker. The
    /// render and mouse paths share this snapshot so responsive geometry is
    /// never reconstructed from coordinates during input handling.
    pub agent_attachment_picker_row_areas: Vec<FileManagerRowArea>,
    pub tab_bar_rect: Rect,
    pub tab_hit_areas: Vec<Rect>,
    /// Stage app entries sharing the tab strip with the terminal tabs. Held
    /// apart from `tab_hit_areas` because that vector is index-aligned with
    /// `ws.tabs`; each entry carries its instance identity so a rect retained
    /// across close and reopen cannot authorize the new instance.
    pub stage_tab_hit_areas: Vec<crate::ui::surface_host::StageTabHitArea>,
    pub tab_scroll_left_hit_area: Rect,
    pub tab_scroll_right_hit_area: Rect,
    pub new_tab_hit_area: Rect,
    pub terminal_area: Rect,
    pub mobile_header_rect: Rect,
    /// The three targets the mobile header projects: a button at each edge and
    /// the active-tab strip between them.
    pub mobile_header_hits: crate::ui::MobileHeaderHitAreas,
    pub toast_hit_area: Rect,
    pub pane_infos: Vec<PaneInfo>,
    pub split_borders: Vec<SplitBorder>,
}

/// The file manager's raster preview, opened to fill the frame.
///
/// Identity is the exact path, matching the Trail's selection authority: a row
/// index would survive a directory refresh that moved the file, and the viewer
/// would then be showing something the user did not open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewViewerState {
    pub source_path: std::path::PathBuf,
}

/// The device picker for sending the selection over Taildrop.
///
/// The selection is copied in when the picker opens rather than read back from
/// the file manager on send. The two can drift: a directory refresh, or the
/// reader moving the cursor with the picker up, would otherwise send a
/// different file from the one the menu was opened on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailscaleSendState {
    pub paths: Vec<std::path::PathBuf>,
    /// Empty when the tailnet has no other machines, or when loading failed —
    /// `status` says which.
    pub devices: Vec<crate::tailscale::TailscaleDevice>,
    pub selected: usize,
    /// The one line under the list: what went wrong, or what was sent where.
    /// `None` before anything has happened.
    pub status: Option<String>,
    /// True while a send is running. Kept so a second Enter cannot start a
    /// second send on top of the first.
    pub sending: bool,
    /// Devices this picker has successfully sent to, by target name.
    ///
    /// Drawn as a mark on the row. The status line alone was not enough: it
    /// names the last outcome, and a reader who is not sure whether the press
    /// registered presses again — the same file went out several times before
    /// this field existed.
    pub sent_targets: Vec<String>,
}

impl TailscaleSendState {
    /// Move the highlight, stopping at the ends rather than wrapping.
    ///
    /// Wrapping in a destination list is a hazard: holding Down past the last
    /// device silently lands back on the first, and the reader presses Enter on
    /// a machine they did not mean to pick.
    pub fn move_selection(&mut self, forward: bool) -> bool {
        if self.devices.is_empty() {
            return false;
        }
        let last = self.devices.len() - 1;
        let next = if forward {
            self.selected.saturating_add(1).min(last)
        } else {
            self.selected.saturating_sub(1)
        };
        if next == self.selected {
            return false;
        }
        self.selected = next;
        true
    }

    pub fn selected_device(&self) -> Option<&crate::tailscale::TailscaleDevice> {
        self.devices.get(self.selected)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    Onboarding,
    ReleaseNotes,
    ProductAnnouncement,
    /// No overlay: keys go to the layout.
    #[default]
    Navigate,
    Prefix,
    Copy,
    Terminal,
    AttachFile,
    RenameWorkspace,
    RenameTab,
    RenamePane,
    RenameFile,
    NewLinkedWorktree,
    OpenExistingWorktree,
    ConfirmRemoveWorktree,
    Resize,
    ConfirmClose,
    ConfirmFileDelete,
    ContextMenu,
    Settings,
    GlobalMenu,
    KeybindHelp,
    Navigator,
    AgentReferencePicker,
    PreviewViewer,
    TailscaleSend,
}

impl Mode {
    /// Whether keys in this mode are commands/navigation (an ASCII input source is wanted) rather
    /// than free text. This is an explicit **allowlist** of the prefix command/navigation realm:
    /// any mode NOT listed defaults to leaving the user's IME alone (the safe default), so adding a
    /// new text-entry or overlay mode can never silently force ASCII. Used by
    /// `sync_prefix_input_source` (gated by `switch_ascii_input_source_in_prefix`) so multi-level
    /// prefix commands keep ASCII until they return to the terminal.
    ///
    /// Known limitation: the search boxes in `Navigator` and `KeybindHelp` are also held on ASCII,
    /// since this `Mode`-level predicate can't see `search_focused` (non-ASCII filtering there
    /// would need a runtime check).
    pub(crate) fn wants_ascii_input(self) -> bool {
        matches!(
            self,
            Mode::Prefix
                | Mode::Navigate
                | Mode::Navigator
                | Mode::Copy
                | Mode::Resize
                | Mode::ConfirmClose
                | Mode::ConfirmFileDelete
                | Mode::ConfirmRemoveWorktree
                | Mode::ContextMenu
                | Mode::GlobalMenu
                | Mode::KeybindHelp
                | Mode::AttachFile
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NavigatorTarget {
    Workspace {
        ws_idx: usize,
    },
    Tab {
        ws_idx: usize,
        tab_idx: usize,
    },
    Pane {
        ws_idx: usize,
        tab_idx: usize,
        pane_id: PaneId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NavigatorRow {
    pub target: NavigatorTarget,
    pub depth: u8,
    pub label: String,
    pub meta: String,
    pub status: AgentState,
    pub seen: bool,
    pub is_current: bool,
    pub is_workspace: bool,
    pub is_tab: bool,
    pub expanded: bool,
    pub search_text: String,
    /// Whether this row itself matched the active query/state filter, as
    /// opposed to being included as ancestor context or cascaded subtree of a
    /// matching workspace or tab. Always true when no filter is active.
    pub matched: bool,
}

/// One rendered line in the navigator body. Spacer lines separate workspace
/// groups visually and are not selectable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NavigatorDisplayLine {
    Spacer,
    Row(usize),
}

pub(crate) fn navigator_display_lines(rows: &[NavigatorRow]) -> Vec<NavigatorDisplayLine> {
    let mut lines = Vec::with_capacity(rows.len().saturating_mul(2));
    for (idx, row) in rows.iter().enumerate() {
        if row.is_workspace && !lines.is_empty() {
            lines.push(NavigatorDisplayLine::Spacer);
        }
        lines.push(NavigatorDisplayLine::Row(idx));
    }
    lines
}

pub(crate) fn navigator_display_index_of_row(
    lines: &[NavigatorDisplayLine],
    row_idx: usize,
) -> Option<usize> {
    lines
        .iter()
        .position(|line| *line == NavigatorDisplayLine::Row(row_idx))
}

pub(crate) fn navigator_first_row_at_or_after(
    lines: &[NavigatorDisplayLine],
    line_idx: usize,
) -> Option<usize> {
    lines.get(line_idx..)?.iter().find_map(|line| match line {
        NavigatorDisplayLine::Row(idx) => Some(*idx),
        NavigatorDisplayLine::Spacer => None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NavigatorStateFilter {
    Blocked,
    Working,
    Idle,
    Done,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct NavigatorState {
    pub query: String,
    pub selected: usize,
    pub scroll: usize,
    pub search_focused: bool,
    pub state_filter: Option<NavigatorStateFilter>,
    pub expanded_workspaces: std::collections::HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CopyModeState {
    pub pane_id: PaneId,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub entry_offset_from_bottom: usize,
    pub selection: Option<CopyModeSelection>,
    pub search: CopyModeSearchState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CopyModeSelection {
    Character,
    Linewise { anchor_row: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CopyModeSearchDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CopyModeSearchPrompt {
    pub direction: CopyModeSearchDirection,
    pub query: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CopyModeSearchState {
    pub prompt: Option<CopyModeSearchPrompt>,
    pub query: String,
    pub direction: Option<CopyModeSearchDirection>,
    pub matches: Vec<crate::pane::TerminalTextMatch>,
    pub current: Option<usize>,
    pub geometry: Option<(u16, u16)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AgentPanelSort {
    #[default]
    Spaces,
    Priority,
}

/// How workspace chat drawers decide to be open, mirrored from
/// [`crate::config::ChatDrawerModeConfig`] at startup and on config reload.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ChatDrawerMode {
    #[default]
    AllActive,
    Focused,
    Manual,
}

// ---------------------------------------------------------------------------
// Settings UI state
// ---------------------------------------------------------------------------

/// Which section of the settings panel is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    Theme,
    Sound,
    Toast,
    PaneLabels,
    Preview,
    Experiments,
    Integrations,
}

impl SettingsSection {
    pub const ALL: &[Self] = &[
        Self::Theme,
        Self::Sound,
        Self::Toast,
        Self::PaneLabels,
        Self::Preview,
        Self::Integrations,
        Self::Experiments,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::Sound => "sound",
            Self::Toast => "toasts",
            Self::PaneLabels => "pane labels",
            Self::Preview => "preview",
            Self::Experiments => "experiments",
            Self::Integrations => "integrations",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExperimentSetting {
    PaneHistory,
    SwitchAsciiInputSourceInPrefix,
    /// Announced-only ("soon"): no backend yet, the toggle is inert.
    TilingFix,
}

impl ExperimentSetting {
    pub(crate) const ALL: [Self; 3] = [
        Self::PaneHistory,
        Self::SwitchAsciiInputSourceInPrefix,
        Self::TilingFix,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::PaneHistory => "pane screen history",
            Self::SwitchAsciiInputSourceInPrefix => {
                "switch to ascii input source in prefix (macOS)"
            }
            Self::TilingFix => "tiling fix (soon)",
        }
    }

    /// Purpose line shown for the selected experiment — announced features
    /// carry their intent here before the backend exists.
    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::PaneHistory => "persist and restore pane screen contents across restarts",
            Self::SwitchAsciiInputSourceInPrefix => {
                "avoid non-ascii input sources swallowing prefix-mode keys"
            }
            Self::TilingFix => {
                "hand preview placement to your desktop tiling manager so the \
                 focused terminal and the chromium preview snap into one \
                 optimized side-by-side tiled layout. not implemented yet — \
                 announced surface only, the toggle is inert"
            }
        }
    }

    /// Whether the experiment has a working backend; announced-only entries
    /// render but cannot be toggled.
    pub(crate) fn is_available(self) -> bool {
        !matches!(self, Self::TilingFix)
    }

    pub(crate) fn enabled(self, state: &AppState) -> bool {
        match self {
            Self::PaneHistory => state.pane_history_persistence_enabled(),
            Self::SwitchAsciiInputSourceInPrefix => {
                state.switch_ascii_input_source_in_prefix_enabled()
            }
            Self::TilingFix => false,
        }
    }
}

/// All built-in theme names in display order.
pub const THEME_NAMES: &[&str] = &[
    "catppuccin",
    "catppuccin-latte",
    "terminal",
    "tokyo-night",
    "tokyo-night-day",
    "dracula",
    "nord",
    "gruvbox",
    "gruvbox-light",
    "one-dark",
    "one-light",
    "solarized",
    "solarized-light",
    "kanagawa",
    "kanagawa-lotus",
    "rose-pine",
    "rose-pine-dawn",
    "vesper",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuListState {
    pub highlighted: usize,
}

impl MenuListState {
    pub fn new(highlighted: usize) -> Self {
        Self { highlighted }
    }

    pub fn move_prev(&mut self) {
        self.highlighted = self.highlighted.saturating_sub(1);
    }

    pub fn move_next(&mut self, item_count: usize) {
        if item_count > 0 {
            self.highlighted = (self.highlighted + 1).min(item_count - 1);
        }
    }

    pub fn hover(&mut self, idx: Option<usize>) {
        if let Some(idx) = idx {
            self.highlighted = idx;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectionListState {
    pub selected: usize,
}

impl SelectionListState {
    pub fn new(selected: usize) -> Self {
        Self { selected }
    }

    pub fn move_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_next(&mut self, item_count: usize) {
        if item_count > 0 {
            self.selected = (self.selected + 1).min(item_count - 1);
        }
    }

    pub fn select(&mut self, idx: usize) {
        self.selected = idx;
    }
}

#[derive(Debug, Clone)]
pub struct ThemeRuntimeConfig {
    pub manual_name: String,
    pub dark_name: String,
    pub light_name: String,
    pub auto_switch: bool,
    pub custom: Option<crate::config::CustomThemeColors>,
    pub legacy_accent: Option<String>,
}

#[derive(Clone, Default, PartialEq)]
pub struct SettingsState {
    /// Which section tab is active.
    pub section: SettingsSection,
    /// Selected item index within the current section.
    pub list: SelectionListState,
    /// The palette before opening settings (for cancel/restore).
    pub original_palette: Option<Palette>,
    /// The theme name before opening settings.
    pub original_theme: Option<String>,
}

#[derive(Clone)]
pub(crate) enum DragTarget {
    WorkspaceReorder {
        source_ws_idx: usize,
        insert_idx: Option<usize>,
    },
    TabReorder {
        ws_idx: usize,
        source_tab_idx: usize,
        insert_idx: Option<usize>,
    },
    WorkspaceListScrollbar {
        grab_row_offset: u16,
    },
    AgentPanelScrollbar {
        grab_row_offset: u16,
    },
    ProjectsScrollbar {
        grab_row_offset: u16,
    },
    PaneSplit {
        path: Vec<bool>,
        direction: Direction,
        area: Rect,
        grab_offset: u16,
    },
    PaneScrollbar {
        pane_id: crate::layout::PaneId,
        grab_row_offset: u16,
    },
    ReleaseNotesScrollbar {
        grab_row_offset: u16,
    },
    ProductAnnouncementScrollbar {
        grab_row_offset: u16,
    },
    KeybindHelpScrollbar {
        grab_row_offset: u16,
    },
    SidebarDivider,
    SidebarSectionDivider,
}

/// Active mouse drag on a split border or sidebar divider.
#[derive(Clone)]
pub(crate) struct DragState {
    pub target: DragTarget,
}

#[derive(Clone)]
pub(crate) struct WorkspacePressState {
    pub ws_idx: usize,
    pub start_col: u16,
    pub start_row: u16,
}

/// What a "new sub/parallel module" is collecting a name for: the node key
/// the new `[[spaces.node]]` entry will hang under, `None` for top level
/// (TP-DOTS-05/07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingNewModule {
    pub parent: Option<String>,
    /// The key being renamed, when this prompt is a rename rather than a
    /// creation (TP-MOD-32). A rename MUST carry the existing key: deriving a
    /// new one from the new name would re-key the container and leave its
    /// children and members pointing at a module that no longer exists.
    pub rename_key: Option<String>,
}

#[derive(Clone)]
pub(crate) struct TabPressState {
    pub ws_idx: usize,
    pub tab_idx: usize,
    pub start_col: u16,
    pub start_row: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextMenuKind {
    Workspace {
        ws_idx: usize,
    },
    GitWorkspace {
        ws_idx: usize,
        is_linked_worktree: bool,
        has_worktree_children: bool,
        collapsed: bool,
        /// Whether a `[[spaces.split]]` rule already claims this checkout —
        /// the menu offers "Demote from module" only then (TP-RANK-06).
        space_is_custom: bool,
    },
    /// The move submenu a branch row's "Move..." opens: pick which of the
    /// three verbs (K5) — or a new group, or top level. `has_targets` is
    /// false when no node exists yet, which hides the three verbs that would
    /// have nothing to point at.
    MoveWorkspace {
        ws_idx: usize,
        has_targets: bool,
    },
    /// The target picker one of the three verbs opens: every node in the
    /// forest as `(key, display name)`. Selection resolves by index, so the
    /// names never have to be unique.
    MoveTarget {
        ws_idx: usize,
        op: crate::spaces::MoveOp,
        targets: Vec<(String, String)>,
    },
    /// A chat row's own menu. The session id is resolved at open time so a
    /// list refresh under an open menu can never re-target the move
    /// (TP-CHAT-MOVE-04); `has_move` decides whether "Move back" shows and
    /// `has_live` whether the chat has a running tab to close (TP-AGPANEL-05).
    WorkspaceChat {
        /// The drawer this row was pressed in, or `None` for a daily row.
        ///
        /// TP-CHAT-MOVE-08: the daily section is not a workspace and its chats
        /// belong to no drawer, so there is nothing to exclude from the move
        /// picker. Naming an arbitrary workspace here would silently drop a
        /// legitimate destination from the list.
        ws_idx: Option<usize>,
        session_id: String,
        has_move: bool,
        has_live: bool,
    },
    /// An agents-panel row's own menu. The row already stands for one pane,
    /// so the target rides in the menu rather than being re-derived from
    /// whatever happens to be focused when the item is picked
    /// (TP-AGPANEL-03).
    AgentEntry {
        ws_idx: usize,
        tab_idx: usize,
        pane_id: crate::layout::PaneId,
        /// The chat this row is running, when the ledger knows it.
        ///
        /// TP-AGPANEL-28: carried in the menu rather than looked up when the
        /// verb fires, for the reason TP-AGPANEL-06 already states — a menu
        /// can outlive the panel it was opened from, and a late lookup would
        /// answer for whatever moved into that slot. `None` when the tab was
        /// never opened to resume a known session; such a chat has no identity
        /// the ledger can file, so it cannot be moved.
        session_id: Option<String>,
    },
    /// The drawer picker a chat move opens: open workspaces as
    /// `(ledger key, display name)`, resolved by index like MoveTarget.
    ChatMoveTarget {
        session_id: String,
        targets: Vec<(String, String)>,
    },
    /// A node header's own menu — the module and project rows the tree draws
    /// from `[[spaces.node]]` / `[[spaces.project]]` entries. Creation lands
    /// here because the header IS the parent a new module would hang under;
    /// `collapsed` picks which fold verb the menu offers (TP-DOTS-01).
    NodeHeader {
        node_key: String,
        collapsed: bool,
        /// Whether the overlay — the file the machine owns — declares this
        /// module, which is the only case a delete verb can keep its word
        /// (TP-MOD-26). Resolved when the menu opens, so a reload underneath
        /// an open menu cannot turn the offer into a no-op.
        deletable: bool,
    },
    /// A repository/bucket header's menu. A split rule cannot parent a node,
    /// so offering "new sub-module" here would be a promise the tree cannot
    /// keep — the menu carries the fold verbs only (TP-DOTS-01).
    SpaceHeader {
        space_key: String,
        collapsed: bool,
    },
    Tab {
        ws_idx: usize,
        tab_idx: usize,
    },
    Pane {
        ws_idx: usize,
        tab_idx: usize,
        pane_id: PaneId,
        source_pane_id: Option<PaneId>,
        has_manual_label: bool,
    },
    /// The tree's empty space: the one place a container can be made without
    /// already standing in one (TP-MOD-31). Carries nothing — the blank is
    /// not a row, so there is no identity here to go stale.
    SidebarBlank,
    /// The daily area's own header menu (TP-DAILY-12). Carries only whether
    /// it is folded, the way `SpaceHeader` does: the area names a directory,
    /// not a row, so nothing here can go stale under a refresh.
    DailyHeader {
        collapsed: bool,
    },
    /// Agent selector for a new chat in the daily directory (TP-DAILY-11).
    ///
    /// Carries no index of any kind: the daily section belongs to no
    /// workspace, so there is nothing here that a refresh could invalidate —
    /// unlike every sibling below, this menu cannot go stale.
    DailyNewChat,
    /// Agent selector for a new chat in a pinned project (Projects tab).
    /// Selecting an agent makes it the persisted default and opens the chat.
    /// When the project is also open as a workspace, the menu additionally
    /// offers that workspace's worktree actions (mirroring the Spaces menu).
    ProjectNewChat {
        proj_idx: usize,
        has_workspace: bool,
    },
    /// The Spaces tab's per-workspace "+" menu. Same choices as the Projects
    /// one, keyed by workspace directly: the Spaces row already knows which
    /// workspace it is, so routing through a project index would only add a
    /// lookup that can fail. `offers_worktree` is false for a checkout that is
    /// already a linked worktree — nesting worktrees is not a thing.
    WorkspaceNewChat {
        ws_idx: usize,
        offers_worktree: bool,
    },
    /// Native-FM action model prepared from explicit client-local selection.
    /// C3.1 models intent only; C4/C5 own eventual execution authority.
    File {
        model: FileManagerContextMenuModel,
    },
    /// Anchored app-name popover for one dock entry (SF5.2). The single row
    /// carries the accessible name and activates the app on selection.
    AppDock {
        app: crate::ui::surface_host::BuiltInAppId,
    },
}

/// Right-click context menu state.
#[derive(Clone, PartialEq)]
pub struct ContextMenuState {
    pub kind: ContextMenuKind,
    pub x: u16,
    pub y: u16,
    pub list: MenuListState,
}

impl ContextMenuState {
    pub fn items(&self) -> Vec<&str> {
        match &self.kind {
            ContextMenuKind::Workspace { .. } => vec!["Rename", "Close"],
            ContextMenuKind::GitWorkspace {
                is_linked_worktree: false,
                has_worktree_children: false,
                ..
            } => vec!["Rename", "Close", "New worktree", "Open worktree..."],
            // TP-RANK-06: a branch row can raise its own rank from the menu —
            // the mouse road to `herdr space promote` — and offers the demote
            // only when a rule actually claims it.
            ContextMenuKind::GitWorkspace {
                is_linked_worktree: true,
                space_is_custom,
                ..
            } => {
                let mut items = vec!["Rename", "Close", "Promote to module", "Promote to project"];
                if *space_is_custom {
                    items.push("Demote from module");
                }
                // TP-RANK-13: the mouse road to `herdr space move` starts here.
                items.push("Move...");
                items.push("Delete worktree checkout...");
                items
            }
            ContextMenuKind::GitWorkspace {
                is_linked_worktree: false,
                has_worktree_children: true,
                collapsed: true,
                ..
            } => vec![
                "Rename",
                "Close group",
                "New worktree",
                "Open worktree...",
                "Expand",
            ],
            ContextMenuKind::GitWorkspace {
                is_linked_worktree: false,
                has_worktree_children: true,
                collapsed: false,
                ..
            } => vec![
                "Rename",
                "Close group",
                "New worktree",
                "Open worktree...",
                "Collapse",
            ],
            // TP-RANK-13: the verbs only show when a node exists to point
            // them at; the naming and top-level roads always do.
            ContextMenuKind::MoveWorkspace { has_targets, .. } => {
                let mut items = Vec::new();
                if *has_targets {
                    items.extend(["Under a group...", "Beside a group...", "Above a group..."]);
                }
                items.extend(["Under a new group...", "To top level"]);
                items
            }
            ContextMenuKind::MoveTarget { targets, .. } => {
                targets.iter().map(|(_, label)| label.as_str()).collect()
            }
            // TP-CHAT-MOVE-04: the way back only shows while a re-home is in
            // force — offering it otherwise would be a button that does
            // nothing.
            ContextMenuKind::WorkspaceChat {
                has_move, has_live, ..
            } => {
                let mut items = vec!["Move to branch..."];
                if *has_move {
                    items.push("Move back");
                }
                // TP-AGPANEL-05: the close verb only appears while the chat
                // has a tab running behind it, and it comes last — the one
                // item here that cannot be undone.
                if *has_live {
                    items.push("Close agent");
                }
                items
            }
            // TP-AGPANEL-03: the agents panel lists what is running, so the
            // single verb it owns is ending one.
            // TP-AGPANEL-28: the panel row can send its chat somewhere, when
            // the ledger knows which chat it is. The move verb comes first and
            // the close verb stays last — the one item here that cannot be
            // undone belongs at the end (TP-AGPANEL-05's ordering).
            ContextMenuKind::AgentEntry { session_id, .. } => {
                if session_id.is_some() {
                    vec!["Move to...", "Close agent"]
                } else {
                    vec!["Close agent"]
                }
            }
            ContextMenuKind::ChatMoveTarget { targets, .. } => {
                targets.iter().map(|(_, label)| label.as_str()).collect()
            }
            // TP-DOTS-01/10: every module header creates — the node header
            // and the bucket header alike, because to the person using the
            // tree both ARE modules — plus the one fold verb the current
            // state calls for. Buckets can parent modules (TP-NODE-08).
            ContextMenuKind::NodeHeader {
                collapsed,
                deletable,
                ..
            } => {
                // TP-DOTS-13: the branch road leads — the point of a module
                // is the branches inside it.
                let mut items = vec![
                    "New branch...",
                    "New sub-module...",
                    "New parallel module...",
                ];
                items.push(if *collapsed { "Expand" } else { "Collapse" });
                // TP-MOD-32: renaming rewrites the machine's own file, so it
                // is offered on exactly the modules a delete is — a rename
                // written into the overlay for a hand-written module loses
                // to it at first-match and would do nothing at all.
                if *deletable {
                    items.push("Rename module...");
                }
                // TP-MOD-08/26: last, because it is the only item that takes
                // something away — and only when there is something the
                // machine can take back.
                if *deletable {
                    items.push("Delete module");
                }
                items
            }
            // TP-MOD-28: a bucket keeps the menu it had. A split rule is taken
            // back by the branch verb that wrote it, not by this one.
            ContextMenuKind::SpaceHeader { collapsed, .. } => {
                let mut items = vec![
                    "New branch...",
                    "New sub-module...",
                    "New parallel module...",
                ];
                items.push(if *collapsed { "Expand" } else { "Collapse" });
                items
            }
            ContextMenuKind::Tab { .. } => vec!["New tab", "Rename", "Close"],
            ContextMenuKind::ProjectNewChat {
                has_workspace: false,
                ..
            } => crate::app::projects::CHAT_AGENTS.to_vec(),
            ContextMenuKind::ProjectNewChat {
                has_workspace: true,
                ..
            } => crate::app::projects::PROJECT_CHAT_MENU_WITH_WORKTREES.to_vec(),
            // TP-MOD-31: one verb. "New project" is deliberately absent — a
            // project is a node that claims a repository, and the blank has
            // no repository to claim, so the entry could never be honoured.
            ContextMenuKind::SidebarBlank => vec!["New module..."],
            // TP-DAILY-12: the area's own verbs. No branch or sub-module
            // entries: the daily directory is not a repository and holds no
            // tree beneath it, so those would be offers it cannot keep.
            ContextMenuKind::DailyHeader { collapsed } => {
                vec![
                    "New chat...",
                    if *collapsed { "Expand" } else { "Collapse" },
                ]
            }
            // TP-DAILY-11: agents only. The daily directory is not a checkout,
            // so a worktree verb here would be an offer the tree cannot keep.
            ContextMenuKind::DailyNewChat => crate::app::projects::CHAT_AGENTS.to_vec(),
            ContextMenuKind::WorkspaceNewChat {
                offers_worktree: false,
                ..
            } => crate::app::projects::CHAT_AGENTS.to_vec(),
            ContextMenuKind::WorkspaceNewChat {
                offers_worktree: true,
                ..
            } => crate::app::projects::PROJECT_CHAT_MENU_WITH_WORKTREES.to_vec(),
            ContextMenuKind::File { model } => {
                model.items.iter().map(|item| item.label.as_str()).collect()
            }
            ContextMenuKind::AppDock { app } => match app {
                crate::ui::surface_host::BuiltInAppId::Terminal => vec!["Terminal"],
                crate::ui::surface_host::BuiltInAppId::Files => vec!["Files"],
            },
            ContextMenuKind::Pane {
                has_manual_label: true,
                source_pane_id: Some(_),
                ..
            } => vec![
                "Rename pane",
                "Clear pane name",
                "Swap with focused pane",
                "Split right",
                "Split down",
                "Zoom",
                "Close pane",
            ],
            ContextMenuKind::Pane {
                has_manual_label: false,
                source_pane_id: Some(_),
                ..
            } => vec![
                "Rename pane",
                "Swap with focused pane",
                "Split right",
                "Split down",
                "Zoom",
                "Close pane",
            ],
            ContextMenuKind::Pane {
                has_manual_label: true,
                source_pane_id: None,
                ..
            } => vec![
                "Rename pane",
                "Clear pane name",
                "Split right",
                "Split down",
                "Zoom",
                "Close pane",
            ],
            ContextMenuKind::Pane {
                has_manual_label: false,
                source_pane_id: None,
                ..
            } => vec![
                "Rename pane",
                "Split right",
                "Split down",
                "Zoom",
                "Close pane",
            ],
        }
    }

    pub fn item_enabled(&self, idx: usize) -> bool {
        match &self.kind {
            ContextMenuKind::File { model } => {
                model.items.get(idx).is_some_and(|item| item.enabled)
            }
            _ => idx < self.items().len(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    NeedsAttention,
    Finished,
    UpdateInstalled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastTarget {
    pub workspace_id: String,
    pub pane_id: PaneId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastNotification {
    pub kind: ToastKind,
    pub title: String,
    pub context: String,
    pub position: Option<crate::config::ToastHerdrPosition>,
    pub target: Option<ToastTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingAgentNotification {
    pub pane_id: PaneId,
    pub workspace_id: String,
    pub agent_label: String,
    pub known_agent: Option<crate::detect::Agent>,
    pub kind: ToastKind,
    pub state: AgentState,
    pub deadline: std::time::Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentNotificationDelivery {
    pub pane_id: PaneId,
    pub workspace_id: String,
    pub agent_label: String,
    pub known_agent: Option<crate::detect::Agent>,
    pub kind: ToastKind,
    pub toast: Option<ToastNotification>,
    pub client_notification: Option<ToastNotification>,
    pub sound: Option<crate::sound::Sound>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyFeedback {
    pub message: String,
}

pub struct ReleaseNotesState {
    pub version: String,
    pub body: String,
    pub scroll: u16,
    pub preview: bool,
}

pub struct ProductAnnouncementState {
    pub version: String,
    pub id: String,
    pub title: String,
    pub body: String,
    pub scroll: u16,
    pub preview: bool,
}

/// Which mobile drawer is open, if any.
///
/// Two edges answer two questions. The left one answers "which project am I
/// in" — a session-wide question asked when the context changes. The right one
/// answers "which tab of this project" — a question asked constantly and
/// answered within one workspace. They used to share a single scrolling list,
/// which buried the frequent question under the rare one.
///
/// One enum rather than two booleans, so the mutual exclusion is structural:
/// there is no value here that describes both drawers being open, and opening
/// one closes the other by construction rather than by every caller
/// remembering to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MobileDrawer {
    #[default]
    None,
    /// Spaces, then agents, then the global menu.
    Spaces,
    /// The active workspace's tabs.
    Tabs,
}

impl MobileDrawer {
    pub fn is_open(self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct KeybindHelpState {
    pub scroll: u16,
    pub query: String,
    pub search_focused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarWidthSource {
    #[default]
    ConfigDefault,
    Persisted,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PaneFocusTarget {
    pub workspace_id: String,
    pub pane_id: PaneId,
}

/// Identifies one connected client.
///
/// This shares the input-source id space, where `0` is the local/monolithic
/// path, so the id a client's input already carries is the same id its view
/// resolves through.
pub type ClientId = u64;

/// Declares the state that belongs to one display rather than to the session.
///
/// The generated pair is the whole point. A surface is declared once here, and
/// both the save side and the load side are generated from that one line, so a
/// surface cannot be half-migrated. Half-migration is the failure shape this
/// feature exists to remove: the field keeps living in one place, every display
/// keeps resolving the same value, and the symptom — a menu opening on all of
/// them at once — looks like a rendering bug rather than a missing swap.
///
/// The fields stay on `AppState` as registers rather than moving into the
/// bundle, because they are read in hundreds of places and every one of those
/// reads means "the display being served". The bundle is only what a display
/// parks while another one is being served.
macro_rules! client_surfaces {
    (
        inherited { $( $(#[$meta:meta])* $field:ident : $ty:ty ),+ $(,)? }
        broadcast {
$( $(#[$bmeta:meta])* $bfield:ident : $bty:ty ),+ $(,)? }
        owned { $( $(#[$ometa:meta])* $ofield:ident : $oty:ty ),+ $(,)? }
        ephemeral { $( $(#[$emeta:meta])* $efield:ident : $ety:ty ),+ $(,)? }
    ) => {
        /// Everything about *what this display is looking at*, parked while
        /// another display is being served. Never persisted as session truth.
        #[derive(Clone, Default)]
        pub(super) struct ClientSurfaces {
            $( $(#[$meta])* pub(super) $field: $ty, )+
            $( $(#[$bmeta])* pub(super) $bfield: $bty, )+
        }

        /// What a display holds that is never copied — only handed over.
        ///
        /// Kept apart from [`ClientSurfaces`] for one reason: that bundle is
        /// cloned on entry so the next park has something to compare against,
        /// and these are the surfaces where a clone means walking a
        /// directory's worth of entries, per display, per frame. Nothing
        /// compares them, so nothing needs a second copy of them.
        #[derive(Default)]
        pub(super) struct ClientOwned {
            $( $(#[$ometa])* pub(super) $ofield: $oty, )+
            $( $(#[$emeta])* pub(super) $efield: $ety, )+
        }

        impl AppState {
            /// Lifts the serving display's surfaces out of the registers.
            ///
            /// This is a move, not a copy: swapping displays costs the size of
            /// the bundle, not a deep clone of what it holds.
            fn take_surfaces(&mut self) -> ClientSurfaces {
                ClientSurfaces {
                    $( $field: std::mem::take(&mut self.$field), )+
                    $( $bfield: std::mem::take(&mut self.$bfield), )+
                }
            }

            fn take_owned(&mut self) -> ClientOwned {
                ClientOwned {
                    $( $ofield: std::mem::take(&mut self.$ofield), )+
                    $( $efield: std::mem::take(&mut self.$efield), )+
                }
            }

            fn install_owned(&mut self, owned: ClientOwned) {
                $( self.$ofield = owned.$ofield; )+
                $( self.$efield = owned.$efield; )+
            }

            /// Builds what a display seen for the first time starts with.
            ///
            /// Inherited and broadcast surfaces come from the default, so a
            /// display attaching now lands where the session is being driven
            /// rather than on workspace zero. Owned and ephemeral surfaces do
            /// not: a display that has just attached has not opened a file
            /// browser, is not halfway through a drag, and is not holding a
            /// blocking picker. Handing it one is the shared-focus complaint
            /// arriving at the moment a monitor is plugged in.
            ///
            /// TP-SUR-ADOPT-01, TP-SUR-ADOPT-02
            fn adopted_surfaces(&self) -> ClientSurfaces {
                let mut adopted = self.default_surfaces.clone();
                // The first display served IS the session — the monolithic
                // case — and takes over whatever the session had open,
                // exactly as the owned surfaces do. Only a display attaching
                // NEXT TO already-seen displays is born clean: what the
                // session holds is already owned by one of them.
                if !self.surfaces_by_client.is_empty() {
                    self.strip_person_opened_surfaces(&mut adopted);
                }
                adopted
            }

            /// Installs a display's surfaces into the registers.
            fn install_surfaces(&mut self, surfaces: ClientSurfaces) {
                $( self.$field = surfaces.$field; )+
                $( self.$bfield = surfaces.$bfield; )+
            }

            /// Moves the surfaces a display actually changed into the default.
            ///
            /// Field by field, and only where the value differs from what that
            /// display parked last time. Promoting the whole bundle on every
            /// park would make the default mean "whatever was rendered last"
            /// instead of "where the session is being driven" — and the render
            /// loop parks every attached display on every frame, so the
            /// default would be overwritten continuously by displays that did
            /// nothing.
            ///
            /// That distinction is load-bearing, not cosmetic. The default is
            /// what resolves when no display is acting, which is the path the
            /// API and the notification logic run on: clobber it and a pane
            /// sitting in a background workspace starts reading as foreground,
            /// so its agent finishes in silence.
            ///
            /// Private surfaces are absent here on purpose. A surface that
            /// cannot be compared cannot be told apart from a display merely
            /// being looked at, so it can never be promoted safely — and a
            /// surface that is never promoted stays `Default` in the default
            /// bundle, which means a display seen for the first time starts
            /// without it. For a blocking picker holding a live filesystem
            /// view that is not a limitation but the behaviour you want: a
            /// display that has just attached has not opened anything.
            ///
            /// TP-SUR-DEFAULT-01
            fn promote_changed_surfaces(
                &mut self,
                parked: Option<&ClientSurfaces>,
                outgoing: &ClientSurfaces,
            ) {
                $(
                    if parked.map(|previous| &previous.$field) != Some(&outgoing.$field) {
                        self.default_surfaces.$field = outgoing.$field.clone();
                    }
                )+
                $(
                    if parked.map(|previous| &previous.$bfield) != Some(&outgoing.$bfield) {
                        self.default_surfaces.$bfield = outgoing.$bfield.clone();
                    }
                )+
            }

            /// Hands every display a change the session made on its own.
            ///
            /// Broadcast surfaces are the ones where a change with no display
            /// behind it is an instruction rather than a preference: focusing
            /// a pane through the API puts the session in terminal mode, and a
            /// display still parked in navigate mode would swallow everything
            /// its user typed. Inherited surfaces are deliberately excluded —
            /// choosing a workspace for one display is the whole point of
            /// keeping them apart.
            ///
            /// TP-SUR-BROADCAST-01
            fn broadcast_session_changes(&mut self, outgoing: &ClientSurfaces) {
                $(
                    if self.default_surfaces.$bfield != outgoing.$bfield {
                        for parked in self.surfaces_by_client.values_mut() {
                            parked.$bfield = outgoing.$bfield.clone();
                        }
                    }
                )+
            }
        }
    };
}

impl AppState {
    /// Clears everything a person opened on some other screen from a bundle a
    /// freshly attached display is about to adopt.
    ///
    /// The default bundle is "where the session is being driven", and for the
    /// inherited and presentational surfaces adopting it is right: a new
    /// display should land on the session's workspace, not on workspace zero.
    /// But the default also accumulates overlays — a popup, a preview viewer,
    /// a context menu, a half-typed rename — promoted there when the display
    /// that opened them parked. Adopting those is the shared-focus complaint
    /// arriving at the moment a monitor is plugged in, in its worst form:
    /// after a restart every display re-attaches, so every display is "seen
    /// for the first time" and every one of them is born inside the overlay
    /// one screen had open before the restart.
    ///
    /// A display that has just attached has not opened anything. The same
    /// sentence already governs the owned surfaces (TP-SUR-ADOPT-01); this
    /// extends it to the person-opened broadcast surfaces.
    ///
    /// TP-SUR-ADOPT-02
    fn strip_person_opened_surfaces(&self, adopted: &mut ClientSurfaces) {
        adopted.popup_pane = None;
        adopted.preview_viewer = None;
        adopted.context_menu = None;
        adopted.copy_mode = None;
        adopted.global_menu = MenuListState::default();
        adopted.settings = SettingsState::default();
        adopted.navigator = NavigatorState::default();
        adopted.keybind_help = KeybindHelpState::default();
        adopted.worktree_create = None;
        adopted.worktree_open = None;
        adopted.worktree_remove = None;
        adopted.file_manager_rename = None;
        adopted.file_manager_delete_confirmation = None;
        adopted.name_input = String::new();
        adopted.name_input_replace_on_type = false;
        adopted.creating_new_tab = false;
        adopted.requested_new_tab_name = None;
        adopted.rename_pane_target = None;
        // Transient feedback belongs to the screen that earned it.
        adopted.toast = None;
        adopted.copy_feedback = None;
        // A mode whose overlay was just stripped would swallow the new
        // display's first keystrokes into an overlay it cannot see. Session
        // announcements (onboarding, release notes) are not person-opened and
        // stay; Navigate and Terminal are already safe landings.
        if !matches!(
            adopted.mode,
            Mode::Onboarding
                | Mode::ReleaseNotes
                | Mode::ProductAnnouncement
                | Mode::Navigate
                | Mode::Terminal
        ) {
            adopted.mode = adopted.overlay_return_mode.take().unwrap_or({
                if self.active.is_some() {
                    Mode::Terminal
                } else {
                    Mode::Navigate
                }
            });
        }
        adopted.overlay_return_mode = None;
    }
}

client_surfaces! {
    inherited {

    /// The workspace this display is in.
    active: Option<usize>,
    }
    broadcast {
    /// The file browser's own prompts. The directory they act on is still
    /// session-wide, but the confirmation and the rename field are things a
    /// person opened, on one screen, with a cursor in them.
    file_manager_rename: Option<FileManagerRenameState>,
    file_manager_delete_confirmation: Option<FileManagerDeleteConfirmation>,
    /// Sidebar geometry, which is measured against the display it is drawn
    /// on. Two displays are two different widths, so one width cannot be
    /// right on both, and the collapse a narrow display needs is exactly
    /// what a wide one should not be forced into. The configured defaults
    /// and the bounds around them stay session-wide; only what this display
    /// currently is belongs here.
    sidebar_width: u16,
    sidebar_collapsed: bool,
    sidebar_width_source: SidebarWidthSource,
    sidebar_section_split: f32,
    /// Transient feedback, which belongs on the screen that earned it.
    toast: Option<ToastNotification>,
    copy_feedback: Option<CopyFeedback>,
    /// Which of Spaces, Projects and Files the left rail is showing.
    ///
    /// Its own comment already called it state that lives in the client
    /// layer; it was simply kept in one place. One display switching to
    /// Projects moved every display to Projects.
    sidebar_tab: SidebarTab,
    /// The highlighted row in the sidebar.
    selected: usize,
    /// Where each scrollable list sits. A scroll position is a statement
    /// about one viewport, and the displays do not share a viewport.
    workspace_scroll: usize,
    agent_panel_scroll: usize,
    projects_scroll: usize,
    tab_scroll: usize,
    tab_scroll_follow_active: bool,
    mobile_switcher_scroll: usize,
    /// Which tree rows this display has folded away.
    collapsed_space_keys: std::collections::HashSet<String>,
    collapsed_project_paths: std::collections::HashSet<std::path::PathBuf>,
    /// Which chat drawers this display has opened, and which derived-open
    /// drawers it has quieted. The same sentence as the folds above: a
    /// drawer is a statement about one screen, and the shared set was how
    /// one display's activations shoved another's drawers around.
    expanded_chat_workspaces: std::collections::HashSet<String>,
    suppressed_chat_drawers: std::collections::HashSet<String>,
    /// Whether this display folded the daily-chats section away.
    ///
    /// TP-DAILY-03: the same sentence as the folds above — a fold is a
    /// statement about one screen. It lives here rather than in a shared
    /// field so that folding the section on a laptop does not close it on the
    /// monitor next to it.
    daily_section_collapsed: bool,
    /// Whether this display asked the daily section for every chat it holds
    /// rather than the glance surface's five (TP-DAILY-04).
    daily_section_expanded: bool,
    /// Which overlay, if any, owns this display's input.
    mode: Mode,
    /// The mode to return to when the overlay on top of it closes.
    overlay_return_mode: Option<Mode>,
    /// The right-click menu, which opens where one display was clicked.
    context_menu: Option<ContextMenuState>,
    /// Highlight position in the global menu.
    global_menu: MenuListState,
    /// The floating pane an overlay puts on top of the layout.
    popup_pane: Option<PopupPaneState>,
    /// Scrollback selection, which is a cursor on one display's screen.
    copy_mode: Option<CopyModeState>,
    /// The full-surface preview one display opened.
    preview_viewer: Option<PreviewViewerState>,
    /// Settings, the navigator and the keybind sheet each carry a cursor and
    /// a query, which belong to whoever is typing.
    settings: SettingsState,
    navigator: NavigatorState,
    keybind_help: KeybindHelpState,
    /// The worktree dialogs, each a multi-step form.
    worktree_create: Option<WorktreeCreateState>,
    worktree_open: Option<WorktreeOpenState>,
    worktree_remove: Option<WorktreeRemoveState>,
    /// The shared text field every naming prompt types into, and the prompts
    /// that read it. Splitting the field without the prompts would let one
    /// display's keystrokes land in another display's dialog.
    name_input: String,
    name_input_replace_on_type: bool,
    creating_new_tab: bool,
    requested_new_tab_name: Option<String>,
    rename_pane_target: Option<PaneId>,
    }
    owned {
    /// The phone shell's drawer, and everything that describes its position.
    ///
    /// Owned rather than broadcast: a broadcast surface promotes what one
    /// display changed into what every later display adopts, which is right
    /// for a mode and wrong for a drawer. Shared, a phone attached beside a
    /// desktop opened and closed the desktop's drawer with its own — which
    /// drawer was open, where its cursor sat, whether the active workspace's
    /// chats were folded, and whether the client had been handed back its own
    /// selection gesture were one value for every display at once. A display
    /// that has just attached has not opened a drawer (TP-MOB-75).
    mobile_drawer: MobileDrawer,
    mobile_drawer_cursor: usize,
    mobile_active_chats_folded: bool,
    mobile_select_mode: Option<bool>,
    /// Which app surface this display is looking at, and — since it is the
    /// same surface — the directory behind it.
    ///
    /// This is the level above the workspace, and leaving it shared is what
    /// made one display opening Files send every other display to Files too.
    ///
    /// It sits here rather than with the inherited surfaces because it can
    /// only ever be as inheritable as the contents it points at, and those
    /// cannot be promoted: comparing a directory listing on every park means
    /// walking a directory's worth of entries per display per frame. Keeping
    /// the pair together is what makes a stage pointing at contents that do
    /// not exist unrepresentable rather than merely repaired.
    ///
    /// The consequence is the behaviour you want anyway: a display seen for
    /// the first time starts in the terminal, because a display that has just
    /// attached has not opened a file browser.
    stage: crate::ui::surface_host::StageState,
    /// The directory this display is browsing, and where it is in it.
    ///
    /// Private rather than broadcast because a directory listing is not
    /// something the session decides on a display's behalf, and because
    /// comparing one on every park would mean walking a directory's worth of
    /// entries per display per frame. A display seen for the first time opens
    /// its own browser rather than joining someone else's position in theirs.
    file_manager: Option<crate::fm::FmState>,
    /// Where the location rail on this display is pointed.
    file_manager_locations: crate::app::file_manager_locations::FileManagerLocationsState,
    /// A navigation this display's rail asked for, waiting to be served.
    ///
    /// Shared, this is consumed by whichever display the scheduled loop
    /// reaches first — lowest id, every time — so a click on any other
    /// display navigates that one's browser instead, and the display that
    /// was actually clicked sits inert. The request has to belong to the
    /// display that made it, because the browser it acts on does.
    request_file_manager_location_navigation: Option<FileManagerLocationNavigationRequest>,
    /// The rest of what this display asked its browser to do. Each one is
    /// resolved against the asking display's rail focus, current directory
    /// and selection, so consumed in another display's view it acts on the
    /// wrong browser or — far more often — on none, and the click reads as
    /// ignored.
    request_file_manager_rename: Option<FileManagerRenameRequest>,
    request_file_manager_bulk_rename: Option<FileManagerBulkRenameRequest>,
    request_file_manager_delete: Option<FileManagerDeleteRequest>,
    request_file_manager_context_action: Option<FileManagerContextActionIntent>,
    }
    ephemeral {
    /// A gesture in flight, which is private for the same reason it is
    /// per-display. Every one of these is anchored to a rectangle in
    /// one display's last frame, so letting another display resolve it would
    /// apply the drag to geometry it never saw.
    drag: Option<DragState>,
    workspace_press: Option<WorkspacePressState>,
    tab_press: Option<TabPressState>,
    selection: Option<Selection>,
    selection_autoscroll: Option<SelectionAutoscroll>,
    right_click_passthrough: Option<RightClickPassthroughGesture>,
    /// Blocking pickers, which own the keyboard of the display that opened
    /// them and no other. They carry a live filesystem view, which is why
    /// they are private: comparing one is neither cheap nor meaningful.
    agent_attachment_picker: Option<AgentAttachmentPickerState>,
    agent_reference_picker: Option<crate::app::agent_reference_picker::AgentReferencePickerState>,
    }
}

/// All application state — pure data, no channels or async runtime.
/// Testable without PTYs or a tokio runtime.
pub struct AppState {
    pub terminals:
        std::collections::HashMap<crate::terminal::TerminalId, crate::terminal::TerminalState>,
    /// Recently closed agents — dead rows carrying a revival recipe, written
    /// by the close/exit triggers and read by the agents panel. Server-side
    /// session fact (see `closed_agents` module); nothing ever ticks for it.
    pub(crate) closed_agents: crate::app::closed_agents::ClosedAgentLedger,
    /// Terminal ids whose size is currently owned by a direct attach client.
    pub direct_attach_resize_locks: std::collections::HashSet<crate::terminal::TerminalId>,
    pub(crate) pane_id_aliases: std::collections::HashMap<u32, PaneId>,
    pub(crate) public_pane_id_aliases: std::collections::HashMap<String, PaneId>,
    pub workspaces: Vec<Workspace>,
    pub active: Option<usize>,
    /// Whose view the workspace accessors resolve right now.
    ///
    /// Set for the duration of one client's input routing and one client's
    /// render pass, and `None` outside those windows — restore, an API call
    /// that names no client, and the render path taken when nothing is
    /// attached. Accessors fall back to the workspace default there, which is
    /// the value a newly attaching client adopts.
    ///
    /// Client-local presentation state: never persisted, never sent over the
    /// wire as session truth.
    pub(super) viewer: Option<ClientId>,
    /// The workspace each client is in.
    ///
    /// `active` above is the *resolved* value for the current viewer, not
    /// storage: the viewer window swaps a client's workspace in on the way in
    /// and saves it back on the way out. Keeping `active` as the resolved
    /// register is deliberate — it is read in several hundred places, and
    /// every one of them means "the workspace the display being served is in".
    ///
    /// Client-local presentation state: never persisted.
    pub(super) surfaces_by_client: std::collections::HashMap<ClientId, ClientSurfaces>,
    /// The workspace a client adopts before it has chosen one, and the value
    /// resolved when no client is acting.
    ///
    /// Tracks the most recent actual switch, so a display attaching later
    /// lands where the session is being driven.
    pub(super) default_surfaces: ClientSurfaces,
    /// What each display holds that is only ever handed over, never copied.
    ///
    /// A display being served has no entry here: its state is in the
    /// registers. That is what makes the swap free.
    pub(super) owned_by_client: std::collections::HashMap<ClientId, ClientOwned>,
    /// The owned surfaces held when no display has ever been served.
    pub(super) default_owned: ClientOwned,
    pub(crate) previous_pane_focus: Option<PaneFocusTarget>,
    pub selected: usize,
    pub mode: Mode,
    /// The non-default focus owner (`Resize`/`Copy`) remembered when a
    /// blocking overlay opened, restored by `leave_modal` while still valid.
    /// Client-local transient presentation state — never persisted.
    pub(crate) overlay_return_mode: Option<Mode>,
    /// When `Some`, the native file manager is open and its directory list
    /// replaces the terminal panes in the center area. `None` = closed (the
    /// panes render as usual). Client-side presentation state (v1 TUI-only,
    /// per the runtime/client boundary), swapped in like `SidebarTab` content.
    pub file_manager: Option<crate::fm::FmState>,
    /// Typed client-local owner of the active WorkspaceStage surface. It does
    /// not create or replace server, workspace, tab, pane, or terminal IDs.
    pub(crate) stage: crate::ui::surface_host::StageState,
    /// Blocking client-local picker that keeps terminal panes in the base
    /// layer. It owns no watcher, worker, process, pane, or server state.
    pub agent_attachment_picker: Option<AgentAttachmentPickerState>,
    /// At most one typed request may cross the scheduled delivery boundary.
    pub request_agent_attachment_delivery: Option<AgentAttachmentDeliveryRequest>,
    /// Client-local source paths prepared for future native-FM paste actions.
    /// Closing the FM does not discard clipboard content; no filesystem work
    /// is performed merely by storing these paths.
    pub file_manager_clipboard: Vec<PathBuf>,
    /// Client-local icon glyph profile for native-FM entry rows. `Nerd`
    /// matches the sidebar/AppDock icon language; `Ascii` is the
    /// deterministic no-font fallback and the canonical cross-machine
    /// visual-fixture profile. Never persisted, never wire protocol.
    pub file_icon_profile: crate::fm::entry_kind::IconProfile,
    /// Pure client-local projection of the App-owned bounded operation worker.
    /// Render/input consume this state but never perform filesystem work.
    pub file_manager_operation: Option<FileManagerOperationState>,
    /// The file manager's raster preview, opened to fill the frame.
    ///
    /// Pure client presentation: the server has no opinion about how large a
    /// preview is drawn. The viewer holds no pixels of its own — it changes
    /// which rect the one raster preview is decoded and placed into, so
    /// enlarging is a bigger decode rather than an upscale of the panel-sized
    /// one.
    pub preview_viewer: Option<PreviewViewerState>,
    /// Devices the user pinned to the top of the send picker, in their order.
    ///
    /// Held on the state rather than read from config at open time so the
    /// picker and the file that persists it cannot disagree mid-session.
    pub tailscale_pinned_devices: Vec<String>,
    /// The Taildrop destination picker, while it is open.
    ///
    /// Client presentation: the tailnet is not herdr's session state, and the
    /// picker holds only what it needs to ask the question and report the
    /// answer.
    pub tailscale_send: Option<TailscaleSendState>,
    /// Exact native-FM identities awaiting an explicit destructive choice.
    /// Opening or rendering this modal never performs filesystem work.
    pub file_manager_delete_confirmation: Option<FileManagerDeleteConfirmation>,
    /// Exact native-FM identities owned by the file Rename text modal.
    pub file_manager_rename: Option<FileManagerRenameState>,
    /// One validated rename request awaiting C4's operation-time preflight.
    pub request_file_manager_rename: Option<FileManagerRenameRequest>,
    /// One complete bulk mapping awaiting C4's all-or-nothing preflight.
    pub request_file_manager_bulk_rename: Option<FileManagerBulkRenameRequest>,
    /// One confirmed, revalidated delete request for the App-owned worker.
    pub request_file_manager_delete: Option<FileManagerDeleteRequest>,
    /// Exact, revalidated client-local intent from the native-FM context menu.
    /// C3 only emits this tag; C4/C5 will consume it to perform real work.
    pub request_file_manager_context_action: Option<FileManagerContextActionIntent>,
    /// One exact current path and focused agent terminal identity awaiting the
    /// App-owned C5 send boundary. Preparing it performs no runtime side effect.
    pub request_file_manager_agent_handoff:
        Option<crate::app::agent_reference_picker::AgentReferenceRequest>,
    /// Blocking client-local agent target picker for the reference action.
    /// It owns no watcher, worker, process, pane, or server state.
    pub agent_reference_picker:
        Option<crate::app::agent_reference_picker::AgentReferencePickerState>,
    /// Prepared, bounded Files-locations data. Filesystem/environment discovery
    /// happens only when this projection is refreshed, never during render.
    pub file_manager_locations_model: FileManagerLocationsModel,
    pub request_file_manager_location_navigation: Option<FileManagerLocationNavigationRequest>,
    /// Explicit client-local root identity for the Native Files locations
    /// surface. It is never inferred from cwd and never persisted or sent
    /// through the server protocol.
    pub(crate) file_manager_locations:
        crate::app::file_manager_locations::FileManagerLocationsState,
    /// Exact row path and surface intent prepared by input and consumed once
    /// by the App-owned scheduled navigation boundary.
    pub should_quit: bool,
    /// In monolithic --no-session mode, detach exits the app because there is no server to detach from.
    pub detach_exits: bool,
    /// Set when the current client should detach from the persistent session.
    /// The server's event loop checks this and handles client detach.
    pub detach_requested: bool,
    pub request_new_workspace: bool,
    pub request_new_tab: bool,
    pub request_new_linked_worktree: Option<usize>,
    pub request_open_existing_worktree: Option<usize>,
    /// The workspace a "move under a new group" is naming a group for while
    /// the rename input collects the name. Client-local like every modal
    /// fact: naming a group on one display never opens an input on another.
    pub pending_move_new_group: Option<usize>,
    /// The module a "New branch..." will hang its rule under while the
    /// worktree dialog collects the branch name (TP-DOTS-14/15). Client-local
    /// like every dialog fact; consumed at submit, disarmed on cancel.
    pub pending_branch_module: Option<String>,
    /// The parent a "new sub/parallel module" is collecting a name for
    /// (TP-DOTS-05): `Some(PendingNewModule)` arms the rename input, the
    /// inner parent is where the new node hangs (`None` = top level).
    /// Client-local for the same reason as `pending_move_new_group`.
    pub pending_new_module: Option<PendingNewModule>,
    /// Read-only mirror of the chat ledger's re-homes (session id → target
    /// ledger key), refreshed by the same sync that projects the rows. The
    /// state layer needs it to build the chat menu; the ledger itself lives
    /// on the App and is the only writer.
    pub chat_move_overrides: std::collections::BTreeMap<String, String>,
    /// Ledger keys this display filed a chat into, most recent first.
    ///
    /// TP-CHAT-MOVE-09: the request that started this feature said it should
    /// take "a few clicks" because the work develops spontaneously. A list
    /// that grows with every workspace and module answers that badly — the
    /// place you filed something into a minute ago is overwhelmingly the place
    /// you mean next. Client-local and not persisted: it is a convenience of
    /// this session's hand, not a fact about the tree.
    pub recent_move_targets: Vec<String>,
    /// A chat re-home decision waiting for the App loop, which owns the
    /// ledger: `(session_id, Some(target))` moves, `(session_id, None)`
    /// withdraws (TP-CHAT-MOVE-04).
    pub request_chat_move: Option<(String, Option<String>)>,
    pub request_new_workspace_cwd: Option<std::path::PathBuf>,
    pub request_remove_linked_worktree: Option<usize>,
    pub request_submit_worktree_create: bool,
    pub request_submit_worktree_open: bool,
    pub request_submit_worktree_remove: bool,
    pub request_reload_config: bool,
    /// Set when the headless server should ask attached clients to reload
    /// their client-local sound config from disk.
    pub request_client_config_reload: bool,
    /// Set when UI interaction requested a clipboard write that must be
    /// handled by the outer App/event loop instead of directly from AppState.
    pub request_clipboard_write: Option<Vec<u8>>,
    /// Set when a Projects-tab chat row (resume) or "(no chats)" row (new
    /// chat) was clicked; consumed by the event loop to spawn the tab.
    pub request_project_chat_tab: Option<ProjectChatTabRequest>,
    /// Set whenever tab focus changes; consumed by the event loop to check
    /// whether the now-focused tab has a wired browser preview to surface.
    /// A bare flag (no indices): the consumer resolves the ACTIVE tab at
    /// consume time, so the request can never go stale.
    pub request_preview_show: bool,
    pub creating_new_tab: bool,
    pub requested_new_tab_name: Option<String>,
    pub pending_workspace_create_cwd: Option<std::path::PathBuf>,
    pub rename_pane_target: Option<PaneId>,
    pub worktree_create: Option<WorktreeCreateState>,
    pub worktree_open: Option<WorktreeOpenState>,
    pub worktree_remove: Option<WorktreeRemoveState>,
    pub worktree_directory: std::path::PathBuf,
    pub collapsed_space_keys: std::collections::HashSet<String>,
    /// Folded `[[spaces.project]]` headers, keyed by the project's `key`.
    /// Same lifecycle as `collapsed_space_keys`: session-persisted UI state.
    pub collapsed_project_keys: std::collections::HashSet<String>,
    /// Validated `[[spaces.split]]` rules, in config order. A checkout claimed
    /// by one of these groups under the rule's key instead of its repository's.
    /// Config-derived presentation state: refreshed on load and on reload.
    pub space_split_rules: Vec<crate::spaces::SpaceSplitRule>,
    /// Validated `[[spaces.project]]` umbrellas, in config order. A space
    /// claimed by one of these nests under the project's top-level header.
    /// Config-derived presentation state: refreshed on load and on reload.
    pub space_projects: Vec<crate::spaces::SpaceProject>,
    /// The validated node forest (`[[spaces.node]]` + projects doubled as
    /// parentless nodes), cycles and ghosts already cut loose.
    pub space_nodes: Vec<crate::spaces::SpaceNode>,
    /// Which of `space_nodes` came from the overlay the machine writes.
    ///
    /// The merge that builds `space_nodes` deliberately forgets where each
    /// entry came from — on screen they are the same thing. The delete verb is
    /// the one place the difference matters (TP-MOD-26), so the provenance is
    /// kept beside the forest and re-derived by the same call that re-derives
    /// it. Empty is the honest default: nothing is deletable until a load says
    /// otherwise.
    pub managed_node_keys: std::collections::HashSet<String>,
    /// Row-kind icons for the Spaces tree (`[spaces.icons]`), defaults filled.
    /// Config-derived presentation state: refreshed on load and on reload.
    pub space_icons: crate::config::SpaceIconsConfig,
    /// Expanded, absolute project directories pinned to the Projects tab, in
    /// config order (`[projects] pinned` with `~` resolved). TUI/client state.
    pub projects_pinned: Vec<std::path::PathBuf>,
    /// Cached chat sessions per pinned project, aligned with `projects_pinned`.
    /// Filled by `refresh_project_sessions*`; read-only during render.
    pub projects_sessions: Vec<ProjectSessions>,
    /// Live click-bridge tab↔session bindings (preview↔tab sync), newest
    /// first. Refreshed by the runtime's fingerprint poll; shared runtime
    /// fact, read-only for any presentation layer.
    pub preview_bindings: Vec<crate::preview_bindings::PreviewBinding>,
    /// Configured preview window placement mode (`[preview] placement`).
    /// Unimplemented ("soon") modes behave like the default at runtime.
    pub preview_placement: crate::config::PreviewPlacement,
    /// Pinned project paths whose chat list is collapsed in the Projects tab.
    pub collapsed_project_paths: std::collections::HashSet<std::path::PathBuf>,
    /// Chats remembered for each workspace, keyed by
    /// [`crate::persist::workspace_chats::ledger_key`] and newest first.
    /// Presentation cache filled from the ledger outside render, mirroring
    /// `projects_sessions` — the render path never reads a file.
    pub workspace_chat_rows: std::collections::HashMap<String, Vec<WorkspaceChatRow>>,
    /// Ledger keys whose chat drawer is OPEN in the Spaces tab.
    ///
    /// The inverse of `collapsed_project_paths`, deliberately: the Projects tab
    /// pins a handful of projects and opening them all is useful, while Spaces
    /// routinely holds a dozen-plus workspaces — opening every drawer at once
    /// would bury the workspace list the tab exists for. So closed is the
    /// default and this records the exceptions.
    pub expanded_chat_workspaces: std::collections::HashSet<String>,
    /// Whether this display folded the daily-chats section away (TP-DAILY-03).
    pub daily_section_collapsed: bool,
    /// Whether this display asked the daily section for every chat it holds
    /// rather than the glance surface's five (TP-DAILY-04).
    pub daily_section_expanded: bool,
    /// Whether this display shows only the tree it is working in: the active
    /// checkout and the ones running an agent, with the module chain above
    /// them. Per display for the same reason the folds are — focusing one
    /// screen must not narrow another (TP-FOCUS-SW-05).
    pub spaces_focus_only: bool,
    /// Drawers this display has opened all the way, past the five rows the
    /// glance surface keeps (TP-DRAW-10). Per display, like every other
    /// drawer set here.
    pub fully_open_chat_drawers: std::collections::HashSet<String>,
    /// Drawers this display has quieted while a mode derives them open.
    ///
    /// The all-active drawer mode opens every branch holding a live agent;
    /// folding one of those rows cannot go through `expanded_chat_workspaces`
    /// (the derivation would reopen it on the next frame), so the fold lands
    /// here instead. Per display for the same reason the folds are: one
    /// screen quieting a drawer must not quiet it anywhere else.
    pub suppressed_chat_drawers: std::collections::HashSet<String>,
    /// The directory whose chats belong to no checkout — the daily ones.
    ///
    /// `$HOME` in practice, and stated rather than derived at the point of use
    /// so tests can point it somewhere harmless and so a client that has no
    /// home has an honest `None` instead of a guess. `None` means the section
    /// does not exist at all, which is also what an empty directory produces.
    ///
    /// Why this exists: a chat started outside any repository files itself
    /// under `$HOME`, and since TP-WSID-01 made `effective_cwd` prefer the
    /// checkout, no workspace claims that directory any more. Measured
    /// 2026-08-12: 1266 transcripts there, 0 workspaces holding it — those
    /// chats were reachable from nowhere in the sidebar.
    pub daily_chat_cwd: Option<std::path::PathBuf>,
    /// Git branch per live terminal cwd, for the agent panel's secondary
    /// label. Kept fresh by the runtime's HEAD-mtime fingerprint poll;
    /// read-only during render.
    pub(crate) tab_branch_cache:
        std::collections::HashMap<std::path::PathBuf, super::tab_branches::TabBranchEntry>,
    /// Incremental per-file parse cache for the Projects tab: unchanged
    /// session files ((mtime, size) key) are never re-read, so refreshes cost
    /// only the diff.
    pub sessions_parse_cache: crate::claude_sessions::SessionParseCache,
    /// Agent CLI id used when opening a NEW chat from the Projects tab
    /// (`[projects] default_chat_agent`, one of `projects::CHAT_AGENTS`).
    /// Resuming existing chats always uses claude regardless of this value.
    pub default_chat_agent: String,
    /// Footer "actives" toggle (`[projects] actives_only`, default ON): the
    /// Projects tab lists only chats currently open as tabs.
    pub projects_actives_only: bool,
    pub request_complete_onboarding: bool,
    pub name_input: String,
    pub name_input_replace_on_type: bool,
    pub release_notes: Option<ReleaseNotesState>,
    pub product_announcement: Option<ProductAnnouncementState>,
    pub keybind_help: KeybindHelpState,
    pub navigator: NavigatorState,
    pub copy_mode: Option<CopyModeState>,
    /// Which content the sidebar's top section shows (Spaces/Projects/Files).
    pub sidebar_tab: SidebarTab,
    pub workspace_scroll: usize,
    pub agent_panel_scroll: usize,
    /// Top-anchored row offset for the Projects sidebar tab (and the pattern
    /// the future Files tab reuses). Clamped in `compute_view` because the
    /// projects list length changes underneath it via the session polls.
    pub projects_scroll: usize,
    pub tab_scroll: usize,
    pub tab_scroll_follow_active: bool,
    pub mobile_switcher_scroll: usize,
    /// Which mobile drawer is open, if any.
    pub mobile_drawer: MobileDrawer,
    /// Set while Herdr has released mouse capture so the client's own
    /// selection gesture works, holding the capture setting to restore.
    ///
    /// With mouse reporting on, an iOS terminal's press-and-hold selection is
    /// suppressed: the drag goes to the application, and the client's handles
    /// never appear. Turning reporting off hands the gesture back, and the
    /// copy lands on the phone's clipboard through OSC 52 — which is the
    /// clipboard path Herdr already takes over SSH. This is the same trick the
    /// tmux world spells `set -g mouse off`.
    pub mobile_select_mode: Option<bool>,
    /// Document row of the open drawer's keyboard cursor.
    ///
    /// A drawer is a touch surface, and on the clients this is built for a tap
    /// is not a reliable click: iOS terminals bind long-press and drag to their
    /// own gestures, so what reaches Herdr is a keystroke. Every row a finger
    /// can reach has to be reachable from the keyboard too, or the drawer's
    /// whole purpose is unavailable on the platform it was written for.
    pub mobile_drawer_cursor: usize,
    /// Whether the reader folded away the active workspace's chats on a phone.
    ///
    /// The phone shell opens them by default because it draws no cell small
    /// enough to press instead (TP-MOB-67); this remembers a deliberate fold.
    /// It is client presentation state, not a shared session fact, so it does
    /// not join `expanded_chat_workspaces` — that set is the desktop's
    /// per-workspace preference and folding on a phone must not rewrite it.
    pub mobile_active_chats_folded: bool,
    // View geometry (computed before render, consumed by render + mouse)
    pub view: ViewState,
    /// Transient shell capture/preview state. Never persisted and never owns
    /// runtime resources.
    pub(crate) shell_interaction: crate::ui::shell::ShellInteractionState,
    /// Committed client-local shell presentation preferences. SF3.3 persists
    /// this aggregate through the versioned shell snapshot contract.
    pub(crate) shell_presentation: crate::ui::shell::ShellPresentationState,
    /// What clicking each part of each edge bar does, derived from config.
    ///
    /// Kept out of `shell_presentation` because that aggregate feeds the
    /// geometry cache key and these do not decide geometry: a command line
    /// belongs to what a click means, not to where a rectangle is. Derived
    /// rather than persisted, for the same reason the bars themselves are —
    /// config is the source, and writing it to the session file would let the
    /// disk disagree with the file the person edits (CLA6).
    pub(crate) shell_bar_chrome: crate::ui::shell::ShellBarChrome,
    /// The machine's last reading, as data. Render reads it and never fills it:
    /// a draw that could sample would sample once per frame, which is the cost
    /// this whole seam exists to avoid.
    pub(crate) resources: crate::resource::ResourceSample,
    pub(crate) drag: Option<DragState>,
    pub(crate) workspace_press: Option<WorkspacePressState>,
    pub(crate) tab_press: Option<TabPressState>,
    pub selection: Option<Selection>,
    pub selection_autoscroll: Option<SelectionAutoscroll>,
    pub context_menu: Option<ContextMenuState>,
    // Notifications
    pub update_available: Option<String>,
    pub update_install_command: String,
    pub latest_release_notes_available: bool,
    pub update_dismissed: bool,
    pub config_diagnostic: Option<String>,
    pub toast: Option<ToastNotification>,
    pub pending_agent_notifications: std::collections::HashMap<PaneId, PendingAgentNotification>,
    pub copy_feedback: Option<CopyFeedback>,
    /// Last reported focus state for the outer terminal hosting herdr.
    /// None means unsupported or not yet reported, which preserves active-pane suppression.
    pub outer_terminal_focus: Option<bool>,
    // Config
    pub prefix_code: KeyCode,
    pub prefix_mods: KeyModifiers,
    pub default_sidebar_width: u16,
    pub sidebar_width: u16,
    pub sidebar_min_width: u16,
    pub sidebar_max_width: u16,
    pub mobile_width_threshold: u16,
    pub sidebar_width_source: SidebarWidthSource,
    pub sidebar_width_auto: bool,
    pub sidebar_collapsed: bool,
    /// Set when the person expands the sidebar themselves.
    ///
    /// A short viewport folds the sidebar to its status rail on its own. That
    /// is the right default and the wrong override: someone who deliberately
    /// opened the sidebar on a fourteen-row terminal has answered the question
    /// the heuristic was guessing at, and the guess must stop arguing.
    pub sidebar_expanded_explicitly: bool,
    pub sidebar_collapsed_mode: crate::config::SidebarCollapsedModeConfig,
    /// Ratio of sidebar height allocated to the workspaces section.
    pub sidebar_section_split: f32,
    /// Whether each half of the left panel wears a frame. Travels beside the
    /// split ratio because the same function projects both section rectangles
    /// from the pair.
    pub sidebar_chrome: crate::ui::shell::SidebarChrome,
    pub agent_panel_sort: AgentPanelSort,
    /// Which of the drawer modes governs `workspace_chat_drawer_collapsed`.
    pub chat_drawer_mode: ChatDrawerMode,
    /// Transient session-wide projection override for the built-in Agents view.
    pub agent_view_override: Option<crate::api::schema::AgentViewSetParams>,
    pub sidebar_agents: crate::config::AgentsSidebarConfig,
    pub sidebar_spaces: crate::config::SpacesSidebarConfig,
    pub next_agent_state_change_seq: u64,
    /// Capture mouse input for Herdr's own mouse UI. When false, Herdr only
    /// captures mouse while the focused pane app requests mouse reporting.
    pub mouse_capture: bool,
    pub copy_on_select: bool,
    pub right_click_passthrough_modifiers: Option<KeyModifiers>,
    pub right_click_passthrough: Option<RightClickPassthroughGesture>,
    pub redraw_on_focus_gained: bool,
    pub mouse_scroll_lines: usize,
    pub confirm_close: bool,
    pub prompt_new_tab_name: bool,
    pub prompt_new_workspace_name: bool,
    pub pane_borders: bool,
    pub pane_gaps: bool,
    pub show_agent_labels_on_pane_borders: bool,
    pub hide_tab_bar_when_single_tab: bool,
    /// Whether each display holds its own workspace, tab and focused pane.
    /// False mirrors one view onto every display, which is the behaviour
    /// this feature replaced and the way back to it.
    pub per_display_focus: bool,
    pub pane_history_persistence: bool,
    /// Expose the focused pane's cursor anchor to the outer terminal even when
    /// the pane requested `?25l`. See `[experimental] reveal_hidden_cursor_for_cjk_ime`.
    pub reveal_hidden_cursor_for_cjk_ime: bool,
    /// Restrict cursor reveal to focused panes whose detected agent matches
    /// one of these. When false, apply to any focused pane.
    pub cjk_ime_agent_filter_configured: bool,
    pub cjk_ime_agents: Vec<crate::detect::Agent>,
    /// DECSCUSR shape parameter (1–6) for the IME anchor cursor.
    pub cjk_ime_cursor_shape: u8,
    /// While prefix mode is active, switch the macOS host input source to an
    /// ASCII-capable layout so prefix commands register as ASCII even when a
    /// CJK IME is active. macOS only; a no-op elsewhere. See
    /// `[experimental] switch_ascii_input_source_in_prefix`.
    pub switch_ascii_input_source_in_prefix: bool,
    pub kitty_graphics_enabled: bool,
    pub default_shell: String,
    pub shell_mode: crate::config::ShellModeConfig,
    pub new_terminal_cwd: NewTerminalCwdConfig,
    pub pane_scrollback_limit_bytes: usize,
    #[allow(dead_code)] // kept for backward compat; palette.accent is the source of truth
    pub accent: Color,
    pub sound: SoundConfig,
    pub local_sound_playback: bool,
    pub toast_config: ToastConfig,
    pub keybinds: Keybinds,
    /// Frame counter for spinner animations (wraps around).
    pub spinner_tick: u32,
    /// UI color palette — all sidebar/UI colors centralized for theming.
    pub palette: Palette,
    /// Currently applied theme name (for settings UI).
    pub theme_name: String,
    /// Runtime theme configuration used to resolve manual and auto-switch palettes.
    pub theme_runtime: ThemeRuntimeConfig,
    /// Last known foreground host terminal appearance.
    pub host_terminal_appearance: Option<HostAppearance>,
    /// True when the foreground host explicitly reported appearance via Mode 2031.
    pub host_terminal_appearance_explicit: bool,
    /// Settings panel state.
    pub settings: SettingsState,
    /// Cached integration recommendations for onboarding/settings UI.
    pub integration_recommendations: Vec<crate::integration::IntegrationRecommendation>,
    /// Cached detection manifest source/version summaries for runtime/API status.
    pub agent_manifest_summaries: Vec<crate::detect::manifest::AgentManifestSummary>,
    /// Cached remote detection manifest update diagnostics for runtime/API status.
    pub agent_manifest_update_status: crate::detect::manifest_update::ManifestUpdateStatus,
    /// Result messages from the latest integration install action.
    pub integration_install_messages: Vec<String>,
    /// Installed or linked plugins known to this running Herdr instance.
    pub(crate) installed_plugins: InstalledPluginRegistry,
    /// Pane ids opened through the plugin pane API.
    pub(crate) plugin_panes: std::collections::HashMap<PaneId, PluginPaneRecord>,
    /// Runtime image layers owned by API clients and composited over panes.
    pub(crate) pane_graphics_layers: std::collections::HashMap<PaneId, PaneGraphicsLayer>,
    /// Active streaming graphics owner token by pane id.
    pub(crate) pane_graphics_streams: std::collections::HashMap<PaneId, String>,
    /// Monotonic marker for accepted pane graphics mutations.
    pub(crate) pane_graphics_revision: u64,
    /// Session-modal terminal popup. This is intentionally outside workspace layouts.
    pub(crate) popup_pane: Option<PopupPaneState>,
    /// Recent plugin action/event command executions.
    pub(crate) plugin_command_logs: Vec<crate::api::schema::PluginCommandLogInfo>,
    pub(crate) next_plugin_command_log_id: u64,
    pub(crate) plugin_commands_in_flight: usize,
    /// Highlight state for the bottom-right global launcher menu.
    pub global_menu: MenuListState,
    /// Resolved host terminal default colors for theming embedded panes.
    pub host_terminal_theme: TerminalTheme,
    /// Last known foreground host terminal cell size in pixels.
    pub(crate) host_cell_size: crate::kitty_graphics::HostCellSize,
    /// Set when a persisted session snapshot would change.
    pub session_dirty: bool,
    /// Terminal runtimes that should be shut down by the app/runtime layer
    /// after state has detached their terminal metadata.
    pub(crate) terminal_runtime_shutdowns: Vec<crate::terminal::TerminalId>,
}

impl AppState {
    /// The client whose view is currently being resolved, if any.
    // The window is written by production and only read back by the tests that
    // pin the context contract; the per-client tab resolution reads it next.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn viewer(&self) -> Option<ClientId> {
        self.viewer
    }

    /// Enters `viewer`'s view and returns the previous one so the caller can
    /// put it back.
    ///
    /// Callers must pair this with [`AppState::restore_viewer`] on every exit
    /// path. The pairing is kept structural — one wrapper enters, calls an
    /// inner function that owns all the early returns, and restores — so an
    /// early return cannot leave another client's view installed.
    pub(crate) fn enter_viewer(&mut self, viewer: Option<ClientId>) -> Option<ClientId> {
        let previous = self.viewer;
        self.set_viewer(viewer);
        previous
    }

    /// Puts back the viewer returned by [`AppState::enter_viewer`].
    pub(crate) fn restore_viewer(&mut self, previous: Option<ClientId>) {
        self.set_viewer(previous);
    }

    /// Mirrors the viewer into every workspace.
    ///
    /// `Workspace` owns the per-client tab map but has no path back to
    /// `AppState`, so the id is pushed down instead of being threaded through
    /// every accessor call site. This is the only writer; the field is private
    /// on both sides so no other code can install a view.
    ///
    /// A workspace created *during* a viewer window keeps `None` until the next
    /// window opens. That is correct rather than merely tolerable: a brand-new
    /// workspace has one tab, so the default it falls back to is the same tab
    /// the viewer would have resolved to.
    fn set_viewer(&mut self, viewer: Option<ClientId>) {
        // Mirror mode is the way back to one view on every display. Recording
        // no viewer at all is what turns it off: with `viewer` left `None`,
        // every accessor resolves the shared default, which is exactly the
        // behaviour this feature replaced.
        if !self.per_display_focus {
            return;
        }
        // Read before anything is inserted: a display attaching right now must
        // not make the display that was alone stop being the session halfway
        // through this swap, or what it was holding is parked in a slot it
        // will never look in again.
        let sole_display = self.sole_display();

        // Park the serving display's surfaces before installing the incoming
        // display's. The fields are registers, not storage, so this is a
        // context switch rather than a set of assignments.
        let outgoing = self.take_surfaces();
        match self.viewer {
            Some(previous) => {
                let parked = self.surfaces_by_client.insert(previous, outgoing.clone());
                // Only what this display actually moved counts as driving the
                // session, and so as what a display attaching later adopts.
                self.promote_changed_surfaces(parked.as_ref(), &outgoing);
            }
            // Nothing was being served, so these registers hold whatever the
            // session itself last set — an API call, a restore, a startup.
            // That is a switch by definition and belongs in the default whole,
            // and for broadcast surfaces it belongs to every display too.
            None => {
                self.broadcast_session_changes(&outgoing);
                self.default_surfaces = outgoing;
            }
        }

        let incoming = match viewer {
            // Seeded rather than read, and left in place rather than taken.
            //
            // Seeded: a display adopts the default the first time it is seen
            // and is its own from then on. Without that, a display nobody has
            // touched keeps resolving the default, and the default follows
            // whoever switched last — which is the shared-focus complaint
            // wearing a different hat.
            //
            // Left in place: the entry is what the next park compares against
            // to decide whether this display actually moved. Taking it out
            // makes every park look like a change, and the default — the value
            // the API and the notification path resolve through — is then
            // overwritten on every frame by displays that did nothing.
            Some(client) => match self.surfaces_by_client.get(&client) {
                Some(parked) => parked.clone(),
                None => {
                    let adopted = self.adopted_surfaces();
                    self.surfaces_by_client.insert(client, adopted.clone());
                    adopted
                }
            },
            None => self.default_surfaces.clone(),
        };
        self.install_surfaces(incoming);

        // The owned half is handed over rather than copied, so the park and
        // the install are both moves.
        //
        // With one display, that display *is* the session: the register, the
        // no-viewer state and that display's state are one thing, so they
        // share a single slot rather than being kept in step. Every monolithic
        // run is this case, and it behaves exactly as it did before any of
        // this existed. The moment a second display attaches the slots
        // separate and the two browse independently.
        let outgoing_owned = self.take_owned();
        match self.viewer.or(sole_display) {
            Some(owner) => {
                self.owned_by_client.insert(owner, outgoing_owned);
            }
            None => self.default_owned = outgoing_owned,
        }
        let first_display = self.surfaces_by_client.len() <= 1;
        let incoming_owned = match viewer.or(sole_display) {
            Some(owner) => self.owned_by_client.remove(&owner).unwrap_or_else(|| {
                if first_display {
                    // The first display served takes over what the session
                    // had open before any display existed.
                    std::mem::take(&mut self.default_owned)
                } else {
                    // A display attaching later has not opened a file browser,
                    // is not halfway through a drag and is holding no picker.
                    // Handing it one is the shared-focus complaint arriving at
                    // the moment a monitor is plugged in. TP-SUR-ADOPT-01
                    ClientOwned::default()
                }
            }),
            None => std::mem::take(&mut self.default_owned),
        };
        self.install_owned(incoming_owned);

        self.reconcile_surfaces_with_session();

        self.viewer = viewer;
        for workspace in &mut self.workspaces {
            workspace.set_viewer(viewer);
        }
    }

    /// Drops every view a departed client held.
    ///
    /// A slot left behind keeps an index that tab removals still have to
    /// maintain, and it would still count as a viewer when a tab negotiates
    /// its size against the displays actually watching it.
    ///
    /// TP-MCF-TAB-02
    pub(crate) fn forget_client(&mut self, client: ClientId) {
        self.surfaces_by_client.remove(&client);
        self.owned_by_client.remove(&client);
        for workspace in &mut self.workspaces {
            workspace.forget_client(client);
        }
    }

    /// Repairs a freshly installed bundle against facts the session has since
    /// changed.
    ///
    /// A parked bundle can name a state that was true when the display was
    /// last served and is not any more. The one that bites: a display that
    /// attached before the session had any workspace parked "no workspace",
    /// and would keep resolving that forever — its renders then take the
    /// workspace-less path and resize live panes to a fallback area.
    ///
    /// TP-MCF-WS-03
    fn reconcile_surfaces_with_session(&mut self) {
        if self.active.is_none() {
            // An empty slot is the absence of a choice, not a choice. Fill it
            // from the default only — never invent a workspace. "No active
            // workspace" is a real state the session can be in, and forcing a
            // workspace here makes a background pane look foreground, which
            // silently suppresses its agent notifications.
            self.active = self.default_surfaces.active;
        }
    }

    /// Every display the scheduled work has to be run for, in a stable order.
    ///
    /// Workers run outside every display's window, where the registers hold
    /// the session's own view rather than any display's. That was fine while
    /// there was one file browser; now there is one per display, and a worker
    /// that only ever sees the registers refreshes a listing nobody is looking
    /// at while the ones on screen go stale.
    ///
    /// `None` is the session itself, and it is the whole list when no display
    /// has been served — every monolithic run, and every test that never
    /// attaches one.
    ///
    /// TP-SUR-FM-02
    pub(crate) fn displays_to_serve(&self) -> Vec<Option<ClientId>> {
        if self.surfaces_by_client.is_empty() {
            return vec![None];
        }
        let mut displays: Vec<ClientId> = self.surfaces_by_client.keys().copied().collect();
        // Stable so a worker's turn does not depend on hash order, which would
        // make which display wins a race change from run to run.
        displays.sort_unstable();
        displays.into_iter().map(Some).collect()
    }

    /// The one display there is, if there is exactly one.
    ///
    /// Not "the display being served": this answers whether the session and a
    /// display are the same thing, which is what lets them share one slot for
    /// the surfaces that are handed over rather than copied.
    /// Whether more than one display is attached.
    ///
    /// Below that, the session and the single display are the same thing and
    /// share one of everything.
    pub(crate) fn has_several_displays(&self) -> bool {
        self.surfaces_by_client.len() > 1
    }

    pub(crate) fn sole_display(&self) -> Option<ClientId> {
        let mut clients = self.surfaces_by_client.keys();
        match (clients.next(), clients.next()) {
            (Some(only), None) => Some(*only),
            _ => None,
        }
    }

    /// Whether any display is looking at the Files surface.
    ///
    /// The filesystem watcher, the preview worker and the IO worker all run
    /// outside every display's window, where the registers hold the session
    /// default rather than any particular display's view. Asking the default
    /// whether Files is open would stop the listing from refreshing the moment
    /// the default happened to name the terminal — while a display sat in
    /// Files watching a directory quietly go stale.
    ///
    /// TP-SUR-STAGE-03
    pub(crate) fn files_generation_in_use(&self) -> Option<u32> {
        let showing = |stage: &crate::ui::surface_host::StageState| {
            (stage.surface_view() == crate::ui::surface_host::StageSurfaceView::NativeFiles)
                .then(|| stage.active_instance_generation())
                .flatten()
        };
        showing(&self.stage)
            .or_else(|| showing(&self.default_owned.stage))
            .or_else(|| {
                self.owned_by_client
                    .values()
                    .find_map(|s| showing(&s.stage))
            })
    }

    pub(crate) fn mark_session_dirty(&mut self) {
        self.session_dirty = true;
    }

    pub(crate) fn remove_alias_shadowed_by_new_pane(&mut self, pane_id: PaneId) {
        self.pane_id_aliases.remove(&pane_id.raw());
    }

    pub fn sound_enabled(&self) -> bool {
        self.sound.enabled
    }

    pub fn toast_delivery(&self) -> ToastDelivery {
        self.toast_config.delivery
    }

    pub fn agent_border_labels_enabled(&self) -> bool {
        self.show_agent_labels_on_pane_borders
    }

    pub fn pane_history_persistence_enabled(&self) -> bool {
        self.pane_history_persistence
    }

    pub fn switch_ascii_input_source_in_prefix_enabled(&self) -> bool {
        self.switch_ascii_input_source_in_prefix
    }

    pub(crate) fn pane_exposes_host_cursor(
        &self,
        _ws_idx: usize,
        _pane_id: crate::layout::PaneId,
    ) -> bool {
        true
    }

    pub(crate) fn integration_updates_available(&self) -> bool {
        self.integration_recommendations
            .iter()
            .any(|item| item.state == crate::integration::IntegrationStatusKind::Outdated)
    }

    pub(crate) fn refresh_agent_manifest_summaries(&mut self) {
        self.agent_manifest_summaries = crate::detect::manifest::manifest_summaries();
    }

    /// Rebuild the Projects-tab chat cache from `projects_dir` (the
    /// `.../.claude/projects` root, injected for testability). This is the only
    /// place the reader touches the filesystem — render/compute must never scan
    /// the disk. Best-effort: a project with no chats keeps an empty list.
    /// Fill in the chat drawer's titles and ages from the agent's own store.
    ///
    /// The ledger records an association, not a transcript: the title lives in
    /// the agent's store, which is keyed by the directory the agent was
    /// launched in. So each workspace's own directory is read and whatever
    /// matches is filled in. A chat started elsewhere stays untitled by design
    /// — the drawer degrades to a short id rather than hiding the association,
    /// which is the part that cannot be recovered from anywhere else.
    /// Fill each workspace's drawer from the agent's own transcript store,
    /// merged with what the ledger observed, newest first.
    ///
    /// TP-DRAW-01/02: the two sources answer different questions and neither
    /// alone is enough. The store holds every chat ever started in a
    /// workspace's directory — measured 2026-07-30, 1510 of them across the
    /// open workspaces, of which the ledger knew 14, because the ledger only
    /// began recording the day it was written. The ledger in turn holds chats
    /// the store cannot be asked for: a chat that started elsewhere and moved
    /// in is filed under the directory it started in, not this one (measured:
    /// one worktree had 1 chat in its own store directory and 4 in the ledger).
    ///
    /// So: union, keyed by session id, store winning on title and mtime.
    pub(crate) fn merge_workspace_chat_rows_in(&mut self, projects_dir: &std::path::Path) {
        // Only slightly more than the drawer shows: parsing opens whole files,
        // and a busy directory holds hundreds of them.
        const DRAWER_FETCH_LIMIT: usize = 12;
        // TP-DRAW-10: a drawer opened all the way reads deeper, because
        // otherwise "show older" would promise chats the parse never fetched.
        // Still bounded: parsing opens whole files.
        const DRAWER_FETCH_LIMIT_FULL: usize = 60;
        let mut keys: Vec<(String, String)> = self
            .workspaces
            .iter()
            .map(|ws| {
                (
                    crate::persist::workspace_chats::ledger_key(ws.effective_cwd()),
                    ws.effective_cwd().to_string_lossy().into_owned(),
                )
            })
            .collect();

        // TP-DAILY-01: the daily directory is read whether or not anything
        // lives there. Every other key on this list is here because a
        // workspace asked for it; this one is here because nothing ever will
        // — since TP-WSID-01 made `effective_cwd` prefer the checkout, no
        // workspace holds `$HOME`, and the chats started there had become
        // reachable from nowhere in the sidebar.
        //
        // Pushed only when the list does not already carry that key: a
        // workspace standing in the same directory produces the same key, and
        // reading one directory twice in one pass buys nothing. It no longer
        // silences the section — see TP-DAILY-09.
        if let Some(daily) = self.daily_chat_cwd.clone() {
            let key = crate::persist::workspace_chats::ledger_key(&daily);
            if !keys.iter().any(|(existing, _)| existing == &key) {
                keys.push((key, daily.to_string_lossy().into_owned()));
            }
        }

        for (key, cwd) in &keys {
            let limit = if self.fully_open_chat_drawers.contains(key) {
                DRAWER_FETCH_LIMIT_FULL
            } else {
                DRAWER_FETCH_LIMIT
            };
            let (sessions, _) = crate::claude_sessions::read_recent_sessions_for_project_cached(
                projects_dir,
                cwd,
                limit,
                &mut self.sessions_parse_cache,
            );
            let rows = self.workspace_chat_rows.entry(key.clone()).or_default();
            for session in &sessions {
                match rows.iter_mut().find(|row| row.session_id == session.id) {
                    // TP-DRAW-03: one row per chat. The store is authoritative
                    // for what a chat is called and when it last moved.
                    Some(row) => {
                        row.title = Some(session.title.clone());
                        row.last_modified = Some(session.last_modified);
                    }
                    None => rows.push(WorkspaceChatRow {
                        session_id: session.id.clone(),
                        agent: "claude".to_string(),
                        title: Some(session.title.clone()),
                        last_seen_ms: system_time_to_ms(session.last_modified),
                        last_modified: Some(session.last_modified),
                    }),
                }
            }
        }

        // TP-DRAW-06: a chat the ledger saw but this workspace's own directory
        // does not hold lives under whichever directory it started in. Look for
        // it in the other open workspaces' directories rather than leaving the
        // row as a bare id — the association is real, only the filing differs.
        // TP-DRAW-07: only rows that are still untitled cost a read.
        let unresolved: Vec<String> = self
            .workspace_chat_rows
            .values()
            .flatten()
            .filter(|row| row.title.is_none())
            .map(|row| row.session_id.clone())
            .collect();
        if !unresolved.is_empty() {
            let mut found: std::collections::HashMap<
                String,
                crate::claude_sessions::ClaudeSession,
            > = std::collections::HashMap::new();
            for (_, cwd) in &keys {
                let (sessions, _) = crate::claude_sessions::read_recent_sessions_for_project_cached(
                    projects_dir,
                    cwd,
                    DRAWER_FETCH_LIMIT,
                    &mut self.sessions_parse_cache,
                );
                for session in sessions {
                    if unresolved.contains(&session.id) {
                        found.entry(session.id.clone()).or_insert(session);
                    }
                }
            }
            for row in self.workspace_chat_rows.values_mut().flatten() {
                if row.title.is_some() {
                    continue;
                }
                if let Some(session) = found.get(&row.session_id) {
                    row.title = Some(session.title.clone());
                    row.last_modified = Some(session.last_modified);
                }
            }
        }

        // TP-DRAW-04: newest first, the order the Projects tab already uses.
        // A row whose transcript was never located still sorts, on the moment
        // the ledger last saw it.
        for rows in self.workspace_chat_rows.values_mut() {
            rows.sort_by(|a, b| b.sort_key().cmp(&a.sort_key()));
        }
    }

    pub(crate) fn refresh_project_sessions_in(&mut self, projects_dir: &std::path::Path) {
        // Parse only slightly more than the sidebar can show: opening a
        // session file reads it whole, and busy projects hold hundreds of
        // files / tens of MB — parsing them all froze the Projects tab.
        const PROJECT_SESSIONS_FETCH_LIMIT: usize = 8;
        let pinned = self.projects_pinned.clone();
        self.projects_sessions = pinned
            .iter()
            .map(|path| {
                let path_str = path.to_string_lossy();
                let (sessions, total_count) =
                    crate::claude_sessions::read_recent_sessions_for_project_cached(
                        projects_dir,
                        &path_str,
                        PROJECT_SESSIONS_FETCH_LIMIT,
                        &mut self.sessions_parse_cache,
                    );
                ProjectSessions {
                    path: path.clone(),
                    sessions,
                    total_count,
                }
            })
            .collect();
    }

    /// The (workspace, tab) already wired to Claude Code session
    /// `session_id`, if any. The Projects tab focuses that tab instead of
    /// resuming a duplicate; a closed tab takes its wiring with it.
    pub(crate) fn find_resumed_chat_tab(&self, session_id: &str) -> Option<(usize, usize)> {
        self.workspaces.iter().enumerate().find_map(|(ws_idx, ws)| {
            ws.tabs
                .iter()
                .position(|tab| tab.resumed_session_id.as_deref() == Some(session_id))
                .map(|tab_idx| (ws_idx, tab_idx))
        })
    }

    /// Whether `ws_idx`'s chat drawer is folded shut, under the configured
    /// drawer mode. The single evaluation gate: every surface that draws,
    /// counts, or toggles a drawer asks here.
    ///
    /// Closed is the default: opening every workspace at once would bury the
    /// workspace list the tab exists for.
    ///
    /// TP-DRAWER-03: `all-active` derives an open drawer for every workspace
    /// with a live agent entry, unless this display quieted it
    /// (TP-DRAWER-04); the expanded set still opens agent-less drawers by
    /// hand. `focused` and `manual` never derive — only the expanded set
    /// speaks there (TP-DRAWER-05). Both sets are per-display surfaces, so
    /// what this display derives or quiets moves no drawer on another one.
    pub(crate) fn chat_drawer_collapsed(&self, ws_idx: usize) -> bool {
        let Some(workspace) = self.workspaces.get(ws_idx) else {
            return true;
        };
        // TP-WSID-03: openness keys through the same directory the content
        // reads by — a drawer whose rows come from the checkout must not
        // track its open state under the birthplace.
        let key = crate::persist::workspace_chats::ledger_key(workspace.effective_cwd());
        let opened_by_hand = self.expanded_chat_workspaces.contains(&key);
        match self.chat_drawer_mode {
            ChatDrawerMode::AllActive => {
                let derived = !self.suppressed_chat_drawers.contains(&key)
                    && self.workspace_has_agent_entries(ws_idx);
                !(opened_by_hand || derived)
            }
            ChatDrawerMode::Focused | ChatDrawerMode::Manual => !opened_by_hand,
        }
    }

    /// Whether this display has folded `node_key`'s subtree away.
    ///
    /// TP-NODE-07: a fold recorded by the retired session-wide project set
    /// still reads as folded — the migration's one-way door — but every new
    /// fold and every unfold lives in the per-display set, so folding a node
    /// is a statement about one screen (TP-NODE-06, HP18).
    pub(crate) fn node_folded(&self, node_key: &str) -> bool {
        self.collapsed_space_keys
            .contains(&Self::node_fold_key(node_key))
            || self.collapsed_project_keys.contains(node_key)
    }

    pub(crate) fn fold_node(&mut self, node_key: String) {
        self.collapsed_space_keys
            .insert(Self::node_fold_key(&node_key));
        self.mark_session_dirty();
    }

    /// Withdraws a fold, wherever it was recorded, and reports whether
    /// anything changed. Removing a legacy record is deliberate: leaving it
    /// would re-fold every screen forever, which is the exact complaint the
    /// per-display migration exists to end.
    pub(crate) fn unfold_node(&mut self, node_key: &str) -> bool {
        let forward = self
            .collapsed_space_keys
            .remove(&Self::node_fold_key(node_key));
        let legacy = self.collapsed_project_keys.remove(node_key);
        if forward || legacy {
            self.mark_session_dirty();
        }
        forward || legacy
    }

    /// The per-display fold key for a tree node. The NUL byte cannot appear
    /// in a TOML string, so no user-authored space key can ever collide with
    /// a node's fold record.
    fn node_fold_key(node_key: &str) -> String {
        format!("\u{0}node:{node_key}")
    }

    /// Whether any pane of `ws_idx` would put a row in the agents panel —
    /// the panel's own has-an-agent criterion (an agent name, or a detected
    /// agent label), so the drawer derivation and the panel can never
    /// disagree about what "active agent" means.
    fn workspace_has_agent_entries(&self, ws_idx: usize) -> bool {
        let Some(workspace) = self.workspaces.get(ws_idx) else {
            return false;
        };
        workspace.tabs.iter().any(|tab| {
            tab.panes.values().any(|pane| {
                self.terminals
                    .get(&pane.attached_terminal_id)
                    .is_some_and(|terminal| {
                        terminal.agent_name.is_some() || terminal.effective_agent_label().is_some()
                    })
            })
        })
    }

    pub(crate) fn global_menu_attention_badge_visible(&self) -> bool {
        self.update_available.is_some() || self.integration_updates_available()
    }

    pub(crate) fn global_menu_item_has_badge(&self, item: &str) -> bool {
        (item == "update ready" && self.update_available.is_some())
            || (item == "settings" && self.integration_updates_available())
    }

    pub(crate) fn settings_section_has_badge(&self, section: SettingsSection) -> bool {
        section == SettingsSection::Integrations && self.integration_updates_available()
    }

    pub(crate) fn focused_pane_requests_mouse_capture_from(
        &self,
        terminal_runtimes: &crate::terminal::TerminalRuntimeRegistry,
    ) -> bool {
        self.mode == Mode::Terminal
            && self
                .active
                .and_then(|idx| self.focused_runtime_in_workspace(terminal_runtimes, idx))
                .and_then(crate::terminal::TerminalRuntime::input_state)
                .is_some_and(crate::pane::InputState::mouse_reporting_enabled)
    }

    pub(crate) fn should_capture_host_mouse_from(
        &self,
        terminal_runtimes: &crate::terminal::TerminalRuntimeRegistry,
    ) -> bool {
        self.mouse_capture
            || self.popup_pane.is_some()
            || self.focused_pane_requests_mouse_capture_from(terminal_runtimes)
    }

    pub fn is_prefix_key(&self, key: crate::input::TerminalKey) -> bool {
        crate::config::terminal_key_matches_combo(key, (self.prefix_code, self.prefix_mods))
    }

    pub fn estimate_pane_size(&self) -> (u16, u16) {
        if let Some(info) = self.view.pane_infos.first() {
            (info.rect.height, info.rect.width)
        } else {
            (24, 80)
        }
    }

    /// Returns true when the given (workspace, tab, pane) refers to the
    /// currently focused pane in the active workspace's active tab.
    pub(crate) fn runtime_for_pane_in_workspace<'a>(
        &'a self,
        terminal_runtimes: &'a crate::terminal::TerminalRuntimeRegistry,
        ws_idx: usize,
        pane_id: crate::layout::PaneId,
    ) -> Option<&'a crate::terminal::TerminalRuntime> {
        #[cfg(test)]
        if let Some(runtime) = self.workspaces.get(ws_idx)?.test_runtimes.get(&pane_id) {
            return Some(runtime);
        }
        #[cfg(test)]
        if let Some(runtime) = self
            .workspaces
            .get(ws_idx)?
            .tabs
            .iter()
            .find_map(|tab| tab.runtimes.get(&pane_id))
        {
            return Some(runtime);
        }
        let terminal_id = self.workspaces.get(ws_idx)?.terminal_id(pane_id)?;
        terminal_runtimes.get(terminal_id)
    }

    #[cfg(test)]
    pub(crate) fn runtime_for_pane<'a>(
        &'a self,
        terminal_runtimes: &'a crate::terminal::TerminalRuntimeRegistry,
        pane_id: crate::layout::PaneId,
    ) -> Option<&'a crate::terminal::TerminalRuntime> {
        self.workspaces.iter().find_map(|ws| {
            #[cfg(test)]
            if let Some(runtime) = ws.test_runtimes.get(&pane_id) {
                return Some(runtime);
            }
            #[cfg(test)]
            if let Some(runtime) = ws.tabs.iter().find_map(|tab| tab.runtimes.get(&pane_id)) {
                return Some(runtime);
            }
            let terminal_id = ws.terminal_id(pane_id)?;
            terminal_runtimes.get(terminal_id)
        })
    }

    pub(crate) fn focused_runtime_in_workspace<'a>(
        &'a self,
        terminal_runtimes: &'a crate::terminal::TerminalRuntimeRegistry,
        ws_idx: usize,
    ) -> Option<&'a crate::terminal::TerminalRuntime> {
        let ws = self.workspaces.get(ws_idx)?;
        let pane_id = ws.focused_pane_id()?;
        self.runtime_for_pane_in_workspace(terminal_runtimes, ws_idx, pane_id)
    }

    pub fn is_active_pane(
        &self,
        ws_idx: usize,
        tab_idx: usize,
        pane_id: crate::layout::PaneId,
    ) -> bool {
        let Some(active_ws_idx) = self.active else {
            return false;
        };
        if ws_idx != active_ws_idx {
            return false;
        }
        let Some(ws) = self.workspaces.get(ws_idx) else {
            return false;
        };
        if tab_idx != ws.active_tab_index() {
            return false;
        }
        ws.active_tab().map(|tab| tab.layout.focused()) == Some(pane_id)
    }
}

#[cfg(test)]
pub fn key_matches(
    key: &crossterm::event::KeyEvent,
    expected_code: KeyCode,
    expected_mods: KeyModifiers,
) -> bool {
    crate::config::terminal_key_matches_combo(
        crate::input::TerminalKey::from(*key),
        (expected_code, expected_mods),
    )
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
impl AppState {
    /// Create an AppState for testing — no channels, no PTYs.
    pub fn test_new() -> Self {
        Self {
            sidebar_chrome: crate::ui::shell::SidebarChrome::NONE,
            terminals: std::collections::HashMap::new(),
            closed_agents: Default::default(),
            direct_attach_resize_locks: std::collections::HashSet::new(),
            pane_id_aliases: std::collections::HashMap::new(),
            public_pane_id_aliases: std::collections::HashMap::new(),
            workspaces: Vec::new(),
            active: None,
            viewer: None,
            surfaces_by_client: std::collections::HashMap::new(),
            default_surfaces: ClientSurfaces::default(),
            owned_by_client: std::collections::HashMap::new(),
            default_owned: ClientOwned::default(),
            previous_pane_focus: None,
            selected: 0,
            mode: Mode::Navigate,
            overlay_return_mode: None,
            file_manager: None,
            stage: Default::default(),
            agent_attachment_picker: None,
            request_agent_attachment_delivery: None,
            file_manager_clipboard: Vec::new(),
            file_icon_profile: crate::fm::entry_kind::IconProfile::Nerd,
            preview_viewer: None,
            tailscale_pinned_devices: Vec::new(),
            tailscale_send: None,
            file_manager_operation: None,
            file_manager_delete_confirmation: None,
            file_manager_rename: None,
            request_file_manager_rename: None,
            request_file_manager_bulk_rename: None,
            request_file_manager_delete: None,
            request_file_manager_context_action: None,
            request_file_manager_agent_handoff: None,
            agent_reference_picker: None,
            file_manager_locations_model: FileManagerLocationsModel::default(),
            file_manager_locations: Default::default(),
            request_file_manager_location_navigation: None,
            should_quit: false,
            detach_exits: false,
            detach_requested: false,
            request_new_workspace: false,
            request_new_tab: false,
            request_new_linked_worktree: None,
            request_open_existing_worktree: None,
            pending_move_new_group: None,
            pending_new_module: None,
            pending_branch_module: None,
            chat_move_overrides: Default::default(),
            recent_move_targets: Vec::new(),
            request_chat_move: None,
            request_new_workspace_cwd: None,
            request_remove_linked_worktree: None,
            request_submit_worktree_create: false,
            request_submit_worktree_open: false,
            request_submit_worktree_remove: false,
            request_reload_config: false,
            request_client_config_reload: false,
            request_clipboard_write: None,
            request_project_chat_tab: None,
            request_preview_show: false,
            creating_new_tab: false,
            requested_new_tab_name: None,
            pending_workspace_create_cwd: None,
            rename_pane_target: None,
            worktree_create: None,
            worktree_open: None,
            worktree_remove: None,
            worktree_directory: std::path::PathBuf::from("/tmp/herdr-worktrees"),
            collapsed_space_keys: std::collections::HashSet::new(),
            collapsed_project_keys: std::collections::HashSet::new(),
            space_split_rules: Vec::new(),
            space_projects: Vec::new(),
            space_nodes: Vec::new(),
            managed_node_keys: std::collections::HashSet::new(),
            space_icons: Default::default(),
            projects_pinned: Vec::new(),
            projects_sessions: Vec::new(),
            preview_bindings: Vec::new(),
            preview_placement: crate::config::PreviewPlacement::default(),
            collapsed_project_paths: std::collections::HashSet::new(),
            workspace_chat_rows: std::collections::HashMap::new(),
            // Tests point this at a scratch directory when they mean to; the
            // default is no daily section at all, so every existing fixture
            // keeps producing exactly the list it produced before.
            daily_chat_cwd: None,
            spaces_focus_only: false,
            fully_open_chat_drawers: std::collections::HashSet::new(),
            expanded_chat_workspaces: std::collections::HashSet::new(),
            daily_section_collapsed: false,
            daily_section_expanded: false,
            suppressed_chat_drawers: std::collections::HashSet::new(),
            tab_branch_cache: std::collections::HashMap::new(),
            sessions_parse_cache: Default::default(),
            default_chat_agent: "claude".to_string(),
            // Test fixtures exercise the full (unfiltered) Projects list;
            // the production default (ON) comes from the config path in
            // `App::new` (`[projects] actives_only`, absent → true).
            projects_actives_only: false,
            request_complete_onboarding: false,
            name_input: String::new(),
            name_input_replace_on_type: false,
            release_notes: None,
            product_announcement: None,
            keybind_help: KeybindHelpState::default(),
            navigator: NavigatorState::default(),
            copy_mode: None,
            sidebar_tab: SidebarTab::Spaces,
            workspace_scroll: 0,
            agent_panel_scroll: 0,
            projects_scroll: 0,
            tab_scroll: 0,
            tab_scroll_follow_active: true,
            mobile_switcher_scroll: 0,
            mobile_drawer: MobileDrawer::None,
            mobile_drawer_cursor: 0,
            mobile_active_chats_folded: false,
            mobile_select_mode: None,
            view: ViewState {
                layout: ViewLayout::Desktop,
                shell: Default::default(),
                sidebar_rect: Rect::default(),
                workspace_card_areas: Vec::new(),
                workspace_chat_row_areas: Vec::new(),
                workspace_more_chats_areas: Vec::new(),
                daily_header_area: None,
                daily_chat_row_areas: Vec::new(),
                module_chat_row_areas: Vec::new(),
                daily_more_area: None,
                workspace_group_header_areas: Vec::new(),
                workspace_project_header_areas: Vec::new(),
                workspace_empty_module_areas: Vec::new(),
                sidebar_tab_hit_areas: Vec::new(),
                project_row_areas: Vec::new(),
                app_dock_entry_areas: Vec::new(),
                file_manager_locations: Default::default(),
                file_manager_miller: Default::default(),
                file_manager_trail: Default::default(),
                file_manager_row_areas: Vec::new(),
                file_manager_row_action_areas: Vec::new(),
                file_manager_header_action_areas: Vec::new(),
                preview_viewer_content_area: None,
                file_manager_action_bar: None,
                agent_attachment_action_area: None,
                agent_worktree_action_area: None,
                agent_attachment_picker_row_areas: Vec::new(),
                tab_bar_rect: Rect::default(),
                tab_hit_areas: Vec::new(),
                stage_tab_hit_areas: Vec::new(),
                tab_scroll_left_hit_area: Rect::default(),
                tab_scroll_right_hit_area: Rect::default(),
                new_tab_hit_area: Rect::default(),
                terminal_area: Rect::default(),
                mobile_header_rect: Rect::default(),
                mobile_header_hits: crate::ui::MobileHeaderHitAreas::default(),
                toast_hit_area: Rect::default(),
                pane_infos: Vec::new(),
                split_borders: Vec::new(),
            },
            shell_interaction: Default::default(),
            shell_presentation: crate::ui::shell::ShellPresentationState::new(26),
            shell_bar_chrome: crate::ui::shell::ShellBarChrome::default(),
            resources: crate::resource::ResourceSample::default(),
            drag: None,
            workspace_press: None,
            tab_press: None,
            selection: None,
            selection_autoscroll: None,
            context_menu: None,
            update_available: None,
            update_install_command: "herdr update".into(),
            latest_release_notes_available: false,
            update_dismissed: false,
            config_diagnostic: None,
            toast: None,
            pending_agent_notifications: std::collections::HashMap::new(),
            copy_feedback: None,
            outer_terminal_focus: None,
            prefix_code: KeyCode::Char('b'),
            prefix_mods: KeyModifiers::CONTROL,
            default_sidebar_width: 26,
            sidebar_width: 26,
            sidebar_min_width: 18,
            sidebar_max_width: 36,
            mobile_width_threshold: crate::config::DEFAULT_MOBILE_WIDTH_THRESHOLD,
            sidebar_width_source: SidebarWidthSource::ConfigDefault,
            sidebar_width_auto: false,
            sidebar_collapsed: false,
            sidebar_expanded_explicitly: false,
            sidebar_collapsed_mode: crate::config::SidebarCollapsedModeConfig::Compact,
            sidebar_section_split: 0.5,
            agent_panel_sort: AgentPanelSort::Spaces,
            chat_drawer_mode: ChatDrawerMode::AllActive,
            agent_view_override: None,
            sidebar_agents: crate::config::AgentsSidebarConfig::default(),
            sidebar_spaces: crate::config::SpacesSidebarConfig::default(),
            next_agent_state_change_seq: 0,
            mouse_capture: true,
            copy_on_select: true,
            right_click_passthrough_modifiers: None,
            right_click_passthrough: None,
            redraw_on_focus_gained: true,
            mouse_scroll_lines: crate::config::DEFAULT_MOUSE_SCROLL_LINES,
            confirm_close: true,
            prompt_new_tab_name: true,
            prompt_new_workspace_name: false,
            pane_borders: true,
            pane_gaps: false,
            show_agent_labels_on_pane_borders: false,
            hide_tab_bar_when_single_tab: false,
            per_display_focus: true,
            pane_history_persistence: false,
            reveal_hidden_cursor_for_cjk_ime: false,
            cjk_ime_agent_filter_configured: false,
            cjk_ime_agents: Vec::new(),
            cjk_ime_cursor_shape: 2, // steady_block
            switch_ascii_input_source_in_prefix: false,
            kitty_graphics_enabled: false,
            default_shell: String::new(),
            shell_mode: crate::config::ShellModeConfig::Auto,
            new_terminal_cwd: NewTerminalCwdConfig::Follow,
            pane_scrollback_limit_bytes: crate::config::DEFAULT_SCROLLBACK_LIMIT_BYTES,
            accent: Color::Cyan,
            sound: SoundConfig {
                enabled: false,
                ..SoundConfig::default()
            },
            local_sound_playback: false,
            toast_config: ToastConfig::default(),
            keybinds: Keybinds::default(),
            spinner_tick: 0,
            palette: Palette::catppuccin(),
            theme_name: "catppuccin".to_string(),
            theme_runtime: ThemeRuntimeConfig {
                manual_name: "catppuccin".to_string(),
                dark_name: "catppuccin".to_string(),
                light_name: "catppuccin-latte".to_string(),
                auto_switch: false,
                custom: None,
                legacy_accent: None,
            },
            host_terminal_appearance: None,
            host_terminal_appearance_explicit: false,
            settings: SettingsState {
                section: SettingsSection::Theme,
                list: SelectionListState::new(0),
                original_palette: None,
                original_theme: None,
            },
            integration_recommendations: Vec::new(),
            agent_manifest_summaries: Vec::new(),
            agent_manifest_update_status:
                crate::detect::manifest_update::ManifestUpdateStatus::default(),
            integration_install_messages: Vec::new(),
            installed_plugins: std::collections::HashMap::new(),
            plugin_panes: std::collections::HashMap::new(),
            pane_graphics_layers: std::collections::HashMap::new(),
            pane_graphics_streams: std::collections::HashMap::new(),
            pane_graphics_revision: 0,
            popup_pane: None,
            plugin_command_logs: Vec::new(),
            next_plugin_command_log_id: 1,
            plugin_commands_in_flight: 0,
            global_menu: MenuListState::new(0),
            host_terminal_theme: TerminalTheme::default(),
            host_cell_size: crate::kitty_graphics::HostCellSize::default(),
            session_dirty: false,
            terminal_runtime_shutdowns: Vec::new(),
        }
    }

    /// Populate missing `TerminalState` entries for every pane so tests that
    /// read or write terminal metadata don't need to manually create them.
    pub fn ensure_test_terminals(&mut self) {
        use crate::terminal::TerminalState;
        for ws in &self.workspaces {
            for tab in &ws.tabs {
                for pane in tab.panes.values() {
                    if !self.terminals.contains_key(&pane.attached_terminal_id) {
                        let cwd = ws.identity_cwd.clone();
                        self.terminals.insert(
                            pane.attached_terminal_id.clone(),
                            TerminalState::new(pane.attached_terminal_id.clone(), cwd),
                        );
                    }
                }
            }
        }
    }

    pub fn test_with_adversarial_identity_state() -> Self {
        let mut state = Self::test_new();
        state.workspaces = vec![crate::workspace::Workspace::test_adversarial_identity_state()];
        state.active = Some(0);
        state.selected = 0;
        state.ensure_test_terminals();
        state
    }

    pub fn assert_invariants_for_test(&self) {
        if self.workspaces.is_empty() {
            assert!(
                self.active.is_none(),
                "empty app state must not have active workspace {:?}",
                self.active
            );
            assert_eq!(
                self.selected, 0,
                "empty app state should keep selected workspace at 0"
            );
            assert!(
                self.pane_id_aliases.is_empty(),
                "empty app state must not keep raw pane aliases"
            );
            assert!(
                self.public_pane_id_aliases.is_empty(),
                "empty app state must not keep public pane aliases"
            );
            assert!(
                self.previous_pane_focus.is_none(),
                "empty app state must not keep previous pane focus"
            );
            assert!(
                self.plugin_panes.is_empty(),
                "empty app state must not keep plugin pane records"
            );
            assert!(
                self.pending_agent_notifications.is_empty(),
                "empty app state must not keep pending agent notifications"
            );
            assert!(
                self.copy_mode.is_none(),
                "empty app state must not keep copy mode"
            );
            assert!(
                self.rename_pane_target.is_none(),
                "empty app state must not keep rename pane target"
            );
            assert!(
                self.selection.is_none(),
                "empty app state must not keep text selection"
            );
            assert!(
                self.selection_autoscroll.is_none(),
                "empty app state must not keep selection autoscroll"
            );
            if let Some(toast) = &self.toast {
                assert!(
                    toast.target.is_none(),
                    "empty app state must not keep pane-targeted toast"
                );
            }
            assert!(
                self.right_click_passthrough.is_none(),
                "empty app state must not keep right-click passthrough gesture"
            );
            assert!(
                self.drag.is_none(),
                "empty app state must not keep drag state"
            );
            assert!(
                self.workspace_press.is_none(),
                "empty app state must not keep workspace press state"
            );
            assert!(
                self.tab_press.is_none(),
                "empty app state must not keep tab press state"
            );
            assert!(
                self.context_menu.is_none(),
                "empty app state must not keep context menu"
            );
            return;
        }

        assert!(
            self.selected < self.workspaces.len(),
            "selected workspace {} out of bounds for {} workspaces",
            self.selected,
            self.workspaces.len()
        );
        let active = self
            .active
            .expect("non-empty app state must have active workspace");
        assert!(
            active < self.workspaces.len(),
            "active workspace {} out of bounds for {} workspaces",
            active,
            self.workspaces.len()
        );

        let mut workspace_ids = std::collections::HashSet::new();
        let mut workspace_id_to_idx = std::collections::HashMap::new();
        let mut pane_ids = std::collections::HashSet::new();
        let mut attached_terminal_ids = std::collections::HashSet::new();
        for (ws_idx, ws) in self.workspaces.iter().enumerate() {
            assert!(
                workspace_ids.insert(ws.id.clone()),
                "duplicate workspace id {} at workspace index {}",
                ws.id,
                ws_idx
            );
            workspace_id_to_idx.insert(ws.id.clone(), ws_idx);
            ws.assert_invariants_for_test();

            for tab in &ws.tabs {
                for (pane_id, pane) in &tab.panes {
                    assert!(
                        pane_ids.insert(*pane_id),
                        "pane {:?} appears in more than one workspace",
                        pane_id
                    );
                    assert!(
                        attached_terminal_ids.insert(pane.attached_terminal_id.clone()),
                        "terminal {} is attached to more than one app pane",
                        pane.attached_terminal_id
                    );
                    assert!(
                        self.terminals.contains_key(&pane.attached_terminal_id),
                        "pane {:?} is attached to missing terminal {}",
                        pane_id,
                        pane.attached_terminal_id
                    );
                }
            }
        }

        let assert_live_pane = |pane_id: PaneId, context: &str| {
            assert!(
                pane_ids.contains(&pane_id),
                "{context} references missing pane {:?}",
                pane_id
            );
        };
        let assert_workspace_pane = |workspace_id: &str, pane_id: PaneId, context: &str| {
            let ws_idx = workspace_id_to_idx
                .get(workspace_id)
                .copied()
                .unwrap_or_else(|| panic!("{context} references missing workspace {workspace_id}"));
            assert!(
                self.workspaces[ws_idx].pane_state(pane_id).is_some(),
                "{context} references pane {:?} outside workspace {}",
                pane_id,
                workspace_id
            );
        };
        let assert_workspace_index = |ws_idx: usize, context: &str| {
            assert!(
                ws_idx < self.workspaces.len(),
                "{context} references workspace index {} out of bounds for {} workspaces",
                ws_idx,
                self.workspaces.len()
            );
        };
        let assert_tab_index = |ws_idx: usize, tab_idx: usize, context: &str| {
            assert_workspace_index(ws_idx, context);
            assert!(
                tab_idx < self.workspaces[ws_idx].tabs.len(),
                "{context} references tab index {} out of bounds for workspace {} with {} tabs",
                tab_idx,
                ws_idx,
                self.workspaces[ws_idx].tabs.len()
            );
        };

        for (&raw, &pane_id) in &self.pane_id_aliases {
            assert_live_pane(pane_id, &format!("raw pane alias {raw}"));
        }
        for (public_id, &pane_id) in &self.public_pane_id_aliases {
            assert_live_pane(pane_id, &format!("public pane alias {public_id}"));
        }
        if let Some(focus) = &self.previous_pane_focus {
            assert_workspace_pane(&focus.workspace_id, focus.pane_id, "previous pane focus");
        }
        if let Some(toast) = &self.toast {
            if let Some(target) = &toast.target {
                assert_workspace_pane(&target.workspace_id, target.pane_id, "toast target");
            }
        }
        for (&pane_id, notification) in &self.pending_agent_notifications {
            assert_eq!(
                pane_id, notification.pane_id,
                "pending agent notification map key must match payload pane id"
            );
            assert_workspace_pane(
                &notification.workspace_id,
                notification.pane_id,
                "pending agent notification",
            );
        }
        if let Some(popup) = &self.popup_pane {
            assert!(
                self.terminals.contains_key(&popup.terminal_id),
                "popup {:?} references missing terminal {}",
                popup.pane_id,
                popup.terminal_id
            );
            assert!(
                !attached_terminal_ids.contains(&popup.terminal_id),
                "popup terminal {} must not be attached to a tiled pane",
                popup.terminal_id
            );
        }
        for &pane_id in self.plugin_panes.keys() {
            assert_live_pane(pane_id, "plugin pane record");
        }
        if let Some(copy_mode) = &self.copy_mode {
            assert_live_pane(copy_mode.pane_id, "copy mode");
        }
        if let Some(pane_id) = self.rename_pane_target {
            assert_live_pane(pane_id, "rename pane target");
        }
        if let Some(selection) = &self.selection {
            assert_live_pane(selection.pane_id, "text selection");
        } else {
            assert!(
                self.selection_autoscroll.is_none(),
                "selection autoscroll must not remain without an active text selection"
            );
        }
        if let Some(gesture) = &self.right_click_passthrough {
            assert_live_pane(gesture.pane_info.id, "right-click passthrough gesture");
        }
        if let Some(drag) = &self.drag {
            match &drag.target {
                DragTarget::WorkspaceReorder {
                    source_ws_idx,
                    insert_idx,
                } => {
                    assert_workspace_index(*source_ws_idx, "workspace drag source");
                    if let Some(insert_idx) = insert_idx {
                        assert!(
                            *insert_idx <= self.workspaces.len(),
                            "workspace drag insert index {} out of bounds for {} workspaces",
                            insert_idx,
                            self.workspaces.len()
                        );
                    }
                }
                DragTarget::TabReorder {
                    ws_idx,
                    source_tab_idx,
                    insert_idx,
                } => {
                    assert_tab_index(*ws_idx, *source_tab_idx, "tab drag source");
                    if let Some(insert_idx) = insert_idx {
                        assert!(
                            *insert_idx <= self.workspaces[*ws_idx].tabs.len(),
                            "tab drag insert index {} out of bounds for workspace {} with {} tabs",
                            insert_idx,
                            ws_idx,
                            self.workspaces[*ws_idx].tabs.len()
                        );
                    }
                }
                DragTarget::PaneScrollbar { pane_id, .. } => {
                    assert_live_pane(*pane_id, "pane scrollbar drag")
                }
                _ => {}
            }
        }
        if let Some(press) = &self.workspace_press {
            assert_workspace_index(press.ws_idx, "workspace press");
        }
        if let Some(press) = &self.tab_press {
            assert_tab_index(press.ws_idx, press.tab_idx, "tab press");
        }
        if let Some(menu) = &self.context_menu {
            match menu.kind {
                ContextMenuKind::Workspace { ws_idx }
                | ContextMenuKind::GitWorkspace { ws_idx, .. }
                | ContextMenuKind::MoveWorkspace { ws_idx, .. }
                | ContextMenuKind::MoveTarget { ws_idx, .. } => {
                    assert_workspace_index(ws_idx, "context menu workspace")
                }
                // TP-CHAT-MOVE-08: a chat row names a drawer only when it was
                // pressed in one; a daily row was not, and there is no index
                // to check.
                ContextMenuKind::WorkspaceChat { ws_idx, .. } => {
                    if let Some(ws_idx) = ws_idx {
                        assert_workspace_index(ws_idx, "context menu chat workspace");
                    }
                }
                ContextMenuKind::ChatMoveTarget { .. } => {
                    // Carries a session id and pre-resolved ledger keys; no
                    // index-shaped identity to validate.
                }
                ContextMenuKind::Tab { ws_idx, tab_idx } => {
                    assert_tab_index(ws_idx, tab_idx, "context menu tab")
                }
                ContextMenuKind::WorkspaceNewChat { ws_idx, .. } => {
                    assert!(
                        ws_idx < self.workspaces.len(),
                        "workspace new-chat menu references workspace {} outside the list (len {})",
                        ws_idx,
                        self.workspaces.len()
                    );
                }
                // TP-DAILY-11/12 / TP-MOD-31: nothing index-shaped to
                // validate — these name a directory, a fold state, or no row
                // at all, and a refresh cannot invalidate any of them.
                ContextMenuKind::DailyNewChat
                | ContextMenuKind::SidebarBlank
                | ContextMenuKind::DailyHeader { .. } => {}
                ContextMenuKind::ProjectNewChat { proj_idx, .. } => {
                    assert!(
                        proj_idx < self.projects_sessions.len(),
                        "project new-chat menu references project {} outside the cache (len {})",
                        proj_idx,
                        self.projects_sessions.len()
                    );
                }
                ContextMenuKind::File { ref model } => {
                    assert!(
                        self.file_manager.is_some(),
                        "file context menu requires an open file manager"
                    );
                    assert!(
                        !model.paths.is_empty(),
                        "file context menu requires explicit prepared paths"
                    );
                }
                ContextMenuKind::Pane {
                    ws_idx,
                    tab_idx,
                    pane_id,
                    source_pane_id,
                    ..
                } => {
                    assert_tab_index(ws_idx, tab_idx, "context menu pane tab");
                    assert!(
                        self.workspaces[ws_idx].tabs[tab_idx]
                            .panes
                            .contains_key(&pane_id),
                        "context menu pane references pane {:?} outside workspace {} tab {}",
                        pane_id,
                        ws_idx,
                        tab_idx
                    );
                    if let Some(source_pane_id) = source_pane_id {
                        assert_live_pane(source_pane_id, "context menu source pane");
                    }
                }
                ContextMenuKind::AgentEntry {
                    ws_idx,
                    tab_idx,
                    pane_id,
                    ..
                } => {
                    // TP-AGPANEL-03: the row's target is the menu's identity —
                    // it must still name a live pane, or the close would fire
                    // at a slot something else has moved into.
                    assert_tab_index(ws_idx, tab_idx, "context menu agent tab");
                    assert!(
                        self.workspaces[ws_idx].tabs[tab_idx]
                            .panes
                            .contains_key(&pane_id),
                        "agent menu references pane {:?} outside workspace {} tab {}",
                        pane_id,
                        ws_idx,
                        tab_idx
                    );
                }
                ContextMenuKind::AppDock { .. } => {
                    // The dock popover references only a closed built-in app
                    // id; there is no index-shaped identity to validate.
                }
                ContextMenuKind::NodeHeader { .. } | ContextMenuKind::SpaceHeader { .. } => {
                    // Header menus carry config keys, not indices; a key that
                    // stops resolving degrades to a no-op menu, never to an
                    // out-of-bounds index (TP-DOTS-01).
                }
            }
        }
    }

    pub fn insert_test_runtime(
        &mut self,
        pane_id: crate::layout::PaneId,
        runtime: crate::terminal::TerminalRuntime,
    ) {
        if let Some(ws) = self
            .workspaces
            .iter_mut()
            .find(|ws| ws.terminal_id(pane_id).is_some())
        {
            ws.insert_test_runtime(pane_id, runtime);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
    use std::path::Path;

    struct AttachmentTempDir(PathBuf);

    impl AttachmentTempDir {
        fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "herdr-attachment-{tag}-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).expect("create attachment fixture");
            Self(path)
        }
    }

    impl Drop for AttachmentTempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn focused_agent_attachment_state(root: &Path) -> (AppState, PaneId) {
        let mut state = AppState::test_new();
        let mut workspace = crate::workspace::Workspace::test_new("attachment");
        workspace.identity_cwd = root.to_path_buf();
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace.tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        state.workspaces = vec![workspace];
        state.active = Some(0);
        state.selected = 0;
        state.mode = Mode::Terminal;
        state.ensure_test_terminals();
        state
            .terminals
            .get_mut(&terminal_id)
            .expect("focused terminal")
            .set_agent_name("codex".into());
        state.view.terminal_area = Rect::new(0, 0, 80, 24);
        (state, pane_id)
    }

    // TP-M1.2-OPEN: picker state binds the exact stable identities and starts
    // from the same workspace cwd authority as the native FM.
    #[test]
    fn opening_attachment_picker_binds_exact_target_and_workspace_cwd() {
        let root = AttachmentTempDir::new("open");
        let (mut state, pane_id) = focused_agent_attachment_state(&root.0);
        let workspace_id = state.workspaces[0].id.clone();
        let terminal_id = state.terminal_id_for_pane(0, pane_id).unwrap();

        assert_eq!(state.open_agent_attachment_picker(), Ok(()));

        let picker = state
            .agent_attachment_picker
            .as_ref()
            .expect("picker state");
        assert_eq!(picker.file_manager.cwd, root.0);
        assert_eq!(picker.target.workspace_id, workspace_id);
        assert_eq!(picker.target.pane_id, pane_id);
        assert_eq!(picker.target.terminal_id, terminal_id);
        assert_eq!(state.mode, Mode::AttachFile);
        assert_eq!(state.view.agent_attachment_action_area, None);
    }

    // TP-M1.2-TINY: incomplete modal geometry declines before allocating
    // picker/FM state and returns one stable visible-reason classification.
    #[test]
    fn attachment_picker_tiny_area_declines_with_visible_reason() {
        let root = AttachmentTempDir::new("tiny");
        let (mut state, _) = focused_agent_attachment_state(&root.0);
        state.view.terminal_area = Rect::new(0, 0, 17, 10);

        assert_eq!(
            state.open_agent_attachment_picker(),
            Err(AgentAttachmentOpenError::InsufficientSpace)
        );
        assert!(state.agent_attachment_picker.is_none());
        assert_eq!(state.mode, Mode::Terminal);
        let toast = state.toast.as_ref().expect("visible size failure");
        assert_eq!(toast.kind, ToastKind::NeedsAttention);
        assert_eq!(toast.title, "attach file unavailable");
        assert_eq!(toast.context, "attachment picker needs more terminal space");
    }

    // TP-M1.2-UNAVAILABLE: capability loss fails closed and gives the user a
    // stable reason instead of silently consuming the configured action.
    #[test]
    fn attachment_picker_unavailable_target_is_visible_and_non_mutating() {
        let root = AttachmentTempDir::new("unavailable");
        let (mut state, pane_id) = focused_agent_attachment_state(&root.0);
        let terminal_id = state.terminal_id_for_pane(0, pane_id).unwrap();
        state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_agent_name(String::new());

        assert_eq!(
            state.open_agent_attachment_picker(),
            Err(AgentAttachmentOpenError::Unavailable)
        );
        assert!(state.agent_attachment_picker.is_none());
        assert_eq!(state.mode, Mode::Terminal);
        let toast = state.toast.as_ref().expect("visible target failure");
        assert_eq!(toast.kind, ToastKind::NeedsAttention);
        assert_eq!(toast.title, "attach file unavailable");
        assert_eq!(toast.context, "focused pane is not an available agent");
    }

    // TP-M1.2-CANCEL: overlay cancellation owns no runtime resource and
    // restores terminal mode without preparing a delivery request.
    #[test]
    fn attachment_picker_escape_restores_valid_focus_without_delivery() {
        let root = AttachmentTempDir::new("cancel");
        let (mut state, pane_id) = focused_agent_attachment_state(&root.0);
        state.open_agent_attachment_picker().unwrap();

        state.close_agent_attachment_picker();

        assert!(state.agent_attachment_picker.is_none());
        assert_eq!(state.mode, Mode::Terminal);
        assert_eq!(state.workspaces[0].focused_pane_id(), Some(pane_id));
        assert!(state.request_agent_attachment_delivery.is_none());
    }

    // TP-M1.2-AUTHORITY: v1 exposes exactly one current regular UTF-8 file;
    // directories are navigation targets, never attachment authority.
    #[test]
    fn attachment_picker_accepts_one_regular_file_and_disables_other_targets() {
        let root = AttachmentTempDir::new("authority");
        let file = root.0.join("photo ünicode.png");
        let directory = root.0.join("folder");
        std::fs::write(&file, b"png").unwrap();
        std::fs::create_dir(&directory).unwrap();
        let (mut state, _) = focused_agent_attachment_state(&root.0);
        state.open_agent_attachment_picker().unwrap();

        let picker = state.agent_attachment_picker.as_mut().unwrap();
        picker.file_manager.cursor = picker
            .file_manager
            .entries
            .iter()
            .position(|entry| entry.path == file)
            .unwrap();
        assert_eq!(state.agent_attachment_selected_file(), Some(file));

        let picker = state.agent_attachment_picker.as_mut().unwrap();
        picker.file_manager.cursor = picker
            .file_manager
            .entries
            .iter()
            .position(|entry| entry.path == directory)
            .unwrap();
        assert_eq!(state.agent_attachment_selected_file(), None);
    }

    // TP-C6.1-MODEL/LIFECYCLE: filesystem discovery happens before render.
    // Existing well-known directories are kept in Finder order, missing
    // favorites are omitted, configured pins remain visible but marked
    // inaccessible, and duplicate path authority stays with the first section.
    #[test]
    fn file_locations_preparation_uses_live_home_and_pin_state() {
        use std::sync::atomic::{AtomicU64, Ordering};

        struct TempHome(PathBuf);
        impl Drop for TempHome {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let home = TempHome(std::env::temp_dir().join(format!(
            "herdr-sidebar-model-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )));
        std::fs::create_dir_all(home.0.join("Downloads")).expect("create Downloads");
        std::fs::create_dir_all(home.0.join("Documents")).expect("create Documents");
        std::fs::write(home.0.join("Desktop"), b"not a directory").expect("create non-dir Desktop");
        let missing_pin = home.0.join("missing-pin");

        let model = FileManagerLocationsModel::from_home_and_pins(
            &home.0,
            &[home.0.clone(), missing_pin.clone()],
        );

        let favorites = model
            .section(FileManagerLocationSectionKind::Favorites)
            .expect("favorites section");
        assert_eq!(
            favorites
                .items
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            ["Home", "Downloads", "Documents"]
        );
        assert!(favorites.items.iter().all(|item| item.accessible));

        let pinned = model
            .section(FileManagerLocationSectionKind::Pinned)
            .expect("inaccessible configured pin remains visible");
        assert_eq!(pinned.items.len(), 1, "Home duplicate is removed");
        assert_eq!(pinned.items[0].path, missing_pin);
        assert!(!pinned.items[0].accessible);

        let locations = model
            .section(FileManagerLocationSectionKind::Locations)
            .expect("root location");
        assert_eq!(locations.items[0].label, "Root");
        assert!(locations.items[0].path.is_absolute());
    }

    // TP-C6.1-MODEL: adversarial configuration cannot create an unbounded
    // client-side sidebar model or move later duplicates ahead of first use.
    #[test]
    fn file_locations_model_is_bounded_across_all_sections() {
        let items = (0..FILE_MANAGER_LOCATIONS_MAX_ITEMS + 32)
            .map(|index| FileManagerLocationItem {
                label: format!("item-{index}"),
                path: PathBuf::from(format!("/virtual/{index}")),
                icon: FileManagerLocationIcon::Pin,
                accessible: true,
                ejectable: false,
            })
            .collect();
        let model = FileManagerLocationsModel::from_sources(Vec::new(), items, Vec::new());

        assert_eq!(model.item_count(), FILE_MANAGER_LOCATIONS_MAX_ITEMS);
        assert_eq!(model.sections[0].items[0].path, PathBuf::from("/virtual/0"));
    }

    #[test]
    fn agent_terminal_keeps_final_child_cursor_exposed() {
        let mut state = AppState::test_new();
        let ws = crate::workspace::Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        state.terminals.insert(
            ws.tabs[0].panes[&pane_id].attached_terminal_id.clone(),
            crate::terminal::TerminalState::new(
                ws.tabs[0].panes[&pane_id].attached_terminal_id.clone(),
                std::path::PathBuf::from("/tmp"),
            ),
        );
        state
            .terminals
            .get_mut(&ws.tabs[0].panes[&pane_id].attached_terminal_id)
            .expect("terminal state")
            .launch_argv = Some(vec!["codex".to_string()]);
        state.workspaces = vec![ws];

        assert!(state.pane_exposes_host_cursor(0, pane_id));
    }

    #[test]
    fn adversarial_identity_state_satisfies_app_invariants_after_mutation() {
        let mut state = AppState::test_with_adversarial_identity_state();
        state.assert_invariants_for_test();

        let ws = &mut state.workspaces[0];
        let active_public = ws.tabs[ws.active_tab_index()].number;
        assert_ne!(ws.active_tab_index() + 1, active_public);
        let new_pane = ws.test_split(ratatui::layout::Direction::Horizontal);
        assert!(ws.public_pane_number(new_pane).is_some());
        state.ensure_test_terminals();

        state.assert_invariants_for_test();
    }

    fn navigator_row_for_display(is_workspace: bool) -> NavigatorRow {
        NavigatorRow {
            target: NavigatorTarget::Workspace { ws_idx: 0 },
            depth: if is_workspace { 0 } else { 1 },
            label: String::new(),
            meta: String::new(),
            status: crate::detect::AgentState::Idle,
            seen: true,
            is_current: false,
            is_workspace,
            is_tab: false,
            expanded: true,
            search_text: String::new(),
            matched: true,
        }
    }

    #[test]
    fn navigator_display_lines_separate_workspace_groups() {
        let rows = vec![
            navigator_row_for_display(true),
            navigator_row_for_display(false),
            navigator_row_for_display(true),
            navigator_row_for_display(false),
        ];
        assert_eq!(
            navigator_display_lines(&rows),
            vec![
                NavigatorDisplayLine::Row(0),
                NavigatorDisplayLine::Row(1),
                NavigatorDisplayLine::Spacer,
                NavigatorDisplayLine::Row(2),
                NavigatorDisplayLine::Row(3),
            ]
        );
    }

    #[test]
    fn navigator_display_lines_have_no_leading_spacer() {
        let rows = vec![
            navigator_row_for_display(true),
            navigator_row_for_display(false),
        ];
        assert_eq!(
            navigator_display_lines(&rows),
            vec![NavigatorDisplayLine::Row(0), NavigatorDisplayLine::Row(1)]
        );
        assert!(navigator_display_lines(&[]).is_empty());
    }

    #[test]
    fn navigator_display_index_maps_row_to_line() {
        let rows = vec![
            navigator_row_for_display(true),
            navigator_row_for_display(false),
            navigator_row_for_display(true),
        ];
        let lines = navigator_display_lines(&rows);
        assert_eq!(navigator_display_index_of_row(&lines, 2), Some(3));
        assert_eq!(navigator_display_index_of_row(&lines, 9), None);
    }

    #[test]
    fn navigator_first_row_skips_spacer_lines() {
        let rows = vec![
            navigator_row_for_display(true),
            navigator_row_for_display(false),
            navigator_row_for_display(true),
        ];
        let lines = navigator_display_lines(&rows);
        // Line 2 is the spacer before the second workspace.
        assert_eq!(navigator_first_row_at_or_after(&lines, 2), Some(2));
        assert_eq!(navigator_first_row_at_or_after(&lines, 4), None);
    }

    #[test]
    fn built_in_theme_names_resolve() {
        for name in THEME_NAMES {
            assert!(
                Palette::from_name(name).is_some(),
                "theme should resolve: {name}"
            );
        }
    }

    #[test]
    fn light_theme_aliases_resolve() {
        for name in ["light", "latte", "tokyo-day", "onelight", "lotus", "dawn"] {
            assert!(
                Palette::from_name(name).is_some(),
                "theme should resolve: {name}"
            );
        }
    }

    // ---- Projects tab cache (refresh_project_sessions*) ----------------------

    /// Isolated fake `.claude/projects` root; never touches the real `~/.claude`.
    struct FakeProjectsRoot {
        root: std::path::PathBuf,
    }

    impl FakeProjectsRoot {
        fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "herdr-state-cs-{}-{}-{}",
                std::process::id(),
                tag,
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&root).expect("create temp projects root");
            Self { root }
        }

        fn write_session(&self, project: &str, session_id: &str, lines: &[&str]) {
            use std::io::Write as _;
            let dir = self
                .root
                .join(crate::claude_sessions::encode_project_path(project));
            std::fs::create_dir_all(&dir).expect("create project dir");
            let path = dir.join(format!("{session_id}.jsonl"));
            let mut file = std::fs::File::create(&path).expect("create session file");
            for line in lines {
                writeln!(file, "{line}").expect("write session line");
            }
        }
    }

    impl Drop for FakeProjectsRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    // P2: a fresh AppState has empty Projects-tab state.
    #[test]
    fn test_new_projects_state_is_empty() {
        let state = AppState::test_new();
        assert!(state.projects_pinned.is_empty());
        assert!(state.projects_sessions.is_empty());
        assert!(state.collapsed_project_paths.is_empty());
    }

    /// A workspace at a scratch directory, plus its ledger key.
    fn drawer_probe_workspace(state: &mut AppState, tag: &str) -> (std::path::PathBuf, String) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let cwd = std::env::temp_dir().join(format!(
            "herdr-drawer-{}-{}-{}",
            std::process::id(),
            tag,
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&cwd).expect("probe workspace dir");
        let mut workspace = crate::workspace::Workspace::test_new(tag);
        workspace.identity_cwd = cwd.clone();
        state.workspaces.push(workspace);
        let key = crate::persist::workspace_chats::ledger_key(&cwd);
        (cwd, key)
    }

    // TP-DRAW-01: the drawer lists what the agent's own store holds, not only
    // what the ledger happened to witness. Measured 2026-07-30: the open
    // workspaces held 1510 transcripts between them and the ledger knew 14,
    // because the ledger only began recording the day it was written — so a
    // ledger-only drawer showed almost nothing.
    #[test]
    fn the_drawer_lists_chats_the_ledger_never_saw() {
        let fake = FakeProjectsRoot::new("store-only");
        let mut state = AppState::test_new();
        let (cwd, key) = drawer_probe_workspace(&mut state, "storeonly");
        for (id, title) in [("a", "first chat"), ("b", "second chat"), ("c", "third")] {
            fake.write_session(
                &cwd.to_string_lossy(),
                id,
                &[format!(r#"{{"type":"custom-title","customTitle":"{title}"}}"#).as_str()],
            );
        }

        assert!(
            !state.workspace_chat_rows.contains_key(&key),
            "the ledger has never seen this workspace"
        );
        state.merge_workspace_chat_rows_in(&fake.root);

        let rows = state.workspace_chat_rows.get(&key).expect("rows appear");
        assert_eq!(rows.len(), 3, "every stored chat is listed: {rows:?}");
        let _ = std::fs::remove_dir_all(&cwd);
    }

    // TP-DRAW-02 + TP-DRAW-03: union, not replacement, and one row per chat.
    // The ledger holds chats the store cannot be asked for — a chat that
    // started elsewhere is filed under the directory it started in (measured:
    // one worktree had 1 chat in its own directory and 4 in the ledger).
    #[test]
    fn the_drawer_unions_the_store_and_the_ledger_without_duplicating() {
        let fake = FakeProjectsRoot::new("union");
        let mut state = AppState::test_new();
        let (cwd, key) = drawer_probe_workspace(&mut state, "union");
        fake.write_session(
            &cwd.to_string_lossy(),
            "shared",
            &[r#"{"type":"custom-title","customTitle":"in both"}"#],
        );
        fake.write_session(
            &cwd.to_string_lossy(),
            "store-only",
            &[r#"{"type":"custom-title","customTitle":"store only"}"#],
        );
        state.workspace_chat_rows.insert(
            key.clone(),
            vec![
                WorkspaceChatRow {
                    session_id: "shared".into(),
                    agent: "claude".into(),
                    title: None,
                    last_seen_ms: 1,
                    last_modified: None,
                },
                WorkspaceChatRow {
                    session_id: "ledger-only".into(),
                    agent: "claude".into(),
                    title: None,
                    last_seen_ms: 2,
                    last_modified: None,
                },
            ],
        );

        state.merge_workspace_chat_rows_in(&fake.root);

        let rows = state.workspace_chat_rows.get(&key).expect("rows");
        assert_eq!(rows.len(), 3, "three distinct chats: {rows:?}");
        assert_eq!(
            rows.iter().filter(|row| row.session_id == "shared").count(),
            1,
            "a chat known to both sources is one row"
        );
        assert_eq!(
            rows.iter()
                .find(|row| row.session_id == "shared")
                .and_then(|row| row.title.as_deref()),
            Some("in both"),
            "the store is authoritative for the title"
        );
        let _ = std::fs::remove_dir_all(&cwd);
    }

    /// A scratch directory standing in for `$HOME`, plus its ledger key.
    ///
    /// Never the real home: these tests write transcripts, and the one
    /// directory a user cannot afford to have a test write into is theirs.
    fn daily_probe_dir(state: &mut AppState, tag: &str) -> (std::path::PathBuf, String) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let cwd = std::env::temp_dir().join(format!(
            "herdr-daily-{}-{}-{}",
            std::process::id(),
            tag,
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&cwd).expect("daily probe dir");
        let key = crate::persist::workspace_chats::ledger_key(&cwd);
        state.daily_chat_cwd = Some(cwd.clone());
        (cwd, key)
    }

    // TP-DAILY-01: the whole point. Every other drawer is read because a
    // workspace asked for it; this directory has no workspace and never will,
    // so a read that follows the workspace list finds nothing. Measured
    // 2026-08-12: 1266 transcripts under $HOME, 0 workspaces holding it.
    #[test]
    fn daily_chats_are_read_even_when_no_workspace_lives_in_that_directory() {
        let fake = FakeProjectsRoot::new("daily-nows");
        let mut state = AppState::test_new();
        let (cwd, key) = daily_probe_dir(&mut state, "nows");
        for (id, title) in [("d1", "quick question"), ("d2", "scratch thought")] {
            fake.write_session(
                &cwd.to_string_lossy(),
                id,
                &[format!(r#"{{"type":"custom-title","customTitle":"{title}"}}"#).as_str()],
            );
        }
        assert!(
            state.workspaces.is_empty(),
            "the premise: nothing claims this directory"
        );

        state.merge_workspace_chat_rows_in(&fake.root);

        let rows = state.workspace_chat_rows.get(&key).expect("daily rows");
        assert_eq!(rows.len(), 2, "both spontaneous chats are listed: {rows:?}");
        let _ = std::fs::remove_dir_all(&cwd);
    }

    // TP-DAILY-01: a client with no home, or one whose home holds no chats,
    // must not be a client with a broken sidebar.
    #[test]
    fn a_daily_directory_that_is_not_there_costs_nothing() {
        let fake = FakeProjectsRoot::new("daily-missing");
        let mut state = AppState::test_new();
        state.daily_chat_cwd = Some(std::env::temp_dir().join("herdr-daily-does-not-exist"));
        state.merge_workspace_chat_rows_in(&fake.root);
        let key = crate::persist::workspace_chats::ledger_key(
            state.daily_chat_cwd.as_deref().expect("set above"),
        );
        assert!(
            state
                .workspace_chat_rows
                .get(&key)
                .is_none_or(Vec::is_empty),
            "a directory with no transcripts contributes no rows"
        );

        // And with no home at all the merge is simply the merge it always was.
        let mut homeless = AppState::test_new();
        assert!(homeless.daily_chat_cwd.is_none(), "the default is None");
        homeless.merge_workspace_chat_rows_in(&fake.root);
        assert!(
            homeless.workspace_chat_rows.is_empty(),
            "no home, no daily section, no change to anything else"
        );
    }

    // TP-DAILY-01: the read budget is the drawer's, not a second one. The
    // glance surface reads twelve; a section opened all the way reads sixty,
    // or "show older" would promise chats the parse never fetched. Two
    // different answers to "how many" would be two different mental models of
    // one drawer. The rows those chats become belong to the emission layer
    // and carry their own id there; naming it here would register a gate that
    // has no test behind it yet.
    #[test]
    fn an_open_daily_drawer_reads_past_the_glance_limit() {
        let fake = FakeProjectsRoot::new("daily-deep");
        let mut state = AppState::test_new();
        let (cwd, key) = daily_probe_dir(&mut state, "deep");
        for index in 0..14 {
            fake.write_session(
                &cwd.to_string_lossy(),
                &format!("d{index:02}"),
                &[format!(r#"{{"type":"custom-title","customTitle":"chat {index}"}}"#).as_str()],
            );
        }

        state.merge_workspace_chat_rows_in(&fake.root);
        assert_eq!(
            state.workspace_chat_rows.get(&key).map(Vec::len),
            Some(12),
            "closed, the section reads the glance budget"
        );

        let mut open = AppState::test_new();
        open.daily_chat_cwd = Some(cwd.clone());
        open.fully_open_chat_drawers.insert(key.clone());
        open.merge_workspace_chat_rows_in(&fake.root);
        assert_eq!(
            open.workspace_chat_rows.get(&key).map(Vec::len),
            Some(14),
            "opened all the way, it reads everything it could show"
        );
        let _ = std::fs::remove_dir_all(&cwd);
    }

    // TP-DAILY-01 + TP-DRAW-03: one row per chat here too. A chat the ledger
    // witnessed and the store also holds is one conversation, and two rows
    // for it raise the question "which one is live" that has no answer.
    #[test]
    fn a_daily_chat_the_ledger_also_saw_stays_one_row() {
        let fake = FakeProjectsRoot::new("daily-union");
        let mut state = AppState::test_new();
        let (cwd, key) = daily_probe_dir(&mut state, "union");
        fake.write_session(
            &cwd.to_string_lossy(),
            "both",
            &[r#"{"type":"custom-title","customTitle":"seen twice"}"#],
        );
        state.workspace_chat_rows.insert(
            key.clone(),
            vec![WorkspaceChatRow {
                session_id: "both".into(),
                agent: "claude".into(),
                title: None,
                last_seen_ms: 1,
                last_modified: None,
            }],
        );

        state.merge_workspace_chat_rows_in(&fake.root);

        let rows = state.workspace_chat_rows.get(&key).expect("daily rows");
        assert_eq!(rows.len(), 1, "one conversation, one row: {rows:?}");
        assert_eq!(
            rows[0].title.as_deref(),
            Some("seen twice"),
            "the store is authoritative for the title, exactly as elsewhere"
        );
        let _ = std::fs::remove_dir_all(&cwd);
    }

    // TP-DRAW-04: newest first, the order the Projects tab already uses. A
    // drawer in arbitrary order cannot answer "which chat was I just in".
    #[test]
    fn drawer_rows_are_ordered_newest_first() {
        let fake = FakeProjectsRoot::new("order");
        let mut state = AppState::test_new();
        let (cwd, key) = drawer_probe_workspace(&mut state, "order");
        state.workspace_chat_rows.insert(
            key.clone(),
            vec![
                WorkspaceChatRow {
                    session_id: "old".into(),
                    agent: "claude".into(),
                    title: Some("older".into()),
                    last_seen_ms: 10,
                    last_modified: None,
                },
                WorkspaceChatRow {
                    session_id: "new".into(),
                    agent: "claude".into(),
                    title: Some("newer".into()),
                    last_seen_ms: 9_000,
                    last_modified: None,
                },
                WorkspaceChatRow {
                    session_id: "mid".into(),
                    agent: "claude".into(),
                    title: Some("middle".into()),
                    last_seen_ms: 500,
                    last_modified: None,
                },
            ],
        );

        state.merge_workspace_chat_rows_in(&fake.root);

        let order: Vec<&str> = state.workspace_chat_rows[&key]
            .iter()
            .map(|row| row.session_id.as_str())
            .collect();
        assert_eq!(order, vec!["new", "mid", "old"]);
        let _ = std::fs::remove_dir_all(&cwd);
    }

    // TP-DRAW-05: every row can be dated. A row whose transcript was never
    // located still knows when the ledger last saw it, and that answers the
    // same question — a drawer where only some rows carry an age reads as
    // broken rather than partial.
    #[test]
    fn a_row_without_a_located_transcript_still_reports_its_last_activity() {
        let row = WorkspaceChatRow {
            session_id: "unplaced".into(),
            agent: "claude".into(),
            title: None,
            last_seen_ms: 1_700_000_000_000,
            last_modified: None,
        };
        assert_eq!(row.last_activity_ms(), 1_700_000_000_000);

        let placed = WorkspaceChatRow {
            last_modified: Some(
                std::time::UNIX_EPOCH + std::time::Duration::from_millis(1_800_000_000_000),
            ),
            ..row
        };
        assert_eq!(
            placed.last_activity_ms(),
            1_800_000_000_000,
            "the transcript's own mtime wins when it is known"
        );
    }

    // TP-DRAW-06: a chat filed under another workspace's directory is looked
    // for there before the row gives up and shows a bare id. This is the
    // `9433af4a · claude` case the user reported.
    #[test]
    fn a_chat_filed_under_another_workspace_is_still_titled() {
        let fake = FakeProjectsRoot::new("cross-slug");
        let mut state = AppState::test_new();
        let (home_cwd, _) = drawer_probe_workspace(&mut state, "elsewhere");
        let (branch_cwd, branch_key) = drawer_probe_workspace(&mut state, "branch");
        // The transcript lives under the OTHER workspace's directory.
        fake.write_session(
            &home_cwd.to_string_lossy(),
            "moved",
            &[r#"{"type":"custom-title","customTitle":"started elsewhere"}"#],
        );
        state.workspace_chat_rows.insert(
            branch_key.clone(),
            vec![WorkspaceChatRow {
                session_id: "moved".into(),
                agent: "claude".into(),
                title: None,
                last_seen_ms: 5,
                last_modified: None,
            }],
        );

        state.merge_workspace_chat_rows_in(&fake.root);

        let row = &state.workspace_chat_rows[&branch_key][0];
        assert_eq!(
            row.title.as_deref(),
            Some("started elsewhere"),
            "the title is found in the directory the chat was filed under"
        );
        assert!(row.last_modified.is_some(), "and so is its age");
        let _ = std::fs::remove_dir_all(&home_cwd);
        let _ = std::fs::remove_dir_all(&branch_cwd);
    }

    // TP-WSCHAT-21: the drawer shows the chat's NAME, the way the Projects tab
    // does. A session id is not an answer to "which chat did I work with" — the
    // ledger supplies the association and the agent's own store supplies the
    // title, and the two are joined here.
    #[test]
    fn drawer_rows_take_their_title_from_the_agents_own_store() {
        let fake = FakeProjectsRoot::new("drawer-titles");
        let cwd = std::env::temp_dir().join("herdr-drawer-title-probe");
        std::fs::create_dir_all(&cwd).expect("probe workspace dir");
        fake.write_session(
            &cwd.to_string_lossy(),
            "titled-session",
            &[r#"{"type":"custom-title","customTitle":"branch review"}"#],
        );

        let mut state = AppState::test_new();
        let mut workspace = crate::workspace::Workspace::test_new("titled");
        workspace.identity_cwd = cwd.clone();
        state.workspaces = vec![workspace];
        let key = crate::persist::workspace_chats::ledger_key(&cwd);
        state.workspace_chat_rows.insert(
            key.clone(),
            vec![
                WorkspaceChatRow {
                    session_id: "titled-session".into(),
                    agent: "claude".into(),
                    title: None,
                    last_seen_ms: 1,
                    last_modified: None,
                },
                WorkspaceChatRow {
                    session_id: "elsewhere-session".into(),
                    agent: "claude".into(),
                    title: None,
                    last_seen_ms: 2,
                    last_modified: None,
                },
            ],
        );

        state.merge_workspace_chat_rows_in(&fake.root);

        let rows = &state.workspace_chat_rows[&key];
        let titled = rows
            .iter()
            .find(|row| row.session_id == "titled-session")
            .expect("the ledger row survives the merge");
        assert_eq!(titled.title.as_deref(), Some("branch review"));
        assert!(
            titled.last_modified.is_some(),
            "a resolved row also gets its age"
        );
        // The fallback still exists — a chat filed under a directory herdr has
        // no workspace for cannot be titled — but it is now the last resort
        // rather than the first answer for every chat that moved (TP-DRAW-06).
        let elsewhere = rows
            .iter()
            .find(|row| row.session_id == "elsewhere-session")
            .expect("a chat the store cannot place is still listed");
        assert_eq!(elsewhere.title, None);
        assert!(elsewhere.display_label().starts_with("elsewhe"));

        let _ = std::fs::remove_dir_all(&cwd);
    }

    // P3: refresh reads the reader for each pinned path, aligned and newest-first.
    #[test]
    fn refresh_project_sessions_in_populates_cache() {
        let fake = FakeProjectsRoot::new("populate");
        fake.write_session(
            "/home/x/proj",
            "sess-1",
            &[r#"{"type":"custom-title","customTitle":"hello"}"#],
        );

        let mut state = AppState::test_new();
        state.projects_pinned = vec![std::path::PathBuf::from("/home/x/proj")];
        state.refresh_project_sessions_in(&fake.root);

        assert_eq!(state.projects_sessions.len(), 1);
        let project = &state.projects_sessions[0];
        assert_eq!(project.path, std::path::PathBuf::from("/home/x/proj"));
        assert_eq!(project.sessions.len(), 1);
        assert_eq!(project.sessions[0].title, "hello");
    }

    // P4: refresh with no pinned projects yields an empty cache and never panics.
    #[test]
    fn refresh_project_sessions_in_empty_pinned() {
        let fake = FakeProjectsRoot::new("empty-pinned");
        let mut state = AppState::test_new();
        state.refresh_project_sessions_in(&fake.root);
        assert!(state.projects_sessions.is_empty());
    }

    // P5: a pinned project with no chats keeps an entry with an empty session list.
    #[test]
    fn refresh_project_sessions_in_pinned_without_chats() {
        let fake = FakeProjectsRoot::new("no-chats");
        let mut state = AppState::test_new();
        state.projects_pinned = vec![
            std::path::PathBuf::from("/home/x/never-opened"),
            std::path::PathBuf::from("/home/x/also-empty"),
        ];
        state.refresh_project_sessions_in(&fake.root);

        assert_eq!(state.projects_sessions.len(), 2);
        assert_eq!(
            state.projects_sessions[0].path,
            std::path::PathBuf::from("/home/x/never-opened")
        );
        assert!(state
            .projects_sessions
            .iter()
            .all(|p| p.sessions.is_empty()));
    }

    // T5b: the wired-tab lookup finds the tab across workspaces and releases
    // the wiring when the tab closes (so the chat can be resumed again).
    #[test]
    fn find_resumed_chat_tab_locates_and_releases_wiring() {
        let mut state = AppState::test_new();
        let ws_a = crate::workspace::Workspace::test_new("a");
        let mut ws_b = crate::workspace::Workspace::test_new("b");
        let tab_idx = ws_b.test_add_tab(Some("chat"));
        ws_b.tabs[tab_idx].resumed_session_id = Some("sess-1".to_string());
        state.workspaces = vec![ws_a, ws_b];

        assert_eq!(state.find_resumed_chat_tab("sess-1"), Some((1, tab_idx)));
        assert_eq!(state.find_resumed_chat_tab("sess-404"), None);

        state.workspaces[1].close_tab(tab_idx);
        assert_eq!(
            state.find_resumed_chat_tab("sess-1"),
            None,
            "closing the tab must release the wiring"
        );
    }

    #[test]
    fn key_matches_requires_exact_modifiers() {
        assert!(key_matches(
            &KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
        ));

        assert!(!key_matches(
            &KeyEvent::new(
                KeyCode::Char('b'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            ),
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
        ));
    }

    #[test]
    fn key_matches_letters_case_insensitively() {
        assert!(key_matches(
            &KeyEvent::new(KeyCode::Char('B'), KeyModifiers::SHIFT),
            KeyCode::Char('b'),
            KeyModifiers::SHIFT,
        ));
    }

    // TP-RANK-06: the branch row's menu offers promotion, and the demote
    // only when a rule already claims the checkout.
    #[test]
    fn linked_worktree_menu_offers_promotion_and_conditional_demote() {
        let plain = ContextMenuState {
            kind: ContextMenuKind::GitWorkspace {
                ws_idx: 0,
                is_linked_worktree: true,
                has_worktree_children: false,
                collapsed: false,
                space_is_custom: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::default(),
        };
        assert!(plain.items().contains(&"Promote to module"));
        assert!(plain.items().contains(&"Promote to project"));
        assert!(!plain.items().contains(&"Demote from module"));

        let claimed = ContextMenuState {
            kind: ContextMenuKind::GitWorkspace {
                ws_idx: 0,
                is_linked_worktree: true,
                has_worktree_children: false,
                collapsed: false,
                space_is_custom: true,
            },
            x: 0,
            y: 0,
            list: MenuListState::default(),
        };
        assert!(claimed.items().contains(&"Demote from module"));
    }

    // TP-RANK-07: the plan the menu writes matches what the CLI would write.
    #[test]
    fn promote_plan_from_a_workspace_row_matches_the_cli_shape() {
        let mut state = AppState::test_new();
        let mut ws = crate::workspace::Workspace::test_new("tiling");
        ws.cached_git_branch = Some("worktree/Tiling".into());
        ws.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr-tiling"),
            is_linked_worktree: true,
        });
        state.workspaces = vec![ws];

        let plan = state
            .promote_plan_for_workspace(0, false)
            .expect("a branch checkout is promotable");
        assert_eq!(plan.key, "herdr:worktree-tiling");
        assert_eq!(plan.branch, "worktree/Tiling");
        assert_eq!(plan.repo_root, std::path::PathBuf::from("/repo/herdr"));
        assert!(plan.project.is_none());

        let project_plan = state
            .promote_plan_for_workspace(0, true)
            .expect("project rank is promotable too");
        assert_eq!(
            project_plan.project.as_ref().map(|p| p.key.as_str()),
            Some("project:worktree-tiling")
        );

        assert!(
            state.promote_plan_for_workspace(9, false).is_none(),
            "a missing row promotes nothing"
        );
    }

    // TP-RANK-12: the move plan is the promote plan carrying a parent (and
    // possibly the group it creates) — never a project rank.
    #[test]
    fn move_plan_carries_the_parent_and_the_new_group() {
        let mut state = AppState::test_new();
        let mut ws = crate::workspace::Workspace::test_new("tiling");
        ws.cached_git_branch = Some("worktree/Tiling".into());
        ws.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: std::path::PathBuf::from("/repo/herdr"),
            checkout_path: std::path::PathBuf::from("/repo/herdr-tiling"),
            is_linked_worktree: true,
        });
        state.workspaces = vec![ws];

        let node = crate::cli::space::NodePlan {
            key: "group:ops".into(),
            name: "Ops".into(),
            parent: None,
        };
        let plan = state
            .move_plan_for_workspace(0, Some("group:ops".into()), Some(node.clone()))
            .expect("a branch checkout is movable");
        assert_eq!(plan.parent.as_deref(), Some("group:ops"));
        assert_eq!(plan.node.as_ref(), Some(&node));
        assert_eq!(plan.key, "herdr:worktree-tiling");
        assert_eq!(plan.branch, "worktree/Tiling");
        assert!(plan.project.is_none(), "a move never changes rank");

        assert!(
            state.move_plan_for_workspace(9, None, None).is_none(),
            "a missing row moves nothing"
        );
    }

    // TP-RANK-13: the move submenu offers the three verbs only when a node
    // exists to point them at; the group and top-level roads always show.
    #[test]
    fn the_move_menu_hides_the_verbs_without_targets() {
        let with_targets = ContextMenuState {
            kind: ContextMenuKind::MoveWorkspace {
                ws_idx: 0,
                has_targets: true,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(
            with_targets.items(),
            &[
                "Under a group...",
                "Beside a group...",
                "Above a group...",
                "Under a new group...",
                "To top level",
            ]
        );

        let without = ContextMenuState {
            kind: ContextMenuKind::MoveWorkspace {
                ws_idx: 0,
                has_targets: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(
            without.items(),
            &["Under a new group...", "To top level"],
            "verbs with nothing to point at do not render"
        );
    }

    // TP-CHAT-MOVE-04: a chat row's menu always offers the move road and
    // only offers the way back while a re-home is actually in force.
    #[test]
    fn the_chat_menu_offers_move_and_conditionally_back() {
        let plain = ContextMenuState {
            kind: ContextMenuKind::WorkspaceChat {
                ws_idx: Some(0),
                session_id: "s1".into(),
                has_move: false,
                has_live: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(plain.items(), &["Move to branch..."]);

        let moved = ContextMenuState {
            kind: ContextMenuKind::WorkspaceChat {
                ws_idx: Some(0),
                session_id: "s1".into(),
                has_move: true,
                has_live: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(moved.items(), &["Move to branch...", "Move back"]);

        let picker = ContextMenuState {
            kind: ContextMenuKind::ChatMoveTarget {
                session_id: "s1".into(),
                targets: vec![("/repo/b".into(), "feature-b".into())],
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(picker.items(), &["feature-b"]);
    }

    // TP-RANK-13: the target picker shows display names and resolves by
    // index, so duplicate names can never mis-route a move.
    #[test]
    fn the_target_picker_lists_the_forest_by_name() {
        let picker = ContextMenuState {
            kind: ContextMenuKind::MoveTarget {
                ws_idx: 0,
                op: crate::spaces::MoveOp::Under,
                targets: vec![
                    ("group:ui".into(), "UI".into()),
                    ("group:ops".into(), "Ops".into()),
                ],
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(picker.items(), &["UI", "Ops"]);
    }

    #[test]
    fn linked_worktree_context_menu_keeps_safe_close_and_explicit_remove() {
        let menu = ContextMenuState {
            kind: ContextMenuKind::GitWorkspace {
                ws_idx: 0,
                is_linked_worktree: true,
                has_worktree_children: false,
                collapsed: false,
                space_is_custom: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };

        // TP-RANK-06 added the promotion pair and TP-RANK-13 the move road;
        // the subject here — Close stays safe and checkout removal stays a
        // separate explicit action — holds.
        assert_eq!(
            menu.items(),
            &[
                "Rename",
                "Close",
                "Promote to module",
                "Promote to project",
                "Move...",
                "Delete worktree checkout..."
            ]
        );
    }

    #[test]
    fn git_workspace_context_menu_keeps_remove_for_managed_worktrees_only() {
        let menu = ContextMenuState {
            kind: ContextMenuKind::GitWorkspace {
                ws_idx: 0,
                is_linked_worktree: false,
                has_worktree_children: false,
                collapsed: false,
                space_is_custom: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };

        assert_eq!(
            menu.items(),
            &["Rename", "Close", "New worktree", "Open worktree..."]
        );
    }

    #[test]
    fn parent_worktree_context_menu_uses_repo_actions() {
        let menu = ContextMenuState {
            kind: ContextMenuKind::GitWorkspace {
                ws_idx: 0,
                is_linked_worktree: false,
                has_worktree_children: true,
                collapsed: false,
                space_is_custom: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };

        assert_eq!(
            menu.items(),
            &[
                "Rename",
                "Close group",
                "New worktree",
                "Open worktree...",
                "Collapse"
            ]
        );
    }

    fn file_action_bar_model(
        kind: FileManagerActionBarSelectionKind,
        paths: Vec<PathBuf>,
        copy_reason: Option<FileManagerActionDisabledReason>,
        delete_reason: Option<FileManagerActionDisabledReason>,
    ) -> FileManagerActionBarModel {
        let selection = FileManagerActionBarSelection {
            label: paths
                .first()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| String::from("selection")),
            paths,
            kind,
        };
        let actions = FileManagerHeaderAction::ALL.map(|action| {
            let disabled_reason = match action {
                FileManagerHeaderAction::Copy => copy_reason,
                FileManagerHeaderAction::Delete => delete_reason,
                FileManagerHeaderAction::Paste | FileManagerHeaderAction::NewFolder => None,
            };
            FileManagerActionState {
                action,
                enabled: disabled_reason.is_none(),
                disabled_reason,
            }
        });
        FileManagerActionBarModel {
            selection: Some(selection),
            clipboard_count: 0,
            actions,
        }
    }

    fn file_context_item(
        model: &FileManagerContextMenuModel,
        action: FileManagerContextMenuAction,
    ) -> &FileManagerContextMenuItem {
        model
            .items
            .iter()
            .find(|item| item.action == action)
            .expect("context action item")
    }

    // TP-C3.1-CONTEXT-MODEL: cursor focus or an empty prepared selection does
    // not invent a file context menu; one file/directory carries exact paths.
    #[test]
    fn file_context_menu_requires_explicit_prepared_selection() {
        let empty = FileManagerActionBarModel {
            selection: None,
            clipboard_count: 0,
            actions: FileManagerHeaderAction::ALL.map(|action| FileManagerActionState {
                action,
                enabled: false,
                disabled_reason: Some(FileManagerActionDisabledReason::NoSelection),
            }),
        };
        assert!(FileManagerContextMenuModel::from_action_bar(&empty).is_none());

        for (kind, expected_kind, name) in [
            (
                FileManagerActionBarSelectionKind::File,
                FileManagerContextMenuTargetKind::File,
                "file.txt",
            ),
            (
                FileManagerActionBarSelectionKind::Directory,
                FileManagerContextMenuTargetKind::Directory,
                "directory",
            ),
        ] {
            let path = PathBuf::from("/prepared").join(name);
            let action_bar = file_action_bar_model(kind, vec![path.clone()], None, None);
            let model = FileManagerContextMenuModel::from_action_bar(&action_bar)
                .expect("explicit context model");
            assert_eq!(model.target_kind, expected_kind);
            assert_eq!(model.paths, vec![path]);
            assert!(model.items.iter().all(|item| {
                match item.action {
                    FileManagerContextMenuAction::Compress => {
                        !item.enabled
                            && item.disabled_reason
                                == Some(FileManagerActionDisabledReason::UnsupportedAction)
                    }
                    // Neither fixture name promises a picture, so Enlarge is
                    // correctly unavailable for both.
                    FileManagerContextMenuAction::Enlarge => {
                        !item.enabled
                            && item.disabled_reason
                                == Some(FileManagerActionDisabledReason::UnsupportedSelection)
                    }
                    // Taildrop takes files, so the directory half of this
                    // fixture must find the entry present and disabled while
                    // the file half finds it usable.
                    FileManagerContextMenuAction::SendTailscale => {
                        if matches!(expected_kind, FileManagerContextMenuTargetKind::Directory) {
                            !item.enabled
                                && item.disabled_reason
                                    == Some(FileManagerActionDisabledReason::UnsupportedSelection)
                        } else {
                            item.enabled
                        }
                    }
                    _ => item.enabled,
                }
            }));
        }
    }

    // TP-C3.1-CONTEXT-MODEL: the six core actions retain deterministic order;
    // read-only state disables only cwd-writing actions.
    #[test]
    fn single_file_context_menu_has_stable_order_and_read_only_authority() {
        let action_bar = file_action_bar_model(
            FileManagerActionBarSelectionKind::File,
            vec![PathBuf::from("/prepared/file.txt")],
            None,
            Some(FileManagerActionDisabledReason::ReadOnlyTarget),
        );
        let model = FileManagerContextMenuModel::from_action_bar(&action_bar)
            .expect("single-file context model");
        assert_eq!(
            model
                .items
                .iter()
                .map(|item| item.action.clone())
                .collect::<Vec<_>>(),
            vec![
                FileManagerContextMenuAction::Open,
                FileManagerContextMenuAction::Enlarge,
                FileManagerContextMenuAction::Copy,
                FileManagerContextMenuAction::Rename,
                FileManagerContextMenuAction::Delete,
                FileManagerContextMenuAction::Compress,
                FileManagerContextMenuAction::SendAgent,
                FileManagerContextMenuAction::SendTailscale,
            ]
        );
        assert_eq!(
            model
                .items
                .iter()
                .map(|item| item.label.clone())
                .collect::<Vec<_>>(),
            vec![
                "Open",
                "Enlarge",
                "Copy",
                "Rename",
                "Delete",
                "Compress",
                "Add Reference to Agent...",
                "Send with Tailscale...",
            ]
        );
        for action in [
            FileManagerContextMenuAction::Open,
            FileManagerContextMenuAction::Copy,
            FileManagerContextMenuAction::SendAgent,
            // A readable file can always be sent; the read-only target in this
            // fixture blocks only the actions that write into the directory.
            FileManagerContextMenuAction::SendTailscale,
        ] {
            assert!(file_context_item(&model, action).enabled);
        }
        for action in [
            FileManagerContextMenuAction::Rename,
            FileManagerContextMenuAction::Delete,
        ] {
            let item = file_context_item(&model, action);
            assert!(!item.enabled);
            assert_eq!(
                item.disabled_reason,
                Some(FileManagerActionDisabledReason::ReadOnlyTarget)
            );
        }
        let compress = file_context_item(&model, FileManagerContextMenuAction::Compress);
        assert!(!compress.enabled);
        assert_eq!(
            compress.disabled_reason,
            Some(FileManagerActionDisabledReason::UnsupportedAction)
        );
    }

    // TP-C3.1-CONTEXT-MODEL: multiple selection permits only bulk-capable
    // actions while preserving prepared path order.
    #[test]
    fn multiple_file_context_menu_disables_single_target_actions() {
        let paths = vec![
            PathBuf::from("/prepared/file2.txt"),
            PathBuf::from("/prepared/file10.txt"),
        ];
        let action_bar = file_action_bar_model(
            FileManagerActionBarSelectionKind::Multiple,
            paths.clone(),
            None,
            None,
        );
        let model = FileManagerContextMenuModel::from_action_bar(&action_bar)
            .expect("multiple context model");
        assert_eq!(
            model.target_kind,
            FileManagerContextMenuTargetKind::Multiple
        );
        assert_eq!(model.paths, paths);
        for action in [
            FileManagerContextMenuAction::Copy,
            FileManagerContextMenuAction::Delete,
        ] {
            assert!(file_context_item(&model, action).enabled);
        }
        let compress = file_context_item(&model, FileManagerContextMenuAction::Compress);
        assert!(!compress.enabled);
        assert_eq!(
            compress.disabled_reason,
            Some(FileManagerActionDisabledReason::UnsupportedAction)
        );
        for action in [
            FileManagerContextMenuAction::Open,
            FileManagerContextMenuAction::Rename,
            FileManagerContextMenuAction::SendAgent,
        ] {
            let item = file_context_item(&model, action);
            assert!(!item.enabled);
            assert_eq!(
                item.disabled_reason,
                Some(FileManagerActionDisabledReason::MultipleSelection)
            );
        }
    }

    // TP-C3.1-CONTEXT-MODEL: unsupported, stale, and in-flight selection
    // authority disables every item, with in-flight already carrying priority.
    #[test]
    fn invalid_or_in_flight_file_context_menu_fails_closed() {
        for reason in [
            FileManagerActionDisabledReason::UnsupportedSelection,
            FileManagerActionDisabledReason::StaleSelection,
            FileManagerActionDisabledReason::OperationInFlight,
        ] {
            let action_bar = file_action_bar_model(
                FileManagerActionBarSelectionKind::Unavailable,
                vec![PathBuf::from("/prepared/unavailable")],
                Some(reason),
                Some(reason),
            );
            let model = FileManagerContextMenuModel::from_action_bar(&action_bar)
                .expect("fail-closed context model");
            assert_eq!(
                model.target_kind,
                FileManagerContextMenuTargetKind::Unavailable
            );
            assert!(model
                .items
                .iter()
                .all(|item| { !item.enabled && item.disabled_reason == Some(reason) }));
        }

        let mixed = file_action_bar_model(
            FileManagerActionBarSelectionKind::Unavailable,
            vec![PathBuf::from("/prepared/in-flight")],
            Some(FileManagerActionDisabledReason::UnsupportedSelection),
            Some(FileManagerActionDisabledReason::OperationInFlight),
        );
        let model = FileManagerContextMenuModel::from_action_bar(&mixed)
            .expect("mixed-priority context model");
        assert!(model.items.iter().all(|item| {
            !item.enabled
                && item.disabled_reason == Some(FileManagerActionDisabledReason::OperationInFlight)
        }));
    }

    // TP-C3.1-CONTEXT-MODEL: the global popup kind exposes the exact file
    // labels without changing the established menu state shape.
    #[test]
    fn file_context_kind_exposes_deterministic_labels() {
        let action_bar = file_action_bar_model(
            FileManagerActionBarSelectionKind::File,
            vec![PathBuf::from("/prepared/file.txt")],
            None,
            None,
        );
        let model =
            FileManagerContextMenuModel::from_action_bar(&action_bar).expect("file context model");
        let menu = ContextMenuState {
            kind: ContextMenuKind::File { model },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };
        assert_eq!(
            menu.items(),
            vec![
                "Open",
                "Enlarge",
                "Copy",
                "Rename",
                "Delete",
                "Compress",
                "Add Reference to Agent...",
                "Send with Tailscale...",
            ]
        );
    }

    fn plugin_file_action(
        plugin_id: &str,
        action_id: &str,
        title: &str,
        contexts: Vec<crate::api::schema::PluginActionContext>,
    ) -> crate::api::schema::PluginActionInfo {
        crate::api::schema::PluginActionInfo {
            plugin_id: plugin_id.into(),
            action_id: action_id.into(),
            title: title.into(),
            description: None,
            contexts,
            file_extensions: Vec::new(),
            command: vec!["inspect".into()],
            platforms: None,
        }
    }

    fn plugin_file_action_for(
        plugin_id: &str,
        title: &str,
        file_extensions: &[&str],
    ) -> crate::api::schema::PluginActionInfo {
        crate::api::schema::PluginActionInfo {
            file_extensions: file_extensions
                .iter()
                .map(|extension| (*extension).to_owned())
                .collect(),
            ..plugin_file_action(
                plugin_id,
                "open",
                title,
                vec![crate::api::schema::PluginActionContext::File],
            )
        }
    }

    fn menu_labels(
        selection: Vec<PathBuf>,
        candidates: &[crate::api::schema::PluginActionInfo],
    ) -> Vec<String> {
        let action_bar = file_action_bar_model(
            if selection.len() == 1 {
                FileManagerActionBarSelectionKind::File
            } else {
                FileManagerActionBarSelectionKind::Multiple
            },
            selection,
            None,
            None,
        );
        FileManagerContextMenuModel::from_action_bar_with_plugins(&action_bar, candidates)
            .expect("file context model")
            .items
            .into_iter()
            .map(|item| item.label)
            .collect()
    }

    fn menu_item(
        selection: Vec<PathBuf>,
        action: FileManagerContextMenuAction,
    ) -> FileManagerContextMenuItem {
        let action_bar = file_action_bar_model(
            if selection.len() == 1 {
                FileManagerActionBarSelectionKind::File
            } else {
                FileManagerActionBarSelectionKind::Multiple
            },
            selection,
            None,
            None,
        );
        FileManagerContextMenuModel::from_action_bar(&action_bar)
            .expect("file context model")
            .items
            .into_iter()
            .find(|item| item.action == action)
            .expect("the menu must offer this action")
    }

    // TP-FOPEN-13: a file with a picture offers Enlarge. Without it a PDF or
    // an image has no working entry in the menu at all: the built-in Open only
    // descends into directories, and a plugin that does not handle the type is
    // now correctly absent.
    #[test]
    fn a_picture_offers_enlarge() {
        for name in ["manual.pdf", "photo.png", "scan.TIFF"] {
            let item = menu_item(
                vec![PathBuf::from(format!("/prepared/{name}"))],
                FileManagerContextMenuAction::Enlarge,
            );
            assert!(item.enabled, "{name} should offer Enlarge: {item:?}");
        }
    }

    // TP-FOPEN-14: a file drawn from cells has nothing to enlarge, so the
    // entry is disabled with a reason rather than enabled and inert. An entry
    // that looks available and does nothing reads as a frozen application.
    #[test]
    fn a_file_without_a_picture_disables_enlarge_with_a_reason() {
        for name in ["notes.txt", "report.xlsx", "Makefile"] {
            let item = menu_item(
                vec![PathBuf::from(format!("/prepared/{name}"))],
                FileManagerContextMenuAction::Enlarge,
            );
            assert!(!item.enabled, "{name} should not offer Enlarge: {item:?}");
            assert_eq!(
                item.disabled_reason,
                Some(FileManagerActionDisabledReason::UnsupportedSelection),
                "{name} should say why"
            );
        }
    }

    // TP-FOPEN-15: the viewer shows one file, so a multiple selection has no
    // unambiguous answer and says so.
    #[test]
    fn a_multiple_selection_disables_enlarge() {
        let item = menu_item(
            vec![
                PathBuf::from("/prepared/a.png"),
                PathBuf::from("/prepared/b.png"),
            ],
            FileManagerContextMenuAction::Enlarge,
        );
        assert!(!item.enabled);
        assert_eq!(
            item.disabled_reason,
            Some(FileManagerActionDisabledReason::MultipleSelection)
        );
    }

    // TP-FOPEN-08: an action that handles only spreadsheets is not offered on
    // a PDF. This is the reported defect: the spreadsheet plugin was offered
    // on every file, and choosing it launched the spreadsheet editor on a
    // document it cannot read, leaving an empty tab with nothing to explain
    // it. The action is absent rather than disabled — a greyed-out entry would
    // suggest a temporary condition, when the action simply does not apply.
    #[test]
    fn a_plugin_action_is_offered_only_for_the_extensions_it_handles() {
        let sheets = plugin_file_action_for("cypack.sheets", "Open in New Tab", &["xlsx", "csv"]);

        let on_pdf = menu_labels(
            vec![PathBuf::from("/prepared/manual.pdf")],
            std::slice::from_ref(&sheets),
        );
        assert!(
            !on_pdf.iter().any(|label| label == "Open in New Tab"),
            "a spreadsheet action must not be offered on a pdf: {on_pdf:?}"
        );

        let on_workbook = menu_labels(vec![PathBuf::from("/prepared/report.xlsx")], &[sheets]);
        assert!(
            on_workbook.iter().any(|label| label == "Open in New Tab"),
            "the same action must still be offered on a workbook: {on_workbook:?}"
        );
    }

    // TP-FOPEN-09: an action that names no extensions is still offered on
    // every file. Every manifest installed today omits the field, so treating
    // an empty list as "nothing" would silently disable every existing plugin.
    #[test]
    fn a_plugin_action_without_extensions_is_offered_on_every_file() {
        let generic = plugin_file_action_for("example.tool", "Inspect", &[]);
        for name in ["manual.pdf", "report.xlsx", "Makefile"] {
            let labels = menu_labels(
                vec![PathBuf::from(format!("/prepared/{name}"))],
                std::slice::from_ref(&generic),
            );
            assert!(
                labels.iter().any(|label| label == "Inspect"),
                "{name} should still offer an unrestricted action: {labels:?}"
            );
        }
    }

    // TP-FOPEN-10: a selection where only some files match does not offer the
    // action. Offering it there runs the wrong program on the files that did
    // not match — the reported defect, one selection wider.
    #[test]
    fn a_plugin_action_is_withheld_from_a_partly_matching_selection() {
        let sheets = plugin_file_action_for("cypack.sheets", "Open in New Tab", &["xlsx"]);
        let labels = menu_labels(
            vec![
                PathBuf::from("/prepared/a.xlsx"),
                PathBuf::from("/prepared/b.pdf"),
            ],
            &[sheets],
        );
        assert!(
            !labels.iter().any(|label| label == "Open in New Tab"),
            "a partly matching selection must not offer the action: {labels:?}"
        );
    }

    // TP-C3.3-PLUGIN-SURFACE: plugin actions append after built-ins in stable
    // qualified-id order, preserve one/many prepared paths, and produce only
    // a neutral public plugin invocation payload (no command side effect).
    #[test]
    fn file_context_menu_appends_plugins_and_serializes_exact_path_intent() {
        use crate::api::schema::PluginActionContext;

        let paths = vec![
            PathBuf::from("/prepared/file2.txt"),
            PathBuf::from("/prepared/file 10.txt"),
        ];
        let action_bar = file_action_bar_model(
            FileManagerActionBarSelectionKind::Multiple,
            paths.clone(),
            None,
            None,
        );
        let candidates = vec![
            plugin_file_action(
                "zeta.files",
                "inspect",
                "Inspect with Zeta",
                vec![PluginActionContext::File],
            ),
            plugin_file_action(
                "ignored.workspace",
                "inspect",
                "Wrong context",
                vec![PluginActionContext::Workspace],
            ),
            plugin_file_action(
                "alpha.files",
                "inspect",
                "Inspect with Alpha",
                vec![PluginActionContext::File],
            ),
        ];
        let model =
            FileManagerContextMenuModel::from_action_bar_with_plugins(&action_bar, &candidates)
                .expect("file context model");

        assert_eq!(model.paths, paths);
        // Eight built-in entries, then the plugin ones. The index moved when
        // Send with Tailscale joined the built-ins; the shape being asserted —
        // plugins come last, in sorted order — is unchanged.
        assert_eq!(model.items.len(), 10);
        assert_eq!(
            model.items[8..]
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Inspect with Alpha", "Inspect with Zeta"]
        );
        let plugin_action = FileManagerContextMenuAction::Plugin {
            plugin_id: "alpha.files".into(),
            action_id: "inspect".into(),
        };
        assert_eq!(model.items[8].action, plugin_action);
        assert!(model.items[8].enabled);

        let intent = FileManagerContextActionIntent {
            action: model.items[8].action.clone(),
            paths: model.paths.clone(),
        };
        let params = intent
            .plugin_invocation_params()
            .expect("plugin invocation params");
        assert_eq!(params.plugin_id.as_deref(), Some("alpha.files"));
        assert_eq!(params.action_id, "inspect");
        let context = params.context.expect("file invocation context");
        assert_eq!(
            context.file_paths,
            vec!["/prepared/file2.txt", "/prepared/file 10.txt"]
        );
        assert_eq!(context.invocation_source.as_deref(), Some("file_manager"));
    }

    // TP-C3.3-PLUGIN-SURFACE: lossy path conversion is forbidden. A Unix path
    // that JSON cannot represent exactly keeps built-ins but exposes no plugin
    // action that could receive the wrong target.
    #[cfg(unix)]
    #[test]
    fn file_context_menu_hides_plugins_for_non_utf8_paths() {
        use crate::api::schema::PluginActionContext;
        use std::os::unix::ffi::OsStringExt;

        let path = PathBuf::from(std::ffi::OsString::from_vec(vec![b'/', b'x', 0xff]));
        let action_bar = file_action_bar_model(
            FileManagerActionBarSelectionKind::File,
            vec![path],
            None,
            None,
        );
        let candidates = vec![plugin_file_action(
            "example.files",
            "inspect",
            "Inspect",
            vec![PluginActionContext::File],
        )];
        let model =
            FileManagerContextMenuModel::from_action_bar_with_plugins(&action_bar, &candidates)
                .expect("built-in file context model");

        assert_eq!(model.items.len(), FileManagerContextMenuAction::ALL.len());
        assert!(FileManagerContextActionIntent {
            action: FileManagerContextMenuAction::Plugin {
                plugin_id: "example.files".into(),
                action_id: "inspect".into(),
            },
            paths: model.paths,
        }
        .plugin_invocation_params()
        .is_none());
    }
}

#[cfg(test)]
mod viewer_context_tests {
    use super::*;

    // TP-MCF-CTX-01
    #[test]
    fn entering_a_viewer_returns_the_previous_one_for_restoration() {
        let mut state = AppState::test_new();
        assert_eq!(state.viewer(), None, "a fresh state has no viewer");

        let previous = state.enter_viewer(Some(7));
        assert_eq!(previous, None);
        assert_eq!(state.viewer(), Some(7));

        let nested_previous = state.enter_viewer(Some(9));
        assert_eq!(nested_previous, Some(7));
        assert_eq!(state.viewer(), Some(9));

        state.restore_viewer(nested_previous);
        assert_eq!(state.viewer(), Some(7));

        state.restore_viewer(previous);
        assert_eq!(state.viewer(), None);
    }

    // TP-MCF-CTX-02
    #[test]
    fn the_viewer_context_reaches_every_workspace() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("first"));
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("second"));

        state.enter_viewer(Some(4));
        for workspace in &state.workspaces {
            assert_eq!(
                workspace.viewer(),
                Some(4),
                "workspace accessors resolve through the mirrored viewer"
            );
        }

        state.restore_viewer(None);
        for workspace in &state.workspaces {
            assert_eq!(workspace.viewer(), None);
        }
    }

    // TP-MCF-WS-01
    #[test]
    fn two_clients_stay_in_different_workspaces() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("left"));
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("right"));
        state.active = Some(0);

        // Both displays attach on the workspace the session is on.
        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        // The second display moves to the other workspace.
        state.active = Some(1);
        state.restore_viewer(None);

        state.enter_viewer(Some(1));
        assert_eq!(
            state.active,
            Some(0),
            "the display nobody moved must stay in its own workspace"
        );
        state.restore_viewer(None);

        state.enter_viewer(Some(2));
        assert_eq!(
            state.active,
            Some(1),
            "the display that moved keeps its move"
        );
        state.restore_viewer(None);
    }

    // TP-MCF-WS-02
    #[test]
    fn a_client_that_never_moved_adopts_the_workspace_the_session_is_driven_to() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("left"));
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("right"));
        state.active = Some(0);

        state.enter_viewer(Some(1));
        state.active = Some(1);
        state.restore_viewer(None);

        // A display attaching afterwards lands where the session is being
        // driven, not on workspace zero.
        state.enter_viewer(Some(9));
        assert_eq!(state.active, Some(1));
        state.restore_viewer(None);

        // And a departed display leaves nothing behind.
        state.forget_client(1);
        assert!(!state.surfaces_by_client.contains_key(&1));
    }

    // TP-SUR-ADOPT-02
    #[test]
    fn a_display_seen_for_the_first_time_is_born_without_person_opened_overlays() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("main"));
        state.active = Some(0);
        state.mode = Mode::Terminal;

        // Display 1 opens a popup, a preview viewer and a context menu, then
        // parks — promoting what it changed into the session default.
        state.enter_viewer(Some(1));
        state.popup_pane = Some(crate::app::state::PopupPaneState {
            pane_id: crate::layout::PaneId::alloc(),
            terminal_id: crate::terminal::TerminalId::alloc(),
            width: None,
            height: None,
        });
        state.preview_viewer = Some(PreviewViewerState {
            source_path: std::path::PathBuf::from("/tmp/adopt.png"),
        });
        state.overlay_return_mode = Some(Mode::Terminal);
        state.mode = Mode::PreviewViewer;
        state.restore_viewer(None);

        // A display attaching now has opened nothing: no popup, no viewer, no
        // menu, and an input mode that will not swallow its first keystroke.
        state.enter_viewer(Some(2));
        assert!(
            state.popup_pane.is_none(),
            "a freshly attached display must not be born holding another \
             display's popup"
        );
        assert!(
            state.preview_viewer.is_none(),
            "a freshly attached display must not be born inside another \
             display's preview viewer"
        );
        assert!(
            state.context_menu.is_none(),
            "a freshly attached display must not be born with a context menu \
             open"
        );
        assert!(
            !matches!(
                state.mode,
                Mode::PreviewViewer | Mode::ContextMenu | Mode::GlobalMenu | Mode::Copy
            ),
            "an overlay mode without its overlay eats every keystroke; \
             adopted mode was {:?}",
            state.mode
        );
        state.restore_viewer(None);

        // Display 1 keeps everything it opened: stripping the adoption must
        // not strip the opener.
        state.enter_viewer(Some(1));
        assert!(
            state.popup_pane.is_some(),
            "the display that opened the popup keeps it"
        );
        assert!(matches!(state.mode, Mode::PreviewViewer));
        state.restore_viewer(None);
    }

    // The broadcast rule must keep working while overlay-opening API requests
    // are scoped to one display (TP-SUR-BROADCAST-05): a session instruction
    // still reaches parked displays, or a display left in navigate mode
    // swallows what its user types after the API focuses a pane.
    //
    // TP-SUR-BROADCAST-01
    // TP-SUR-BROADCAST-06: the remaining broadcast members are presentational,
    // and this states it instead of leaving it as a comment.
    //
    // The popup-leak family came from a PERSON-opened surface sitting in this
    // class: set with no display behind it, it was copied to every screen. The
    // rail's tab, its selected row and its scroll offsets are different in kind
    // — nobody "opens" them, they describe where a list is looking — so
    // broadcasting them is intended, not an oversight. If a person-opened
    // surface is ever added to this class, the guard is the seam
    // (TP-SUR-BROADCAST-05), not this row.
    #[test]
    fn the_rails_presentational_broadcast_members_reach_parked_displays() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("main"));
        state.active = Some(0);

        // Two displays have been seen and parked.
        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        state.restore_viewer(None);

        // The session changes presentational rail state with nobody serving.
        state.sidebar_tab = SidebarTab::Projects;
        state.selected = 0;
        state.workspace_scroll = 7;

        state.enter_viewer(Some(2));
        assert_eq!(
            state.sidebar_tab,
            SidebarTab::Projects,
            "the rail's tab is presentational: a session-level change is an \
             instruction and reaches a parked display"
        );
        assert_eq!(state.workspace_scroll, 7);
        state.restore_viewer(None);

        // And the guard that matters: a person-opened surface in the same class
        // must NOT be born on a display that never asked for it. This is the
        // distinction the class comment relies on.
        state.popup_pane = None;
        state.enter_viewer(Some(3));
        assert!(
            state.popup_pane.is_none(),
            "a display seen for the first time is born without a popup — the \
             seam owns that, not the broadcast class"
        );
        state.restore_viewer(None);
    }

    #[test]
    fn a_session_instruction_still_reaches_parked_displays() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("main"));
        state.active = Some(0);
        state.mode = Mode::Navigate;

        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        state.restore_viewer(None);

        // The API focuses a pane: the session goes to terminal mode.
        state.mode = Mode::Terminal;

        state.enter_viewer(Some(2));
        assert_eq!(
            state.mode,
            Mode::Terminal,
            "a mode change with no display behind it is an instruction and must \
             reach a parked display"
        );
        state.restore_viewer(None);
    }

    // TP-SUR-DEFAULT-01
    #[test]
    fn a_displays_chat_drawer_folds_stay_its_own() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("left"));
        state.active = Some(0);

        // Display 2 opens a drawer and parks.
        state.enter_viewer(Some(2));
        state.expanded_chat_workspaces.insert("k".into());
        state.restore_viewer(None);

        // Display 3 may adopt the last broadcast state on attach, but what it
        // does to ITS view must never leak back into display 2's.
        state.enter_viewer(Some(3));
        state.expanded_chat_workspaces.remove("k");
        state.restore_viewer(None);

        state.enter_viewer(Some(2));
        assert!(
            state.expanded_chat_workspaces.contains("k"),
            "display 2 keeps its own drawer folds"
        );
        state.restore_viewer(None);
    }

    // The suppress set is the same sentence for the derived-open rows: one
    // display quieting a drawer must not quiet it anywhere else.
    #[test]
    fn a_displays_drawer_suppressions_stay_its_own() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("left"));
        state.active = Some(0);

        state.enter_viewer(Some(2));
        state.suppressed_chat_drawers.insert("k".into());
        state.restore_viewer(None);

        state.enter_viewer(Some(3));
        state.suppressed_chat_drawers.remove("k");
        state.restore_viewer(None);

        state.enter_viewer(Some(2));
        assert!(
            state.suppressed_chat_drawers.contains("k"),
            "display 2 keeps its own suppressions"
        );
        state.restore_viewer(None);
    }

    #[test]
    fn serving_a_display_that_did_not_move_leaves_the_session_default_alone() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("left"));
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("right"));
        state.active = Some(0);

        // A display attaches and settles on workspace zero.
        state.enter_viewer(Some(2));
        state.restore_viewer(None);

        // Something with no display behind it moves the session -- an API
        // call, a restore, a keybind handled outside any view.
        state.active = Some(1);

        // The render loop now serves that display several times. It is only
        // looking; it never moves.
        for _ in 0..4 {
            state.enter_viewer(Some(2));
            assert_eq!(
                state.active,
                Some(0),
                "the display keeps its own workspace while it is being served"
            );
            state.restore_viewer(None);

            assert_eq!(
                state.active,
                Some(1),
                "and looking at it must not drag the session onto its workspace"
            );
        }

        // The session default is what the notification path resolves through:
        // a pane on workspace zero has to keep reading as background, or its
        // agent finishes in silence.
        assert!(
            !state.pane_is_in_active_tab(0, crate::layout::PaneId::from_raw(1)),
            "a pane outside the session workspace must stay in the background"
        );
    }

    // TP-SUR-OVERLAY-01
    #[test]
    fn a_menu_opened_on_one_display_does_not_open_on_the_others() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("only"));
        state.active = Some(0);

        // Both displays are known and in the base mode.
        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        state.restore_viewer(None);

        // One of them right-clicks.
        state.enter_viewer(Some(1));
        state.mode = Mode::ContextMenu;
        state.context_menu = Some(ContextMenuState {
            kind: ContextMenuKind::Workspace { ws_idx: 0 },
            x: 4,
            y: 9,
            list: MenuListState::new(0),
        });
        state.restore_viewer(None);

        state.enter_viewer(Some(2));
        assert!(
            state.context_menu.is_none(),
            "the display nobody clicked must not grow a menu"
        );
        assert_ne!(
            state.mode,
            Mode::ContextMenu,
            "and must not have its input taken by one"
        );
        state.restore_viewer(None);

        // The display that opened it still has it.
        state.enter_viewer(Some(1));
        assert!(state.context_menu.is_some());
        assert_eq!(state.mode, Mode::ContextMenu);
        state.restore_viewer(None);
    }

    // TP-SUR-OVERLAY-02
    #[test]
    fn each_display_types_into_its_own_prompt() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("only"));
        state.active = Some(0);

        // Both displays are known before either types.
        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        state.restore_viewer(None);

        state.enter_viewer(Some(1));
        state.mode = Mode::RenameTab;
        state.name_input = "from display one".to_string();
        state.restore_viewer(None);

        state.enter_viewer(Some(2));
        assert_eq!(
            state.name_input, "",
            "one display's keystrokes must not appear in another's field"
        );
        state.name_input = "from display two".to_string();
        state.restore_viewer(None);

        state.enter_viewer(Some(1));
        assert_eq!(state.name_input, "from display one");
        state.restore_viewer(None);
    }

    // TP-SUR-BROADCAST-01
    #[test]
    fn a_mode_the_session_sets_itself_reaches_every_display() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("only"));
        state.active = Some(0);

        // Two displays are already attached and settled.
        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        state.restore_viewer(None);

        // The session puts itself in terminal mode with no display behind it
        // -- an API call focusing a pane does exactly this.
        state.mode = Mode::Terminal;

        for client in [1, 2] {
            state.enter_viewer(Some(client));
            assert_eq!(
                state.mode,
                Mode::Terminal,
                "a display parked in another mode would swallow everything typed into it"
            );
            state.restore_viewer(None);
        }
    }

    // TP-SUR-BROADCAST-02
    #[test]
    fn a_workspace_the_session_picks_does_not_move_the_displays() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("left"));
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("right"));
        state.active = Some(0);

        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        state.restore_viewer(None);

        // The counterpart to the broadcast above: choosing a workspace for one
        // display is the point of keeping them apart, so this must not travel.
        state.active = Some(1);

        state.enter_viewer(Some(1));
        assert_eq!(
            state.active,
            Some(0),
            "an API workspace switch must not drag a display that is watching another"
        );
        state.restore_viewer(None);
    }

    // TP-SUR-RAIL-01
    #[test]
    fn the_left_rail_and_its_scroll_belong_to_each_display() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("only"));
        state.active = Some(0);

        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        state.restore_viewer(None);

        // One display goes to Projects, scrolls, and folds a row away.
        state.enter_viewer(Some(1));
        state.sidebar_tab = SidebarTab::Projects;
        state.projects_scroll = 12;
        state.selected = 3;
        state.collapsed_space_keys.insert("space-a".to_string());
        state.restore_viewer(None);

        state.enter_viewer(Some(2));
        assert_eq!(
            state.sidebar_tab,
            SidebarTab::Spaces,
            "the other display must stay on the rail tab it was on"
        );
        assert_eq!(state.projects_scroll, 0, "a scroll is about one viewport");
        assert_eq!(state.selected, 0);
        assert!(
            state.collapsed_space_keys.is_empty(),
            "folding a row away is a choice about one screen"
        );
        state.restore_viewer(None);

        state.enter_viewer(Some(1));
        assert_eq!(state.sidebar_tab, SidebarTab::Projects);
        assert_eq!(state.projects_scroll, 12);
        assert_eq!(state.selected, 3);
        state.restore_viewer(None);
    }

    // TP-SUR-GEOMETRY-01
    #[test]
    fn sidebar_geometry_is_measured_against_the_display_it_is_drawn_on() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("only"));
        state.active = Some(0);

        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        state.restore_viewer(None);

        // A narrow display collapses its sidebar; a wide one must not be
        // dragged into a layout chosen for a screen it is not.
        state.enter_viewer(Some(1));
        state.sidebar_collapsed = true;
        state.sidebar_width = 12;
        state.restore_viewer(None);

        state.enter_viewer(Some(2));
        assert!(!state.sidebar_collapsed);
        assert_ne!(state.sidebar_width, 12);
        state.restore_viewer(None);
    }

    // TP-SUR-GESTURE-01
    #[test]
    fn a_display_seen_for_the_first_time_is_not_halfway_through_a_drag() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("only"));
        state.active = Some(0);

        state.enter_viewer(Some(1));
        state.drag = Some(DragState {
            target: DragTarget::WorkspaceListScrollbar { grab_row_offset: 2 },
        });
        state.restore_viewer(None);

        // A gesture is anchored to a rectangle in one display's last frame,
        // so a display that never saw that frame cannot be given it.
        state.enter_viewer(Some(2));
        assert!(state.drag.is_none());
        state.restore_viewer(None);

        state.enter_viewer(Some(1));
        assert!(state.drag.is_some(), "and the display dragging keeps it");
        state.restore_viewer(None);
    }

    // TP-SUR-FM-05
    #[test]
    fn a_request_one_display_made_is_not_consumed_by_another() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("only"));
        state.active = Some(0);

        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        state.restore_viewer(None);

        // The second display clicks a location in the rail.
        state.enter_viewer(Some(2));
        state.request_file_manager_location_navigation = Some(
            FileManagerLocationNavigationRequest::from(std::path::PathBuf::from("/tmp")),
        );
        state.restore_viewer(None);

        // Scheduled work serves displays in order, lowest first. The display
        // that clicked nothing must find nothing to do -- otherwise it takes
        // the other display's request and navigates its own browser with it,
        // and the display that actually clicked is left inert.
        state.enter_viewer(Some(1));
        assert!(
            state.request_file_manager_location_navigation.is_none(),
            "a request belongs to the display that made it"
        );
        state.restore_viewer(None);

        state.enter_viewer(Some(2));
        assert!(
            state.request_file_manager_location_navigation.is_some(),
            "and is still waiting when that display is served"
        );
        state.restore_viewer(None);
    }

    // TP-SUR-FM-06
    #[test]
    fn every_browser_request_belongs_to_the_display_that_made_it() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("only"));
        state.active = Some(0);

        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        state.restore_viewer(None);

        // The second display asks its browser to do things.
        state.enter_viewer(Some(2));
        state.request_file_manager_delete = Some(FileManagerDeleteRequest {
            kind: FileManagerDeleteKind::Trash,
            paths: vec![std::path::PathBuf::from("/tmp/x")],
        });
        state.restore_viewer(None);

        // Every one of these acts on the asking display's browser and its
        // rail focus, so none of them may be visible to another display --
        // consumed there, they act on the wrong browser or, more often,
        // on none at all, and the click looks ignored.
        state.enter_viewer(Some(1));
        assert!(state.request_file_manager_delete.is_none());
        state.restore_viewer(None);

        state.enter_viewer(Some(2));
        assert!(
            state.request_file_manager_delete.is_some(),
            "and is still waiting when the asking display is served"
        );
        state.restore_viewer(None);
    }

    // TP-SUR-FM-07
    #[test]
    fn a_context_action_is_visible_only_where_it_was_raised() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("only"));
        state.active = Some(0);

        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        state.restore_viewer(None);

        // A context action raised on the second display. Three different
        // consumers compete for this one field -- send-to-agent, plugin
        // dispatch, and the ordinary actions -- and every one of them must
        // look for it in the view that raised it. A consumer left outside
        // finds the registers holding nobody's request once a second display
        // attaches, and its whole branch goes quiet.
        state.enter_viewer(Some(2));
        state.request_file_manager_context_action = Some(FileManagerContextActionIntent {
            action: FileManagerContextMenuAction::Copy,
            paths: vec![std::path::PathBuf::from("/tmp/x")],
        });
        state.restore_viewer(None);

        // Outside every display's view -- where the three consumers used to
        // run -- there is nothing to act on.
        assert!(
            state.request_file_manager_context_action.is_none(),
            "no consumer may see it from outside a display's view"
        );

        state.enter_viewer(Some(1));
        assert!(state.request_file_manager_context_action.is_none());
        state.restore_viewer(None);

        state.enter_viewer(Some(2));
        assert!(
            state.request_file_manager_context_action.is_some(),
            "it is still waiting where it was raised"
        );
        state.restore_viewer(None);
    }

    // TP-MCF-MODE-01
    #[test]
    fn mirror_mode_puts_every_display_back_on_one_view() {
        let mut state = AppState::test_new();
        state.per_display_focus = false;
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("left"));
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("right"));
        state.active = Some(0);

        // With mirroring on, one display moving moves the session, and every
        // other display follows it -- the behaviour this feature replaced.
        state.enter_viewer(Some(1));
        state.active = Some(1);
        state.restore_viewer(None);

        state.enter_viewer(Some(2));
        assert_eq!(
            state.active,
            Some(1),
            "mirror mode must reproduce the shared view exactly"
        );
        assert_eq!(state.viewer(), None, "no per-display view is recorded");
        assert!(state.surfaces_by_client.is_empty());
        state.restore_viewer(None);
    }

    // TP-MCF-MODE-02
    #[test]
    fn a_session_with_no_clients_resolves_the_default_without_panicking() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("only"));
        state.active = Some(0);

        // A display attaches, moves nothing, and leaves again.
        state.enter_viewer(Some(1));
        state.restore_viewer(None);
        state.forget_client(1);

        assert_eq!(state.active, Some(0));
        assert_eq!(state.workspaces[0].active_tab_index(), 0);
    }

    // TP-MCF-WS-03
    #[test]
    fn a_client_that_attached_before_any_workspace_existed_follows_the_first_one() {
        let mut state = AppState::test_new();
        assert_eq!(state.active, None, "the session has no workspace yet");

        // The display attaches and renders before the session has a workspace.
        state.enter_viewer(Some(1));
        state.restore_viewer(None);

        // The session then creates one, outside any viewer window, the way the
        // API and startup paths do.
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("first"));
        state.active = Some(0);

        state.enter_viewer(Some(1));
        assert_eq!(
            state.active,
            Some(0),
            "an empty slot is the absence of a choice; the display must follow \
             the session instead of resolving to no workspace forever"
        );
        state.restore_viewer(None);
    }

    // TP-MCF-CTX-04
    #[test]
    fn a_workspace_created_inside_a_viewer_window_falls_back_to_the_default() {
        let mut state = AppState::test_new();
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("before"));
        state.enter_viewer(Some(2));

        // Mirroring runs when the window opens, so a workspace added while it
        // is open carries no viewer until the next window. That is safe rather
        // than merely tolerated: a brand-new workspace has one tab, so the
        // default it falls back to is the tab the viewer would have resolved.
        state
            .workspaces
            .push(crate::workspace::Workspace::test_new("during"));
        assert_eq!(state.workspaces[0].viewer(), Some(2));
        assert_eq!(state.workspaces[1].viewer(), None);

        // The next window reconciles it.
        state.restore_viewer(None);
        state.enter_viewer(Some(2));
        assert_eq!(state.workspaces[1].viewer(), Some(2));
    }
}

#[cfg(test)]
mod chat_drawer_mode_tests {
    use super::*;

    fn state_with_workspace(cwd: &str) -> (AppState, String) {
        let mut state = AppState::test_new();
        let mut ws = crate::workspace::Workspace::test_new("a");
        ws.identity_cwd = std::path::PathBuf::from(cwd);
        let key = crate::persist::workspace_chats::ledger_key(&ws.identity_cwd);
        state.workspaces.push(ws);
        (state, key)
    }

    fn give_workspace_an_agent(state: &mut AppState, ws_idx: usize) {
        let pane = state.workspaces[ws_idx].tabs[0].root_pane;
        let id = state.workspaces[ws_idx].terminal_id(pane).unwrap().clone();
        let mut terminal = crate::terminal::state::TerminalState::new(id.clone(), "/tmp".into());
        terminal.set_agent_name("planner".into());
        state.terminals.insert(id, terminal);
    }

    // TP-DRAWER-03
    #[test]
    fn an_agent_workspace_derives_an_open_drawer_in_all_active() {
        let (mut state, _key) = state_with_workspace("/repo/a");
        give_workspace_an_agent(&mut state, 0);

        assert_eq!(state.chat_drawer_mode, ChatDrawerMode::AllActive);
        assert!(
            !state.chat_drawer_collapsed(0),
            "a live agent derives an open drawer in all-active"
        );
    }

    #[test]
    fn an_agentless_workspace_opens_only_by_hand() {
        let (mut state, key) = state_with_workspace("/repo/a");

        assert!(
            state.chat_drawer_collapsed(0),
            "no agent, no derivation — closed stays the default"
        );

        state.expanded_chat_workspaces.insert(key);
        assert!(
            !state.chat_drawer_collapsed(0),
            "the expanded set still opens a drawer by hand"
        );
    }

    // TP-DRAWER-04
    #[test]
    fn a_quieted_drawer_stays_shut_despite_a_live_agent() {
        let (mut state, key) = state_with_workspace("/repo/a");
        give_workspace_an_agent(&mut state, 0);

        state.suppressed_chat_drawers.insert(key);
        assert!(
            state.chat_drawer_collapsed(0),
            "quieting beats the derivation — the person's hand wins"
        );
    }

    // TP-DRAWER-05
    #[test]
    fn focused_and_manual_never_derive_an_open_drawer() {
        for mode in [ChatDrawerMode::Focused, ChatDrawerMode::Manual] {
            let (mut state, key) = state_with_workspace("/repo/a");
            give_workspace_an_agent(&mut state, 0);
            state.chat_drawer_mode = mode;

            assert!(
                state.chat_drawer_collapsed(0),
                "{mode:?} must not derive an open drawer"
            );

            state.expanded_chat_workspaces.insert(key.clone());
            assert!(
                !state.chat_drawer_collapsed(0),
                "{mode:?} still honours the expanded set"
            );
        }
    }

    // TP-DRAWER-08: the whole pipeline is per-display. One display quieting a
    // derived drawer changes what IT sees and nothing about what any other
    // LIVING display sees — the constitution's sentence, proven at the
    // verdict level rather than set by set. (A display seen for the first
    // time adopts the driven default on purpose — TP-SUR-DEFAULT-01 — which
    // is why both displays are introduced before either acts.)
    #[test]
    fn one_displays_quieting_never_moves_anothers_derived_drawer() {
        let (mut state, key) = state_with_workspace("/repo/a");
        give_workspace_an_agent(&mut state, 0);
        state.active = Some(0);

        // Both displays exist before either touches anything.
        state.enter_viewer(Some(2));
        state.restore_viewer(None);
        state.enter_viewer(Some(3));
        state.restore_viewer(None);

        // Display 2 quiets the derived drawer and parks.
        state.enter_viewer(Some(2));
        state.suppressed_chat_drawers.insert(key);
        assert!(state.chat_drawer_collapsed(0), "display 2 closed its view");
        state.restore_viewer(None);

        // Display 3 still sees the derivation.
        state.enter_viewer(Some(3));
        assert!(
            !state.chat_drawer_collapsed(0),
            "display 3's derived drawer is untouched by display 2's quieting"
        );
        state.restore_viewer(None);

        // And display 2 still sees its own quieting.
        state.enter_viewer(Some(2));
        assert!(
            state.chat_drawer_collapsed(0),
            "display 2 keeps the view it chose"
        );
        state.restore_viewer(None);
    }

    // The derivation and the agents panel must speak the same criterion, or
    // a drawer opens for a workspace the panel does not list (or the other
    // way round) and the two surfaces argue about what "active agent" means.
    #[test]
    fn the_derivation_uses_the_panels_agent_criterion() {
        let (mut state, _key) = state_with_workspace("/repo/a");
        let pane = state.workspaces[0].tabs[0].root_pane;
        let id = state.workspaces[0].terminal_id(pane).unwrap().clone();

        // A terminal with no agent name and no detected agent puts no row in
        // the agents panel, so it must not derive a drawer either.
        state.terminals.insert(
            id.clone(),
            crate::terminal::state::TerminalState::new(id.clone(), "/tmp".into()),
        );
        assert!(
            state.chat_drawer_collapsed(0),
            "an agent-less terminal is not an agent entry"
        );

        // A detected agent with no name IS a panel row — and derives.
        if let Some(terminal) = state.terminals.get_mut(&id) {
            terminal.set_detected_state(Some(crate::detect::Agent::Claude), AgentState::Working);
        }
        assert!(
            !state.chat_drawer_collapsed(0),
            "a detected agent is a panel row, so it derives"
        );
    }
}
