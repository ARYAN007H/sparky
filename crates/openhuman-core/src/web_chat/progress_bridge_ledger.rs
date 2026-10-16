//! Run-ledger persistence and error reporting for web-chat progress events.

use crate::agent::session_store::with_ledger;

pub(in crate::web_chat) fn ledger_upsert_agent_run(
    config: &crate::config::Config,
    upsert: tinyagents_session::run_ledger::AgentRunUpsert,
) {
    if let Err(err) = with_ledger(&config.workspace_dir, |ledger| async move {
        ledger.upsert_agent_run(upsert).await
    }) {
        log::warn!("[run_ledger][web_channel] failed to upsert run: {err}");
    }
}

pub(in crate::web_chat) fn ledger_append_event(
    config: &crate::config::Config,
    event: tinyagents_session::run_ledger::RunEventAppend,
) {
    if let Err(err) = with_ledger(&config.workspace_dir, |ledger| async move {
        ledger.append_run_event(event).await
    }) {
        log::warn!("[run_ledger][web_channel] failed to append event: {err}");
    }
}

pub(in crate::web_chat) fn ledger_upsert_telemetry(
    config: &crate::config::Config,
    telemetry: tinyagents_session::run_ledger::RunTelemetryUpsert,
) {
    if let Err(err) = with_ledger(&config.workspace_dir, |ledger| async move {
        ledger.upsert_run_telemetry(telemetry).await
    }) {
        log::warn!("[run_ledger][web_channel] failed to upsert telemetry: {err}");
    }
}

pub(in crate::web_chat) fn ledger_get_telemetry(
    config: &crate::config::Config,
    run_id: &str,
) -> Option<tinyagents_session::run_ledger::RunTelemetry> {
    let run_id_owned = run_id.to_string();
    match with_ledger(&config.workspace_dir, |ledger| async move {
        ledger.get_agent_run(&run_id_owned).await
    }) {
        Ok(Some(run)) => {
            let telemetry = run.telemetry;
            log::debug!(
                "[run_ledger][web_channel] read telemetry run_id={} present={}",
                run_id,
                telemetry.is_some()
            );
            telemetry
        }
        Ok(None) => {
            log::debug!(
                "[run_ledger][web_channel] telemetry unavailable; run missing run_id={}",
                run_id
            );
            None
        }
        Err(err) => {
            log::warn!(
                "[run_ledger][web_channel] failed to read telemetry run_id={} err={err}",
                run_id
            );
            None
        }
    }
}
