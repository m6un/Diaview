use diaview::herdr;
use diaview::layout;
use diaview::parser::mermaid;
use diaview::renderer::canvas;
use std::io::IsTerminal;
use std::path::Path;

fn main() {
    run().unwrap_or_else(|e| exit_err(&e));
}

fn run() -> Result<(), String> {
    let mut inline = false;
    let mut herdr_file: Option<String> = None;
    let mut sidecar: Option<(String, String)> = None;
    let mut input: Option<String> = None;
    let mut from_stdin = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--inline" | "-i" => inline = true,
            "--herdr" => {
                herdr_file = Some(
                    args.next()
                        .ok_or_else(|| "--herdr requires a Mermaid file".to_string())?,
                );
            }
            "--herdr-sidecar" => {
                let origin = args
                    .next()
                    .ok_or_else(|| "--herdr-sidecar requires an origin pane id".to_string())?;
                let path = args
                    .next()
                    .ok_or_else(|| "--herdr-sidecar requires a Mermaid file".to_string())?;
                sidecar = Some((origin, path));
            }
            "--help" | "-h" => {
                println!(
                    "Diaview\n\nUSAGE:\n    diaview [--inline] [diagram.mmd | -]\n    diaview --herdr diagram.mmd\n\nOPTIONS:\n    -i, --inline    Render ANSI output to stdout\n    --herdr         Open a Herdr sidecar pane for agent-backed edits\n    -h, --help      Print help\n    -V, --version   Print version\n\nREQUIRES:\n    Nerd Fonts v3 glyph support"
                );
                return Ok(());
            }
            "--version" | "-V" => {
                println!("diaview {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-" => {
                if input.is_some() {
                    return Err("Only one input may be provided".into());
                }
                input = Some(arg);
                from_stdin = true;
            }
            _ if arg.starts_with('-') => return Err(format!("Unknown option: {arg}")),
            _ if input.is_some() => return Err("Only one input may be provided".into()),
            _ => input = Some(arg),
        }
    }

    if let Some((origin, path)) = sidecar {
        if inline || herdr_file.is_some() || input.is_some() {
            return Err("--herdr-sidecar cannot be combined with other input".into());
        }
        let path = Path::new(&path);
        let source = read_file(path)?;
        let graph = parse_and_layout(&source)?;
        return canvas::render_herdr_sidecar(&graph, path, &origin, source)
            .map_err(|e| format!("Render error: {e}"));
    }

    if let Some(path) = herdr_file {
        if inline || input.is_some() || from_stdin || path == "-" {
            return Err("--herdr requires a real Mermaid file and cannot use stdin".into());
        }
        let origin = std::env::var("HERDR_PANE_ID").map_err(|_| {
            "--herdr must be run from a Herdr-managed pane (HERDR_PANE_ID missing)".to_string()
        })?;
        let diagram = herdr::absolute_file(&path)?;
        let exe =
            std::env::current_exe().map_err(|e| format!("Failed to locate executable: {e}"))?;
        return herdr::launch_sidecar(&exe, &diagram, &origin);
    }

    let input = match input {
        Some(_path) if from_stdin => read_stdin()?,
        Some(path) => read_file(Path::new(&path))?,
        None => {
            let stdin = std::io::stdin();
            if stdin.is_terminal() {
                return Err("Missing input: provide a file, '-', or piped stdin".into());
            }
            from_stdin = true;
            read_stdin()?
        }
    };

    let graph = parse_and_layout(&input)?;

    if inline || from_stdin {
        canvas::render_inline(&graph).map_err(|e| format!("Render error: {e}"))
    } else {
        canvas::render(&graph).map_err(|e| format!("Render error: {e}"))
    }
}

fn parse_and_layout(input: &str) -> Result<diaview::model::Graph, String> {
    let mut graph = mermaid::parse(input).map_err(|e| format!("Parse error: {e}"))?;
    layout::layout(&mut graph);
    Ok(graph)
}

fn read_file(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file '{}': {e}", path.display()))
}

fn read_stdin() -> Result<String, String> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("Failed to read stdin: {e}"))?;
    Ok(input)
}

fn exit_err(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}
