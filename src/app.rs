use crate::model::{Graph, Node};

#[derive(Debug, Clone)]
pub struct AppState {
    pub graph: Graph,
    pub selected_node: Option<String>,
}

impl AppState {
    pub fn new(graph: Graph) -> Self {
        let selected_node = selectable_ids(&graph).into_iter().next();
        Self {
            graph,
            selected_node,
        }
    }

    pub fn select_next(&mut self) {
        self.cycle_selection(1);
    }

    pub fn select_prev(&mut self) {
        self.cycle_selection(-1);
    }

    fn cycle_selection(&mut self, step: isize) {
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
        app.select_next();
        assert_eq!(app.selected_node.as_deref(), Some("B"));
        app.select_next();
        assert_eq!(app.selected_node.as_deref(), Some("A"));
        app.select_prev();
        assert_eq!(app.selected_node.as_deref(), Some("B"));
    }

    #[test]
    fn dummy_nodes_are_skipped() {
        let mut app = AppState::new(graph(vec![node("__dummy0", 0.0, 0.0), node("A", 0.0, 5.0)]));
        assert_eq!(app.selected_node.as_deref(), Some("A"));
        app.select_next();
        assert_eq!(app.selected_node.as_deref(), Some("A"));
    }
}
