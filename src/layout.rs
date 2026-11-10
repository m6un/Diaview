use crate::model::Graph;

mod simple;

pub use simple::{LayoutProfile, LayoutSpacing, SimpleLayoutEngine};

pub trait LayoutEngine {
    fn layout(&self, graph: &mut Graph);
}

pub fn layout(graph: &mut Graph) {
    layout_with(&SimpleLayoutEngine::default(), graph);
}

pub fn layout_compact(graph: &mut Graph) {
    layout_with(&SimpleLayoutEngine::compact(), graph);
}

pub fn layout_with<E: LayoutEngine + ?Sized>(engine: &E, graph: &mut Graph) {
    engine.layout(graph);
}
