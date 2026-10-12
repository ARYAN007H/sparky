//! Which source serves a module: release, linked, source, or a local file.
//!
//! The decision is a pure function of configuration plus two facts about the
//! build ([`linked_factory_available`], [`source_artifact`]), so every mode and
//! every fallback is testable without loading anything. `release` is the
//! default everywhere and the only mode CI and shipped builds use; `linked` and
//! `source` exist for local development and local tests.

use std::path::PathBuf;

use super::types::ModuleSource;
use crate::config::schema::ModuleSourceMode;
use crate::config::Config;

/// The outcome of choosing a source for one module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourcePlan {
    /// What the configuration asked for.
    pub requested: ModuleSourceMode,
    /// What will actually serve the module.
    pub served: ModuleSource,
    /// The local artifact to load, for [`ModuleSource::LocalFile`] and
    /// [`ModuleSource::Source`].
    pub artifact: Option<PathBuf>,
    /// Why `served` differs from `requested`. A static string: it ends up in
    /// `ModuleStatus.detail`, so it never carries a path, URL or credential.
    pub fallback: Option<&'static str>,
}

/// Whether this binary registered an in-process factory for `id`.
///
/// The factory registry arrives with `openhuman-modules-linked` (E0.2); until
/// then no binary has one, so a `linked` request always falls back.
#[must_use]
pub(super) fn linked_factory_available(_id: &str) -> bool {
    false
}

/// The locally built artifact for `id`, if one exists.
///
/// Produced by the source-build step (E0.4); until it lands there is none, so a
/// `source` request without an explicit path falls back to `release`.
#[must_use]
pub(super) fn source_artifact(_id: &str) -> Option<PathBuf> {
    None
}

/// Choose the source for `id` under `config`.
pub(super) fn plan(config: &Config, id: &str) -> SourcePlan {
    plan_with(
        config,
        id,
        linked_factory_available(id),
        source_artifact(id),
    )
}

/// [`plan`] with the build facts supplied, for tests.
pub(super) fn plan_with(
    config: &Config,
    id: &str,
    linked_available: bool,
    built_artifact: Option<PathBuf>,
) -> SourcePlan {
    let mut mode = config.modules.source;
    if let Some(entry) = config.modules.overrides.iter().find(|entry| entry.id == id) {
        match entry.validate() {
            Err(reason) => {
                log::warn!("[modules] ignoring override for '{id}': {reason}");
            }
            Ok(()) => {
                if let Some(path) = &entry.path {
                    return SourcePlan {
                        requested: ModuleSourceMode::Source,
                        served: ModuleSource::LocalFile,
                        artifact: Some(PathBuf::from(path)),
                        fallback: None,
                    };
                }
                if let Some(source) = entry.source {
                    mode = source;
                }
            }
        }
    }

    let released = |fallback: &'static str| {
        SourcePlan {
            requested: mode,
            served: ModuleSource::Release,
            artifact: None,
            fallback: Some(fallback),
        }
    };
    match mode {
        ModuleSourceMode::Release => SourcePlan {
            requested: mode,
            served: ModuleSource::Release,
            artifact: None,
            fallback: None,
        },
        ModuleSourceMode::Linked if linked_available => SourcePlan {
            requested: mode,
            served: ModuleSource::Linked,
            artifact: None,
            fallback: None,
        },
        ModuleSourceMode::Linked => {
            released("no linked factory is registered in this binary; serving the release")
        }
        ModuleSourceMode::Source => match built_artifact {
            Some(path) => SourcePlan {
                requested: mode,
                served: ModuleSource::Source,
                artifact: Some(path),
                fallback: None,
            },
            None => released("no source-built artifact is available; serving the release"),
        },
    }
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
