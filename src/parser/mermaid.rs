use crate::model::*;
use std::collections::HashMap;

pub fn parse(input: &str) -> Result<Graph, String> {
    let lines = preprocess(input);
    if lines.is_empty() {
        return Err("empty input".into());
    }

    let (direction, rest) = parse_header(&lines)?;

    let mut nodes: Vec<Node> = Vec::new();
    let mut node_map: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    let mut group_stack: Vec<usize> = Vec::new();

    let mut statements: Vec<String> = Vec::new();
    for line in rest {
        for part in line.split(';') {
            let s = part.trim().to_string();
            if !s.is_empty() {
                statements.push(s);
            }
        }
    }

    for stmt in &statements {
        if let Some(group) =
            parse_subgraph_start(stmt, group_stack.last().map(|&idx| groups[idx].id.clone()))?
        {
            groups.push(group);
            group_stack.push(groups.len() - 1);
            continue;
        }

        if stmt.eq_ignore_ascii_case("end") {
            group_stack
                .pop()
                .ok_or_else(|| "unexpected 'end' without matching subgraph".to_string())?;
            continue;
        }

        let parsed_node_ids = parse_statement(stmt, &mut nodes, &mut node_map, &mut edges)?;
        if let Some(&group_idx) = group_stack.last() {
            add_group_members(&mut groups[group_idx], parsed_node_ids);
        }
    }

    if !group_stack.is_empty() {
        return Err("unclosed subgraph block".into());
    }

    Ok(Graph {
        direction,
        nodes,
        edges,
        groups,
    })
}

fn preprocess(input: &str) -> Vec<String> {
    input
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("%%"))
        .map(|l| l.to_string())
        .collect()
}

fn parse_header<'a>(lines: &'a [String]) -> Result<(Direction, &'a [String]), String> {
    let first = &lines[0];
    let tokens: Vec<&str> = first.split_whitespace().collect();
    if tokens.len() < 2 {
        return Err(format!("invalid header: '{first}'"));
    }
    let keyword = tokens[0].to_lowercase();
    if keyword != "graph" && keyword != "flowchart" {
        return Err(format!(
            "expected 'graph' or 'flowchart', got '{}'",
            tokens[0]
        ));
    }
    let dir = match tokens[1] {
        "TD" | "TB" => Direction::TopDown,
        "LR" => Direction::LeftRight,
        other => return Err(format!("unsupported direction: '{other}'")),
    };
    Ok((dir, &lines[1..]))
}

fn parse_statement(
    stmt: &str,
    nodes: &mut Vec<Node>,
    node_map: &mut HashMap<String, usize>,
    edges: &mut Vec<Edge>,
) -> Result<Vec<String>, String> {
    if let Some(parsed_edge) = try_parse_edge(stmt)? {
        let ParsedEdgeStatement {
            source,
            target,
            edge,
        } = parsed_edge;
        ensure_node(nodes, node_map, &source.id, &source.shape, &source.label);
        ensure_node(nodes, node_map, &target.id, &target.shape, &target.label);
        edges.push(edge);
        Ok(vec![source.id, target.id])
    } else {
        let node = parse_node_decl(stmt)?;
        ensure_node(nodes, node_map, &node.id, &node.shape, &node.label);
        Ok(vec![node.id])
    }
}

fn parse_subgraph_start(stmt: &str, parent: Option<String>) -> Result<Option<Group>, String> {
    let Some(rest) = stmt.strip_prefix("subgraph") else {
        return Ok(None);
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return Err("subgraph missing id or label".into());
    }

    let (id, label) = if let Some(open) = rest.find('[') {
        let id = rest[..open].trim();
        validate_id(id)?;
        let close = rest
            .rfind(']')
            .ok_or_else(|| format!("unclosed '[' in subgraph '{stmt}'"))?;
        (id.to_string(), rest[open + 1..close].trim().to_string())
    } else if let Some(open) = rest.find('(') {
        let id = rest[..open].trim();
        validate_id(id)?;
        let close = rest
            .rfind(')')
            .ok_or_else(|| format!("unclosed '(' in subgraph '{stmt}'"))?;
        (id.to_string(), rest[open + 1..close].trim().to_string())
    } else {
        (rest.to_string(), rest.to_string())
    };

    Ok(Some(Group {
        id,
        label,
        node_ids: Vec::new(),
        parent,
        x: None,
        y: None,
        width: None,
        height: None,
    }))
}

fn add_group_members(group: &mut Group, node_ids: Vec<String>) {
    for node_id in node_ids {
        if !group.node_ids.iter().any(|existing| existing == &node_id) {
            group.node_ids.push(node_id);
        }
    }
}

fn ensure_node(
    nodes: &mut Vec<Node>,
    node_map: &mut HashMap<String, usize>,
    id: &str,
    shape: &NodeShape,
    label: &str,
) {
    if let Some(&idx) = node_map.get(id) {
        let existing = &mut nodes[idx];
        if existing.label == existing.id && label != id {
            existing.label = label.to_string();
            existing.shape = shape.clone();
        }
    } else {
        let idx = nodes.len();
        nodes.push(Node {
            id: id.to_string(),
            label: label.to_string(),
            shape: shape.clone(),
            x: None,
            y: None,
            width: None,
            height: None,
        });
        node_map.insert(id.to_string(), idx);
    }
}

#[derive(Debug, Clone)]
struct ParsedNodeDecl {
    id: String,
    shape: NodeShape,
    label: String,
}

#[derive(Debug, Clone)]
struct ParsedEdgeStatement {
    source: ParsedNodeDecl,
    target: ParsedNodeDecl,
    edge: Edge,
}

impl ParsedNodeDecl {
    fn new(id: impl Into<String>, shape: NodeShape, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            shape,
            label: label.into(),
        }
    }
}

const EDGE_OPS: &[(&str, EdgeStyle, Arrowhead)] = &[
    ("==>", EdgeStyle::Solid, Arrowhead::Normal),
    ("-.->", EdgeStyle::Dashed, Arrowhead::Normal),
    ("-.-", EdgeStyle::Dashed, Arrowhead::None),
    ("-->", EdgeStyle::Solid, Arrowhead::Normal),
    ("---", EdgeStyle::Solid, Arrowhead::None),
    ("--->", EdgeStyle::Solid, Arrowhead::Normal),
];

fn try_parse_edge(stmt: &str) -> Result<Option<ParsedEdgeStatement>, String> {
    if let Some(result) = try_inline_label_edge(stmt)? {
        return Ok(Some(result));
    }

    for &(op, ref style, ref arrow) in EDGE_OPS {
        if let Some(pos) = find_edge_op(stmt, op) {
            let lhs = stmt[..pos].trim();
            let mut rhs = stmt[pos + op.len()..].trim().to_string();

            let label = if rhs.starts_with('|') {
                let end_pipe = rhs[1..]
                    .find('|')
                    .ok_or_else(|| format!("unclosed pipe label in '{stmt}'"))?;
                let lbl = rhs[1..1 + end_pipe].to_string();
                rhs = rhs[1 + end_pipe + 1..].trim().to_string();
                Some(lbl)
            } else {
                None
            };

            let source = parse_node_decl(lhs)?;
            let target = parse_node_decl(&rhs)?;
            let edge = Edge {
                source: source.id.clone(),
                target: target.id.clone(),
                label,
                style: style.clone(),
                arrowhead: arrow.clone(),
                route: None,
            };
            return Ok(Some(ParsedEdgeStatement {
                source,
                target,
                edge,
            }));
        }
    }

    Ok(None)
}

fn try_inline_label_edge(stmt: &str) -> Result<Option<ParsedEdgeStatement>, String> {
    let Some(dash_pos) = stmt.find(" -- ") else {
        return Ok(None);
    };

    let lhs = stmt[..dash_pos].trim();
    let after = &stmt[dash_pos + " -- ".len()..];

    let Some((op, style, arrow, op_pos)) = EDGE_OPS
        .iter()
        .filter_map(|&(op, ref s, ref a)| {
            find_edge_op(after, op).map(|p| (op, s.clone(), a.clone(), p))
        })
        .min_by_key(|t| t.3)
    else {
        return Ok(None);
    };

    let label_text = after[..op_pos].trim().to_string();
    let rhs = after[op_pos + op.len()..].trim();

    if label_text.is_empty() || rhs.is_empty() {
        return Ok(None);
    }

    let source = parse_node_decl(lhs)?;
    let target = parse_node_decl(rhs)?;

    let edge = Edge {
        source: source.id.clone(),
        target: target.id.clone(),
        label: Some(label_text),
        style,
        arrowhead: arrow,
        route: None,
    };

    Ok(Some(ParsedEdgeStatement {
        source,
        target,
        edge,
    }))
}

fn find_edge_op(s: &str, op: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let op_bytes = op.as_bytes();
    let op_len = op_bytes.len();
    if bytes.len() < op_len {
        return None;
    }
    let mut depth_square = 0i32;
    let mut depth_paren = 0i32;
    let mut depth_curly = 0i32;
    let mut i = 0;
    while i + op_len <= bytes.len() {
        match bytes[i] {
            b'[' => depth_square += 1,
            b']' => depth_square -= 1,
            b'(' => depth_paren += 1,
            b')' => depth_paren -= 1,
            b'{' => depth_curly += 1,
            b'}' => depth_curly -= 1,
            _ => {}
        }
        if depth_square == 0 && depth_paren == 0 && depth_curly == 0 {
            if &bytes[i..i + op_len] == op_bytes {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn parse_node_decl(s: &str) -> Result<ParsedNodeDecl, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty node declaration".into());
    }

    if let Some(open) = s.find("((") {
        let id = &s[..open];
        validate_id(id)?;
        let rest = &s[open + 2..];
        let close = rest
            .find("))")
            .ok_or_else(|| format!("unclosed '((' in node '{s}'"))?;
        let label = rest[..close].to_string();
        return Ok(ParsedNodeDecl::new(id, NodeShape::Circle, label));
    }

    if let Some(open) = s.find("[(") {
        let id = &s[..open];
        validate_id(id)?;
        let rest = &s[open + 2..];
        let close = rest
            .rfind(")]")
            .ok_or_else(|| format!("unclosed '[(' in node '{s}'"))?;
        let label = rest[..close].to_string();
        return Ok(ParsedNodeDecl::new(id, NodeShape::Rectangle, label));
    }

    if let Some(open) = s.find('(') {
        let id = &s[..open];
        validate_id(id)?;
        let rest = &s[open + 1..];
        let close = rest
            .rfind(')')
            .ok_or_else(|| format!("unclosed '(' in node '{s}'"))?;
        let label = rest[..close].to_string();
        return Ok(ParsedNodeDecl::new(id, NodeShape::RoundedRect, label));
    }

    if let Some(open) = s.find('[') {
        let id = &s[..open];
        validate_id(id)?;
        let rest = &s[open + 1..];
        let close = rest
            .rfind(']')
            .ok_or_else(|| format!("unclosed '[' in node '{s}'"))?;
        let label = rest[..close].to_string();
        return Ok(ParsedNodeDecl::new(id, NodeShape::Rectangle, label));
    }

    if let Some(open) = s.find('{') {
        let id = &s[..open];
        validate_id(id)?;
        let rest = &s[open + 1..];
        let close = rest
            .rfind('}')
            .ok_or_else(|| format!("unclosed '{{' in node '{s}'"))?;
        let label = rest[..close].to_string();
        return Ok(ParsedNodeDecl::new(id, NodeShape::Diamond, label));
    }

    validate_id(s)?;
    Ok(ParsedNodeDecl::new(s, NodeShape::Rectangle, s))
}

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("node id is empty".into());
    }
    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!("invalid node id: '{id}'"));
    }
    Ok(())
}
