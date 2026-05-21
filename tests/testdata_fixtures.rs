use diaview::model::*;
use diaview::testdata::fixtures;

fn phase15_fixtures() -> [(&'static str, fn() -> &'static str); 5] {
    [
        ("fan-in sink", fixtures::phase15_fan_in_sink_mermaid),
        ("fan-out router", fixtures::phase15_fan_out_router_mermaid),
        ("back-edge cycle", fixtures::phase15_back_edge_cycle_mermaid),
        (
            "telemetry overlay",
            fixtures::phase15_telemetry_overlay_mermaid,
        ),
        (
            "grouped architecture",
            fixtures::phase15_grouped_architecture_mermaid,
        ),
    ]
}

fn assert_all_positioned(graph: &Graph) {
    for node in &graph.nodes {
        assert!(node.x.is_some(), "node {} missing x", node.id);
        assert!(node.y.is_some(), "node {} missing y", node.id);
        assert!(node.width.is_some(), "node {} missing width", node.id);
        assert!(node.height.is_some(), "node {} missing height", node.id);
    }
}

fn assert_no_node_rectangle_overlaps(graph: &Graph) {
    for (i, a) in graph.nodes.iter().enumerate() {
        for b in graph.nodes.iter().skip(i + 1) {
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

fn graph_bounds(graph: &Graph) -> (f64, f64) {
    graph.nodes.iter().fold((0.0_f64, 0.0_f64), |(w, h), node| {
        let right = node.x.unwrap_or(0.0) + node.width.unwrap_or(0.0);
        let bottom = node.y.unwrap_or(0.0) + node.height.unwrap_or(0.0);
        (w.max(right), h.max(bottom))
    })
}

fn parse_and_layout_phase15_fixture(name: &str, mermaid: &str) -> Graph {
    let mut graph = diaview::parser::mermaid::parse(mermaid)
        .unwrap_or_else(|err| panic!("{name} fixture failed to parse: {err}"));
    diaview::layout::layout(&mut graph);
    graph
}

#[test]
fn simple_fixture_parses_layouts_and_renders() {
    let mut graph = diaview::parser::mermaid::parse(fixtures::simple_mermaid()).unwrap();
    assert_eq!(graph.nodes.len(), 14);
    assert_eq!(graph.edges.len(), 13);
    assert!(graph.groups.is_empty());
    assert!(graph.nodes.iter().any(|node| node.id == "ROUTER"));
    assert!(graph.nodes.iter().any(|node| node.id == "METRICS"));

    diaview::layout::layout(&mut graph);
    assert_all_positioned(&graph);
    assert_no_node_rectangle_overlaps(&graph);

    let rendered = diaview::renderer::canvas::render_to_string(&graph).unwrap();
    assert!(rendered.contains("Request Router"));
    assert!(rendered.contains("Metrics"));
}

#[test]
fn complex_architecture_fixture_parses() {
    let graph = diaview::parser::mermaid::parse(fixtures::complex_architecture_mermaid()).unwrap();

    assert_eq!(graph.nodes.len(), 66);
    assert_eq!(graph.edges.len(), 87);
    assert!(graph.nodes.iter().any(|node| node.id == "ROUTER"));
    assert!(graph.nodes.iter().any(|node| node.id == "SUCCESS"));
}

#[test]
fn phase15_diagnostic_fixtures_parse_layout_without_node_overlaps() {
    for (name, fixture) in phase15_fixtures() {
        let graph = parse_and_layout_phase15_fixture(name, fixture());

        assert_all_positioned(&graph);
        assert_no_node_rectangle_overlaps(&graph);
    }
}

#[test]
fn dump_phase15_diagnostic_fixture_metrics() {
    for (name, fixture) in phase15_fixtures() {
        let graph = parse_and_layout_phase15_fixture(name, fixture());
        let (width, height) = graph_bounds(&graph);

        println!(
            "phase 1.5 {name}: {} nodes, {} edges, bounds {:.0}x{:.0}",
            graph.nodes.len(),
            graph.edges.len(),
            width,
            height
        );
    }
}
