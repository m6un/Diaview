use ratatui::style::Color;

use crate::model::NodeShape;

const BACKGROUND: Color = Color::Rgb(17, 18, 16);
const SURFACE: Color = Color::Rgb(22, 23, 21);
const RAISED_SURFACE: Color = Color::Rgb(26, 27, 25);
const TEXT: Color = Color::Rgb(185, 183, 177);
const MUTED: Color = Color::Rgb(86, 86, 81);
const EDGE: Color = Color::Rgb(92, 92, 87);
const ACCENT_BLUE: Color = Color::Rgb(41, 148, 255);
const ACCENT_CORAL: Color = Color::Rgb(239, 111, 121);
const SHADOW: Color = Color::Rgb(8, 9, 8);

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
    pub accent_primary: Color,
    pub accent_secondary: Color,
    pub rectangle: NodeTheme,
    pub rounded_rect: NodeTheme,
    pub diamond: NodeTheme,
    pub circle: NodeTheme,
    pub database: NodeTheme,
}

impl Theme {
    pub const fn default_dark() -> Self {
        Self {
            background: BACKGROUND,
            text: TEXT,
            muted: MUTED,
            edge: EDGE,
            edge_label: TEXT,
            arrowhead: EDGE,
            shadow: SHADOW,
            accent_primary: ACCENT_BLUE,
            accent_secondary: ACCENT_CORAL,

            rectangle: NodeTheme {
                border: ACCENT_BLUE,
                fill: SURFACE,
                text: TEXT,
                icon: ACCENT_BLUE,
            },
            rounded_rect: NodeTheme {
                border: ACCENT_BLUE,
                fill: RAISED_SURFACE,
                text: TEXT,
                icon: ACCENT_BLUE,
            },
            diamond: NodeTheme {
                border: ACCENT_BLUE,
                fill: RAISED_SURFACE,
                text: TEXT,
                icon: ACCENT_BLUE,
            },
            circle: NodeTheme {
                border: ACCENT_BLUE,
                fill: SURFACE,
                text: TEXT,
                icon: ACCENT_BLUE,
            },
            database: NodeTheme {
                border: ACCENT_BLUE,
                fill: RAISED_SURFACE,
                text: TEXT,
                icon: ACCENT_BLUE,
            },
        }
    }

    pub const fn node(self, shape: &NodeShape) -> NodeTheme {
        match shape {
            NodeShape::Rectangle => self.rectangle,
            NodeShape::RoundedRect => self.rounded_rect,
            NodeShape::Diamond => self.diamond,
            NodeShape::Circle => self.circle,
            NodeShape::Database => self.database,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_keeps_node_chrome_to_one_primary_accent() {
        let theme = Theme::default_dark();
        for node in [
            theme.rectangle,
            theme.rounded_rect,
            theme.diamond,
            theme.circle,
            theme.database,
        ] {
            assert_eq!(node.border, theme.accent_primary);
            assert_eq!(node.icon, theme.accent_primary);
            assert_eq!(node.text, theme.text);
        }
    }

    #[test]
    fn default_theme_uses_neutral_edge_labels_and_arrowheads() {
        let theme = Theme::default_dark();
        assert_eq!(theme.edge_label, theme.text);
        assert_eq!(theme.arrowhead, theme.edge);
    }
}
