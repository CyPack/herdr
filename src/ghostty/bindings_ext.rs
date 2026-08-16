//! Hand-written FFI declarations for local libghostty-vt patches.
//!
//! `bindings.rs` is generated offline by rust-bindgen from the vendored
//! headers, so regenerating it would silently drop anything added by hand.
//! Externs for symbols that exist only through `vendor/patches/libghostty-vt/`
//! live here instead, written in bindgen's own shapes, and each one names the
//! patch that provides it — remove the extern when the patch is removed.

use super::bindings::{
    GhosttyAllocator, GhosttyFormatter, GhosttyFormatterTerminalOptions, GhosttyResult,
    GhosttyTerminal, GhosttyTerminalScreen,
};

unsafe extern "C" {
    /// Create a formatter for one specific screen of a terminal.
    ///
    /// Patch 0002 (`0002-formatter-screen-new.patch`): unlike
    /// `ghostty_formatter_terminal_new`, which always formats the currently
    /// active screen, this formats the requested screen even while the other
    /// one is active — the primary screen's scrollback stays readable while a
    /// fullscreen application holds the alternate screen. Only the
    /// screen-scoped extras in `options.extra.screen` apply; a null
    /// `options.selection` formats the whole screen.
    pub fn ghostty_formatter_screen_new(
        allocator: *const GhosttyAllocator,
        formatter: *mut GhosttyFormatter,
        terminal: GhosttyTerminal,
        screen: GhosttyTerminalScreen,
        options: GhosttyFormatterTerminalOptions,
    ) -> GhosttyResult;
}
