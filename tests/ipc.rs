use diaview::ipc::{ActionDocument, SelectedNode, write_action_document};
use serde_json::Value;

#[test]
fn serializes_exact_schema_and_preserves_mermaid() {
    let doc = ActionDocument::new(
        SelectedNode::new("B", "JWT Validator"),
        "Add a Redis cache layer before this for token revocation",
        "graph TD\n    A[Request] --> B{JWT Validator}\n    B --> C[Allow]",
    );

    let json: Value = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(json["protocol"], "diaview.action");
    assert_eq!(json["version"], 1);
    assert_eq!(json["selected_node"]["id"], "B");
    assert_eq!(json["selected_node"]["label"], "JWT Validator");
    assert_eq!(
        json["prompt"],
        "Add a Redis cache layer before this for token revocation"
    );
    assert_eq!(
        json["mermaid"],
        "graph TD\n    A[Request] --> B{JWT Validator}\n    B --> C[Allow]"
    );
}

#[test]
fn escapes_unicode_and_special_characters() {
    let doc = ActionDocument::new(
        SelectedNode::new("n\"1", "π <δ> \u{1F680}"),
        "Use \"quotes\", slashes \\, and emoji 🚀",
        "graph TD\n    A[«α»] --> B[β & γ]\n    B --> C[δ/ε]",
    );

    let json: Value = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(json["selected_node"]["id"], "n\"1");
    assert_eq!(json["selected_node"]["label"], "π <δ> 🚀");
    assert_eq!(json["prompt"], "Use \"quotes\", slashes \\, and emoji 🚀");
    assert_eq!(
        json["mermaid"],
        "graph TD\n    A[«α»] --> B[β & γ]\n    B --> C[δ/ε]"
    );
}

#[test]
fn writer_emits_one_json_document_only() {
    let doc = ActionDocument::new(
        SelectedNode::new("A", "Request"),
        "Prompt",
        "graph TD\nA-->B",
    );
    let mut out = Vec::new();
    write_action_document(&mut out, &doc).unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        serde_json::to_string(&doc).unwrap()
    );
}
