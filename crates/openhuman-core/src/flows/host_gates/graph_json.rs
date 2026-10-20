//! Read-only traversal helpers for TinyFlows' normalized workflow JSON.

use serde_json::Value;

pub(crate) struct GraphNode<'a> {
    pub(crate) id: &'a str,
    pub(crate) kind: &'a str,
    pub(crate) config: &'a Value,
}

/// Iterates structurally valid nodes from TinyFlows' normalized graph JSON.
pub(crate) fn nodes(graph: &Value) -> impl Iterator<Item = GraphNode<'_>> {
    graph
        .get("nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|node| {
            Some(GraphNode {
                id: node.get("id")?.as_str()?,
                kind: node.get("kind")?.as_str()?,
                config: node.get("config").unwrap_or(&Value::Null),
            })
        })
}
