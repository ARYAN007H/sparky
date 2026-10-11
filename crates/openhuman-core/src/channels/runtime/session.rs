//! Host-owned logout generation and cancellation for channel orchestration.
use std::future::Future;
use std::sync::{LazyLock, Mutex};
use tokio_util::sync::CancellationToken;

static SESSION: LazyLock<Mutex<CancellationToken>> =
    LazyLock::new(|| Mutex::new(CancellationToken::new()));

pub(crate) fn channel_session() -> CancellationToken {
    SESSION
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

pub(crate) fn invalidate_channel_session() {
    let mut current = SESSION.lock().unwrap_or_else(|error| error.into_inner());
    current.cancel();
    *current = CancellationToken::new();
}

/// Stop the host dispatch generation on logout, even with independent model auth.
/// Module resource owners explicitly drain their handles when dispatch ends.
pub(super) async fn run_in_session(
    session: CancellationToken,
    runtime: impl Future<Output = anyhow::Result<()>>,
) -> anyhow::Result<()> {
    tokio::select! {
        biased;
        _ = session.cancelled() => Ok(()),
        result = runtime => result,
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
