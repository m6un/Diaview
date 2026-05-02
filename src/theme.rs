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
/// This is intentionally static: no config files and no terminal theme probing.
/// The default is a Catppuccin Mocha-inspired dark palette tuned for modern
/// terminals with 24-bit color support.
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
    /// A high-contrast-but-soft dark theme inspired by Catppuccin Mocha.
    pub const fn default_dark() -> Self {
        Self {
            background: Color::Rgb(30, 30, 46), // base
            text: Color::Rgb(205, 214, 244),    // text
            muted: Color::Rgb(127, 132, 156),   // overlay1
            edge: Color::Rgb(166, 153, 132),    // muted warm taupe
            edge_label: Color::Rgb(205, 214, 244),
            arrowhead: Color::Rgb(166, 153, 132),
            shadow: Color::Rgb(28, 28, 34),
            rectangle: NodeTheme {
                border: Color::Rgb(137, 180, 250), // blue
                fill: Color::Rgb(36, 39, 58),      // mantle/surface blend
                text: Color::Rgb(205, 214, 244),
                icon: Color::Rgb(137, 180, 250),
            },
            rounded_rect: NodeTheme {
                border: Color::Rgb(137, 220, 235), // sky
                fill: Color::Rgb(35, 43, 56),
                text: Color::Rgb(205, 214, 244),
                icon: Color::Rgb(137, 220, 235),
            },
            diamond: NodeTheme {
                border: Color::Rgb(249, 226, 175), // yellow
                fill: Color::Rgb(48, 43, 38),
                text: Color::Rgb(250, 244, 210),
                icon: Color::Rgb(249, 226, 175),
            },
            circle: NodeTheme {
                border: Color::Rgb(166, 227, 161), // green
                fill: Color::Rgb(35, 48, 43),
                text: Color::Rgb(226, 246, 218),
                icon: Color::Rgb(166, 227, 161),
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
