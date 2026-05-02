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
            _ => path = Some(arg),
        }
    }

    let input = match path {
        Some(path) => std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("Failed to read file '{path}': {e}");
            std::process::exit(1);
        }),
        None => fixtures::complex_architecture_mermaid().to_string(),
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
