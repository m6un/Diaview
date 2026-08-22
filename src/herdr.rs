use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::app::ActionSubmission;

pub fn launch_sidecar(exe: &Path, diagram: &Path, origin_pane: &str) -> Result<(), String> {
    let cwd = diagram.parent().unwrap_or_else(|| Path::new("."));
    let split = Command::new("herdr")
        .args([
            "pane",
            "split",
            "--pane",
            origin_pane,
            "--direction",
            "right",
            "--cwd",
        ])
        .arg(cwd)
        .arg("--focus")
        .output()
        .map_err(|e| format!("Failed to run herdr pane split: {e}"))?;
    if !split.status.success() {
        return Err(format!(
            "herdr pane split failed: {}",
            String::from_utf8_lossy(&split.stderr)
        ));
    }

    let pane = parse_split_pane_id(&split.stdout)?;
    let command = build_sidecar_command(exe, origin_pane, diagram);
    let run = Command::new("herdr")
        .args(["pane", "run", &pane, &command])
        .output()
        .map_err(|e| format!("Failed to run herdr pane run: {e}"))?;
    if !run.status.success() {
        return Err(format!(
            "herdr pane run failed: {}",
            String::from_utf8_lossy(&run.stderr)
        ));
    }
    Ok(())
}

pub fn prompt_agent(
    origin_pane: &str,
    diagram: &Path,
    submission: &ActionSubmission,
) -> Result<(), String> {
    let instruction = build_agent_instruction(diagram, submission);
    let output = Command::new("herdr")
        .args(["agent", "prompt", origin_pane, &instruction])
        .output()
        .map_err(|e| format!("Failed to run herdr agent prompt: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "herdr agent prompt failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

pub fn build_sidecar_command(exe: &Path, origin_pane: &str, diagram: &Path) -> String {
    [
        shell_quote(&exe.to_string_lossy()),
        shell_quote("--herdr-sidecar"),
        shell_quote(origin_pane),
        shell_quote(&diagram.to_string_lossy()),
    ]
    .join(" ")
}

pub fn build_agent_instruction(diagram: &Path, submission: &ActionSubmission) -> String {
    format!(
        "Diaview action on selected Mermaid node.\n\nNode id: {}\nNode label: {}\nMermaid file: {}\nUser instruction: {}\n\nEdit and save that Mermaid file directly. Stay within Diaview's supported Mermaid flowchart subset.",
        submission.node_id,
        submission.node_label,
        diagram.display(),
        submission.prompt
    )
}

pub fn parse_split_pane_id(stdout: &[u8]) -> Result<String, String> {
    let value: Value = serde_json::from_slice(stdout)
        .map_err(|e| format!("Could not parse herdr pane split JSON: {e}"))?;
    value
        .pointer("/result/pane/pane_id")
        .or_else(|| value.pointer("/result/pane_id"))
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| "herdr pane split JSON did not include a pane id".to_string())
}

pub fn absolute_file(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if !path.is_file() {
        return Err(format!(
            "--herdr requires a real Mermaid file: {}",
            path.display()
        ));
    }
    path.canonicalize()
        .map_err(|e| format!("Failed to resolve '{}': {e}", path.display()))
}

pub fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".into();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_split_pane_id_defensively() {
        let json = br#"{"result":{"pane":{"pane_id":"w1:p2"}}}"#;
        assert_eq!(parse_split_pane_id(json).unwrap(), "w1:p2");
        assert!(parse_split_pane_id(br#"{"result":{}}"#).is_err());
        assert!(parse_split_pane_id(b"not json").is_err());
    }

    #[test]
    fn shell_quote_handles_spaces_and_quotes() {
        assert_eq!(shell_quote("/tmp/a b"), "'/tmp/a b'");
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn sidecar_command_quotes_every_argument() {
        let command = build_sidecar_command(
            Path::new("/tmp/diaview bin"),
            "w1:p2",
            Path::new("/tmp/diagram's file.mmd"),
        );
        assert_eq!(
            command,
            "'/tmp/diaview bin' '--herdr-sidecar' 'w1:p2' '/tmp/diagram'\\''s file.mmd'"
        );
    }

    #[test]
    fn agent_instruction_contains_required_context() {
        let submission = ActionSubmission {
            node_id: "B".into(),
            node_label: "JWT Validator".into(),
            prompt: "Add cache".into(),
        };
        let instruction = build_agent_instruction(Path::new("/tmp/a.mmd"), &submission);
        assert!(instruction.contains("Node id: B"));
        assert!(instruction.contains("Node label: JWT Validator"));
        assert!(instruction.contains("Mermaid file: /tmp/a.mmd"));
        assert!(instruction.contains("User instruction: Add cache"));
        assert!(instruction.contains("supported Mermaid flowchart subset"));
    }
}
