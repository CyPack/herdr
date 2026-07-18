use crate::config::{Keybinds, NewTerminalCwdConfig, SoundConfig, ToastConfig, ToastDelivery};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Direction, Rect};
use ratatui::style::Color;
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
    Copy,
    Rename,
    Delete,
    Compress,
    SendAgent,
    Plugin {
        plugin_id: String,
        action_id: String,
    },
}

impl FileManagerContextMenuAction {
    pub const ALL: [Self; 6] = [
        Self::Open,
        Self::Copy,
        Self::Rename,
        Self::Delete,
        Self::Compress,
        Self::SendAgent,
    ];

    pub fn label(&self) -> &str {
        match self {
            Self::Open => "Open",
            Self::Copy => "Copy",
            Self::Rename => "Rename",
            Self::Delete => "Delete",
            Self::Compress => "Compress",
            Self::SendAgent => "Send to Agent",
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
                        FileManagerContextMenuAction::Compress => {
                            Some(FileManagerActionDisabledReason::UnsupportedAction)
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

#[cfg(test)]
pub use super::file_manager_sidebar::{
    FileManagerSidebarIcon, FileManagerSidebarSectionKind, FILE_MANAGER_SIDEBAR_MAX_ITEMS,
};
pub use super::file_manager_sidebar::{
    FileManagerSidebarItem, FileManagerSidebarModel, FileManagerSidebarRowArea,
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
    /// Hit areas for the Spaces/Projects/Files header tabs (one per
    /// `SidebarTab::ALL`, in order). Empty when the sidebar is collapsed.
    pub sidebar_tab_hit_areas: Vec<Rect>,
    /// Laid-out rows for the Projects tab (project headers + chat sessions).
    /// Empty on every non-Projects tab and when the sidebar is collapsed.
    pub project_row_areas: Vec<ProjectRowArea>,
    /// Exact clickable item rows for the prepared Files sidebar. Empty on
    /// every non-Files tab and when the sidebar is collapsed.
    pub file_manager_sidebar_row_areas: Vec<FileManagerSidebarRowArea>,
    /// Complete AppDock entry targets for the current frame. Empty whenever
    /// the live shell projects no dock region.
    pub app_dock_entry_areas: Vec<crate::ui::app_dock::AppDockEntryArea>,
    /// Bounded logical Miller columns and dividers projected for the current
    /// Files frame. Empty while Files is closed or its body cannot fit one
    /// complete minimum-width column.
    #[allow(dead_code)] // P1 establishes compute ownership; P2 removes this
    // once render/input consume the snapshot.
    pub(crate) file_manager_miller: crate::ui::MillerViewSnapshot,
    /// Visible CURRENT rows for the native file manager. Empty while FM is
    /// closed or when its content area has no drawable rows.
    pub file_manager_row_areas: Vec<FileManagerRowArea>,
    /// Exact, disjoint action targets at the right edge of visible CURRENT
    /// rows. Empty while FM is closed or the row cannot fit a complete action.
    pub file_manager_row_action_areas: Vec<FileManagerRowActionArea>,
    /// Named native-FM header actions for this frame. Empty while FM is closed
    /// or when the header cannot preserve its minimum identity width.
    pub file_manager_header_action_areas: Vec<FileManagerHeaderActionArea>,
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
    pub tab_scroll_left_hit_area: Rect,
    pub tab_scroll_right_hit_area: Rect,
    pub new_tab_hit_area: Rect,
    pub terminal_area: Rect,
    pub mobile_header_rect: Rect,
    pub mobile_menu_hit_area: Rect,
    pub toast_hit_area: Rect,
    pub pane_infos: Vec<PaneInfo>,
    pub split_borders: Vec<SplitBorder>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Onboarding,
    ReleaseNotes,
    ProductAnnouncement,
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
}

impl Mode {
    /// Whether keys in this mode are commands/navigation (an ASCII input source is wanted) rather
    /// than free text. This is an explicit **allowlist** of the prefix command/navigation realm:
    /// any mode NOT listed defaults to leaving the user's IME alone (the safe default), so adding a
    /// new text-entry or overlay mode can never silently force ASCII. Used by
    /// `sync_prefix_input_source` (gated by `switch_ascii_input_source_in_prefix`) so multi-level
    /// prefix commands keep ASCII until they return to the terminal.
    ///
    /// Known limitation: `Navigator`'s search box is also held on ASCII, since this `Mode`-level
    /// predicate can't see `search_focused` (non-ASCII filtering there would need a runtime check).
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NavigatorStateFilter {
    Blocked,
    Working,
    Idle,
    Done,
}

#[derive(Debug, Clone, Default)]
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

// ---------------------------------------------------------------------------
// Settings UI state
// ---------------------------------------------------------------------------

/// Which section of the settings panel is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub(crate) struct DragState {
    pub target: DragTarget,
}

pub(crate) struct WorkspacePressState {
    pub ws_idx: usize,
    pub start_col: u16,
    pub start_row: u16,
}

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
    /// Agent selector for a new chat in a pinned project (Projects tab).
    /// Selecting an agent makes it the persisted default and opens the chat.
    /// When the project is also open as a workspace, the menu additionally
    /// offers that workspace's worktree actions (mirroring the Spaces menu).
    ProjectNewChat {
        proj_idx: usize,
        has_workspace: bool,
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
            ContextMenuKind::GitWorkspace {
                is_linked_worktree: true,
                ..
            } => vec!["Rename", "Close", "Delete worktree checkout..."],
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
            ContextMenuKind::Tab { .. } => vec!["New tab", "Rename", "Close"],
            ContextMenuKind::ProjectNewChat {
                has_workspace: false,
                ..
            } => crate::app::projects::CHAT_AGENTS.to_vec(),
            ContextMenuKind::ProjectNewChat {
                has_workspace: true,
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

pub struct KeybindHelpState {
    pub scroll: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarWidthSource {
    ConfigDefault,
    Persisted,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PaneFocusTarget {
    pub workspace_id: String,
    pub pane_id: PaneId,
}

/// All application state — pure data, no channels or async runtime.
/// Testable without PTYs or a tokio runtime.
pub struct AppState {
    pub terminals:
        std::collections::HashMap<crate::terminal::TerminalId, crate::terminal::TerminalState>,
    /// Terminal ids whose size is currently owned by a direct attach client.
    pub direct_attach_resize_locks: std::collections::HashSet<crate::terminal::TerminalId>,
    pub(crate) pane_id_aliases: std::collections::HashMap<u32, PaneId>,
    pub(crate) public_pane_id_aliases: std::collections::HashMap<String, PaneId>,
    pub workspaces: Vec<Workspace>,
    pub active: Option<usize>,
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
    /// Prepared, bounded Files-sidebar data. Filesystem/environment discovery
    /// happens only when this projection is refreshed, never during render.
    pub file_manager_sidebar: FileManagerSidebarModel,
    /// Exact row path prepared by input and consumed once by the App-owned
    /// scheduled navigation boundary.
    pub request_file_manager_sidebar_navigation: Option<PathBuf>,
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
    pub rename_pane_target: Option<PaneId>,
    pub worktree_create: Option<WorktreeCreateState>,
    pub worktree_open: Option<WorktreeOpenState>,
    pub worktree_remove: Option<WorktreeRemoveState>,
    pub worktree_directory: std::path::PathBuf,
    pub collapsed_space_keys: std::collections::HashSet<String>,
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
    // View geometry (computed before render, consumed by render + mouse)
    pub view: ViewState,
    /// Transient shell capture/preview state. Never persisted and never owns
    /// runtime resources.
    pub(crate) shell_interaction: crate::ui::shell::ShellInteractionState,
    /// Committed client-local shell presentation preferences. SF3.3 persists
    /// this aggregate through the versioned shell snapshot contract.
    pub(crate) shell_presentation: crate::ui::shell::ShellPresentationState,
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
    pub sidebar_collapsed_mode: crate::config::SidebarCollapsedModeConfig,
    /// Ratio of sidebar height allocated to the workspaces section.
    pub sidebar_section_split: f32,
    pub agent_panel_sort: AgentPanelSort,
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
    pub pane_borders: bool,
    pub pane_gaps: bool,
    pub show_agent_labels_on_pane_borders: bool,
    pub hide_tab_bar_when_single_tab: bool,
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
    /// Recent plugin action/event command executions.
    pub(crate) plugin_command_logs: Vec<crate::api::schema::PluginCommandLogInfo>,
    pub(crate) next_plugin_command_log_id: u64,
    pub(crate) plugin_commands_in_flight: usize,
    /// Highlight state for the bottom-right global launcher menu.
    pub global_menu: MenuListState,
    /// Resolved host terminal default colors for theming embedded panes.
    pub host_terminal_theme: TerminalTheme,
    /// Set when a persisted session snapshot would change.
    pub session_dirty: bool,
    /// Terminal runtimes that should be shut down by the app/runtime layer
    /// after state has detached their terminal metadata.
    pub(crate) terminal_runtime_shutdowns: Vec<crate::terminal::TerminalId>,
}

impl AppState {
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
        self.mouse_capture || self.focused_pane_requests_mouse_capture_from(terminal_runtimes)
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
            terminals: std::collections::HashMap::new(),
            direct_attach_resize_locks: std::collections::HashSet::new(),
            pane_id_aliases: std::collections::HashMap::new(),
            public_pane_id_aliases: std::collections::HashMap::new(),
            workspaces: Vec::new(),
            active: None,
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
            file_manager_operation: None,
            file_manager_delete_confirmation: None,
            file_manager_rename: None,
            request_file_manager_rename: None,
            request_file_manager_bulk_rename: None,
            request_file_manager_delete: None,
            request_file_manager_context_action: None,
            request_file_manager_agent_handoff: None,
            agent_reference_picker: None,
            file_manager_sidebar: FileManagerSidebarModel::default(),
            request_file_manager_sidebar_navigation: None,
            should_quit: false,
            detach_exits: false,
            detach_requested: false,
            request_new_workspace: false,
            request_new_tab: false,
            request_new_linked_worktree: None,
            request_open_existing_worktree: None,
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
            rename_pane_target: None,
            worktree_create: None,
            worktree_open: None,
            worktree_remove: None,
            worktree_directory: std::path::PathBuf::from("/tmp/herdr-worktrees"),
            collapsed_space_keys: std::collections::HashSet::new(),
            projects_pinned: Vec::new(),
            projects_sessions: Vec::new(),
            preview_bindings: Vec::new(),
            preview_placement: crate::config::PreviewPlacement::default(),
            collapsed_project_paths: std::collections::HashSet::new(),
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
            keybind_help: KeybindHelpState { scroll: 0 },
            navigator: NavigatorState::default(),
            copy_mode: None,
            sidebar_tab: SidebarTab::Spaces,
            workspace_scroll: 0,
            agent_panel_scroll: 0,
            projects_scroll: 0,
            tab_scroll: 0,
            tab_scroll_follow_active: true,
            mobile_switcher_scroll: 0,
            view: ViewState {
                layout: ViewLayout::Desktop,
                shell: Default::default(),
                sidebar_rect: Rect::default(),
                workspace_card_areas: Vec::new(),
                sidebar_tab_hit_areas: Vec::new(),
                project_row_areas: Vec::new(),
                file_manager_sidebar_row_areas: Vec::new(),
                app_dock_entry_areas: Vec::new(),
                file_manager_miller: Default::default(),
                file_manager_row_areas: Vec::new(),
                file_manager_row_action_areas: Vec::new(),
                file_manager_header_action_areas: Vec::new(),
                file_manager_action_bar: None,
                agent_attachment_action_area: None,
                agent_worktree_action_area: None,
                agent_attachment_picker_row_areas: Vec::new(),
                tab_bar_rect: Rect::default(),
                tab_hit_areas: Vec::new(),
                tab_scroll_left_hit_area: Rect::default(),
                tab_scroll_right_hit_area: Rect::default(),
                new_tab_hit_area: Rect::default(),
                terminal_area: Rect::default(),
                mobile_header_rect: Rect::default(),
                mobile_menu_hit_area: Rect::default(),
                toast_hit_area: Rect::default(),
                pane_infos: Vec::new(),
                split_borders: Vec::new(),
            },
            shell_interaction: Default::default(),
            shell_presentation: crate::ui::shell::ShellPresentationState::new(26),
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
            sidebar_collapsed_mode: crate::config::SidebarCollapsedModeConfig::Compact,
            sidebar_section_split: 0.5,
            agent_panel_sort: AgentPanelSort::Spaces,
            next_agent_state_change_seq: 0,
            mouse_capture: true,
            copy_on_select: true,
            right_click_passthrough_modifiers: None,
            right_click_passthrough: None,
            redraw_on_focus_gained: true,
            mouse_scroll_lines: crate::config::DEFAULT_MOUSE_SCROLL_LINES,
            confirm_close: true,
            prompt_new_tab_name: true,
            pane_borders: true,
            pane_gaps: false,
            show_agent_labels_on_pane_borders: false,
            hide_tab_bar_when_single_tab: false,
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
            plugin_command_logs: Vec::new(),
            next_plugin_command_log_id: 1,
            plugin_commands_in_flight: 0,
            global_menu: MenuListState::new(0),
            host_terminal_theme: TerminalTheme::default(),
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
                | ContextMenuKind::GitWorkspace { ws_idx, .. } => {
                    assert_workspace_index(ws_idx, "context menu workspace")
                }
                ContextMenuKind::Tab { ws_idx, tab_idx } => {
                    assert_tab_index(ws_idx, tab_idx, "context menu tab")
                }
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
                ContextMenuKind::AppDock { .. } => {
                    // The dock popover references only a closed built-in app
                    // id; there is no index-shaped identity to validate.
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
    fn file_sidebar_preparation_uses_live_home_and_pin_state() {
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

        let model = FileManagerSidebarModel::from_home_and_pins(
            &home.0,
            &[home.0.clone(), missing_pin.clone()],
        );

        let favorites = model
            .section(FileManagerSidebarSectionKind::Favorites)
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
            .section(FileManagerSidebarSectionKind::Pinned)
            .expect("inaccessible configured pin remains visible");
        assert_eq!(pinned.items.len(), 1, "Home duplicate is removed");
        assert_eq!(pinned.items[0].path, missing_pin);
        assert!(!pinned.items[0].accessible);

        let locations = model
            .section(FileManagerSidebarSectionKind::Locations)
            .expect("root location");
        assert_eq!(locations.items[0].label, "Root");
        assert!(locations.items[0].path.is_absolute());
    }

    // TP-C6.1-MODEL: adversarial configuration cannot create an unbounded
    // client-side sidebar model or move later duplicates ahead of first use.
    #[test]
    fn file_sidebar_model_is_bounded_across_all_sections() {
        let items = (0..FILE_MANAGER_SIDEBAR_MAX_ITEMS + 32)
            .map(|index| FileManagerSidebarItem {
                label: format!("item-{index}"),
                path: PathBuf::from(format!("/virtual/{index}")),
                icon: FileManagerSidebarIcon::Pin,
                accessible: true,
                ejectable: false,
            })
            .collect();
        let model = FileManagerSidebarModel::from_sources(Vec::new(), items, Vec::new());

        assert_eq!(model.item_count(), FILE_MANAGER_SIDEBAR_MAX_ITEMS);
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
        let active_public = ws.tabs[ws.active_tab].number;
        assert_ne!(ws.active_tab + 1, active_public);
        let new_pane = ws.test_split(ratatui::layout::Direction::Horizontal);
        assert!(ws.public_pane_number(new_pane).is_some());
        state.ensure_test_terminals();

        state.assert_invariants_for_test();
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

    #[test]
    fn linked_worktree_context_menu_keeps_safe_close_and_explicit_remove() {
        let menu = ContextMenuState {
            kind: ContextMenuKind::GitWorkspace {
                ws_idx: 0,
                is_linked_worktree: true,
                has_worktree_children: false,
                collapsed: false,
            },
            x: 0,
            y: 0,
            list: MenuListState::new(0),
        };

        assert_eq!(
            menu.items(),
            &["Rename", "Close", "Delete worktree checkout..."]
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
                if item.action == FileManagerContextMenuAction::Compress {
                    !item.enabled
                        && item.disabled_reason
                            == Some(FileManagerActionDisabledReason::UnsupportedAction)
                } else {
                    item.enabled
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
                FileManagerContextMenuAction::Copy,
                FileManagerContextMenuAction::Rename,
                FileManagerContextMenuAction::Delete,
                FileManagerContextMenuAction::Compress,
                FileManagerContextMenuAction::SendAgent,
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
                "Copy",
                "Rename",
                "Delete",
                "Compress",
                "Send to Agent",
            ]
        );
        for action in [
            FileManagerContextMenuAction::Open,
            FileManagerContextMenuAction::Copy,
            FileManagerContextMenuAction::SendAgent,
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
                "Copy",
                "Rename",
                "Delete",
                "Compress",
                "Send to Agent",
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
            command: vec!["inspect".into()],
            platforms: None,
        }
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
        assert_eq!(model.items.len(), 8);
        assert_eq!(
            model.items[6..]
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Inspect with Alpha", "Inspect with Zeta"]
        );
        let plugin_action = FileManagerContextMenuAction::Plugin {
            plugin_id: "alpha.files".into(),
            action_id: "inspect".into(),
        };
        assert_eq!(model.items[6].action, plugin_action);
        assert!(model.items[6].enabled);

        let intent = FileManagerContextActionIntent {
            action: model.items[6].action.clone(),
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
