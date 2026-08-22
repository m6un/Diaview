use crate::layout;
use crate::model::{Direction, Graph, Node};
use crate::parser::mermaid;

#[derive(Debug, Clone)]
pub struct AppState {
    pub graph: Graph,
    pub selected_node: Option<String>,
    pub mode: AppMode,
    pub actions_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Browsing,
    Prompt(String),
    Waiting { update_status: Option<UpdateStatus> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    AgentError(String),
    MermaidError(String),
    FileError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionSubmission {
    pub node_id: String,
    pub node_label: String,
    pub prompt: String,
}

impl AppState {
    pub fn new(graph: Graph) -> Self {
        let selected_node = selectable_ids(&graph).into_iter().next();
        Self {
            graph,
            selected_node,
            mode: AppMode::Browsing,
            actions_enabled: false,
        }
    }

    pub fn is_prompting(&self) -> bool {
        matches!(self.mode, AppMode::Prompt(_))
    }

    pub fn select_next(&mut self) {
        if matches!(self.mode, AppMode::Browsing) {
            self.cycle_selection(1);
        }
    }

    pub fn select_prev(&mut self) {
        if matches!(self.mode, AppMode::Browsing) {
            self.cycle_selection(-1);
        }
    }

    pub fn enable_actions(&mut self) {
        self.actions_enabled = true;
    }

    pub fn open_prompt(&mut self) {
        if self.actions_enabled
            && matches!(self.mode, AppMode::Browsing)
            && self.selected_node().is_some()
        {
            self.mode = AppMode::Prompt(String::new());
        }
    }

    pub fn push_prompt_char(&mut self, ch: char) {
        if let AppMode::Prompt(prompt) = &mut self.mode {
            prompt.push(ch);
        }
    }

    pub fn backspace_prompt(&mut self) {
        if let AppMode::Prompt(prompt) = &mut self.mode {
            prompt.pop();
        }
    }

    pub fn cancel_prompt(&mut self) {
        if self.is_prompting() {
            self.mode = AppMode::Browsing;
        }
    }

    pub fn stop_waiting(&mut self) {
        if matches!(self.mode, AppMode::Waiting { .. }) {
            self.mode = AppMode::Browsing;
        }
    }

    pub fn submit_prompt_to_waiting(&mut self) -> Option<ActionSubmission> {
        let AppMode::Prompt(prompt) = &self.mode else {
            return None;
        };
        if prompt.trim().is_empty() {
            return None;
        }
        let node = self.selected_node()?;
        let submission = ActionSubmission {
            node_id: node.id.clone(),
            node_label: node.label.clone(),
            prompt: prompt.clone(),
        };
        self.mode = AppMode::Waiting {
            update_status: None,
        };
        Some(submission)
    }

    pub fn reload_mermaid(&mut self, source: &str) -> Result<(), String> {
        let previous = self.selected_node.clone();
        let mut graph = mermaid::parse(source)?;
        layout::layout(&mut graph);
        let ids = selectable_ids(&graph);
        self.selected_node = previous
            .filter(|id| ids.iter().any(|candidate| candidate == id))
            .or_else(|| ids.into_iter().next());
        self.graph = graph;
        self.mode = AppMode::Browsing;
        Ok(())
    }

    pub fn set_waiting_status(&mut self, status: UpdateStatus) {
        if matches!(self.mode, AppMode::Waiting { .. }) {
            self.mode = AppMode::Waiting {
                update_status: Some(status),
            };
        }
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

    #[test]
    fn standalone_actions_are_disabled() {
        let mut app = AppState::new(graph(Direction::TopDown, vec![node("A", 0.0, 0.0)]));

        app.open_prompt();

        assert_eq!(app.mode, AppMode::Browsing);
    }

    #[test]
    fn prompt_transitions_to_waiting_and_rejects_whitespace() {
        let mut app = AppState::new(graph(Direction::TopDown, vec![node("A", 0.0, 0.0)]));
        app.enable_actions();

        app.open_prompt();
        assert_eq!(app.mode, AppMode::Prompt(String::new()));
        app.push_prompt_char('q');
        app.push_prompt_char('π');
        app.backspace_prompt();
        assert_eq!(app.mode, AppMode::Prompt("q".into()));
        app.cancel_prompt();
        assert_eq!(app.mode, AppMode::Browsing);

        app.open_prompt();
        app.push_prompt_char(' ');
        app.push_prompt_char('\t');
        assert_eq!(app.submit_prompt_to_waiting(), None);
        app.push_prompt_char('🚀');
        app.push_prompt_char(' ');
        assert_eq!(
            app.submit_prompt_to_waiting(),
            Some(ActionSubmission {
                node_id: "A".into(),
                node_label: "A".into(),
                prompt: " \t🚀 ".into(),
            })
        );
        assert_eq!(
            app.mode,
            AppMode::Waiting {
                update_status: None
            }
        );
    }

    #[test]
    fn esc_behavior_for_prompt_and_waiting() {
        let mut app = AppState::new(graph(Direction::TopDown, vec![node("A", 0.0, 0.0)]));
        app.enable_actions();
        app.open_prompt();
        app.cancel_prompt();
        assert_eq!(app.mode, AppMode::Browsing);
        app.mode = AppMode::Waiting {
            update_status: None,
        };
        app.stop_waiting();
        assert_eq!(app.mode, AppMode::Browsing);
    }

    #[test]
    fn selection_does_not_change_while_prompting() {
        let mut app = AppState::new(graph(
            Direction::TopDown,
            vec![node("A", 0.0, 0.0), node("B", 0.0, 5.0)],
        ));
        app.enable_actions();

        app.open_prompt();
        app.select_next();
        app.select_prev();

        assert_eq!(app.selected_node.as_deref(), Some("A"));
    }

    #[test]
    fn valid_reload_preserves_or_falls_back_selection() {
        let mut app = AppState::new(graph(
            Direction::TopDown,
            vec![node("A", 0.0, 0.0), node("B", 0.0, 5.0)],
        ));
        app.selected_node = Some("B".into());
        app.mode = AppMode::Waiting {
            update_status: None,
        };

        app.reload_mermaid("flowchart TD\nA[Start] --> B[End]\n")
            .unwrap();
        assert_eq!(app.selected_node.as_deref(), Some("B"));
        assert_eq!(app.mode, AppMode::Browsing);

        app.selected_node = Some("B".into());
        app.reload_mermaid("flowchart TD\nC[New] --> D[Other]\n")
            .unwrap();
        assert_eq!(app.selected_node.as_deref(), Some("C"));
    }

    #[test]
    fn invalid_reload_retains_graph_and_records_error() {
        let mut app = AppState::new(graph(Direction::TopDown, vec![node("A", 0.0, 0.0)]));
        app.mode = AppMode::Waiting {
            update_status: None,
        };
        let old_graph = app.graph.clone();

        let error = app.reload_mermaid("flowchart TD\nA[").unwrap_err();
        app.set_waiting_status(UpdateStatus::MermaidError(error));

        assert_eq!(app.graph.nodes[0].id, old_graph.nodes[0].id);
        assert!(matches!(
            app.mode,
            AppMode::Waiting {
                update_status: Some(UpdateStatus::MermaidError(_))
            }
        ));
    }
}
