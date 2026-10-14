use crate::core::runtime::DispatchCtx;
use super::*;

#[test]
fn help_flag_short_circuits_without_probing_a_driver() {
    run_subsystems_command(&DispatchCtx::process_default(), &["--help".to_string()]).expect("help succeeds");
    run_subsystems_command(&DispatchCtx::process_default(), &["-h".to_string()]).expect("short help succeeds");
}

#[test]
fn bare_invocation_renders_the_table() {
    run_subsystems_command(&DispatchCtx::process_default(), &[]).expect("table renders");
}

#[test]
fn namespace_has_a_registered_cli_adapter() {
    assert!(
        crate::core::all::cli_handler_for_namespace("subsystems").is_some(),
        "bare `openhuman subsystems` must reach the table, not the generic help"
    );
}
