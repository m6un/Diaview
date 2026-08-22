use std::{
    fs,
    io::{self, stdout},
    path::Path,
    time::Duration,
};

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

use crate::app::{AppMode, AppState, UpdateStatus};
use crate::herdr;
use crate::model::{
    Arrowhead, Direction, EdgeClass, EdgeStyle, Graph, Group, Node, NodeShape, PortSide, RoutePlan,
};
use crate::stencil::{ArtifactKind, NodeStencil, stencil_for_node};
use crate::theme::{NodeTheme, Theme};

pub fn render(graph: &Graph) -> io::Result<()> {
    let mut out = stdout();
    let _guard = TerminalGuard::enter(&mut out)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;
    let mut app = AppState::new(graph.clone());

    loop {
        terminal.draw(|frame| render_app_to_frame(&mut app, frame))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Tab => app.select_next(),
                KeyCode::BackTab => app.select_prev(),
                _ => {}
            }
        }
    }

    Ok(())
}

pub fn render_herdr_sidecar(
    graph: &Graph,
    diagram_path: &Path,
    origin_pane: &str,
    source: String,
) -> io::Result<()> {
    let mut out = stdout();
    let _guard = TerminalGuard::enter(&mut out)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;
    let mut app = AppState::new(graph.clone());
    app.enable_actions();
    let mut last_source = source;

    loop {
        terminal.draw(|frame| render_app_to_frame(&mut app, frame))?;

        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
        {
            match &app.mode {
                AppMode::Browsing => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('i') | KeyCode::Enter => app.open_prompt(),
                    KeyCode::Tab => app.select_next(),
                    KeyCode::BackTab => app.select_prev(),
                    _ => {}
                },
                AppMode::Prompt(_) => match key.code {
                    KeyCode::Char(ch) => app.push_prompt_char(ch),
                    KeyCode::Backspace => app.backspace_prompt(),
                    KeyCode::Esc => app.cancel_prompt(),
                    KeyCode::Enter => {
                        if let Some(submission) = app.submit_prompt_to_waiting()
                            && let Err(error) =
                                herdr::prompt_agent(origin_pane, diagram_path, &submission)
                        {
                            app.set_waiting_status(UpdateStatus::AgentError(error));
                        }
                    }
                    _ => {}
                },
                AppMode::Waiting { .. } => {
                    if key.code == KeyCode::Esc {
                        app.stop_waiting();
                    }
                }
            }
        }

        if matches!(app.mode, AppMode::Waiting { .. }) {
            match fs::read_to_string(diagram_path) {
                Ok(next_source) if next_source != last_source => {
                    match app.reload_mermaid(&next_source) {
                        Ok(()) => last_source = next_source,
                        Err(error) => app.set_waiting_status(UpdateStatus::MermaidError(error)),
                    }
                }
                Ok(_) => {}
                Err(error) => app.set_waiting_status(UpdateStatus::FileError(format!(
                    "Failed to read diagram: {error}"
                ))),
            }
        }
    }

    Ok(())
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter(out: &mut io::Stdout) -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(err) = execute!(out, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(err);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

pub fn render_inline(graph: &Graph) -> io::Result<()> {
    let output = render_to_string(graph)?;
    println!("{output}");
    Ok(())
}

pub fn render_to_string(graph: &Graph) -> io::Result<String> {
    let theme = Theme::default();
    render_to_string_with_theme(graph, &theme)
}

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

pub fn render_to_frame(graph: &Graph, frame: &mut Frame) {
    let theme = Theme::default();
    render_to_frame_with_theme(graph, frame, &theme);
}

pub fn render_to_frame_with_theme(graph: &Graph, frame: &mut Frame, theme: &Theme) {
    let area = frame.area();
    if area.width > 0 && area.height > 0 {
        render_graph(graph, frame, area, theme, None);
    }
}

pub fn render_app_to_frame(app: &mut AppState, frame: &mut Frame) {
    let theme = Theme::default();
    render_app_to_frame_with_theme(app, frame, &theme);
}

pub fn render_app_to_frame_with_theme(app: &mut AppState, frame: &mut Frame, theme: &Theme) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let graph_area = graph_area(area);
    let mut graph = app.graph.clone();
    let (graph_width, graph_height) = graph_bounds(&graph);
    let offset_x = graph_area.x + graph_area.width.saturating_sub(graph_width) / 2;
    let offset_y = graph_area.y + graph_area.height.saturating_sub(graph_height) / 2;
    translate_graph(&mut graph, offset_x as f64, offset_y as f64);
    render_graph(
        &graph,
        frame,
        graph_area,
        theme,
        app.selected_node.as_deref(),
    );
    render_status_bar(app, frame, area, theme);
}

fn graph_area(area: Rect) -> Rect {
    Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1))
}

pub fn render_centered_to_frame(graph: &Graph, frame: &mut Frame) {
    let theme = Theme::default();
    render_centered_to_frame_with_theme(graph, frame, &theme);
}

pub fn render_centered_to_frame_with_theme(graph: &Graph, frame: &mut Frame, theme: &Theme) {
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let (graph_width, graph_height) = graph_bounds(graph);
    let offset_x = area.x + area.width.saturating_sub(graph_width) / 2;
    let offset_y = area.y + area.height.saturating_sub(graph_height) / 2;

    if offset_x == 0 && offset_y == 0 {
        render_graph(graph, frame, area, theme, None);
    } else {
        let mut centered = graph.clone();
        translate_graph(&mut centered, offset_x as f64, offset_y as f64);
        render_graph(&centered, frame, area, theme, None);
    }
}

fn translate_graph(graph: &mut Graph, offset_x: f64, offset_y: f64) {
    for node in &mut graph.nodes {
        node.x = node.x.map(|x| x + offset_x);
        node.y = node.y.map(|y| y + offset_y);
    }

    for group in &mut graph.groups {
        group.x = group.x.map(|x| x + offset_x);
        group.y = group.y.map(|y| y + offset_y);
    }

    for edge in &mut graph.edges {
        if let Some(route) = &mut edge.route {
            for point in &mut route.points {
                point.x += offset_x;
                point.y += offset_y;
            }
            route.source_port.x += offset_x;
            route.source_port.y += offset_y;
            route.target_port.x += offset_x;
            route.target_port.y += offset_y;
            if let Some(anchor) = &mut route.label_anchor {
                anchor.x += offset_x;
                anchor.y += offset_y;
            }
        }
    }
}

fn render_graph(
    graph: &Graph,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    selected_node: Option<&str>,
) {
    for group in &graph.groups {
        render_group(group, frame, area, theme);
    }

    for node in &graph.nodes {
        render_node_shadow(node, frame, area, theme);
    }

    for node in &graph.nodes {
        render_node(
            node,
            frame,
            area,
            theme,
            selected_node == Some(node.id.as_str()),
        );
    }

    render_edges(graph, frame, area, theme);
}

fn render_status_bar(app: &AppState, frame: &mut Frame, area: Rect, theme: &Theme) {
    let y = area.bottom().saturating_sub(1);
    let rect = Rect::new(area.x, y, area.width, 1);
    let selected = app
        .selected_node()
        .map(|node| format!("{} {}", node.id, node.label))
        .unwrap_or_else(|| "no selection".to_string());
    let text = match &app.mode {
        AppMode::Browsing if app.actions_enabled => {
            format!(" {selected} | Tab/Shift+Tab select | i/Enter action | q quit")
        }
        AppMode::Browsing => format!(" {selected} | Tab/Shift+Tab select | q quit"),
        AppMode::Prompt(prompt) => {
            format!(" {selected} | prompt: {prompt} | Enter send | Esc cancel")
        }
        AppMode::Waiting {
            update_status: None,
        } => format!(" {selected} | Waiting for agent; watching file | Esc stop waiting"),
        AppMode::Waiting {
            update_status: Some(UpdateStatus::AgentError(error)),
        } => format!(" {selected} | Agent update error: {error} | waiting | Esc stop waiting"),
        AppMode::Waiting {
            update_status: Some(UpdateStatus::MermaidError(error)),
        } => format!(" {selected} | Invalid Mermaid: {error} | waiting | Esc stop waiting"),
        AppMode::Waiting {
            update_status: Some(UpdateStatus::FileError(error)),
        } => format!(" {selected} | File update error: {error} | waiting | Esc stop waiting"),
    };
    let bar = Paragraph::new(text).style(Style::default().fg(theme.text).bg(theme.accent_primary));
    frame.render_widget(bar, rect);
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

    let shadow_x = rect.x.saturating_add(rect.width);
    if shadow_x < area.right() {
        for y in rect.y.saturating_add(1)..rect.y.saturating_add(rect.height) {
            if y < area.bottom() {
                buf[(shadow_x, y)].set_char('▏').set_style(style);
            }
        }
    }

    let shadow_y = rect.y.saturating_add(rect.height);
    if shadow_y < area.bottom() {
        for x in rect.x..rect.x.saturating_add(rect.width) {
            if x < area.right() {
                buf[(x, shadow_y)].set_char('▔').set_style(style);
            }
        }
    }
}

fn render_node(node: &Node, frame: &mut Frame, area: Rect, theme: &Theme, selected: bool) {
    if node.id.starts_with("__dummy") {
        return;
    }

    let rect = match node_rect(node) {
        Some(rect) if rect_inside(rect, area) => rect,
        _ => return,
    };

    let stencil = stencil_for_node(node);
    let mut node_theme = node_theme_for(node, stencil, theme);
    if selected {
        node_theme.fill = theme.accent_primary;
        node_theme.text = Color::White;
        node_theme.icon = Color::White;
    }
    frame
        .buffer_mut()
        .set_style(rect, Style::default().bg(node_theme.fill));

    let label = center_label(node, node_theme, stencil, rect);
    frame.render_widget(label, rect);
}

fn node_theme_for(node: &Node, stencil: NodeStencil, theme: &Theme) -> NodeTheme {
    let base = theme.node(&node.shape);
    match stencil.kind {
        ArtifactKind::Generic => base,
        ArtifactKind::Database => theme.database,
        ArtifactKind::Security => artifact_node_theme(base, theme.accent_secondary),
        ArtifactKind::Bucket
        | ArtifactKind::Queue
        | ArtifactKind::Event
        | ArtifactKind::Function
        | ArtifactKind::Worker
        | ArtifactKind::Cache
        | ArtifactKind::ApiGateway
        | ArtifactKind::Observability
        | ArtifactKind::External => artifact_node_theme(base, theme.accent_primary),
    }
}

fn artifact_node_theme(base: NodeTheme, accent: Color) -> NodeTheme {
    NodeTheme {
        border: accent,
        fill: base.fill,
        text: base.text,
        icon: accent,
    }
}

fn stencil_icon_line<'a>(node: &'a Node, node_theme: NodeTheme, stencil: NodeStencil) -> Line<'a> {
    Line::from(vec![
        Span::styled(stencil.icon, Style::default().fg(node_theme.icon)),
        Span::raw("  "),
        Span::styled(node.label.clone(), Style::default().fg(node_theme.text)),
    ])
}

fn shape_label_line<'a>(node: &'a Node, node_theme: NodeTheme) -> Line<'a> {
    match node.shape {
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
        NodeShape::Rectangle | NodeShape::RoundedRect | NodeShape::Database => Line::from(
            Span::styled(node.label.clone(), Style::default().fg(node_theme.text)),
        ),
    }
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

fn center_label(
    node: &Node,
    node_theme: NodeTheme,
    stencil: NodeStencil,
    area: Rect,
) -> Paragraph<'_> {
    let v_pad = if area.height > 1 {
        (area.height.saturating_sub(1)) / 2
    } else {
        0
    };

    let mut lines: Vec<Line<'_>> = Vec::new();
    for _ in 0..v_pad {
        lines.push(Line::from(""));
    }

    let label_line = if stencil.is_generic() {
        shape_label_line(node, node_theme)
    } else {
        stencil_icon_line(node, node_theme, stencil)
    }
    .centered();

    lines.push(label_line);

    Paragraph::new(Text::from(lines)).style(Style::default().fg(node_theme.text))
}

fn render_edges(graph: &Graph, frame: &mut Frame, _area: Rect, theme: &Theme) {
    for edge in &graph.edges {
        let source = graph.nodes.iter().find(|n| n.id == edge.source);
        let target = graph.nodes.iter().find(|n| n.id == edge.target);

        if let (Some(src), Some(tgt)) = (source, target)
            && node_has_nonnegative_origin(src)
            && node_has_nonnegative_origin(tgt)
        {
            render_edge(src, tgt, edge, frame, &graph.direction, &graph.nodes, theme);
        }
    }
}

fn node_center(node: &Node) -> Option<(u16, u16)> {
    match (node.x, node.y, node.width, node.height) {
        (Some(x), Some(y), Some(w), Some(h)) if x >= 0.0 && y >= 0.0 => {
            Some(((x + w / 2.0) as u16, (y + h / 2.0) as u16))
        }
        _ => None,
    }
}

fn node_has_nonnegative_origin(node: &Node) -> bool {
    matches!((node.x, node.y), (Some(x), Some(y)) if x >= 0.0 && y >= 0.0)
}

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
            let dx = toward_x as i32 - cx as i32;
            let dy = toward_y as i32 - cy as i32;
            if is_source && dy > 0 {
                Some((cx, y + h))
            } else if !is_source && dy < 0 {
                if allow_side_target && dx.abs() > (w as i32 / 2) {
                    if dx > 0 {
                        Some((x + w, cy))
                    } else {
                        Some((x.saturating_sub(1), cy))
                    }
                } else {
                    Some((cx, y.saturating_sub(1)))
                }
            } else {
                connection_point_heuristic((x, y, w, h), (cx, cy), (toward_x, toward_y))
            }
        }
        Direction::LeftRight => {
            let dx = toward_x as i32 - cx as i32;
            if is_source && dx > 0 {
                Some((x + w, cy))
            } else if !is_source && dx < 0 {
                Some((x.saturating_sub(1), cy))
            } else {
                connection_point_heuristic((x, y, w, h), (cx, cy), (toward_x, toward_y))
            }
        }
    }
}

fn connection_point_heuristic(
    (x, y, w, h): (u16, u16, u16, u16),
    (cx, cy): (u16, u16),
    (toward_x, toward_y): (u16, u16),
) -> Option<(u16, u16)> {
    let dx = toward_x as i32 - cx as i32;
    let dy = toward_y as i32 - cy as i32;

    if dx.abs() * (h as i32) > dy.abs() * (w as i32) {
        if dx > 0 {
            Some((x + w, cy))
        } else {
            Some((x.saturating_sub(1), cy))
        }
    } else if dy > 0 {
        Some((cx, y + h))
    } else {
        Some((cx, y.saturating_sub(1)))
    }
}

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
        Some(EdgeClass::Error) => theme.accent_secondary,
        Some(EdgeClass::Primary | EdgeClass::BackEdge | EdgeClass::External) | None => theme.edge,
    };
    Style::default().fg(fg).bg(Color::Reset)
}

fn draw_edge_label(
    buf: &mut ratatui::buffer::Buffer,
    label: &str,
    lx: u16,
    ly: u16,
    theme: &Theme,
    nodes: &[Node],
) {
    let label_width = label.chars().count() as u16;
    let ly = choose_label_y(buf, lx, ly, label_width, theme, nodes);
    let label_style = Style::default().fg(theme.edge_label).bg(Color::Reset);

    for (i, ch) in label.chars().enumerate() {
        let px = lx + i as u16;
        if px < buf.area.x + buf.area.width
            && ly < buf.area.y + buf.area.height
            && !is_inside_any_node(px, ly, nodes)
        {
            buf[(px, ly)].set_char(ch).set_style(label_style);
        }
    }
}

fn choose_label_y(
    buf: &ratatui::buffer::Buffer,
    lx: u16,
    preferred_y: u16,
    label_width: u16,
    theme: &Theme,
    nodes: &[Node],
) -> u16 {
    const OFFSETS: [i16; 9] = [0, -1, 1, -2, 2, -3, 3, -4, 4];
    for offset in OFFSETS {
        let y = preferred_y as i32 + offset as i32;
        if y < buf.area.y as i32 || y >= (buf.area.y + buf.area.height) as i32 {
            continue;
        }
        let y = y as u16;
        if label_row_is_clear(buf, lx, y, label_width, theme, nodes) {
            return y;
        }
    }
    preferred_y
}

fn label_row_is_clear(
    buf: &ratatui::buffer::Buffer,
    lx: u16,
    ly: u16,
    label_width: u16,
    theme: &Theme,
    nodes: &[Node],
) -> bool {
    let right = lx
        .saturating_add(label_width)
        .min(buf.area.x + buf.area.width);
    for px in lx..right {
        let cell = &buf[(px, ly)];
        if is_inside_any_node(px, ly, nodes) {
            return false;
        }
        if cell.fg == theme.edge_label && cell.symbol() != " " {
            return false;
        }
    }
    true
}

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

    let touches_left_side = end.0 == x.saturating_sub(1);
    let touches_right_side = end.0 == x + w;
    let touches_top_side = end.1 == y.saturating_sub(1);
    let touches_bottom_side = end.1 == y + h;

    if touches_left_side {
        Some((1, 0))
    } else if touches_right_side {
        Some((-1, 0))
    } else if touches_top_side {
        Some((0, 1))
    } else if touches_bottom_side {
        Some((0, -1))
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
            if let Some(arrow) = arrowhead_char(arrow_dx, arrow_dy, &edge.arrowhead)
                && end.0 < buf_area.x + buf_area.width
                && end.1 < buf_area.y + buf_area.height
            {
                buf[(end.0, end.1)].set_char(arrow).set_style(edge_style);
            }
        }

        if let Some(label) = &edge.label {
            let anchor = route
                .label_anchor
                .as_ref()
                .or_else(|| route.points.get(route.points.len() / 2));
            if let Some(anchor) = anchor {
                let lx = (anchor.x.max(0.0).round() as u16)
                    .saturating_sub((label.chars().count() / 2) as u16);
                let ly = (anchor.y.max(0.0).round() as u16).saturating_sub(1);
                draw_edge_label(buf, label, lx, ly, theme, all_nodes);
            }
        }

        return;
    }

    let mut arrow_dx;
    let mut arrow_dy;

    let (label_x, label_y) = if *direction == Direction::TopDown {
        let mid_y = (start.1 + end.1) / 2;

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
            if edge.style == EdgeStyle::Solid {
                let corner1 = if end.0 > start.0 {
                    if mid_y >= start.1 { '└' } else { '┌' }
                } else if mid_y >= start.1 {
                    '┘'
                } else {
                    '┐'
                };
                let corner2 = if start.0 < end.0 {
                    if end.1 >= mid_y { '┐' } else { '┘' }
                } else if end.1 >= mid_y {
                    '┌'
                } else {
                    '└'
                };
                set_cell(buf, start.0, mid_y, corner1, edge_style, all_nodes);
                set_cell(buf, end.0, mid_y, corner2, edge_style, all_nodes);
            } else {
                let c = edge_h_char(&edge.style);
                set_cell(buf, start.0, mid_y, c, edge_style, all_nodes);
                set_cell(buf, end.0, mid_y, c, edge_style, all_nodes);
            }
        }

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

        if start.0 != end.0 && end.1 == mid_y {
            arrow_dx = end.0 as i32 - start.0 as i32;
            arrow_dy = 0;
        } else {
            arrow_dx = 0;
            arrow_dy = end.1 as i32 - mid_y as i32;
        }

        ((start.0 + end.0) / 2, mid_y)
    } else {
        let mid_x = (start.0 + end.0) / 2;

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
            if edge.style == EdgeStyle::Solid {
                let corner1 = if end.1 > start.1 {
                    if mid_x >= start.0 { '┐' } else { '┌' }
                } else if mid_x >= start.0 {
                    '┘'
                } else {
                    '└'
                };
                let corner2 = if start.1 < end.1 {
                    if end.0 >= mid_x { '└' } else { '┘' }
                } else if end.0 >= mid_x {
                    '┌'
                } else {
                    '┐'
                };
                set_cell(buf, mid_x, start.1, corner1, edge_style, all_nodes);
                set_cell(buf, mid_x, end.1, corner2, edge_style, all_nodes);
            } else {
                let c = edge_v_char(&edge.style);
                set_cell(buf, mid_x, start.1, c, edge_style, all_nodes);
                set_cell(buf, mid_x, end.1, c, edge_style, all_nodes);
            }
        }

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

        if start.1 != end.1 && end.0 == mid_x {
            arrow_dx = 0;
            arrow_dy = end.1 as i32 - start.1 as i32;
        } else {
            arrow_dx = end.0 as i32 - mid_x as i32;
            arrow_dy = 0;
        }

        (mid_x, (start.1 + end.1) / 2)
    };

    if let Some((dx, dy)) = arrow_vector_into_node(tgt, end) {
        arrow_dx = dx;
        arrow_dy = dy;
    }

    if let Some(arrow) = arrowhead_char(arrow_dx, arrow_dy, &edge.arrowhead)
        && end.0 < buf_area.x + buf_area.width
        && end.1 < buf_area.y + buf_area.height
    {
        buf[(end.0, end.1)].set_char(arrow).set_style(edge_style);
    }

    if let Some(ref label) = edge.label {
        let actually_horizontal = if *direction == Direction::TopDown {
            start.0 != end.0
        } else {
            start.1 == end.1
        };

        let (lx, ly) = if actually_horizontal {
            let start_x = label_x.saturating_sub((label.len() / 2) as u16);
            (start_x, label_y.saturating_sub(1))
        } else {
            (label_x + 1, label_y)
        };

        draw_edge_label(buf, label, lx, ly, theme, all_nodes);
    }
}
