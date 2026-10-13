//! The hook shell is TinyBox's selection with the hook command filled in.

use super::*;
use tinybox_bus::ShellFlavor;

fn posix() -> ShellInvocation {
    ShellInvocation {
        flavor: ShellFlavor::Posix,
        program: "/bin/bash".into(),
        arguments: vec![
            "-p".into(),
            "-c".into(),
            format!("set -o pipefail\n{PLACEHOLDER}"),
        ],
        raw_arguments: None,
        bootstrap_environment: Vec::new(),
        remove_environment: vec!["BASH_ENV".into()],
    }
}

fn cmd() -> ShellInvocation {
    ShellInvocation {
        flavor: ShellFlavor::Cmd,
        program: r"C:\Windows\System32\cmd.exe".into(),
        arguments: vec!["/D".into(), "/S".into(), "/C".into()],
        raw_arguments: Some(format!("\"{PLACEHOLDER}\"")),
        bootstrap_environment: Vec::new(),
        remove_environment: Vec::new(),
    }
}

#[test]
fn the_hook_command_replaces_the_placeholder_in_the_selected_arguments() {
    let process = from_selection(&posix(), "echo 'a b' | cat");
    assert_eq!(process.get_program(), "/bin/bash");
    let args: Vec<_> = process
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert_eq!(args, ["-p", "-c", "set -o pipefail\necho 'a b' | cat"]);
    assert!(process
        .get_envs()
        .any(|(name, value)| name == "BASH_ENV" && value.is_none()));
}

#[test]
fn a_command_that_contains_the_placeholder_text_is_not_rewritten_twice() {
    let process = from_selection(&posix(), PLACEHOLDER);
    let last = process.get_args().last().unwrap().to_string_lossy().into_owned();
    assert_eq!(last, format!("set -o pipefail\n{PLACEHOLDER}"));
}

#[test]
fn a_selection_must_embed_the_command_exactly_once() {
    assert!(embeds_placeholder_once(&posix()));
    assert!(embeds_placeholder_once(&cmd()));
    let mut none = posix();
    none.arguments = vec!["-c".into(), "true".into()];
    assert!(!embeds_placeholder_once(&none));
    let mut twice = posix();
    twice.arguments.push(PLACEHOLDER.into());
    assert!(!embeds_placeholder_once(&twice));
}

#[test]
fn an_unresolved_selection_fails_the_spawn_instead_of_picking_a_shell() {
    // A fresh slot is process-global state; this test only reads the factory's
    // behavior when nothing was installed or after clearing it.
    *slot().write().unwrap() = None;
    let process = command("echo hi");
    assert_eq!(process.as_std().get_program(), UNAVAILABLE);
}
