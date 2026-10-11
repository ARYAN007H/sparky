//! Host install slot, process-wide in production and thread-local in unit tests.

use super::backend::KeyringBackend;
use std::sync::{Arc, RwLock};

#[cfg(not(test))]
fn slot() -> &'static RwLock<Option<Arc<dyn KeyringBackend>>> {
    static SLOT: std::sync::OnceLock<RwLock<Option<Arc<dyn KeyringBackend>>>> =
        std::sync::OnceLock::new();
    SLOT.get_or_init(|| RwLock::new(None))
}

#[cfg(test)]
fn slot() -> &'static RwLock<Option<Arc<dyn KeyringBackend>>> {
    thread_local! {
        static SLOT: &'static RwLock<Option<Arc<dyn KeyringBackend>>> =
            Box::leak(Box::new(RwLock::new(None)));
    }
    SLOT.with(|slot| *slot)
}

/// Install a host provider before core boot, replacing the previous provider.
pub fn install_keyring_backend(provider: Arc<dyn KeyringBackend>) {
    *slot().write().unwrap_or_else(|e| e.into_inner()) = Some(provider);
    super::reset_availability_cache();
}

/// The provider installed by the host, if any.
pub fn installed_keyring_backend() -> Option<Arc<dyn KeyringBackend>> {
    slot().read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Remove the installed provider. Call only while the core is stopped.
pub fn clear_keyring_backend() {
    *slot().write().unwrap_or_else(|e| e.into_inner()) = None;
    super::reset_availability_cache();
}
