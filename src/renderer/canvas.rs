use std::io::{self, stdout};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::model::{Arrowhead, Direction, EdgeStyle, Graph, Node, NodeShape};

/// Color per node shape for visual distinction.
fn shape_color(shape: &NodeShape) -> Color {
    match shape {
        NodeShape::RoundedRect => Color::Cyan,
        NodeShape::Rectangle => Color::Blue,
        NodeShape::Diamond => Color::Yellow,
        NodeShape::Circle => Color::Green,
    }
}

/// Border type per node shape.
fn shape_border_type(shape: &NodeShape) -> BorderType {
    match shape {
        NodeShape::RoundedRect => BorderType::Rounded,
        NodeShape::Rectangle => BorderType::Plain,
        NodeShape::Diamond => BorderType::Double,
        NodeShape::Circle => BorderType::Rounded,
    }
}

/// Full terminal render — sets up crossterm, renders, waits for keypress, cleans up.
pub fn render(graph: &Graph) -> io::Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|frame| render_to_frame(graph, frame))?;

    // Wait for any keypress to exit
    loop {
        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                break;
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

/// Testable inner function — draws graph onto a Frame.
pub fn render_to_frame(graph: &Graph, frame: &mut Frame) {
    let area = frame.area();

    // Render nodes first
    for node in &graph.nodes {
        render_node(node, frame, area);
    }

    // Render edges on top so arrowheads aren't overwritten by borders
    render_edges(graph, frame, area);
}

/// Render a single node as a bordered Block with a centered label Paragraph.
fn render_node(node: &Node, frame: &mut Frame, _area: Rect) {
    if node.id.starts_with("__dummy") {
        return; // skip rendering dummy layout nodes completely
    }

    let (x, y, w, h) = match (node.x, node.y, node.width, node.height) {
        (Some(x), Some(y), Some(w), Some(h)) => (x, y, w, h),
        _ => return, // skip nodes without layout
    };

    let rect = Rect::new(x as u16, y as u16, w as u16, h as u16);

    let color = shape_color(&node.shape);

    // All shapes use bordered Block — Diamond uses Double borders for visual distinction,
    // Circle and RoundedRect use Rounded, Rectangle uses Plain.
    let border_type = shape_border_type(&node.shape);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(color));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    if inner.width > 0 && inner.height > 0 {
        // For diamonds, prefix label with ◇ to indicate the shape
        let display_label = match node.shape {
            NodeShape::Diamond => format!("◇ {}", node.label),
            _ => node.label.clone(),
        };
        let label = center_label(&display_label, inner);
        frame.render_widget(label, inner);
    }
}

/// Create a Paragraph that centers the text both horizontally and vertically within the area.
fn center_label(text: &str, area: Rect) -> Paragraph<'_> {
    // Vertical centering: compute top padding as blank lines
    let v_pad = if area.height > 1 {
        (area.height.saturating_sub(1)) / 2
    } else {
        0
    };

    let mut lines: Vec<Line<'_>> = Vec::new();
    for _ in 0..v_pad {
        lines.push(Line::from(""));
    }
    lines.push(Line::from(text).centered());

    Paragraph::new(Text::from(lines))
}



/// Render all edges in the graph.
fn render_edges(graph: &Graph, frame: &mut Frame, _area: Rect) {
    for edge in &graph.edges {
        let source = graph.nodes.iter().find(|n| n.id == edge.source);
        let target = graph.nodes.iter().find(|n| n.id == edge.target);

        if let (Some(src), Some(tgt)) = (source, target) {
            render_edge(src, tgt, edge, frame, &graph.direction, &graph.nodes);
        }
    }
}

/// Compute the center of a node.
fn node_center(node: &Node) -> Option<(u16, u16)> {
    match (node.x, node.y, node.width, node.height) {
        (Some(x), Some(y), Some(w), Some(h)) => Some(((x + w / 2.0) as u16, (y + h / 2.0) as u16)),
        _ => None,
    }
}

/// Compute the connection point just outside the border of a node.
/// Uses layout direction to pick the correct side:
/// - TopDown: source exits from bottom, target enters from top
/// - LeftRight: source exits from right, target enters from left
/// Falls back to "toward center" heuristic only when nodes are on the same layer.
fn connection_point(
    node: &Node,
    toward_x: u16,
    toward_y: u16,
    direction: &Direction,
    is_source: bool,
) -> Option<(u16, u16)> {
    let (x, y, w, h) = match (node.x, node.y, node.width, node.height) {
        (Some(x), Some(y), Some(w), Some(h)) => (x as u16, y as u16, w as u16, h as u16),
        _ => return None,
    };

    let cx = x + w / 2;
    let cy = y + h / 2;

    if node.id.starts_with("__dummy") {
        return Some((cx, cy));
    }

    match direction {
        Direction::TopDown => {
            // Check if nodes are on roughly the same layer (same y region)
            let dy = toward_y as i32 - cy as i32;
            if is_source && dy > 0 {
                // Source exits from bottom center
                Some((cx, y + h))
            } else if !is_source && dy < 0 {
                // Target enters from top center (one cell above the border)
                Some((cx, y.saturating_sub(1)))
            } else {
                // Same layer or unusual arrangement — fall back to heuristic
                connection_point_heuristic(x, y, w, h, cx, cy, toward_x, toward_y)
            }
        }
        Direction::LeftRight => {
            let dx = toward_x as i32 - cx as i32;
            if is_source && dx > 0 {
                // Source exits from right center
                Some((x + w, cy))
            } else if !is_source && dx < 0 {
                // Target enters from left center
                Some((x.saturating_sub(1), cy))
            } else {
                connection_point_heuristic(x, y, w, h, cx, cy, toward_x, toward_y)
            }
        }
    }
}

/// Fallback heuristic: pick the border side closest to the target point.
fn connection_point_heuristic(
    x: u16, y: u16, w: u16, h: u16,
    cx: u16, cy: u16,
    toward_x: u16, toward_y: u16,
) -> Option<(u16, u16)> {
    let dx = toward_x as i32 - cx as i32;
    let dy = toward_y as i32 - cy as i32;

    if dx.abs() * (h as i32) > dy.abs() * (w as i32) {
        if dx > 0 {
            Some((x + w, cy))
        } else {
            Some((x.saturating_sub(1), cy))
        }
    } else {
        if dy > 0 {
            Some((cx, y + h))
        } else {
            Some((cx, y.saturating_sub(1)))
        }
    }
}

/// Edge character for the given style.
fn edge_h_char(style: &EdgeStyle) -> char {
    match style {
        EdgeStyle::Solid => '─',
        EdgeStyle::Dashed => '╌',
        EdgeStyle::Dotted => '┄',
    }
}

fn edge_v_char(style: &EdgeStyle) -> char {
    match style {
        EdgeStyle::Solid => '│',
        EdgeStyle::Dashed => '╎',
        EdgeStyle::Dotted => '┆',
    }
}

/// Arrowhead character based on direction.
fn arrowhead_char(dx: i32, dy: i32, arrowhead: &Arrowhead) -> Option<char> {
    match arrowhead {
        Arrowhead::None => None,
        Arrowhead::Normal | Arrowhead::Open => {
            if dy.abs() >= dx.abs() {
                if dy > 0 {
                    Some('▼')
                } else {
                    Some('▲')
                }
            } else if dx > 0 {
                Some('▶')
            } else {
                Some('◀')
            }
        }
    }
}

/// Check if a cell position is inside any node's bounding box.
fn is_inside_any_node(px: u16, py: u16, nodes: &[Node]) -> bool {
    for node in nodes {
        if let (Some(nx), Some(ny), Some(nw), Some(nh)) =
            (node.x, node.y, node.width, node.height)
        {
            let nx = nx as u16;
            let ny = ny as u16;
            let nw = nw as u16;
            let nh = nh as u16;
            if px >= nx && px < nx + nw && py >= ny && py < ny + nh {
                return true;
            }
        }
    }
    false
}

/// Render a single edge between two nodes using orthogonal routing.
fn render_edge(
    src: &Node,
    tgt: &Node,
    edge: &crate::model::Edge,
    frame: &mut Frame,
    direction: &Direction,
    all_nodes: &[Node],
) {
    let src_center = match node_center(src) {
        Some(c) => c,
        None => return,
    };
    let tgt_center = match node_center(tgt) {
        Some(c) => c,
        None => return,
    };

    let start = match connection_point(src, tgt_center.0, tgt_center.1, direction, true) {
        Some(p) => p,
        None => return,
    };
    let end = match connection_point(tgt, src_center.0, src_center.1, direction, false) {
        Some(p) => p,
        None => return,
    };

    let buf = frame.buffer_mut();
    let buf_area = buf.area;

    let edge_style = Style::default().fg(Color::DarkGray);

    // Helper: set a cell only if it's in bounds and NOT inside any node's bounding box
    let set_cell = |buf: &mut ratatui::buffer::Buffer, px: u16, py: u16, ch: char, style: Style, nodes: &[Node]| {
        if px < buf.area.x + buf.area.width
            && py < buf.area.y + buf.area.height
            && py >= buf.area.y
            && !is_inside_any_node(px, py, nodes)
        {
            buf[(px, py)].set_char(ch).set_style(style);
        }
    };

    let min_max = |a: u16, b: u16| if a < b { (a, b) } else { (b, a) };

    let arrow_dx;
    let arrow_dy;
    let label_x;
    let label_y;

    if *direction == Direction::TopDown {
        let mid_y = (start.1 + end.1) / 2;

        // 1. Vertical from start to mid_y
        let (y0, y1) = min_max(start.1, mid_y);
        for y in y0..=y1 {
            set_cell(buf, start.0, y, edge_v_char(&edge.style), edge_style, all_nodes);
        }

        // 2. Horizontal from start.0 to end.0 at mid_y
        if start.0 != end.0 {
            let (x0, x1) = min_max(start.0, end.0);
            for x in x0..=x1 {
                set_cell(buf, x, mid_y, edge_h_char(&edge.style), edge_style, all_nodes);
            }
            // Corners
            if edge.style == EdgeStyle::Solid {
                let corner1 = if end.0 > start.0 {
                    if mid_y >= start.1 { '└' } else { '┌' }
                } else {
                    if mid_y >= start.1 { '┘' } else { '┐' }
                };
                let corner2 = if start.0 < end.0 {
                    if end.1 >= mid_y { '┐' } else { '┘' }
                } else {
                    if end.1 >= mid_y { '┌' } else { '└' }
                };
                set_cell(buf, start.0, mid_y, corner1, edge_style, all_nodes);
                set_cell(buf, end.0, mid_y, corner2, edge_style, all_nodes);
            } else {
                let c = edge_h_char(&edge.style);
                set_cell(buf, start.0, mid_y, c, edge_style, all_nodes);
                set_cell(buf, end.0, mid_y, c, edge_style, all_nodes);
            }
        }

        // 3. Vertical from mid_y to end.1
        let (y0, y1) = min_max(mid_y, end.1);
        for y in y0..=y1 {
            set_cell(buf, end.0, y, edge_v_char(&edge.style), edge_style, all_nodes);
        }

        arrow_dx = 0;
        arrow_dy = end.1 as i32 - mid_y as i32;

        label_x = (start.0 + end.0) / 2;
        label_y = mid_y;
    } else {
        let mid_x = (start.0 + end.0) / 2;

        // 1. Horizontal from start to mid_x
        let (x0, x1) = min_max(start.0, mid_x);
        for x in x0..=x1 {
            set_cell(buf, x, start.1, edge_h_char(&edge.style), edge_style, all_nodes);
        }

        // 2. Vertical from start.1 to end.1 at mid_x
        if start.1 != end.1 {
            let (y0, y1) = min_max(start.1, end.1);
            for y in y0..=y1 {
                set_cell(buf, mid_x, y, edge_v_char(&edge.style), edge_style, all_nodes);
            }
            // Corners
            if edge.style == EdgeStyle::Solid {
                let corner1 = if end.1 > start.1 {
                    if mid_x >= start.0 { '┐' } else { '┌' }
                } else {
                    if mid_x >= start.0 { '┘' } else { '└' }
                };
                let corner2 = if start.1 < end.1 {
                    if end.0 >= mid_x { '└' } else { '┘' }
                } else {
                    if end.0 >= mid_x { '┌' } else { '┐' }
                };
                set_cell(buf, mid_x, start.1, corner1, edge_style, all_nodes);
                set_cell(buf, mid_x, end.1, corner2, edge_style, all_nodes);
            } else {
                let c = edge_v_char(&edge.style);
                set_cell(buf, mid_x, start.1, c, edge_style, all_nodes);
                set_cell(buf, mid_x, end.1, c, edge_style, all_nodes);
            }
        }

        // 3. Horizontal from mid_x to end.0
        let (x0, x1) = min_max(mid_x, end.0);
        for x in x0..=x1 {
            set_cell(buf, x, end.1, edge_h_char(&edge.style), edge_style, all_nodes);
        }

        arrow_dx = end.0 as i32 - mid_x as i32;
        arrow_dy = 0;

        label_x = mid_x;
        label_y = (start.1 + end.1) / 2;
    }

    // Arrowhead at the end point — always draw (even near nodes) so it's visible
    if let Some(arrow) = arrowhead_char(arrow_dx, arrow_dy, &edge.arrowhead) {
        if end.0 < buf_area.x + buf_area.width && end.1 < buf_area.y + buf_area.height {
            buf[(end.0, end.1)].set_char(arrow).set_style(Style::default().fg(Color::White));
        }
    }

    // Edge label at midpoint, offset perpendicular to the middle segment
    if let Some(ref label) = edge.label {
        let actually_horizontal = if *direction == Direction::TopDown {
            start.0 != end.0
        } else {
            start.1 == end.1
        };

        let (lx, ly) = if actually_horizontal {
            // Offset label above the horizontal line
            let start_x = label_x.saturating_sub((label.len() / 2) as u16);
            (start_x, label_y.saturating_sub(1))
        } else {
            // Offset label to the right of the vertical line
            (label_x + 1, label_y)
        };

        // Clear background behind label text, then draw the label
        for (i, ch) in label.chars().enumerate() {
            let px = lx + i as u16;
            if px < buf_area.x + buf_area.width && ly < buf_area.y + buf_area.height {
                // Clear cell first (reset to space), then set label char
                buf[(px, ly)].set_char(' ');
                buf[(px, ly)].set_char(ch).set_style(Style::default().fg(Color::Gray));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Helper to build a graph with laid-out nodes.
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
            }],
        }
    }

    #[test]
    fn test_rounded_rect_corners() {
        let graph = test_graph();
        let backend = TestBackend::new(30, 14);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
        let buf = terminal.backend().buffer();

        // RoundedRect corners: ╭ at top-left, ╮ at top-right, ╰ at bottom-left, ╯ at bottom-right
        // Node A at (5, 1) with width 11, height 3
        assert_eq!(buf[(5, 1)].symbol(), "╭", "top-left corner of node A");
        assert_eq!(buf[(15, 1)].symbol(), "╮", "top-right corner of node A");
        assert_eq!(buf[(5, 3)].symbol(), "╰", "bottom-left corner of node A");
        assert_eq!(buf[(15, 3)].symbol(), "╯", "bottom-right corner of node A");
    }

    #[test]
    fn test_rectangle_corners() {
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
        };
        let backend = TestBackend::new(20, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
        let buf = terminal.backend().buffer();

        // Plain rect corners: ┌ ┐ └ ┘
        assert_eq!(buf[(2, 1)].symbol(), "┌");
        assert_eq!(buf[(10, 1)].symbol(), "┐");
        assert_eq!(buf[(2, 3)].symbol(), "└");
        assert_eq!(buf[(10, 3)].symbol(), "┘");
    }

    #[test]
    fn test_node_label_present() {
        let graph = test_graph();
        let backend = TestBackend::new(30, 14);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
        let buf = terminal.backend().buffer();

        // Node A label "Start" should appear inside the node (y=2 is the middle row, inner area)
        // Inner area: x=6..15, y=2
        // "Start" is 5 chars, inner width=9, so centered at x = 6 + (9-5)/2 = 8
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
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
        let buf = terminal.backend().buffer();

        // Node B at y=8, inner row at y=9
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
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
        let buf = terminal.backend().buffer();

        // Edge goes from node A bottom (y=3) to node B top (y=8)
        // Center x = 5 + 11/2 = 10
        // The vertical line should be somewhere between y=3 and y=8 at x=10
        // Check a midpoint
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
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
        let buf = terminal.backend().buffer();

        // Arrow should be one cell above node B top border: (10, 7)
        let sym = buf[(10, 7)].symbol();
        assert_eq!(sym, "▼", "Arrowhead should be ▼ just above target node, got: '{sym}'");
    }

    #[test]
    fn test_edge_label_rendered() {
        let graph = test_graph();
        let backend = TestBackend::new(30, 14);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
        let buf = terminal.backend().buffer();

        // Edge label "next" should appear near the midpoint of the edge
        // midpoint y = (3+8)/2 = 5, x = 10, label starts at x=11
        let rendered: String = (11..15).map(|x| buf[(x, 5)].symbol().to_string()).collect();
        assert_eq!(rendered, "next", "Edge label should be 'next', got: '{rendered}'");
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
            }],
        };
        let backend = TestBackend::new(20, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
        let buf = terminal.backend().buffer();

        // Midpoint of edge: x=5, y=4
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
            }],
        };
        let backend = TestBackend::new(20, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
        let buf = terminal.backend().buffer();

        // Target top at (5, 7) should NOT have an arrowhead
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
        };
        let backend = TestBackend::new(20, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
        let buf = terminal.backend().buffer();

        // Diamond now renders as a Double-bordered Block with "◇ OK?" label
        // Double borders: ╔ at top-left (3,1), ╗ at top-right (13,1)
        assert_eq!(buf[(3, 1)].symbol(), "╔", "Diamond top-left should be ╔");
        assert_eq!(buf[(13, 1)].symbol(), "╗", "Diamond top-right should be ╗");
        assert_eq!(buf[(3, 5)].symbol(), "╚", "Diamond bottom-left should be ╚");
        assert_eq!(buf[(13, 5)].symbol(), "╝", "Diamond bottom-right should be ╝");

        // Inner area: x=4..13, y=2..5. Label "◇ OK?" (5 display chars) centered.
        // Inner width = 9, so padded start = 4 + (9-5)/2 = 6
        // Check the label contains "OK?" somewhere in the inner area at the middle row (y=3)
        let rendered: String = (4..13).map(|x| buf[(x, 3)].symbol().to_string()).collect();
        assert!(
            rendered.contains("OK?"),
            "Diamond label should contain 'OK?', got: '{rendered}'"
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
            }],
        };
        let backend = TestBackend::new(30, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
        let buf = terminal.backend().buffer();

        // Horizontal edge at y=2, arrowhead one cell left of node R's left border: (14, 2)
        let sym = buf[(14, 2)].symbol();
        assert_eq!(sym, "▶", "Horizontal arrowhead should be ▶, got: '{sym}'");

        // Check horizontal line somewhere in the middle
        let mid_sym = buf[(10, 2)].symbol();
        assert_eq!(mid_sym, "─", "Horizontal edge should use ─, got: '{mid_sym}'");
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
        };
        let backend = TestBackend::new(20, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        // Should not panic
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
    }
}
