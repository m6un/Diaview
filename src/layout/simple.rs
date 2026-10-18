use super::LayoutEngine;
use crate::model::{
    Arrowhead, Direction, Edge, EdgeClass, EdgeStyle, Graph, Node, NodeShape, Port, PortSide,
    RoutePlan, RoutePoint,
};
use crate::stencil::node_display_cell_width;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap, HashMap, HashSet, VecDeque},
};

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
const SHARED_SINK_BUNDLE_MIN_DEGREE: usize = 3;
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
    let source_keys: Vec<_> = graph
        .edges
        .iter()
        .zip(&classes)
        .map(|(edge, class)| {
            (
                edge.source.clone(),
                source_side(edge, class, &node_by_id, &graph.direction),
            )
        })
        .collect();
    let target_keys: Vec<_> = graph
        .edges
        .iter()
        .zip(&classes)
        .map(|(edge, class)| {
            (
                edge.target.clone(),
                target_side(edge, class, &node_by_id, &graph.direction),
            )
        })
        .collect();
    let source_offsets = port_offsets(&source_keys);
    let target_offsets = port_offsets(&target_keys);
    let bundles = detect_bundles(graph);
    let perimeter = graph_perimeter(&graph.nodes);
    let telemetry_degrees = telemetry_degrees(graph, &classes);
    let mut lane_allocator = LaneAllocator::default();
    let mut specs = Vec::with_capacity(graph.edges.len());
    let backedge_target_degrees: HashMap<String, usize> = graph
        .edges
        .iter()
        .zip(&classes)
        .filter(|(_, class)| **class == EdgeClass::BackEdge)
        .fold(HashMap::new(), |mut counts, (edge, _)| {
            *counts.entry(edge.target.clone()).or_insert(0) += 1;
            counts
        });

    for (idx, edge) in graph.edges.iter().enumerate() {
        let (Some(src), Some(tgt)) = (node_by_id.get(&edge.source), node_by_id.get(&edge.target))
        else {
            specs.push(None);
            continue;
        };
        let source_port = make_port(src, source_keys[idx].1.clone(), source_offsets[idx]);
        let target_port = make_port(tgt, target_keys[idx].1.clone(), target_offsets[idx]);
        let class = &classes[idx];
        let bundle = bundle_for_edge(edge, &bundles);
        let (lane_id, preferred_lane, required_lane) = if *class == EdgeClass::BackEdge {
            let lane = if backedge_target_degrees
                .get(&edge.target)
                .copied()
                .unwrap_or(0)
                >= 3
            {
                lane_allocator.reserve_backedge_bus(tgt, &graph.direction)
            } else {
                lane_allocator.reserve_perimeter(&graph.direction, &perimeter)
            };
            let route_lane = perimeter_route_lane(&graph.direction, lane.coord);
            (Some(lane.id), route_lane, Some(route_lane))
        } else if let Some(bundle) = bundle {
            let route_lane =
                bundle_route_lane(&graph.direction, bundle.kind, &source_port, &target_port);
            (
                Some(lane_allocator.bundle_lane(bundle.key)),
                route_lane,
                Some(route_lane),
            )
        } else if *class == EdgeClass::Telemetry
            && should_route_telemetry_on_perimeter(edge, &telemetry_degrees)
        {
            let lane = lane_allocator.reserve_perimeter(&graph.direction, &perimeter);
            let route_lane = perimeter_route_lane(&graph.direction, lane.coord);
            (Some(lane.id), route_lane, Some(route_lane))
        } else {
            let lane = lane_allocator.reserve_between(&graph.direction, &source_port, &target_port);
            (
                Some(lane.id),
                local_route_lane(&graph.direction, lane.coord),
                None,
            )
        };
        specs.push(Some(RouteSpec {
            preferred: lane_route_points(&source_port, &target_port, preferred_lane),
            source_port,
            target_port,
            lane_id,
            required_lane,
        }));
    }

    let grid = RoutingGrid::new(&specs, &graph.nodes, &perimeter);
    let mut route_occupancy = RouteOccupancy::default();
    for (idx, edge) in graph.edges.iter_mut().enumerate() {
        let Some(spec) = &specs[idx] else {
            edge.route = None;
            continue;
        };
        let identity = RouteIdentity::new(edge, spec);
        let Some(points) = route_points(
            spec.preferred.clone(),
            &spec.source_port,
            &spec.target_port,
            &graph.nodes,
            &grid,
            spec.required_lane,
            &identity,
            &route_occupancy,
        ) else {
            edge.route = None;
            continue;
        };
        route_occupancy.add(identity, &points);
        edge.route = Some(RoutePlan {
            points,
            source_port: spec.source_port.clone(),
            target_port: spec.target_port.clone(),
            lane_id: spec.lane_id,
            class: classes[idx].clone(),
            label_anchor: None,
        });
    }

    // Post-pass: one deterministic sequential sweep. Reroute each edge
    // against the occupancy of all other routed edges, keeping only strictly
    // cheaper candidates. Routes already conflict-free are skipped.
    for idx in 0..graph.edges.len() {
        let Some(spec) = &specs[idx] else { continue };
        let Some(current_points) = graph.edges[idx]
            .route
            .as_ref()
            .map(|route| route.points.clone())
        else {
            continue;
        };
        let identity = RouteIdentity::new(&graph.edges[idx], spec);
        let others = occupancy_excluding(graph, &specs, idx);
        let current_cost = route_conflict_cost(&current_points, &identity, &others);
        if current_cost == 0 {
            continue;
        }
        let Some(candidate) = route_points(
            spec.preferred.clone(),
            &spec.source_port,
            &spec.target_port,
            &graph.nodes,
            &grid,
            spec.required_lane,
            &identity,
            &others,
        ) else {
            continue;
        };
        if route_conflict_cost(&candidate, &identity, &others) < current_cost {
            graph.edges[idx].route.as_mut().unwrap().points = candidate;
        }
    }
    assign_route_label_anchors(graph);
}

fn occupancy_excluding(graph: &Graph, specs: &[Option<RouteSpec>], skip: usize) -> RouteOccupancy {
    let mut occupancy = RouteOccupancy::default();
    for (idx, edge) in graph.edges.iter().enumerate() {
        if idx == skip {
            continue;
        }
        let (Some(spec), Some(route)) = (&specs[idx], &edge.route) else {
            continue;
        };
        occupancy.add(RouteIdentity::new(edge, spec), &route.points);
    }
    occupancy
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
    perimeter_by_target: HashMap<String, ReservedLane>,
    bundle_ids: HashMap<(String, BundleKind), usize>,
}

#[derive(Debug, Clone, Copy)]
struct ReservedLane {
    id: usize,
    coord: f64,
}

#[derive(Debug, Clone, Copy)]
enum RouteLane {
    Vertical(f64),
    Horizontal(f64),
}

struct RouteSpec {
    preferred: Vec<RoutePoint>,
    source_port: Port,
    target_port: Port,
    lane_id: Option<usize>,
    required_lane: Option<RouteLane>,
}

struct RoutingGrid {
    xs: Vec<f64>,
    ys: Vec<f64>,
    clear: Vec<bool>,
    right_clear: Vec<bool>,
    down_clear: Vec<bool>,
}

#[derive(Clone)]
struct RouteIdentity {
    source: String,
    target: String,
    lane_id: Option<usize>,
    source_cells: [(i64, i64); 2],
    target_cells: [(i64, i64); 2],
}

#[derive(Default)]
struct RouteOccupancy {
    routes: Vec<RouteIdentity>,
    horizontal: BTreeMap<i64, Vec<IndexedSegment>>,
    vertical: BTreeMap<i64, Vec<IndexedSegment>>,
}

#[derive(Clone, Copy)]
struct IndexedSegment {
    route: usize,
    segment: RasterSegment,
}

#[derive(Clone, Copy)]
struct RasterSegment {
    horizontal: bool,
    fixed: i64,
    start: i64,
    end: i64,
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

    /// One shared lane per repeated-feedback BackEdge target node (>= 3
    /// same-target back-edges): a local bus just outside the target's
    /// return side (TD: right of target, LR: below target). Distinct
    /// targets still get distinct lanes.
    fn reserve_backedge_bus(&mut self, target: &Node, direction: &Direction) -> ReservedLane {
        if let Some(lane) = self.perimeter_by_target.get(&target.id) {
            return *lane;
        }
        let coord = match direction {
            Direction::TopDown => {
                target.x.unwrap_or(0.0) + target.width.unwrap_or(MIN_NODE_WIDTH) + 1.0
            }
            Direction::LeftRight => {
                target.y.unwrap_or(0.0) + target.height.unwrap_or(MIN_NODE_HEIGHT) + 1.0
            }
        };
        let id = self.alloc_id();
        let lane = ReservedLane { id, coord };
        self.perimeter_by_target.insert(target.id.clone(), lane);
        lane
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

fn detect_bundles(graph: &Graph) -> HashMap<(String, BundleKind), BundleAssignment> {
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

fn local_route_lane(direction: &Direction, coordinate: f64) -> RouteLane {
    match direction {
        Direction::TopDown => RouteLane::Horizontal(coordinate),
        Direction::LeftRight => RouteLane::Vertical(coordinate),
    }
}

fn perimeter_route_lane(direction: &Direction, coordinate: f64) -> RouteLane {
    match direction {
        Direction::TopDown => RouteLane::Vertical(coordinate),
        Direction::LeftRight => RouteLane::Horizontal(coordinate),
    }
}

fn bundle_route_lane(
    direction: &Direction,
    kind: BundleKind,
    source_port: &Port,
    target_port: &Port,
) -> RouteLane {
    match (direction, kind) {
        (Direction::TopDown, BundleKind::SharedSink) => RouteLane::Horizontal(target_port.y - 2.0),
        (Direction::TopDown, BundleKind::SharedSource) => {
            RouteLane::Horizontal(source_port.y + 2.0)
        }
        (Direction::LeftRight, BundleKind::SharedSink) => RouteLane::Vertical(target_port.x - 2.0),
        (Direction::LeftRight, BundleKind::SharedSource) => {
            RouteLane::Vertical(source_port.x + 2.0)
        }
    }
}

fn is_semantic_bus_endpoint(node: &Node) -> bool {
    let text = format!("{} {}", node.id, node.label).to_lowercase();
    contains_bus_term(&text)
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

fn lane_route_points(source_port: &Port, target_port: &Port, lane: RouteLane) -> Vec<RoutePoint> {
    let source_stub = outward_point(source_port);
    let target_tail = outward_point(target_port);
    let mut points = vec![
        RoutePoint {
            x: source_port.x,
            y: source_port.y,
        },
        source_stub.clone(),
    ];
    match lane {
        RouteLane::Vertical(x) => points.extend([
            RoutePoint {
                x,
                y: source_stub.y,
            },
            RoutePoint {
                x,
                y: target_tail.y,
            },
        ]),
        RouteLane::Horizontal(y) => points.extend([
            RoutePoint {
                x: source_stub.x,
                y,
            },
            RoutePoint {
                x: target_tail.x,
                y,
            },
        ]),
    }
    points.extend([
        target_tail,
        RoutePoint {
            x: target_port.x,
            y: target_port.y,
        },
    ]);
    points.dedup_by(same_point);
    points
}

impl RouteIdentity {
    fn new(edge: &Edge, spec: &RouteSpec) -> Self {
        Self {
            source: edge.source.clone(),
            target: edge.target.clone(),
            lane_id: spec.lane_id,
            source_cells: [
                raster_point(&spec.preferred[0]),
                raster_point(&outward_point(&spec.source_port)),
            ],
            target_cells: [
                raster_point(&outward_point(&spec.target_port)),
                raster_point(spec.preferred.last().unwrap()),
            ],
        }
    }
}

impl RouteOccupancy {
    fn add(&mut self, identity: RouteIdentity, points: &[RoutePoint]) {
        let route = self.routes.len();
        self.routes.push(identity);
        for segment in points.windows(2).map(raster_segment) {
            let indexed = IndexedSegment { route, segment };
            if segment.horizontal {
                self.horizontal
                    .entry(segment.fixed)
                    .or_default()
                    .push(indexed);
            } else {
                self.vertical
                    .entry(segment.fixed)
                    .or_default()
                    .push(indexed);
            }
        }
    }
}

fn route_conflict_cost(
    points: &[RoutePoint],
    identity: &RouteIdentity,
    occupied_routes: &RouteOccupancy,
) -> usize {
    points
        .windows(2)
        .map(raster_segment)
        .map(|segment| segment_conflict_cost_for(&segment, identity, occupied_routes))
        .sum()
}

fn segment_conflict_cost(
    from: &RoutePoint,
    to: &RoutePoint,
    identity: &RouteIdentity,
    occupied_routes: &RouteOccupancy,
) -> usize {
    segment_conflict_cost_for(
        &raster_segment(&[from.clone(), to.clone()]),
        identity,
        occupied_routes,
    )
}

fn segment_conflict_cost_for(
    candidate: &RasterSegment,
    identity: &RouteIdentity,
    occupied: &RouteOccupancy,
) -> usize {
    let mut cost = 0;
    if candidate.horizontal {
        if let Some(collinear) = occupied.horizontal.get(&candidate.fixed) {
            cost += collinear
                .iter()
                .map(|prior| indexed_conflict(candidate, identity, occupied, prior))
                .sum::<usize>();
        }
        for perpendicular in occupied.vertical.range(candidate.start..=candidate.end) {
            cost += perpendicular
                .1
                .iter()
                .filter(|prior| {
                    prior.segment.start <= candidate.fixed && prior.segment.end >= candidate.fixed
                })
                .map(|prior| indexed_conflict(candidate, identity, occupied, prior))
                .sum::<usize>();
        }
    } else {
        if let Some(collinear) = occupied.vertical.get(&candidate.fixed) {
            cost += collinear
                .iter()
                .map(|prior| indexed_conflict(candidate, identity, occupied, prior))
                .sum::<usize>();
        }
        for perpendicular in occupied.horizontal.range(candidate.start..=candidate.end) {
            cost += perpendicular
                .1
                .iter()
                .filter(|prior| {
                    prior.segment.start <= candidate.fixed && prior.segment.end >= candidate.fixed
                })
                .map(|prior| indexed_conflict(candidate, identity, occupied, prior))
                .sum::<usize>();
        }
    }
    cost
}

fn indexed_conflict(
    candidate: &RasterSegment,
    identity: &RouteIdentity,
    occupied: &RouteOccupancy,
    prior: &IndexedSegment,
) -> usize {
    let prior_identity = &occupied.routes[prior.route];
    if intentional_lane_sharing(identity, prior_identity) {
        return 0;
    }
    let cost = segment_intersection_cells(candidate, &prior.segment);
    let exemptions = identity
        .source_cells
        .into_iter()
        .chain(identity.target_cells)
        .filter(|cell| {
            cell_is_shared_endpoint_exempt(identity, prior_identity, *cell)
                && candidate.contains(*cell)
                && prior.segment.contains(*cell)
        })
        .collect::<HashSet<_>>()
        .len();
    cost.saturating_sub(exemptions)
}

fn intentional_lane_sharing(a: &RouteIdentity, b: &RouteIdentity) -> bool {
    a.lane_id.is_some() && a.lane_id == b.lane_id && (a.source == b.source || a.target == b.target)
}

fn cell_is_shared_endpoint_exempt(a: &RouteIdentity, b: &RouteIdentity, cell: (i64, i64)) -> bool {
    [(&a.source, &a.source_cells), (&a.target, &a.target_cells)]
        .into_iter()
        .any(|(a_id, a_cells)| {
            [(&b.source, &b.source_cells), (&b.target, &b.target_cells)]
                .into_iter()
                .any(|(b_id, b_cells)| {
                    a_id == b_id && a_cells.contains(&cell) && b_cells.contains(&cell)
                })
        })
}

fn raster_segment(points: &[RoutePoint]) -> RasterSegment {
    let a = raster_point(&points[0]);
    let b = raster_point(&points[1]);
    if a.1 == b.1 {
        RasterSegment {
            horizontal: true,
            fixed: a.1,
            start: a.0.min(b.0),
            end: a.0.max(b.0),
        }
    } else {
        RasterSegment {
            horizontal: false,
            fixed: a.0,
            start: a.1.min(b.1),
            end: a.1.max(b.1),
        }
    }
}

fn raster_point(point: &RoutePoint) -> (i64, i64) {
    (point.x.round() as i64, point.y.round() as i64)
}

impl RasterSegment {
    fn contains(&self, (x, y): (i64, i64)) -> bool {
        if self.horizontal {
            y == self.fixed && x >= self.start && x <= self.end
        } else {
            x == self.fixed && y >= self.start && y <= self.end
        }
    }
}

fn segment_intersection_cells(a: &RasterSegment, b: &RasterSegment) -> usize {
    if a.horizontal == b.horizontal {
        if a.fixed != b.fixed {
            return 0;
        }
        let overlap = a.end.min(b.end) - a.start.max(b.start);
        return if overlap >= 0 {
            overlap as usize + 1
        } else {
            0
        };
    }
    let (horizontal, vertical) = if a.horizontal { (a, b) } else { (b, a) };
    usize::from(
        vertical.fixed >= horizontal.start
            && vertical.fixed <= horizontal.end
            && horizontal.fixed >= vertical.start
            && horizontal.fixed <= vertical.end,
    )
}

#[allow(clippy::too_many_arguments)]
fn route_points(
    preferred: Vec<RoutePoint>,
    source_port: &Port,
    target_port: &Port,
    nodes: &[Node],
    grid: &RoutingGrid,
    required_lane: Option<RouteLane>,
    identity: &RouteIdentity,
    occupied_routes: &RouteOccupancy,
) -> Option<Vec<RoutePoint>> {
    if route_is_valid(&preferred, source_port, target_port, nodes)
        && route_conflict_cost(&preferred, identity, occupied_routes) == 0
    {
        return Some(preferred);
    }
    obstacle_avoiding_points(
        source_port,
        target_port,
        nodes,
        grid,
        required_lane,
        identity,
        occupied_routes,
    )
}

fn obstacle_avoiding_points(
    source_port: &Port,
    target_port: &Port,
    nodes: &[Node],
    grid: &RoutingGrid,
    required_lane: Option<RouteLane>,
    identity: &RouteIdentity,
    occupied_routes: &RouteOccupancy,
) -> Option<Vec<RoutePoint>> {
    let source_stub = outward_point(source_port);
    let target_tail = outward_point(target_port);
    let sx = coordinate_index(&grid.xs, source_stub.x);
    let sy = coordinate_index(&grid.ys, source_stub.y);
    let tx = coordinate_index(&grid.xs, target_tail.x);
    let ty = coordinate_index(&grid.ys, target_tail.y);
    let start = sx.zip(sy).map(|(x, y)| y * grid.xs.len() + x)?;
    let goal = tx.zip(ty).map(|(x, y)| y * grid.xs.len() + x)?;
    if !grid.clear[start] || !grid.clear[goal] {
        return None;
    }

    let core = compressed_grid_path(grid, start, goal, required_lane, identity, occupied_routes)?;
    let mut points = vec![RoutePoint {
        x: source_port.x,
        y: source_port.y,
    }];
    points.extend(core);
    points.push(RoutePoint {
        x: target_port.x,
        y: target_port.y,
    });
    points.dedup_by(same_point);

    route_is_valid(&points, source_port, target_port, nodes).then_some(points)
}

impl RoutingGrid {
    fn new(specs: &[Option<RouteSpec>], nodes: &[Node], perimeter: &GraphPerimeter) -> Self {
        let mut xs = vec![perimeter.max_x + NODE_GAP_WITHIN_LAYER];
        let mut ys = vec![perimeter.max_y + LAYER_GAP];
        for spec in specs.iter().flatten() {
            let source_stub = outward_point(&spec.source_port);
            let target_tail = outward_point(&spec.target_port);
            xs.extend([
                spec.source_port.x,
                spec.target_port.x,
                source_stub.x,
                target_tail.x,
            ]);
            ys.extend([
                spec.source_port.y,
                spec.target_port.y,
                source_stub.y,
                target_tail.y,
            ]);
            match spec.required_lane {
                Some(RouteLane::Vertical(x)) => xs.push(x),
                Some(RouteLane::Horizontal(y)) => ys.push(y),
                None => {}
            }
        }
        for node in nodes.iter().filter(|node| !is_dummy_node(node)) {
            if let Some((x, y, width, height)) = node_cell_rect(node) {
                if x > 0 {
                    xs.push((x - 1) as f64);
                }
                if y > 0 {
                    ys.push((y - 1) as f64);
                }
                xs.push((x + width) as f64);
                ys.push((y + height) as f64);
            }
        }
        sort_coordinates(&mut xs);
        sort_coordinates(&mut ys);

        let width = xs.len();
        let clear: Vec<bool> = ys
            .iter()
            .flat_map(|&y| {
                xs.iter()
                    .map(move |&x| !point_hits_real_node(&RoutePoint { x, y }, nodes))
            })
            .collect();
        let mut right_clear = vec![false; clear.len()];
        let mut down_clear = vec![false; clear.len()];
        for cell in 0..clear.len() {
            if !clear[cell] {
                continue;
            }
            let point = grid_point(cell, &xs, &ys);
            if cell % width + 1 < width && clear[cell + 1] {
                right_clear[cell] =
                    !segment_hits_any_real_node(&point, &grid_point(cell + 1, &xs, &ys), nodes);
            }
            if cell + width < clear.len() && clear[cell + width] {
                down_clear[cell] =
                    !segment_hits_any_real_node(&point, &grid_point(cell + width, &xs, &ys), nodes);
            }
        }
        Self {
            xs,
            ys,
            clear,
            right_clear,
            down_clear,
        }
    }

    fn edge_is_clear(&self, from: usize, to: usize) -> bool {
        if to == from + 1 {
            self.right_clear[from]
        } else if from == to + 1 {
            self.right_clear[to]
        } else if to > from {
            self.down_clear[from]
        } else {
            self.down_clear[to]
        }
    }
}

fn sort_coordinates(coordinates: &mut Vec<f64>) {
    coordinates.retain(|value| value.is_finite() && *value >= 0.0);
    coordinates.sort_by(|a, b| a.total_cmp(b));
    coordinates.dedup_by(|a, b| (*a - *b).abs() < 0.01);
}

fn compressed_grid_path(
    grid: &RoutingGrid,
    start: usize,
    goal: usize,
    required_lane: Option<RouteLane>,
    identity: &RouteIdentity,
    occupied_routes: &RouteOccupancy,
) -> Option<Vec<RoutePoint>> {
    const DIRECTIONS: usize = 5;
    const PERIMETER_STATES: usize = 2;
    const STATE_COUNT: usize = DIRECTIONS * PERIMETER_STATES;

    let width = grid.xs.len();
    let height = grid.ys.len();
    let start_uses_perimeter = false;
    let start_state = state_index(0, start_uses_perimeter);
    let mut distances = vec![[None; STATE_COUNT]; grid.clear.len()];
    let mut previous = vec![[None; STATE_COUNT]; grid.clear.len()];
    let mut right_conflicts = vec![None; grid.clear.len()];
    let mut down_conflicts = vec![None; grid.clear.len()];
    let mut queue = BinaryHeap::new();
    distances[start][start_state] = Some((0_usize, 0_usize, 0_i64));
    queue.push(Reverse((
        0_usize,
        0_usize,
        0_i64,
        start,
        0_u8,
        start_uses_perimeter,
    )));

    while let Some(Reverse((conflicts, bends, length, cell, incoming, used_perimeter))) =
        queue.pop()
    {
        let state = state_index(incoming, used_perimeter);
        if distances[cell][state] != Some((conflicts, bends, length)) {
            continue;
        }
        if cell == goal && (required_lane.is_none() || used_perimeter) {
            return Some(compress_route_points(reconstruct_grid_path(
                cell,
                incoming,
                used_perimeter,
                &previous,
                &grid.xs,
                &grid.ys,
            )));
        }

        for next in adjacent_grid_cells(cell, width, height)
            .into_iter()
            .flatten()
        {
            if !grid.clear[next] || !grid.edge_is_clear(cell, next) {
                continue;
            }
            let from = grid_point(cell, &grid.xs, &grid.ys);
            let to = grid_point(next, &grid.xs, &grid.ys);
            let next_direction = step_direction(&from, &to);
            if directions_are_opposite(incoming, next_direction) {
                continue;
            }
            let conflict_cache = if next.abs_diff(cell) == 1 {
                &mut right_conflicts[cell.min(next)]
            } else {
                &mut down_conflicts[cell.min(next)]
            };
            let segment_conflicts = *conflict_cache.get_or_insert_with(|| {
                segment_conflict_cost(&from, &to, identity, occupied_routes)
            });
            let next_conflicts = conflicts + segment_conflicts;
            let next_bends = bends
                + usize::from(
                    incoming != 0 && direction_axis(incoming) != direction_axis(next_direction),
                );
            let next_length = length + route_distance(&from, &to);
            let next_used_perimeter =
                used_perimeter || segment_uses_required_lane(&from, &to, required_lane);
            let next_state = state_index(next_direction, next_used_perimeter);
            let next_cost = (next_conflicts, next_bends, next_length);
            if distances[next][next_state].is_some_and(|cost| cost <= next_cost) {
                continue;
            }
            distances[next][next_state] = Some(next_cost);
            previous[next][next_state] = Some((cell, incoming, used_perimeter));
            queue.push(Reverse((
                next_conflicts,
                next_bends,
                next_length,
                next,
                next_direction,
                next_used_perimeter,
            )));
        }
    }

    None
}

fn state_index(direction: u8, used_perimeter: bool) -> usize {
    direction as usize * 2 + usize::from(used_perimeter)
}

fn step_direction(from: &RoutePoint, to: &RoutePoint) -> u8 {
    if to.x > from.x {
        1
    } else if to.y > from.y {
        2
    } else if to.x < from.x {
        3
    } else {
        4
    }
}

fn direction_axis(direction: u8) -> u8 {
    match direction {
        1 | 3 => 1,
        2 | 4 => 2,
        _ => 0,
    }
}

fn directions_are_opposite(a: u8, b: u8) -> bool {
    matches!((a, b), (1, 3) | (3, 1) | (2, 4) | (4, 2))
}

fn adjacent_grid_cells(cell: usize, width: usize, height: usize) -> [Option<usize>; 4] {
    let x = cell % width;
    let y = cell / width;
    [
        (x + 1 < width).then_some(cell + 1),
        (y + 1 < height).then_some(cell + width),
        (x > 0).then(|| cell - 1),
        (y > 0).then(|| cell - width),
    ]
}

fn grid_point(cell: usize, xs: &[f64], ys: &[f64]) -> RoutePoint {
    RoutePoint {
        x: xs[cell % xs.len()],
        y: ys[cell / xs.len()],
    }
}

fn segment_uses_required_lane(
    from: &RoutePoint,
    to: &RoutePoint,
    required_lane: Option<RouteLane>,
) -> bool {
    required_lane.is_some_and(|lane| match lane {
        RouteLane::Vertical(x) => same(from.x, x) && same(to.x, x),
        RouteLane::Horizontal(y) => same(from.y, y) && same(to.y, y),
    })
}

fn route_distance(a: &RoutePoint, b: &RoutePoint) -> i64 {
    (((a.x - b.x).abs() + (a.y - b.y).abs()) * 1_000.0).round() as i64
}

type PreviousStates = [Option<(usize, u8, bool)>; 10];

fn reconstruct_grid_path(
    mut cell: usize,
    mut direction: u8,
    mut used_perimeter: bool,
    previous: &[PreviousStates],
    xs: &[f64],
    ys: &[f64],
) -> Vec<RoutePoint> {
    let mut points = vec![grid_point(cell, xs, ys)];
    while let Some((prior_cell, prior_direction, prior_used_perimeter)) =
        previous[cell][state_index(direction, used_perimeter)]
    {
        cell = prior_cell;
        direction = prior_direction;
        used_perimeter = prior_used_perimeter;
        points.push(grid_point(cell, xs, ys));
    }
    points.reverse();
    points
}

fn compress_route_points(points: Vec<RoutePoint>) -> Vec<RoutePoint> {
    if points.len() < 3 {
        return points;
    }
    let mut compressed = vec![points[0].clone()];
    for window in points.windows(3) {
        let same_vertical = same(window[0].x, window[1].x) && same(window[1].x, window[2].x);
        let same_horizontal = same(window[0].y, window[1].y) && same(window[1].y, window[2].y);
        if !same_vertical && !same_horizontal {
            compressed.push(window[1].clone());
        }
    }
    compressed.push(points.last().unwrap().clone());
    compressed
}

fn outward_point(port: &Port) -> RoutePoint {
    let (dx, dy) = match port.side {
        PortSide::Top => (0.0, -1.0),
        PortSide::Right => (1.0, 0.0),
        PortSide::Bottom => (0.0, 1.0),
        PortSide::Left => (-1.0, 0.0),
    };
    RoutePoint {
        x: port.x + dx,
        y: port.y + dy,
    }
}

fn coordinate_index(coordinates: &[f64], value: f64) -> Option<usize> {
    coordinates
        .iter()
        .position(|candidate| same(*candidate, value))
}

fn route_is_valid(points: &[RoutePoint], source: &Port, target: &Port, nodes: &[Node]) -> bool {
    let source_stub = outward_point(source);
    let target_tail = outward_point(target);
    points.first().is_some_and(|point| same_port(point, source))
        && points
            .get(1)
            .is_some_and(|point| same_point_ref(point, &source_stub))
        && points.last().is_some_and(|point| same_port(point, target))
        && points
            .get(points.len().saturating_sub(2))
            .is_some_and(|point| same_point_ref(point, &target_tail))
        && points.windows(2).all(|segment| {
            (same(segment[0].x, segment[1].x) || same(segment[0].y, segment[1].y))
                && !segment_hits_any_real_node(&segment[0], &segment[1], nodes)
        })
}

fn same_port(point: &RoutePoint, port: &Port) -> bool {
    same(point.x, port.x) && same(point.y, port.y)
}

fn same_point(a: &mut RoutePoint, b: &mut RoutePoint) -> bool {
    same_point_ref(a, b)
}

fn same_point_ref(a: &RoutePoint, b: &RoutePoint) -> bool {
    same(a.x, b.x) && same(a.y, b.y)
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

fn point_hits_real_node(point: &RoutePoint, nodes: &[Node]) -> bool {
    nodes
        .iter()
        .filter(|node| !is_dummy_node(node))
        .any(|node| segment_hits_node_cells(point, point, node))
}

fn segment_hits_any_real_node(a: &RoutePoint, b: &RoutePoint, nodes: &[Node]) -> bool {
    nodes
        .iter()
        .filter(|node| !is_dummy_node(node))
        .any(|node| segment_hits_node_cells(a, b, node))
}

fn segment_hits_node_cells(a: &RoutePoint, b: &RoutePoint, node: &Node) -> bool {
    let Some((x, y, width, height)) = node_cell_rect(node) else {
        return false;
    };
    if same(a.x, b.x) {
        let px = a.x.round() as i64;
        let y1 = a.y.min(b.y).round() as i64;
        let y2 = a.y.max(b.y).round() as i64;
        px >= x && px < x + width && y1 < y + height && y2 >= y
    } else if same(a.y, b.y) {
        let py = a.y.round() as i64;
        let x1 = a.x.min(b.x).round() as i64;
        let x2 = a.x.max(b.x).round() as i64;
        py >= y && py < y + height && x1 < x + width && x2 >= x
    } else {
        true
    }
}

fn node_cell_rect(node: &Node) -> Option<(i64, i64, i64, i64)> {
    let (Some(x), Some(y), Some(width), Some(height)) = (node.x, node.y, node.width, node.height)
    else {
        return None;
    };
    if [x, y, width, height]
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
    {
        Some((x as i64, y as i64, width as i64, height as i64))
    } else {
        None
    }
}

fn assign_route_label_anchors(graph: &mut Graph) {
    let route_segments: Vec<Vec<RasterSegment>> = graph
        .edges
        .iter()
        .map(|edge| {
            edge.route
                .as_ref()
                .map(|route| route.points.windows(2).map(raster_segment).collect())
                .unwrap_or_default()
        })
        .collect();
    let mut occupied_labels = HashSet::new();
    // Laid-out horizontal extent from nodes and routed points: labels on
    // vertical segments prefer the inward side so they never push bounds
    // outward when the opposite side is clear.
    let (min_x, max_x) = graph
        .nodes
        .iter()
        .filter(|node| !is_dummy_node(node))
        .filter_map(|node| node.x.zip(node.width).map(|(x, w)| (x, x + w)))
        .chain(
            graph
                .edges
                .iter()
                .filter_map(|edge| edge.route.as_ref())
                .flat_map(|route| route.points.iter())
                .filter_map(|point| point.x.is_finite().then_some((point.x, point.x))),
        )
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), (a, b)| {
            (lo.min(a), hi.max(b))
        });

    for edge_index in 0..graph.edges.len() {
        let Some(label) = graph.edges[edge_index].label.clone() else {
            continue;
        };
        let Some(route) = graph.edges[edge_index].route.as_ref() else {
            continue;
        };
        let mut segments: Vec<_> = route.points.windows(2).collect();
        segments.sort_by(|a, b| route_distance(&b[0], &b[1]).cmp(&route_distance(&a[0], &a[1])));
        let width = label.chars().count() as i64;
        let mut chosen = None;
        'segments: for segment in segments {
            let a = raster_point(&segment[0]);
            let b = raster_point(&segment[1]);
            let horizontal = a.1 == b.1;
            let center_y = if horizontal { a.1 } else { (a.1 + b.1) / 2 };
            let center_xs: Vec<i64> = if horizontal {
                vec![(a.0 + b.0) / 2]
            } else {
                let right = a.0 + 1 + width / 2;
                let left = a.0 - 1 - width / 2;
                // Prefer the inward side of the segment: left when the
                // segment sits in the right half of the laid-out extent.
                if (a.0 as f64) >= (min_x + max_x) / 2.0 {
                    vec![left, right]
                } else {
                    vec![right, left]
                }
            };
            for &center_x in &center_xs {
                for offset in [0_i64, -1, 1, -2, 2, -3, 3, -4, 4] {
                    let row = center_y + offset;
                    let left = center_x - width / 2;
                    if label_footprint_is_clear(
                        left,
                        row,
                        width,
                        edge_index,
                        &graph.nodes,
                        &route_segments,
                        &occupied_labels,
                    ) {
                        for x in left..left + width {
                            occupied_labels.insert((x, row));
                        }
                        chosen = Some(RoutePoint {
                            x: center_x as f64,
                            y: (row + 1) as f64,
                        });
                        break 'segments;
                    }
                }
            }
        }
        let fallback = route_label_anchor(Some(&label), &route.points);
        graph.edges[edge_index].route.as_mut().unwrap().label_anchor = chosen.or(fallback);
    }
}

fn label_footprint_is_clear(
    left: i64,
    row: i64,
    width: i64,
    own_edge: usize,
    nodes: &[Node],
    route_segments: &[Vec<RasterSegment>],
    occupied_labels: &HashSet<(i64, i64)>,
) -> bool {
    row >= 0
        && left >= 0
        && (left..left + width).all(|x| {
            let cell = (x, row);
            !occupied_labels.contains(&cell)
                && !nodes
                    .iter()
                    .filter(|node| !is_dummy_node(node))
                    .any(|node| {
                        node_cell_rect(node).is_some_and(|(nx, ny, nw, nh)| {
                            x >= nx && x < nx + nw && row >= ny && row < ny + nh
                        })
                    })
                && !route_segments.iter().enumerate().any(|(index, segments)| {
                    index != own_edge && segments.iter().any(|segment| segment.contains(cell))
                })
        })
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

#[cfg(test)]
mod routing_tests {
    use super::*;

    #[test]
    fn endpoint_interiors_are_obstacles() {
        let nodes = vec![
            test_node("source", 1.0, 1.0, 3.0, 3.0),
            test_node("target", 8.0, 1.0, 3.0, 3.0),
        ];

        assert!(point_hits_real_node(&RoutePoint { x: 2.0, y: 2.0 }, &nodes));
        assert!(point_hits_real_node(&RoutePoint { x: 9.0, y: 2.0 }, &nodes));
    }

    #[test]
    fn unroutable_geometry_is_exposed_instead_of_restoring_a_collision() {
        let nodes = vec![
            test_node("source", 4.0, 4.0, 2.0, 2.0),
            test_node("target", 14.0, 4.0, 2.0, 2.0),
            test_node("top", 0.0, 0.0, 10.0, 1.0),
            test_node("bottom", 0.0, 10.0, 10.0, 1.0),
            test_node("left", 0.0, 1.0, 1.0, 9.0),
            test_node("right", 9.0, 1.0, 1.0, 9.0),
        ];
        let source_port = Port {
            x: 6.0,
            y: 5.0,
            side: PortSide::Right,
        };
        let target_port = Port {
            x: 13.0,
            y: 5.0,
            side: PortSide::Left,
        };

        let perimeter = graph_perimeter(&nodes);
        let grid = test_grid(&source_port, &target_port, &nodes, &perimeter, None);
        assert!(
            obstacle_avoiding_points(
                &source_port,
                &target_port,
                &nodes,
                &grid,
                None,
                &test_identity(&source_port, &target_port),
                &RouteOccupancy::default(),
            )
            .is_none()
        );
    }

    #[test]
    fn required_perimeter_lane_is_present_in_route() {
        let source_port = Port {
            x: 11.0,
            y: 30.0,
            side: PortSide::Right,
        };
        let target_port = Port {
            x: 12.0,
            y: 14.5,
            side: PortSide::Right,
        };
        let perimeter = GraphPerimeter {
            max_x: 12.0,
            max_y: 32.0,
        };
        let required_lane = Some(RouteLane::Vertical(20.0));
        let grid = test_grid(&source_port, &target_port, &[], &perimeter, required_lane);
        let points = obstacle_avoiding_points(
            &source_port,
            &target_port,
            &[],
            &grid,
            required_lane,
            &test_identity(&source_port, &target_port),
            &RouteOccupancy::default(),
        )
        .unwrap();

        assert!(
            points.iter().any(|point| same(point.x, 20.0)),
            "{points:#?}"
        );
    }

    #[test]
    fn clean_preferred_route_skips_compressed_search() {
        let source_port = Port {
            x: 0.0,
            y: 1.0,
            side: PortSide::Right,
        };
        let target_port = Port {
            x: 10.0,
            y: 1.0,
            side: PortSide::Left,
        };
        let preferred = lane_route_points(&source_port, &target_port, RouteLane::Vertical(5.0));
        let perimeter = GraphPerimeter {
            max_x: 10.0,
            max_y: 2.0,
        };
        let grid = test_grid(&source_port, &target_port, &[], &perimeter, None);

        assert_eq!(
            route_points(
                preferred.clone(),
                &source_port,
                &target_port,
                &[],
                &grid,
                None,
                &test_identity(&source_port, &target_port),
                &RouteOccupancy::default(),
            )
            .as_deref(),
            Some(preferred.as_slice())
        );
    }

    #[test]
    fn large_coordinates_do_not_expand_the_search_grid() {
        let source_port = Port {
            x: 1_000_000_000.0,
            y: 1_000_000_000.0,
            side: PortSide::Right,
        };
        let target_port = Port {
            x: 2_000_000_000.0,
            y: 2_000_000_000.0,
            side: PortSide::Left,
        };
        let perimeter = GraphPerimeter {
            max_x: 2_000_000_001.0,
            max_y: 2_000_000_001.0,
        };
        let grid = test_grid(&source_port, &target_port, &[], &perimeter, None);
        assert!(grid.xs.len() <= 5 && grid.ys.len() <= 5);
        assert!(grid.clear.len() <= 25);
        assert!(
            obstacle_avoiding_points(
                &source_port,
                &target_port,
                &[],
                &grid,
                None,
                &test_identity(&source_port, &target_port),
                &RouteOccupancy::default(),
            )
            .is_some()
        );
    }

    fn test_identity(source_port: &Port, target_port: &Port) -> RouteIdentity {
        RouteIdentity {
            source: "source".into(),
            target: "target".into(),
            lane_id: None,
            source_cells: [
                raster_point(&RoutePoint {
                    x: source_port.x,
                    y: source_port.y,
                }),
                raster_point(&outward_point(source_port)),
            ],
            target_cells: [
                raster_point(&outward_point(target_port)),
                raster_point(&RoutePoint {
                    x: target_port.x,
                    y: target_port.y,
                }),
            ],
        }
    }

    fn test_grid(
        source_port: &Port,
        target_port: &Port,
        nodes: &[Node],
        perimeter: &GraphPerimeter,
        required_lane: Option<RouteLane>,
    ) -> RoutingGrid {
        RoutingGrid::new(
            &[Some(RouteSpec {
                preferred: vec![],
                source_port: source_port.clone(),
                target_port: target_port.clone(),
                lane_id: None,
                required_lane,
            })],
            nodes,
            perimeter,
        )
    }

    fn test_node(id: &str, x: f64, y: f64, width: f64, height: f64) -> Node {
        Node {
            id: id.into(),
            label: id.into(),
            shape: NodeShape::Rectangle,
            x: Some(x),
            y: Some(y),
            width: Some(width),
            height: Some(height),
        }
    }
}
