use crate::model::{Direction, Graph, Node, NodeShape};
use std::collections::{HashMap, HashSet, VecDeque};

// ── Layout constants ────────────────────────────────────────────────────────
// All values are in character-cell units.

/// Horizontal padding inside a node (each side).
const PADDING_H: f64 = 2.0;

/// Vertical padding inside a node (each side).
/// Smaller than horizontal because terminal chars are ~2× taller than wide.
const PADDING_V: f64 = 1.0;

/// Minimum node width so tiny labels still look good.
const MIN_WIDTH: f64 = 10.0;

/// Minimum node height.
const MIN_HEIGHT: f64 = 3.0;

/// Horizontal gap between sibling nodes in the same layer.
const SPACING_H: f64 = 8.0;

/// Vertical gap between layers.
const SPACING_V: f64 = 5.0;

/// Extra size multiplier for diamonds (they need more room because the label
/// sits inside a rotated square).
const DIAMOND_FACTOR: f64 = 1.4;

// ── Public API ──────────────────────────────────────────────────────────────

/// Lay out every node in `graph`, filling in `x`, `y`, `width`, `height`.
///
/// The algorithm:
/// 1. Size every node from its label + padding.
/// 2. Assign layers via BFS from root nodes (no incoming edges).
/// 3. Order nodes within each layer (insertion-stable, with a barycenter
///    heuristic to reduce edge crossings).
/// 4. Assign coordinates: layer index controls the "main axis" position;
///    position within the layer controls the "cross axis".
pub fn layout(graph: &mut Graph) {
    if graph.nodes.is_empty() {
        return;
    }

    // Step 1: compute sizes
    size_nodes(&mut graph.nodes);

    // Step 2: assign layers
    let id_to_idx: HashMap<String, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();

    let mut layers = assign_layers(graph, &id_to_idx);

    // Step 2.5: insert dummy nodes for long edges
    insert_dummies(graph, &mut layers, &id_to_idx);

    // Rebuild index mapping since graph.nodes may have grown
    let id_to_idx: HashMap<String, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();

    // Step 3: order within layers (barycenter heuristic)
    let ordered_layers = order_layers(&layers, graph, &id_to_idx);

    // Step 4: assign positions
    assign_positions(graph, &ordered_layers, &id_to_idx);
}

// ── Step 1: sizing ──────────────────────────────────────────────────────────

fn size_nodes(nodes: &mut [Node]) {
    for node in nodes.iter_mut() {
        let label_len = node.label.len() as f64;

        let (w, h) = match node.shape {
            NodeShape::Diamond => {
                // Diamond needs extra room because text is inside a rotated box.
                // +2.0 accounts for left/right border characters.
                let w = (label_len + PADDING_H * 2.0 + 2.0) * DIAMOND_FACTOR;
                let h = (1.0 + PADDING_V * 2.0 + 2.0) * DIAMOND_FACTOR;
                (w, h)
            }
            NodeShape::Circle => {
                // Circle: both dimensions should match (but adjusted for aspect ratio).
                // +2.0 accounts for left/right border characters.
                let w = label_len + PADDING_H * 2.0 + 2.0;
                // Terminal chars are ~2× tall, so to make a visual circle the
                // height in rows should be roughly w/2.
                let h = (w / 2.0).max(1.0 + PADDING_V * 2.0 + 2.0);
                (w, h)
            }
            NodeShape::Rectangle | NodeShape::RoundedRect => {
                // +2.0 accounts for left/right border characters.
                let w = label_len + PADDING_H * 2.0 + 2.0;
                let h = 1.0 + PADDING_V * 2.0 + 2.0;
                (w, h)
            }
        };

        node.width = Some(w.max(MIN_WIDTH));
        node.height = Some(h.max(MIN_HEIGHT));
    }
}

// ── Step 2: layer assignment (BFS from roots) ───────────────────────────────

/// Returns `Vec<Vec<usize>>` — each inner vec is the set of node indices in
/// that layer (layer 0 = roots).
fn assign_layers(graph: &Graph, id_to_idx: &HashMap<String, usize>) -> Vec<Vec<usize>> {
    let n = graph.nodes.len();

    // Build adjacency: children[i] = nodes that i points to.
    let mut children: Vec<Vec<usize>> = vec![vec![]; n];
    let mut in_degree: Vec<usize> = vec![0; n];

    for edge in &graph.edges {
        if let (Some(&src), Some(&tgt)) = (
            id_to_idx.get(&edge.source),
            id_to_idx.get(&edge.target),
        ) {
            children[src].push(tgt);
            in_degree[tgt] += 1;
        }
    }

    // Roots: nodes with no incoming edges.
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut depth: Vec<Option<usize>> = vec![None; n];

    for i in 0..n {
        if in_degree[i] == 0 {
            queue.push_back(i);
            depth[i] = Some(0);
        }
    }

    // BFS — assign each node the layer = max(parent layers) + 1 so that
    // long edges push nodes deeper rather than creating overlaps.
    // We use a modified BFS: process in topological order and always take
    // the maximum depth offered by any parent.
    // Re-do with Kahn's algorithm for proper topological order.
    let mut topo_order: Vec<usize> = Vec::with_capacity(n);
    let mut remaining_in = in_degree.clone();
    let mut topo_queue: VecDeque<usize> = VecDeque::new();

    for i in 0..n {
        if remaining_in[i] == 0 {
            topo_queue.push_back(i);
        }
    }

    while let Some(u) = topo_queue.pop_front() {
        topo_order.push(u);
        for &v in &children[u] {
            remaining_in[v] -= 1;
            if remaining_in[v] == 0 {
                topo_queue.push_back(v);
            }
        }
    }

    // Handle nodes not reached by topo sort (cycles / disconnected).
    let in_topo: HashSet<usize> = topo_order.iter().copied().collect();
    for i in 0..n {
        if !in_topo.contains(&i) {
            topo_order.push(i);
        }
    }

    // Assign layers: depth = max(depth of any parent) + 1
    let mut node_layer: Vec<usize> = vec![0; n];
    // parent map
    let mut parents: Vec<Vec<usize>> = vec![vec![]; n];
    for (u, kids) in children.iter().enumerate() {
        for &v in kids {
            parents[v].push(u);
        }
    }

    for &u in &topo_order {
        let layer = if parents[u].is_empty() {
            0
        } else {
            parents[u].iter().map(|&p| node_layer[p] + 1).max().unwrap_or(0)
        };
        node_layer[u] = layer;
    }

    // Group by layer.
    let max_layer = node_layer.iter().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<usize>> = vec![vec![]; max_layer + 1];
    for (i, &l) in node_layer.iter().enumerate() {
        layers[l].push(i);
    }

    layers
}

// ── Step 2.5: dummy node insertion for long edges ───────────────────────────

fn insert_dummies(graph: &mut Graph, layers: &mut Vec<Vec<usize>>, id_to_idx: &HashMap<String, usize>) {
    // Reverse map to easily find node layer
    let mut node_to_layer = HashMap::new();
    for (l, layer) in layers.iter().enumerate() {
        for &n in layer {
            node_to_layer.insert(n, l);
        }
    }

    let mut new_edges = Vec::new();
    let mut edges_to_remove = HashSet::new();

    for (i, edge) in graph.edges.iter().enumerate() {
        if let (Some(&u), Some(&v)) = (id_to_idx.get(&edge.source), id_to_idx.get(&edge.target)) {
            let l_u = *node_to_layer.get(&u).unwrap_or(&0);
            let l_v = *node_to_layer.get(&v).unwrap_or(&0);

            if l_v > l_u + 1 {
                // Long edge spanning multiple layers!
                edges_to_remove.insert(i);

                let mut current_src = edge.source.clone();

                for l in (l_u + 1)..l_v {
                    let dummy_id = format!("__dummy_{}_{}_{}", edge.source, edge.target, l);
                    
                    // Avoid inserting the same dummy multiple times
                    if !id_to_idx.contains_key(&dummy_id) {
                        let dummy_node = Node {
                            id: dummy_id.clone(),
                            label: "".into(),
                            shape: NodeShape::Rectangle,
                            x: None,
                            y: None,
                            width: Some(2.0),
                            height: Some(2.0),
                        };
                        let dummy_idx = graph.nodes.len();
                        graph.nodes.push(dummy_node);

                        // Add to layer
                        while layers.len() <= l {
                            layers.push(vec![]);
                        }
                        layers[l].push(dummy_idx);
                    }

                    // Connect current source to this dummy
                    new_edges.push(crate::model::Edge {
                        source: current_src.clone(),
                        target: dummy_id.clone(),
                        // Place label on the first segment only
                        label: if current_src == edge.source { edge.label.clone() } else { None },
                        style: edge.style.clone(),
                        // Only the final segment gets the arrowhead
                        arrowhead: crate::model::Arrowhead::None,
                    });

                    current_src = dummy_id;
                }

                // Connect the last dummy to the actual target
                new_edges.push(crate::model::Edge {
                    source: current_src,
                    target: edge.target.clone(),
                    label: None,
                    style: edge.style.clone(),
                    arrowhead: edge.arrowhead.clone(),
                });
            }
        }
    }

    // Remove old edges, add new segmented edges
    let mut final_edges = Vec::new();
    for (i, edge) in graph.edges.drain(..).enumerate() {
        if !edges_to_remove.contains(&i) {
            final_edges.push(edge);
        }
    }
    final_edges.extend(new_edges);
    graph.edges = final_edges;
}

// ── Step 3: ordering within layers (barycenter heuristic) ───────────────────

fn order_layers(
    layers: &[Vec<usize>],
    graph: &Graph,
    id_to_idx: &HashMap<String, usize>,
) -> Vec<Vec<usize>> {
    if layers.is_empty() {
        return vec![];
    }

    let n = graph.nodes.len();

    // Build adjacency for cross-layer neighbours.
    let mut children: Vec<Vec<usize>> = vec![vec![]; n];
    let mut parents: Vec<Vec<usize>> = vec![vec![]; n];
    for edge in &graph.edges {
        if let (Some(&src), Some(&tgt)) = (
            id_to_idx.get(&edge.source),
            id_to_idx.get(&edge.target),
        ) {
            children[src].push(tgt);
            parents[tgt].push(src);
        }
    }

    // Start with the insertion order.
    let mut ordered: Vec<Vec<usize>> = layers.to_vec();

    // Position lookup: pos_in_layer[node_idx] = position within its layer.
    let mut pos_in_layer: Vec<usize> = vec![0; n];
    for layer in &ordered {
        for (pos, &node) in layer.iter().enumerate() {
            pos_in_layer[node] = pos;
        }
    }

    // Run a few sweeps of the barycenter heuristic.
    let sweeps = 4;
    for _sweep in 0..sweeps {
        // Forward sweep (top → bottom).
        for li in 1..ordered.len() {
            let mut barycenters: Vec<(usize, f64)> = ordered[li]
                .iter()
                .map(|&node| {
                    let parent_positions: Vec<f64> = parents[node]
                        .iter()
                        .map(|&p| pos_in_layer[p] as f64)
                        .collect();
                    let bc = if parent_positions.is_empty() {
                        pos_in_layer[node] as f64
                    } else {
                        parent_positions.iter().sum::<f64>() / parent_positions.len() as f64
                    };
                    (node, bc)
                })
                .collect();
            barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            ordered[li] = barycenters.iter().map(|&(node, _)| node).collect();
            for (pos, &node) in ordered[li].iter().enumerate() {
                pos_in_layer[node] = pos;
            }
        }

        // Backward sweep (bottom → top).
        for li in (0..ordered.len().saturating_sub(1)).rev() {
            let mut barycenters: Vec<(usize, f64)> = ordered[li]
                .iter()
                .map(|&node| {
                    let child_positions: Vec<f64> = children[node]
                        .iter()
                        .map(|&c| pos_in_layer[c] as f64)
                        .collect();
                    let bc = if child_positions.is_empty() {
                        pos_in_layer[node] as f64
                    } else {
                        child_positions.iter().sum::<f64>() / child_positions.len() as f64
                    };
                    (node, bc)
                })
                .collect();
            barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            ordered[li] = barycenters.iter().map(|&(node, _)| node).collect();
            for (pos, &node) in ordered[li].iter().enumerate() {
                pos_in_layer[node] = pos;
            }
        }
    }

    ordered
}

// ── Step 4: coordinate assignment ───────────────────────────────────────────

fn assign_positions(
    graph: &mut Graph,
    layers: &[Vec<usize>],
    _id_to_idx: &HashMap<String, usize>,
) {
    if layers.is_empty() {
        return;
    }

    let is_lr = graph.direction == Direction::LeftRight;

    // For each layer, compute the total width (or height in LR mode) so we
    // can centre nodes.
    // Pre-read sizes out so we don't borrow graph mutably while reading.
    let sizes: Vec<(f64, f64)> = graph
        .nodes
        .iter()
        .map(|n| (n.width.unwrap_or(MIN_WIDTH), n.height.unwrap_or(MIN_HEIGHT)))
        .collect();

    // For TopDown: layers are rows (y increases), within a row nodes are spread on x.
    // For LeftRight: layers are columns (x increases), within a column nodes are spread on y.

    // Compute per-layer span along the "cross axis" so we can centre.
    let layer_spans: Vec<f64> = layers
        .iter()
        .map(|layer| {
            let mut span = 0.0;
            for (i, &node_idx) in layer.iter().enumerate() {
                let (w, h) = sizes[node_idx];
                span += if is_lr { h } else { w };
                if i + 1 < layer.len() {
                    span += SPACING_H;
                }
            }
            span
        })
        .collect();

    let max_cross_span = layer_spans
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);

    // Main-axis cursor.
    let mut main_cursor: f64 = 0.0;

    for (li, layer) in layers.iter().enumerate() {
        // Centre this layer's nodes relative to the widest layer.
        let cross_offset = (max_cross_span - layer_spans[li]) / 2.0;

        // Max extent along the main axis in this layer (for advancing the cursor).
        let mut max_main_extent: f64 = 0.0;

        let mut cross_cursor = cross_offset;

        for &node_idx in layer {
            let (w, h) = sizes[node_idx];

            let (x, y) = if is_lr {
                // main axis = x (layer index), cross axis = y
                (main_cursor, cross_cursor)
            } else {
                // main axis = y (layer index), cross axis = x
                (cross_cursor, main_cursor)
            };

            graph.nodes[node_idx].x = Some(x);
            graph.nodes[node_idx].y = Some(y);

            if is_lr {
                cross_cursor += h + SPACING_H;
                max_main_extent = max_main_extent.max(w);
            } else {
                cross_cursor += w + SPACING_H;
                max_main_extent = max_main_extent.max(h);
            }
        }

        main_cursor += max_main_extent + SPACING_V;
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::testdata::fixtures;

    // ── helpers ─────────────────────────────────────────────────────────

    /// Assert every node has Some values for x, y, width, height.
    fn assert_all_positioned(graph: &Graph) {
        for node in &graph.nodes {
            assert!(
                node.x.is_some(),
                "node {} missing x",
                node.id
            );
            assert!(
                node.y.is_some(),
                "node {} missing y",
                node.id
            );
            assert!(
                node.width.is_some(),
                "node {} missing width",
                node.id
            );
            assert!(
                node.height.is_some(),
                "node {} missing height",
                node.id
            );
        }
    }

    /// Assert no two nodes' bounding boxes overlap.
    fn assert_no_overlaps(graph: &Graph) {
        let nodes = &graph.nodes;
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                let a = &nodes[i];
                let b = &nodes[j];
                let ax = a.x.unwrap();
                let ay = a.y.unwrap();
                let aw = a.width.unwrap();
                let ah = a.height.unwrap();
                let bx = b.x.unwrap();
                let by = b.y.unwrap();
                let bw = b.width.unwrap();
                let bh = b.height.unwrap();

                let overlap_x = ax < bx + bw && bx < ax + aw;
                let overlap_y = ay < by + bh && by < ay + ah;
                assert!(
                    !(overlap_x && overlap_y),
                    "nodes {} and {} overlap: A({},{} {}x{}) B({},{} {}x{})",
                    a.id,
                    b.id,
                    ax,
                    ay,
                    aw,
                    ah,
                    bx,
                    by,
                    bw,
                    bh
                );
            }
        }
    }

    fn find_node<'a>(graph: &'a Graph, id: &str) -> &'a Node {
        graph.nodes.iter().find(|n| n.id == id).unwrap()
    }

    // ── actual tests ────────────────────────────────────────────────────

    #[test]
    fn test_simple_two_node_positions() {
        let mut g = fixtures::simple_two_node();
        layout(&mut g);
        assert_all_positioned(&g);
        assert_no_overlaps(&g);
    }

    #[test]
    fn test_topdown_later_layers_have_larger_y() {
        let mut g = fixtures::simple_two_node();
        layout(&mut g);
        let a = find_node(&g, "A");
        let b = find_node(&g, "B");
        assert!(
            b.y.unwrap() > a.y.unwrap(),
            "In TopDown, B (layer 1) should have y > A (layer 0)"
        );
    }

    #[test]
    fn test_leftright_later_layers_have_larger_x() {
        let mut g = fixtures::left_right_chain();
        layout(&mut g);
        let a = find_node(&g, "A");
        let b = find_node(&g, "B");
        let c = find_node(&g, "C");
        assert!(
            b.x.unwrap() > a.x.unwrap(),
            "In LeftRight, B should have x > A"
        );
        assert!(
            c.x.unwrap() > b.x.unwrap(),
            "In LeftRight, C should have x > B"
        );
    }

    #[test]
    fn test_diamond_decision_no_overlap() {
        let mut g = fixtures::diamond_decision();
        layout(&mut g);
        assert_all_positioned(&g);
        assert_no_overlaps(&g);
    }

    #[test]
    fn test_diamond_children_side_by_side() {
        let mut g = fixtures::diamond_decision();
        layout(&mut g);

        let c = find_node(&g, "C");
        let d = find_node(&g, "D");

        // C and D are both children of B — they should be on the same layer (same y)
        // and spread horizontally.
        let cy = c.y.unwrap();
        let dy = d.y.unwrap();
        assert!(
            (cy - dy).abs() < 0.01,
            "C and D should share the same y (same layer), got cy={cy} dy={dy}"
        );

        // They should be at different x positions.
        assert!(
            (c.x.unwrap() - d.x.unwrap()).abs() > 1.0,
            "C and D should be at different x positions"
        );
    }

    #[test]
    fn test_diamond_children_well_separated() {
        let mut g = fixtures::diamond_decision();
        layout(&mut g);

        let c = find_node(&g, "C");
        let d = find_node(&g, "D");

        // C and D are siblings (both children of B). They must be spread
        // horizontally with enough room that they don't visually collide.
        // With proper sizing + spacing they should be at least 10 chars apart.
        let separation = (c.x.unwrap() - d.x.unwrap()).abs();
        assert!(
            separation >= 10.0,
            "C and D should be at least 10 chars apart, got {separation}"
        );
    }

    #[test]
    fn test_single_node_graph() {
        let mut g = Graph {
            direction: Direction::TopDown,
            nodes: vec![Node {
                id: "X".into(),
                label: "Solo".into(),
                shape: NodeShape::Rectangle,
                x: None,
                y: None,
                width: None,
                height: None,
            }],
            edges: vec![],
        };
        layout(&mut g);
        assert_all_positioned(&g);
        let x = find_node(&g, "X");
        assert!(x.width.unwrap() >= MIN_WIDTH);
        assert!(x.height.unwrap() >= MIN_HEIGHT);
    }

    #[test]
    fn test_disconnected_nodes() {
        let mut g = Graph {
            direction: Direction::TopDown,
            nodes: vec![
                Node {
                    id: "A".into(),
                    label: "Island 1".into(),
                    shape: NodeShape::Rectangle,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
                Node {
                    id: "B".into(),
                    label: "Island 2".into(),
                    shape: NodeShape::Circle,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
            ],
            edges: vec![],
        };
        layout(&mut g);
        assert_all_positioned(&g);
        assert_no_overlaps(&g);
    }

    #[test]
    fn test_left_right_chain_no_overlap() {
        let mut g = fixtures::left_right_chain();
        layout(&mut g);
        assert_all_positioned(&g);
        assert_no_overlaps(&g);
    }

    #[test]
    fn test_node_sizing_diamond_larger() {
        let mut g = fixtures::diamond_decision();
        layout(&mut g);
        let b = find_node(&g, "B"); // Diamond shape
        let a = find_node(&g, "A"); // RoundedRect, same label length roughly

        // Diamond should be wider than a normal rect with a similar label,
        // because of DIAMOND_FACTOR.
        assert!(
            b.width.unwrap() > a.width.unwrap(),
            "Diamond node B should be wider than rounded rect A (label lengths similar)"
        );
    }

    #[test]
    fn test_min_size_enforced() {
        let mut g = Graph {
            direction: Direction::TopDown,
            nodes: vec![Node {
                id: "T".into(),
                label: "X".into(), // very short label
                shape: NodeShape::Rectangle,
                x: None,
                y: None,
                width: None,
                height: None,
            }],
            edges: vec![],
        };
        layout(&mut g);
        let t = find_node(&g, "T");
        assert!(
            t.width.unwrap() >= MIN_WIDTH,
            "Width should be at least MIN_WIDTH"
        );
        assert!(
            t.height.unwrap() >= MIN_HEIGHT,
            "Height should be at least MIN_HEIGHT"
        );
    }

    #[test]
    fn test_empty_graph() {
        let mut g = Graph {
            direction: Direction::TopDown,
            nodes: vec![],
            edges: vec![],
        };
        layout(&mut g); // should not panic
    }
}
