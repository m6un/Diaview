#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    TopDown,
    LeftRight,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeShape {
    Rectangle,
    RoundedRect,
    Diamond,
    Circle,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EdgeStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arrowhead {
    Normal,
    Open,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
    pub style: EdgeStyle,
    pub arrowhead: Arrowhead,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    pub direction: Direction,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
