use crate::model::Graph;

mod simple;

pub use simple::SimpleLayoutEngine;

/// A graph layout implementation.
///
/// Layout engines prepare a parsed [`Graph`] for rendering by assigning node
/// positions and sizes, and may insert helper routing nodes when needed.
pub trait LayoutEngine {
    /// Lay out every node in `graph`, filling in `x`, `y`, `width`, `height`.
    fn layout(&self, graph: &mut Graph);
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
