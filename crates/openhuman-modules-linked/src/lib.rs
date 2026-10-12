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
//! One cargo feature per module, named for its registry id, plus `all`. The
//! table is what the current vendor pins support:
//!
//! | feature          | status |
//! | ---------------- | ------ |
//! | `tinybox`        | linked |
//! | `tinychannels`   | linked |
//! | `tinyconnectors` | linked |
//! | `tinydocs`       | linked |
//! | `tinyvoice`      | linked |
//! | `tinymcp`        | waits: `static-link` exists but `linked_module` is not exported |
//! | `tinycomputer`   | waits: no `static-link` |
//! | `tinyjuice`      | waits: no `static-link` |
//! | `tinysearch`     | waits: no `static-link` |
//! | `tinywallet`     | waits: no `static-link`; also see the attestation note below |
//! | `tinyhosts`      | waits: no module crate yet |
//!
//! The waiting modules need the upstream static-link work (E0.3) and a re-pin;
//! each gets its feature here, and a line in [`install`], when that lands.
//!
//! # Attestation
//!
//! A linked module is attested by the executable's digest, not by a release
//! digest. Confidential members (tinywallet's attested calls) must accept that
//! digest in test builds for tinywallet to be linkable; until that is
//! confirmed, treat tinywallet as `source`-only for local testing.

use openhuman_rpc::embed::modules::{register_linked, LinkedFactory};

/// The modules this build links, as `(registry id, factory)`.
///
/// Empty when no module feature is enabled.
#[must_use]
pub fn linked_modules() -> Vec<(&'static str, LinkedFactory)> {
    #[allow(unused_mut)]
    let mut modules: Vec<(&'static str, LinkedFactory)> = Vec::new();
    #[cfg(feature = "tinybox")]
    modules.push(("tinybox", tinybox_module::linked_module));
    #[cfg(feature = "tinychannels")]
    modules.push(("tinychannels", tinychannels_module::linked_module));
    #[cfg(feature = "tinyconnectors")]
    modules.push(("tinyconnectors", tinyconnectors::linked_module));
    #[cfg(feature = "tinydocs")]
    modules.push(("tinydocs", tinydocs_module::linked_module));
    #[cfg(feature = "tinyvoice")]
    modules.push(("tinyvoice", tinyvoice_module::linked_module));
    modules
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
