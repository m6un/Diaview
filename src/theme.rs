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
/// and no terminal theme probing. The default uses a Srcery-inspired dark
/// terminal palette tuned for modern terminals with 24-bit color support.
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
    /// Srcery-inspired dark terminal palette for diagrams.
    pub const fn default_dark() -> Self {
        Self {
            // Palette adapted from terminalcolors.com Srcery Default.
            background: Color::Rgb(28, 27, 25),    // background #1c1b19
            text: Color::Rgb(252, 232, 195),       // foreground #fce8c3
            muted: Color::Rgb(145, 129, 117),      // bright black #918175
            edge: Color::Rgb(186, 166, 127),       // white/light grey #baa67f
            edge_label: Color::Rgb(254, 208, 110), // bright yellow #fed06e
            arrowhead: Color::Rgb(186, 166, 127),  // white/light grey #baa67f
            shadow: Color::Rgb(15, 14, 13),
            rectangle: NodeTheme {
                border: Color::Rgb(104, 168, 228), // bright blue #68a8e4
                fill: Color::Rgb(47, 47, 43),
                text: Color::Rgb(252, 232, 195),   // foreground #fce8c3
                icon: Color::Rgb(104, 168, 228),   // bright blue #68a8e4
            },
            rounded_rect: NodeTheme {
                border: Color::Rgb(152, 188, 55),  // bright green #98bc37
                fill: Color::Rgb(43, 53, 38),
                text: Color::Rgb(252, 232, 195),   // foreground #fce8c3
                icon: Color::Rgb(152, 188, 55),    // bright green #98bc37
            },
            diamond: NodeTheme {
                border: Color::Rgb(247, 83, 65),   // bright red #f75341
                fill: Color::Rgb(58, 38, 32),
                text: Color::Rgb(252, 232, 195),   // foreground #fce8c3
                icon: Color::Rgb(247, 83, 65),     // bright red #f75341
            },
            circle: NodeTheme {
                border: Color::Rgb(255, 92, 143),  // bright magenta #ff5c8f
                fill: Color::Rgb(54, 39, 53),
                text: Color::Rgb(252, 232, 195),   // foreground #fce8c3
                icon: Color::Rgb(255, 92, 143),    // bright magenta #ff5c8f
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
