use std::sync::Arc;

use super::CoreContext;

impl CoreContext {
    /// The embedder-supplied config for the current dispatch, if there is one.
    ///
    /// The read path for `config::ops::load_config_with_timeout`.
    pub fn current_embedder_config() -> Option<crate::config::Config> {
        Self::current().and_then(|ctx| ctx.latest_embedder_config())
    }

    pub(crate) fn latest_embedder_config(&self) -> Option<crate::config::Config> {
        let snapshot = self.latest_embedder_config.as_ref()?;
        let mut config = snapshot.lock().ok()?.clone();
        config.context_snapshot = Some(Arc::downgrade(snapshot));
        Some(config)
    }

    /// Read the embedder-supplied config for the current dispatch without
    /// cloning it. Prefer this over [`Self::current_embedder_config`] when only
    /// a field or a predicate is needed: a `Config` clone is a large copy.
    pub fn with_current_embedder_config<R>(
        f: impl FnOnce(&crate::config::Config) -> R,
    ) -> Option<R> {
        Self::current().and_then(|ctx| {
            let snapshot = ctx.latest_embedder_config.as_ref()?;
            let config = snapshot.lock().ok()?;
            Some(f(&config))
        })
    }
}
