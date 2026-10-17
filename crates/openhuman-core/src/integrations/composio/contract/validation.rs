//! Composio contract validation that can be shared by flow authoring stages.

/// A graph node reduced to the Composio fields needed by the connection gate.
#[derive(Debug, Clone)]
pub(crate) struct ConnectionRefNode {
    pub id: String,
    pub slug: String,
    pub connection_ref: String,
}

/// A live account reduced to the fields used by the connection gate.
#[derive(Debug, Clone)]
pub(crate) struct ConnectedAccountRef {
    pub connection_ref: String,
    pub toolkit: Option<String>,
}

/// Parses a `composio:<toolkit>:<connection_id>` reference.
pub(crate) fn parse_composio_connection_ref(conn_ref: &str) -> Option<(&str, &str)> {
    let rest = conn_ref.strip_prefix("composio:")?;
    let (toolkit, id) = rest.rsplit_once(':')?;
    if toolkit.trim().is_empty() || id.trim().is_empty() {
        return None;
    }
    Some((toolkit.trim(), id.trim()))
}

fn first_connection_ref_for_toolkit<'a>(
    connections: &'a [ConnectedAccountRef],
    toolkit: &str,
) -> Option<&'a str> {
    connections
        .iter()
        .find(|connection| {
            connection
                .toolkit
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case(toolkit))
        })
        .map(|connection| connection.connection_ref.as_str())
}

/// Validates connection shape, toolkit match and (when available) account id.
/// `None` connections means live lookup failed and preserves fail-open behavior.
pub(crate) fn validate_connection_refs(
    nodes: &[ConnectionRefNode],
    connections: Option<&[ConnectedAccountRef]>,
) -> Vec<String> {
    use crate::integrations::composio::contract::toolkit_from_slug;

    let mut errors = Vec::new();
    for node in nodes {
        let slug = node.slug.as_str();
        if slug.starts_with('=') || slug.starts_with("oh:") || node.connection_ref.trim().is_empty() {
            continue;
        }
        let Some(slug_toolkit) = toolkit_from_slug(slug) else {
            continue;
        };
        let conn_ref = node.connection_ref.as_str();
        let Some((ref_toolkit, ref_id)) = parse_composio_connection_ref(conn_ref) else {
            errors.push(format!(
                "Node '{}': `connection_ref` `{conn_ref}` is malformed — a Composio account ref \
                 must look like `composio:<toolkit>:<connection_id>` (e.g. \
                 `composio:{slug_toolkit}:<id>`). Call list_flow_connections and copy a \
                 `connection_ref` value verbatim.",
                node.id
            ));
            continue;
        };
        if !ref_toolkit.eq_ignore_ascii_case(&slug_toolkit) {
            let hint = match connections.and_then(|items| first_connection_ref_for_toolkit(items, &slug_toolkit)) {
                Some(reference) => format!(" — did you mean `{reference}`?"),
                None => format!(" — no `{slug_toolkit}` account is connected; connect one with composio_connect (or ask the user to), then use its `connection_ref`"),
            };
            errors.push(format!(
                "Node '{}': `connection_ref` `{conn_ref}` names the `{ref_toolkit}` toolkit but the \
                 tool_call slug `{slug}` is a `{slug_toolkit}` action{hint}.",
                node.id
            ));
            continue;
        }
        let Some(connections) = connections else { continue };
        let id_exists = connections.iter().any(|connection| {
            connection.toolkit.as_deref().is_some_and(|toolkit| toolkit.eq_ignore_ascii_case(&slug_toolkit))
                && parse_composio_connection_ref(&connection.connection_ref)
                    .is_some_and(|(_, id)| id.eq_ignore_ascii_case(ref_id))
        });
        if id_exists { continue; }
        match first_connection_ref_for_toolkit(connections, &slug_toolkit) {
            Some(reference) => errors.push(format!(
                "Node '{}': `connection_ref` `{conn_ref}` does not match any connected `{slug_toolkit}` account — did you mean `{reference}`?",
                node.id
            )),
            None => errors.push(format!(
                "Node '{}': `connection_ref` `{conn_ref}` names a `{slug_toolkit}` account, but none is connected. Connect one with composio_connect (or ask the user to), then use its `connection_ref`.",
                node.id
            )),
        }
    }
    errors
}
