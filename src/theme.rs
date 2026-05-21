use ratatui::style::Color;

use crate::model::NodeShape;

/// Per-shape visual treatment for a rendered node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeTheme {
    pub border: Color,
    pub fill: Color,
    pub text: Color,
    pub icon: Color,
}

/// Curated rendering palette for Diaview.
///
/// This is intentionally static for now: no config files, no theme selector,
/// and no terminal theme probing. The default uses Ayu Dark, adapted from
/// https://github.com/postrednik/opencode-ayu-theme and the upstream Ayu palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub background: Color,
    pub text: Color,
    pub muted: Color,
    pub edge: Color,
    pub edge_label: Color,
    pub arrowhead: Color,
    pub shadow: Color,
    pub rectangle: NodeTheme,
    pub rounded_rect: NodeTheme,
    pub diamond: NodeTheme,
    pub circle: NodeTheme,
}

impl Theme {
    /// Ayu Dark terminal palette for diagrams.
    pub const fn default_dark() -> Self {
        Self {
            // Ayu base UI colors.
            background: Color::Rgb(13, 16, 23),   // ayuBg #0D1017
            text: Color::Rgb(191, 189, 182),      // ayuFg #BFBDB6
            muted: Color::Rgb(98, 109, 122),      // ayuComment #626d7a
            edge: Color::Rgb(108, 115, 128),      // ayuGutter #6C7380
            edge_label: Color::Rgb(230, 180, 80), // ayuAccent #E6B450
            arrowhead: Color::Rgb(230, 180, 80),  // ayuAccent #E6B450
            shadow: Color::Rgb(8, 10, 14),

            // Shape colors map to the strongest semantic colors in Ayu:
            // blue/entity, cyan/tag, yellow/accent, green/string.
            rectangle: NodeTheme {
                border: Color::Rgb(122, 209, 255), // lighter Ayu blue
                fill: Color::Rgb(16, 20, 28),      // ayuEditorBg #10141C
                text: Color::Rgb(191, 189, 182),   // ayuFg #BFBDB6
                icon: Color::Rgb(122, 209, 255),
            },
            rounded_rect: NodeTheme {
                border: Color::Rgb(57, 186, 230), // ayuTag #39BAE6
                fill: Color::Rgb(20, 24, 33),     // ayuPanelBg #141821
                text: Color::Rgb(191, 189, 182),
                icon: Color::Rgb(57, 186, 230),
            },
            diamond: NodeTheme {
                border: Color::Rgb(255, 205, 102), // lighter Ayu yellow
                fill: Color::Rgb(32, 29, 20),      // warm dark accent surface
                text: Color::Rgb(230, 192, 138),   // ayuSpecial #E6C08A
                icon: Color::Rgb(255, 205, 102),
            },
            circle: NodeTheme {
                border: Color::Rgb(193, 235, 104), // lighter Ayu green
                fill: Color::Rgb(22, 31, 21),      // dark green surface
                text: Color::Rgb(191, 189, 182),
                icon: Color::Rgb(193, 235, 104),
            },
        }
    }

    pub const fn node(self, shape: &NodeShape) -> NodeTheme {
        match shape {
            NodeShape::Rectangle => self.rectangle,
            NodeShape::RoundedRect => self.rounded_rect,
            NodeShape::Diamond => self.diamond,
            NodeShape::Circle => self.circle,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_dark()
    }
}
