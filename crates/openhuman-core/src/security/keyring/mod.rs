//! User-scoped secret storage. Hosts may install a backend before boot.
//! Otherwise an explicit environment override wins, followed by encrypted-file
//! storage with an environment, host-provided, or workspace-file master key.

mod adapter;
pub mod backend;
pub mod crypto;
pub mod encrypted_file_backend;
pub mod encrypted_store;
pub mod error;
pub mod file_store;
mod install;
pub mod ops;
pub mod store;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use backend::KeyringBackend;
pub use encrypted_file_backend::init_master_key;
pub use encrypted_file_backend::key_source::{
    clear_master_key_source, install_master_key_source, installed_master_key_source,
    MasterKeySource,
};
pub use encrypted_store::SecretStore;
pub use error::KeyringError;
pub use install::{clear_keyring_backend, install_keyring_backend, installed_keyring_backend};
pub use ops::{
    backend_name, delete, get, get_or_create_random, is_available, migrate_from_file,
    reset_availability_cache, set, MigrationOutcome,
};
pub use store::init_workspace;

#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use ops::force_backend_for_test;

#[cfg(test)]
#[path = "keyring_tests.rs"]
mod tests;
