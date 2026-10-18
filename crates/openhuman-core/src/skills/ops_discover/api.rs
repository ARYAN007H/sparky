//! Public discovery entry points: the workspace-trust check and the handful
//! of `load_workflow_metadata*` / `discover_workflows*` shims that select a
//! root scan (see [`super::scan`]) for a given caller shape.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, RwLock};

use {crate::skills::ops_types::TRUST_MARKER, tinyskills::Skill as Workflow};

use super::scan::{discover_filtered, ALL_ROOT_KINDS, WORKFLOW_ROOT_KINDS};

/// The visibility state that the metadata cache must keep separate even when
/// callers share one workspace directory.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct DiscoveryGate {
    tenant: Option<crate::core::runtime::Tenant>,
    skills_component: bool,
    flows_component: bool,
    tinyflows_component: bool,
    skills_domain: Option<bool>,
    flows_domain: Option<bool>,
}

impl DiscoveryGate {
    pub(crate) fn from_dispatch(ctx: &crate::core::runtime::DispatchCtx) -> Self {
        let components = ctx.components();
        let domains = ctx.domains();
        Self {
            tenant: ctx.tenant().ok(),
            skills_component: components.skills,
            flows_component: components.flows,
            tinyflows_component: components.tinyflows,
            skills_domain: domains.map(|domains| domains.skills),
            flows_domain: domains.map(|domains| domains.flows),
        }
    }

    pub(crate) fn from_config(
        ctx: &crate::core::runtime::DispatchCtx,
        config: &crate::config::Config,
        domains: Option<crate::core::runtime::DomainSet>,
    ) -> Self {
        let domains = domains.or_else(|| ctx.domains());
        Self {
            tenant: ctx.tenant().ok(),
            skills_component: config.components.skills,
            flows_component: config.components.flows,
            tinyflows_component: config.components.tinyflows,
            skills_domain: domains.map(|domains| domains.skills),
            flows_domain: domains.map(|domains| domains.flows),
        }
    }

    fn skills_enabled(&self) -> bool {
        cfg!(feature = "skills")
            && self.skills_component
            && self.skills_domain.unwrap_or(true)
    }

    pub(crate) fn flow_authoring_enabled(&self) -> bool {
        cfg!(feature = "skills")
            && cfg!(feature = "flows")
            && cfg!(feature = "modules")
            && self.skills_component
            && self.flows_component
            && self.tinyflows_component
            && self.skills_domain.unwrap_or(true)
            && self.flows_domain.unwrap_or(true)
            && self.tenant.is_some()
    }
}

/// Workspace, home, trust, and the exact tenant and narrowing gates that
/// selected the catalogue. Two profiles can share a workspace without
/// sharing a cached view of optional builtin resources.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct MetadataCacheKey {
    workspace: PathBuf,
    home: Option<PathBuf>,
    trusted: bool,
    gate: DiscoveryGate,
    flow_generation: Option<u64>,
}
static METADATA_CACHE: LazyLock<RwLock<HashMap<MetadataCacheKey, Vec<Workflow>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static METADATA_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Drop the process-wide metadata snapshot after a skill is created, installed,
/// or removed. Live sessions receive `WorkflowsChanged` separately and rebuild
/// their own catalogue from the next cached snapshot.
pub(crate) fn invalidate_workflow_metadata_cache() {
    if let Ok(mut cache) = METADATA_CACHE.write() {
        cache.clear();
        METADATA_GENERATION.fetch_add(1, Ordering::SeqCst);
    }
}

/// Initialize the legacy skills directory in the specified workspace.
///
/// Creates `<workspace>/skills/` and a placeholder `README.md` so the folder
/// is visible to the user. New-style skills should live under
/// `<workspace>/.openhuman/skills/` instead, but this directory is kept for
/// backward compatibility.
pub fn init_workflows_dir(workspace_dir: &Path) -> Result<(), String> {
    let skills_dir = workspace_dir.join("skills");
    std::fs::create_dir_all(&skills_dir).map_err(|e| {
        format!(
            "failed to create skills directory {}: {e}",
            skills_dir.display()
        )
    })?;

    let readme_path = skills_dir.join("README.md");
    if !readme_path.exists() {
        let content = "# Skills\n\nPut one skill per directory under this folder.\n";
        std::fs::write(&readme_path, content)
            .map_err(|e| format!("failed to write {}: {e}", readme_path.display()))?;
    }

    Ok(())
}

/// The home directory skill discovery scans for user-scope roots
/// (`~/.openhuman/skills/`, `~/.agents/skills/`, `~/.openhuman/workflows/`).
///
/// `dirs::home_dir()` for every ordinary caller. `None` when the ambient
/// [`CoreContext`](crate::core::runtime::CoreContext) was derived with
/// `user_skill_roots = false` — an embedded agent whose host installed its
/// skills explicitly and must not see the operator's own — so the caller's
/// discovery pipeline scans no user scope at all, exactly as passing `None`
/// for `home_dir` always has.
pub fn discovery_home_dir() -> Option<PathBuf> {
    if crate::core::runtime::CoreContext::current_user_skill_roots() {
        dirs::home_dir()
    } else {
        log::debug!("[skills][discover] user-scope roots hidden by the ambient context");
        None
    }
}

/// Backwards-compatible shim for callers that only have a workspace path.
///
/// Delegates to [`discover_workflows`] with the current user's home directory
/// so user-scope skills (`~/.openhuman/skills/`, `~/.agents/skills/`) are
/// surfaced for existing production callers (`agent::session_host::builder`,
/// `channels::runtime::startup`). Previously this shim passed `None` for the
/// home directory, which silently dropped user-installed skills from the
/// main runtime path.
///
/// Project-scope (workspace) skills still take precedence over user-scope
/// on name collisions.
// keep until v0.66 (review after 2027-01-11): this is the cached, ambient-user
// discovery entry point; discover_workflows alone omits those host semantics.
pub fn load_workflow_metadata(workspace_dir: &Path) -> Vec<Workflow> {
    load_workflow_metadata_with_gate(
        workspace_dir,
        discovery_home_dir(),
        DiscoveryGate::from_dispatch(&crate::core::runtime::DispatchCtx::ambient()),
    )
}

/// Discover workflow metadata using the caller's supplied configuration and
/// dispatch identity. This is the entry point for session and host builders
/// that already hold an explicit workspace/profile pair.
pub fn load_workflow_metadata_for(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &crate::config::Config,
) -> Vec<Workflow> {
    load_workflow_metadata_for_domains(ctx, config, None)
}

/// As [`load_workflow_metadata_for`] with the host's resolved domain set when
/// startup has already narrowed it.
pub(crate) fn load_workflow_metadata_for_domains(
    ctx: &crate::core::runtime::DispatchCtx,
    config: &crate::config::Config,
    domains: Option<crate::core::runtime::DomainSet>,
) -> Vec<Workflow> {
    let home = match ctx.resolved() {
        Some(core) if core.user_skill_roots() => dirs::home_dir(),
        Some(_) => None,
        None => discovery_home_dir(),
    };
    load_workflow_metadata_with_gate(
        &config.workspace_dir,
        home,
        DiscoveryGate::from_config(ctx, config, domains),
    )
}

pub(super) fn load_workflow_metadata_with_gate(
    workspace_dir: &Path,
    home: Option<PathBuf>,
    gate: DiscoveryGate,
) -> Vec<Workflow> {
    if !gate.skills_enabled() {
        log::debug!("[skills][discover] load_workflow_metadata -> [] ([components] skills off)");
        return Vec::new();
    }
    let trusted = is_workspace_trusted(workspace_dir);
    let flow_generation = if gate.flow_authoring_enabled() {
        crate::skills::bundled::flow_authoring::generation_if_current(workspace_dir)
    } else {
        None
    };
    let key = MetadataCacheKey {
        workspace: workspace_dir.to_path_buf(),
        home: home.clone(),
        trusted,
        gate: gate.clone(),
        flow_generation,
    };
    if let Ok(cache) = METADATA_CACHE.read() {
        if let Some(workflows) = cache.get(&key) {
            return workflows.clone();
        }
    }
    let generation = METADATA_GENERATION.load(Ordering::SeqCst);
    let workflows = discover_workflows_inner_with_gate(
        home.as_deref(),
        Some(workspace_dir),
        trusted,
        &gate,
        flow_generation,
    );
    if let Ok(mut cache) = METADATA_CACHE.write() {
        if METADATA_GENERATION.load(Ordering::SeqCst) == generation {
            cache.insert(key, workflows.clone());
        }
    }
    workflows
}

/// Discover skills from every supported location.
///
/// * `home_dir` — user home (typically `dirs::home_dir()`), scanned for
///   `~/.openhuman/skills/` and `~/.agents/skills/`.
/// * `workspace_dir` — current workspace, scanned for project-scope paths.
/// * `trusted` — whether the caller has verified the project trust marker.
///   Project-scope skills are silently skipped when `false`.
///
/// On name collisions, project-scope wins over user-scope and a warning is
/// attached to the retained skill.
pub fn discover_workflows(
    home_dir: Option<&Path>,
    workspace_dir: Option<&Path>,
    trusted: bool,
) -> Vec<Workflow> {
    #[cfg(test)]
    DISCOVERY_CALLS.with(|c| c.set(c.get() + 1));
    let gate = DiscoveryGate::from_dispatch(&crate::core::runtime::DispatchCtx::ambient());
    let generation = workspace_dir
        .filter(|_| gate.flow_authoring_enabled())
        .and_then(crate::skills::bundled::flow_authoring::generation_if_current);
    discover_workflows_inner_with_gate(home_dir, workspace_dir, trusted, &gate, generation)
}

#[cfg(test)]
thread_local! {
    /// Test-only counter of full on-disk discovery passes made on this thread.
    /// Discovery re-reads and re-parses every skill bundle under every root, so
    /// a caller that runs it twice for one lookup pays the whole tree twice
    /// (#6166). Thread-local so parallel tests can't perturb each other's count.
    pub(crate) static DISCOVERY_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Whether the workspace has opted into loading project-scope skills.
///
/// Looks for `<workspace>/.openhuman/trust`. The marker file's contents are
/// ignored — presence is sufficient.
pub fn is_workspace_trusted(workspace_dir: &Path) -> bool {
    workspace_dir.join(".openhuman").join(TRUST_MARKER).exists()
}

pub(crate) fn discover_workflows_inner(
    home_dir: Option<&Path>,
    workspace_dir: Option<&Path>,
    trusted: bool,
) -> Vec<Workflow> {
    let gate = DiscoveryGate::from_dispatch(&crate::core::runtime::DispatchCtx::ambient());
    let generation = workspace_dir
        .filter(|_| gate.flow_authoring_enabled())
        .and_then(crate::skills::bundled::flow_authoring::generation_if_current);
    discover_workflows_inner_with_gate(home_dir, workspace_dir, trusted, &gate, generation)
}

pub(crate) fn discover_workflows_inner_with_gate(
    home_dir: Option<&Path>,
    workspace_dir: Option<&Path>,
    trusted: bool,
    gate: &DiscoveryGate,
    flow_generation: Option<u64>,
) -> Vec<Workflow> {
    discover_filtered(
        home_dir,
        workspace_dir,
        trusted,
        ALL_ROOT_KINDS,
        gate,
        flow_generation,
    )
}

/// Discover only automation bundles under the `workflows/` roots.
/// Capability skills are deliberately excluded; they remain available to the
/// agent harness and run/describe paths.
///
/// Note: bundles authored *before* the skills→workflows rename live under the
/// `skills/` roots and will therefore not appear in this automations-only view;
/// new automations created via "New workflow" land in `~/.openhuman/workflows/`.
pub fn discover_automations(
    home_dir: Option<&Path>,
    workspace_dir: Option<&Path>,
    trusted: bool,
) -> Vec<Workflow> {
    tracing::debug!(
        trusted,
        has_home = home_dir.is_some(),
        has_workspace = workspace_dir.is_some(),
        "[workflows] discover:automations:enter"
    );
    let gate = DiscoveryGate::from_dispatch(&crate::core::runtime::DispatchCtx::ambient());
    discover_filtered(
        home_dir,
        workspace_dir,
        trusted,
        WORKFLOW_ROOT_KINDS,
        &gate,
        None,
    )
}
