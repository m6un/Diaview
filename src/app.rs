use crate::model::{Direction, Graph, Node};

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
    let mut nodes: Vec<&Node> = graph
        .nodes
        .iter()
        .filter(|node| !node.id.starts_with("__dummy"))
        .collect();

    nodes.sort_by(|a, b| match graph.direction {
        Direction::TopDown => coord(a.y)
            .total_cmp(&coord(b.y))
            .then_with(|| coord(a.x).total_cmp(&coord(b.x)))
            .then_with(|| a.id.cmp(&b.id)),
        Direction::LeftRight => coord(a.x)
            .total_cmp(&coord(b.x))
            .then_with(|| coord(a.y).total_cmp(&coord(b.y)))
            .then_with(|| a.id.cmp(&b.id)),
    });

    nodes.into_iter().map(|node| node.id.clone()).collect()
}

fn coord(value: Option<f64>) -> f64 {
    value
        .filter(|value| value.is_finite())
        .unwrap_or(f64::INFINITY)
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

    fn graph(direction: Direction, nodes: Vec<Node>) -> Graph {
        Graph {
            direction,
            nodes,
            edges: Vec::<Edge>::new(),
            groups: vec![],
        }
    }

    #[test]
    fn top_down_selection_follows_visual_order() {
        let app = AppState::new(graph(
            Direction::TopDown,
            vec![
                node("A", 10.0, 10.0),
                node("B", 0.0, 0.0),
                node("C", 5.0, 0.0),
            ],
        ));

        assert_eq!(selectable_ids(&app.graph), vec!["B", "C", "A"]);
        assert_eq!(app.selected_node.as_deref(), Some("B"));
    }

    #[test]
    fn left_right_selection_follows_visual_order() {
        let app = AppState::new(graph(
            Direction::LeftRight,
            vec![
                node("A", 10.0, 10.0),
                node("B", 0.0, 5.0),
                node("C", 0.0, 0.0),
            ],
        ));

        assert_eq!(selectable_ids(&app.graph), vec!["C", "B", "A"]);
        assert_eq!(app.selected_node.as_deref(), Some("C"));
    }

    #[test]
    fn reverse_cycles_through_same_order() {
        let mut app = AppState::new(graph(
            Direction::TopDown,
            vec![node("A", 10.0, 10.0), node("B", 0.0, 0.0)],
        ));

        app.select_prev();
        assert_eq!(app.selected_node.as_deref(), Some("A"));
        app.select_prev();
        assert_eq!(app.selected_node.as_deref(), Some("B"));
    }

    #[test]
    fn dummy_nodes_are_skipped() {
        let mut app = AppState::new(graph(
            Direction::TopDown,
            vec![node("__dummy0", 0.0, 0.0), node("A", 0.0, 5.0)],
        ));
        assert_eq!(app.selected_node.as_deref(), Some("A"));
        app.select_next();
        assert_eq!(app.selected_node.as_deref(), Some("A"));
    }

    #[test]
    fn missing_and_non_finite_coordinates_fall_back_to_id() {
        let mut missing = node("B", 0.0, 0.0);
        missing.y = None;
        let mut non_finite = node("A", 0.0, 0.0);
        non_finite.y = Some(f64::NAN);

        let graph = graph(
            Direction::TopDown,
            vec![missing, non_finite, node("C", 0.0, 0.0)],
        );

        assert_eq!(selectable_ids(&graph), vec!["C", "A", "B"]);
    }
}
