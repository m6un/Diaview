use diaview::parser::mermaid;
use diaview::layout;
use diaview::renderer::canvas;

fn main() {
    let input = std::env::args()
        .nth(1)
        .map(|path| std::fs::read_to_string(&path).expect("Failed to read file"))
        .unwrap_or_else(|| {
            r#"graph TD
    A[Start] --> B{Decision}
    B -->|yes| C(Process)
    B -->|no| D((End))
    C --> D
"#
            .to_string()
        });

    let mut graph = mermaid::parse(&input).unwrap_or_else(|e| {
        eprintln!("Parse error: {e}");
        std::process::exit(1);
    });

    layout::layout(&mut graph);

    canvas::render(&graph).unwrap_or_else(|e| {
        eprintln!("Render error: {e}");
        std::process::exit(1);
    });
}
