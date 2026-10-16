//! Hosted channel delivery and messaging.
//!
//! Empty for now: the code for this area moves here from `openhuman-core`'s
//! `backend/` and its consumers, and plugs back into the core through the
//! generic registries on `openhuman_embed::__host` (`channel_callback`). [`register`] is the
//! one place it does so; `crate::register::register_all` calls it from
//! `install()`.

use crate::install::InstallOptions;

/// Registers this area's hooks with the core. Idempotent.
pub(crate) fn register(_options: &InstallOptions) {
    log::trace!("[tinyhumans] channels: nothing to register yet");
}
