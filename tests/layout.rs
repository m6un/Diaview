use std::collections::HashSet;

use diaview::layout::layout;
use diaview::model::*;
use diaview::testdata::fixtures;

fn node(id: &str, label: &str, shape: NodeShape) -> Node {
    Node {
        id: id.into(),
        label: label.into(),
        shape,
        x: None,
        y: None,
        width: None,
        height: None,
    }
}

fn edge(source: &str, target: &str) -> Edge {
    styled_edge(source, target, None, EdgeStyle::Solid)
}

fn styled_edge(source: &str, target: &str, label: Option<&str>, style: EdgeStyle) -> Edge {
    Edge {
        source: source.into(),
        target: target.into(),
        label: label.map(str::to_owned),
        style,
        arrowhead: Arrowhead::Normal,
        route: None,
    }
}

fn assert_all_positioned(graph: &Graph) {
    for node in &graph.nodes {
        assert!(node.x.is_some(), "node {} missing x", node.id);
        assert!(node.y.is_some(), "node {} missing y", node.id);
        assert!(node.width.is_some(), "node {} missing width", node.id);
        assert!(node.height.is_some(), "node {} missing height", node.id);
    }
}

fn assert_no_overlaps(graph: &Graph) {
    for i in 0..graph.nodes.len() {
        for j in (i + 1)..graph.nodes.len() {
            let a = &graph.nodes[i];
            let b = &graph.nodes[j];
            let ax = a.x.unwrap();
            let ay = a.y.unwrap();
            let aw = a.width.unwrap();
            let ah = a.height.unwrap();
            let bx = b.x.unwrap();
            let by = b.y.unwrap();
            let bw = b.width.unwrap();
            let bh = b.height.unwrap();

            let overlap_x = ax < bx + bw && bx < ax + aw;
            let overlap_y = ay < by + bh && by < ay + ah;
            assert!(
                !(overlap_x && overlap_y),
                "nodes {} and {} overlap: A({},{} {}x{}) B({},{} {}x{})",
                a.id,
                b.id,
                ax,
                ay,
                aw,
                ah,
                bx,
                by,
                bw,
                bh
            );
        }
    }
}

fn find_node<'a>(graph: &'a Graph, id: &str) -> &'a Node {
    graph.nodes.iter().find(|n| n.id == id).unwrap()
}

#[test]
fn test_simple_two_node_positions() {
    let mut g = fixtures::simple_two_node();
    layout(&mut g);
    assert_all_positioned(&g);
    assert_no_overlaps(&g);
}

#[test]
fn test_topdown_later_layers_have_larger_y() {
    let mut g = fixtures::simple_two_node();
    layout(&mut g);
    assert!(find_node(&g, "B").y.unwrap() > find_node(&g, "A").y.unwrap());
}

#[test]
fn test_leftright_later_layers_have_larger_x() {
    let mut g = fixtures::left_right_chain();
    layout(&mut g);
    assert!(find_node(&g, "B").x.unwrap() > find_node(&g, "A").x.unwrap());
    assert!(find_node(&g, "C").x.unwrap() > find_node(&g, "B").x.unwrap());
}

#[test]
fn test_diamond_decision_no_overlap() {
    let mut g = fixtures::diamond_decision();
    layout(&mut g);
    assert_all_positioned(&g);
    assert_no_overlaps(&g);
}

#[test]
fn test_subgraph_group_bounds_cover_members() {
    let mut g = Graph {
        direction: Direction::TopDown,
        nodes: vec![
            node("A", "Gateway", NodeShape::Rectangle),
            node("B", "Service", NodeShape::Rectangle),
            node("C", "Outside", NodeShape::Rectangle),
        ],
        edges: vec![edge("A", "B")],
        groups: vec![Group {
            id: "API".into(),
            label: "API Layer".into(),
            node_ids: vec!["A".into(), "B".into()],
            parent: None,
            x: None,
            y: None,
            width: None,
            height: None,
        }],
    };
    layout(&mut g);

    let group = &g.groups[0];
    let (gx, gy, gw, gh) = (
        group.x.unwrap(),
        group.y.unwrap(),
        group.width.unwrap(),
        group.height.unwrap(),
    );
    for node_id in ["A", "B"] {
        let node = find_node(&g, node_id);
        assert!(node.x.unwrap() >= gx);
        assert!(node.y.unwrap() >= gy);
        assert!(node.x.unwrap() + node.width.unwrap() <= gx + gw);
        assert!(node.y.unwrap() + node.height.unwrap() <= gy + gh);
    }
}

#[test]
fn test_diamond_children_side_by_side() {
    let mut g = fixtures::diamond_decision();
    layout(&mut g);
    let c = find_node(&g, "C");
    let d = find_node(&g, "D");
    assert!((c.y.unwrap() - d.y.unwrap()).abs() < 0.01);
    assert!((c.x.unwrap() - d.x.unwrap()).abs() > 1.0);
}

#[test]
fn test_diamond_children_well_separated() {
    let mut g = fixtures::diamond_decision();
    layout(&mut g);
    let separation = (find_node(&g, "C").x.unwrap() - find_node(&g, "D").x.unwrap()).abs();
    assert!(separation >= 10.0, "got {separation}");
}

#[test]
fn test_single_node_graph() {
    let mut g = Graph {
        direction: Direction::TopDown,
        nodes: vec![node("X", "Solo", NodeShape::Rectangle)],
        edges: vec![],
        groups: vec![],
    };
    layout(&mut g);
    assert_all_positioned(&g);
    let x = find_node(&g, "X");
    assert!(x.width.unwrap() >= "Solo".len() as f64);
    assert!(x.height.unwrap() >= 1.0);
}

#[test]
fn test_disconnected_nodes() {
    let mut g = Graph {
        direction: Direction::TopDown,
        nodes: vec![
            node("A", "Island 1", NodeShape::Rectangle),
            node("B", "Island 2", NodeShape::Circle),
        ],
        edges: vec![],
        groups: vec![],
    };
    layout(&mut g);
    assert_all_positioned(&g);
    assert_no_overlaps(&g);
}

#[test]
fn test_left_right_chain_no_overlap() {
    let mut g = fixtures::left_right_chain();
    layout(&mut g);
    assert_all_positioned(&g);
    assert_no_overlaps(&g);
}

#[test]
fn test_node_sizing_diamond_larger() {
    let mut g = fixtures::diamond_decision();
    layout(&mut g);
    assert!(find_node(&g, "B").width.unwrap() > find_node(&g, "A").width.unwrap());
}

#[test]
fn test_min_size_enforced_by_short_label_behavior() {
    let mut short = Graph {
        direction: Direction::TopDown,
        nodes: vec![node("T", "X", NodeShape::Rectangle)],
        edges: vec![],
        groups: vec![],
    };
    let mut longer = Graph {
        direction: Direction::TopDown,
        nodes: vec![node("L", "Longer label than X", NodeShape::Rectangle)],
        edges: vec![],
        groups: vec![],
    };
    layout(&mut short);
    layout(&mut longer);
    let t = find_node(&short, "T");
    let l = find_node(&longer, "L");
    assert!(t.width.unwrap() >= "X".len() as f64);
    assert!(t.height.unwrap() >= 1.0);
    assert!(l.width.unwrap() > t.width.unwrap());
}

#[test]
fn test_layout_is_deterministic_for_same_input() {
    let mut first = fixtures::diamond_decision();
    let mut second = fixtures::diamond_decision();
    layout(&mut first);
    layout(&mut second);
    assert_eq!(first, second);
}

#[test]
fn test_empty_graph() {
    let mut g = Graph {
        direction: Direction::TopDown,
        nodes: vec![],
        edges: vec![],
        groups: vec![],
    };
    layout(&mut g);
}

#[test]
fn test_routed_edges_have_points() {
    let mut g = fixtures::diamond_decision();
    layout(&mut g);
    for edge in &g.edges {
        assert!(edge.route.as_ref().unwrap().points.len() >= 2);
    }
}

#[test]
fn test_fan_out_ports_are_distinct() {
    let mut g = fixtures::diamond_decision();
    layout(&mut g);
    let ports: Vec<_> = g
        .edges
        .iter()
        .filter(|edge| edge.source == "B")
        .map(|edge| edge.route.as_ref().unwrap().source_port.clone())
        .collect();
    assert_eq!(ports.len(), 2);
    assert_ne!(ports[0], ports[1]);
}

#[test]
fn test_fan_in_ports_are_distinct() {
    let mut g = Graph {
        direction: Direction::TopDown,
        nodes: vec![
            node("A", "A", NodeShape::Rectangle),
            node("B", "B", NodeShape::Rectangle),
            node("C", "Sink", NodeShape::Rectangle),
        ],
        edges: vec![edge("A", "C"), edge("B", "C")],
        groups: vec![],
    };
    layout(&mut g);
    let ports: Vec<_> = g
        .edges
        .iter()
        .map(|edge| edge.route.as_ref().unwrap().target_port.clone())
        .collect();
    assert_eq!(ports.len(), 2);
    assert_ne!(ports[0], ports[1]);
}

#[test]
fn phase15_shared_sink_incoming_edges_share_trunk_coordinate() {
    let mut graph = Graph {
        direction: Direction::TopDown,
        nodes: vec![
            node("A", "Producer A", NodeShape::Rectangle),
            node("B", "Producer B", NodeShape::Rectangle),
            node("C", "Producer C", NodeShape::Rectangle),
            node("D", "Producer D", NodeShape::Rectangle),
            node("SUCCESS", "Unified success sink", NodeShape::Circle),
        ],
        edges: vec![
            edge("A", "SUCCESS"),
            edge("B", "SUCCESS"),
            edge("C", "SUCCESS"),
            edge("D", "SUCCESS"),
        ],
        groups: vec![],
    };
    layout(&mut graph);
    let incoming: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.target == "SUCCESS")
        .map(|edge| edge.route.as_ref().unwrap())
        .collect();
    assert!(incoming.len() >= 4);
    let trunk_y = incoming[0].points[1].y;
    let lane_id = incoming[0].lane_id;
    for route in incoming {
        assert_eq!(route.lane_id, lane_id);
        assert!((route.points[1].y - trunk_y).abs() < 0.01);
        assert!((route.points[2].y - trunk_y).abs() < 0.01);
    }
}

#[test]
fn phase15_non_bundled_fan_out_edges_get_multiple_lane_ids() {
    let mut graph = Graph {
        direction: Direction::LeftRight,
        nodes: vec![
            node("ROUTER", "Route request", NodeShape::Diamond),
            node("API", "Public API", NodeShape::Rectangle),
            node("ADMIN", "Admin API", NodeShape::Rectangle),
            node("WEBHOOKS", "Webhook worker", NodeShape::Rectangle),
            node("EXPORTS", "Export worker", NodeShape::Rectangle),
        ],
        edges: vec![
            edge("ROUTER", "API"),
            edge("ROUTER", "ADMIN"),
            edge("ROUTER", "WEBHOOKS"),
            edge("ROUTER", "EXPORTS"),
        ],
        groups: vec![],
    };
    layout(&mut graph);
    let lane_ids: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.source == "ROUTER")
        .map(|edge| edge.route.as_ref().and_then(|route| route.lane_id).unwrap())
        .collect();
    let unique_lane_ids: HashSet<_> = lane_ids.iter().copied().collect();
    assert!(lane_ids.len() >= 4);
    assert!(unique_lane_ids.len() > 1);
}

#[test]
fn login_is_not_misclassified_as_telemetry() {
    let mut graph = Graph {
        direction: Direction::TopDown,
        nodes: vec![
            node("AUTH", "Auth", NodeShape::Rectangle),
            node("LOGIN", "Login", NodeShape::Rectangle),
        ],
        edges: vec![edge("AUTH", "LOGIN")],
        groups: vec![],
    };
    layout(&mut graph);
    let login_edge = graph
        .edges
        .iter()
        .find(|edge| edge.source == "AUTH" && edge.target == "LOGIN")
        .unwrap();
    assert_ne!(
        login_edge.route.as_ref().map(|route| &route.class),
        Some(&EdgeClass::Telemetry)
    );
}

#[test]
fn single_telemetry_edge_uses_local_route_instead_of_perimeter_wall() {
    let mut graph = Graph {
        direction: Direction::TopDown,
        nodes: vec![
            node("WORKER", "Worker", NodeShape::Rectangle),
            node("METRICS", "Metrics", NodeShape::Rectangle),
        ],
        edges: vec![styled_edge("WORKER", "METRICS", None, EdgeStyle::Dotted)],
        groups: vec![],
    };
    layout(&mut graph);
    let max_node_right = graph
        .nodes
        .iter()
        .filter(|node| !node.id.starts_with("__dummy"))
        .map(|node| node.x.unwrap() + node.width.unwrap())
        .fold(0.0_f64, f64::max);
    let route = graph.edges[0].route.as_ref().unwrap();
    assert_eq!(route.class, EdgeClass::Telemetry);
    assert!(route.points.iter().all(|point| point.x <= max_node_right));
}

fn telemetry_overlay_graph() -> Graph {
    Graph {
        direction: Direction::LeftRight,
        nodes: vec![
            node("EDGE", "Edge", NodeShape::Rectangle),
            node("API", "API", NodeShape::Rectangle),
            node("WORKER", "Worker", NodeShape::Rectangle),
            node("DB", "Database", NodeShape::Rectangle),
            node("LOGS", "Logs", NodeShape::Rectangle),
            node("METRICS", "Metrics", NodeShape::Rectangle),
            node("ALERTS", "Alerts", NodeShape::Rectangle),
        ],
        edges: vec![
            edge("EDGE", "API"),
            edge("API", "WORKER"),
            edge("WORKER", "DB"),
            styled_edge("EDGE", "LOGS", None, EdgeStyle::Dotted),
            styled_edge("API", "LOGS", None, EdgeStyle::Dotted),
            styled_edge("WORKER", "LOGS", None, EdgeStyle::Dotted),
            styled_edge("DB", "LOGS", None, EdgeStyle::Dotted),
            styled_edge("EDGE", "METRICS", None, EdgeStyle::Dotted),
            styled_edge("API", "METRICS", None, EdgeStyle::Dotted),
            styled_edge("WORKER", "METRICS", None, EdgeStyle::Dotted),
            styled_edge("DB", "METRICS", None, EdgeStyle::Dotted),
            edge("METRICS", "ALERTS"),
        ],
        groups: vec![],
    }
}

#[test]
fn phase15_telemetry_overlay_edges_are_classified_telemetry() {
    let mut graph = telemetry_overlay_graph();
    layout(&mut graph);
    let telemetry_edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| {
            edge.route
                .as_ref()
                .is_some_and(|route| route.class == EdgeClass::Telemetry)
        })
        .collect();
    assert!(telemetry_edges.len() >= 4);
    let metrics_to_alerts = graph
        .edges
        .iter()
        .find(|edge| edge.source == "METRICS" && edge.target == "ALERTS")
        .unwrap();
    assert_eq!(
        metrics_to_alerts.route.as_ref().map(|route| &route.class),
        Some(&EdgeClass::Telemetry)
    );
}

#[test]
fn phase15_lr_telemetry_uses_lower_perimeter_route_when_not_bundled() {
    let mut graph = telemetry_overlay_graph();
    layout(&mut graph);
    let max_node_bottom = graph
        .nodes
        .iter()
        .filter(|node| !node.id.starts_with("__dummy"))
        .map(|node| node.y.unwrap() + node.height.unwrap())
        .fold(0.0_f64, f64::max);
    let route = graph
        .edges
        .iter()
        .find(|edge| edge.source == "METRICS" && edge.target == "ALERTS")
        .unwrap()
        .route
        .as_ref()
        .unwrap();
    assert_eq!(route.class, EdgeClass::Telemetry);
    assert!(route.points.iter().any(|point| point.y > max_node_bottom));
}

#[test]
fn error_and_back_edge_classification_precedes_telemetry() {
    let mut graph = Graph {
        direction: Direction::LeftRight,
        nodes: vec![
            node("API", "API", NodeShape::Rectangle),
            node("LOGS", "Logs", NodeShape::Rectangle),
        ],
        edges: vec![
            styled_edge("API", "LOGS", Some("error telemetry"), EdgeStyle::Dotted),
            styled_edge("LOGS", "API", Some("metric retry"), EdgeStyle::Dotted),
        ],
        groups: vec![],
    };
    layout(&mut graph);
    let api_to_logs = graph
        .edges
        .iter()
        .find(|edge| edge.source == "API")
        .unwrap();
    let logs_to_api = graph
        .edges
        .iter()
        .find(|edge| edge.source == "LOGS")
        .unwrap();
    assert_eq!(api_to_logs.route.as_ref().unwrap().class, EdgeClass::Error);
    assert_eq!(
        logs_to_api.route.as_ref().unwrap().class,
        EdgeClass::BackEdge
    );
}

fn cyclic_signal_graph(direction: Direction) -> Graph {
    Graph {
        direction,
        nodes: vec![
            node("WEBHOOK", "Temporal webhook", NodeShape::Rectangle),
            node("WORKFLOW", "Workflow", NodeShape::Rectangle),
            node("ACTIVITY", "Activity", NodeShape::Rectangle),
            node("SIGNAL", "Signal loop", NodeShape::Rectangle),
        ],
        edges: vec![
            edge("WEBHOOK", "WORKFLOW"),
            edge("WORKFLOW", "ACTIVITY"),
            edge("ACTIVITY", "SIGNAL"),
            edge("SIGNAL", "WORKFLOW"),
            edge("SIGNAL", "WEBHOOK"),
        ],
        groups: vec![],
    }
}

#[test]
fn topdown_back_edges_use_right_outer_perimeter_lanes() {
    let mut graph = cyclic_signal_graph(Direction::TopDown);
    layout(&mut graph);
    let max_node_right = graph
        .nodes
        .iter()
        .filter(|node| !node.id.starts_with("__dummy"))
        .map(|node| node.x.unwrap() + node.width.unwrap())
        .fold(0.0_f64, f64::max);
    let back_routes: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.source == "SIGNAL")
        .map(|edge| edge.route.as_ref().unwrap())
        .collect();
    assert_eq!(back_routes.len(), 2);
    for route in &back_routes {
        assert_eq!(route.class, EdgeClass::BackEdge);
        assert_eq!(route.source_port.side, PortSide::Right);
        assert_eq!(route.target_port.side, PortSide::Right);
        assert!(route.points.iter().any(|point| point.x > max_node_right));
    }
    let lane_ids: HashSet<_> = back_routes
        .iter()
        .map(|route| route.lane_id.unwrap())
        .collect();
    let lane_xs: HashSet<_> = back_routes
        .iter()
        .map(|route| {
            route
                .points
                .iter()
                .map(|point| point.x)
                .fold(0.0_f64, f64::max)
                .round() as i64
        })
        .collect();
    assert_eq!(lane_ids.len(), 2);
    assert_eq!(lane_xs.len(), 2);
}

#[test]
fn leftright_back_edges_use_bottom_outer_perimeter_lanes() {
    let mut graph = cyclic_signal_graph(Direction::LeftRight);
    layout(&mut graph);
    let max_node_bottom = graph
        .nodes
        .iter()
        .filter(|node| !node.id.starts_with("__dummy"))
        .map(|node| node.y.unwrap() + node.height.unwrap())
        .fold(0.0_f64, f64::max);
    let back_routes: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.source == "SIGNAL")
        .map(|edge| edge.route.as_ref().unwrap())
        .collect();
    assert_eq!(back_routes.len(), 2);
    for route in &back_routes {
        assert_eq!(route.class, EdgeClass::BackEdge);
        assert_eq!(route.source_port.side, PortSide::Bottom);
        assert_eq!(route.target_port.side, PortSide::Bottom);
        assert!(route.points.iter().any(|point| point.y > max_node_bottom));
    }
    let lane_ids: HashSet<_> = back_routes
        .iter()
        .map(|route| route.lane_id.unwrap())
        .collect();
    let lane_ys: HashSet<_> = back_routes
        .iter()
        .map(|route| {
            route
                .points
                .iter()
                .map(|point| point.y)
                .fold(0.0_f64, f64::max)
                .round() as i64
        })
        .collect();
    assert_eq!(lane_ids.len(), 2);
    assert_eq!(lane_ys.len(), 2);
}
