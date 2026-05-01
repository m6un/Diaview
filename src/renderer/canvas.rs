use std::io::{self, stdout};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{CrosstermBackend, TestBackend},
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

/// Render the whole graph into terminal scrollback instead of taking over the screen.
///
/// This is useful for large diagrams that exceed the current viewport. It renders to an
/// in-memory Ratatui backend sized to the graph bounds, then prints the buffer as plain text.
pub fn render_inline(graph: &Graph) -> io::Result<()> {
    let output = render_to_string(graph)?;
    println!("{output}");
    Ok(())
}

/// Render the whole graph into a string. Exposed for tests and inline mode.
pub fn render_to_string(graph: &Graph) -> io::Result<String> {
    let (width, height) = graph_bounds(graph);
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| render_to_frame(graph, frame))?;
    let buf = terminal.backend().buffer();

    let mut out = String::new();
    for y in 0..buf.area.height {
        let last_non_space = (0..buf.area.width)
            .rev()
            .find(|&x| buf[(x, y)].symbol() != " ");

        let Some(last_x) = last_non_space else {
            out.push('\n');
            continue;
        };

        let mut current_fg = Color::Reset;
        for x in 0..=last_x {
            let cell = &buf[(x, y)];
            if cell.fg != current_fg {
                out.push_str(&ansi_fg(cell.fg));
                current_fg = cell.fg;
            }
            out.push_str(cell.symbol());
        }
        if current_fg != Color::Reset {
            out.push_str("\x1b[39m");
        }
        out.push('\n');
    }
    Ok(out)
}

fn ansi_fg(color: Color) -> String {
    match color {
        Color::Reset => "\x1b[39m".to_string(),
        Color::Black => "\x1b[30m".to_string(),
        Color::Red => "\x1b[31m".to_string(),
        Color::Green => "\x1b[32m".to_string(),
        Color::Yellow => "\x1b[33m".to_string(),
        Color::Blue => "\x1b[34m".to_string(),
        Color::Magenta => "\x1b[35m".to_string(),
        Color::Cyan => "\x1b[36m".to_string(),
        Color::Gray => "\x1b[37m".to_string(),
        Color::DarkGray => "\x1b[90m".to_string(),
        Color::LightRed => "\x1b[91m".to_string(),
        Color::LightGreen => "\x1b[92m".to_string(),
        Color::LightYellow => "\x1b[93m".to_string(),
        Color::LightBlue => "\x1b[94m".to_string(),
        Color::LightMagenta => "\x1b[95m".to_string(),
        Color::LightCyan => "\x1b[96m".to_string(),
        Color::White => "\x1b[97m".to_string(),
        Color::Rgb(r, g, b) => format!("\x1b[38;2;{r};{g};{b}m"),
        Color::Indexed(i) => format!("\x1b[38;5;{i}m"),
    }
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

    (max_x.ceil() as u16, max_y.ceil() as u16)
}

/// Testable inner function — draws graph onto a Frame.
pub fn render_to_frame(graph: &Graph, frame: &mut Frame) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    // Render crisp box-based nodes first.
    for node in &graph.nodes {
        render_node(node, frame, area);
    }

    // Render edges on top so arrowheads aren't overwritten by borders.
    render_edges(graph, frame, area);


}

/// Render a single node as a clean terminal-native boxed shape.
fn render_node(node: &Node, frame: &mut Frame, area: Rect) {
    if node.id.starts_with("__dummy") {
        return;
    }

    let (x, y, w, h) = match (node.x, node.y, node.width, node.height) {
        (Some(x), Some(y), Some(w), Some(h)) => (x, y, w, h),
        _ => return, // skip nodes without layout
    };

    let rect = Rect::new(x as u16, y as u16, w as u16, h as u16);
    // Ratatui widgets expect their render area to be inside the frame buffer.
    // Large diagrams can extend below/right of the current terminal viewport;
    // skip off-screen/partially clipped nodes until we add pan/scroll support.
    if rect.x < area.x
        || rect.y < area.y
        || rect.x.saturating_add(rect.width) > area.x.saturating_add(area.width)
        || rect.y.saturating_add(rect.height) > area.y.saturating_add(area.height)
    {
        return;
    }

    let color = shape_color(&node.shape);

    let border_type = shape_border_type(&node.shape);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(color));

    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    if inner.width > 0 && inner.height > 0 {
        let display_label = match node.shape {
            NodeShape::Diamond => format!("◆ {}", node.label),
            NodeShape::Circle => format!("● {}", node.label),
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
    allow_side_target: bool,
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
            let dx = toward_x as i32 - cx as i32;
            let dy = toward_y as i32 - cy as i32;
            if is_source && dy > 0 {
                // Source exits from bottom center
                Some((cx, y + h))
            } else if !is_source && dy < 0 {
                // If a routed/dummy segment is clearly beside the target, enter from
                // the side instead of forcing an ugly loop back into the top.
                if allow_side_target && dx.abs() > (w as i32 / 2) {
                    if dx > 0 {
                        Some((x + w, cy))
                    } else {
                        Some((x.saturating_sub(1), cy))
                    }
                } else {
                    // Target enters from top center (one cell above the border)
                    Some((cx, y.saturating_sub(1)))
                }
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

fn arrow_vector_into_node(node: &Node, end: (u16, u16)) -> Option<(i32, i32)> {
    if node.id.starts_with("__dummy") {
        return None;
    }
    let (x, y, w, h) = match (node.x, node.y, node.width, node.height) {
        (Some(x), Some(y), Some(w), Some(h)) => (x as u16, y as u16, w as u16, h as u16),
        _ => return None,
    };

    if end.0 == x.saturating_sub(1) {
        Some((1, 0)) // left side: point right into node
    } else if end.0 == x + w {
        Some((-1, 0)) // right side: point left into node
    } else if end.1 == y.saturating_sub(1) {
        Some((0, 1)) // top side: point down into node
    } else if end.1 == y + h {
        Some((0, -1)) // bottom side: point up into node
    } else {
        None
    }
}

const DIR_UP: u8 = 0b0001;
const DIR_DOWN: u8 = 0b0010;
const DIR_LEFT: u8 = 0b0100;
const DIR_RIGHT: u8 = 0b1000;

fn char_to_dirs(ch: &str) -> u8 {
    match ch {
        "│" => DIR_UP | DIR_DOWN,
        "─" => DIR_LEFT | DIR_RIGHT,
        "┌" => DIR_DOWN | DIR_RIGHT,
        "┐" => DIR_DOWN | DIR_LEFT,
        "└" => DIR_UP | DIR_RIGHT,
        "┘" => DIR_UP | DIR_LEFT,
        "├" => DIR_UP | DIR_DOWN | DIR_RIGHT,
        "┤" => DIR_UP | DIR_DOWN | DIR_LEFT,
        "┬" => DIR_LEFT | DIR_RIGHT | DIR_DOWN,
        "┴" => DIR_LEFT | DIR_RIGHT | DIR_UP,
        "┼" => DIR_UP | DIR_DOWN | DIR_LEFT | DIR_RIGHT,
        _ => 0,
    }
}

fn dirs_to_char(dirs: u8) -> char {
    match dirs {
        d if d == (DIR_UP | DIR_DOWN) => '│',
        d if d == (DIR_LEFT | DIR_RIGHT) => '─',
        d if d == (DIR_DOWN | DIR_RIGHT) => '┌',
        d if d == (DIR_DOWN | DIR_LEFT) => '┐',
        d if d == (DIR_UP | DIR_RIGHT) => '└',
        d if d == (DIR_UP | DIR_LEFT) => '┘',
        d if d == (DIR_UP | DIR_DOWN | DIR_RIGHT) => '├',
        d if d == (DIR_UP | DIR_DOWN | DIR_LEFT) => '┤',
        d if d == (DIR_LEFT | DIR_RIGHT | DIR_DOWN) => '┬',
        d if d == (DIR_LEFT | DIR_RIGHT | DIR_UP) => '┴',
        d if d == (DIR_UP | DIR_DOWN | DIR_LEFT | DIR_RIGHT) => '┼',
        d if d & (DIR_LEFT | DIR_RIGHT) != 0 => '─',
        d if d & (DIR_UP | DIR_DOWN) != 0 => '│',
        _ => ' ',
    }
}

fn glyph_dirs(ch: char) -> u8 {
    match ch {
        '│' => DIR_UP | DIR_DOWN,
        '─' => DIR_LEFT | DIR_RIGHT,
        '┌' => DIR_DOWN | DIR_RIGHT,
        '┐' => DIR_DOWN | DIR_LEFT,
        '└' => DIR_UP | DIR_RIGHT,
        '┘' => DIR_UP | DIR_LEFT,
        _ => 0,
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

    let start = match connection_point(src, tgt_center.0, tgt_center.1, direction, true, false) {
        Some(p) => p,
        None => return,
    };
    let end = match connection_point(tgt, src_center.0, src_center.1, direction, false, src.id.starts_with("__dummy")) {
        Some(p) => p,
        None => return,
    };

    let buf = frame.buffer_mut();
    let buf_area = buf.area;

    let edge_style = Style::default().fg(Color::DarkGray);

    // Helper: set a cell only if it's in bounds and NOT inside any node's bounding box.
    // Solid edge glyphs merge with existing solid edge glyphs so shared branches form
    // proper junctions (`┴`, `┬`, `┼`, etc.) instead of overwriting each other.
    let set_cell = |buf: &mut ratatui::buffer::Buffer, px: u16, py: u16, ch: char, style: Style, nodes: &[Node]| {
        if px < buf.area.x + buf.area.width
            && py < buf.area.y + buf.area.height
            && py >= buf.area.y
            && !is_inside_any_node(px, py, nodes)
        {
            if edge.style == EdgeStyle::Solid {
                let existing_dirs = char_to_dirs(buf[(px, py)].symbol());
                let new_dirs = glyph_dirs(ch);
                if existing_dirs != 0 && new_dirs != 0 {
                    buf[(px, py)].set_char(dirs_to_char(existing_dirs | new_dirs)).set_style(style);
                    return;
                }
            }
            buf[(px, py)].set_char(ch).set_style(style);
        }
    };

    let min_max = |a: u16, b: u16| if a < b { (a, b) } else { (b, a) };

    let mut arrow_dx;
    let mut arrow_dy;
    let label_x;
    let label_y;

    if *direction == Direction::TopDown {
        let mid_y = (start.1 + end.1) / 2;

        // 1. Vertical from start to mid_y. If this edge has a bend, leave the
        // bend cell for the corner glyph so it doesn't over-connect as `┼`.
        let (y0, y1) = min_max(start.1, mid_y);
        for y in y0..=y1 {
            if start.0 == end.0 || y != mid_y {
                set_cell(buf, start.0, y, edge_v_char(&edge.style), edge_style, all_nodes);
            }
        }

        // 2. Horizontal from start.0 to end.0 at mid_y
        if start.0 != end.0 {
            let (x0, x1) = min_max(start.0, end.0);
            for x in x0..=x1 {
                if x != start.0 && x != end.0 {
                    set_cell(buf, x, mid_y, edge_h_char(&edge.style), edge_style, all_nodes);
                }
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

        // 3. Vertical from mid_y to end.1. If this edge has a bend, leave the
        // bend cell for the corner glyph.
        let (y0, y1) = min_max(mid_y, end.1);
        for y in y0..=y1 {
            if start.0 == end.0 || y != mid_y {
                set_cell(buf, end.0, y, edge_v_char(&edge.style), edge_style, all_nodes);
            }
        }

        // Arrow direction should reflect the actual final segment into the target.
        // Most TopDown edges enter vertically from above, but side-entry routed
        // edges can end on the same row as the horizontal middle segment.
        if start.0 != end.0 && end.1 == mid_y {
            arrow_dx = end.0 as i32 - start.0 as i32;
            arrow_dy = 0;
        } else {
            arrow_dx = 0;
            arrow_dy = end.1 as i32 - mid_y as i32;
        }

        label_x = (start.0 + end.0) / 2;
        label_y = mid_y;
    } else {
        let mid_x = (start.0 + end.0) / 2;

        // 1. Horizontal from start to mid_x. If this edge has a bend, leave the
        // bend cell for the corner glyph.
        let (x0, x1) = min_max(start.0, mid_x);
        for x in x0..=x1 {
            if start.1 == end.1 || x != mid_x {
                set_cell(buf, x, start.1, edge_h_char(&edge.style), edge_style, all_nodes);
            }
        }

        // 2. Vertical from start.1 to end.1 at mid_x
        if start.1 != end.1 {
            let (y0, y1) = min_max(start.1, end.1);
            for y in y0..=y1 {
                if y != start.1 && y != end.1 {
                    set_cell(buf, mid_x, y, edge_v_char(&edge.style), edge_style, all_nodes);
                }
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

        // 3. Horizontal from mid_x to end.0. If this edge has a bend, leave the
        // bend cell for the corner glyph.
        let (x0, x1) = min_max(mid_x, end.0);
        for x in x0..=x1 {
            if start.1 == end.1 || x != mid_x {
                set_cell(buf, x, end.1, edge_h_char(&edge.style), edge_style, all_nodes);
            }
        }

        // Arrow direction should reflect the actual final segment into the target.
        // Most LeftRight edges enter horizontally from the left, but vertical
        // side-entry routes can end on the same column as the middle segment.
        if start.1 != end.1 && end.0 == mid_x {
            arrow_dx = 0;
            arrow_dy = end.1 as i32 - start.1 as i32;
        } else {
            arrow_dx = end.0 as i32 - mid_x as i32;
            arrow_dy = 0;
        }

        label_x = mid_x;
        label_y = (start.1 + end.1) / 2;
    }

    // Prefer arrowheads that point into the target node's touched side. This
    // matters for long/dummy-routed edges that enter from the left/right side.
    if let Some((dx, dy)) = arrow_vector_into_node(tgt, end) {
        arrow_dx = dx;
        arrow_dy = dy;
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

        // Diamond renders as a clean semantic boxed node, not fake diagonal borders.
        assert_eq!(buf[(3, 1)].symbol(), "╔", "Diamond box top-left should be ╔");
        assert_eq!(buf[(13, 1)].symbol(), "╗", "Diamond box top-right should be ╗");
        assert_eq!(buf[(3, 5)].symbol(), "╚", "Diamond box bottom-left should be ╚");
        assert_eq!(buf[(13, 5)].symbol(), "╝", "Diamond box bottom-right should be ╝");
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
                },
                Edge {
                    source: "A".into(),
                    target: "C".into(),
                    label: None,
                    style: EdgeStyle::Solid,
                    arrowhead: Arrowhead::Normal,
                },
            ],
        };

        let backend = TestBackend::new(40, 18);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
        let buf = terminal.backend().buffer();

        // Both edges leave A through the same vertical segment, then split left/right.
        // The split cell should be a clean T-junction, not whichever corner was drawn last.
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
        };
        let backend = TestBackend::new(20, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        // Should not panic
        terminal.draw(|frame| render_to_frame(&graph, frame)).unwrap();
    }

    fn dump_graph(name: &str, graph: &Graph, width: u16, height: u16) {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render_to_frame(graph, frame)).unwrap();
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
        // Diagnostic: render the same graph main.rs uses, at a realistic terminal size,
        // and print the buffer so we can see what the user sees.
        let input = r#"graph TD
    A[Start] --> B{Decision}
    B -->|yes| C(Process)
    B -->|no| D((End))
    C --> D
"#;
        let mut graph = crate::parser::mermaid::parse(input).unwrap();
        crate::layout::layout(&mut graph);

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
        let mut graph = crate::parser::mermaid::parse(
            crate::testdata::fixtures::complex_architecture_mermaid(),
        )
        .unwrap();
        crate::layout::layout(&mut graph);
        dump_graph("complex architecture", &graph, 180, 70);
    }
}
