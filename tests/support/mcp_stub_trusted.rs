//! Test-only declaration of the Cargo-built `test-mcp-stub` as an
//! operator-trusted stdio server.
//!
//! TEMPORARY. tinymcp starts every stdio server that is not `operator_trusted`
//! inside a TinyBox jail and asks TinyBox for `Enforced` isolation. TinyBox's
//! macOS Seatbelt backend only reports best-effort enforcement and its Linux
//! Landlock backend reports no process isolation, so no jailed server can
//! start on either platform today. Until the host can choose the enforcement
//! level (tinyhumansai/tinymcp#126) and TinyBox has a Linux backend with real
//! process isolation, the e2e tests declare the stub in the operator-owned
//! `config.toml` (`[[mcp_client.servers]]`, `operator_trusted = true`), which
//! tinymcp starts as an ordinary child process.
//!
//! Registry-installed servers (`mcp.json`, the registry RPCs) are never
//! operator-trusted, so tests of that path cannot use this helper; they are
//! `#[ignore]`d with the same two references until the jail can start them.

#![allow(dead_code)]

/// A `config.toml` fragment declaring the stub as an operator-trusted stdio
/// server called `name`, to append to a test config.
pub fn trusted_stub_toml(name: &str, stub: &str) -> String {
    format!(
        "\n[[mcp_client.servers]]\nname = {name:?}\ncommand = {stub:?}\noperator_trusted = true\n"
    )
}
