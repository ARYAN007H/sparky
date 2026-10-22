use super::*;

pub(super) struct EventWiring {
    pub(super) sink: EventSink,
    pub(super) bridge: Arc<OpenhumanEventBridge>,
    pub(super) turn_journal: Option<journal::TurnJournal>,
    pub(super) turn_completed_sink:
        Option<tokio::sync::mpsc::Sender<crate::progress::AgentProgress>>,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn attach(
    on_progress: Option<tokio::sync::mpsc::Sender<crate::progress::AgentProgress>>,
    is_subagent: bool,
    defer_turn_completed_to_caller: bool,
    journal_run_id: tinyagents_harness::ids::RunId,
    native_subagent: Option<&NativeSubagentRunContext>,
    model: &str,
    provider_id: String,
    max_iterations: usize,
    subagent_scope: Option<SubagentScope>,
    cursor: &tinyagents_harness::events::IterationCursor,
    tool_names: &crate::agent::tinyagents::observability::ToolNameMap,
    failure_map: &crate::agent::tinyagents::observability::ToolFailureMap,
    provider_usage_carry: &crate::agent::tinyagents::observability::ProviderUsageCarry,
    bridge_tool_sets: Vec<Arc<Vec<Box<dyn tinytools::Tool>>>>,
    run_context: &OpenHumanRunContext,
    ctx: &tinyagents_harness::context::RunContext<OpenHumanRunContext>,
) -> EventWiring {
    let turn_completed_sink = (!is_subagent && !defer_turn_completed_to_caller)
        .then(|| on_progress.clone())
        .flatten();
    // The EventSink is created unconditionally (bridge, cap pauser and the
    // durable journal of issue #4249, 05.1 all subscribe; it is inert when
    // nothing does), so an unobserved turn stays reconstructable.
    //
    // Mint the durable run id *before* the sink and seed the sink stream prefix
    // with it (`with_stream_id`), so every persisted observation's `event_id` is
    // the restart-stable `{run_id}-evt-{offset}` a late-attach replay
    // reconstructs the timeline from (05.1). The same id keys the journal + status.
    let sink = EventSink::with_stream_id(journal_run_id.as_str());
    if let Some(native) = native_subagent {
        *native
            .child_events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(sink.clone());
        native
            .parent_events
            .emit(tinyagents_harness::events::AgentEvent::SubAgentRunStarted {
                name: native.name.clone(),
                depth: native.depth,
                run_id: journal_run_id.clone(),
            });
    }
    // Attach the event bridge for EVERY turn — including an unobserved
    // (`on_progress = None`) background/cron turn (#4467, item 3). The bridge's
    // `record_usage` feeds the global cost tracker on each `UsageRecorded` event
    // *during* the run, so a run that burns N model calls and then fails still
    // contributes that spend to the wallet/cost surfaces — the post-run
    // `record_unobserved_turn_usage` fallback only runs on the success path
    // and never sees a failed run's usage. With `on_progress = None` the bridge
    // still records cost but its progress `send`s are inert no-ops, so there is
    // no spurious streaming; every turn has an event sink.
    let bridge = {
        let bridge = OpenhumanEventBridge::with_scope(
            on_progress,
            model,
            provider_id.clone(),
            max_iterations,
            subagent_scope.clone(),
            cursor.clone(),
            tool_names.clone(),
            failure_map.clone(),
            provider_usage_carry.clone(),
            bridge_tool_sets.clone(),
            run_context.dispatch_ctx(),
        );
        bridge.set_definition(
            run_context
                .memory_identity
                .as_ref()
                .and_then(|identity| identity.agent_id.as_deref()),
        );
        sink.subscribe(bridge.clone());
        bridge
    };

    // Durable event journal + status store (issue #4249, 05.1). Attached *in
    // addition to* the bridge above: the EventSink fans out to both, so the
    // existing progress/global-bus path is untouched. Best-effort and non-fatal
    // — a failure to open/attach the journal returns `None` and the turn runs
    // unaffected. The handle stamps the terminal status once the run returns.
    // A sub-agent turn records under its task scope as the status thread id, so
    // `list_by_thread` can enumerate a task's runs (full parent/root lineage is
    // a 05.2/05.3 follow-up).
    let journal_thread_id = subagent_scope
        .as_ref()
        .map(|scope| tinyagents_harness::ids::ThreadId::new(scope.task_id.clone()));
    let turn_journal = journal::attach_turn_journal(
        &run_context.dispatch_ctx(),
        &sink,
        model,
        journal_run_id.clone(),
        journal_thread_id,
        ctx.config.lineage.parent_run_id.clone(),
        ctx.config.lineage.root_run_id.clone(),
    )
    .await;
    if let Some(native) = native_subagent {
        *native
            .child_journal
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = turn_journal.clone();
    }
    if !is_subagent {
        if let Some(crate::security::turn_origin::AgentTurnOrigin::WebChat {
            request_id: Some(request_id),
            ..
        }) = run_context.origin.as_ref()
        {
            journal::register_request_journal_run(
                &run_context.dispatch_ctx(),
                request_id,
                journal_run_id.as_str(),
            );
        }
    }

    // Per-turn controls for the driver. Best-effort thread label for the
    // delivery/requeue observability events and the metadata on any requeued
    // steer: a sub-agent uses its task id; the interactive/channel parent turn
    // reads the turn origin.
    EventWiring {
        sink,
        bridge,
        turn_journal,
        turn_completed_sink,
    }
}
