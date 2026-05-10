#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
pub struct RoutePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PortSide {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub x: f64,
    pub y: f64,
    pub side: PortSide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeClass {
    Primary,
    Telemetry,
    Error,
    BackEdge,
    External,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoutePlan {
    pub points: Vec<RoutePoint>,
    pub source_port: Port,
    pub target_port: Port,
    pub lane_id: Option<usize>,
    pub class: EdgeClass,
    pub label_anchor: Option<RoutePoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub label: Option<String>,
    pub style: EdgeStyle,
    pub arrowhead: Arrowhead,
    pub route: Option<RoutePlan>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    pub direction: Direction,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
