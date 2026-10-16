//! Agent artifacts as a host serves them: resolving a finished artifact's
//! file under the roots the core may read from.
//!
//! The constructors below the line exist for host tests that need a real
//! artifact on disk; production hosts only resolve.

pub use openhuman_core::artifacts::{FileRoots, resolve_ready_file};

#[doc(hidden)]
pub use openhuman_core::artifacts::{
    ArtifactKind, ArtifactMeta, ArtifactStatus, create_artifact, finalize_artifact,
};
