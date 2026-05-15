use crate::model::{
    Direction, Edge, EdgeClass, EdgeStyle, Graph, Node, NodeShape, Port, PortSide, RoutePlan,
    RoutePoint,
};
use std::collections::{HashMap, HashSet, VecDeque};

// ── Layout constants ────────────────────────────────────────────────────────
// All values are in character-cell units.

/// Horizontal padding inside a node (each side).
const PADDING_H: f64 = 1.0;

/// Vertical padding inside a node (each side).
/// Keep this compact; terminal rows are already visually tall.
const PADDING_V: f64 = 0.0;

/// Minimum node width so tiny labels still look good.
const MIN_WIDTH: f64 = 8.0;

/// Minimum node height.
/// Three rows gives cards a true middle row for vertically centered text.
const MIN_HEIGHT: f64 = 3.0;

/// Horizontal gap between sibling nodes in the same layer.
const SPACING_H: f64 = 8.0;

/// Vertical gap between layers.
const SPACING_V: f64 = 5.0;

/// Extra size multiplier for diamonds.
/// Kept at 1.0 because diamonds are rendered as semantic boxed nodes (`◆ label`).
const DIAMOND_FACTOR: f64 = 1.0;

// ── Public API ──────────────────────────────────────────────────────────────

/// A graph layout implementation.
///
/// Layout engines prepare a parsed [`Graph`] for rendering by assigning node
/// positions and sizes, and may insert helper routing nodes when needed.
pub trait LayoutEngine {
    /// Lay out every node in `graph`, filling in `x`, `y`, `width`, `height`.
    fn layout(&self, graph: &mut Graph);
}

/// Diaview's current native layered layout engine.
///
/// This preserves the pre-abstraction layout behavior while providing a stable
/// extension point for future engines.
#[derive(Debug, Default, Clone, Copy)]
pub struct SimpleLayoutEngine;

impl LayoutEngine for SimpleLayoutEngine {
    fn layout(&self, graph: &mut Graph) {
        layout_graph(graph);
    }
}

/// Lay out `graph` with the default [`SimpleLayoutEngine`].
///
/// Kept as the ergonomic public API for callers that do not need to choose an
/// engine explicitly.
pub fn layout(graph: &mut Graph) {
    layout_with(&SimpleLayoutEngine, graph);
}

/// Lay out `graph` with a caller-provided layout engine.
pub fn layout_with<E: LayoutEngine + ?Sized>(engine: &E, graph: &mut Graph) {
    engine.layout(graph);
}

/// The default engine algorithm:
/// 1. Size every node from its label + padding.
/// 2. Assign layers via BFS from root nodes (no incoming edges).
/// 3. Order nodes within each layer (insertion-stable, with a barycenter
///    heuristic to reduce edge crossings).
/// 4. Assign coordinates: layer index controls the "main axis" position;
///    position within the layer controls the "cross axis".
fn layout_graph(graph: &mut Graph) {
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

    // Step 5: compute group bounds from positioned member nodes.
    assign_group_bounds(graph);

    // Step 6: compute layout-owned edge route metadata.
    assign_route_plans(graph);
}

// ── Step 1: sizing ──────────────────────────────────────────────────────────

fn size_nodes(nodes: &mut [Node]) {
    for node in nodes.iter_mut() {
        let label_len = match node.shape {
            // Semantic boxed shapes render an icon plus a space before the label.
            NodeShape::Diamond | NodeShape::Circle => node.label.len() as f64 + 2.0,
            _ => node.label.len() as f64,
        };

        let (w, h) = match node.shape {
            NodeShape::Diamond => {
                let mut w = (label_len + PADDING_H * 2.0 + 2.0) * DIAMOND_FACTOR;
                let h = 1.0 + PADDING_V * 2.0;
                w = w.ceil();
                if w as i64 % 2 != 0 {
                    w += 1.0;
                }
                (w, h)
            }
            NodeShape::Circle | NodeShape::Rectangle | NodeShape::RoundedRect => {
                let mut w = label_len + PADDING_H * 2.0 + 2.0;
                w = w.ceil();
                if w as i64 % 2 != 0 {
                    w += 1.0;
                }
                let h = 1.0 + PADDING_V * 2.0;
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
        if let (Some(&src), Some(&tgt)) = (id_to_idx.get(&edge.source), id_to_idx.get(&edge.target))
        {
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
            parents[u]
                .iter()
                .map(|&p| node_layer[p] + 1)
                .max()
                .unwrap_or(0)
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

fn insert_dummies(
    graph: &mut Graph,
    layers: &mut Vec<Vec<usize>>,
    id_to_idx: &HashMap<String, usize>,
) {
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
                            width: Some(0.0),
                            height: Some(0.0),
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
                        label: if current_src == edge.source {
                            edge.label.clone()
                        } else {
                            None
                        },
                        style: edge.style.clone(),
                        // Only the final segment gets the arrowhead
                        arrowhead: crate::model::Arrowhead::None,
                        route: None,
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
                    route: None,
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
        if let (Some(&src), Some(&tgt)) = (id_to_idx.get(&edge.source), id_to_idx.get(&edge.target))
        {
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

fn assign_positions(graph: &mut Graph, layers: &[Vec<usize>], _id_to_idx: &HashMap<String, usize>) {
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
        .map(|n| {
            if n.id.starts_with("__dummy") {
                (0.0, 0.0) // Dummy nodes take 0 height/width so they don't break Y offsets
            } else {
                (n.width.unwrap_or(MIN_WIDTH), n.height.unwrap_or(MIN_HEIGHT))
            }
        })
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

    let max_cross_span = layer_spans.iter().copied().fold(0.0_f64, f64::max);

    // Main-axis cursor.
    let mut main_cursor: f64 = 0.0;

    for (li, layer) in layers.iter().enumerate() {
        // Centre this layer's nodes relative to the widest layer.
        let cross_offset = (max_cross_span - layer_spans[li]) / 2.0;

        // Max extent along the main axis in this layer (for advancing the cursor).
        // Dummy route points are zero-sized, but they should sit in the middle of
        // the layer lane rather than at the top/left edge; otherwise long edges
        // can run flush along real node borders.
        let max_main_extent = layer
            .iter()
            .map(|&node_idx| {
                let (w, h) = sizes[node_idx];
                if is_lr { w } else { h }
            })
            .fold(0.0_f64, f64::max);

        let mut cross_cursor = cross_offset;

        for &node_idx in layer {
            let (w, h) = sizes[node_idx];
            let is_dummy = graph.nodes[node_idx].id.starts_with("__dummy");

            let (x, y) = if is_lr {
                // main axis = x (layer index), cross axis = y
                let x = if is_dummy {
                    main_cursor + max_main_extent / 2.0
                } else {
                    main_cursor
                };
                (x, cross_cursor)
            } else {
                // main axis = y (layer index), cross axis = x
                let y = if is_dummy {
                    main_cursor + max_main_extent / 2.0
                } else {
                    main_cursor
                };
                (cross_cursor, y)
            };

            graph.nodes[node_idx].x = Some(x);
            graph.nodes[node_idx].y = Some(y);

            if is_lr {
                cross_cursor += h + SPACING_H;
            } else {
                cross_cursor += w + SPACING_H;
            }
        }

        main_cursor += max_main_extent + SPACING_V;
    }

    align_long_edge_targets(graph, layers, is_lr);
}

/// When a long edge is split by a dummy route point, prefer placing a lone
/// target under/after that route point. Otherwise a `B -> D` edge that skips
/// over `C` gets averaged between `C -> D` and the dummy parent, causing the
/// routed branch to swing right, down, and then back left in an ugly loop.
fn align_long_edge_targets(graph: &mut Graph, layers: &[Vec<usize>], is_lr: bool) {
    let id_to_idx: HashMap<String, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();

    let mut layer_of = HashMap::new();
    for (li, layer) in layers.iter().enumerate() {
        for &idx in layer {
            layer_of.insert(idx, li);
        }
    }

    for edge in &graph.edges {
        if !edge.source.starts_with("__dummy") {
            continue;
        }
        let (Some(&dummy_idx), Some(&target_idx)) =
            (id_to_idx.get(&edge.source), id_to_idx.get(&edge.target))
        else {
            continue;
        };
        if graph.nodes[target_idx].id.starts_with("__dummy") {
            continue;
        }

        let Some(&target_layer) = layer_of.get(&target_idx) else {
            continue;
        };
        let visible_count = layers[target_layer]
            .iter()
            .filter(|&&idx| !graph.nodes[idx].id.starts_with("__dummy"))
            .count();
        if visible_count != 1 {
            continue;
        }

        if is_lr {
            if let (Some(dummy_y), Some(target_h)) =
                (graph.nodes[dummy_idx].y, graph.nodes[target_idx].height)
            {
                graph.nodes[target_idx].y = Some((dummy_y - target_h / 2.0).max(0.0));
            }
        } else if let (Some(dummy_x), Some(target_w)) =
            (graph.nodes[dummy_idx].x, graph.nodes[target_idx].width)
        {
            graph.nodes[target_idx].x = Some((dummy_x - target_w / 2.0).max(0.0));
        }
    }
}

// ── Step 5: group bounds ───────────────────────────────────────────────────

fn assign_group_bounds(graph: &mut Graph) {
    if graph.groups.is_empty() {
        return;
    }

    let node_by_id: HashMap<String, &Node> = graph
        .nodes
        .iter()
        .filter(|node| !node.id.starts_with("__dummy"))
        .map(|node| (node.id.clone(), node))
        .collect();

    const GROUP_PADDING_X: f64 = 2.0;
    const GROUP_PADDING_Y: f64 = 1.0;

    for group in &mut graph.groups {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for node_id in &group.node_ids {
            let Some(node) = node_by_id.get(node_id) else {
                continue;
            };
            let (Some(x), Some(y), Some(width), Some(height)) =
                (node.x, node.y, node.width, node.height)
            else {
                continue;
            };

            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + width);
            max_y = max_y.max(y + height);
        }

        if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
            let x = (min_x - GROUP_PADDING_X).max(0.0);
            let y = (min_y - GROUP_PADDING_Y).max(0.0);
            group.x = Some(x);
            group.y = Some(y);
            group.width = Some(max_x - x + GROUP_PADDING_X);
            group.height = Some(max_y - y + GROUP_PADDING_Y);
        }
    }
}

// ── Step 6: route metadata ─────────────────────────────────────────────────

fn assign_route_plans(graph: &mut Graph) {
    let node_by_id: HashMap<String, Node> = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node.clone()))
        .collect();
    let id_to_idx: HashMap<String, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| (node.id.clone(), idx))
        .collect();

    let classes: Vec<EdgeClass> = graph
        .edges
        .iter()
        .map(|edge| classify_edge(edge, &node_by_id, &id_to_idx, &graph.direction))
        .collect();

    let mut source_keys = Vec::with_capacity(graph.edges.len());
    let mut target_keys = Vec::with_capacity(graph.edges.len());
    for (edge, class) in graph.edges.iter().zip(classes.iter()) {
        source_keys.push((
            edge.source.clone(),
            source_side(edge, class, &node_by_id, &graph.direction),
        ));
        target_keys.push((
            edge.target.clone(),
            target_side(edge, class, &node_by_id, &graph.direction),
        ));
    }

    let source_offsets = port_offsets(&source_keys);
    let target_offsets = port_offsets(&target_keys);
    let bundles = detect_bundles(graph, &node_by_id);
    let perimeter = graph_perimeter(&graph.nodes);
    let telemetry_degrees = telemetry_degrees(graph, &classes);
    let mut lane_allocator = LaneAllocator::default();

    for (idx, edge) in graph.edges.iter_mut().enumerate() {
        let (Some(src), Some(tgt)) = (node_by_id.get(&edge.source), node_by_id.get(&edge.target))
        else {
            edge.route = None;
            continue;
        };

        let source_side = source_keys[idx].1.clone();
        let target_side = target_keys[idx].1.clone();
        let source_port = make_port(src, source_side, source_offsets[idx]);
        let target_port = make_port(tgt, target_side, target_offsets[idx]);
        let class = &classes[idx];
        let bundle = bundle_for_edge(edge, &bundles);
        let (lane_id, points) = if let Some(bundle) = bundle {
            let lane_id = lane_allocator.bundle_lane(bundle.key.clone());
            let trunk_coord =
                bundle_trunk_coordinate(&graph.direction, bundle.kind, &source_port, &target_port);
            (
                Some(lane_id),
                orthogonal_points(&graph.direction, &source_port, &target_port, trunk_coord),
            )
        } else if *class == EdgeClass::Telemetry
            && should_route_telemetry_on_perimeter(edge, &telemetry_degrees)
        {
            let lane = lane_allocator.reserve_perimeter(&graph.direction, &perimeter);
            (
                Some(lane.id),
                telemetry_perimeter_points(
                    &graph.direction,
                    &source_port,
                    &target_port,
                    lane.coord,
                ),
            )
        } else {
            let lane = lane_allocator.reserve_between(&graph.direction, &source_port, &target_port);
            (
                Some(lane.id),
                orthogonal_points(&graph.direction, &source_port, &target_port, lane.coord),
            )
        };
        let label_anchor = route_midpoint(&points);

        edge.route = Some(RoutePlan {
            points,
            source_port,
            target_port,
            lane_id,
            class: classes[idx].clone(),
            label_anchor,
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BundleKind {
    SharedSink,
    SharedSource,
}

#[derive(Debug, Clone)]
struct BundleAssignment {
    key: (String, BundleKind),
    kind: BundleKind,
}

#[derive(Debug, Default)]
struct LaneAllocator {
    next_id: usize,
    band_counts: HashMap<BandKey, usize>,
    perimeter_counts: HashMap<Direction, usize>,
    bundle_ids: HashMap<(String, BundleKind), usize>,
}

#[derive(Debug, Clone, Copy)]
struct ReservedLane {
    id: usize,
    coord: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct GraphPerimeter {
    max_x: f64,
    max_y: f64,
}

fn graph_perimeter(nodes: &[Node]) -> GraphPerimeter {
    let mut perimeter = GraphPerimeter::default();
    for node in nodes {
        if node.id.starts_with("__dummy") {
            continue;
        }
        if let (Some(x), Some(y), Some(w), Some(h)) = (node.x, node.y, node.width, node.height) {
            perimeter.max_x = perimeter.max_x.max(x + w);
            perimeter.max_y = perimeter.max_y.max(y + h);
        }
    }
    perimeter
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BandKey {
    direction: Direction,
    start: i64,
    end: i64,
}

impl LaneAllocator {
    fn reserve_between(
        &mut self,
        direction: &Direction,
        source_port: &Port,
        target_port: &Port,
    ) -> ReservedLane {
        let (start, end, mid) = match direction {
            Direction::TopDown => (
                source_port.y.min(target_port.y).round() as i64,
                source_port.y.max(target_port.y).round() as i64,
                (source_port.y + target_port.y) / 2.0,
            ),
            Direction::LeftRight => (
                source_port.x.min(target_port.x).round() as i64,
                source_port.x.max(target_port.x).round() as i64,
                (source_port.x + target_port.x) / 2.0,
            ),
        };
        let key = BandKey {
            direction: direction.clone(),
            start,
            end,
        };
        let rank = self.band_counts.entry(key).or_insert(0);
        let coord = mid + lane_offset(*rank);
        *rank += 1;
        let id = self.alloc_id();
        ReservedLane { id, coord }
    }

    fn reserve_perimeter(
        &mut self,
        direction: &Direction,
        perimeter: &GraphPerimeter,
    ) -> ReservedLane {
        let rank = self.perimeter_counts.entry(direction.clone()).or_insert(0);
        let coord = match direction {
            Direction::TopDown => perimeter.max_x + 3.0 + *rank as f64,
            Direction::LeftRight => perimeter.max_y + 2.0 + *rank as f64,
        };
        *rank += 1;
        let id = self.alloc_id();
        ReservedLane { id, coord }
    }

    fn bundle_lane(&mut self, key: (String, BundleKind)) -> usize {
        if let Some(id) = self.bundle_ids.get(&key) {
            return *id;
        }
        let id = self.alloc_id();
        self.bundle_ids.insert(key, id);
        id
    }

    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

fn lane_offset(rank: usize) -> f64 {
    if rank == 0 {
        0.0
    } else {
        let step = (rank + 1) / 2;
        if rank % 2 == 1 {
            -(step as f64)
        } else {
            step as f64
        }
    }
}

fn detect_bundles(
    graph: &Graph,
    nodes: &HashMap<String, Node>,
) -> HashMap<(String, BundleKind), BundleAssignment> {
    let mut incoming: HashMap<String, usize> = HashMap::new();
    let mut outgoing: HashMap<String, usize> = HashMap::new();
    for edge in &graph.edges {
        *incoming.entry(edge.target.clone()).or_default() += 1;
        *outgoing.entry(edge.source.clone()).or_default() += 1;
    }

    let mut bundles = HashMap::new();
    for node in &graph.nodes {
        let in_degree = incoming.get(&node.id).copied().unwrap_or(0);
        let out_degree = outgoing.get(&node.id).copied().unwrap_or(0);
        let semantic_bus = is_semantic_bus_endpoint(node);

        // High fan-in sinks are the common "success/logs/metrics" wall in architecture
        // diagrams, so bundle them even when the label is not explicitly observability-ish.
        if in_degree >= 4 {
            let key = (node.id.clone(), BundleKind::SharedSink);
            bundles.insert(
                key.clone(),
                BundleAssignment {
                    key,
                    kind: BundleKind::SharedSink,
                },
            );
        }

        // Fan-out from ordinary routers is often the main content and should keep separate
        // lanes. Only collapse source trunks for semantic bus/queue/telemetry endpoints.
        if out_degree >= 4 && semantic_bus {
            let key = (node.id.clone(), BundleKind::SharedSource);
            bundles.insert(
                key.clone(),
                BundleAssignment {
                    key,
                    kind: BundleKind::SharedSource,
                },
            );
        }
    }

    // Also recognize semantic endpoints that were introduced as dummy-free parsed nodes but
    // have no matching node entry for some reason.
    for (id, degree) in incoming {
        if degree >= 4 && nodes.get(&id).is_some_and(is_semantic_bus_endpoint) {
            let key = (id, BundleKind::SharedSink);
            bundles.insert(
                key.clone(),
                BundleAssignment {
                    key,
                    kind: BundleKind::SharedSink,
                },
            );
        }
    }

    bundles
}

fn bundle_for_edge(
    edge: &Edge,
    bundles: &HashMap<(String, BundleKind), BundleAssignment>,
) -> Option<BundleAssignment> {
    bundles
        .get(&(edge.target.clone(), BundleKind::SharedSink))
        .cloned()
        .or_else(|| {
            bundles
                .get(&(edge.source.clone(), BundleKind::SharedSource))
                .cloned()
        })
}

fn is_semantic_bus_endpoint(node: &Node) -> bool {
    let text = format!("{} {}", node.id, node.label).to_lowercase();
    contains_bus_term(&text)
}

fn bundle_trunk_coordinate(
    direction: &Direction,
    kind: BundleKind,
    source_port: &Port,
    target_port: &Port,
) -> f64 {
    match (direction, kind) {
        (Direction::TopDown, BundleKind::SharedSink) => target_port.y - 2.0,
        (Direction::TopDown, BundleKind::SharedSource) => source_port.y + 2.0,
        (Direction::LeftRight, BundleKind::SharedSink) => target_port.x - 2.0,
        (Direction::LeftRight, BundleKind::SharedSource) => source_port.x + 2.0,
    }
}

fn classify_edge(
    edge: &Edge,
    nodes: &HashMap<String, Node>,
    id_to_idx: &HashMap<String, usize>,
    direction: &Direction,
) -> EdgeClass {
    let text = edge_text(edge, nodes);

    if is_back_edge(edge, nodes, id_to_idx, direction) {
        return EdgeClass::BackEdge;
    }
    if contains_any(&text, &["error", "fail", "failure", "deny", "denied"]) {
        return EdgeClass::Error;
    }
    if edge.style == EdgeStyle::Dashed
        || edge.style == EdgeStyle::Dotted
        || contains_telemetry_term(&text)
    {
        return EdgeClass::Telemetry;
    }
    if contains_any(
        &text,
        &[
            "external", "provider", "stripe", "sendgrid", "github", "slack",
        ],
    ) {
        return EdgeClass::External;
    }
    EdgeClass::Primary
}

fn edge_text(edge: &Edge, nodes: &HashMap<String, Node>) -> String {
    let mut text = String::new();
    text.push_str(&edge.source);
    text.push(' ');
    text.push_str(&edge.target);
    if let Some(label) = &edge.label {
        text.push(' ');
        text.push_str(label);
    }
    if let Some(node) = nodes.get(&edge.source) {
        text.push(' ');
        text.push_str(&node.label);
    }
    if let Some(node) = nodes.get(&edge.target) {
        text.push(' ');
        text.push_str(&node.label);
    }
    text.to_lowercase()
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn contains_telemetry_term(text: &str) -> bool {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(|token| {
            matches!(
                token,
                "log"
                    | "logs"
                    | "metric"
                    | "metrics"
                    | "trace"
                    | "traces"
                    | "alert"
                    | "alerts"
                    | "telemetry"
                    | "observability"
            ) || token.starts_with("monitor")
        })
}

fn contains_bus_term(text: &str) -> bool {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(|token| {
            matches!(token, "event" | "events" | "queue" | "bus") || contains_telemetry_term(token)
        })
}

fn telemetry_degrees(graph: &Graph, classes: &[EdgeClass]) -> HashMap<String, usize> {
    let mut degrees = HashMap::new();
    for (edge, class) in graph.edges.iter().zip(classes.iter()) {
        if *class == EdgeClass::Telemetry {
            *degrees.entry(edge.source.clone()).or_default() += 1;
            *degrees.entry(edge.target.clone()).or_default() += 1;
        }
    }
    degrees
}

fn should_route_telemetry_on_perimeter(
    edge: &Edge,
    telemetry_degrees: &HashMap<String, usize>,
) -> bool {
    const PERIMETER_TELEMETRY_DEGREE: usize = 3;
    telemetry_degrees
        .get(&edge.source)
        .copied()
        .unwrap_or(0)
        .max(telemetry_degrees.get(&edge.target).copied().unwrap_or(0))
        >= PERIMETER_TELEMETRY_DEGREE
}

fn is_back_edge(
    edge: &Edge,
    nodes: &HashMap<String, Node>,
    id_to_idx: &HashMap<String, usize>,
    direction: &Direction,
) -> bool {
    let (Some(src), Some(tgt)) = (nodes.get(&edge.source), nodes.get(&edge.target)) else {
        return false;
    };
    let main_src = match direction {
        Direction::TopDown => src.y,
        Direction::LeftRight => src.x,
    };
    let main_tgt = match direction {
        Direction::TopDown => tgt.y,
        Direction::LeftRight => tgt.x,
    };
    if let (Some(src_main), Some(tgt_main)) = (main_src, main_tgt) {
        return tgt_main < src_main;
    }
    match (id_to_idx.get(&edge.source), id_to_idx.get(&edge.target)) {
        (Some(src_idx), Some(tgt_idx)) => tgt_idx < src_idx,
        _ => false,
    }
}

fn source_side(
    edge: &Edge,
    class: &EdgeClass,
    nodes: &HashMap<String, Node>,
    direction: &Direction,
) -> PortSide {
    if *class == EdgeClass::BackEdge {
        return back_edge_side(edge, nodes, true, direction);
    }
    match direction {
        Direction::TopDown => PortSide::Bottom,
        Direction::LeftRight => PortSide::Right,
    }
}

fn target_side(
    edge: &Edge,
    class: &EdgeClass,
    nodes: &HashMap<String, Node>,
    direction: &Direction,
) -> PortSide {
    if *class == EdgeClass::BackEdge {
        return back_edge_side(edge, nodes, false, direction);
    }
    match direction {
        Direction::TopDown => PortSide::Top,
        Direction::LeftRight => PortSide::Left,
    }
}

fn back_edge_side(
    edge: &Edge,
    nodes: &HashMap<String, Node>,
    is_source: bool,
    direction: &Direction,
) -> PortSide {
    let (Some(src), Some(tgt)) = (nodes.get(&edge.source), nodes.get(&edge.target)) else {
        return match direction {
            Direction::TopDown => PortSide::Left,
            Direction::LeftRight => PortSide::Top,
        };
    };
    match direction {
        Direction::TopDown => {
            let src_x = src.x.unwrap_or(0.0);
            let tgt_x = tgt.x.unwrap_or(0.0);
            if (is_source && tgt_x >= src_x) || (!is_source && src_x < tgt_x) {
                PortSide::Right
            } else {
                PortSide::Left
            }
        }
        Direction::LeftRight => {
            let src_y = src.y.unwrap_or(0.0);
            let tgt_y = tgt.y.unwrap_or(0.0);
            if (is_source && tgt_y >= src_y) || (!is_source && src_y < tgt_y) {
                PortSide::Bottom
            } else {
                PortSide::Top
            }
        }
    }
}

fn port_offsets(keys: &[(String, PortSide)]) -> Vec<f64> {
    let mut grouped: HashMap<(String, PortSide), Vec<usize>> = HashMap::new();
    for (idx, key) in keys.iter().enumerate() {
        grouped.entry(key.clone()).or_default().push(idx);
    }

    let mut offsets = vec![0.5; keys.len()];
    for indices in grouped.values_mut() {
        indices.sort_unstable();
        let count = indices.len();
        for (rank, &idx) in indices.iter().enumerate() {
            offsets[idx] = (rank + 1) as f64 / (count + 1) as f64;
        }
    }
    offsets
}

fn make_port(node: &Node, side: PortSide, offset: f64) -> Port {
    let x = node.x.unwrap_or(0.0);
    let y = node.y.unwrap_or(0.0);
    let w = node.width.unwrap_or(MIN_WIDTH);
    let h = node.height.unwrap_or(MIN_HEIGHT);
    let (px, py) = match side {
        PortSide::Top => (x + w * offset, y - 1.0),
        PortSide::Right => (x + w, y + h * offset),
        PortSide::Bottom => (x + w * offset, y + h),
        PortSide::Left => (x - 1.0, y + h * offset),
    };
    Port { x: px, y: py, side }
}

fn orthogonal_points(
    direction: &Direction,
    source_port: &Port,
    target_port: &Port,
    lane_coord: f64,
) -> Vec<RoutePoint> {
    let mut points = vec![RoutePoint {
        x: source_port.x,
        y: source_port.y,
    }];

    match direction {
        Direction::TopDown => {
            points.push(RoutePoint {
                x: source_port.x,
                y: lane_coord,
            });
            points.push(RoutePoint {
                x: target_port.x,
                y: lane_coord,
            });
        }
        Direction::LeftRight => {
            points.push(RoutePoint {
                x: lane_coord,
                y: source_port.y,
            });
            points.push(RoutePoint {
                x: lane_coord,
                y: target_port.y,
            });
        }
    }

    points.push(RoutePoint {
        x: target_port.x,
        y: target_port.y,
    });
    points.dedup_by(|a, b| (a.x - b.x).abs() < 0.01 && (a.y - b.y).abs() < 0.01);
    points
}

fn telemetry_perimeter_points(
    direction: &Direction,
    source_port: &Port,
    target_port: &Port,
    perimeter_coord: f64,
) -> Vec<RoutePoint> {
    let mut points = vec![RoutePoint {
        x: source_port.x,
        y: source_port.y,
    }];

    match direction {
        Direction::TopDown => {
            points.push(RoutePoint {
                x: perimeter_coord,
                y: source_port.y,
            });
            points.push(RoutePoint {
                x: perimeter_coord,
                y: target_port.y,
            });
        }
        Direction::LeftRight => {
            points.push(RoutePoint {
                x: source_port.x,
                y: perimeter_coord,
            });
            points.push(RoutePoint {
                x: target_port.x,
                y: perimeter_coord,
            });
        }
    }

    points.push(RoutePoint {
        x: target_port.x,
        y: target_port.y,
    });
    points.dedup_by(|a, b| (a.x - b.x).abs() < 0.01 && (a.y - b.y).abs() < 0.01);
    points
}

fn route_midpoint(points: &[RoutePoint]) -> Option<RoutePoint> {
    points.get(points.len() / 2).cloned()
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
            assert!(node.x.is_some(), "node {} missing x", node.id);
            assert!(node.y.is_some(), "node {} missing y", node.id);
            assert!(node.width.is_some(), "node {} missing width", node.id);
            assert!(node.height.is_some(), "node {} missing height", node.id);
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
    fn test_subgraph_group_bounds_cover_members() {
        let mut g = crate::parser::mermaid::parse(
            r#"
            graph TD
            subgraph API[API Layer]
                A[Gateway] --> B[Service]
            end
            C[Outside]
            "#,
        )
        .unwrap();
        layout(&mut g);

        let group = &g.groups[0];
        let (gx, gy, gw, gh) = (
            group.x.unwrap(),
            group.y.unwrap(),
            group.width.unwrap(),
            group.height.unwrap(),
        );
        for node_id in ["A", "B"] {
            let node = find_node(&g, node_id);
            assert!(node.x.unwrap() >= gx);
            assert!(node.y.unwrap() >= gy);
            assert!(node.x.unwrap() + node.width.unwrap() <= gx + gw);
            assert!(node.y.unwrap() + node.height.unwrap() <= gy + gh);
        }
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
            groups: vec![],
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
            groups: vec![],
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
            groups: vec![],
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
    fn test_simple_layout_engine_matches_default_entrypoint() {
        let mut via_default = fixtures::diamond_decision();
        let mut via_engine = fixtures::diamond_decision();

        layout(&mut via_default);
        layout_with(&SimpleLayoutEngine, &mut via_engine);

        assert_eq!(via_engine, via_default);
    }

    #[test]
    fn test_layout_engine_trait_object() {
        let engine: &dyn LayoutEngine = &SimpleLayoutEngine;
        let mut g = fixtures::simple_two_node();

        layout_with(engine, &mut g);

        assert_all_positioned(&g);
    }

    #[test]
    fn test_empty_graph() {
        let mut g = Graph {
            direction: Direction::TopDown,
            nodes: vec![],
            edges: vec![],
            groups: vec![],
        };
        layout(&mut g); // should not panic
    }

    #[test]
    fn test_routed_edges_have_points() {
        let mut g = fixtures::diamond_decision();
        layout(&mut g);

        for edge in &g.edges {
            let route = edge.route.as_ref().expect("edge should have a route plan");
            assert!(
                route.points.len() >= 2,
                "edge {} -> {} should have at least two route points",
                edge.source,
                edge.target
            );
        }
    }

    #[test]
    fn test_fan_out_ports_are_distinct() {
        let mut g = fixtures::diamond_decision();
        layout(&mut g);

        let ports: Vec<_> = g
            .edges
            .iter()
            .filter(|edge| edge.source == "B")
            .map(|edge| edge.route.as_ref().unwrap().source_port.clone())
            .collect();

        assert_eq!(ports.len(), 2);
        assert_ne!(ports[0], ports[1], "fan-out source ports should differ");
    }

    #[test]
    fn test_fan_in_ports_are_distinct() {
        let mut g = Graph {
            direction: Direction::TopDown,
            nodes: vec![
                Node {
                    id: "A".into(),
                    label: "A".into(),
                    shape: NodeShape::Rectangle,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
                Node {
                    id: "B".into(),
                    label: "B".into(),
                    shape: NodeShape::Rectangle,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
                Node {
                    id: "C".into(),
                    label: "Sink".into(),
                    shape: NodeShape::Rectangle,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
            ],
            edges: vec![
                Edge {
                    source: "A".into(),
                    target: "C".into(),
                    label: None,
                    style: EdgeStyle::Solid,
                    arrowhead: Arrowhead::Normal,
                    route: None,
                },
                Edge {
                    source: "B".into(),
                    target: "C".into(),
                    label: None,
                    style: EdgeStyle::Solid,
                    arrowhead: Arrowhead::Normal,
                    route: None,
                },
            ],
            groups: vec![],
        };
        layout(&mut g);

        let ports: Vec<_> = g
            .edges
            .iter()
            .map(|edge| edge.route.as_ref().unwrap().target_port.clone())
            .collect();

        assert_eq!(ports.len(), 2);
        assert_ne!(ports[0], ports[1], "fan-in target ports should differ");
    }

    #[test]
    fn phase15_shared_sink_incoming_edges_share_trunk_coordinate() {
        let mut graph = crate::parser::mermaid::parse(fixtures::phase15_fan_in_sink_mermaid())
            .expect("phase 1.5 fan-in fixture should parse");
        layout(&mut graph);

        let incoming: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| edge.target == "SUCCESS")
            .map(|edge| {
                edge.route
                    .as_ref()
                    .expect("incoming sink edge should be routed")
            })
            .collect();
        assert!(
            incoming.len() >= 4,
            "fixture should exercise shared sink bundling"
        );

        let trunk_y = incoming[0].points[1].y;
        let lane_id = incoming[0].lane_id;
        for route in incoming {
            assert_eq!(
                route.lane_id, lane_id,
                "bundled sink edges should share a lane id"
            );
            assert!(
                (route.points[1].y - trunk_y).abs() < 0.01,
                "bundled sink edges should share the horizontal trunk y coordinate"
            );
            assert!(
                (route.points[2].y - trunk_y).abs() < 0.01,
                "bundled sink edges should stay on the shared horizontal trunk"
            );
        }
    }

    #[test]
    fn phase15_non_bundled_fan_out_edges_get_multiple_lane_ids() {
        let mut graph = crate::parser::mermaid::parse(fixtures::phase15_fan_out_router_mermaid())
            .expect("phase 1.5 fan-out fixture should parse");
        layout(&mut graph);

        let lane_ids: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| edge.source == "ROUTER")
            .map(|edge| edge.route.as_ref().and_then(|route| route.lane_id).unwrap())
            .collect();
        let unique_lane_ids: HashSet<_> = lane_ids.iter().copied().collect();

        assert!(
            lane_ids.len() >= 4,
            "fixture should exercise fan-out routing"
        );
        assert!(
            unique_lane_ids.len() > 1,
            "ordinary fan-out routes should reserve multiple lanes instead of one wall"
        );
    }

    #[test]
    fn login_is_not_misclassified_as_telemetry() {
        let mut graph = crate::parser::mermaid::parse(fixtures::simple_mermaid())
            .expect("simple fixture should parse");
        layout(&mut graph);

        let login_edge = graph
            .edges
            .iter()
            .find(|edge| edge.source == "AUTH" && edge.target == "LOGIN")
            .expect("simple fixture should include auth-to-login edge");
        assert_ne!(
            login_edge.route.as_ref().map(|route| &route.class),
            Some(&EdgeClass::Telemetry),
            "Login contains the letters 'log' but must not be routed as telemetry"
        );
    }

    #[test]
    fn single_telemetry_edge_uses_local_route_instead_of_perimeter_wall() {
        let mut graph = crate::parser::mermaid::parse(fixtures::simple_mermaid())
            .expect("simple fixture should parse");
        layout(&mut graph);

        let max_node_right = graph
            .nodes
            .iter()
            .filter(|node| !node.id.starts_with("__dummy"))
            .map(|node| node.x.unwrap() + node.width.unwrap())
            .fold(0.0_f64, f64::max);
        let telemetry_edge = graph
            .edges
            .iter()
            .find(|edge| edge.source == "WORKER" && edge.target == "METRICS")
            .expect("simple fixture should include worker-to-metrics telemetry edge");
        let route = telemetry_edge
            .route
            .as_ref()
            .expect("edge should have a route");

        assert_eq!(route.class, EdgeClass::Telemetry);
        assert!(
            route.points.iter().all(|point| point.x <= max_node_right),
            "one-off telemetry route should stay local instead of creating a perimeter side-wall"
        );
    }

    #[test]
    fn phase15_telemetry_overlay_edges_are_classified_telemetry() {
        let mut graph =
            crate::parser::mermaid::parse(fixtures::phase15_telemetry_overlay_mermaid())
                .expect("phase 1.5 telemetry fixture should parse");
        layout(&mut graph);

        let telemetry_edges: Vec<_> = graph
            .edges
            .iter()
            .filter(|edge| {
                edge.route
                    .as_ref()
                    .is_some_and(|route| route.class == EdgeClass::Telemetry)
            })
            .collect();
        assert!(
            telemetry_edges.len() >= 4,
            "fixture should include classified telemetry overlay edges"
        );
        let metrics_to_alerts = graph
            .edges
            .iter()
            .find(|edge| edge.source == "METRICS" && edge.target == "ALERTS")
            .expect("fixture should include metrics-to-alerts telemetry");
        assert_eq!(
            metrics_to_alerts.route.as_ref().map(|route| &route.class),
            Some(&EdgeClass::Telemetry)
        );
    }

    #[test]
    fn phase15_lr_telemetry_uses_lower_perimeter_route_when_not_bundled() {
        let mut graph =
            crate::parser::mermaid::parse(fixtures::phase15_telemetry_overlay_mermaid())
                .expect("phase 1.5 telemetry fixture should parse");
        layout(&mut graph);

        let max_node_bottom = graph
            .nodes
            .iter()
            .filter(|node| !node.id.starts_with("__dummy"))
            .map(|node| node.y.unwrap() + node.height.unwrap())
            .fold(0.0_f64, f64::max);
        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.source == "METRICS" && edge.target == "ALERTS")
            .expect("fixture should include non-bundled metrics-to-alerts telemetry");
        let route = edge.route.as_ref().expect("edge should have a route");

        assert_eq!(route.class, EdgeClass::Telemetry);
        assert!(
            route.points.iter().any(|point| point.y > max_node_bottom),
            "telemetry route should touch lower perimeter below node bounds"
        );
    }

    #[test]
    fn error_and_back_edge_classification_precedes_telemetry() {
        let mut graph = Graph {
            direction: Direction::LeftRight,
            nodes: vec![
                Node {
                    id: "API".into(),
                    label: "API".into(),
                    shape: NodeShape::Rectangle,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
                Node {
                    id: "LOGS".into(),
                    label: "Logs".into(),
                    shape: NodeShape::Rectangle,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
            ],
            edges: vec![
                Edge {
                    source: "API".into(),
                    target: "LOGS".into(),
                    label: Some("error telemetry".into()),
                    style: EdgeStyle::Dotted,
                    arrowhead: Arrowhead::Normal,
                    route: None,
                },
                Edge {
                    source: "LOGS".into(),
                    target: "API".into(),
                    label: Some("metric retry".into()),
                    style: EdgeStyle::Dotted,
                    arrowhead: Arrowhead::Normal,
                    route: None,
                },
            ],
            groups: vec![],
        };
        layout(&mut graph);

        let api_to_logs = graph
            .edges
            .iter()
            .find(|edge| edge.source == "API")
            .unwrap();
        let logs_to_api = graph
            .edges
            .iter()
            .find(|edge| edge.source == "LOGS")
            .unwrap();
        assert_eq!(api_to_logs.route.as_ref().unwrap().class, EdgeClass::Error);
        assert_eq!(
            logs_to_api.route.as_ref().unwrap().class,
            EdgeClass::BackEdge
        );
    }
}
