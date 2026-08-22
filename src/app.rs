use crate::model::{Graph, Node};

#[derive(Debug, Clone)]
pub struct AppState {
    pub graph: Graph,
    pub selected_node: Option<String>,
    pub viewport_x: u16,
    pub viewport_y: u16,
}

impl AppState {
    pub fn new(graph: Graph) -> Self {
        let selected_node = selectable_ids(&graph).into_iter().next();
        Self {
            graph,
            selected_node,
            viewport_x: 0,
            viewport_y: 0,
        }
    }

    pub fn select_next(&mut self, view_width: u16, view_height: u16) {
        self.cycle_selection(1, view_width, view_height);
    }

    pub fn select_prev(&mut self, view_width: u16, view_height: u16) {
        self.cycle_selection(-1, view_width, view_height);
    }

    fn cycle_selection(&mut self, step: isize, view_width: u16, view_height: u16) {
        let ids = selectable_ids(&self.graph);
        if ids.is_empty() {
            self.selected_node = None;
            return;
        }

        let current = self
            .selected_node
            .as_ref()
            .and_then(|id| ids.iter().position(|candidate| candidate == id))
            .unwrap_or(0) as isize;
        let next = (current + step).rem_euclid(ids.len() as isize) as usize;
        self.selected_node = Some(ids[next].clone());
        self.ensure_selected_visible(view_width, view_height);
    }

    pub fn pan_by(&mut self, dx: i16, dy: i16, view_width: u16, view_height: u16) {
        let (max_x, max_y) = self.pan_bounds(view_width, view_height);
        self.viewport_x = (self.viewport_x as i32 + dx as i32).clamp(0, max_x as i32) as u16;
        self.viewport_y = (self.viewport_y as i32 + dy as i32).clamp(0, max_y as i32) as u16;
    }

    pub fn pan_bounds(&self, view_width: u16, view_height: u16) -> (u16, u16) {
        let (width, height) = graph_bounds(&self.graph);
        (
            width.saturating_sub(view_width),
            height.saturating_sub(view_height),
        )
    }

    pub fn ensure_selected_visible(&mut self, view_width: u16, view_height: u16) {
        let Some(node) = self.selected_node() else {
            return;
        };
        let Some((x, y, w, h)) = node_bounds(node) else {
            return;
        };

        if x < self.viewport_x {
            self.viewport_x = x;
        } else if x.saturating_add(w) > self.viewport_x.saturating_add(view_width) {
            self.viewport_x = x.saturating_add(w).saturating_sub(view_width);
        }

        if y < self.viewport_y {
            self.viewport_y = y;
        } else if y.saturating_add(h) > self.viewport_y.saturating_add(view_height) {
            self.viewport_y = y.saturating_add(h).saturating_sub(view_height);
        }

        let (max_x, max_y) = self.pan_bounds(view_width, view_height);
        self.viewport_x = self.viewport_x.min(max_x);
        self.viewport_y = self.viewport_y.min(max_y);
    }

    pub fn selected_node(&self) -> Option<&Node> {
        let selected = self.selected_node.as_ref()?;
        self.graph.nodes.iter().find(|node| &node.id == selected)
    }
}

pub fn selectable_ids(graph: &Graph) -> Vec<String> {
    let mut ids: Vec<String> = graph
        .nodes
        .iter()
        .filter(|node| !node.id.starts_with("__dummy"))
        .map(|node| node.id.clone())
        .collect();
    ids.sort();
    ids
}

pub fn graph_bounds(graph: &Graph) -> (u16, u16) {
    let mut max_x = 1.0_f64;
    let mut max_y = 1.0_f64;

    for node in &graph.nodes {
        if let Some((x, y, w, h)) = node_bounds(node) {
            max_x = max_x.max((x + w + 2) as f64);
            max_y = max_y.max((y + h + 2) as f64);
        }
    }

    for group in &graph.groups {
        if let (Some(x), Some(y), Some(w), Some(h)) = (group.x, group.y, group.width, group.height)
        {
            max_x = max_x.max(x + w + 2.0);
            max_y = max_y.max(y + h + 2.0);
        }
    }

    for edge in &graph.edges {
        if let Some(route) = &edge.route {
            for point in &route.points {
                max_x = max_x.max(point.x + 2.0);
                max_y = max_y.max(point.y + 2.0);
            }
            max_x = max_x
                .max(route.source_port.x + 2.0)
                .max(route.target_port.x + 2.0);
            max_y = max_y
                .max(route.source_port.y + 2.0)
                .max(route.target_port.y + 2.0);
        }
    }

    (max_x.ceil() as u16, max_y.ceil() as u16)
}

fn node_bounds(node: &Node) -> Option<(u16, u16, u16, u16)> {
    let (x, y, w, h) = (node.x?, node.y?, node.width?, node.height?);
    Some((x.max(0.0) as u16, y.max(0.0) as u16, w as u16, h as u16))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Direction, Edge, NodeShape};

    fn node(id: &str, x: f64, y: f64) -> Node {
        Node {
            id: id.into(),
            label: id.into(),
            shape: NodeShape::Rectangle,
            x: Some(x),
            y: Some(y),
            width: Some(6.0),
            height: Some(3.0),
        }
    }

    fn graph(nodes: Vec<Node>) -> Graph {
        Graph {
            direction: Direction::TopDown,
            nodes,
            edges: Vec::<Edge>::new(),
            groups: vec![],
        }
    }

    #[test]
    fn selection_cycles_by_id() {
        let mut app = AppState::new(graph(vec![node("B", 0.0, 0.0), node("A", 0.0, 5.0)]));
        assert_eq!(app.selected_node.as_deref(), Some("A"));
        app.select_next(80, 24);
        assert_eq!(app.selected_node.as_deref(), Some("B"));
        app.select_next(80, 24);
        assert_eq!(app.selected_node.as_deref(), Some("A"));
        app.select_prev(80, 24);
        assert_eq!(app.selected_node.as_deref(), Some("B"));
    }

    #[test]
    fn dummy_nodes_are_skipped() {
        let mut app = AppState::new(graph(vec![node("__dummy0", 0.0, 0.0), node("A", 0.0, 5.0)]));
        assert_eq!(app.selected_node.as_deref(), Some("A"));
        app.select_next(80, 24);
        assert_eq!(app.selected_node.as_deref(), Some("A"));
    }

    #[test]
    fn pan_is_clamped_to_bounds() {
        let mut app = AppState::new(graph(vec![node("A", 50.0, 30.0)]));
        app.pan_by(100, 100, 20, 10);
        assert_eq!(app.pan_bounds(20, 10), (38, 25));
        assert_eq!((app.viewport_x, app.viewport_y), (38, 25));
        app.pan_by(-100, -100, 20, 10);
        assert_eq!((app.viewport_x, app.viewport_y), (0, 0));
    }

    #[test]
    fn ensure_visible_scrolls_to_selected_node() {
        let mut app = AppState::new(graph(vec![node("A", 50.0, 30.0)]));
        app.ensure_selected_visible(20, 10);
        assert_eq!((app.viewport_x, app.viewport_y), (36, 23));
    }

    #[test]
    fn cycling_keeps_every_node_visible_in_80x23_viewport() {
        let nodes = (0..12)
            .map(|i| node(&format!("N{i:02}"), 90.0, (i * 6) as f64))
            .collect();
        let mut app = AppState::new(graph(nodes));
        let ids = selectable_ids(&app.graph);
        app.ensure_selected_visible(80, 23);

        for expected in ids {
            assert_eq!(app.selected_node.as_deref(), Some(expected.as_str()));
            assert_selected_fully_visible(&app, 80, 23);
            app.select_next(80, 23);
        }
    }

    fn assert_selected_fully_visible(app: &AppState, width: u16, height: u16) {
        let (x, y, w, h) = node_bounds(app.selected_node().unwrap()).unwrap();
        assert!(x >= app.viewport_x, "selected node is left of viewport");
        assert!(y >= app.viewport_y, "selected node is above viewport");
        assert!(
            x + w <= app.viewport_x + width,
            "selected node is right of viewport"
        );
        assert!(
            y + h <= app.viewport_y + height,
            "selected node is below viewport"
        );
    }
}
