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

use crate::model::{Arrowhead, EdgeStyle, Graph, Node, NodeShape};

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
        NodeShape::Diamond => BorderType::Rounded,
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
    let (x, y, w, h) = match (node.x, node.y, node.width, node.height) {
        (Some(x), Some(y), Some(w), Some(h)) => (x, y, w, h),
        _ => return, // skip nodes without layout
    };

    let rect = Rect::new(x as u16, y as u16, w as u16, h as u16);

    let color = shape_color(&node.shape);

    match node.shape {
        NodeShape::Diamond => {
            render_diamond(node, frame, rect, color);
        }
        NodeShape::Circle => {
            render_circle(node, frame, rect, color);
        }
        _ => {
            // Rectangle / RoundedRect — use Block widget
            let border_type = shape_border_type(&node.shape);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(border_type)
                .border_style(Style::default().fg(color));

            // Label centered inside the block (accounting for border padding)
            let inner = block.inner(rect);
            frame.render_widget(block, rect);

            if inner.width > 0 && inner.height > 0 {
                let label = center_label(&node.label, inner);
                frame.render_widget(label, inner);
            }
        }
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

/// Render a diamond shape manually using characters.
fn render_diamond(node: &Node, frame: &mut Frame, rect: Rect, color: Color) {
    let buf = frame.buffer_mut();
    let cx = rect.x + rect.width / 2;
    let cy = rect.y + rect.height / 2;
    let half_w = (rect.width / 2) as i16;
    let half_h = (rect.height / 2) as i16;

    // Draw the diamond outline
    for row in 0..rect.height {
        let dy = (row as i16 - half_h).unsigned_abs() as u16;
        // scale horizontal span by height/width
        let span = if half_h > 0 {
            ((half_w as u32 * (half_h as u32 - dy as u32)) / half_h as u32) as u16
        } else {
            0
        };

        let left = cx.saturating_sub(span);
        let right = cx + span;
        let y = rect.y + row;

        if y >= buf.area.y + buf.area.height {
            continue;
        }

        let style = Style::default().fg(color);

        if row == 0 || row == rect.height - 1 {
            // Top/bottom point
            if cx < buf.area.x + buf.area.width {
                buf[(cx, y)].set_char('◆').set_style(style);
            }
        } else {
            // Left and right edges
            if left >= buf.area.x && left < buf.area.x + buf.area.width {
                buf[(left, y)].set_char('/').set_style(style);
            }
            if right >= buf.area.x && right < buf.area.x + buf.area.width && right != left {
                buf[(right, y)].set_char('\\').set_style(style);
            }
        }
    }

    // Center label
    let label_y = cy;
    let label_len = node.label.len() as u16;
    let label_x = cx.saturating_sub(label_len / 2);
    for (i, ch) in node.label.chars().enumerate() {
        let px = label_x + i as u16;
        if px < buf.area.x + buf.area.width && label_y < buf.area.y + buf.area.height {
            buf[(px, label_y)].set_char(ch);
        }
    }
}

/// Render a circle shape using rounded borders (visually approximated).
fn render_circle(node: &Node, frame: &mut Frame, rect: Rect, color: Color) {
    // Use a rounded rect as a circle approximation in the terminal
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    if inner.width > 0 && inner.height > 0 {
        let label = center_label(&node.label, inner);
        frame.render_widget(label, inner);
    }
}

/// Render all edges in the graph.
fn render_edges(graph: &Graph, frame: &mut Frame, _area: Rect) {
    for edge in &graph.edges {
        let source = graph.nodes.iter().find(|n| n.id == edge.source);
        let target = graph.nodes.iter().find(|n| n.id == edge.target);

        if let (Some(src), Some(tgt)) = (source, target) {
            render_edge(src, tgt, edge, frame);
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

/// Compute the connection point just outside the border of a node toward a target point.
/// This ensures arrowheads and edge lines don't land on the border itself.
fn connection_point(node: &Node, toward_x: u16, toward_y: u16) -> Option<(u16, u16)> {
    let (x, y, w, h) = match (node.x, node.y, node.width, node.height) {
        (Some(x), Some(y), Some(w), Some(h)) => (x as u16, y as u16, w as u16, h as u16),
        _ => return None,
    };

    let cx = x + w / 2;
    let cy = y + h / 2;

    let dx = toward_x as i32 - cx as i32;
    let dy = toward_y as i32 - cy as i32;

    // Determine which border side to connect to, place point one cell outside the border
    if dx.abs() * (h as i32) > dy.abs() * (w as i32) {
        // Horizontal dominates
        if dx > 0 {
            Some((x + w, cy)) // one cell right of right border
        } else {
            Some((x.saturating_sub(1), cy)) // one cell left of left border
        }
    } else {
        // Vertical dominates
        if dy > 0 {
            Some((cx, y + h)) // one cell below bottom border
        } else {
            Some((cx, y.saturating_sub(1))) // one cell above top border
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

/// Render a single edge between two nodes using orthogonal routing.
fn render_edge(src: &Node, tgt: &Node, edge: &crate::model::Edge, frame: &mut Frame) {
    let src_center = match node_center(src) {
        Some(c) => c,
        None => return,
    };
    let tgt_center = match node_center(tgt) {
        Some(c) => c,
        None => return,
    };

    let start = match connection_point(src, tgt_center.0, tgt_center.1) {
        Some(p) => p,
        None => return,
    };
    let end = match connection_point(tgt, src_center.0, src_center.1) {
        Some(p) => p,
        None => return,
    };

    let buf = frame.buffer_mut();
    let buf_area = buf.area;

    let edge_style = Style::default().fg(Color::DarkGray);

    let dx = end.0 as i32 - start.0 as i32;
    let dy = end.1 as i32 - start.1 as i32;

    // Orthogonal routing: go vertical first, then horizontal (for top-down),
    // or horizontal first then vertical (for left-right).
    // Simple L-shaped routing.

    let mid_x;
    let mid_y;

    if dy.abs() >= dx.abs() {
        // Primarily vertical: go vertical to midpoint, then horizontal, then vertical
        mid_x = start.0;
        mid_y = end.1;

        // Vertical segment from start
        let (y0, y1) = if start.1 <= end.1 {
            (start.1, end.1)
        } else {
            (end.1, start.1)
        };

        // Draw vertical line at start.x
        for y in y0..=y1 {
            if mid_x < buf_area.x + buf_area.width && y < buf_area.y + buf_area.height && y >= buf_area.y {
                buf[(mid_x, y)].set_char(edge_v_char(&edge.style)).set_style(edge_style);
            }
        }

        // Draw horizontal line at end.y from start.x to end.x
        if mid_x != end.0 {
            let (x0, x1) = if mid_x <= end.0 {
                (mid_x, end.0)
            } else {
                (end.0, mid_x)
            };
            for x in x0..=x1 {
                if x < buf_area.x + buf_area.width && mid_y < buf_area.y + buf_area.height && mid_y >= buf_area.y {
                    buf[(x, mid_y)].set_char(edge_h_char(&edge.style)).set_style(edge_style);
                }
            }
            // Corner at the bend
            if mid_x < buf_area.x + buf_area.width && mid_y < buf_area.y + buf_area.height {
                buf[(mid_x, mid_y)].set_char('┼').set_style(edge_style);
            }
        }
    } else {
        // Primarily horizontal: go horizontal first, then vertical
        mid_x = end.0;
        mid_y = start.1;

        // Horizontal line at start.y
        let (x0, x1) = if start.0 <= end.0 {
            (start.0, end.0)
        } else {
            (end.0, start.0)
        };
        for x in x0..=x1 {
            if x < buf_area.x + buf_area.width && mid_y < buf_area.y + buf_area.height && mid_y >= buf_area.y {
                buf[(x, mid_y)].set_char(edge_h_char(&edge.style)).set_style(edge_style);
            }
        }

        // Vertical line at end.x
        if mid_y != end.1 {
            let (y0, y1) = if mid_y <= end.1 {
                (mid_y, end.1)
            } else {
                (end.1, mid_y)
            };
            for y in y0..=y1 {
                if mid_x < buf_area.x + buf_area.width && y < buf_area.y + buf_area.height && y >= buf_area.y {
                    buf[(mid_x, y)].set_char(edge_v_char(&edge.style)).set_style(edge_style);
                }
            }
            // Corner at (end.x, start.y)
            if mid_x < buf_area.x + buf_area.width && mid_y < buf_area.y + buf_area.height {
                buf[(mid_x, mid_y)].set_char('┼').set_style(edge_style);
            }
        }
    }

    // Arrowhead at the end point
    let arrow_dx = end.0 as i32 - start.0 as i32;
    let arrow_dy = end.1 as i32 - start.1 as i32;
    if let Some(arrow) = arrowhead_char(arrow_dx, arrow_dy, &edge.arrowhead) {
        if end.0 < buf_area.x + buf_area.width && end.1 < buf_area.y + buf_area.height {
            buf[(end.0, end.1)].set_char(arrow).set_style(Style::default().fg(Color::White));
        }
    }

    // Edge label at midpoint
    if let Some(ref label) = edge.label {
        let label_x = ((start.0 as i32 + end.0 as i32) / 2) as u16;
        let label_y = ((start.1 as i32 + end.1 as i32) / 2) as u16;

        // Place label text, offset by 1 to the right to avoid overwriting the line
        let lx = label_x + 1;
        for (i, ch) in label.chars().enumerate() {
            let px = lx + i as u16;
            if px < buf_area.x + buf_area.width && label_y < buf_area.y + buf_area.height {
                buf[(px, label_y)].set_char(ch).set_style(Style::default().fg(Color::Gray));
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

        // Center of diamond: x=8, y=3. Label "OK?" starts around x=7
        let rendered: String = (7..10).map(|x| buf[(x, 3)].symbol().to_string()).collect();
        assert_eq!(rendered, "OK?", "Diamond label should read 'OK?', got: '{rendered}'");
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
