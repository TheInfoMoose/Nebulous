use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Pure CRT obsidian background painted across the entire terminal buffer
/// so host terminal themes never bleed through (`#000000`).
pub const BG_BLACK: Color = Color::Rgb(0, 0, 0);

/// Subtle warm dark phosphor canvas for modals and active panels (`#0C0800`).
pub const PANEL_BG: Color = Color::Rgb(12, 8, 0);

/// Classic P3 phosphor amber for primary text, active borders, and ASCII logo (`#FFB000`).
pub const AMBER_PRIMARY: Color = Color::Rgb(255, 176, 0);

/// High-intensity gold for focused items, active stage tabs, and `NEBULOUS` center glyphs (`#FFD866`).
pub const AMBER_BRIGHT: Color = Color::Rgb(255, 216, 102);

/// Muted amber for inactive borders, nebula outer dust rings, and secondary labels (`#996300`).
pub const AMBER_DIM: Color = Color::Rgb(153, 99, 0);

/// Deep amber for subtle grid dividers and outer starfield (`#5C3A00`).
pub const AMBER_DARK: Color = Color::Rgb(92, 58, 0);

/// High-contrast orange-amber for `critical`/`high` bugs, recurring regressions,
/// and `[REQUIRED FIELD LOCK]` validation alerts (`#FF5500`).
pub const AMBER_ALERT: Color = Color::Rgb(255, 85, 0);

/// Width threshold for selecting the full 5-line × 76-column ASCII Nebula banner.
pub const FULL_BANNER_MIN_WIDTH: u16 = 78;
/// Height threshold for selecting the full 5-line × 76-column ASCII Nebula banner.
pub const FULL_BANNER_MIN_HEIGHT: u16 = 26;

/// Locked dimensions of the full ASCII Nebula banner (`5 lines × 76 cols`).
pub const FULL_BANNER_COLS: u16 = 76;
pub const FULL_BANNER_LINES: u16 = 5;

/// Locked dimensions of the compact ASCII Nebula banner (`3 lines × 60 cols`).
pub const COMPACT_BANNER_COLS: u16 = 60;
pub const COMPACT_BANNER_LINES: u16 = 3;

/// Aspect-locked tier for the centered ASCII Nebula + `NEBULOUS` header banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerTier {
    /// 5 lines × 76 columns (`width >= 78 && height >= 26`).
    Full,
    /// 3 lines × 60 columns (`width < 78 || height < 26`).
    Compact,
}

/// Non-deforming, aspect-locked ASCII Nebula + `NEBULOUS` banner.
#[derive(Debug, Clone)]
pub struct NebulaBanner {
    pub tier: BannerTier,
    pub cols: u16,
    pub height: u16,
    pub lines: Vec<Line<'static>>,
}

/// Base style painting pure CRT black background with primary P3 phosphor amber foreground.
pub fn style_base() -> Style {
    Style::default().fg(AMBER_PRIMARY).bg(BG_BLACK)
}

/// High-intensity gold phosphor style on pure black background.
pub fn style_bright() -> Style {
    Style::default()
        .fg(AMBER_BRIGHT)
        .bg(BG_BLACK)
        .add_modifier(Modifier::BOLD)
}

/// Muted secondary phosphor style on pure black background.
pub fn style_dim() -> Style {
    Style::default().fg(AMBER_DIM).bg(BG_BLACK)
}

/// Deep starfield/divider phosphor style on pure black background.
pub fn style_dark() -> Style {
    Style::default().fg(AMBER_DARK).bg(BG_BLACK)
}

/// High-contrast alert style for critical/high bugs, regressions, and required field locks.
pub fn style_alert() -> Style {
    Style::default()
        .fg(AMBER_ALERT)
        .bg(BG_BLACK)
        .add_modifier(Modifier::BOLD)
}

/// Highlight style for selected menu/table rows in the Amber-on-Black TUI.
pub fn style_selected() -> Style {
    Style::default()
        .fg(AMBER_BRIGHT)
        .bg(PANEL_BG)
        .add_modifier(Modifier::BOLD)
}

/// Border style depending on whether a pane or field is currently focused.
pub fn style_border(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(AMBER_PRIMARY)
            .bg(BG_BLACK)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(AMBER_DIM).bg(BG_BLACK)
    }
}

/// Selects the aspect-locked, non-deforming ASCII Nebula + `NEBULOUS` header banner
/// based on the current terminal dimensions.
///
/// - When `width >= 78 && height >= 26`, returns the **Full Banner (`5 lines × 76 cols`)**
///   with multi-line block-letter `N E B U L O U S` glyphs framed by swirling cosmic dust.
/// - When `width < 78 || height < 26`, returns the **Compact Banner (`3 lines × 60 cols`)**
///   with centered `╔═╗ N E B U L O U S ╔═╗` art so the logo never wraps, clips, or distorts.
pub fn select_nebula_banner(width: u16, height: u16) -> NebulaBanner {
    if width >= FULL_BANNER_MIN_WIDTH && height >= FULL_BANNER_MIN_HEIGHT {
        build_full_nebula_banner()
    } else {
        build_compact_nebula_banner()
    }
}

fn build_full_nebula_banner() -> NebulaBanner {
    let dust_outer = Style::default().fg(AMBER_DARK).bg(BG_BLACK);
    let dust_inner = Style::default().fg(AMBER_DIM).bg(BG_BLACK);
    let glyph_frame = Style::default()
        .fg(AMBER_PRIMARY)
        .bg(BG_BLACK)
        .add_modifier(Modifier::BOLD);
    let glyph_center = Style::default()
        .fg(AMBER_BRIGHT)
        .bg(BG_BLACK)
        .add_modifier(Modifier::BOLD);

    // Every line is strictly 14 + 48 + 14 = 76 columns wide.
    let lines = vec![
        Line::from(vec![
            Span::styled("* . · ", dust_outer),
            Span::styled("░▒▓ ( ✦ ", dust_inner),
            Span::styled("╔════════════ ✦ ", glyph_frame),
            Span::styled("N E B U L O U S", glyph_center),
            Span::styled(" ✦ ═════════════╗", glyph_frame),
            Span::styled(" ✦ ) ▓▒░", dust_inner),
            Span::styled(" · . *", dust_outer),
        ]),
        Line::from(vec![
            Span::styled(" · ✦ ", dust_outer),
            Span::styled("░▒▓ (( * ", dust_inner),
            Span::styled(
                " ███╗  ████╗ ████╗ ██╗██ ██╗   ████╗ ██╗██ ████╗",
                glyph_center,
            ),
            Span::styled(" * )) ▓▒░", dust_inner),
            Span::styled(" ✦ · ", dust_outer),
        ]),
        Line::from(vec![
            Span::styled("* . ", dust_outer),
            Span::styled("░▒▓ ((( ✦ ", dust_inner),
            Span::styled(
                " ████╗ ██╔═╝ ████╔ ██║██ ██║   ██╔██ ██║██ ╚███╗",
                glyph_center,
            ),
            Span::styled(" ✦ ))) ▓▒░", dust_inner),
            Span::styled(" . *", dust_outer),
        ]),
        Line::from(vec![
            Span::styled(" · ✦ ", dust_outer),
            Span::styled("░▒▓ (( * ", dust_inner),
            Span::styled(
                " ██╔██ ████╗ ████╝ ╚███╔ ████╗ ╚███╔ ╚███╔ ████╔",
                glyph_frame,
            ),
            Span::styled(" * )) ▓▒░", dust_inner),
            Span::styled(" ✦ · ", dust_outer),
        ]),
        Line::from(vec![
            Span::styled("* . · ", dust_outer),
            Span::styled("░▒▓ ( ✦ ", dust_inner),
            Span::styled("╚════════ ✦ ", glyph_frame),
            Span::styled("NEBULOUS COMMAND TRACKER", glyph_center),
            Span::styled(" ✦ ════════╝", glyph_frame),
            Span::styled(" ✦ ) ▓▒░", dust_inner),
            Span::styled(" · . *", dust_outer),
        ]),
    ];

    NebulaBanner {
        tier: BannerTier::Full,
        cols: FULL_BANNER_COLS,
        height: FULL_BANNER_LINES,
        lines,
    }
}

fn build_compact_nebula_banner() -> NebulaBanner {
    let dust_outer = Style::default().fg(AMBER_DARK).bg(BG_BLACK);
    let dust_inner = Style::default().fg(AMBER_DIM).bg(BG_BLACK);
    let glyph_frame = Style::default()
        .fg(AMBER_PRIMARY)
        .bg(BG_BLACK)
        .add_modifier(Modifier::BOLD);
    let glyph_center = Style::default()
        .fg(AMBER_BRIGHT)
        .bg(BG_BLACK)
        .add_modifier(Modifier::BOLD);

    // Every line is strictly 12 + 36 + 12 = 60 columns wide.
    let lines = vec![
        Line::from(vec![
            Span::styled("* .·", dust_outer),
            Span::styled("░▒▓( ✦  ", dust_inner),
            Span::styled("╔══════════════════════════════════╗", glyph_frame),
            Span::styled("  ✦ )▓▒░", dust_inner),
            Span::styled("·. *", dust_outer),
        ]),
        Line::from(vec![
            Span::styled("·✦ ", dust_outer),
            Span::styled("░▒▓(( *  ", dust_inner),
            Span::styled("║     ", glyph_frame),
            Span::styled("╔═╗ N E B U L O U S ╔═╗", glyph_center),
            Span::styled("      ║", glyph_frame),
            Span::styled("  * ))▓▒░", dust_inner),
            Span::styled(" ✦·", dust_outer),
        ]),
        Line::from(vec![
            Span::styled("* .·", dust_outer),
            Span::styled("░▒▓( ✦  ", dust_inner),
            Span::styled("╚══════════ ✦ ", glyph_frame),
            Span::styled("NEBULOUS", glyph_center),
            Span::styled(" ✦ ══════════╝", glyph_frame),
            Span::styled("  ✦ )▓▒░", dust_inner),
            Span::styled("·. *", dust_outer),
        ]),
    ];

    NebulaBanner {
        tier: BannerTier::Compact,
        cols: COMPACT_BANNER_COLS,
        height: COMPACT_BANNER_LINES,
        lines,
    }
}
