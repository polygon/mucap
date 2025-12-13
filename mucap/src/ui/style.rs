//! Style module for UI theming and visual customization.
//!
//! This module provides color definitions and styling utilities for the NoteView UI.
//! Colors are organized by purpose (background, bars, selections, notes, playhead, cursor)
//! making it easy to customize the visual appearance.
//!
//! # Modding
//!
//! To create custom themes, you can:
//! - Extend this module with additional theme factory methods
//! - Load theme configurations from files at runtime
//! - Add theme switching functionality to the UI
//! - Implement theme presets (dark, high contrast, colorblind-friendly, etc.)

use nih_plug_vizia::vizia::vg;

/// Complete color palette for the NoteView interface.
///
/// Contains all color values used throughout the note editor UI, organized by visual element.
/// Colors use the vizia vg::Color format which supports both RGB and RGBA values.
///
/// # Examples
///
/// ```ignore
/// let colors = StyleColors::dark();    // Dark theme with warm orange tones
/// let colors = StyleColors::bright();  // Bright theme with cool blue tones
/// let colors = StyleColors::default(); // Default theme (currently dark)
/// // Use colors.bg_dark, colors.note_selected_bright, etc.
/// ```
pub struct StyleColors {
    // Background gradient
    pub bg_dark: vg::Color,
    pub bg_light: vg::Color,

    // Bar indicators
    pub bar_glow_bright: vg::Color,
    pub bar_glow_dim: vg::Color,

    // Selection overlay gradient
    pub selection_bright: vg::Color,
    pub selection_mid_dim: vg::Color,
    pub selection_mid_bright: vg::Color,

    // Note colors
    pub note_unselected: vg::Color,
    pub note_selected_bright: vg::Color,
    pub note_rim: vg::Color,

    // Playhead position bar
    pub playhead_transparent: vg::Color,
    pub playhead_semi: vg::Color,
    pub playhead_opaque: vg::Color,
    pub playhead_base: vg::Color,

    // Cursor
    pub cursor: vg::Color,
}

impl StyleColors {
    /// Creates a dark theme with warm orange/amber tones for notes and cool blue/purple
    /// tones for the interface background.
    ///
    /// This provides good visual contrast and is friendly for extended editing sessions.
    ///
    /// # Notes for Modders
    ///
    /// To add more themes, create similar factory methods like:
    /// - `bright()` - A light theme for daytime use
    /// - `high_contrast()` - For accessibility needs
    /// - `colorblind_friendly()` - For specific color vision deficiencies
    /// - `custom()` or `from_config()` - For user-defined themes
    ///
    /// When choosing colors, ensure:
    /// - Sufficient contrast ratios for accessibility
    /// - Colorblind-friendly palettes when possible
    pub fn dark() -> Self {
        Self {
            bg_dark: vg::Color::rgb(0, 0, 16),
            bg_light: vg::Color::rgb(16, 16, 42),
            bar_glow_bright: vg::Color::rgba(128, 64, 12, 255),
            bar_glow_dim: vg::Color::rgba(128, 64, 12, 0),
            selection_bright: vg::Color::rgba(64, 255, 16, 60),
            selection_mid_dim: vg::Color::rgba(92, 92, 24, 40),
            selection_mid_bright: vg::Color::rgba(92, 192, 24, 40),
            note_unselected: vg::Color::rgb(220, 120, 12),
            note_selected_bright: vg::Color::rgb(120, 220, 12),
            note_rim: vg::Color::rgb(232, 232, 232),
            playhead_transparent: vg::Color::rgba(92, 92, 128, 0),
            playhead_semi: vg::Color::rgba(92, 92, 128, 64),
            playhead_opaque: vg::Color::rgba(92, 92, 128, 128),
            playhead_base: vg::Color::rgba(92, 92, 128, 255),
            cursor: vg::Color::rgba(156, 156, 156, 172),
        }
    }

    /// Creates a bright theme with clean white background and soft pastel accents.
    ///
    /// Perfect for daytime use and well-lit environments. Features:
    /// - Smooth white gradient background
    /// - Pastel teal notes with pastel coral highlights for gentle contrast
    /// - Soft pastel amber bar indicators that glow gently
    /// - Pastel green selection overlays for excellent visibility
    /// - Bright pastel peach playhead for visual variety
    /// - Soft, approachable, non-fatiguing aesthetic
    ///
    /// The bright theme uses soft pastel colors that are easy on the eyes
    /// while maintaining excellent readability and color variety.
    pub fn bright() -> Self {
        Self {
            // Clean white gradient for a clean, professional look
            bg_dark: vg::Color::rgb(222, 222, 232),
            bg_light: vg::Color::rgb(252, 252, 254),

            // Soft pastel amber bar indicators, very gentle on the eyes
            bar_glow_bright: vg::Color::rgba(240, 190, 130, 200),
            bar_glow_dim: vg::Color::rgba(240, 190, 130, 0),

            // Soft pastel green selection overlay with smooth gradient
            selection_bright: vg::Color::rgba(160, 230, 200, 80),
            selection_mid_dim: vg::Color::rgba(190, 240, 220, 50),
            selection_mid_bright: vg::Color::rgba(170, 235, 210, 60),

            // Pastel teal notes with pastel coral highlights for gentle contrast
            note_unselected: vg::Color::rgb(140, 200, 220),
            note_selected_bright: vg::Color::rgb(240, 160, 140),
            note_rim: vg::Color::rgb(120, 120, 140),

            // Bright pastel peach playhead for excellent visibility and variety
            playhead_transparent: vg::Color::rgba(240, 160, 120, 0),
            playhead_semi: vg::Color::rgba(240, 160, 120, 100),
            playhead_opaque: vg::Color::rgba(240, 160, 120, 160),
            playhead_base: vg::Color::rgba(240, 160, 120, 240),

            // Soft pastel mauve cursor for visibility and variety
            cursor: vg::Color::rgba(180, 140, 160, 220),
        }
    }
}

impl Default for StyleColors {
    /// Returns the default theme, currently the dark theme.
    fn default() -> Self {
        Self::dark()
    }
}
