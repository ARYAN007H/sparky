//! Test-only declaration of the Cargo-built `test-mcp-stub` as an
//! operator-trusted stdio server.
//!
//! Registry-installed servers (`mcp.json`, the registry RPCs) are never
//! operator-trusted, so tinymcp starts them inside a TinyBox jail. With
//! tinymcp 0.19.5 the host declares best-effort enforcement, which macOS
//! Seatbelt satisfies, and tinybox v0.1.40 ships the Linux bubblewrap backend
//! that EX63 (bubblewrap 0.13.1) runs, so the tests on the registry path start
//! the stub jailed on both platforms and no longer need this helper.
//!
//! The trusted stub is still needed only where a test is not on the registry
//! path: it declares the Cargo-built stub in the operator-owned `config.toml`
//! (`[[mcp_client.servers]]`, `operator_trusted = true`), which tinymcp starts
//! as an ordinary child process without a jail.

#![allow(dead_code)]

/// A `config.toml` fragment declaring the stub as an operator-trusted stdio
/// server called `name`, to append to a test config.
pub fn trusted_stub_toml(name: &str, stub: &str) -> String {
    format!(
        "\n[[mcp_client.servers]]\nname = {name:?}\ncommand = {stub:?}\noperator_trusted = true\n"
    )
}
