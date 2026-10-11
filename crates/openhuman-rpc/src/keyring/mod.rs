//! OS keychain providers installed by desktop, CLI and TUI hosts before boot.
//! Loading the RPC library or serving a SaaS gateway does not install them.

mod backend;
mod master_key;
pub use backend::OsKeyringBackend;
pub use master_key::OsMasterKeySource;

/// Preserve encrypted-file storage by default, with its master key in the OS
/// keychain. The explicit `os` override installs the native secret backend;
/// explicit file backends retain their meaning in host boot as well.
pub fn install() {
    use crate::core_host::security::keyring::{
        install_keyring_backend, install_master_key_source, installed_keyring_backend,
        installed_master_key_source,
    };
    use std::sync::Arc;
    if installed_master_key_source().is_none() {
        install_master_key_source(Arc::new(OsMasterKeySource::default()));
    }
    if installed_keyring_backend().is_none()
        && std::env::var("OPENHUMAN_KEYRING_BACKEND")
            .is_ok_and(|v| v.trim().eq_ignore_ascii_case("os"))
    {
        install_keyring_backend(Arc::new(OsKeyringBackend::new()));
    }
}
