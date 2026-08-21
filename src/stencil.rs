use crate::model::{Node, NodeShape};

// Nerd Fonts v3 Material Design glyphs. Diaview requires a Nerd Font-patched
// terminal font or Symbols Nerd Font Mono configured as a fallback.
const ICON_DATABASE: &str = "\u{f01bc}"; // nf-md-database
const ICON_BUCKET: &str = "\u{f1415}"; // nf-md-bucket
const ICON_QUEUE: &str = "\u{f1296}"; // nf-md-tray_full
const ICON_EVENT: &str = "\u{f04c1}"; // nf-md-source_fork
const ICON_FUNCTION: &str = "\u{f0295}"; // nf-md-function
const ICON_WORKER: &str = "\u{f0493}"; // nf-md-cog
const ICON_CACHE: &str = "\u{f00e8}"; // nf-md-cached
const ICON_GATEWAY: &str = "\u{f11e2}"; // nf-md-router
const ICON_SECURITY: &str = "\u{f0498}"; // nf-md-shield
const ICON_OBSERVABILITY: &str = "\u{f0430}"; // nf-md-pulse
const ICON_CLIENT: &str = "\u{f0379}"; // nf-md-monitor
const ICON_EXTERNAL: &str = "\u{f015f}"; // nf-md-cloud

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Generic,
    Database,
    Bucket,
    Queue,
    Event,
    Function,
    Worker,
    Cache,
    ApiGateway,
    Security,
    Observability,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeStencil {
    pub kind: ArtifactKind,
    pub icon: &'static str,
}

impl NodeStencil {
    pub const fn generic() -> Self {
        Self {
            kind: ArtifactKind::Generic,
            icon: "",
        }
    }

    pub const fn new(kind: ArtifactKind, icon: &'static str) -> Self {
        Self { kind, icon }
    }

    pub const fn is_generic(self) -> bool {
        matches!(self.kind, ArtifactKind::Generic)
    }
}

pub fn stencil_for_node(node: &Node) -> NodeStencil {
    if node.id.starts_with("__dummy") || matches!(node.shape, NodeShape::Diamond) {
        return NodeStencil::generic();
    }

    if matches!(node.shape, NodeShape::Database) {
        return NodeStencil::new(ArtifactKind::Database, ICON_DATABASE);
    }

    infer_stencil(&format!("{} {}", node.id, node.label))
}

pub fn node_display_cell_width(node: &Node) -> usize {
    let label_width = node.label.chars().count();
    let stencil = stencil_for_node(node);
    if !stencil.is_generic() {
        return 1 + 2 + label_width;
    }

    match node.shape {
        NodeShape::Diamond | NodeShape::Circle => label_width + 2,
        NodeShape::Rectangle | NodeShape::RoundedRect | NodeShape::Database => label_width,
    }
}

fn infer_stencil(text: &str) -> NodeStencil {
    let lower = text.to_ascii_lowercase();
    let tokens = tokens(&lower);

    if has_token(&tokens, &["s3", "r2", "bucket", "buckets", "blob", "blobs"])
        || contains_any(&lower, &["object storage", "blob storage"])
    {
        return NodeStencil::new(ArtifactKind::Bucket, ICON_BUCKET);
    }

    if contains_any(&lower, &["durable object", "cloudflare worker"])
        || has_token(&tokens, &["worker", "workers"])
    {
        return NodeStencil::new(ArtifactKind::Worker, ICON_WORKER);
    }

    if has_token(&tokens, &["lambda", "function", "functions", "fn"])
        || contains_any(&lower, &["cloud function", "edge function"])
    {
        return NodeStencil::new(ArtifactKind::Function, ICON_FUNCTION);
    }

    if contains_any(&lower, &["event bus", "dead letter topic"])
        || has_token(
            &tokens,
            &[
                "event",
                "events",
                "eventbridge",
                "kafka",
                "kinesis",
                "pubsub",
                "stream",
                "streams",
                "topic",
                "topics",
            ],
        )
    {
        return NodeStencil::new(ArtifactKind::Event, ICON_EVENT);
    }

    if contains_any(&lower, &["dead letter"])
        || has_token(
            &tokens,
            &["queue", "queues", "sqs", "rabbitmq", "nats", "dlq"],
        )
    {
        return NodeStencil::new(ArtifactKind::Queue, ICON_QUEUE);
    }

    if has_token(
        &tokens,
        &[
            "redis",
            "memcached",
            "cache",
            "cached",
            "caches",
            "cdn",
            "kv",
            "elasticache",
        ],
    ) {
        return NodeStencil::new(ArtifactKind::Cache, ICON_CACHE);
    }

    if has_token(
        &tokens,
        &[
            "postgres",
            "postgresql",
            "mysql",
            "mariadb",
            "sqlite",
            "rds",
            "aurora",
            "dynamodb",
            "mongo",
            "mongodb",
            "cassandra",
            "clickhouse",
            "warehouse",
            "database",
            "databases",
            "db",
            "d1",
        ],
    ) {
        return NodeStencil::new(ArtifactKind::Database, ICON_DATABASE);
    }

    if contains_any(&lower, &["api gateway", "load balancer"])
        || has_token(&tokens, &["api", "gateway", "ingress", "router", "alb"])
    {
        return NodeStencil::new(ArtifactKind::ApiGateway, ICON_GATEWAY);
    }

    if has_token(
        &tokens,
        &[
            "auth",
            "authenticated",
            "authentication",
            "authorization",
            "jwt",
            "oauth",
            "iam",
            "cognito",
            "okta",
            "security",
            "waf",
            "tls",
            "captcha",
        ],
    ) {
        return NodeStencil::new(ArtifactKind::Security, ICON_SECURITY);
    }

    if has_token(
        &tokens,
        &[
            "metrics",
            "metric",
            "logs",
            "log",
            "traces",
            "trace",
            "telemetry",
            "observability",
            "cloudwatch",
            "datadog",
            "prometheus",
            "grafana",
            "alerts",
            "alert",
            "audit",
            "analytics",
        ],
    ) {
        return NodeStencil::new(ArtifactKind::Observability, ICON_OBSERVABILITY);
    }

    if has_token(&tokens, &["client", "browser", "mobile"]) {
        return NodeStencil::new(ArtifactKind::External, ICON_CLIENT);
    }

    if has_token(
        &tokens,
        &[
            "external",
            "third",
            "party",
            "stripe",
            "github",
            "slack",
            "pager",
            "pagerduty",
            "dns",
            "cloud",
        ],
    ) {
        return NodeStencil::new(ArtifactKind::External, ICON_EXTERNAL);
    }

    NodeStencil::generic()
}

fn tokens(text: &str) -> Vec<&str> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect()
}

fn has_token(tokens: &[&str], needles: &[&str]) -> bool {
    tokens
        .iter()
        .any(|token| needles.iter().any(|needle| token == needle))
}

fn contains_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(label: &str) -> Node {
        Node {
            id: label.replace(' ', "_"),
            label: label.into(),
            shape: NodeShape::Rectangle,
            x: None,
            y: None,
            width: None,
            height: None,
        }
    }

    #[test]
    fn common_artifacts_use_expected_nerd_font_icons() {
        for (label, kind, icon) in [
            ("Postgres DB", ArtifactKind::Database, ICON_DATABASE),
            ("S3 Bucket", ArtifactKind::Bucket, ICON_BUCKET),
            ("SQS Queue", ArtifactKind::Queue, ICON_QUEUE),
            ("Kafka Event Bus", ArtifactKind::Event, ICON_EVENT),
            ("Resize Function", ArtifactKind::Function, ICON_FUNCTION),
            ("Email Worker", ArtifactKind::Worker, ICON_WORKER),
            ("Redis Cache", ArtifactKind::Cache, ICON_CACHE),
            ("API Gateway", ArtifactKind::ApiGateway, ICON_GATEWAY),
            ("JWT Auth", ArtifactKind::Security, ICON_SECURITY),
            ("Metrics", ArtifactKind::Observability, ICON_OBSERVABILITY),
            ("Web Client", ArtifactKind::External, ICON_CLIENT),
            ("Stripe External", ArtifactKind::External, ICON_EXTERNAL),
        ] {
            assert_eq!(stencil_for_node(&node(label)), NodeStencil::new(kind, icon));
        }
    }
}
