//! The jail OpenHuman grants stdio MCP servers.
//!
//! A stdio MCP server is third-party code that runs with the user's
//! credentials in its environment. The `tinymcp` contract
//! (`McpClientConfig::stdio_sandbox`, `docs/specs/stdio-sandbox.md` in the
//! module) starts every server that is not `operator_trusted` inside a
//! TinyBox jail built from this config, and refuses it when none is given.
//! Registry-installed and `mcp.json`-declared servers are never
//! operator-trusted, so the host must supply the jail; this module builds it
//! from the same policy the shell tool's native jail uses:
//!
//! - the workspace is a dedicated writable directory under the core's
//!   `workspace_dir`;
//! - read and write grants come from `[runtime.local_jail]` plus the
//!   TinyBox-resolved toolchain grants (`sandbox::local_grants`), each passed
//!   through the host's protected-path floor;
//! - the untrusted-code isolation floor stays on, except on the macOS desktop
//!   where Seatbelt folder confinement is the host's accepted jail
//!   (`sandbox::native_jail::jail_isolation`), and there the credential stores
//!   are denied by path because Seatbelt reads allow-by-default.

use std::path::{Path, PathBuf};

use tinymcp_bus::{StdioSandboxConfig, StdioSandboxEnforcement};

use crate::config::Config;
use crate::security::SecurityPolicy;

/// Directory under `workspace_dir` that is the stdio servers' writable root.
const WORKSPACE_DIR: &str = "mcp_stdio";

/// Builds the jail for `config`, or `None` when the host cannot offer one
/// (no TinyBox support in this build, or the workspace cannot be created).
/// `None` makes the module refuse stdio servers rather than start them
/// unconfined.
#[cfg(feature = "modules")]
pub(super) fn stdio_sandbox(config: &Config) -> Option<StdioSandboxConfig> {
    let workspace = match prepare_workspace(&config.workspace_dir) {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(%error, "[mcp] stdio sandbox workspace unavailable; stdio servers will be refused");
            return None;
        }
    };
    let saas = crate::core::runtime::is_saas();
    let home = crate::sandbox::local_grants::grant_home(saas, dirs::home_dir);
    let jail = &config.runtime.local_jail;
    let isolated = require_process_isolation(saas);
    let deny_read = if isolated {
        Vec::new()
    } else {
        credential_denials(config, home.as_deref(), &workspace)
    };
    Some(StdioSandboxConfig {
        workspace: path_string(&workspace),
        read_only: admit_all(&jail.extra_read_only, home.as_deref()),
        read_write: admit_all(&jail.extra_read_write, home.as_deref()),
        deny_read,
        // MCP servers commonly call remote APIs and `npx` starts children.
        allow_net: true,
        allow_subprocess: true,
        require_process_isolation: isolated,
        enforcement: enforcement(isolated),
    })
}

#[cfg(not(feature = "modules"))]
pub(super) fn stdio_sandbox(_config: &Config) -> Option<StdioSandboxConfig> {
    None
}

/// How long a resolved set of toolchain grants is reused. The toolchains an
/// MCP server needs (`npx`, `uvx`, ...) do not move between workspace opens, but
/// a runtime installed while the core runs should still be picked up soon.
#[cfg(feature = "modules")]
const GRANT_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(60);

/// Toolchain grants by [`grants_key`], each with the instant it was resolved.
#[cfg(feature = "modules")]
static GRANT_CACHE: std::sync::LazyLock<
    parking_lot::Mutex<std::collections::HashMap<String, (std::time::Instant, Grants)>>,
> = std::sync::LazyLock::new(Default::default);

/// The read-only and read-write mounts TinyBox resolved.
#[cfg(feature = "modules")]
type Grants = (Vec<PathBuf>, Vec<PathBuf>);

/// What the resolved grants depend on: the operator's jail configuration, the
/// home directory and the `PATH` the toolchains are found on. The workspace is
/// deliberately absent: it is not an input to the resolution.
#[cfg(feature = "modules")]
fn grants_key(config: &Config, home: Option<&Path>) -> String {
    format!(
        "{}|{}|{}",
        serde_json::to_string(&config.runtime.local_jail).unwrap_or_default(),
        home.map(path_string).unwrap_or_default(),
        std::env::var("PATH").unwrap_or_default()
    )
}

#[cfg(feature = "modules")]
fn cached_grants(key: &str) -> Option<Grants> {
    let cache = GRANT_CACHE.lock();
    let (resolved_at, grants) = cache.get(key)?;
    (resolved_at.elapsed() < GRANT_CACHE_TTL).then(|| grants.clone())
}

/// Adds the TinyBox-resolved toolchain grants (the runtimes `npx`, `uvx` and
/// friends live in) to `sandbox`. A resolution failure keeps the config
/// grants only: the jail never widens on error.
///
/// TinyMCP admits one lifecycle transition at a time and the host serializes
/// every workspace's open behind one gate (`module_client::LIFECYCLE_GATE`), so
/// whatever runs here is paid once per agent, in turn, while all the others
/// wait. Resolving through the TinyBox module on each open made N agents cost N
/// module calls inside that gate; the result is the same for all of them, so it
/// is resolved once per minute.
#[cfg(feature = "modules")]
pub(super) async fn with_toolchain_grants(config: &Config, sandbox: &mut StdioSandboxConfig) {
    use crate::sandbox::types::SandboxPolicy;
    let home =
        crate::sandbox::local_grants::grant_home(crate::core::runtime::is_saas(), dirs::home_dir);
    let key = grants_key(config, home.as_deref());
    if let Some((read_only, read_write)) = cached_grants(&key) {
        merge(&mut sandbox.read_only, &read_only);
        merge(&mut sandbox.read_write, &read_write);
        tracing::debug!("[mcp] stdio sandbox grants reused from the cache");
        return;
    }
    let seed = SandboxPolicy {
        backend: crate::sandbox::types::SandboxBackendKind::Local,
        workspace_root: PathBuf::from(&sandbox.workspace),
        state_dir: config.workspace_dir.clone(),
        read_only_mounts: Vec::new(),
        read_write_mounts: Vec::new(),
        allow_network: sandbox.allow_net,
        env_passthrough: Vec::new(),
        docker_overrides: None,
    };
    match crate::sandbox::local_grants::with_resolved_grants(config, &seed).await {
        Ok(resolved) => {
            merge(&mut sandbox.read_only, &resolved.read_only_mounts);
            merge(&mut sandbox.read_write, &resolved.read_write_mounts);
            GRANT_CACHE.lock().insert(
                key,
                (
                    std::time::Instant::now(),
                    (resolved.read_only_mounts, resolved.read_write_mounts),
                ),
            );
            tracing::debug!(
                read_only = sandbox.read_only.len(),
                read_write = sandbox.read_write.len(),
                "[mcp] stdio sandbox grants resolved"
            );
        }
        Err(error) => tracing::warn!(
            %error,
            "[mcp] toolchain grants unavailable; stdio sandbox keeps configured grants only"
        ),
    }
}

#[cfg(not(feature = "modules"))]
pub(super) async fn with_toolchain_grants(_config: &Config, _sandbox: &mut StdioSandboxConfig) {}

/// Whether the backend must meet the untrusted-code isolation floor. The
/// macOS desktop sandbox (Seatbelt) is accepted as folder confinement for the
/// local user, exactly as the shell tool's jail accepts it; a SaaS tenant
/// always requires the floor.
pub(super) fn require_process_isolation(saas: bool) -> bool {
    saas || !cfg!(target_os = "macos")
}

/// How the module treats a backend that cannot meet the isolation floor
/// (`StdioSandboxEnforcement`, tinymcp contract 1.27). Where the host accepts a
/// best-effort backend (the macOS desktop's Seatbelt) the jail is `Declared`,
/// so the module starts the server instead of refusing the best-effort
/// backend; everywhere else it is `Enforced`.
pub(super) fn enforcement(isolated: bool) -> StdioSandboxEnforcement {
    if isolated {
        StdioSandboxEnforcement::Enforced
    } else {
        StdioSandboxEnforcement::Declared
    }
}

fn merge(into: &mut Vec<String>, extra: &[PathBuf]) {
    for path in extra {
        let path = path_string(path);
        if !into.contains(&path) {
            into.push(path);
        }
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn prepare_workspace(workspace_dir: &Path) -> std::io::Result<PathBuf> {
    let dir = workspace_dir.join(WORKSPACE_DIR);
    std::fs::create_dir_all(&dir)?;
    dir.canonicalize()
}

/// Existing absolute grants from configuration, `~/` expanded, each past the
/// host's protected-path floor and the credential-ancestor check.
fn admit_all(raw: &[String], home: Option<&Path>) -> Vec<String> {
    let mut out = Vec::new();
    for entry in raw {
        let expanded = match (entry.strip_prefix("~/"), home) {
            (Some(rest), Some(home)) => home.join(rest),
            _ => PathBuf::from(entry),
        };
        if !expanded.is_absolute() || !expanded.exists() {
            continue;
        }
        // Check the spelled path as well as the resolved one: on macOS `/etc`
        // resolves to `/private/etc`, which the prefix floor does not name.
        let path = expanded.canonicalize().unwrap_or_else(|_| expanded.clone());
        if SecurityPolicy::is_always_forbidden(&expanded)
            || SecurityPolicy::is_always_forbidden(&path)
            || exposes_credentials(&path, home)
        {
            tracing::warn!("[mcp] refused a stdio sandbox grant: host floor forbids it");
            continue;
        }
        let path = path_string(&path);
        if !out.contains(&path) {
            out.push(path);
        }
    }
    out
}

/// Whether granting `path` would expose a credential store under `home`.
fn exposes_credentials(path: &Path, home: Option<&Path>) -> bool {
    let Some(home) = home else { return false };
    [".ssh", ".gnupg", ".aws", ".azure", ".kube"]
        .iter()
        .any(|dir| home.join(dir).starts_with(path))
}

#[cfg(feature = "modules")]
fn credential_denials(config: &Config, home: Option<&Path>, workspace: &Path) -> Vec<String> {
    let stores = home
        .map(crate::security::policy::home_credential_stores)
        .unwrap_or_default()
        .into_iter()
        .chain(crate::security::credentials::credential_files(config));
    let mut denied = Vec::new();
    for store in stores {
        let Ok(real) = store.canonicalize() else {
            continue;
        };
        if workspace.starts_with(&real) {
            tracing::warn!("[mcp] stdio workspace is inside a credential store; not denying it");
            continue;
        }
        let real = path_string(&real);
        if !denied.contains(&real) {
            denied.push(real);
        }
    }
    denied
}

#[cfg(test)]
#[path = "host_sandbox_tests.rs"]
mod tests;
