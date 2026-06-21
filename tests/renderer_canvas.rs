use diaview::model::*;
use diaview::renderer::canvas::{render_to_frame, render_to_frame_with_theme, render_to_string};
use diaview::theme::Theme;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

const DIR_UP: u8 = 0b0001;
const DIR_DOWN: u8 = 0b0010;
const DIR_LEFT: u8 = 0b0100;
const DIR_RIGHT: u8 = 0b1000;

fn glyph_dirs(ch: char) -> u8 {
    match ch {
        '│' => DIR_UP | DIR_DOWN,
        '─' => DIR_LEFT | DIR_RIGHT,
        '┌' => DIR_DOWN | DIR_RIGHT,
        '┐' => DIR_DOWN | DIR_LEFT,
        '└' => DIR_UP | DIR_RIGHT,
        '┘' => DIR_UP | DIR_LEFT,
        '├' => DIR_UP | DIR_DOWN | DIR_RIGHT,
        '┤' => DIR_UP | DIR_DOWN | DIR_LEFT,
        '┬' => DIR_LEFT | DIR_RIGHT | DIR_DOWN,
        '┴' => DIR_LEFT | DIR_RIGHT | DIR_UP,
        '┼' => DIR_UP | DIR_DOWN | DIR_LEFT | DIR_RIGHT,
        _ => 0,
    }
}

fn rounded_point(x: f64, y: f64) -> (u16, u16) {
    (x.max(0.0).round() as u16, y.max(0.0).round() as u16)
}

fn outward_delta(side: &PortSide) -> (i32, i32) {
    match side {
        PortSide::Top => (0, -1),
        PortSide::Right => (1, 0),
        PortSide::Bottom => (0, 1),
        PortSide::Left => (-1, 0),
    }
}

fn offset_point(point: (u16, u16), delta: (i32, i32)) -> (u16, u16) {
    (
        (point.0 as i32 + delta.0).max(0) as u16,
        (point.1 as i32 + delta.1).max(0) as u16,
    )
}

fn push_distinct(points: &mut Vec<(u16, u16)>, point: (u16, u16)) {
    if points.last().copied() != Some(point) {
        points.push(point);
    }
}

fn routed_render_points(route: &RoutePlan) -> Vec<(u16, u16)> {
    let source_port = rounded_point(route.source_port.x, route.source_port.y);
    let target_port = rounded_point(route.target_port.x, route.target_port.y);

    let mut points = Vec::new();
    push_distinct(&mut points, source_port);
    for point in &route.points {
        push_distinct(&mut points, rounded_point(point.x, point.y));
    }
    push_distinct(&mut points, target_port);

    if points.is_empty() {
        return points;
    }

    let source_stub = offset_point(points[0], outward_delta(&route.source_port.side));
    if points.get(1).copied() != Some(source_stub) {
        points.insert(1, source_stub);
    }

    let target_index = points.len() - 1;
    let target_tail = offset_point(points[target_index], outward_delta(&route.target_port.side));
    if target_index == 0 || points.get(target_index - 1).copied() != Some(target_tail) {
        points.insert(target_index, target_tail);

        let tail_index = points.len() - 2;
        if tail_index > 0 {
            let prev = points[tail_index - 1];
            let tail = points[tail_index];
            if prev.0 != tail.0 && prev.1 != tail.1 {
                let pre_tail = match route.target_port.side {
                    PortSide::Top | PortSide::Bottom => (prev.0, tail.1),
                    PortSide::Left | PortSide::Right => (tail.0, prev.1),
                };
                if pre_tail != prev && pre_tail != tail {
                    points.insert(tail_index, pre_tail);
                }
            }
        }
    }

    points
}

fn graph_bounds(graph: &Graph) -> (u16, u16) {
    let mut max_x = 1.0_f64;
    let mut max_y = 1.0_f64;

    for node in &graph.nodes {
        if let (Some(x), Some(y), Some(w), Some(h)) = (node.x, node.y, node.width, node.height) {
            max_x = max_x.max(x + w + 2.0);
            max_y = max_y.max(y + h + 2.0);
        }
    }

    for edge in &graph.edges {
        if let Some(route) = &edge.route {
            for (x, y) in routed_render_points(route) {
                max_x = max_x.max(x as f64 + 2.0);
                max_y = max_y.max(y as f64 + 2.0);
            }

            if let Some(label) = &edge.label {
                let anchor = route
                    .label_anchor
                    .as_ref()
                    .or_else(|| route.points.get(route.points.len() / 2));
                if let Some(anchor) = anchor {
                    let label_width = label.chars().count() as u16;
                    let lx =
                        (anchor.x.max(0.0).round() as u16).saturating_sub((label.len() / 2) as u16);
                    let ly = (anchor.y.max(0.0).round() as u16).saturating_sub(1);
                    max_x = max_x.max((lx + label_width + 1) as f64);
                    max_y = max_y.max((ly + 2) as f64);
                }
            }
        }
    }

    for group in &graph.groups {
        if let (Some(x), Some(y), Some(w), Some(h)) = (group.x, group.y, group.width, group.height)
        {
            max_x = max_x.max(x + w + 2.0);
            max_y = max_y.max(y + h + 2.0);
        }
    }

    (max_x.ceil() as u16, max_y.ceil() as u16)
}

fn test_graph() -> Graph {
    Graph {
        direction: Direction::TopDown,
        nodes: vec![
            Node {
                id: "A".into(),
                label: "Start".into(),
                shape: NodeShape::RoundedRect,
                x: Some(5.0),
                y: Some(1.0),
                width: Some(11.0),
                height: Some(3.0),
            },
            Node {
                id: "B".into(),
                label: "End".into(),
                shape: NodeShape::RoundedRect,
                x: Some(5.0),
                y: Some(8.0),
                width: Some(11.0),
                height: Some(3.0),
            },
        ],
        edges: vec![Edge {
            source: "A".into(),
            target: "B".into(),
            label: Some("next".into()),
            style: EdgeStyle::Solid,
            arrowhead: Arrowhead::Normal,
            route: None,
        }],
        groups: vec![],
    }
}

#[test]
fn test_group_bounds_render_behind_nodes() {
    let mut graph = diaview::parser::mermaid::parse(
        r#"
        graph TD
        subgraph API[API Layer]
            A[Gateway] --> B[Service]
        end
        "#,
    )
    .unwrap();
    diaview::layout::layout(&mut graph);

    let output = render_to_string(&graph).unwrap();
    assert!(output.contains("API Layer"));
    assert!(output.contains('┌'));
    assert!(output.contains('┘'));
}

#[test]
fn test_rounded_rect_renders_as_borderless_card() {
    let graph = test_graph();
    let theme = Theme::default();
    let backend = TestBackend::new(30, 14);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    assert_eq!(buf[(5, 1)].symbol(), " ");
    assert_eq!(buf[(5, 1)].bg, theme.rounded_rect.fill);
    assert_eq!(buf[(15, 3)].symbol(), " ");
    assert_eq!(buf[(15, 3)].bg, theme.rounded_rect.fill);
}

#[test]
fn test_rectangle_renders_as_borderless_card() {
    let graph = Graph {
        direction: Direction::TopDown,
        nodes: vec![Node {
            id: "R".into(),
            label: "Box".into(),
            shape: NodeShape::Rectangle,
            x: Some(2.0),
            y: Some(1.0),
            width: Some(9.0),
            height: Some(3.0),
        }],
        edges: vec![],
        groups: vec![],
    };
    let backend = TestBackend::new(20, 8);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    let theme = Theme::default();
    assert_eq!(buf[(2, 1)].symbol(), " ");
    assert_eq!(buf[(2, 1)].bg, theme.rectangle.fill);
    assert_eq!(buf[(10, 3)].symbol(), " ");
    assert_eq!(buf[(10, 3)].bg, theme.rectangle.fill);
}

#[test]
fn test_node_fill_stays_inside_node_bounds() {
    let graph = test_graph();
    let theme = Theme::default();
    let backend = TestBackend::new(30, 14);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    assert_eq!(buf[(6, 2)].bg, theme.rounded_rect.fill);
    assert_eq!(buf[(5, 1)].bg, theme.rounded_rect.fill);
    assert_eq!(buf[(5, 2)].bg, theme.rounded_rect.fill);
    assert_eq!(buf[(16, 1)].bg, Color::Reset);
    assert_eq!(buf[(16, 2)].symbol(), "▏");
    assert_eq!(buf[(16, 2)].fg, theme.shadow);
    assert_eq!(buf[(16, 2)].bg, Color::Reset);
    assert_eq!(buf[(5, 4)].symbol(), "▔");
    assert_eq!(buf[(5, 4)].fg, theme.shadow);
    assert_eq!(buf[(5, 4)].bg, Color::Reset);
}

#[test]
fn test_edge_label_is_text_only() {
    let graph = test_graph();
    let theme = Theme::default();
    let backend = TestBackend::new(30, 14);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    let rendered: String = (11..15).map(|x| buf[(x, 5)].symbol().to_string()).collect();
    assert_eq!(rendered, "next");
    for x in 11..15 {
        assert_eq!(buf[(x, 5)].fg, theme.edge_label);
        assert_eq!(buf[(x, 5)].bg, Color::Reset);
    }
}

#[test]
fn test_inline_output_emits_background_ansi() {
    let output = render_to_string(&test_graph()).unwrap();
    assert!(
        output.contains("\x1b[48;2;"),
        "inline output should include truecolor background escapes"
    );
    assert!(
        output.contains("\x1b[49m"),
        "inline output should reset background color"
    );
}

#[test]
fn test_routed_label_extends_inline_bounds() {
    let mut graph = diaview::parser::mermaid::parse(
        r#"
        graph LR
        A[Start] -->|metrics| B[End]
        "#,
    )
    .unwrap();
    diaview::layout::layout(&mut graph);

    let output = render_to_string(&graph).unwrap();
    assert!(
        output.contains("metrics"),
        "routed edge label should not be clipped in inline output:\n{output}"
    );
}

#[test]
fn test_single_dashed_and_dotted_routed_bends_use_corner_glyphs() {
    for (style, horizontal, vertical) in
        [(EdgeStyle::Dashed, '╌', '╎'), (EdgeStyle::Dotted, '┄', '┆')]
    {
        let graph = Graph {
            direction: Direction::TopDown,
            nodes: vec![
                Node {
                    id: "A".into(),
                    label: "A".into(),
                    shape: NodeShape::Rectangle,
                    x: Some(20.0),
                    y: Some(0.0),
                    width: Some(5.0),
                    height: Some(3.0),
                },
                Node {
                    id: "B".into(),
                    label: "B".into(),
                    shape: NodeShape::Rectangle,
                    x: Some(20.0),
                    y: Some(8.0),
                    width: Some(5.0),
                    height: Some(3.0),
                },
            ],
            edges: vec![Edge {
                source: "A".into(),
                target: "B".into(),
                label: None,
                style,
                arrowhead: Arrowhead::None,
                route: Some(RoutePlan {
                    points: vec![
                        RoutePoint { x: 2.0, y: 2.0 },
                        RoutePoint { x: 6.0, y: 2.0 },
                        RoutePoint { x: 6.0, y: 5.0 },
                    ],
                    source_port: Port {
                        x: 2.0,
                        y: 2.0,
                        side: PortSide::Right,
                    },
                    target_port: Port {
                        x: 6.0,
                        y: 5.0,
                        side: PortSide::Top,
                    },
                    lane_id: None,
                    class: EdgeClass::Primary,
                    label_anchor: None,
                }),
            }],
            groups: vec![],
        };

        let output = render_to_string(&graph).unwrap();
        assert!(
            !output.contains('┼'),
            "single routed bend should not render as a crossing:\n{output}"
        );
        assert!(
            output.contains('┐'),
            "single routed bend should render as a corner:\n{output}"
        );
        assert!(
            output.contains(horizontal),
            "routed straight horizontal cells should keep the edge style:\n{output}"
        );
        assert!(
            output.contains(vertical),
            "routed straight vertical cells should keep the edge style:\n{output}"
        );
    }
}

#[test]
fn test_routed_top_port_arrowhead_has_immediate_tail_cell() {
    let graph = Graph {
        direction: Direction::TopDown,
        nodes: vec![
            Node {
                id: "A".into(),
                label: "A".into(),
                shape: NodeShape::Rectangle,
                x: Some(2.0),
                y: Some(1.0),
                width: Some(6.0),
                height: Some(3.0),
            },
            Node {
                id: "B".into(),
                label: "B".into(),
                shape: NodeShape::Rectangle,
                x: Some(10.0),
                y: Some(8.0),
                width: Some(6.0),
                height: Some(3.0),
            },
        ],
        edges: vec![Edge {
            source: "A".into(),
            target: "B".into(),
            label: None,
            style: EdgeStyle::Solid,
            arrowhead: Arrowhead::Normal,
            route: Some(RoutePlan {
                points: vec![
                    RoutePoint { x: 5.0, y: 4.0 },
                    RoutePoint { x: 20.0, y: 4.0 },
                    RoutePoint { x: 20.0, y: 7.0 },
                    RoutePoint { x: 13.0, y: 7.0 },
                ],
                source_port: Port {
                    x: 5.0,
                    y: 4.0,
                    side: PortSide::Bottom,
                },
                target_port: Port {
                    x: 13.0,
                    y: 7.0,
                    side: PortSide::Top,
                },
                lane_id: None,
                class: EdgeClass::Primary,
                label_anchor: None,
            }),
        }],
        groups: vec![],
    };
    let backend = TestBackend::new(26, 14);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    assert_eq!(buf[(13, 7)].symbol(), "▼");
    assert!(
        glyph_dirs(buf[(13, 6)].symbol().chars().next().unwrap()) & DIR_DOWN != 0,
        "tail cell above ▼ should connect downward, got {:?}",
        buf[(13, 6)].symbol()
    );
    assert_eq!(
        buf[(14, 7)].symbol(),
        " ",
        "arrowhead row should not keep a horizontal segment attached to the ▼"
    );
}

#[test]
fn test_routed_bottom_source_port_gets_vertical_stub_before_horizontal_turn() {
    let graph = Graph {
        direction: Direction::TopDown,
        nodes: vec![
            Node {
                id: "A".into(),
                label: "A".into(),
                shape: NodeShape::Rectangle,
                x: Some(2.0),
                y: Some(1.0),
                width: Some(6.0),
                height: Some(3.0),
            },
            Node {
                id: "B".into(),
                label: "B".into(),
                shape: NodeShape::Rectangle,
                x: Some(12.0),
                y: Some(8.0),
                width: Some(6.0),
                height: Some(3.0),
            },
        ],
        edges: vec![Edge {
            source: "A".into(),
            target: "B".into(),
            label: None,
            style: EdgeStyle::Solid,
            arrowhead: Arrowhead::None,
            route: Some(RoutePlan {
                points: vec![
                    RoutePoint { x: 5.0, y: 4.0 },
                    RoutePoint { x: 15.0, y: 4.0 },
                    RoutePoint { x: 15.0, y: 7.0 },
                ],
                source_port: Port {
                    x: 5.0,
                    y: 4.0,
                    side: PortSide::Bottom,
                },
                target_port: Port {
                    x: 15.0,
                    y: 7.0,
                    side: PortSide::Top,
                },
                lane_id: None,
                class: EdgeClass::Primary,
                label_anchor: None,
            }),
        }],
        groups: vec![],
    };
    let backend = TestBackend::new(24, 14);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    assert_eq!(buf[(5, 4)].symbol(), "│");
    assert!(
        glyph_dirs(buf[(5, 5)].symbol().chars().next().unwrap()) & DIR_UP != 0,
        "source stub should continue down to the turn, got {:?}",
        buf[(5, 5)].symbol()
    );
}

#[test]
fn test_dotted_telemetry_routed_edge_starts_at_source_port() {
    let graph = Graph {
        direction: Direction::TopDown,
        nodes: vec![
            Node {
                id: "A".into(),
                label: "A".into(),
                shape: NodeShape::Rectangle,
                x: Some(2.0),
                y: Some(1.0),
                width: Some(6.0),
                height: Some(3.0),
            },
            Node {
                id: "B".into(),
                label: "B".into(),
                shape: NodeShape::Rectangle,
                x: Some(12.0),
                y: Some(8.0),
                width: Some(6.0),
                height: Some(3.0),
            },
        ],
        edges: vec![Edge {
            source: "A".into(),
            target: "B".into(),
            label: None,
            style: EdgeStyle::Dotted,
            arrowhead: Arrowhead::None,
            route: Some(RoutePlan {
                points: vec![
                    RoutePoint { x: 5.0, y: 4.0 },
                    RoutePoint { x: 15.0, y: 4.0 },
                    RoutePoint { x: 15.0, y: 7.0 },
                ],
                source_port: Port {
                    x: 5.0,
                    y: 4.0,
                    side: PortSide::Bottom,
                },
                target_port: Port {
                    x: 15.0,
                    y: 7.0,
                    side: PortSide::Top,
                },
                lane_id: None,
                class: EdgeClass::Telemetry,
                label_anchor: None,
            }),
        }],
        groups: vec![],
    };
    let theme = Theme::default();
    let backend = TestBackend::new(24, 14);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame_with_theme(&graph, frame, &theme))
        .unwrap();
    let buf = terminal.backend().buffer();

    assert_eq!(buf[(5, 4)].symbol(), "┆");
    assert_eq!(buf[(5, 4)].fg, theme.muted);
    assert_ne!(
        buf[(6, 4)].symbol(),
        "┄",
        "telemetry edge should not begin as a detached horizontal run"
    );
}

#[test]
fn test_node_label_present() {
    let graph = test_graph();
    let backend = TestBackend::new(30, 14);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    let rendered: String = (6..15).map(|x| buf[(x, 2)].symbol().to_string()).collect();
    assert!(
        rendered.contains("Start"),
        "Node A label should contain 'Start', got: '{rendered}'"
    );
}

#[test]
fn test_node_b_label_present() {
    let graph = test_graph();
    let backend = TestBackend::new(30, 14);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    let rendered: String = (6..15).map(|x| buf[(x, 9)].symbol().to_string()).collect();
    assert!(
        rendered.contains("End"),
        "Node B label should contain 'End', got: '{rendered}'"
    );
}

#[test]
fn test_edge_vertical_line() {
    let graph = test_graph();
    let backend = TestBackend::new(30, 14);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    let mid_y = 5;
    let sym = buf[(10, mid_y)].symbol();
    assert!(
        sym == "│" || sym == "╎" || sym == "┆",
        "Edge should have a vertical line char at (10, {mid_y}), got: '{sym}'"
    );
}

#[test]
fn test_arrowhead_present() {
    let graph = test_graph();
    let backend = TestBackend::new(30, 14);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    let sym = buf[(10, 7)].symbol();
    assert_eq!(
        sym, "▼",
        "Arrowhead should be ▼ just above target node, got: '{sym}'"
    );
}

#[test]
fn test_edge_label_rendered() {
    let graph = test_graph();
    let backend = TestBackend::new(30, 14);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    let rendered: String = (11..15).map(|x| buf[(x, 5)].symbol().to_string()).collect();
    assert_eq!(
        rendered, "next",
        "Edge label should be 'next', got: '{rendered}'"
    );
}

#[test]
fn test_dashed_edge() {
    let graph = Graph {
        direction: Direction::TopDown,
        nodes: vec![
            Node {
                id: "A".into(),
                label: "X".into(),
                shape: NodeShape::Rectangle,
                x: Some(2.0),
                y: Some(0.0),
                width: Some(7.0),
                height: Some(3.0),
            },
            Node {
                id: "B".into(),
                label: "Y".into(),
                shape: NodeShape::Rectangle,
                x: Some(2.0),
                y: Some(7.0),
                width: Some(7.0),
                height: Some(3.0),
            },
        ],
        edges: vec![Edge {
            source: "A".into(),
            target: "B".into(),
            label: None,
            style: EdgeStyle::Dashed,
            arrowhead: Arrowhead::Normal,
            route: None,
        }],
        groups: vec![],
    };
    let backend = TestBackend::new(20, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    let sym = buf[(5, 4)].symbol();
    assert_eq!(sym, "╎", "Dashed edge should use ╎, got: '{sym}'");
}

#[test]
fn test_no_arrowhead_when_none() {
    let graph = Graph {
        direction: Direction::TopDown,
        nodes: vec![
            Node {
                id: "A".into(),
                label: "X".into(),
                shape: NodeShape::Rectangle,
                x: Some(2.0),
                y: Some(0.0),
                width: Some(7.0),
                height: Some(3.0),
            },
            Node {
                id: "B".into(),
                label: "Y".into(),
                shape: NodeShape::Rectangle,
                x: Some(2.0),
                y: Some(7.0),
                width: Some(7.0),
                height: Some(3.0),
            },
        ],
        edges: vec![Edge {
            source: "A".into(),
            target: "B".into(),
            label: None,
            style: EdgeStyle::Solid,
            arrowhead: Arrowhead::None,
            route: None,
        }],
        groups: vec![],
    };
    let backend = TestBackend::new(20, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    let sym = buf[(5, 7)].symbol();
    assert_ne!(sym, "▼", "No arrowhead expected when Arrowhead::None");
}

#[test]
fn test_diamond_label() {
    let graph = Graph {
        direction: Direction::TopDown,
        nodes: vec![Node {
            id: "D".into(),
            label: "OK?".into(),
            shape: NodeShape::Diamond,
            x: Some(3.0),
            y: Some(1.0),
            width: Some(11.0),
            height: Some(5.0),
        }],
        edges: vec![],
        groups: vec![],
    };
    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    assert_eq!(buf[(3, 1)].symbol(), " ");
    assert_eq!(buf[(3, 1)].bg, Theme::default().diamond.fill);
    assert_eq!(buf[(13, 5)].symbol(), " ");
    assert_eq!(buf[(13, 5)].bg, Theme::default().diamond.fill);
    let rendered: String = (3..14).map(|x| buf[(x, 3)].symbol().to_string()).collect();
    assert!(
        rendered.contains("◆ OK?"),
        "Diamond label should contain '◆ OK?', got: '{rendered}'"
    );
}

#[test]
fn test_horizontal_edge_with_arrowhead() {
    let graph = Graph {
        direction: Direction::LeftRight,
        nodes: vec![
            Node {
                id: "L".into(),
                label: "L".into(),
                shape: NodeShape::Rectangle,
                x: Some(0.0),
                y: Some(1.0),
                width: Some(7.0),
                height: Some(3.0),
            },
            Node {
                id: "R".into(),
                label: "R".into(),
                shape: NodeShape::Rectangle,
                x: Some(15.0),
                y: Some(1.0),
                width: Some(7.0),
                height: Some(3.0),
            },
        ],
        edges: vec![Edge {
            source: "L".into(),
            target: "R".into(),
            label: None,
            style: EdgeStyle::Solid,
            arrowhead: Arrowhead::Normal,
            route: None,
        }],
        groups: vec![],
    };
    let backend = TestBackend::new(30, 8);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    let sym = buf[(14, 2)].symbol();
    assert_eq!(sym, "▶", "Horizontal arrowhead should be ▶, got: '{sym}'");

    let mid_sym = buf[(10, 2)].symbol();
    assert_eq!(
        mid_sym, "─",
        "Horizontal edge should use ─, got: '{mid_sym}'"
    );
}

#[test]
fn test_shared_branch_uses_junction_glyph() {
    let graph = Graph {
        direction: Direction::TopDown,
        nodes: vec![
            Node {
                id: "A".into(),
                label: "A".into(),
                shape: NodeShape::Rectangle,
                x: Some(10.0),
                y: Some(0.0),
                width: Some(10.0),
                height: Some(3.0),
            },
            Node {
                id: "B".into(),
                label: "B".into(),
                shape: NodeShape::Rectangle,
                x: Some(6.0),
                y: Some(10.0),
                width: Some(10.0),
                height: Some(3.0),
            },
            Node {
                id: "C".into(),
                label: "C".into(),
                shape: NodeShape::Rectangle,
                x: Some(14.0),
                y: Some(10.0),
                width: Some(10.0),
                height: Some(3.0),
            },
        ],
        edges: vec![
            Edge {
                source: "A".into(),
                target: "B".into(),
                label: None,
                style: EdgeStyle::Solid,
                arrowhead: Arrowhead::Normal,
                route: None,
            },
            Edge {
                source: "A".into(),
                target: "C".into(),
                label: None,
                style: EdgeStyle::Solid,
                arrowhead: Arrowhead::Normal,
                route: None,
            },
        ],
        groups: vec![],
    };

    let backend = TestBackend::new(40, 18);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    assert_eq!(buf[(15, 6)].symbol(), "┴");
}

#[test]
fn test_skips_nodes_without_layout() {
    let graph = Graph {
        direction: Direction::TopDown,
        nodes: vec![Node {
            id: "X".into(),
            label: "Ghost".into(),
            shape: NodeShape::Rectangle,
            x: None,
            y: None,
            width: None,
            height: None,
        }],
        edges: vec![],
        groups: vec![],
    };
    let backend = TestBackend::new(20, 8);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(&graph, frame))
        .unwrap();
}

#[test]
fn telemetry_edges_render_with_muted_foreground() {
    let mut graph = diaview::parser::mermaid::parse(
        diaview::testdata::fixtures::phase15_telemetry_overlay_mermaid(),
    )
    .unwrap();
    diaview::layout::layout(&mut graph);
    let theme = Theme::default();
    let (width, height) = graph_bounds(&graph);
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame_with_theme(&graph, frame, &theme))
        .unwrap();
    let buf = terminal.backend().buffer();
    let telemetry_glyphs = ["┄", "┆", "▶", "▼", "▲", "◀"];

    let has_muted_telemetry_glyph = (0..buf.area.height).any(|y| {
        (0..buf.area.width).any(|x| {
            telemetry_glyphs.contains(&buf[(x, y)].symbol()) && buf[(x, y)].fg == theme.muted
        })
    });
    assert!(
        has_muted_telemetry_glyph,
        "at least one telemetry glyph should render with muted foreground"
    );
}

fn dump_graph(name: &str, graph: &Graph, width: u16, height: u16) {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| render_to_frame(graph, frame))
        .unwrap();
    let buf = terminal.backend().buffer();

    println!("\n=== {name} render dump {width}x{height} ===");
    for y in 0..buf.area.height {
        let mut line = String::new();
        for x in 0..buf.area.width {
            line.push_str(buf[(x, y)].symbol());
        }
        println!("|{line}|");
    }
    println!("=== end ===\n");
}

#[test]
fn dump_default_graph() {
    let input = r#"graph TD
A[Start] --> B{Decision}
B -->|yes| C(Process)
B -->|no| D((End))
C --> D
"#;
    let mut graph = diaview::parser::mermaid::parse(input).unwrap();
    diaview::layout::layout(&mut graph);

    dump_graph("default", &graph, 120, 50);

    println!("nodes:");
    for n in &graph.nodes {
        println!(
            "  {} {:?} x={:?} y={:?} w={:?} h={:?} label={:?}",
            n.id, n.shape, n.x, n.y, n.width, n.height, n.label
        );
    }
    println!("edges:");
    for e in &graph.edges {
        println!("  {} -> {} label={:?}", e.source, e.target, e.label);
    }
}

#[test]
fn dump_complex_architecture_graph() {
    let mut graph = diaview::parser::mermaid::parse(
        diaview::testdata::fixtures::complex_architecture_mermaid(),
    )
    .unwrap();
    diaview::layout::layout(&mut graph);

    let (width, height) = graph_bounds(&graph);
    println!(
        "complex architecture fixture: {} nodes, {} edges, bounds {width}x{height}",
        graph.nodes.len(),
        graph.edges.len()
    );
    dump_graph("complex architecture", &graph, width, height);
}
