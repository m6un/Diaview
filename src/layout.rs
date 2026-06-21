use crate::model::Graph;

mod simple;

pub use simple::SimpleLayoutEngine;

pub trait LayoutEngine {
    fn layout(&self, graph: &mut Graph);
}

pub fn layout(graph: &mut Graph) {
    layout_with(&SimpleLayoutEngine, graph);
}

pub fn layout_with<E: LayoutEngine + ?Sized>(engine: &E, graph: &mut Graph) {
    engine.layout(graph);
}
