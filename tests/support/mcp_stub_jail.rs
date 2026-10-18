//! Test-only read grant for the Cargo-built `test-mcp-stub` binary.
//!
//! tinymcp starts every stdio server that is not `operator_trusted` inside a
//! TinyBox jail, and the jail denies whatever it is not granted. The stub
//! lives under `target/`, which no default grant covers, so the tests grant
//! its directory read-only through the product's own `[runtime.local_jail]`
//! setting. This changes nothing about the host's policy: the sandbox, the
//! isolation floor and the protected-path floor all stay in force.

#![allow(dead_code)]

use std::path::Path;

/// The directory holding the stub binary at `stub`.
pub fn stub_dir(stub: &str) -> String {
    Path::new(stub)
        .parent()
        .expect("stub binary has a directory")
        .to_string_lossy()
        .into_owned()
}

/// A `config.toml` fragment granting the stub's directory, to append to a
/// test config that has no `[runtime]` table of its own.
pub fn local_jail_toml(stub: &str) -> String {
    format!(
        "\n[runtime.local_jail]\nextra_read_only = [{:?}]\n",
        stub_dir(stub)
    )
}

/// Grants the stub's directory on an in-memory config.
pub fn grant_stub(config: &mut openhuman_core::config::Config, stub: &str) {
    config
        .runtime
        .local_jail
        .extra_read_only
        .push(stub_dir(stub));
}
