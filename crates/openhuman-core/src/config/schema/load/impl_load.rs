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

pub(crate) use super::config_file_read::parse_config_with_recovery;
pub(super) use super::config_file_read::read_config_with_recovery_or_default;
pub(crate) use super::config_file_read::CONFIG_OWNER_MISMATCH_MARKER;
pub(super) use super::config_file_read::{
    migration_source, parse_config_boxed, parse_toml_off_worker,
};
pub(super) use super::parse_source::parse_source_contents;

impl Config {
    pub async fn load_or_init() -> Result<Self> {
        Self::load_or_init_unenriched()
            .await
            .map(crate::config::enrichers::enriched)
    }

    async fn load_or_init_unenriched() -> Result<Self> {
        if let Some(scoped) = super::saas_scope::saas_scoped_config() {
            return scoped;
        }
        let (default_openhuman_dir, default_workspace_dir) = default_config_and_workspace_dirs()?;
        Self::load_or_init_with_env_lookup(
            &default_openhuman_dir,
            &default_workspace_dir,
            &ProcessEnv,
        )
        .await
        .inspect(super::active_workspace::publish_loaded_workspace)
    }

    pub(crate) async fn load_or_init_with_env_lookup(
        default_openhuman_dir: &Path,
        default_workspace_dir: &Path,
        env: &(dyn EnvLookup + Send + Sync),
    ) -> Result<Self> {
        let (openhuman_dir, workspace_dir, resolution_source) = Box::pin(
            resolve_runtime_config_dirs_with(default_openhuman_dir, default_workspace_dir, env),
        )
        .await?;

        let config_path = openhuman_dir.join("config.toml");

        // With a shared backend the scope's document is the config even when
        // this node has no local file: returning pre-login defaults here would
        // let the next settings save overwrite that document.
        let source = super::source::for_config(&config_path)?;
        let document_exists = source.label() == "document" && source.exists().await?;
        if resolution_source == ConfigResolutionSource::DefaultConfigDir
            && !config_path.exists()
            && !document_exists
        {
            let config = pre_login_config_boxed(config_path.clone(), workspace_dir.clone(), env);

            tracing::debug!(
                path = %config.config_path.display(),
                workspace = %config.workspace_dir.display(),
                source = resolution_source.as_str(),
                initialized = false,
                persisted = false,
                "Config loaded (pre-login, in-memory only — no dirs or files written)"
            );
            return Ok(*config);
        }

        fs::create_dir_all(&openhuman_dir)
            .await
            .context("Failed to create config directory")?;
        fs::create_dir_all(&workspace_dir)
            .await
            .context("Failed to create workspace directory")?;

        // Each branch is its own boxed future. An unoptimised build gives every
        // `Config` temporary its own stack slot, so folding all three branches
        // into this one state machine made the poll frame ~440 KB and stacked
        // on top of the whole agent tower (#6379).
        // Which source holds the config is decided per call: the file unless a
        // shared backend and a scope are installed. A process boots before its
        // backend is installed, so its first load normally reads the file (the
        // bootstrap config); nothing here enforces that ordering.
        if document_exists || source.exists().await? {
            Box::pin(Self::load_existing_config(
                openhuman_dir,
                workspace_dir,
                config_path,
                resolution_source,
                env,
            ))
            .await
        } else {
            Box::pin(Self::init_new_config(
                workspace_dir,
                config_path,
                resolution_source,
                env,
            ))
            .await
        }
    }

    /// Load config from the default user paths, bypassing the
    /// `OPENHUMAN_WORKSPACE` environment variable.
    ///
    /// This is used by the debug dump to load the real user config
    /// for auth token resolution when the dump script overrides
    /// `OPENHUMAN_WORKSPACE` to a throwaway temp directory.
    pub async fn load_from_default_paths() -> Result<Self> {
        Self::load_from_default_paths_unenriched()
            .await
            .map(crate::config::enrichers::enriched)
    }

    async fn load_from_default_paths_unenriched() -> Result<Self> {
        let (default_openhuman_dir, default_workspace_dir) = default_config_and_workspace_dirs()?;
        let (openhuman_dir, workspace_dir, _source) =
            resolve_config_dirs_ignoring_env(&default_openhuman_dir, &default_workspace_dir)
                .await?;
        let config_path = openhuman_dir.join("config.toml");

        if !config_path.exists() {
            let mut config = Config {
                config_path,
                workspace_dir,
                action_dir: default_action_dir(),
                ..Default::default()
            };
            config.apply_env_overrides();
            return Ok(config);
        }

        // NOTE: no backup recovery here by design -- this is the debug-dump path only;
        // `load_or_init()` is the authoritative startup path that handles corruption.
        // However, we still use `read_config_with_recovery_or_default` to handle the
        // non-UTF-8 case: a corrupted file is renamed to `.corrupted.<ts>` so the next
        // authoritative load can create a fresh config.
        let (raw, _read_was_recovered) =
            Box::pin(read_config_with_recovery_or_default(&config_path)).await?;
        let (mut config, _was_corrupted) = parse_config_with_recovery(&config_path, &raw).await;
        config.config_path = config_path.clone();
        config.workspace_dir = workspace_dir;
        config.action_dir = resolve_action_dir(&config.action_dir_override);
        let migration_raw = migration_source(&config_path, &raw).await;
        migrate_legacy_memory_backend(&mut config, &migration_raw);
        migrate_components(&mut config, &migration_raw);
        config.apply_env_overrides();
        // Debug-dump path is read-only; ignore the migration signal (the
        // authoritative `load_or_init` path persists upgraded secrets).
        let _ = decrypt_config_secrets(&mut config, &openhuman_dir)?;
        Ok(config)
    }

    /// Reload a config from an already-resolved `config.toml` path.
    ///
    /// This is for long-lived runtime objects that hold a `Config`
    /// snapshot and need to observe updates written back to the same
    /// file. It deliberately bypasses only `OPENHUMAN_WORKSPACE`
    /// resolution: the caller has already been scoped to a user/workspace,
    /// and following the process-global workspace env var again can cross
    /// streams with unrelated tests or runtime tasks that temporarily
    /// repoint it. Other process env overrides still apply.
    pub async fn load_from_config_path(config_path: &Path, workspace_dir: &Path) -> Result<Self> {
        Self::load_from_config_path_unenriched(config_path, workspace_dir)
            .await
            .map(crate::config::enrichers::enriched)
    }

    async fn load_from_config_path_unenriched(
        config_path: &Path,
        workspace_dir: &Path,
    ) -> Result<Self> {
        let config_path = config_path.to_path_buf();
        let workspace_dir = workspace_dir.to_path_buf();

        // A snapshot reload reads through the config source: the file, or the
        // scope's config document on a shared backend (bootstrap tables still
        // come from the file). The first load of a process does not: see
        // `source`.
        let source = super::source::for_config(&config_path)?;
        if !source.exists().await? {
            let mut config = Config {
                config_path,
                workspace_dir,
                action_dir: default_action_dir(),
                ..Default::default()
            };
            config.apply_env_overrides_from(&ProcessEnvWithoutWorkspace);
            return Ok(config);
        }

        // See the `load_or_init` read branch: a directory at the config path is
        // corruption, not a transient read failure -- fail fast with distinct
        // wording so it pages instead of being demoted (#3962, Codex P2).
        if config_path.is_dir() {
            anyhow::bail!(
                "Config path is a directory, not a file: {}",
                config_path.display()
            );
        }

        let read = Box::pin(source.read()).await?;
        let (config, config_was_corrupted) = parse_source_contents(&read, &config_path).await?;
        let super::source::ConfigRead {
            contents: raw,
            recovered: read_was_recovered,
            ..
        } = read;
        let mut config = *config;
        let config_was_corrupted = config_was_corrupted || read_was_recovered;
        config.config_path = config_path.clone();
        config.workspace_dir = workspace_dir;
        config.action_dir = resolve_action_dir(&config.action_dir_override);
        config.recovered_from_corruption = config_was_corrupted;
        migrate_legacy_inference_url(&mut config);
        let migration_raw = migration_source(&config_path, &raw).await;
        migrate_legacy_memory_backend(&mut config, &migration_raw);
        migrate_components(&mut config, &migration_raw);
        migrate_cloud_provider_slugs(&mut config);
        migrate_search_settings(&mut config);
        config.apply_env_overrides_from(&ProcessEnvWithoutWorkspace);

        if config_was_corrupted {
            tracing::warn!(
                path = %config.config_path.display(),
                "[config] Snapshot reload recovered a corrupted config; skipping persistence"
            );
        }

        Box::pin(crate::config::migrations::run_pending(&mut config)).await;
        Ok(config)
    }

    pub async fn save(&self) -> Result<()> {
        // A thin shim: the real body's future is ~25 KB, and `save` is awaited
        // from ~15 places in the load and migration paths. An unoptimised build
        // gives each of those await sites its own copy of the callee's future in
        // the caller's poll frame (`migrations::run_pending` alone was ~350 KB),
        // so the shim keeps every such copy at pointer size (#6379).
        Box::pin(self.save_inner()).await
    }

    async fn save_inner(&self) -> Result<()> {
        // Where the text lives is the source's business: the file (atomic
        // replace with a `.bak`) or, on a shared backend, the config document.
        let source = super::source::for_config(&self.config_path)?;
        self.save_to(source.as_ref()).await
    }

    /// Save into `source`. The loader also uses it to rewrite the local
    /// bootstrap file when a document-backed load recovered it.
    pub(super) async fn save_to(&self, source: &dyn super::source::ConfigSource) -> Result<()> {
        let mut config_to_save = self.clone();
        super::super::cli_overrides::restore_persisted_inference_fields(&mut config_to_save);
        // A document source seals the whole body under the scope's data key;
        // the process-local field key would make it unreadable on another node.
        // A config seeded from the local file (or saved by a migration before
        // the load decrypted it) can still hold process-key `enc2:` fields, so
        // they are opened first: the document must never nest node-local
        // ciphertext.
        if source.encrypts_body() {
            let openhuman_dir = config_to_save
                .config_path
                .parent()
                .context("Config path must have a parent directory")?
                .to_path_buf();
            decrypt_config_secrets_strict(&mut config_to_save, &openhuman_dir)?;
        } else {
            encrypt_config_secrets(&mut config_to_save)?;
        }

        let toml_str =
            toml::to_string_pretty(&config_to_save).context("Failed to serialize config")?;

        tracing::debug!(source = source.label(), "[config] saving config");
        let expected = *config_to_save.document_version.lock().unwrap();
        let version = source.write_versioned(&toml_str, expected).await?;
        *self.document_version.lock().unwrap() = version;

        Ok(())
    }
}
