use diaview::layout;
use diaview::parser::mermaid;
use diaview::renderer::canvas;
use diaview::testdata::fixtures;

fn main() {
    let mut inline = false;
    let mut path: Option<String> = None;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--inline" | "-i" => inline = true,
            "--help" | "-h" => {
                println!(
                    "Diaview\n\nUSAGE:\n    diaview [--inline] [diagram.mmd]\n\nOPTIONS:\n    -i, --inline    Render ANSI output to stdout\n    -h, --help      Print help\n    -V, --version   Print version\n\nREQUIRES:\n    Nerd Fonts v3 glyph support"
                );
                return;
            }
            "--version" | "-V" => {
                println!("diaview {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            _ if arg.starts_with('-') => {
                eprintln!("Unknown option: {arg}");
                std::process::exit(1);
            }
            _ if path.is_some() => {
                eprintln!("Only one diagram file may be provided");
                std::process::exit(1);
            }
            _ => path = Some(arg),
        }
    }

    let input = match path {
        Some(path) => std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("Failed to read file '{path}': {e}");
            std::process::exit(1);
        }),
        None => fixtures::simple_mermaid().to_string(),
    };

    let mut graph = mermaid::parse(&input).unwrap_or_else(|e| {
        eprintln!("Parse error: {e}");
        std::process::exit(1);
    });

    layout::layout(&mut graph);

    let result = if inline {
        canvas::render_inline(&graph)
    } else {
        canvas::render(&graph)
    };

    result.unwrap_or_else(|e| {
        eprintln!("Render error: {e}");
        std::process::exit(1);
    });
}
