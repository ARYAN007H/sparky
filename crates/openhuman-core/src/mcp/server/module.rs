//! TinyMCP server-protocol adapter over the published bus contract.

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{Map, Value};
use tinymcp_bus::{
    names, ServerHostReply, ServerHttpResponseMetadata, ServerHttpSessionConfig,
    ServerHttpSessionRequest, ServerInput, ServerOperationRef, ServerOperationSnapshot,
    ServerOperationState, ServerSessionConfig,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    config::Config,
    modules::{client::ModuleCallError, client::ModuleClient},
};

use super::handler;

const MODULE: &str = "tinymcp";
const POLL_INTERVAL: Duration = Duration::from_millis(5);
static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, thiserror::Error)]
pub(super) enum ServerError {
    #[error("MCP module {0}")]
    Module(ModuleCallError),
    #[error("MCP server operation was cancelled")]
    Cancelled,
    #[error("MCP server module returned an unknown operation state")]
    UnknownState,
    #[error("MCP server module operation failed: {0}")]
    Failed(String),
    #[error("MCP server module returned no response metadata")]
    MissingHttpMetadata,
    #[error("MCP server module returned an invalid HTTP status")]
    InvalidHttpStatus,
}

#[async_trait]
pub(super) trait ServerPort: Send + Sync {
    async fn open(&self, config: ServerSessionConfig) -> Result<String, ModuleCallError>;
    async fn open_http(&self, config: ServerHttpSessionConfig)
        -> Result<String, ModuleCallError>;
    async fn submit(
        &self,
        session: &str,
        headers: Map<String, Value>,
        input: ServerInput,
    ) -> Result<(), ModuleCallError>;
    async fn poll(&self, session: &str) -> Result<ServerOperationSnapshot, ModuleCallError>;
    async fn complete(
        &self,
        session: &str,
        callback: &str,
        reply: ServerHostReply,
    ) -> Result<(), ModuleCallError>;
    async fn cancel(&self, operation: ServerOperationRef) -> Result<(), ModuleCallError>;
    async fn close(&self, session: &str) -> Result<(), ModuleCallError>;
    async fn http_session(
        &self,
        request: ServerHttpSessionRequest,
    ) -> Result<ServerHttpResponseMetadata, ModuleCallError>;
    async fn metadata(
        &self,
        session: &str,
    ) -> Result<tinymcp_bus::ServerSessionMetadata, ModuleCallError>;
    async fn shutdown(&self) -> Result<usize, ModuleCallError>;
}

struct TinyMcpPort {
    client: ModuleClient,
}

#[async_trait]
impl ServerPort for TinyMcpPort {
    async fn open(&self, config: ServerSessionConfig) -> Result<String, ModuleCallError> {
        self.client.call(MODULE, names::methods::SERVER_OPEN, (config,)).await
    }

    async fn open_http(
        &self,
        config: ServerHttpSessionConfig,
    ) -> Result<String, ModuleCallError> {
        self.client
            .call(MODULE, names::methods::SERVER_OPEN_HTTP, (config,))
            .await
    }

    async fn submit(
        &self,
        session: &str,
        headers: Map<String, Value>,
        input: ServerInput,
    ) -> Result<(), ModuleCallError> {
        self.client
            .call(
                MODULE,
                names::methods::SERVER_SUBMIT,
                (session.to_owned(), headers, input),
            )
            .await
    }

    async fn poll(&self, session: &str) -> Result<ServerOperationSnapshot, ModuleCallError> {
        self.client
            .call(MODULE, names::methods::SERVER_POLL, (session.to_owned(),))
            .await
    }

    async fn complete(
        &self,
        session: &str,
        callback: &str,
        reply: ServerHostReply,
    ) -> Result<(), ModuleCallError> {
        self.client
            .call(
                MODULE,
                names::methods::SERVER_COMPLETE,
                (session.to_owned(), callback.to_owned(), reply),
            )
            .await
    }

    async fn cancel(&self, operation: ServerOperationRef) -> Result<(), ModuleCallError> {
        self.client
            .call(MODULE, names::methods::SERVER_CANCEL, (operation,))
            .await
    }

    async fn close(&self, session: &str) -> Result<(), ModuleCallError> {
        self.client
            .call(MODULE, names::methods::SERVER_CLOSE, (session.to_owned(),))
            .await
    }

    async fn http_session(
        &self,
        request: ServerHttpSessionRequest,
    ) -> Result<ServerHttpResponseMetadata, ModuleCallError> {
        self.client
            .call(MODULE, names::methods::SERVER_HTTP_SESSION, (request,))
            .await
    }

    async fn metadata(
        &self,
        session: &str,
    ) -> Result<tinymcp_bus::ServerSessionMetadata, ModuleCallError> {
        self.client
            .call(
                MODULE,
                names::methods::SERVER_SESSION_METADATA,
                (session.to_owned(),),
            )
            .await
    }

    async fn shutdown(&self) -> Result<usize, ModuleCallError> {
        self.client
            .call(MODULE, names::methods::SERVER_SHUTDOWN, ())
            .await
    }
}

pub(super) struct ServerSession {
    port: Arc<dyn ServerPort>,
    id: String,
    next_operation: AtomicU64,
}

impl ServerSession {
    pub(super) fn port_for_config(config: &Config) -> Arc<dyn ServerPort> {
        Arc::new(TinyMcpPort {
            client: ModuleClient::new(config.clone()),
        })
    }

    pub(super) async fn open(config: &Config) -> Result<Self, ServerError> {
        Self::open_with_port(Self::port_for_config(config)).await
    }

    async fn open_with_port(port: Arc<dyn ServerPort>) -> Result<Self, ServerError> {
        let (info, source_type_prefix, resources) = handler::declarations();
        let sequence = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed).max(1);
        let id = port
            .open(ServerSessionConfig {
                session_id: Uuid::from_u128(u128::from(sequence)).to_string(),
                info,
                source_type_prefix,
                resources,
            })
            .await
            .map_err(ServerError::Module)?;
        Ok(Self {
            port,
            id,
            next_operation: AtomicU64::new(1),
        })
    }

    pub(super) async fn open_http(
        port: Arc<dyn ServerPort>,
        requested_session_id: Option<String>,
        known_session: bool,
        expected_protocol_version: Option<String>,
    ) -> Result<Self, ServerError> {
        let (info, source_type_prefix, resources) = handler::declarations();
        let sequence = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed).max(1);
        let id = port
            .open_http(ServerHttpSessionConfig {
                server: ServerSessionConfig {
                    session_id: Uuid::from_u128(u128::from(sequence)).to_string(),
                    info,
                    source_type_prefix,
                    resources,
                },
                requested_session_id,
                known_session,
                expected_protocol_version,
            })
            .await
            .map_err(ServerError::Module)?;
        Ok(Self {
            port,
            id,
            next_operation: AtomicU64::new(1),
        })
    }

    pub(super) fn id(&self) -> &str {
        &self.id
    }

    pub(super) fn port(&self) -> Arc<dyn ServerPort> {
        Arc::clone(&self.port)
    }

    pub(super) async fn submit(
        &self,
        headers: Map<String, Value>,
        line: String,
        cancellation: &CancellationToken,
    ) -> Result<ServerOperationSnapshot, ServerError> {
        let operation_id = Uuid::from_u128(u128::from(
            self.next_operation.fetch_add(1, Ordering::Relaxed).max(1),
        ))
        .to_string();
        self.port
            .submit(
                &self.id,
                headers,
                ServerInput {
                    operation_id: operation_id.clone(),
                    line,
                },
            )
            .await
            .map_err(ServerError::Module)?;

        loop {
            let snapshot = tokio::select! {
                _ = cancellation.cancelled() => {
                    self.port.cancel(ServerOperationRef {
                        session_id: self.id.clone(),
                        operation_id,
                    }).await.map_err(ServerError::Module)?;
                    return Err(ServerError::Cancelled);
                }
                result = self.port.poll(&self.id) => result.map_err(ServerError::Module)?,
            };
            match snapshot.state.clone() {
                ServerOperationState::Pending => tokio::time::sleep(POLL_INTERVAL).await,
                ServerOperationState::Callback { callback } => {
                    let reply = handler::dispatch(callback.call).await;
                    self.port
                        .complete(&self.id, &callback.id, reply)
                        .await
                        .map_err(ServerError::Module)?;
                }
                ServerOperationState::Complete { .. } => return Ok(snapshot),
                ServerOperationState::Cancelled => return Err(ServerError::Cancelled),
                ServerOperationState::Failed { message } => {
                    return Err(ServerError::Failed(message));
                }
                ServerOperationState::Unknown => return Err(ServerError::UnknownState),
            }
        }
    }

    pub(super) async fn close(&self) -> Result<(), ServerError> {
        self.port
            .close(&self.id)
            .await
            .map_err(ServerError::Module)
    }

    pub(super) async fn http_session(
        &self,
        request: ServerHttpSessionRequest,
    ) -> Result<ServerHttpResponseMetadata, ServerError> {
        self.port
            .http_session(request)
            .await
            .map_err(ServerError::Module)
    }

    pub(super) async fn metadata(
        &self,
    ) -> Result<tinymcp_bus::ServerSessionMetadata, ServerError> {
        self.port
            .metadata(&self.id)
            .await
            .map_err(ServerError::Module)
    }

    pub(super) async fn shutdown(&self) -> Result<usize, ServerError> {
        self.port.shutdown().await.map_err(ServerError::Module)
    }
}

pub(super) fn headers_object(headers: impl Iterator<Item = (String, String)>) -> Map<String, Value> {
    headers
        .map(|(name, value)| (name, Value::String(value)))
        .collect()
}

pub(super) fn session_config(
    session_id: String,
    info: Value,
    source_type_prefix: String,
    resources: Vec<Value>,
) -> ServerSessionConfig {
    ServerSessionConfig {
        session_id,
        info,
        source_type_prefix,
        resources,
    }
}

pub(super) fn module_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

#[cfg(test)]
#[path = "module_tests.rs"]
mod tests;
