/// Test helpers and sample Mermaid diagrams.
pub mod fixtures {
    use crate::model::*;

    /// A simple 2-node linear graph: A("Start") --> B("End")
    pub fn simple_two_node() -> Graph {
        Graph {
            direction: Direction::TopDown,
            nodes: vec![
                Node {
                    id: "A".into(),
                    label: "Start".into(),
                    shape: NodeShape::RoundedRect,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
                Node {
                    id: "B".into(),
                    label: "End".into(),
                    shape: NodeShape::RoundedRect,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
            ],
            edges: vec![Edge {
                source: "A".into(),
                target: "B".into(),
                label: Some("next".into()),
                style: EdgeStyle::Solid,
                arrowhead: Arrowhead::Normal,
                route: None,
            }],
            groups: vec![],
        }
    }

    /// A diamond decision graph: A --> B{Decision} -->|yes| C, -->|no| D
    pub fn diamond_decision() -> Graph {
        Graph {
            direction: Direction::TopDown,
            nodes: vec![
                Node {
                    id: "A".into(),
                    label: "Start".into(),
                    shape: NodeShape::RoundedRect,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
                Node {
                    id: "B".into(),
                    label: "Decision".into(),
                    shape: NodeShape::Diamond,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
                Node {
                    id: "C".into(),
                    label: "Yes path".into(),
                    shape: NodeShape::Rectangle,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
                Node {
                    id: "D".into(),
                    label: "No path".into(),
                    shape: NodeShape::Rectangle,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
            ],
            edges: vec![
                Edge {
                    source: "A".into(),
                    target: "B".into(),
                    label: None,
                    style: EdgeStyle::Solid,
                    arrowhead: Arrowhead::Normal,
                    route: None,
                },
                Edge {
                    source: "B".into(),
                    target: "C".into(),
                    label: Some("yes".into()),
                    style: EdgeStyle::Solid,
                    arrowhead: Arrowhead::Normal,
                    route: None,
                },
                Edge {
                    source: "B".into(),
                    target: "D".into(),
                    label: Some("no".into()),
                    style: EdgeStyle::Dashed,
                    arrowhead: Arrowhead::Normal,
                    route: None,
                },
            ],
            groups: vec![],
        }
    }

    /// Medium-complexity demo graph intended for regular visual inspection.
    pub const SIMPLE_MERMAID: &str = include_str!("../fixtures/simple.mmd");

    /// A larger real-world architecture flowchart used for Phase 1.5 layout/routing inspection.
    pub const COMPLEX_ARCHITECTURE_MERMAID: &str =
        include_str!("../fixtures/complex_architecture.mmd");

    /// Many producers converging on one terminal sink.
    pub const PHASE15_FAN_IN_SINK_MERMAID: &str = r#"
flowchart TD
    START([Request]) --> AUTH[Authenticate]
    START --> RATE[Rate limit]
    AUTH --> PROFILE[Profile service]
    AUTH --> BILLING[Billing service]
    RATE --> QUEUE[Work queue]
    PROFILE --> SUCCESS((Unified success sink))
    BILLING --> SUCCESS
    QUEUE --> SUCCESS
    CACHE[Warm cache] --> SUCCESS
    AUDIT[Audit writer] --> SUCCESS
    NOTIFY[Notifier] --> SUCCESS
"#;

    /// One router distributing work to many downstream services.
    pub const PHASE15_FAN_OUT_ROUTER_MERMAID: &str = r#"
flowchart LR
    ENTRY([Ingress]) --> ROUTER{Route request}
    ROUTER --> API[Public API]
    ROUTER --> ADMIN[Admin API]
    ROUTER --> WEBHOOKS[Webhook worker]
    ROUTER --> EXPORTS[Export worker]
    ROUTER --> SEARCH[Search indexer]
    ROUTER --> EMAIL[Email sender]
    API --> DONE((Done))
    ADMIN --> DONE
    WEBHOOKS --> DONE
    EXPORTS --> DONE
    SEARCH --> DONE
    EMAIL --> DONE
"#;

    /// Mostly-forward graph with an explicit back edge to exercise cycle-ish ordering.
    pub const PHASE15_BACK_EDGE_CYCLE_MERMAID: &str = r#"
flowchart TD
    PLAN[Plan job] --> FETCH[Fetch data]
    FETCH --> VALIDATE{Valid?}
    VALIDATE --> TRANSFORM[Transform]
    TRANSFORM --> STORE[Store result]
    STORE --> COMPLETE((Complete))
    VALIDATE --> RETRY[Retry policy]
    RETRY --> FETCH
    STORE -.-> VALIDATE
"#;

    /// Primary request path overlaid with dashed telemetry edges into shared observability sinks.
    pub const PHASE15_TELEMETRY_OVERLAY_MERMAID: &str = r#"
flowchart LR
    CLIENT([Client]) --> EDGE[Edge]
    EDGE --> API[API]
    API --> WORKER[Worker]
    WORKER --> DB[Database]
    WORKER --> RESPONSE((Response))
    EDGE -.-> LOGS[Logs]
    API -.-> LOGS
    WORKER -.-> LOGS
    DB -.-> LOGS
    EDGE -.-> METRICS[Metrics]
    API -.-> METRICS
    WORKER -.-> METRICS
    DB -.-> METRICS
    METRICS --> ALERTS[Alerts]
"#;

    /// Subgraph-like architecture fixture: grouped by label prefixes because group IR is future work.
    pub const PHASE15_GROUPED_ARCHITECTURE_MERMAID: &str = r#"
flowchart TD
    CLIENT_WEB[Client / Web] --> EDGE_CDN[Edge / CDN]
    CLIENT_MOBILE[Client / Mobile] --> EDGE_CDN
    EDGE_CDN --> API_GATEWAY[API / Gateway]
    API_GATEWAY --> API_AUTH[API / Auth]
    API_GATEWAY --> API_GRAPHQL[API / GraphQL]
    API_AUTH --> SVC_USERS[Services / Users]
    API_GRAPHQL --> SVC_ORDERS[Services / Orders]
    API_GRAPHQL --> SVC_CATALOG[Services / Catalog]
    SVC_USERS --> DATA_USERS[Data / Users DB]
    SVC_ORDERS --> DATA_ORDERS[Data / Orders DB]
    SVC_CATALOG --> DATA_SEARCH[Data / Search]
    SVC_USERS -.-> OBS_LOGS[Observability / Logs]
    SVC_ORDERS -.-> OBS_LOGS
    SVC_CATALOG -.-> OBS_LOGS
    DATA_USERS -.-> OBS_METRICS[Observability / Metrics]
    DATA_ORDERS -.-> OBS_METRICS
    DATA_SEARCH -.-> OBS_METRICS
    OBS_METRICS --> EXT_PAGER[External / Pager]
"#;

    /// Medium-complexity demo graph intended for regular visual inspection.
    pub fn simple_mermaid() -> &'static str {
        SIMPLE_MERMAID
    }

    /// A larger real-world architecture flowchart used for Phase 1.5 layout/routing inspection.
    pub fn complex_architecture_mermaid() -> &'static str {
        COMPLEX_ARCHITECTURE_MERMAID
    }

    pub fn phase15_fan_in_sink_mermaid() -> &'static str {
        PHASE15_FAN_IN_SINK_MERMAID
    }

    pub fn phase15_fan_out_router_mermaid() -> &'static str {
        PHASE15_FAN_OUT_ROUTER_MERMAID
    }

    pub fn phase15_back_edge_cycle_mermaid() -> &'static str {
        PHASE15_BACK_EDGE_CYCLE_MERMAID
    }

    pub fn phase15_telemetry_overlay_mermaid() -> &'static str {
        PHASE15_TELEMETRY_OVERLAY_MERMAID
    }

    pub fn phase15_grouped_architecture_mermaid() -> &'static str {
        PHASE15_GROUPED_ARCHITECTURE_MERMAID
    }

    /// Left-right direction, 3 nodes in a chain
    pub fn left_right_chain() -> Graph {
        Graph {
            direction: Direction::LeftRight,
            nodes: vec![
                Node {
                    id: "A".into(),
                    label: "Input".into(),
                    shape: NodeShape::Rectangle,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
                Node {
                    id: "B".into(),
                    label: "Process".into(),
                    shape: NodeShape::RoundedRect,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
                Node {
                    id: "C".into(),
                    label: "Output".into(),
                    shape: NodeShape::Circle,
                    x: None,
                    y: None,
                    width: None,
                    height: None,
                },
            ],
            edges: vec![
                Edge {
                    source: "A".into(),
                    target: "B".into(),
                    label: None,
                    style: EdgeStyle::Solid,
                    arrowhead: Arrowhead::Normal,
                    route: None,
                },
                Edge {
                    source: "B".into(),
                    target: "C".into(),
                    label: Some("result".into()),
                    style: EdgeStyle::Dotted,
                    arrowhead: Arrowhead::Open,
                    route: None,
                },
            ],
            groups: vec![],
        }
    }
}
