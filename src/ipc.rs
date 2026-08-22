use std::io::Write;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActionDocument {
    pub protocol: Protocol,
    pub version: u32,
    pub selected_node: SelectedNode,
    pub prompt: String,
    pub mermaid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Protocol {
    #[serde(rename = "diaview.action")]
    DiaviewAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelectedNode {
    pub id: String,
    pub label: String,
}

impl SelectedNode {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

impl ActionDocument {
    pub fn new(
        selected_node: SelectedNode,
        prompt: impl Into<String>,
        mermaid: impl Into<String>,
    ) -> Self {
        Self {
            protocol: Protocol::DiaviewAction,
            version: 1,
            selected_node,
            prompt: prompt.into(),
            mermaid: mermaid.into(),
        }
    }
}

pub fn write_action_document<W: Write>(
    writer: W,
    document: &ActionDocument,
) -> serde_json::Result<()> {
    serde_json::to_writer(writer, document)
}
