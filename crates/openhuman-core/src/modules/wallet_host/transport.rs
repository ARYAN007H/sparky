pub(super) enum TransportFailure {
    Unreachable(String),
    Rpc(String),
}

pub(super) fn classify_transport_error(message: String) -> TransportFailure {
    if ["transport failed", "read body failed"]
        .iter()
        .any(|marker| message.contains(marker))
    {
        TransportFailure::Unreachable(message)
    } else {
        TransportFailure::Rpc(message)
    }
}

pub(super) fn validate_module_endpoint_url(raw: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(raw)
        .map_err(|_| "wallet module selected an invalid RPC endpoint URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("wallet module selected an unsafe RPC endpoint URL".into());
    }
    Ok(raw.to_owned())
}
