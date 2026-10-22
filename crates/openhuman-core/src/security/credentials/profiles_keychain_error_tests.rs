//! A keychain read error is not a missing secret. Treating it as one dropped
//! keychain-only OAuth profiles and persisted the drop, and every write
//! rebuilds the whole store (keychain entries included) from that partial view.

use super::*;
use crate::security::secret_storage::{SecretBackend, SecretStorageError};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

/// In-memory keychain whose reads can be made to fail (a locked or denied keychain).
#[derive(Default)]
struct FlakyKeychain {
    map: Mutex<HashMap<String, String>>,
    fail_reads: AtomicBool,
}

impl SecretBackend for FlakyKeychain {
    fn get(&self, key: &str) -> Result<Option<String>, SecretStorageError> {
        if self.fail_reads.load(Ordering::SeqCst) {
            return Err(SecretStorageError::Backend("keychain locked".into()));
        }
        Ok(self.map.lock().unwrap().get(key).cloned())
    }
    fn set(&self, key: &str, value: &str) -> Result<(), SecretStorageError> {
        self.map.lock().unwrap().insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, key: &str) -> Result<(), SecretStorageError> {
        self.map.lock().unwrap().remove(key);
        Ok(())
    }
    fn name(&self) -> &'static str {
        "flaky-keychain"
    }
}

struct Fixture {
    keychain: Arc<FlakyKeychain>,
    store: AuthProfilesStore,
    oauth_id: String,
    _tmp: TempDir,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        crate::security::secret_storage::clear_secret_backend();
    }
}

/// A store holding one keychain-only OAuth profile and one Token profile,
/// with keychain reads failing from here on.
fn locked_keychain_store() -> Fixture {
    let keychain = Arc::new(FlakyKeychain::default());
    crate::security::secret_storage::install_secret_backend(keychain.clone());
    let tmp = TempDir::new().unwrap();
    let store = AuthProfilesStore::new(tmp.path(), true);
    let oauth = AuthProfile::new_oauth(
        "openai-codex",
        "default",
        TokenSet {
            access_token: "kc-access".into(),
            refresh_token: Some("kc-refresh".into()),
            id_token: None,
            expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
            token_type: Some("Bearer".into()),
            scope: None,
        },
    );
    let oauth_id = oauth.id.clone();
    store.upsert_profile(oauth, true).unwrap();
    store
        .upsert_profile(
            AuthProfile::new_token("anthropic", "default", "kc-token".into()),
            true,
        )
        .unwrap();
    let json = std::fs::read_to_string(store.path()).unwrap();
    assert!(
        json.contains(&oauth_id) && !json.contains("kc-access") && !json.contains("kc-token"),
        "precondition: secrets live only in the keychain"
    );
    keychain.fail_reads.store(true, Ordering::SeqCst);
    Fixture {
        keychain,
        store,
        oauth_id,
        _tmp: tmp,
    }
}

#[test]
fn keychain_read_error_fails_the_load_and_keeps_the_profile() {
    let f = locked_keychain_store();
    let json_before = std::fs::read_to_string(f.store.path()).unwrap();

    assert!(
        f.store.load().is_err(),
        "a keychain read error must fail the load"
    );

    assert_eq!(
        std::fs::read_to_string(f.store.path()).unwrap(),
        json_before
    );
    f.keychain.fail_reads.store(false, Ordering::SeqCst);
    let data = f.store.load().expect("load after the keychain unlocks");
    assert!(
        data.profiles.contains_key(&f.oauth_id),
        "profile was purged"
    );
}

#[test]
fn keychain_read_error_refuses_writes_and_keeps_every_secret() {
    let f = locked_keychain_store();
    let json_before = std::fs::read_to_string(f.store.path()).unwrap();
    let keychain_before = f.keychain.map.lock().unwrap().clone();

    let upsert = f.store.upsert_profile(
        AuthProfile::new_token("openrouter", "default", "new-token".into()),
        false,
    );

    assert!(
        upsert.is_err(),
        "a write from a partial view must be refused"
    );
    assert_eq!(
        std::fs::read_to_string(f.store.path()).unwrap(),
        json_before
    );
    assert_eq!(
        *f.keychain.map.lock().unwrap(),
        keychain_before,
        "keychain entries were rewritten from a partial view"
    );
}
