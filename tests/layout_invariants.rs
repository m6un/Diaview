use std::collections::{HashMap, HashSet};

use diaview::layout::layout;
use diaview::model::*;
use diaview::parser::mermaid::parse;
use diaview::testdata::fixtures;

const EPS: f64 = 0.01;
const TEMPORAL_LAYOUT_REGRESSION: &str = include_str!("../fixtures/temporal_layout_regression.mmd");
const TEMPORAL_PAYMENT_FEEDBACK_REGRESSION: &str =
    include_str!("../fixtures/temporal_payment_feedback_regression.mmd");

type MermaidFixture = (&'static str, &'static str);

fn important_fixtures() -> Vec<MermaidFixture> {
    vec![
        ("simple", fixtures::simple_mermaid()),
        (
            "complex architecture",
            fixtures::complex_architecture_mermaid(),
        ),
        (
            "phase15 fan-in sink",
            fixtures::phase15_fan_in_sink_mermaid(),
        ),
        (
            "phase15 fan-out router",
            fixtures::phase15_fan_out_router_mermaid(),
        ),
        (
            "phase15 back-edge cycle",
            fixtures::phase15_back_edge_cycle_mermaid(),
        ),
        (
            "temporal stripe payment baseline",
            fixtures::temporal_stripe_payment_mermaid(),
        ),
        ("temporal layout regression", TEMPORAL_LAYOUT_REGRESSION),
        (
            "temporal payment feedback regression",
            TEMPORAL_PAYMENT_FEEDBACK_REGRESSION,
        ),
        (
            "phase15 telemetry overlay",
            fixtures::phase15_telemetry_overlay_mermaid(),
        ),
        (
            "phase15 grouped architecture",
            fixtures::phase15_grouped_architecture_mermaid(),
        ),
    ]
}

fn adversarial_fixtures() -> Vec<MermaidFixture> {
    vec![
        (
            "td cycle multiple returns",
            r#"
flowchart TD
    A[Start] --> B[Do work]
    B --> C{Need retry?}
    C --> D[Commit]
    D --> E((Done))
    C --> R[Retry]
    R --> B
    D --> B
    E --> A
"#,
        ),
        (
            "lr dense fan-in-out",
            r#"
flowchart LR
    A[Ingress] --> R{Router}
    B[Batch] --> R
    C[Cron] --> R
    R --> S1[Service one]
    R --> S2[Service two]
    R --> S3[Service three]
    R --> S4[Service four]
    S1 --> K((Sink))
    S2 --> K
    S3 --> K
    S4 --> K
"#,
        ),
        (
            "td long cross-layer edge",
            r#"
flowchart TD
    A[Source] --> B[Step one]
    B --> C[Step two]
    C --> D[Step three]
    D --> E[Step four]
    A -->|audit path that skips the whole pipeline| E
"#,
        ),
        (
            "lr long labels",
            r#"
flowchart LR
    A[Extremely long source label that forces a wide terminal card] -->|edge label long enough to need a safe anchor| B[Another very long target label for sizing]
    B --> C[Short]
"#,
        ),
        (
            "td disconnected components",
            r#"
flowchart TD
    A[One] --> B[Two]
    C[Island] --> D[Other island]
    E[Solo]
"#,
        ),
        (
            "td groups",
            r#"
flowchart TD
    subgraph API[API layer]
      A[Gateway] --> B[Handler]
    end
    subgraph Workers[Worker layer]
      C[Queue] --> D[Consumer]
    end
    B --> C
    D --> A
"#,
        ),
    ]
}

#[test]
fn exact_temporal_regression_fixture_parses() {
    let graph = parse(TEMPORAL_LAYOUT_REGRESSION).unwrap();
    for id in [
        "ENTRY", "CHECK", "INTENT", "WAIT", "RESULT", "PAID", "FAILED", "STRIPE", "HOOK", "VERIFY",
    ] {
        assert!(
            graph.nodes.iter().any(|node| node.id == id),
            "missing node {id}"
        );
    }
    assert!(graph.edges.len() >= 15);
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.label.as_deref() == Some("fail/timeout"))
    );
}

#[test]
fn basic_layout_invariants_hold_for_small_baselines() {
    for (name, mut graph) in [
        ("simple two node".to_string(), fixtures::simple_two_node()),
        ("diamond decision".to_string(), fixtures::diamond_decision()),
        ("left-right chain".to_string(), fixtures::left_right_chain()),
    ] {
        layout(&mut graph);
        assert_layout_invariants(&name, &graph).unwrap();
    }
}

#[test]
fn renderer_keeps_temporal_regression_labels_visible() {
    let mut graph = parse(TEMPORAL_LAYOUT_REGRESSION).unwrap();
    layout(&mut graph);
    let rendered = diaview::renderer::canvas::render_to_string(&graph).unwrap();
    for text in ["Client + API", "Verify sig", "fail/timeout"] {
        assert!(
            rendered.contains(text),
            "rendered output missing {text:?}\n{rendered}"
        );
    }
}

#[test]
fn validation_helpers_report_corrupted_layouts() {
    let mut graph = fixtures::simple_two_node();
    layout(&mut graph);
    graph.nodes[0].x = None;
    assert_failure(&graph, "invalid x");

    let mut graph = fixtures::simple_two_node();
    layout(&mut graph);
    graph.nodes[1].x = graph.nodes[0].x;
    graph.nodes[1].y = graph.nodes[0].y;
    assert_failure(&graph, "overlap");

    let mut graph = fixtures::simple_two_node();
    layout(&mut graph);
    graph.edges[0].target = "MISSING".into();
    assert_failure(&graph, "missing target node");

    let mut graph = fixtures::simple_two_node();
    layout(&mut graph);
    let route = graph.edges[0].route.as_mut().unwrap();
    route.points[0].x += 3.0;
    route.points[1].x += 1.0;
    assert_failure(&graph, "first point does not match port");
    assert_failure(&graph, "non-orthogonal");

    let mut graph = fixtures::simple_two_node();
    layout(&mut graph);
    graph.edges[0].label = Some("bad".into());
    graph.edges[0].route.as_mut().unwrap().label_anchor = None;
    assert_failure(&graph, "label has no anchor");

    let mut graph = fixtures::simple_two_node();
    layout(&mut graph);
    graph.edges[0].label = Some("bad".into());
    graph.edges[0].route.as_mut().unwrap().label_anchor = Some(RoutePoint {
        x: f64::NAN,
        y: 1.0,
    });
    assert_failure(&graph, "invalid label anchor");

    let mut graph = fixtures::simple_two_node();
    layout(&mut graph);
    graph.groups.push(Group {
        id: "G".into(),
        label: "bad group".into(),
        node_ids: vec!["A".into()],
        parent: None,
        x: Some(99.0),
        y: Some(99.0),
        width: Some(1.0),
        height: Some(1.0),
    });
    assert_failure(&graph, "does not cover member");

    let graph = Graph {
        direction: Direction::LeftRight,
        nodes: vec![
            laid_out_node("A", 0.0, 0.0, 2.0, 2.0),
            laid_out_node("B", 10.0, 0.0, 2.0, 2.0),
            laid_out_node("C", 5.0, 0.0, 2.0, 2.0),
        ],
        edges: vec![Edge {
            source: "A".into(),
            target: "B".into(),
            label: None,
            style: EdgeStyle::Solid,
            arrowhead: Arrowhead::Normal,
            route: Some(RoutePlan {
                points: vec![RoutePoint { x: 2.0, y: 1.0 }, RoutePoint { x: 9.0, y: 1.0 }],
                source_port: Port {
                    x: 2.0,
                    y: 1.0,
                    side: PortSide::Right,
                },
                target_port: Port {
                    x: 9.0,
                    y: 1.0,
                    side: PortSide::Left,
                },
                lane_id: None,
                class: EdgeClass::Primary,
                label_anchor: None,
            }),
        }],
        groups: vec![],
    };
    assert_failure(&graph, "raster-collides");
}

#[test]
fn stage_d_route_conflict_metrics() {
    for (name, source) in [
        ("temporal", TEMPORAL_LAYOUT_REGRESSION),
        ("complex", fixtures::complex_architecture_mermaid()),
    ] {
        let mut graph = parse(source).unwrap();
        layout(&mut graph);
        // Metrics are only meaningful once every edge has a complete route.
        let unrouted: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| edge.route.is_none())
            .map(|edge| (edge.source.as_str(), edge.target.as_str()))
            .collect();
        assert!(unrouted.is_empty(), "{name}: unrouted edges {unrouted:?}");
        let metrics = route_conflicts(&graph, name);
        let (max_crossings, max_shared) = if name == "temporal" { (8, 8) } else { (80, 80) };
        assert!(
            metrics.perpendicular_crossings <= max_crossings,
            "{name}: perpendicular crossings {} > {max_crossings}",
            metrics.perpendicular_crossings
        );
        assert!(
            metrics.unrelated_shared_cells <= max_shared,
            "{name}: unrelated shared cells {} > {max_shared}",
            metrics.unrelated_shared_cells
        );
    }
}

#[test]
fn layout_invariants_hold_for_important_adversarial_and_generated_fixtures() {
    let mut failures = Vec::new();
    for (name, source) in important_fixtures()
        .into_iter()
        .chain(adversarial_fixtures())
    {
        let mut first = parse(source).unwrap_or_else(|err| panic!("{name} failed to parse: {err}"));
        let mut second =
            parse(source).unwrap_or_else(|err| panic!("{name} failed to parse again: {err}"));
        layout(&mut first);
        layout(&mut second);
        collect_invariant_failures(name, &first, &second, &mut failures);
    }
    for (name, graph) in generated_graph_matrix() {
        let mut first = graph.clone();
        let mut second = graph;
        layout(&mut first);
        layout(&mut second);
        collect_invariant_failures(&name, &first, &second, &mut failures);
    }

    if !failures.is_empty() {
        panic!("layout invariant failures:\n{}", failures.join("\n"));
    }
}

fn generated_graph_matrix() -> Vec<(String, Graph)> {
    let mut graphs = Vec::new();
    for direction in [Direction::TopDown, Direction::LeftRight] {
        for reversed in [false, true] {
            for cycle in [false, true] {
                let mut ids = ["A", "B", "C", "D", "E"];
                if reversed {
                    ids.reverse();
                }
                let nodes = ids
                    .iter()
                    .map(|id| Node {
                        id: (*id).into(),
                        label: format!("Node {id}"),
                        shape: if *id == "C" {
                            NodeShape::Diamond
                        } else {
                            NodeShape::Rectangle
                        },
                        x: None,
                        y: None,
                        width: None,
                        height: None,
                    })
                    .collect();
                let mut edges = vec![
                    edge("A", "B", None),
                    edge("B", "C", Some("to decision")),
                    edge("A", "D", Some("fan out")),
                    edge("C", "E", None),
                    edge("D", "E", Some("fan in")),
                ];
                if cycle {
                    edges.push(edge("E", "B", Some("return")));
                }
                graphs.push((
                    format!(
                        "generated {:?} reversed={reversed} cycle={cycle}",
                        direction
                    ),
                    Graph {
                        direction: direction.clone(),
                        nodes,
                        edges,
                        groups: vec![],
                    },
                ));
            }
        }
    }
    graphs
}

#[test]
fn validator_rejects_route_through_endpoint_interior() {
    let graph = Graph {
        direction: Direction::LeftRight,
        nodes: vec![
            laid_out_node("A", 0.0, 0.0, 5.0, 5.0),
            laid_out_node("B", 10.0, 0.0, 5.0, 5.0),
        ],
        edges: vec![Edge {
            source: "A".into(),
            target: "B".into(),
            label: None,
            style: EdgeStyle::Solid,
            arrowhead: Arrowhead::Normal,
            route: Some(RoutePlan {
                points: vec![
                    RoutePoint { x: 5.0, y: 2.0 },
                    RoutePoint { x: 2.0, y: 2.0 },
                    RoutePoint { x: 9.0, y: 2.0 },
                ],
                source_port: Port {
                    x: 5.0,
                    y: 2.0,
                    side: PortSide::Right,
                },
                target_port: Port {
                    x: 9.0,
                    y: 2.0,
                    side: PortSide::Left,
                },
                lane_id: None,
                class: EdgeClass::Primary,
                label_anchor: None,
            }),
        }],
        groups: vec![],
    };

    let failures = assert_layout_invariants("endpoint interior", &graph).unwrap_err();
    assert!(
        failures.iter().any(|failure| failure.contains("endpoint")),
        "{failures:#?}"
    );
}

#[test]
fn validator_rejects_route_that_immediately_turns_through_port_node() {
    let graph = Graph {
        direction: Direction::LeftRight,
        nodes: vec![
            laid_out_node("A", 0.0, 0.0, 5.0, 5.0),
            laid_out_node("B", 10.0, 0.0, 5.0, 5.0),
        ],
        edges: vec![Edge {
            source: "A".into(),
            target: "B".into(),
            label: None,
            style: EdgeStyle::Solid,
            arrowhead: Arrowhead::Normal,
            route: Some(RoutePlan {
                points: vec![
                    RoutePoint { x: 5.0, y: 2.0 },
                    RoutePoint { x: 5.0, y: 4.0 },
                    RoutePoint { x: 9.0, y: 4.0 },
                    RoutePoint { x: 9.0, y: 2.0 },
                ],
                source_port: Port {
                    x: 5.0,
                    y: 2.0,
                    side: PortSide::Right,
                },
                target_port: Port {
                    x: 9.0,
                    y: 2.0,
                    side: PortSide::Left,
                },
                lane_id: None,
                class: EdgeClass::Primary,
                label_anchor: None,
            }),
        }],
        groups: vec![],
    };

    let failures = assert_layout_invariants("bad stub", &graph).unwrap_err();
    assert!(
        failures.iter().any(|failure| failure.contains("port side")),
        "{failures:#?}"
    );
}

#[derive(Debug, Default, Clone, Copy)]
struct RouteConflictMetrics {
    perpendicular_crossings: usize,
    unrelated_shared_cells: usize,
}

fn route_conflicts(graph: &Graph, _name_tag: &str) -> RouteConflictMetrics {
    let routed: Vec<_> = graph
        .edges
        .iter()
        .filter_map(|edge| {
            edge.route
                .as_ref()
                .map(|route| (edge, route, raster_route(route)))
        })
        .collect();
    let mut metrics = RouteConflictMetrics::default();
    for (index, (edge_a, route_a, cells_a)) in routed.iter().enumerate() {
        for (edge_b, route_b, cells_b) in routed.iter().skip(index + 1) {
            if intentional_bundle_pair(edge_a, route_a, edge_b, route_b) {
                continue;
            }
            for (cell, dirs_a) in cells_a {
                let Some(dirs_b) = cells_b.get(cell) else {
                    continue;
                };
                if shared_endpoint_adjacent_cell(edge_a, route_a, edge_b, route_b, *cell) {
                    continue;
                }
                metrics.unrelated_shared_cells += 1;
                if perpendicular(*dirs_a, *dirs_b) {
                    metrics.perpendicular_crossings += 1;
                }
            }
        }
    }
    metrics
}

fn intentional_bundle_pair(a: &Edge, ar: &RoutePlan, b: &Edge, br: &RoutePlan) -> bool {
    ar.lane_id.is_some()
        && ar.lane_id == br.lane_id
        && (a.source == b.source || a.target == b.target)
}

fn shared_endpoint_adjacent_cell(
    a: &Edge,
    ar: &RoutePlan,
    b: &Edge,
    br: &RoutePlan,
    cell: (i64, i64),
) -> bool {
    for endpoint in [&a.source, &a.target] {
        if (endpoint == &b.source || endpoint == &b.target)
            && endpoint_adjacent_cells(a, ar, endpoint).contains(&cell)
            && endpoint_adjacent_cells(b, br, endpoint).contains(&cell)
        {
            return true;
        }
    }
    false
}

fn endpoint_adjacent_cells(edge: &Edge, route: &RoutePlan, endpoint: &str) -> HashSet<(i64, i64)> {
    if endpoint == edge.source {
        route.points.iter().take(2).map(raster_point).collect()
    } else if endpoint == edge.target {
        route
            .points
            .iter()
            .rev()
            .take(2)
            .map(raster_point)
            .collect()
    } else {
        HashSet::new()
    }
}

fn raster_route(route: &RoutePlan) -> HashMap<(i64, i64), u8> {
    let mut cells = HashMap::new();
    for segment in route.points.windows(2) {
        let mut cell = raster_point(&segment[0]);
        let end = raster_point(&segment[1]);
        while cell != end {
            let next = if cell.0 != end.0 {
                (cell.0 + (end.0 - cell.0).signum(), cell.1)
            } else {
                (cell.0, cell.1 + (end.1 - cell.1).signum())
            };
            let (forward, backward) = if next.0 > cell.0 {
                (8, 4)
            } else if next.0 < cell.0 {
                (4, 8)
            } else if next.1 > cell.1 {
                (2, 1)
            } else {
                (1, 2)
            };
            *cells.entry(cell).or_default() |= forward;
            *cells.entry(next).or_default() |= backward;
            cell = next;
        }
    }
    cells
}

fn raster_point(point: &RoutePoint) -> (i64, i64) {
    (point.x.round() as i64, point.y.round() as i64)
}

fn perpendicular(a: u8, b: u8) -> bool {
    let horizontal = 4 | 8;
    let vertical = 1 | 2;
    (a & horizontal != 0 && b & vertical != 0) || (a & vertical != 0 && b & horizontal != 0)
}

fn laid_out_node(id: &str, x: f64, y: f64, width: f64, height: f64) -> Node {
    Node {
        id: id.into(),
        label: id.into(),
        shape: NodeShape::Rectangle,
        x: Some(x),
        y: Some(y),
        width: Some(width),
        height: Some(height),
    }
}

fn edge(source: &str, target: &str, label: Option<&str>) -> Edge {
    Edge {
        source: source.into(),
        target: target.into(),
        label: label.map(str::to_owned),
        style: EdgeStyle::Solid,
        arrowhead: Arrowhead::Normal,
        route: None,
    }
}

fn collect_invariant_failures(
    name: &str,
    first: &Graph,
    second: &Graph,
    failures: &mut Vec<String>,
) {
    if first != second {
        failures.push(format!("{name}: nondeterministic layout"));
    }
    if let Err(errs) = assert_layout_invariants(name, first) {
        failures.extend(errs);
    }
}

fn assert_failure(graph: &Graph, needle: &str) {
    let failures = assert_layout_invariants("corrupt", graph).unwrap_err();
    assert!(
        failures.iter().any(|failure| failure.contains(needle)),
        "expected failure containing {needle:?}, got {failures:#?}"
    );
}

fn assert_layout_invariants(name: &str, graph: &Graph) -> Result<(), Vec<String>> {
    let mut failures = Vec::new();
    check_node_geometry(name, graph, &mut failures);
    check_endpoint_existence(name, graph, &mut failures);
    check_real_node_overlaps(name, graph, &mut failures);
    check_group_bounds(name, graph, &mut failures);
    check_routes(name, graph, &mut failures);
    check_label_footprints(name, graph, &mut failures);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn check_node_geometry(name: &str, graph: &Graph, failures: &mut Vec<String>) {
    for node in &graph.nodes {
        for (field, value) in [
            ("x", node.x),
            ("y", node.y),
            ("width", node.width),
            ("height", node.height),
        ] {
            match value {
                Some(value) if value.is_finite() && value >= 0.0 => {}
                _ => failures.push(format!(
                    "{name}: node {} has invalid {field}: {value:?}",
                    node.id
                )),
            }
        }
        if !is_dummy(node) && rect(node).is_some_and(|(_, _, w, h)| w <= 0.0 || h <= 0.0) {
            failures.push(format!(
                "{name}: real node {} has non-positive size",
                node.id
            ));
        }
    }
}

fn check_endpoint_existence(name: &str, graph: &Graph, failures: &mut Vec<String>) {
    let ids: HashSet<_> = graph.nodes.iter().map(|node| node.id.as_str()).collect();
    for edge in &graph.edges {
        if !ids.contains(edge.source.as_str()) {
            failures.push(format!(
                "{name}: edge {} -> {} missing source node",
                edge.source, edge.target
            ));
        }
        if !ids.contains(edge.target.as_str()) {
            failures.push(format!(
                "{name}: edge {} -> {} missing target node",
                edge.source, edge.target
            ));
        }
    }
}

fn check_real_node_overlaps(name: &str, graph: &Graph, failures: &mut Vec<String>) {
    let nodes: Vec<_> = graph.nodes.iter().filter(|node| !is_dummy(node)).collect();
    for (i, a) in nodes.iter().enumerate() {
        for b in nodes.iter().skip(i + 1) {
            if rects_overlap(a, b) {
                failures.push(format!("{name}: real nodes {} and {} overlap", a.id, b.id));
            }
        }
    }
}

fn check_group_bounds(name: &str, graph: &Graph, failures: &mut Vec<String>) {
    for group in &graph.groups {
        let Some((gx, gy, gw, gh)) = group_rect(group) else {
            failures.push(format!("{name}: group {} has invalid bounds", group.id));
            continue;
        };
        for node_id in &group.node_ids {
            let Some(node) = graph.nodes.iter().find(|node| node.id == *node_id) else {
                failures.push(format!(
                    "{name}: group {} references missing node {node_id}",
                    group.id
                ));
                continue;
            };
            let Some((x, y, w, h)) = rect(node) else {
                continue;
            };
            if x < gx - EPS || y < gy - EPS || x + w > gx + gw + EPS || y + h > gy + gh + EPS {
                failures.push(format!(
                    "{name}: group {} does not cover member {node_id}",
                    group.id
                ));
            }
        }
    }
}

fn check_routes(name: &str, graph: &Graph, failures: &mut Vec<String>) {
    for edge in &graph.edges {
        let Some(route) = &edge.route else {
            failures.push(format!(
                "{name}: edge {} -> {} missing route",
                edge.source, edge.target
            ));
            continue;
        };
        if route.points.len() < 2 {
            failures.push(format!(
                "{name}: edge {} -> {} route has fewer than 2 points",
                edge.source, edge.target
            ));
        }
        for point in &route.points {
            if !valid_coord(point.x) || !valid_coord(point.y) {
                failures.push(format!(
                    "{name}: edge {} -> {} has invalid route point ({},{})",
                    edge.source, edge.target, point.x, point.y
                ));
            }
        }
        if let Some(first) = route.points.first() {
            assert_point_eq_or_record(
                name,
                edge,
                "first point",
                first,
                &route.source_port,
                failures,
            );
        }
        if let Some(last) = route.points.last() {
            assert_point_eq_or_record(name, edge, "last point", last, &route.target_port, failures);
        }
        check_port(
            name,
            graph,
            edge,
            &edge.source,
            &route.source_port,
            failures,
        );
        check_port(
            name,
            graph,
            edge,
            &edge.target,
            &route.target_port,
            failures,
        );
        check_endpoint_stub(name, edge, route, failures);
        for segment in route.points.windows(2) {
            let a = &segment[0];
            let b = &segment[1];
            if !same(a.x, b.x) && !same(a.y, b.y) {
                failures.push(format!(
                    "{name}: edge {} -> {} has non-orthogonal segment ({},{}) -> ({},{})",
                    edge.source, edge.target, a.x, a.y, b.x, b.y
                ));
            }
            for node in graph.nodes.iter().filter(|node| !is_dummy(node)) {
                let endpoint = node.id == edge.source || node.id == edge.target;
                if raster_segment_hits_node_cells(a, b, node) {
                    failures.push(format!(
                        "{name}: edge {} -> {} segment raster-collides with {} node {}",
                        edge.source,
                        edge.target,
                        if endpoint { "endpoint" } else { "unrelated" },
                        node.id
                    ));
                }
            }
        }
    }
}

fn check_endpoint_stub(name: &str, edge: &Edge, route: &RoutePlan, failures: &mut Vec<String>) {
    if !route
        .points
        .get(1)
        .is_some_and(|point| point_matches_outward_stub(point, &route.source_port))
    {
        failures.push(format!(
            "{name}: edge {} -> {} first segment violates source port side {:?}",
            edge.source, edge.target, route.source_port.side
        ));
    }

    if !route
        .points
        .iter()
        .rev()
        .nth(1)
        .is_some_and(|point| point_matches_outward_stub(point, &route.target_port))
    {
        failures.push(format!(
            "{name}: edge {} -> {} last segment violates target port side {:?}",
            edge.source, edge.target, route.target_port.side
        ));
    }
}

fn point_matches_outward_stub(point: &RoutePoint, port: &Port) -> bool {
    match port.side {
        PortSide::Top => same(point.x, port.x) && same(point.y, port.y - 1.0),
        PortSide::Right => same(point.y, port.y) && same(point.x, port.x + 1.0),
        PortSide::Bottom => same(point.x, port.x) && same(point.y, port.y + 1.0),
        PortSide::Left => same(point.y, port.y) && same(point.x, port.x - 1.0),
    }
}

fn check_label_footprints(name: &str, graph: &Graph, failures: &mut Vec<String>) {
    let route_cells: Vec<HashMap<(i64, i64), u8>> = graph
        .edges
        .iter()
        .map(|edge| edge.route.as_ref().map(raster_route).unwrap_or_default())
        .collect();
    let mut occupied_labels = HashSet::new();
    for (index, edge) in graph.edges.iter().enumerate() {
        let Some(label) = &edge.label else { continue };
        let Some(route) = &edge.route else { continue };
        let Some(anchor) = &route.label_anchor else {
            failures.push(format!(
                "{name}: edge {} -> {} label has no anchor",
                edge.source, edge.target
            ));
            continue;
        };
        if !valid_coord(anchor.x) || !valid_coord(anchor.y) {
            failures.push(format!(
                "{name}: edge {} -> {} has invalid label anchor ({},{})",
                edge.source, edge.target, anchor.x, anchor.y
            ));
            continue;
        }
        let width = label.chars().count() as i64;
        let lx = ((anchor.x.max(0.0).round() as u16)
            .saturating_sub((label.chars().count() / 2) as u16)) as i64;
        let ly = ((anchor.y.max(0.0).round() as u16).saturating_sub(1)) as i64;
        for px in lx..lx + width {
            if px < 0 || ly < 0 {
                failures.push(format!(
                    "{name}: edge {} -> {} label footprint is negative",
                    edge.source, edge.target
                ));
                continue;
            }
            if graph
                .nodes
                .iter()
                .filter(|node| !is_dummy(node))
                .any(|node| node_cell_contains(node, px, ly))
            {
                failures.push(format!(
                    "{name}: edge {} -> {} label footprint collides with a real node",
                    edge.source, edge.target
                ));
            }
            if !occupied_labels.insert((px, ly)) {
                failures.push(format!(
                    "{name}: edge {} -> {} label footprint collides with another label",
                    edge.source, edge.target
                ));
            }
            for (other, cells) in route_cells.iter().enumerate() {
                if other != index && cells.contains_key(&(px, ly)) {
                    failures.push(format!(
                        "{name}: edge {} -> {} label footprint overlaps unrelated edge {} -> {} route",
                        edge.source, edge.target,
                        graph.edges[other].source, graph.edges[other].target
                    ));
                }
            }
        }
    }
}

fn check_port(
    name: &str,
    graph: &Graph,
    edge: &Edge,
    node_id: &str,
    port: &Port,
    failures: &mut Vec<String>,
) {
    if !valid_coord(port.x) || !valid_coord(port.y) {
        failures.push(format!(
            "{name}: edge {} -> {} has invalid {:?} port ({},{})",
            edge.source, edge.target, port.side, port.x, port.y
        ));
    }
    let Some(node) = graph.nodes.iter().find(|node| node.id == node_id) else {
        return;
    };
    let Some((x, y, w, h)) = rect(node) else {
        return;
    };
    let ok = match port.side {
        PortSide::Top => same(port.y, y - 1.0) && port.x >= x - EPS && port.x <= x + w + EPS,
        PortSide::Right => same(port.x, x + w) && port.y >= y - EPS && port.y <= y + h + EPS,
        PortSide::Bottom => same(port.y, y + h) && port.x >= x - EPS && port.x <= x + w + EPS,
        PortSide::Left => same(port.x, x - 1.0) && port.y >= y - EPS && port.y <= y + h + EPS,
    };
    if !ok {
        failures.push(format!(
            "{name}: edge {} -> {} port {:?} is inconsistent with node {node_id}",
            edge.source, edge.target, port.side
        ));
    }
}

fn assert_point_eq_or_record(
    name: &str,
    edge: &Edge,
    label: &str,
    point: &RoutePoint,
    port: &Port,
    failures: &mut Vec<String>,
) {
    if !same(point.x, port.x) || !same(point.y, port.y) {
        failures.push(format!(
            "{name}: edge {} -> {} {label} does not match port",
            edge.source, edge.target
        ));
    }
}

fn rects_overlap(a: &Node, b: &Node) -> bool {
    let (Some((ax, ay, aw, ah)), Some((bx, by, bw, bh))) = (rect(a), rect(b)) else {
        return false;
    };
    ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
}

fn raster_segment_hits_node_cells(a: &RoutePoint, b: &RoutePoint, node: &Node) -> bool {
    if same(a.x, b.x) {
        let x = a.x.round() as i64;
        let y1 = a.y.min(b.y).round() as i64;
        let y2 = a.y.max(b.y).round() as i64;
        (y1..=y2).any(|y| node_cell_contains(node, x, y))
    } else if same(a.y, b.y) {
        let y = a.y.round() as i64;
        let x1 = a.x.min(b.x).round() as i64;
        let x2 = a.x.max(b.x).round() as i64;
        (x1..=x2).any(|x| node_cell_contains(node, x, y))
    } else {
        false
    }
}

fn node_cell_contains(node: &Node, px: i64, py: i64) -> bool {
    let Some((x, y, w, h)) = node_cell_rect(node) else {
        return false;
    };
    px >= x && px < x + w && py >= y && py < y + h
}

fn rect(node: &Node) -> Option<(f64, f64, f64, f64)> {
    Some((node.x?, node.y?, node.width?, node.height?))
}

fn group_rect(group: &Group) -> Option<(f64, f64, f64, f64)> {
    let rect = (group.x?, group.y?, group.width?, group.height?);
    if [rect.0, rect.1, rect.2, rect.3]
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
    {
        Some(rect)
    } else {
        None
    }
}

fn node_cell_rect(node: &Node) -> Option<(i64, i64, i64, i64)> {
    let (x, y, w, h) = rect(node)?;
    if [x, y, w, h]
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
    {
        Some((x as i64, y as i64, w as i64, h as i64))
    } else {
        None
    }
}

fn is_dummy(node: &Node) -> bool {
    node.id.starts_with("__dummy")
}

fn valid_coord(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() < EPS
}
