use super::LayoutEngine;
use crate::model::{
    Arrowhead, Direction, Edge, EdgeClass, EdgeStyle, Graph, Node, NodeShape, Port, PortSide,
    RoutePlan, RoutePoint,
};
use crate::stencil::node_display_cell_width;
use std::collections::{HashMap, HashSet, VecDeque};

const NODE_PADDING_X: f64 = 1.0;
const NODE_PADDING_Y: f64 = 0.0;
const MIN_NODE_WIDTH: f64 = 8.0;
const MIN_NODE_HEIGHT: f64 = 3.0;
const NODE_GAP_WITHIN_LAYER: f64 = 8.0;
const LAYER_GAP: f64 = 5.0;
const DIAMOND_WIDTH_FACTOR: f64 = 1.0;
const GROUP_PADDING_X: f64 = 2.0;
const GROUP_PADDING_Y: f64 = 1.0;
const DUMMY_NODE_PREFIX: &str = "__dummy";
const BARYCENTER_SWEEPS: usize = 4;
const SHARED_SINK_BUNDLE_MIN_DEGREE: usize = 4;
const SHARED_SOURCE_BUNDLE_MIN_DEGREE: usize = 4;
const PERIMETER_TELEMETRY_DEGREE: usize = 3;

#[derive(Debug, Default, Clone, Copy)]
pub struct SimpleLayoutEngine;

impl LayoutEngine for SimpleLayoutEngine {
    fn layout(&self, graph: &mut Graph) {
        layout_graph(graph);
    }
}

fn layout_graph(graph: &mut Graph) {
    if graph.nodes.is_empty() {
        return;
    }

    size_nodes(&mut graph.nodes);

    let node_indices = node_indices_by_id(&graph.nodes);
    let mut layers = assign_layers(graph, &node_indices);
    insert_dummies(graph, &mut layers, &node_indices);

    let node_indices = node_indices_by_id(&graph.nodes);
    let ordered_layers = order_layers(&layers, graph, &node_indices);

    assign_positions(graph, &ordered_layers);
    assign_group_bounds(graph);
    assign_route_plans(graph);
}

fn node_indices_by_id(nodes: &[Node]) -> HashMap<String, usize> {
    nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect()
}

fn is_dummy_id(id: &str) -> bool {
    id.starts_with(DUMMY_NODE_PREFIX)
}

fn is_dummy_node(node: &Node) -> bool {
    is_dummy_id(&node.id)
}

fn size_nodes(nodes: &mut [Node]) {
    for node in nodes.iter_mut() {
        let label_width = label_cell_width(node);
        let (width, height) = node_shape_size(node, label_width);

        node.width = Some(width.max(MIN_NODE_WIDTH));
        node.height = Some(height.max(MIN_NODE_HEIGHT));
    }
}

fn label_cell_width(node: &Node) -> f64 {
    node_display_cell_width(node) as f64
}

fn node_shape_size(node: &Node, label_width: f64) -> (f64, f64) {
    let width = match node.shape {
        NodeShape::Diamond => (label_width + NODE_PADDING_X * 2.0 + 2.0) * DIAMOND_WIDTH_FACTOR,
        NodeShape::Circle | NodeShape::Rectangle | NodeShape::RoundedRect | NodeShape::Database => {
            label_width + NODE_PADDING_X * 2.0 + 2.0
        }
    };

    (round_up_to_even(width), 1.0 + NODE_PADDING_Y * 2.0)
}

fn round_up_to_even(value: f64) -> f64 {
    let mut rounded = value.ceil();
    if rounded as i64 % 2 != 0 {
        rounded += 1.0;
    }
    rounded
}

fn assign_layers(graph: &Graph, node_indices_by_id: &HashMap<String, usize>) -> Vec<Vec<usize>> {
    let adjacency = build_adjacency(graph, node_indices_by_id);
    let mut order = topological_order(&adjacency);
    append_missing_nodes(&mut order, graph.nodes.len());

    let parents_by_node = parents_by_node(&adjacency.children_by_node);
    let layer_by_node = assign_layer_by_node(&order, &parents_by_node);

    group_nodes_by_layer(&layer_by_node)
}

#[derive(Debug, Clone)]
struct Adjacency {
    children_by_node: Vec<Vec<usize>>,
    incoming_count_by_node: Vec<usize>,
}

fn build_adjacency(graph: &Graph, node_indices_by_id: &HashMap<String, usize>) -> Adjacency {
    let node_count = graph.nodes.len();
    let mut children_by_node = vec![vec![]; node_count];
    let mut incoming_count_by_node = vec![0; node_count];

    for edge in &graph.edges {
        let Some(&source_index) = node_indices_by_id.get(&edge.source) else {
            continue;
        };
        let Some(&target_index) = node_indices_by_id.get(&edge.target) else {
            continue;
        };

        children_by_node[source_index].push(target_index);
        incoming_count_by_node[target_index] += 1;
    }

    Adjacency {
        children_by_node,
        incoming_count_by_node,
    }
}

fn topological_order(adjacency: &Adjacency) -> Vec<usize> {
    let node_count = adjacency.children_by_node.len();
    let mut order = Vec::with_capacity(node_count);
    let mut remaining_parent_count = adjacency.incoming_count_by_node.clone();
    let mut ready_nodes = VecDeque::new();

    for (node_index, &parent_count) in remaining_parent_count.iter().enumerate() {
        if parent_count == 0 {
            ready_nodes.push_back(node_index);
        }
    }

    while let Some(node_index) = ready_nodes.pop_front() {
        order.push(node_index);

        for &child_index in &adjacency.children_by_node[node_index] {
            remaining_parent_count[child_index] -= 1;
            if remaining_parent_count[child_index] == 0 {
                ready_nodes.push_back(child_index);
            }
        }
    }

    order
}

fn append_missing_nodes(order: &mut Vec<usize>, node_count: usize) {
    let ordered_nodes: HashSet<usize> = order.iter().copied().collect();
    for node_index in 0..node_count {
        if !ordered_nodes.contains(&node_index) {
            order.push(node_index);
        }
    }
}

fn parents_by_node(children_by_node: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut parents_by_node = vec![vec![]; children_by_node.len()];
    for (parent_index, children) in children_by_node.iter().enumerate() {
        for &child_index in children {
            parents_by_node[child_index].push(parent_index);
        }
    }
    parents_by_node
}

fn assign_layer_by_node(order: &[usize], parents_by_node: &[Vec<usize>]) -> Vec<usize> {
    let mut layer_by_node = vec![0; parents_by_node.len()];
    for &node_index in order {
        layer_by_node[node_index] = parents_by_node[node_index]
            .iter()
            .map(|&parent_index| layer_by_node[parent_index] + 1)
            .max()
            .unwrap_or(0);
    }
    layer_by_node
}

fn group_nodes_by_layer(layer_by_node: &[usize]) -> Vec<Vec<usize>> {
    let max_layer = layer_by_node.iter().copied().max().unwrap_or(0);
    let mut nodes_by_layer = vec![vec![]; max_layer + 1];

    for (node_index, &layer) in layer_by_node.iter().enumerate() {
        nodes_by_layer[layer].push(node_index);
    }

    nodes_by_layer
}

fn insert_dummies(
    graph: &mut Graph,
    layers: &mut Vec<Vec<usize>>,
    node_indices_by_id: &HashMap<String, usize>,
) {
    let layer_by_node = layer_by_node_index(layers);
    let mut known_node_indices = node_indices_by_id.clone();
    let mut replacement_edges = Vec::new();
    let mut removed_edge_indices = HashSet::new();
    let original_edges = graph.edges.clone();

    for (edge_index, edge) in original_edges.iter().enumerate() {
        let Some(&source_index) = known_node_indices.get(&edge.source) else {
            continue;
        };
        let Some(&target_index) = known_node_indices.get(&edge.target) else {
            continue;
        };

        let source_layer = *layer_by_node.get(&source_index).unwrap_or(&0);
        let target_layer = *layer_by_node.get(&target_index).unwrap_or(&0);
        if target_layer <= source_layer + 1 {
            continue;
        }

        removed_edge_indices.insert(edge_index);
        replacement_edges.extend(split_long_edge_with_dummies(
            graph,
            layers,
            &mut known_node_indices,
            edge,
            source_layer,
            target_layer,
        ));
    }

    let mut retained_edges = Vec::new();
    for (edge_index, edge) in graph.edges.drain(..).enumerate() {
        if !removed_edge_indices.contains(&edge_index) {
            retained_edges.push(edge);
        }
    }

    retained_edges.extend(replacement_edges);
    graph.edges = retained_edges;
}

fn layer_by_node_index(layers: &[Vec<usize>]) -> HashMap<usize, usize> {
    let mut layer_by_node = HashMap::new();
    for (layer_index, layer) in layers.iter().enumerate() {
        for &node_index in layer {
            layer_by_node.insert(node_index, layer_index);
        }
    }
    layer_by_node
}

fn split_long_edge_with_dummies(
    graph: &mut Graph,
    layers: &mut Vec<Vec<usize>>,
    known_node_indices: &mut HashMap<String, usize>,
    edge: &Edge,
    source_layer: usize,
    target_layer: usize,
) -> Vec<Edge> {
    let mut segment_edges = Vec::new();
    let mut segment_source = edge.source.clone();

    for layer in (source_layer + 1)..target_layer {
        let dummy_id = format!(
            "{}_{}_{}_{}",
            DUMMY_NODE_PREFIX, edge.source, edge.target, layer
        );
        ensure_dummy_node(graph, layers, known_node_indices, &dummy_id, layer);

        segment_edges.push(Edge {
            source: segment_source.clone(),
            target: dummy_id.clone(),
            label: first_segment_label(edge, &segment_source),
            style: edge.style.clone(),
            arrowhead: Arrowhead::None,
            route: None,
        });

        segment_source = dummy_id;
    }

    segment_edges.push(Edge {
        source: segment_source,
        target: edge.target.clone(),
        label: None,
        style: edge.style.clone(),
        arrowhead: edge.arrowhead.clone(),
        route: None,
    });

    segment_edges
}

fn ensure_dummy_node(
    graph: &mut Graph,
    layers: &mut Vec<Vec<usize>>,
    known_node_indices: &mut HashMap<String, usize>,
    dummy_id: &str,
    layer: usize,
) {
    if known_node_indices.contains_key(dummy_id) {
        return;
    }

    let dummy_index = graph.nodes.len();
    graph.nodes.push(Node {
        id: dummy_id.to_string(),
        label: String::new(),
        shape: NodeShape::Rectangle,
        x: None,
        y: None,
        width: Some(0.0),
        height: Some(0.0),
    });

    while layers.len() <= layer {
        layers.push(vec![]);
    }
    layers[layer].push(dummy_index);
    known_node_indices.insert(dummy_id.to_string(), dummy_index);
}

fn first_segment_label(edge: &Edge, segment_source: &str) -> Option<String> {
    if segment_source == edge.source {
        edge.label.clone()
    } else {
        None
    }
}

fn order_layers(
    layers: &[Vec<usize>],
    graph: &Graph,
    node_indices_by_id: &HashMap<String, usize>,
) -> Vec<Vec<usize>> {
    if layers.is_empty() {
        return vec![];
    }

    let adjacency = build_adjacency(graph, node_indices_by_id);
    let neighbors = LayerNeighbors {
        parents_by_node: parents_by_node(&adjacency.children_by_node),
        children_by_node: adjacency.children_by_node,
    };
    let mut ordered_layers = layers.to_vec();
    let mut position_by_node = positions_by_node(&ordered_layers, graph.nodes.len());

    for _ in 0..BARYCENTER_SWEEPS {
        sweep_layers_down(&mut ordered_layers, &mut position_by_node, &neighbors);
        sweep_layers_up(&mut ordered_layers, &mut position_by_node, &neighbors);
    }

    ordered_layers
}

#[derive(Debug, Clone)]
struct LayerNeighbors {
    parents_by_node: Vec<Vec<usize>>,
    children_by_node: Vec<Vec<usize>>,
}

fn positions_by_node(layers: &[Vec<usize>], node_count: usize) -> Vec<usize> {
    let mut position_by_node = vec![0; node_count];
    for layer in layers {
        refresh_positions_for_layer(layer, &mut position_by_node);
    }
    position_by_node
}

fn sweep_layers_down(
    layers: &mut [Vec<usize>],
    position_by_node: &mut [usize],
    neighbors: &LayerNeighbors,
) {
    for layer in layers.iter_mut().skip(1) {
        *layer = reorder_layer_by_barycenter(layer, &neighbors.parents_by_node, position_by_node);
        refresh_positions_for_layer(layer, position_by_node);
    }
}

fn sweep_layers_up(
    layers: &mut [Vec<usize>],
    position_by_node: &mut [usize],
    neighbors: &LayerNeighbors,
) {
    for layer_index in (0..layers.len().saturating_sub(1)).rev() {
        layers[layer_index] = reorder_layer_by_barycenter(
            &layers[layer_index],
            &neighbors.children_by_node,
            position_by_node,
        );
        refresh_positions_for_layer(&layers[layer_index], position_by_node);
    }
}

fn reorder_layer_by_barycenter(
    layer: &[usize],
    neighbors_by_node: &[Vec<usize>],
    position_by_node: &[usize],
) -> Vec<usize> {
    let mut weighted_nodes: Vec<(usize, f64)> = layer
        .iter()
        .map(|&node_index| {
            (
                node_index,
                barycenter(node_index, &neighbors_by_node[node_index], position_by_node),
            )
        })
        .collect();

    weighted_nodes.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    weighted_nodes
        .into_iter()
        .map(|(node_index, _)| node_index)
        .collect()
}

fn barycenter(node_index: usize, neighbors: &[usize], position_by_node: &[usize]) -> f64 {
    if neighbors.is_empty() {
        return position_by_node[node_index] as f64;
    }

    neighbors
        .iter()
        .map(|&neighbor_index| position_by_node[neighbor_index] as f64)
        .sum::<f64>()
        / neighbors.len() as f64
}

fn refresh_positions_for_layer(layer: &[usize], position_by_node: &mut [usize]) {
    for (position, &node_index) in layer.iter().enumerate() {
        position_by_node[node_index] = position;
    }
}

fn assign_positions(graph: &mut Graph, layers: &[Vec<usize>]) {
    if layers.is_empty() {
        return;
    }

    let left_to_right = graph.direction == Direction::LeftRight;
    let node_sizes = layout_sizes(&graph.nodes);
    let layer_cross_spans = layer_cross_spans(layers, &node_sizes, left_to_right);
    let max_cross_span = layer_cross_spans.iter().copied().fold(0.0_f64, f64::max);
    let mut main_cursor = 0.0;

    for (layer_index, layer) in layers.iter().enumerate() {
        let layer_main_extent = layer_main_extent(layer, &node_sizes, left_to_right);
        let mut cross_cursor = (max_cross_span - layer_cross_spans[layer_index]) / 2.0;

        for &node_index in layer {
            let (x, y) = node_coordinates(
                left_to_right,
                main_cursor,
                cross_cursor,
                layer_main_extent,
                is_dummy_node(&graph.nodes[node_index]),
            );

            graph.nodes[node_index].x = Some(x);
            graph.nodes[node_index].y = Some(y);
            cross_cursor +=
                cross_axis_extent(node_sizes[node_index], left_to_right) + NODE_GAP_WITHIN_LAYER;
        }

        main_cursor += layer_main_extent + LAYER_GAP;
    }

    align_lone_targets_after_dummy_routes(graph, layers, left_to_right);
}

#[derive(Debug, Clone, Copy)]
struct CellSize {
    width: f64,
    height: f64,
}

fn layout_sizes(nodes: &[Node]) -> Vec<CellSize> {
    nodes.iter().map(layout_size).collect()
}

fn layout_size(node: &Node) -> CellSize {
    if is_dummy_node(node) {
        return CellSize {
            width: 0.0,
            height: 0.0,
        };
    }

    CellSize {
        width: node.width.unwrap_or(MIN_NODE_WIDTH),
        height: node.height.unwrap_or(MIN_NODE_HEIGHT),
    }
}

fn layer_cross_spans(
    layers: &[Vec<usize>],
    node_sizes: &[CellSize],
    left_to_right: bool,
) -> Vec<f64> {
    layers
        .iter()
        .map(|layer| layer_cross_span(layer, node_sizes, left_to_right))
        .collect()
}

fn layer_cross_span(layer: &[usize], node_sizes: &[CellSize], left_to_right: bool) -> f64 {
    layer
        .iter()
        .enumerate()
        .map(|(position, &node_index)| {
            let gap = if position + 1 < layer.len() {
                NODE_GAP_WITHIN_LAYER
            } else {
                0.0
            };
            cross_axis_extent(node_sizes[node_index], left_to_right) + gap
        })
        .sum()
}

fn layer_main_extent(layer: &[usize], node_sizes: &[CellSize], left_to_right: bool) -> f64 {
    layer
        .iter()
        .map(|&node_index| main_axis_extent(node_sizes[node_index], left_to_right))
        .fold(0.0_f64, f64::max)
}

fn main_axis_extent(size: CellSize, left_to_right: bool) -> f64 {
    if left_to_right {
        size.width
    } else {
        size.height
    }
}

fn cross_axis_extent(size: CellSize, left_to_right: bool) -> f64 {
    if left_to_right {
        size.height
    } else {
        size.width
    }
}

fn node_coordinates(
    left_to_right: bool,
    main_cursor: f64,
    cross_cursor: f64,
    layer_main_extent: f64,
    is_dummy: bool,
) -> (f64, f64) {
    let main_position = if is_dummy {
        main_cursor + layer_main_extent / 2.0
    } else {
        main_cursor
    };

    if left_to_right {
        (main_position, cross_cursor)
    } else {
        (cross_cursor, main_position)
    }
}

fn align_lone_targets_after_dummy_routes(
    graph: &mut Graph,
    layers: &[Vec<usize>],
    left_to_right: bool,
) {
    let node_indices = node_indices_by_id(&graph.nodes);
    let layer_by_node = layer_by_node_index(layers);

    let edges = graph.edges.clone();
    for edge in &edges {
        if !is_dummy_id(&edge.source) {
            continue;
        }

        let (Some(&dummy_index), Some(&target_index)) = (
            node_indices.get(&edge.source),
            node_indices.get(&edge.target),
        ) else {
            continue;
        };

        if is_dummy_node(&graph.nodes[target_index]) {
            continue;
        }

        let Some(&target_layer) = layer_by_node.get(&target_index) else {
            continue;
        };

        if visible_node_count(&graph.nodes, &layers[target_layer]) != 1 {
            continue;
        }

        align_target_with_dummy(graph, dummy_index, target_index, left_to_right);
    }
}

fn visible_node_count(nodes: &[Node], layer: &[usize]) -> usize {
    layer
        .iter()
        .filter(|&&node_index| !is_dummy_node(&nodes[node_index]))
        .count()
}

fn align_target_with_dummy(
    graph: &mut Graph,
    dummy_index: usize,
    target_index: usize,
    left_to_right: bool,
) {
    if left_to_right {
        if let (Some(dummy_y), Some(target_height)) =
            (graph.nodes[dummy_index].y, graph.nodes[target_index].height)
        {
            graph.nodes[target_index].y = Some((dummy_y - target_height / 2.0).max(0.0));
        }
    } else if let (Some(dummy_x), Some(target_width)) =
        (graph.nodes[dummy_index].x, graph.nodes[target_index].width)
    {
        graph.nodes[target_index].x = Some((dummy_x - target_width / 2.0).max(0.0));
    }
}

fn assign_group_bounds(graph: &mut Graph) {
    if graph.groups.is_empty() {
        return;
    }

    let node_by_id: HashMap<String, &Node> = graph
        .nodes
        .iter()
        .filter(|node| !is_dummy_node(node))
        .map(|node| (node.id.clone(), node))
        .collect();

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

fn assign_route_plans(graph: &mut Graph) {
    let node_by_id: HashMap<String, Node> = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node.clone()))
        .collect();
    let node_indices = node_indices_by_id(&graph.nodes);

    let classes: Vec<EdgeClass> = graph
        .edges
        .iter()
        .map(|edge| classify_edge(edge, &node_by_id, &node_indices, &graph.direction))
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
        let (lane_id, points) = if *class == EdgeClass::BackEdge {
            let lane = lane_allocator.reserve_perimeter(&graph.direction, &perimeter);
            (
                Some(lane.id),
                perimeter_points(&graph.direction, &source_port, &target_port, lane.coord),
            )
        } else if let Some(bundle) = bundle {
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
                perimeter_points(&graph.direction, &source_port, &target_port, lane.coord),
            )
        } else {
            let lane = lane_allocator.reserve_between(&graph.direction, &source_port, &target_port);
            (
                Some(lane.id),
                orthogonal_points(&graph.direction, &source_port, &target_port, lane.coord),
            )
        };
        let label_anchor = route_label_anchor(edge.label.as_deref(), &points);

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
        if is_dummy_node(node) {
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
            Direction::TopDown => perimeter.max_x + NODE_GAP_WITHIN_LAYER + *rank as f64,
            Direction::LeftRight => perimeter.max_y + LAYER_GAP + *rank as f64,
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
        let step = rank.div_ceil(2);
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
        let incoming_count = incoming.get(&node.id).copied().unwrap_or(0);
        let outgoing_count = outgoing.get(&node.id).copied().unwrap_or(0);
        let semantic_bus = is_semantic_bus_endpoint(node);

        if incoming_count >= SHARED_SINK_BUNDLE_MIN_DEGREE {
            insert_bundle(&mut bundles, node.id.clone(), BundleKind::SharedSink);
        }

        if outgoing_count >= SHARED_SOURCE_BUNDLE_MIN_DEGREE && semantic_bus {
            insert_bundle(&mut bundles, node.id.clone(), BundleKind::SharedSource);
        }
    }

    for (id, incoming_count) in incoming {
        if incoming_count >= SHARED_SINK_BUNDLE_MIN_DEGREE
            && nodes.get(&id).is_some_and(is_semantic_bus_endpoint)
        {
            insert_bundle(&mut bundles, id, BundleKind::SharedSink);
        }
    }

    bundles
}

fn insert_bundle(
    bundles: &mut HashMap<(String, BundleKind), BundleAssignment>,
    node_id: String,
    kind: BundleKind,
) {
    let key = (node_id, kind);
    bundles.insert(key.clone(), BundleAssignment { key, kind });
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
    _edge: &Edge,
    _nodes: &HashMap<String, Node>,
    _is_source: bool,
    direction: &Direction,
) -> PortSide {
    match direction {
        Direction::TopDown => PortSide::Right,
        Direction::LeftRight => PortSide::Bottom,
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
    let w = node.width.unwrap_or(MIN_NODE_WIDTH);
    let h = node.height.unwrap_or(MIN_NODE_HEIGHT);
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

fn perimeter_points(
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

fn route_label_anchor(label: Option<&str>, points: &[RoutePoint]) -> Option<RoutePoint> {
    let first = points.first()?;
    if points.len() == 1 {
        return Some(first.clone());
    }

    let (mut anchor, is_vertical) = longest_segment_anchor(points);
    if is_vertical {
        shift_vertical_label_anchor(&mut anchor, label);
    }

    Some(anchor)
}

fn longest_segment_anchor(points: &[RoutePoint]) -> (RoutePoint, bool) {
    let mut anchor = points[0].clone();
    let mut length = -1.0_f64;
    let mut is_vertical = false;

    for segment in points.windows(2) {
        let a = &segment[0];
        let b = &segment[1];
        let dx = (a.x - b.x).abs();
        let dy = (a.y - b.y).abs();
        let segment_length = dx + dy;

        if segment_length > length {
            length = segment_length;
            is_vertical = dy > dx;
            anchor = RoutePoint {
                x: (a.x + b.x) / 2.0,
                y: (a.y + b.y) / 2.0,
            };
        }
    }

    (anchor, is_vertical)
}

fn shift_vertical_label_anchor(anchor: &mut RoutePoint, label: Option<&str>) {
    if let Some(label) = label {
        anchor.x += 1.0 + label.chars().count() as f64 / 2.0;
        anchor.y += 1.0;
    }
}
