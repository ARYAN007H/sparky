//! Linked modules for local development and local tests.
//!
//! Module implementation crates never become core dependencies. This crate is
//! the one place they are linked, and only into a host binary built for local
//! work: `openhuman-cli` with its non-product `linked-modules` feature. A
//! shipped or CI build loads pinned, digest-checked releases instead and never
//! reaches this crate.
//!
//! [`install`] registers one factory per enabled module with the core's
//! `register_linked` seam. Registration alone changes nothing: a module is
//! served from the executable only when the configuration asks for it
//! (`[modules] source = "linked"`, `OPENHUMAN_MODULE_SOURCE=linked`, or a
//! per-module override). Everything else still loads from its release.
//!
//! # Which modules are available
//!
//! One cargo feature per registry module id, plus `all`. Every module crate is
//! built with its own `static-link` feature, which exports a Rust
//! `linked_module()` instead of the C `TINYBUS_MODULE_ABI_V1` symbols, so all
//! eleven share one executable without duplicate symbols:
//!
//! | feature          | crate                                   |
//! | ---------------- | --------------------------------------- |
//! | `tinybox`        | `tinybox-module`                        |
//! | `tinychannels`   | `tinychannels-module`                   |
//! | `tinycomputer`   | `tinycomputer`                          |
//! | `tinyconnectors` | `tinyconnectors`                        |
//! | `tinydocs`       | `tinydocs-module`                       |
//! | `tinyhosts`      | `tinyhosts`                             |
//! | `tinyjuice`      | `tinyjuice-module`                      |
//! | `tinymcp`        | `tinymcp`                               |
//! | `tinysearch`     | `tinysearch`                            |
//! | `tinyvoice`      | `tinyvoice-module`                      |
//! | `tinywallet`     | `tinywallet-module`                     |
//!
//! They resolve against the one `vendor/tinybus`, `vendor/tinyagents` and
//! `tinystoragedrivers` through the root `[patch]` tables: every module repo
//! names those by version range (never `=` pins or nested paths in
//! `[dependencies]`) and patches its own nested copy for standalone builds
//! only. `cargo build -p openhuman-modules-linked --features all` is the proof.
//!
//! # Attestation
//!
//! A linked module is attested by the executable's digest, not by a release
//! digest. Confidential members (tinywallet's attested calls) are delivered to
//! it: `lib_tests.rs` signs a transaction through the linked tinywallet over a
//! confidential call and checks that every admitted module is attested as host
//! code. tinywallet is therefore linkable, not `source`-only.
//!
//! # Process-global state
//!
//! Linked modules share one process. Each module's own runtime, caches and
//! environment reads are its own; the shared surfaces are the module bus the
//! host serves and the `tinystoragedrivers` host callbacks, which are keyed by
//! module scope.

use openhuman_rpc::embed::modules::{register_linked, LinkedFactory};

/// The modules this build links, as `(registry id, factory)`.
///
/// Empty when no module feature is enabled.
#[must_use]
pub fn linked_modules() -> Vec<(&'static str, LinkedFactory)> {
    let modules: Vec<(&'static str, LinkedFactory)> = vec![
        #[cfg(feature = "tinybox")]
        ("tinybox", tinybox_module::linked_module),
        #[cfg(feature = "tinychannels")]
        ("tinychannels", tinychannels_module::linked_module),
        #[cfg(feature = "tinycomputer")]
        ("tinycomputer", tinycomputer::linked_module),
        #[cfg(feature = "tinyconnectors")]
        ("tinyconnectors", tinyconnectors::linked_module),
        #[cfg(feature = "tinydocs")]
        ("tinydocs", tinydocs_module::linked_module),
        #[cfg(feature = "tinyhosts")]
        ("tinyhosts", tinyhosts::linked_module),
        #[cfg(feature = "tinyjuice")]
        ("tinyjuice", tinyjuice_module::linked_module),
        #[cfg(feature = "tinymcp")]
        ("tinymcp", tinymcp_linked_module),
        #[cfg(feature = "tinysearch")]
        ("tinysearch", tinysearch::linked_module),
        #[cfg(feature = "tinyvoice")]
        ("tinyvoice", tinyvoice_module::linked_module),
        #[cfg(feature = "tinywallet")]
        ("tinywallet", tinywallet_module::linked_module),
    ];
    modules
}

/// tinymcp v0.18.1 exports the three ABI entry points under `static-link` but no
/// `linked_module()` wrapper, so the factory is assembled from them here.
#[cfg(feature = "tinymcp")]
fn tinymcp_linked_module() -> tinybus::Result<tinybus::module::LinkedModule> {
    use tinymcp::tinybus_module::{
        tinybus_module_init_v1, tinybus_module_manifest_v1, TINYBUS_MODULE_ABI_V1,
    };
    // SAFETY: these are the module's own generated exports, which is exactly
    // what `module_export!`'s `linked_module()` passes.
    unsafe {
        tinybus::module::LinkedModule::from_exports(
            &TINYBUS_MODULE_ABI_V1,
            tinybus_module_manifest_v1,
            tinybus_module_init_v1,
        )
    }
}

/// Register every enabled module with the core's linked-module seam.
///
/// Call once at boot, before the first module load. Idempotent: a repeat call
/// replaces the same factories.
pub fn install() {
    for (id, factory) in linked_modules() {
        log::debug!("[modules-linked] registering id={id}");
        register_linked(id, factory);
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
