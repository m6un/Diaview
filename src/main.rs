use diaview::layout;
use diaview::parser::mermaid;
use diaview::renderer::canvas;
use std::io::IsTerminal;

fn main() {
    let mut inline = false;
    let mut input: Option<String> = None;
    let mut from_stdin = false;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--inline" | "-i" => inline = true,
            "--help" | "-h" => {
                println!(
                    "Diaview\n\nUSAGE:\n    diaview [--inline] [diagram.mmd | -]\n\nOPTIONS:\n    -i, --inline    Render ANSI output to stdout\n    -h, --help      Print help\n    -V, --version   Print version\n\nREQUIRES:\n    Nerd Fonts v3 glyph support"
                );
                return;
            }
            "--version" | "-V" => {
                println!("diaview {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "-" => {
                if input.is_some() {
                    eprintln!("Only one input may be provided");
                    std::process::exit(1);
                }
                input = Some(arg);
                from_stdin = true;
            }
            _ if arg.starts_with('-') => {
                eprintln!("Unknown option: {arg}");
                std::process::exit(1);
            }
            _ if input.is_some() => {
                eprintln!("Only one input may be provided");
                std::process::exit(1);
            }
            _ => input = Some(arg),
        }
    }

    let input = match input {
        Some(_path) if from_stdin => read_stdin().unwrap_or_else(|e| exit_err(&e)),
        Some(path) => std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("Failed to read file '{path}': {e}");
            std::process::exit(1);
        }),
        None => {
            let stdin = std::io::stdin();
            if stdin.is_terminal() {
                eprintln!("Missing input: provide a file, '-', or piped stdin");
                std::process::exit(1);
            }
            from_stdin = true;
            read_stdin().unwrap_or_else(|e| exit_err(&e))
        }
    };

    let mut graph = mermaid::parse(&input).unwrap_or_else(|e| {
        eprintln!("Parse error: {e}");
        std::process::exit(1);
    });

    layout::layout(&mut graph);

    let result = if inline || from_stdin {
        canvas::render_inline(&graph)
    } else {
        canvas::render(&graph)
    };

    result.unwrap_or_else(|e| {
        eprintln!("Render error: {e}");
        std::process::exit(1);
    });
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
