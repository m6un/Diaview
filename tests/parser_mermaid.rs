use diaview::model::*;
use diaview::parser::mermaid::parse;
use diaview::testdata::fixtures;

#[test]
fn test_simple_two_node_fixture() {
    let input = r#"
        graph TD
        A(Start) -->|next| B(End)
    "#;
    let graph = parse(input).unwrap();
    assert_eq!(graph, fixtures::simple_two_node());
}

#[test]
fn test_diamond_decision_fixture() {
    let input = r#"
        graph TD
        A(Start) --> B{Decision}
        B -->|yes| C[Yes path]
        B -.->|no| D[No path]
    "#;
    let graph = parse(input).unwrap();
    assert_eq!(graph, fixtures::diamond_decision());
}

#[test]
fn test_subgraphs_parse_groups_and_members() {
    let input = r#"
        graph TD
        subgraph API[API Layer]
            A[Gateway] --> B[Service]
            subgraph Workers(Worker Pool)
                C[Worker]
            end
            B --> C
        end
    "#;
    let graph = parse(input).unwrap();

    assert_eq!(graph.groups.len(), 2);
    assert_eq!(graph.groups[0].id, "API");
    assert_eq!(graph.groups[0].label, "API Layer");
    assert_eq!(graph.groups[0].parent, None);
    assert_eq!(graph.groups[0].node_ids, vec!["A", "B", "C"]);
    assert_eq!(graph.groups[1].id, "Workers");
    assert_eq!(graph.groups[1].label, "Worker Pool");
    assert_eq!(graph.groups[1].parent.as_deref(), Some("API"));
    assert_eq!(graph.groups[1].node_ids, vec!["C"]);
}

#[test]
fn test_left_right_chain_fixture() {
    let input = r#"
        graph LR
        A[Input] --> B(Process)
        B -.->|result| C((Output))
    "#;
    let graph = parse(input).unwrap();
    let expected = fixtures::left_right_chain();
    assert_eq!(graph.direction, expected.direction);
    assert_eq!(graph.nodes, expected.nodes);
    assert_eq!(graph.edges.len(), expected.edges.len());
    assert_eq!(graph.edges[0], expected.edges[0]);
    assert_eq!(graph.edges[1].source, expected.edges[1].source);
    assert_eq!(graph.edges[1].target, expected.edges[1].target);
    assert_eq!(graph.edges[1].label, expected.edges[1].label);
    assert_eq!(graph.edges[1].style, EdgeStyle::Dashed);
    assert_eq!(graph.edges[1].arrowhead, Arrowhead::Normal);
}

#[test]
fn test_graph_td() {
    let g = parse("graph TD\nA --> B").unwrap();
    assert_eq!(g.direction, Direction::TopDown);
}

#[test]
fn test_graph_tb() {
    let g = parse("graph TB\nA --> B").unwrap();
    assert_eq!(g.direction, Direction::TopDown);
}

#[test]
fn test_graph_lr() {
    let g = parse("graph LR\nA --> B").unwrap();
    assert_eq!(g.direction, Direction::LeftRight);
}

#[test]
fn test_flowchart_td() {
    let g = parse("flowchart TD\nA --> B").unwrap();
    assert_eq!(g.direction, Direction::TopDown);
}

#[test]
fn test_flowchart_lr() {
    let g = parse("flowchart LR\nA --> B").unwrap();
    assert_eq!(g.direction, Direction::LeftRight);
}

#[test]
fn test_rectangle_node() {
    let g = parse("graph TD\nA[Hello]").unwrap();
    assert_eq!(g.nodes[0].shape, NodeShape::Rectangle);
    assert_eq!(g.nodes[0].label, "Hello");
    assert_eq!(g.nodes[0].id, "A");
}

#[test]
fn test_rounded_rect_node() {
    let g = parse("graph TD\nA(Hello)").unwrap();
    assert_eq!(g.nodes[0].shape, NodeShape::RoundedRect);
    assert_eq!(g.nodes[0].label, "Hello");
}

#[test]
fn test_diamond_node() {
    let g = parse("graph TD\nA{Hello}").unwrap();
    assert_eq!(g.nodes[0].shape, NodeShape::Diamond);
    assert_eq!(g.nodes[0].label, "Hello");
}

#[test]
fn test_circle_node() {
    let g = parse("graph TD\nA((Hello))").unwrap();
    assert_eq!(g.nodes[0].shape, NodeShape::Circle);
    assert_eq!(g.nodes[0].label, "Hello");
}

#[test]
fn test_database_cylinder_node_maps_to_rectangle() {
    let g = parse("graph TD\nDB[(Payment DB)]").unwrap();
    assert_eq!(g.nodes[0].id, "DB");
    assert_eq!(g.nodes[0].shape, NodeShape::Rectangle);
    assert_eq!(g.nodes[0].label, "Payment DB");
}

#[test]
fn test_bare_node() {
    let g = parse("graph TD\nMyNode").unwrap();
    assert_eq!(g.nodes[0].id, "MyNode");
    assert_eq!(g.nodes[0].label, "MyNode");
    assert_eq!(g.nodes[0].shape, NodeShape::Rectangle);
}

#[test]
fn test_all_four_shapes_in_one() {
    let input = "graph TD\nA[Rect] --> B(Round)\nC{Diamond} --> D((Circle))";
    let g = parse(input).unwrap();
    assert_eq!(g.nodes.len(), 4);
    assert_eq!(g.nodes[0].shape, NodeShape::Rectangle);
    assert_eq!(g.nodes[1].shape, NodeShape::RoundedRect);
    assert_eq!(g.nodes[2].shape, NodeShape::Diamond);
    assert_eq!(g.nodes[3].shape, NodeShape::Circle);
}

#[test]
fn test_solid_arrow() {
    let g = parse("graph TD\nA --> B").unwrap();
    assert_eq!(g.edges[0].style, EdgeStyle::Solid);
    assert_eq!(g.edges[0].arrowhead, Arrowhead::Normal);
}

#[test]
fn test_edge_to_database_cylinder_node() {
    let g = parse("graph TD\nA --> DB[(Payment DB)]").unwrap();
    assert_eq!(g.nodes.len(), 2);
    assert_eq!(g.edges.len(), 1);
    assert_eq!(g.edges[0].source, "A");
    assert_eq!(g.edges[0].target, "DB");
    assert_eq!(g.nodes[1].id, "DB");
    assert_eq!(g.nodes[1].shape, NodeShape::Rectangle);
    assert_eq!(g.nodes[1].label, "Payment DB");
}

#[test]
fn test_solid_no_arrow() {
    let g = parse("graph TD\nA --- B").unwrap();
    assert_eq!(g.edges[0].style, EdgeStyle::Solid);
    assert_eq!(g.edges[0].arrowhead, Arrowhead::None);
}

#[test]
fn test_dashed_arrow() {
    let g = parse("graph TD\nA -.-> B").unwrap();
    assert_eq!(g.edges[0].style, EdgeStyle::Dashed);
    assert_eq!(g.edges[0].arrowhead, Arrowhead::Normal);
}

#[test]
fn test_dashed_no_arrow() {
    let g = parse("graph TD\nA -.- B").unwrap();
    assert_eq!(g.edges[0].style, EdgeStyle::Dashed);
    assert_eq!(g.edges[0].arrowhead, Arrowhead::None);
}

#[test]
fn test_thick_arrow() {
    let g = parse("graph TD\nA ==> B").unwrap();
    assert_eq!(g.edges[0].style, EdgeStyle::Solid);
    assert_eq!(g.edges[0].arrowhead, Arrowhead::Normal);
}

#[test]
fn test_pipe_label() {
    let g = parse("graph TD\nA -->|yes| B").unwrap();
    assert_eq!(g.edges[0].label, Some("yes".into()));
}

#[test]
fn test_inline_label() {
    let g = parse("graph TD\nA -- yes --> B").unwrap();
    assert_eq!(g.edges[0].label, Some("yes".into()));
    assert_eq!(g.edges[0].style, EdgeStyle::Solid);
    assert_eq!(g.edges[0].arrowhead, Arrowhead::Normal);
}

#[test]
fn test_inline_label_no_arrow() {
    let g = parse("graph TD\nA -- label --- B").unwrap();
    assert_eq!(g.edges[0].label, Some("label".into()));
    assert_eq!(g.edges[0].arrowhead, Arrowhead::None);
}

#[test]
fn test_no_label() {
    let g = parse("graph TD\nA --> B").unwrap();
    assert_eq!(g.edges[0].label, None);
}

#[test]
fn test_comments_ignored() {
    let input = "\
%% this is a comment
graph TD
%% another comment
A --> B
%% end";
    let g = parse(input).unwrap();
    assert_eq!(g.nodes.len(), 2);
    assert_eq!(g.edges.len(), 1);
}

#[test]
fn test_semicolons() {
    let input = "graph TD\nA --> B; B --> C";
    let g = parse(input).unwrap();
    assert_eq!(g.edges.len(), 2);
    assert_eq!(g.nodes.len(), 3);
}

#[test]
fn test_multiline() {
    let input = "\
graph TD
A[Start] --> B[Middle]
B --> C[End]
";
    let g = parse(input).unwrap();
    assert_eq!(g.nodes.len(), 3);
    assert_eq!(g.edges.len(), 2);
    assert_eq!(g.nodes[0].label, "Start");
    assert_eq!(g.nodes[1].label, "Middle");
    assert_eq!(g.nodes[2].label, "End");
}

#[test]
fn test_node_dedup() {
    let input = "graph TD\nA[Hello] --> B\nA --> B";
    let g = parse(input).unwrap();
    assert_eq!(g.nodes.len(), 2);
    assert_eq!(g.edges.len(), 2);
    assert_eq!(g.nodes[0].label, "Hello");
}

#[test]
fn test_bare_then_declared() {
    let input = "graph TD\nA --> B\nB[World]";
    let g = parse(input).unwrap();
    assert_eq!(g.nodes.len(), 2);
    assert_eq!(g.nodes[1].label, "World");
    assert_eq!(g.nodes[1].shape, NodeShape::Rectangle);
}

#[test]
fn test_positions_none() {
    let g = parse("graph TD\nA --> B").unwrap();
    for n in &g.nodes {
        assert_eq!(n.x, None);
        assert_eq!(n.y, None);
        assert_eq!(n.width, None);
        assert_eq!(n.height, None);
    }
}

#[test]
fn test_empty_input() {
    assert!(parse("").is_err());
}

#[test]
fn test_missing_direction() {
    assert!(parse("graph\nA --> B").is_err());
}

#[test]
fn test_invalid_direction() {
    assert!(parse("graph XX\nA --> B").is_err());
}

#[test]
fn test_bad_keyword() {
    assert!(parse("diagram TD\nA --> B").is_err());
}

#[test]
fn test_unclosed_bracket() {
    assert!(parse("graph TD\nA[hello").is_err());
}

#[test]
fn test_unclosed_paren() {
    assert!(parse("graph TD\nA(hello").is_err());
}

#[test]
fn test_unclosed_double_paren() {
    assert!(parse("graph TD\nA((hello)").is_err());
}

#[test]
fn test_unclosed_curly() {
    assert!(parse("graph TD\nA{hello").is_err());
}

#[test]
fn test_unclosed_pipe_label() {
    assert!(parse("graph TD\nA -->|oops B").is_err());
}

#[test]
fn test_only_comments() {
    assert!(parse("%% nothing here").is_err());
}
