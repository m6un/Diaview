/// Test helpers and sample Mermaid diagrams.
pub mod fixtures {
    use crate::model::*;

    /// A simple 2-node linear graph: A("Start") --> B("End")
    pub fn simple_two_node() -> Graph {
        Graph {
            direction: Direction::TopDown,
            nodes: vec![
                Node {
                    id: "A".into(),
                    label: "Start".into(),
                    shape: NodeShape::RoundedRect,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
                Node {
                    id: "B".into(),
                    label: "End".into(),
                    shape: NodeShape::RoundedRect,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
            ],
            edges: vec![Edge {
                source: "A".into(),
                target: "B".into(),
                label: Some("next".into()),
                style: EdgeStyle::Solid,
                arrowhead: Arrowhead::Normal,
            }],
        }
    }

    /// A diamond decision graph: A --> B{Decision} -->|yes| C, -->|no| D
    pub fn diamond_decision() -> Graph {
        Graph {
            direction: Direction::TopDown,
            nodes: vec![
                Node {
                    id: "A".into(),
                    label: "Start".into(),
                    shape: NodeShape::RoundedRect,
                    x: None, y: None, width: None, height: None,
                },
                Node {
                    id: "B".into(),
                    label: "Decision".into(),
                    shape: NodeShape::Diamond,
                    x: None, y: None, width: None, height: None,
                },
                Node {
                    id: "C".into(),
                    label: "Yes path".into(),
                    shape: NodeShape::Rectangle,
                    x: None, y: None, width: None, height: None,
                },
                Node {
                    id: "D".into(),
                    label: "No path".into(),
                    shape: NodeShape::Rectangle,
                    x: None, y: None, width: None, height: None,
                },
            ],
            edges: vec![
                Edge {
                    source: "A".into(),
                    target: "B".into(),
                    label: None,
                    style: EdgeStyle::Solid,
                    arrowhead: Arrowhead::Normal,
                },
                Edge {
                    source: "B".into(),
                    target: "C".into(),
                    label: Some("yes".into()),
                    style: EdgeStyle::Solid,
                    arrowhead: Arrowhead::Normal,
                },
                Edge {
                    source: "B".into(),
                    target: "D".into(),
                    label: Some("no".into()),
                    style: EdgeStyle::Dashed,
                    arrowhead: Arrowhead::Normal,
                },
            ],
        }
    }

    /// A more complex Mermaid flowchart fixture used for visual renderer dumps.
    pub fn complex_architecture_mermaid() -> &'static str {
"graph LR
    A[Square Rect] -- Link text --> B((Circle))
    A --> C(Round Rect)
    B --> D{Rhombus}
    C --> D
"
    }

    /// Left-right direction, 3 nodes in a chain
    pub fn left_right_chain() -> Graph {
        Graph {
            direction: Direction::LeftRight,
            nodes: vec![
                Node {
                    id: "A".into(),
                    label: "Input".into(),
                    shape: NodeShape::Rectangle,
                    x: None, y: None, width: None, height: None,
                },
                Node {
                    id: "B".into(),
                    label: "Process".into(),
                    shape: NodeShape::RoundedRect,
                    x: None, y: None, width: None, height: None,
                },
                Node {
                    id: "C".into(),
                    label: "Output".into(),
                    shape: NodeShape::Circle,
                    x: None, y: None, width: None, height: None,
                },
            ],
            edges: vec![
                Edge {
                    source: "A".into(),
                    target: "B".into(),
                    label: None,
                    style: EdgeStyle::Solid,
                    arrowhead: Arrowhead::Normal,
                },
                Edge {
                    source: "B".into(),
                    target: "C".into(),
                    label: Some("result".into()),
                    style: EdgeStyle::Dotted,
                    arrowhead: Arrowhead::Open,
                },
            ],
        }
    }
}
