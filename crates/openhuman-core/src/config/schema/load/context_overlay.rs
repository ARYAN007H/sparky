//! Per-context config overrides reapplied after a fresh persisted reload.

use crate::config::Config;
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ConfigContextOverlay {
    base: Value,
    changes: Vec<Change>,
    runtime: Box<Config>,
}

#[derive(Clone)]
struct Change {
    path: Vec<String>,
    value: Option<Value>,
}

impl std::fmt::Debug for ConfigContextOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigContextOverlay")
            .field("changed_paths", &self.changes.len())
            .finish_non_exhaustive()
    }
}

impl ConfigContextOverlay {
    pub(crate) fn derive(parent: &Config, child: &Config) -> anyhow::Result<Arc<Self>> {
        let inherited = parent.context_overlay.lock().unwrap().clone();
        let base = match inherited.as_ref() {
            Some(overlay) => overlay.base.clone(),
            None => serde_json::to_value(parent)?,
        };
        let target = serde_json::to_value(child)?;
        let mut changes = Vec::new();
        diff_values(&base, &target, &mut Vec::new(), &mut changes);

        let mut runtime = child.clone();
        runtime.context_overlay = std::sync::Mutex::new(None);
        runtime.context_snapshot = None;
        Ok(Arc::new(Self {
            base,
            changes,
            runtime: Box::new(runtime),
        }))
    }

    pub(crate) fn reapply(&self, fresh: &Config) -> anyhow::Result<Config> {
        let mut value = serde_json::to_value(fresh)?;
        for change in &self.changes {
            apply_change(&mut value, &change.path, change.value.clone());
        }
        let mut result: Config = serde_json::from_value(value)?;
        copy_runtime_bindings(fresh, &mut result);
        copy_runtime_bindings(&self.runtime, &mut result);
        result.document_snapshot =
            std::sync::Mutex::new(fresh.document_snapshot.lock().unwrap().clone());
        result.context_snapshot = fresh.context_snapshot.clone();
        result.context_overlay = std::sync::Mutex::new(Some(Arc::new(self.clone())));
        Ok(result)
    }
}

fn diff_values(base: &Value, target: &Value, path: &mut Vec<String>, changes: &mut Vec<Change>) {
    match (base, target) {
        (Value::Object(base), Value::Object(target)) => {
            for (key, value) in target {
                path.push(key.clone());
                if let Some(previous) = base.get(key) {
                    diff_values(previous, value, path, changes);
                } else {
                    changes.push(Change {
                        path: path.clone(),
                        value: Some(value.clone()),
                    });
                }
                path.pop();
            }
            for key in base.keys().filter(|key| !target.contains_key(*key)) {
                path.push(key.clone());
                changes.push(Change {
                    path: path.clone(),
                    value: None,
                });
                path.pop();
            }
        }
        _ if base != target => changes.push(Change {
            path: path.clone(),
            value: Some(target.clone()),
        }),
        _ => {}
    }
}

fn apply_change(current: &mut Value, path: &[String], value: Option<Value>) {
    let Some((key, rest)) = path.split_first() else {
        if let Some(value) = value {
            *current = value;
        }
        return;
    };
    let Some(object) = current.as_object_mut() else {
        return;
    };
    if rest.is_empty() {
        if let Some(value) = value {
            object.insert(key.clone(), value);
        } else {
            object.remove(key);
        }
        return;
    }
    let child = object
        .entry(key.clone())
        .or_insert_with(|| Value::Object(Default::default()));
    apply_change(child, rest, value);
}

pub(crate) fn copy_runtime_bindings(source: &Config, target: &mut Config) {
    target.profile_providers = source.profile_providers.clone();
    target.workspace_dir = source.workspace_dir.clone();
    target.action_dir = source.action_dir.clone();
    target.config_path = source.config_path.clone();
    target.cli_inference_snapshot = source.cli_inference_snapshot.clone();
    target.recovered_from_corruption = source.recovered_from_corruption;
    target.ephemeral_route = source.ephemeral_route.clone();
    target.resolved_endpoints = source.resolved_endpoints.clone();
    target.legacy_memory_sources = source.legacy_memory_sources.clone();
    target.memory.legacy_backend = source.memory.legacy_backend.clone();
    target.composio.host_credential = source.composio.host_credential.clone();
    target.search.engine = source.search.engine.clone();
    target.search.enabled_providers = source.search.enabled_providers.clone();
    target.search.parallel_route = source.search.parallel_route.clone();
    target.search.gemini_route = source.search.gemini_route.clone();
    target.search.host_credentials = source.search.host_credentials.clone();
}
