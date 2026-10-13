//! What the module asks of the host while it works on a remix: one
//! completion, one Jev evaluation, one web search, and the learned slang and
//! meme index documents.
//!
//! The contract carries no correlation id, so a callback cannot say which
//! remix it belongs to. Remixes therefore run one at a time per process: the
//! [`Session`] of the running remix is the one callbacks answer for, and a
//! callback arriving with no session is refused as unsupported.
//!
//! Index documents live at `<workspace>/tinymemes/{slang,meme}-index.json`,
//! the files the in-process engine used, so learned state carries over.

use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::de::DeserializeOwned;
use serde_json::Value;
use tinyinference_decisions::{Client, EvaluationRequest};
use tinymemes_bus::{
    host_methods, CompleteRequest, EvaluateEnvelope, HostFailure, HostResult, IndexKind,
    LoadIndexRequest, SaveIndexRequest, SearchRequest,
};

use super::{inference, jev, remixed, search};
use crate::config::Config;
use crate::security::secret_storage::file_store::write_atomic;

const SLANG_FILE: &str = "slang-index.json";
const MEMES_FILE: &str = "meme-index.json";

/// The workspace one remix runs for.
pub(super) struct Session {
    config: Config,
    jev: tokio::sync::OnceCell<Option<Arc<Client>>>,
}

impl Session {
    pub(super) fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            jev: tokio::sync::OnceCell::new(),
        }
    }

    fn index_path(&self, kind: IndexKind) -> PathBuf {
        let file = match kind {
            IndexKind::Slang => SLANG_FILE,
            IndexKind::Memes => MEMES_FILE,
        };
        self.config.workspace_dir.join(remixed::DIR).join(file)
    }
}

static ACTIVE: Mutex<Option<Arc<Session>>> = Mutex::new(None);
static GATE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Clears the active session when the remix ends, including when the future
/// is dropped by a timeout.
struct ActiveGuard;

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        *ACTIVE.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

/// Run `work` as the one active remix, answering its callbacks for `config`.
pub(super) async fn run_with<T>(config: &Config, work: impl Future<Output = T>) -> T {
    let _turn = GATE.lock().await;
    *ACTIVE.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(Session::new(config)));
    let _clear = ActiveGuard;
    work.await
}

/// The session callbacks currently answer for, if a remix is running.
pub(super) fn active() -> Option<Arc<Session>> {
    ACTIVE.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn args<T: DeserializeOwned>(value: Value) -> Result<T, tinybus::Error> {
    let (request,): (T,) = serde_json::from_value(value)
        .map_err(|_| tinybus::Error::failed("invalid tinymemes host request"))?;
    Ok(request)
}

fn reply<T: serde::Serialize>(result: HostResult<T>) -> Result<Value, tinybus::Error> {
    serde_json::to_value(result)
        .map_err(|_| tinybus::Error::failed("invalid tinymemes host response"))
}

/// Answer one host callback for `session`.
pub(super) async fn dispatch(
    session: &Session,
    member: &str,
    payload: Value,
) -> Result<Value, tinybus::Error> {
    log::debug!("[tinymemes] host callback member={member}");
    match member {
        host_methods::COMPLETE => reply(complete(session, args(payload)?).await),
        host_methods::EVALUATE => reply(evaluate(session, args(payload)?).await),
        host_methods::SEARCH => reply(search(session, args(payload)?).await),
        host_methods::LOAD_INDEX => reply(load_index(session, args(payload)?).await),
        host_methods::SAVE_INDEX => reply(save_index(session, args(payload)?).await),
        // The remixed text is returned to the caller, which delivers it
        // itself; nothing is handed over through the module.
        host_methods::DELIVER => reply(HostResult::Ok(())),
        _ => Err(tinybus::Error::failed("unknown tinymemes host operation")),
    }
}

async fn complete(session: &Session, request: CompleteRequest) -> HostResult<String> {
    inference::complete(&session.config, &request.system, &request.user)
        .await
        .map_err(|error| {
            log::warn!("[tinymemes] complete callback failed: {error}");
            HostFailure::Backend
        })
}

async fn evaluate(session: &Session, envelope: EvaluateEnvelope) -> HostResult<EvaluateEnvelope> {
    let Some(client) = session
        .jev
        .get_or_init(|| async { jev::managed(&session.config) })
        .await
    else {
        log::debug!("[tinymemes] evaluate callback: managed jev unavailable");
        return Err(HostFailure::Unsupported);
    };
    let request: EvaluationRequest =
        serde_json::from_value(envelope.body).map_err(|_| HostFailure::Unsupported)?;
    let result = client.evaluate(&request).await.map_err(|error| {
        log::warn!("[tinymemes] evaluate callback failed: {error}");
        HostFailure::Backend
    })?;
    let body = serde_json::to_value(result.response).map_err(|_| HostFailure::Backend)?;
    Ok(EvaluateEnvelope { body })
}

async fn search(
    session: &Session,
    request: SearchRequest,
) -> HostResult<Vec<tinymemes_bus::SearchResult>> {
    let Some(backend) = search::OpenHumanSearch::available(&session.config) else {
        return Err(HostFailure::Unsupported);
    };
    backend
        .search(&request.query, request.max_results as usize)
        .await
        .map_err(|error| {
            // The query is derived from the chat, so only the error is logged.
            log::warn!("[tinymemes] search callback failed: {error}");
            HostFailure::Backend
        })
}

async fn load_index(session: &Session, request: LoadIndexRequest) -> HostResult<Option<String>> {
    let path = session.index_path(request.kind);
    match crate::core::runtime::spawn_blocking_scoped(move || std::fs::read_to_string(path)).await {
        Ok(Ok(json)) => Ok(Some(json)),
        Ok(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Ok(Err(error)) => {
            log::warn!("[tinymemes] cannot read index: {error}");
            Err(HostFailure::Backend)
        }
        Err(error) => {
            log::warn!("[tinymemes] index read task failed: {error}");
            Err(HostFailure::Backend)
        }
    }
}

async fn save_index(session: &Session, request: SaveIndexRequest) -> HostResult<()> {
    let path = session.index_path(request.kind);
    let json = request.json;
    match crate::core::runtime::spawn_blocking_scoped(move || write_atomic(&path, json.as_bytes()))
        .await
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => {
            log::warn!("[tinymemes] cannot save index: {error}");
            Err(HostFailure::Backend)
        }
        Err(error) => {
            log::warn!("[tinymemes] index write task failed: {error}");
            Err(HostFailure::Backend)
        }
    }
}

#[cfg(test)]
#[path = "callbacks_tests.rs"]
mod tests;
