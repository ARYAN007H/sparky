use super::{effective_backend_kind_for, BackendKind};

#[test]
fn explicit_file_backend_wins_over_staging_environment() {
    assert_eq!(
        effective_backend_kind_for(Some("staging"), Some("file"), false),
        BackendKind::File
    );
}

#[test]
fn explicit_encrypted_file_backend_wins_in_dev_environment() {
    assert_eq!(
        effective_backend_kind_for(Some("development"), Some("encrypted_file"), false),
        BackendKind::EncryptedFile
    );
}

#[test]
fn staging_defaults_to_encrypted_file_without_override() {
    assert_eq!(
        effective_backend_kind_for(Some(" staging "), None, false),
        BackendKind::EncryptedFile
    );
}

#[test]
fn unknown_backend_override_falls_back_to_environment_default() {
    assert_eq!(
        effective_backend_kind_for(Some("production"), Some("bogus"), false),
        BackendKind::EncryptedFile
    );
}

#[test]
fn unset_app_environment_defaults_to_encrypted_file() {
    assert_eq!(
        effective_backend_kind_for(None, None, false),
        BackendKind::EncryptedFile
    );
}

#[test]
fn development_environments_also_default_to_encrypted_file() {
    for app_env in ["dev", "development", " dev "] {
        assert_eq!(
            effective_backend_kind_for(Some(app_env), None, false),
            BackendKind::EncryptedFile,
            "{app_env} must not implicitly select plaintext storage"
        );
    }

    for app_env in ["", "production", "staging", "unknown"] {
        assert_eq!(
            effective_backend_kind_for(Some(app_env), None, false),
            BackendKind::EncryptedFile,
            "{app_env} must not silently select plaintext storage"
        );
    }
}

#[test]
fn headless_and_test_hosts_default_to_encrypted_storage() {
    for app_env in [None, Some("development"), Some("dev")] {
        for test_build in [false, true] {
            assert_eq!(
                effective_backend_kind_for(app_env, None, test_build),
                BackendKind::EncryptedFile,
                "a host without an installed backend must use encrypted storage"
            );
        }
    }
}

#[test]
fn installed_backend_takes_precedence_and_is_thread_local() {
    use crate::security::keyring::KeyringBackend;
    use crate::security::keyring::{self, backend::MockBackend};
    use std::sync::Arc;
    let _env = crate::config::test_env::EnvVarGuard::locked().without("OPENHUMAN_KEYRING_BACKEND");
    let dir = tempfile::tempdir().unwrap();
    let _workspace = super::test_scope::ScopedWorkspace::new(dir.path());
    let installed = Arc::new(MockBackend::new());
    keyring::install_keyring_backend(installed.clone());
    let backend = super::build_backend_at(dir.path());
    backend.set("user:key", "host-value").unwrap();
    assert_eq!(
        installed.get("user:key").unwrap().as_deref(),
        Some("host-value")
    );
    assert_eq!(keyring::backend_name(), "mock");
    assert!(keyring::is_available());
    assert!(
        !dir.path().join("keyring/master.key").exists(),
        "probing a host backend must not initialize a file key"
    );
    assert!(
        std::thread::spawn(|| keyring::installed_keyring_backend().is_none())
            .join()
            .unwrap()
    );
    keyring::clear_keyring_backend();
    assert_eq!(super::build_backend_at(dir.path()).name(), "encrypted_file");
}

#[test]
fn explicit_os_backend_requires_a_host_provider() {
    let _env =
        crate::config::test_env::EnvVarGuard::locked().with("OPENHUMAN_KEYRING_BACKEND", "os");
    let dir = tempfile::tempdir().unwrap();
    assert!(super::validate_backend_selection()
        .unwrap_err()
        .contains("host-installed OS"));
    let backend = super::build_backend_at(dir.path());
    assert!(backend
        .get("user:key")
        .unwrap_err()
        .to_string()
        .contains("host-installed OS"));
    assert!(backend.set("user:key", "value").is_err());
    assert!(backend.delete("user:key").is_err());
}
