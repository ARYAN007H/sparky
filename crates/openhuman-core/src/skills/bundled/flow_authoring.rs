//! Bus-backed materialization for the flow-authoring builtin skill.
//!
//! TinyFlows owns the resource bytes. This adapter asks the published Copilot
//! interface for its current files, lets TinySkills own their safe atomic
//! materialization, and keeps a process snapshot so synchronous discovery can
//! verify the disk tree without reaching across the async bus boundary.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

use crate::config::Config;
use crate::core::runtime::{DispatchCtx, DomainSet};
use tinyskills::{MaterializeEntry, MaterializeFile, MaterializeSource, Skill, SkillScope};
use tinyflows_bus::copilot::ResourceFile;

use super::super::ops_discover::api::DiscoveryGate;

const FLOW_SKILL_ID: &str = "flow-authoring";
const FLOW_ROOT_NAME: &str = "builtin-flow-skills";
const COPILOT_RESOURCE_DEADLINE: Duration = Duration::from_secs(60);

#[derive(Clone, Debug)]
struct Snapshot {
    generation: u64,
    files: Vec<ResourceFile>,
}

static SNAPSHOTS: LazyLock<RwLock<HashMap<PathBuf, Snapshot>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Product-owned root for the dynamic skill, separate from static bundles.
pub(crate) fn root(workspace: &Path) -> PathBuf {
    workspace.join(".openhuman").join(FLOW_ROOT_NAME)
}

/// Fetch and materialize Copilot resources when every owning capability is
/// enabled. Failures clear the snapshot and return normally so optional module
/// availability never blocks ordinary chat startup.
pub(crate) async fn prewarm(
    ctx: &DispatchCtx,
    config: &Config,
    domains: Option<DomainSet>,
) {
    let gate = DiscoveryGate::from_config(ctx, config, domains);
    if !gate.flow_authoring_enabled() {
        tracing::debug!(
            skills = config.components.skills,
            flows = config.components.flows,
            tinyflows = config.components.tinyflows,
            "[skills][bundled] flow-authoring prewarm skipped by capability gates"
        );
        return;
    }

    let workspace = config.workspace_dir.as_path();
    let fetched = tokio::time::timeout(COPILOT_RESOURCE_DEADLINE, async {
        let proxy = crate::modules::tinyflows::copilot_proxy(config).await?;
        let member = tinyflows_bus::copilot::METHODS
            .iter()
            .copied()
            .find(|member| *member == "ListResources")
            .ok_or_else(|| "TinyFlows Copilot contract has no ListResources member".to_string())?;
        proxy
            .call::<Vec<ResourceFile>>(member, ())
            .await
            .map_err(|error| error.wire_message())
    })
    .await;

    let resources = match fetched {
        Ok(Ok(files)) => files,
        Ok(Err(error)) => {
            tracing::warn!(%error, "[skills][bundled] flow-authoring resources unavailable; hiding builtin");
            clear_snapshot(workspace);
            return;
        }
        Err(_) => {
            tracing::warn!(
                timeout_seconds = COPILOT_RESOURCE_DEADLINE.as_secs(),
                "[skills][bundled] flow-authoring resource prewarm timed out; hiding builtin"
            );
            clear_snapshot(workspace);
            return;
        }
    };

    if !valid_resource_manifest(&resources) {
        tracing::warn!("[skills][bundled] TinyFlows returned an invalid resource manifest; hiding builtin");
        clear_snapshot(workspace);
        return;
    }
    if !module_is_serving() {
        tracing::warn!("[skills][bundled] TinyFlows module is not serving after ListResources; hiding builtin");
        clear_snapshot(workspace);
        return;
    }

    let previous = SNAPSHOTS
        .read()
        .ok()
        .and_then(|snapshots| snapshots.get(workspace).cloned());
    if let Some(previous) = &previous {
        if !materialization_matches(workspace, previous) {
            tracing::warn!("[skills][bundled] flow-authoring disk tree changed; hiding builtin");
            clear_snapshot(workspace);
            return;
        }
        if previous.files == resources {
            return;
        }
    }

    let generation = NEXT_GENERATION.fetch_add(1, Ordering::SeqCst);
    if let Err(error) = materialize(workspace, &resources) {
        tracing::warn!(%error, "[skills][bundled] flow-authoring materialization failed; hiding builtin");
        clear_snapshot(workspace);
        return;
    }
    if let Ok(mut snapshots) = SNAPSHOTS.write() {
        snapshots.insert(
            workspace.to_path_buf(),
            Snapshot {
                generation,
                files: resources,
            },
        );
    }
    crate::skills::ops_discover::invalidate_workflow_metadata_cache();
}

/// Return the current resource generation only while the owning module and
/// TinySkills-validated on-disk tree still agree.
pub(crate) fn generation_if_current(workspace: &Path) -> Option<u64> {
    if !module_is_serving() {
        clear_snapshot(workspace);
        return None;
    }
    let snapshot = SNAPSHOTS
        .read()
        .ok()
        .and_then(|snapshots| snapshots.get(workspace).cloned())?;
    if !materialization_matches(workspace, &snapshot) {
        tracing::warn!("[skills][bundled] cached flow-authoring materialization changed; hiding builtin");
        clear_snapshot(workspace);
        return None;
    }
    Some(snapshot.generation)
}

/// Load the dynamic skill for synchronous discovery after the metadata path
/// has checked the expected snapshot generation.
pub(crate) fn materialized_skill(workspace: &Path, generation: u64) -> Option<Skill> {
    let snapshot = snapshot_for(workspace, generation)?;
    if !module_is_serving() || !materialization_matches(workspace, &snapshot) {
        clear_snapshot(workspace);
        return None;
    }
    let skill = tinyskills::load_skill_dir(
        &root(workspace).join(FLOW_SKILL_ID),
        SkillScope::Builtin,
    )?;
    (skill.id() == FLOW_SKILL_ID).then_some(skill)
}

/// Read one resource only when the module lifecycle, complete resource
/// inventory, and returned bytes all still match the retained bus snapshot.
pub(crate) fn read_resource(
    workspace: &Path,
    generation: u64,
    skill: &Skill,
    relative_path: &Path,
) -> Result<String, String> {
    let snapshot = snapshot_for(workspace, generation)
        .ok_or_else(|| "flow-authoring resources are unavailable".to_string())?;
    if !module_is_serving() || !materialization_matches(workspace, &snapshot) {
        clear_snapshot(workspace);
        return Err("flow-authoring resources are stale or changed".to_string());
    }
    let requested = relative_path
        .to_str()
        .ok_or_else(|| "resource path is not UTF-8".to_string())?;
    let expected = snapshot
        .files
        .iter()
        .find(|file| file.path == requested)
        .map(|file| file.contents.as_str())
        .ok_or_else(|| "resource is not present in the current TinyFlows snapshot".to_string())?;
    let content = tinyskills::read_resource(skill, relative_path)
        .map_err(|error| error.to_string())?;
    if content != expected {
        clear_snapshot(workspace);
        return Err("flow-authoring resource bytes changed".to_string());
    }
    Ok(content)
}

fn snapshot_for(workspace: &Path, generation: u64) -> Option<Snapshot> {
    SNAPSHOTS
        .read()
        .ok()?
        .get(workspace)
        .filter(|snapshot| snapshot.generation == generation)
        .cloned()
}

fn module_is_serving() -> bool {
    crate::modules::ops::state_of(crate::modules::tinyflows::MODULE_ID)
        == crate::modules::types::ModuleState::Ready
        && crate::modules::runtime::is_module_serving(crate::modules::tinyflows::MODULE_ID)
}

fn valid_resource_manifest(files: &[ResourceFile]) -> bool {
    if files.is_empty() {
        return false;
    }
    let mut paths = BTreeSet::new();
    if files.iter().any(|file| !paths.insert(file.path.as_str())) {
        return false;
    }
    files.iter().any(|file| {
        file.path == tinyskills::WORKFLOW_MD || file.path == tinyskills::SKILL_MD
    })
}

fn materialize(workspace: &Path, resources: &[ResourceFile]) -> Result<(), String> {
    let parent_path = workspace.join(".openhuman");
    std::fs::create_dir_all(&parent_path)
        .map_err(|error| format!("creating builtin skill parent failed: {error}"))?;
    let parent = tinyskills::cap_std::fs::Dir::open_ambient_dir(
        &parent_path,
        tinyskills::cap_std::ambient_authority(),
    )
    .map_err(|error| format!("opening builtin skill parent failed: {error}"))?;
    let files = resources
        .iter()
        .map(|file| MaterializeFile::new(file.path.clone(), file.contents.as_bytes().to_vec()))
        .collect();
    let entry = MaterializeEntry {
        dir_name: FLOW_SKILL_ID.to_string(),
        source: MaterializeSource::Files(files),
    };
    tinyskills::materialize_tree(&parent, FLOW_ROOT_NAME, &[entry])
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn materialization_matches(workspace: &Path, snapshot: &Snapshot) -> bool {
    let directory = root(workspace).join(FLOW_SKILL_ID);
    let Some(skill) = tinyskills::load_skill_dir(&directory, SkillScope::Builtin) else {
        return false;
    };
    if skill.id() != FLOW_SKILL_ID || skill.scope != SkillScope::Builtin {
        return false;
    }

    let Some(manifest) = snapshot.files.iter().find(|file| {
        file.path == tinyskills::WORKFLOW_MD || file.path == tinyskills::SKILL_MD
    }) else {
        return false;
    };
    let Some(location) = skill.location.as_deref() else {
        return false;
    };
    if tinyskills::read_document(location).ok().as_deref() != Some(manifest.contents.as_str()) {
        return false;
    }

    let expected_resources: Vec<PathBuf> = snapshot
        .files
        .iter()
        .filter(|file| file.path != manifest.path)
        .map(|file| PathBuf::from(&file.path))
        .collect();
    let mut actual_resources = skill.resources.clone();
    actual_resources.sort();
    if actual_resources != expected_resources {
        return false;
    }
    expected_resources.iter().all(|path| {
        let Some(expected) = snapshot
            .files
            .iter()
            .find(|file| PathBuf::from(&file.path) == *path)
            .map(|file| file.contents.as_str())
        else {
            return false;
        };
        tinyskills::read_resource(&skill, path).ok().as_deref() == Some(expected)
    })
}

fn clear_snapshot(workspace: &Path) {
    if let Ok(mut snapshots) = SNAPSHOTS.write() {
        snapshots.remove(workspace);
    }
    crate::skills::ops_discover::invalidate_workflow_metadata_cache();
}
