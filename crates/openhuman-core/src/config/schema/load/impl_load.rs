use super::super::Config;
use super::branches::{default_config_boxed, pre_login_config_boxed};
use super::dirs::{
    default_action_dir, default_config_and_workspace_dirs, resolve_action_dir,
    resolve_config_dirs_ignoring_env, resolve_runtime_config_dirs_with, ConfigResolutionSource,
};
use super::env::{EnvLookup, ProcessEnv, ProcessEnvWithoutWorkspace};
use super::migrate::{
    migrate_cloud_provider_slugs, migrate_components, migrate_legacy_inference_url,
    migrate_legacy_memory_backend, migrate_search_settings,
};
use super::secrets::{
    decrypt_config_secrets, decrypt_config_secrets_strict, encrypt_config_secrets,
};
use anyhow::{Context, Result};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use tokio::fs;

/// Guards the "corrupted config read, resetting to defaults" warning so it
/// fires at most once per process lifetime. Without this, a permanently
/// non-UTF-8 config file floods telemetry with hundreds of identical
/// `stream did not contain valid UTF-8` events (#5167).
static WARNED_CONFIG_READ_FAILURE: OnceLock<Mutex<bool>> = OnceLock::new();

/// Try to read `config_path`. On content corruption (non-UTF-8 bytes), rename
/// the corrupted file to `<config_file>.corrupted.<timestamp>`, try the `.bak`
/// backup, and if that also fails return an empty string so the caller's
/// `parse_config_with_recovery` falls through to defaults.
///
/// Only triggers auto-recovery for **content** corruption (`InvalidData`), not
/// for transient or permission errors (`PermissionDenied`, `NotFound`, etc.),
/// which are propagated as errors so the caller can surface them to the user.
///
/// Rate-limits the warning to at most one per process lifetime so a
/// permanently corrupted file does not flood telemetry (#5167).
pub(super) async fn read_config_with_recovery_or_default(
    config_path: &Path,
) -> Result<(String, bool)> {
    let reads = || async {
        match fs::read_to_string(config_path).await {
            Ok(contents) => Ok(contents),
            Err(error) => {
                let ownership = describe_config_ownership(config_path).await;
                Err(anyhow::Error::new(error).context(format!(
                    "Failed to read config file: {}{ownership}",
                    config_path.display()
                )))
            }
        }
    };

    let result = crate::util::retry_with_backoff_async("read config file", 5, 20, reads).await;
    match result {
        Ok(contents) => Ok((contents, false)),
        Err(e) => {
            // Check if this is a content-corruption error (non-UTF-8).
            // Only in that case do we auto-recover by renaming the file
            // and falling back to backup/defaults. Other errors (permission
            // denied, file not found after retries, etc.) are propagated
            // so the caller surfaces them to the user.
            let is_content_corruption = e.chain().any(|cause| {
                cause
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|ioe| ioe.kind() == std::io::ErrorKind::InvalidData)
            });

            if !is_content_corruption {
                tracing::warn!(
                    path = %config_path.display(),
                    error = %format!("{e:#}"),
                    "[config] failed to read config file"
                );
                return Err(e);
            }

            // Rate-limit the warning to once per process lifetime.  The
            // MutexGuard *must* be scoped in its own block so it is dropped
            // before any `.await` below -- holding a non-Send guard across
            // an await would poison the future's Send bound (#5167).
            {
                let warned = WARNED_CONFIG_READ_FAILURE.get_or_init(|| Mutex::new(false));
                let mut guard = warned.lock().unwrap_or_else(|e| e.into_inner());
                if !*guard {
                    tracing::warn!(
                        path = %config_path.display(),
                        error = %e,
                        "[config] Config file contains non-UTF-8 content; \
                         renaming to .corrupted and attempting recovery from backup"
                    );
                    *guard = true;
                }
            }

            // Rename the corrupted file with a UNIX-timestamp suffix so it's
            // recoverable by the user but does not block future config loads.
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let stem = config_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("config");
            let corrupted_name = format!("{stem}.corrupted.{ts}");
            let corrupted_path = config_path.with_file_name(&corrupted_name);
            if let Err(rename_err) = std::fs::rename(config_path, &corrupted_path) {
                tracing::warn!(
                    src = %config_path.display(),
                    dst = %corrupted_path.display(),
                    error = %rename_err,
                    "[config] Failed to rename corrupted config file; \
                     subsequent loads will fail again"
                );
            }

            // Try the backup.
            let backup_path = config_path.with_extension("toml.bak");
            match fs::read_to_string(&backup_path).await {
                Ok(bak_contents) => {
                    tracing::warn!(
                        path = %config_path.display(),
                        backup = %backup_path.display(),
                        "[config] Read of config file failed; recovered from backup"
                    );
                    Ok((bak_contents, true))
                }
                Err(bak_err) => {
                    tracing::warn!(
                        path = %config_path.display(),
                        backup = %backup_path.display(),
                        error = %bak_err,
                        "[config] Backup also unreadable after failed config read; \
                         resetting to defaults"
                    );
                    Ok((String::new(), true))
                }
            }
        }
    }
}

/// Parse the text a [`ConfigSource`](super::source::ConfigSource) returned.
