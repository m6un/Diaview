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

    let last_index = points.len() - 1;
    let target_tail = offset_point(points[last_index], outward_delta(&route.target_port.side));
    if last_index == 0 || points.get(last_index - 1).copied() != Some(target_tail) {
        points.insert(last_index, target_tail);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

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
                route: None,
            }],
            groups: vec![],
        }
    }

    #[test]
    fn test_group_bounds_render_behind_nodes() {
        let mut graph = crate::parser::mermaid::parse(
            r#"
            graph TD
            subgraph API[API Layer]
                A[Gateway] --> B[Service]
            end
            "#,
        )
        .unwrap();
        crate::layout::layout(&mut graph);

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

        // Rounded nodes are now filled cards with no outline glyphs.
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
        // Rectangles are filled cards with no outline glyphs.
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

        // Interior cells inherit the node card fill.
        assert_eq!(buf[(6, 2)].bg, theme.rounded_rect.fill);
        // Border cells also carry the fill so there is no black gap between
        // the border glyph and the card background.
        assert_eq!(buf[(5, 1)].bg, theme.rounded_rect.fill);
        assert_eq!(buf[(5, 2)].bg, theme.rounded_rect.fill);
        // Shadows are thin glyphs, not full background cells.
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

        // The label text stays unchanged and intentionally has no background pill.
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
        let mut graph = crate::parser::mermaid::parse(
            r#"
            graph LR
            A[Start] -->|metrics| B[End]
            "#,
        )
        .unwrap();
        crate::layout::layout(&mut graph);

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
        terminal
            .draw(|frame| render_to_frame(&graph, frame))
            .unwrap();
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
        terminal
            .draw(|frame| render_to_frame(&graph, frame))
            .unwrap();
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
        terminal
            .draw(|frame| render_to_frame(&graph, frame))
            .unwrap();
        let buf = terminal.backend().buffer();

        // Arrow should be one cell above node B top border: (10, 7)
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

        // Edge label "next" should appear near the midpoint of the edge
        // midpoint y = (3+8)/2 = 5, x = 10, label starts at x=11
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
            groups: vec![],
        };
        let backend = TestBackend::new(20, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_to_frame(&graph, frame))
            .unwrap();
        let buf = terminal.backend().buffer();

        // Diamond renders as a clean semantic card; the ◆ icon carries the
        // decision semantics instead of a geometric outline.
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

        // Horizontal edge at y=2, arrowhead one cell left of node R's left border: (14, 2)
        let sym = buf[(14, 2)].symbol();
        assert_eq!(sym, "▶", "Horizontal arrowhead should be ▶, got: '{sym}'");

        // Check horizontal line somewhere in the middle
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
            groups: vec![],
        };
        let backend = TestBackend::new(20, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        // Should not panic
        terminal
            .draw(|frame| render_to_frame(&graph, frame))
            .unwrap();
    }

    #[test]
    fn telemetry_edges_render_with_muted_foreground() {
        let mut graph = crate::parser::mermaid::parse(
            crate::testdata::fixtures::phase15_telemetry_overlay_mermaid(),
        )
        .unwrap();
        crate::layout::layout(&mut graph);
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

        let (width, height) = graph_bounds(&graph);
        println!(
            "complex architecture fixture: {} nodes, {} edges, bounds {width}x{height}",
            graph.nodes.len(),
            graph.edges.len()
        );
        dump_graph("complex architecture", &graph, width, height);
    }
}
