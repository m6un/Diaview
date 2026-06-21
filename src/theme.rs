use ratatui::style::Color;

use crate::model::NodeShape;

const AYU_BACKGROUND: Color = Color::Rgb(13, 16, 23);
const AYU_EDITOR_BACKGROUND: Color = Color::Rgb(16, 20, 28);
const AYU_PANEL_BACKGROUND: Color = Color::Rgb(20, 24, 33);
const AYU_TEXT: Color = Color::Rgb(191, 189, 182);
const AYU_COMMENT: Color = Color::Rgb(98, 109, 122);
const AYU_GUTTER: Color = Color::Rgb(108, 115, 128);
const AYU_ACCENT: Color = Color::Rgb(230, 180, 80);
const AYU_SPECIAL: Color = Color::Rgb(230, 192, 138);
const NODE_BLUE: Color = Color::Rgb(122, 209, 255);
const NODE_CYAN: Color = Color::Rgb(57, 186, 230);
const NODE_YELLOW: Color = Color::Rgb(255, 205, 102);
const NODE_GREEN: Color = Color::Rgb(193, 235, 104);
const WARM_SURFACE: Color = Color::Rgb(32, 29, 20);
const GREEN_SURFACE: Color = Color::Rgb(22, 31, 21);
const SHADOW: Color = Color::Rgb(8, 10, 14);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeTheme {
    pub border: Color,
    pub fill: Color,
    pub text: Color,
    pub icon: Color,
}

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
    pub const fn default_dark() -> Self {
        Self {
            background: AYU_BACKGROUND,
            text: AYU_TEXT,
            muted: AYU_COMMENT,
            edge: AYU_GUTTER,
            edge_label: AYU_ACCENT,
            arrowhead: AYU_ACCENT,
            shadow: SHADOW,

            rectangle: NodeTheme {
                border: NODE_BLUE,
                fill: AYU_EDITOR_BACKGROUND,
                text: AYU_TEXT,
                icon: NODE_BLUE,
            },
            rounded_rect: NodeTheme {
                border: NODE_CYAN,
                fill: AYU_PANEL_BACKGROUND,
                text: AYU_TEXT,
                icon: NODE_CYAN,
            },
            diamond: NodeTheme {
                border: NODE_YELLOW,
                fill: WARM_SURFACE,
                text: AYU_SPECIAL,
                icon: NODE_YELLOW,
            },
            circle: NodeTheme {
                border: NODE_GREEN,
                fill: GREEN_SURFACE,
                text: AYU_TEXT,
                icon: NODE_GREEN,
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
