use diaview::model::{Direction, Edge, EdgeStyle, Arrowhead, Graph, Node, NodeShape};

fn main() {
    let graph = Graph {
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
    };

    println!("Diaview");
    println!("{graph:#?}");
}
