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
    text::{Line, Span, Text},
    widgets::Paragraph,
};

use crate::model::{
    Arrowhead, Direction, EdgeClass, EdgeStyle, Graph, Group, Node, NodeShape, PortSide, RoutePlan,
};
use crate::theme::{NodeTheme, Theme};

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
    let theme = Theme::default();
    render_to_string_with_theme(graph, &theme)
}

/// Render the whole graph into a string using a specific theme.
pub fn render_to_string_with_theme(graph: &Graph, theme: &Theme) -> io::Result<String> {
    let (width, height) = graph_bounds(graph);
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| render_to_frame_with_theme(graph, frame, theme))?;
    let buf = terminal.backend().buffer();

    let mut out = String::new();
    for y in 0..buf.area.height {
        let last_visible = (0..buf.area.width).rev().find(|&x| {
            let cell = &buf[(x, y)];
            cell.symbol() != " " || cell.bg != Color::Reset
        });

        let Some(last_x) = last_visible else {
            out.push('\n');
            continue;
        };

        let mut current_fg = Color::Reset;
        let mut current_bg = Color::Reset;
        for x in 0..=last_x {
            let cell = &buf[(x, y)];
            if cell.bg != current_bg {
                out.push_str(&ansi_bg(cell.bg));
                current_bg = cell.bg;
            }
            if cell.fg != current_fg {
                out.push_str(&ansi_fg(cell.fg));
                current_fg = cell.fg;
            }
            out.push_str(cell.symbol());
        }
        if current_fg != Color::Reset {
            out.push_str("\x1b[39m");
        }
        if current_bg != Color::Reset {
            out.push_str("\x1b[49m");
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

fn ansi_bg(color: Color) -> String {
    match color {
        Color::Reset => "\x1b[49m".to_string(),
        Color::Black => "\x1b[40m".to_string(),
        Color::Red => "\x1b[41m".to_string(),
        Color::Green => "\x1b[42m".to_string(),
        Color::Yellow => "\x1b[43m".to_string(),
        Color::Blue => "\x1b[44m".to_string(),
        Color::Magenta => "\x1b[45m".to_string(),
        Color::Cyan => "\x1b[46m".to_string(),
        Color::Gray => "\x1b[47m".to_string(),
        Color::DarkGray => "\x1b[100m".to_string(),
        Color::LightRed => "\x1b[101m".to_string(),
        Color::LightGreen => "\x1b[102m".to_string(),
        Color::LightYellow => "\x1b[103m".to_string(),
        Color::LightBlue => "\x1b[104m".to_string(),
        Color::LightMagenta => "\x1b[105m".to_string(),
        Color::LightCyan => "\x1b[106m".to_string(),
        Color::White => "\x1b[107m".to_string(),
        Color::Rgb(r, g, b) => format!("\x1b[48;2;{r};{g};{b}m"),
        Color::Indexed(i) => format!("\x1b[48;5;{i}m"),
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

/// Testable inner function — draws graph onto a Frame.
pub fn render_to_frame(graph: &Graph, frame: &mut Frame) {
    let theme = Theme::default();
    render_to_frame_with_theme(graph, frame, &theme);
}

/// Draw the graph using a specific theme.
pub fn render_to_frame_with_theme(graph: &Graph, frame: &mut Frame, theme: &Theme) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    // Render group boxes behind nodes and edges.
    for group in &graph.groups {
        render_group(group, frame, area, theme);
    }

    // Tight, one-cell card shadows first.
    for node in &graph.nodes {
        render_node_shadow(node, frame, area, theme);
    }

    // Render filled terminal-native cards.
    for node in &graph.nodes {
        render_node(node, frame, area, theme);
    }

    // Render edges on top so arrowheads remain visible at node boundaries.
    render_edges(graph, frame, area, theme);
}

fn render_group(group: &Group, frame: &mut Frame, area: Rect, theme: &Theme) {
    let rect = match group_rect(group) {
        Some(rect) if rect_inside(rect, area) && rect.width >= 2 && rect.height >= 2 => rect,
        _ => return,
    };

    let style = Style::default().fg(theme.muted).bg(Color::Reset);
    let buf = frame.buffer_mut();
    let left = rect.x;
    let right = rect.x.saturating_add(rect.width.saturating_sub(1));
    let top = rect.y;
    let bottom = rect.y.saturating_add(rect.height.saturating_sub(1));

    buf[(left, top)].set_char('┌').set_style(style);
    buf[(right, top)].set_char('┐').set_style(style);
    buf[(left, bottom)].set_char('└').set_style(style);
    buf[(right, bottom)].set_char('┘').set_style(style);

    for x in left.saturating_add(1)..right {
        buf[(x, top)].set_char('─').set_style(style);
        buf[(x, bottom)].set_char('─').set_style(style);
    }
    for y in top.saturating_add(1)..bottom {
        buf[(left, y)].set_char('│').set_style(style);
        buf[(right, y)].set_char('│').set_style(style);
    }

    let label = format!(" {} ", group.label);
    let max_label_width = rect.width.saturating_sub(4) as usize;
    for (i, ch) in label.chars().take(max_label_width).enumerate() {
        let x = left.saturating_add(2 + i as u16);
        if x < right {
            buf[(x, top)].set_char(ch).set_style(style);
        }
    }
}

fn group_rect(group: &Group) -> Option<Rect> {
    let (x, y, w, h) = match (group.x, group.y, group.width, group.height) {
        (Some(x), Some(y), Some(w), Some(h)) => (x, y, w, h),
        _ => return None,
    };
    Some(Rect::new(
        x as u16,
        y as u16,
        w.ceil() as u16,
        h.ceil() as u16,
    ))
}

/// Render a very subtle one-cell cast shadow behind a card.
fn render_node_shadow(node: &Node, frame: &mut Frame, area: Rect, theme: &Theme) {
    if node.id.starts_with("__dummy") {
        return;
    }

    let rect = match node_rect(node) {
        Some(rect) if rect_inside(rect, area) => rect,
        _ => return,
    };

    let buf = frame.buffer_mut();
    let style = Style::default().fg(theme.shadow).bg(Color::Reset);

    // Thin right-side shadow. Keep it lighter than the bottom shadow so it
    // reads as a subtle side falloff rather than an outline.
    let shadow_x = rect.x.saturating_add(rect.width);
    if shadow_x < area.right() {
        for y in rect.y.saturating_add(1)..rect.y.saturating_add(rect.height) {
            if y < area.bottom() {
                buf[(shadow_x, y)].set_char('▏').set_style(style);
            }
        }
    }

    // Thin bottom shadow spanning the card width. This is much less heavy than
    // bg-filled cells because `▔` only occupies a small top slice of the cell.
    let shadow_y = rect.y.saturating_add(rect.height);
    if shadow_y < area.bottom() {
        for x in rect.x..rect.x.saturating_add(rect.width) {
            if x < area.right() {
                buf[(x, shadow_y)].set_char('▔').set_style(style);
            }
        }
    }
}

/// Render a single node as a borderless, filled terminal card.
fn render_node(node: &Node, frame: &mut Frame, area: Rect, theme: &Theme) {
    if node.id.starts_with("__dummy") {
        return;
    }

    let rect = match node_rect(node) {
        Some(rect) if rect_inside(rect, area) => rect,
        _ => return, // skip nodes without layout or outside viewport
    };

    let node_theme = theme.node(&node.shape);
    frame
        .buffer_mut()
        .set_style(rect, Style::default().bg(node_theme.fill));

    let label = center_label(node, node_theme, rect);
    frame.render_widget(label, rect);
}

fn node_rect(node: &Node) -> Option<Rect> {
    let (x, y, w, h) = match (node.x, node.y, node.width, node.height) {
        (Some(x), Some(y), Some(w), Some(h)) => (x, y, w, h),
        _ => return None,
    };
    Some(Rect::new(x as u16, y as u16, w as u16, h as u16))
}

fn rect_inside(rect: Rect, area: Rect) -> bool {
    rect.x >= area.x
        && rect.y >= area.y
        && rect.x.saturating_add(rect.width) <= area.x.saturating_add(area.width)
        && rect.y.saturating_add(rect.height) <= area.y.saturating_add(area.height)
}

/// Create a Paragraph that centers the node label both horizontally and vertically.
fn center_label(node: &Node, node_theme: NodeTheme, area: Rect) -> Paragraph<'_> {
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

    let label_line = match node.shape {
        NodeShape::Diamond => Line::from(vec![
            Span::styled("◆", Style::default().fg(node_theme.icon)),
            Span::raw(" "),
            Span::styled(node.label.clone(), Style::default().fg(node_theme.text)),
        ]),
        NodeShape::Circle => Line::from(vec![
            Span::styled("●", Style::default().fg(node_theme.icon)),
            Span::raw(" "),
            Span::styled(node.label.clone(), Style::default().fg(node_theme.text)),
        ]),
        NodeShape::Rectangle | NodeShape::RoundedRect => Line::from(Span::styled(
            node.label.clone(),
            Style::default().fg(node_theme.text),
        )),
    }
    .centered();

    lines.push(label_line);

    Paragraph::new(Text::from(lines)).style(Style::default().fg(node_theme.text))
}

/// Render all edges in the graph.
fn render_edges(graph: &Graph, frame: &mut Frame, _area: Rect, theme: &Theme) {
    for edge in &graph.edges {
        let source = graph.nodes.iter().find(|n| n.id == edge.source);
        let target = graph.nodes.iter().find(|n| n.id == edge.target);

        if let (Some(src), Some(tgt)) = (source, target) {
            render_edge(src, tgt, edge, frame, &graph.direction, &graph.nodes, theme);
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
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    cx: u16,
    cy: u16,
    toward_x: u16,
    toward_y: u16,
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

fn edge_class_style(class: Option<&EdgeClass>, theme: &Theme) -> Style {
    let fg = match class {
        Some(EdgeClass::Telemetry) => theme.muted,
        Some(EdgeClass::Error) => Color::Rgb(243, 139, 168),
        Some(EdgeClass::BackEdge) => Color::Rgb(203, 166, 247),
        Some(EdgeClass::External) => Color::Rgb(116, 199, 236),
        Some(EdgeClass::Primary) | None => theme.edge,
    };
    Style::default().fg(fg).bg(Color::Reset)
}

/// Arrowhead character based on direction.
fn arrowhead_char(dx: i32, dy: i32, arrowhead: &Arrowhead) -> Option<char> {
    match arrowhead {
        Arrowhead::None => None,
        Arrowhead::Normal | Arrowhead::Open => {
            if dy.abs() >= dx.abs() {
                if dy > 0 { Some('▼') } else { Some('▲') }
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

fn add_dir(cells: &mut Vec<((u16, u16), u8)>, point: (u16, u16), dir: u8) {
    if let Some((_, dirs)) = cells.iter_mut().find(|(existing, _)| *existing == point) {
        *dirs |= dir;
    } else {
        cells.push((point, dir));
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

        // If the existing route approached the target on the arrowhead row/column,
        // make it turn onto the tail row/column before the arrowhead. Otherwise a
        // `▼` can still have a horizontal segment attached to it, which reads as a
        // sideways arrow with the wrong head. The final segment into the target must
        // be the one-cell tail in the arrow direction.
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

/// Check if a cell position is inside any node's bounding box.
fn is_inside_any_node(px: u16, py: u16, nodes: &[Node]) -> bool {
    for node in nodes {
        if let (Some(nx), Some(ny), Some(nw), Some(nh)) = (node.x, node.y, node.width, node.height)
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
    theme: &Theme,
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
    let end = match connection_point(
        tgt,
        src_center.0,
        src_center.1,
        direction,
        false,
        src.id.starts_with("__dummy"),
    ) {
        Some(p) => p,
        None => return,
    };

    let buf = frame.buffer_mut();
    let buf_area = buf.area;

    let edge_style = edge_class_style(edge.route.as_ref().map(|route| &route.class), theme);

    // Helper: set a cell only if it's in bounds and NOT inside any node's bounding box.
    // Solid edge glyphs merge with existing solid edge glyphs so shared branches form
    // proper junctions (`┴`, `┬`, `┼`, etc.) instead of overwriting each other.
    let set_cell = |buf: &mut ratatui::buffer::Buffer,
                    px: u16,
                    py: u16,
                    ch: char,
                    style: Style,
                    nodes: &[Node]| {
        if px < buf.area.x + buf.area.width
            && py < buf.area.y + buf.area.height
            && py >= buf.area.y
            && !is_inside_any_node(px, py, nodes)
        {
            if edge.style == EdgeStyle::Solid {
                let existing_dirs = char_to_dirs(buf[(px, py)].symbol());
                let new_dirs = glyph_dirs(ch);
                if existing_dirs != 0 && new_dirs != 0 {
                    buf[(px, py)]
                        .set_char(dirs_to_char(existing_dirs | new_dirs))
                        .set_style(style);
                    return;
                }
            }
            buf[(px, py)].set_char(ch).set_style(style);
        }
    };

    let set_cell_dirs = |buf: &mut ratatui::buffer::Buffer,
                         px: u16,
                         py: u16,
                         dirs: u8,
                         style: Style,
                         nodes: &[Node]| {
        if px < buf.area.x + buf.area.width
            && py < buf.area.y + buf.area.height
            && py >= buf.area.y
            && !is_inside_any_node(px, py, nodes)
        {
            if edge.style == EdgeStyle::Solid {
                let existing_dirs = char_to_dirs(buf[(px, py)].symbol());
                let merged_dirs = existing_dirs | dirs;
                if merged_dirs != 0 {
                    buf[(px, py)]
                        .set_char(dirs_to_char(merged_dirs))
                        .set_style(style);
                    return;
                }
            }

            let has_horizontal = dirs & (DIR_LEFT | DIR_RIGHT) != 0;
            let has_vertical = dirs & (DIR_UP | DIR_DOWN) != 0;
            let ch = if has_horizontal && has_vertical {
                dirs_to_char(dirs)
            } else if has_horizontal {
                edge_h_char(&edge.style)
            } else {
                edge_v_char(&edge.style)
            };
            buf[(px, py)].set_char(ch).set_style(style);
        }
    };

    let min_max = |a: u16, b: u16| if a < b { (a, b) } else { (b, a) };

    if let Some(route) = &edge.route {
        let route_points = routed_render_points(route);

        let mut route_cells = Vec::new();
        if let Some(&first) = route_points.first() {
            route_cells.push(first);
        }
        for segment in route_points.windows(2) {
            let (mut x, mut y) = segment[0];
            let (x1, y1) = segment[1];

            while x != x1 {
                if x1 > x {
                    x += 1;
                } else {
                    x = x.saturating_sub(1);
                }
                route_cells.push((x, y));
            }

            while y != y1 {
                if y1 > y {
                    y += 1;
                } else {
                    y = y.saturating_sub(1);
                }
                route_cells.push((x, y));
            }
        }

        let mut cell_dirs: Vec<((u16, u16), u8)> = Vec::new();
        for segment in route_cells.windows(2) {
            let (x0, y0) = segment[0];
            let (x1, y1) = segment[1];
            let (from_dir, to_dir) = if x1 > x0 {
                (DIR_RIGHT, DIR_LEFT)
            } else if x1 < x0 {
                (DIR_LEFT, DIR_RIGHT)
            } else if y1 > y0 {
                (DIR_DOWN, DIR_UP)
            } else if y1 < y0 {
                (DIR_UP, DIR_DOWN)
            } else {
                continue;
            };

            add_dir(&mut cell_dirs, (x0, y0), from_dir);
            add_dir(&mut cell_dirs, (x1, y1), to_dir);
        }

        for ((x, y), dirs) in cell_dirs {
            set_cell_dirs(buf, x, y, dirs, edge_style, all_nodes);
        }

        if let Some((&end, prev)) = route_points.last().zip(route_points.iter().rev().nth(1)) {
            let mut arrow_dx = end.0 as i32 - prev.0 as i32;
            let mut arrow_dy = end.1 as i32 - prev.1 as i32;
            if let Some((dx, dy)) = arrow_vector_into_node(tgt, end) {
                arrow_dx = dx;
                arrow_dy = dy;
            }
            if let Some(arrow) = arrowhead_char(arrow_dx, arrow_dy, &edge.arrowhead) {
                if end.0 < buf_area.x + buf_area.width && end.1 < buf_area.y + buf_area.height {
                    buf[(end.0, end.1)].set_char(arrow).set_style(edge_style);
                }
            }
        }

        if let Some(label) = &edge.label {
            let anchor = route
                .label_anchor
                .as_ref()
                .or_else(|| route.points.get(route.points.len() / 2));
            if let Some(anchor) = anchor {
                let lx =
                    (anchor.x.max(0.0).round() as u16).saturating_sub((label.len() / 2) as u16);
                let ly = (anchor.y.max(0.0).round() as u16).saturating_sub(1);
                let label_style = Style::default().fg(theme.edge_label).bg(Color::Reset);
                for (i, ch) in label.chars().enumerate() {
                    let px = lx + i as u16;
                    if px < buf_area.x + buf_area.width && ly < buf_area.y + buf_area.height {
                        buf[(px, ly)].set_char(ch).set_style(label_style);
                    }
                }
            }
        }

        return;
    }

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
                set_cell(
                    buf,
                    start.0,
                    y,
                    edge_v_char(&edge.style),
                    edge_style,
                    all_nodes,
                );
            }
        }

        // 2. Horizontal from start.0 to end.0 at mid_y
        if start.0 != end.0 {
            let (x0, x1) = min_max(start.0, end.0);
            for x in x0..=x1 {
                if x != start.0 && x != end.0 {
                    set_cell(
                        buf,
                        x,
                        mid_y,
                        edge_h_char(&edge.style),
                        edge_style,
                        all_nodes,
                    );
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
                set_cell(
                    buf,
                    end.0,
                    y,
                    edge_v_char(&edge.style),
                    edge_style,
                    all_nodes,
                );
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
                set_cell(
                    buf,
                    x,
                    start.1,
                    edge_h_char(&edge.style),
                    edge_style,
                    all_nodes,
                );
            }
        }

        // 2. Vertical from start.1 to end.1 at mid_x
        if start.1 != end.1 {
            let (y0, y1) = min_max(start.1, end.1);
            for y in y0..=y1 {
                if y != start.1 && y != end.1 {
                    set_cell(
                        buf,
                        mid_x,
                        y,
                        edge_v_char(&edge.style),
                        edge_style,
                        all_nodes,
                    );
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
                set_cell(
                    buf,
                    x,
                    end.1,
                    edge_h_char(&edge.style),
                    edge_style,
                    all_nodes,
                );
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
            buf[(end.0, end.1)].set_char(arrow).set_style(edge_style);
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

        let label_style = Style::default().fg(theme.edge_label).bg(Color::Reset);

        // Draw the label as text only. Keep the glyphs unchanged so routing tests
        // and Mermaid label text stay stable.
        for (i, ch) in label.chars().enumerate() {
            let px = lx + i as u16;
            if px < buf_area.x + buf_area.width && ly < buf_area.y + buf_area.height {
                buf[(px, ly)].set_char(ch).set_style(label_style);
            }
        }
    }
}
