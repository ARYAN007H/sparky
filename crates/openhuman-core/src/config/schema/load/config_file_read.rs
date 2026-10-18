use anyhow::Result;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use tokio::fs;

use super::super::Config;
use super::branches::default_config_boxed;

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

pub(crate) async fn parse_config_with_recovery(
    config_path: &Path,
    contents: &str,
) -> (Config, bool) {
    let (config, recovered) = Box::pin(parse_config_boxed(config_path, contents)).await;
    (*config, recovered)
}

/// [`parse_config_with_recovery`] with the config kept behind a `Box`, so the
/// async chain that threads it through (load, recover, migrate, save) moves a
/// pointer instead of an ~8 KB value per hop.
pub(super) async fn parse_config_boxed(config_path: &Path, contents: &str) -> (Box<Config>, bool) {
    let parse_err = match parse_toml_off_worker(contents.to_string()).await {
        Ok(config) => {
            tracing::debug!(
                path = %config_path.display(),
                "[config] Config parsed successfully"
            );
            return (config, false);
        }
        Err(parse_err) => parse_err,
    };

    let backup_path = config_path.with_extension("toml.bak");
    if tokio::fs::try_exists(&backup_path).await.unwrap_or(false) {
        tracing::warn!(
            path = %config_path.display(),
            backup = %backup_path.display(),
            error = %parse_err,
            "[config] Config file is corrupted — attempting recovery from backup"
        );
        match fs::read_to_string(&backup_path).await {
            Ok(bak_contents) => match parse_toml_off_worker(bak_contents).await {
                Ok(bak_config) => {
                    tracing::info!(
                        path = %config_path.display(),
                        backup = %backup_path.display(),
                        "[config] Recovered config from backup"
                    );
                    return (bak_config, true);
                }
                Err(bak_err) => {
                    tracing::warn!(
                        path = %config_path.display(),
                        backup = %backup_path.display(),
                        error = %bak_err,
                        "[config] Backup is also corrupted; resetting to defaults"
                    );
                }
            },
            Err(read_err) => {
                tracing::warn!(
                    path = %config_path.display(),
                    backup = %backup_path.display(),
                    error = %read_err,
                    "[config] Failed to read backup; resetting to defaults"
                );
            }
        }
    } else {
        tracing::warn!(
            path = %config_path.display(),
            error = %parse_err,
            "[config] Config file is corrupted (no backup found); resetting to defaults"
        );
    }

    (default_config_boxed(), true)
}

/// Return the source text that should be inspected by migrations. A readable
/// backup may be the source used to build the config when the primary TOML is
/// malformed.
pub(super) async fn migration_source(config_path: &Path, contents: &str) -> String {
    if parse_toml_off_worker(contents.to_owned()).await.is_ok() {
        return contents.to_owned();
    }
    let backup_path = config_path.with_extension("toml.bak");
    match fs::read_to_string(backup_path).await {
        Ok(backup) if parse_toml_off_worker(backup.clone()).await.is_ok() => backup,
        _ => contents.to_owned(),
    }
}

pub(super) async fn parse_toml_off_worker(contents: String) -> Result<Box<Config>, String> {
    match tokio::task::spawn_blocking(move || {
        super::parse::config_from_toml_str(&contents).map(Box::new)
    })
    .await
    {
        Ok(Ok(config)) => Ok(config),
        Ok(Err(parse_err)) => Err(parse_err.to_string()),
        Err(join_err) => Err(format!("blocking-pool parse join failed: {join_err}")),
    }
}

/// Marker appended to a config-read failure when the file is owned by a
/// different uid than the process trying to read it.
///
/// `core::observability::expected_error_kind` keys on this to keep the failure
/// paging instead of demoting it as an unpreventable user-environment denial:
/// a uid mismatch on a file *we* created with mode 0600 is an OpenHuman defect
/// (typically a container whose entrypoint chowned only the workspace
/// directory, leaving a stale-uid `config.toml` inside it), not an ACL the user
/// has to fix for us.
pub(crate) const CONFIG_OWNER_MISMATCH_MARKER: &str = "[config owner mismatch]";

/// Numeric ownership/permission facts about the config file, appended to the
/// read-failure context.
///
/// An `EACCES` on a file whose `exists()` check just succeeded is fully
/// explained by four numbers we can always obtain: the file's uid/gid, its
/// mode, and the process's effective uid/gid. Without them the operator sees
/// only "Permission denied (os error 13)" and cannot tell a foreign-owner
/// container volume (our bug) from a genuine ACL / antivirus denial (theirs).
///
/// Numeric ids and mode bits are not PII, so this is safe both in the local log
/// and in the error chain that reaches the client.
#[cfg(unix)]
async fn describe_config_ownership(path: &Path) -> String {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let Ok(meta) = fs::metadata(path).await else {
        return String::new();
    };
    // SAFETY: `geteuid`/`getegid` take no arguments, mutate no process state,
    // and are documented as always succeeding.
    let (euid, egid) = unsafe { (libc::geteuid(), libc::getegid()) };
    let mismatch = if meta.uid() == euid {
        String::new()
    } else {
        format!("{CONFIG_OWNER_MISMATCH_MARKER} ")
    };
    format!(
        " {mismatch}(file uid={} gid={} mode={:04o}; process euid={euid} egid={egid})",
        meta.uid(),
        meta.gid(),
        meta.permissions().mode() & 0o777,
    )
}

#[cfg(not(unix))]
async fn describe_config_ownership(_path: &Path) -> String {
    String::new()
}
