use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_diaview"))
}

fn bin_without_herdr_env() -> Command {
    let mut command = bin();
    command.env_remove("HERDR_PANE_ID");
    command
}

fn simple_diagram() -> &'static str {
    "flowchart TD\n    A-->B\n"
}

#[test]
fn file_inline_input() {
    let dir = tempfile_dir();
    let path = dir.join("diagram.mmd");
    std::fs::write(&path, simple_diagram()).unwrap();

    let output = bin()
        .args(["--inline", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("A"));
}

#[test]
fn explicit_stdin_input() {
    let mut child = bin()
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(simple_diagram().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("A"));
}

#[test]
fn literal_dash_counts_as_input_not_unknown_option() {
    let mut child = bin()
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(simple_diagram().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
}

#[test]
fn implicit_piped_stdin_defaults_inline() {
    let mut child = bin()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(simple_diagram().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("A"));
}

#[test]
fn no_input_errors_without_piped_stdin() {
    let output = bin().output().unwrap();
    assert!(!output.status.success());
}

#[test]
fn duplicate_input_rejected() {
    let dir = tempfile_dir();
    let path = dir.join("diagram.mmd");
    std::fs::write(&path, simple_diagram()).unwrap();

    let output = bin()
        .args([path.to_str().unwrap(), "extra"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Only one input"));
}

#[test]
fn herdr_requires_file_argument() {
    let output = bin_without_herdr_env().arg("--herdr").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires a Mermaid file"));
}

#[test]
fn herdr_rejects_stdin() {
    let output = bin_without_herdr_env()
        .args(["--herdr", "-"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot use stdin"));
}

#[test]
fn herdr_requires_pane_env() {
    let dir = tempfile_dir();
    let path = dir.join("diagram.mmd");
    std::fs::write(&path, simple_diagram()).unwrap();

    let output = bin_without_herdr_env()
        .args(["--herdr", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("HERDR_PANE_ID missing"));
}

#[test]
fn help_prints_usage() {
    let output = bin().arg("--help").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("USAGE"));
}

#[test]
fn version_prints_version() {
    let output = bin().arg("--version").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn unknown_option_rejected() {
    let output = bin().arg("--nope").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unknown option"));
}

fn tempfile_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("diaview-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
