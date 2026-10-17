impl SpawnAsyncSubagentTool {
    async fn execute_with_context_inner(
        &self,
        args: serde_json::Value,
        _options: ToolCallOptions,
        tool_context: Option<&dyn ToolRunContext>,
        run_context: crate::agent::tinyagents::host::OpenHumanRunContext,
        detached_parent: Option<
            tinyagents_harness::context::RunContext<
                crate::agent::tinyagents::host::OpenHumanRunContext,
            >,
        >,
    ) -> anyhow::Result<ToolResult> {
        let dispatch = run_context.dispatch_ctx();
        let AsyncSpawnArgs {
            agent_id,
            prompt,
            context,
            model_override,
            task_title,
            task_key,
            force_fresh,
        } = decode_async_spawn_args(&args);

        if let Some(refusal) = refuse_empty_spawn_args(&agent_id, &prompt) {
            return Ok(refusal);
        }
        let Some(detached_parent) = detached_parent else {
            return Ok(ToolResult::error(
                "spawn_async_subagent requires a live harness run context.",
            ));
        };

        let parent = match run_context.parent.clone() {
            Some(parent) => parent,
            None => {
                return Ok(ToolResult::error(
                    "spawn_async_subagent called outside of an agent turn",
                ));
            }
        };

        // Preserve the actual caller's authority before any filesystem/session
        // effects. A scoped refusal cannot become an accepted/running reference.
        if let Err(error) = running_subagents::admit_spawn(&dispatch, &parent.workspace_dir) {
            return Ok(ToolResult::error(format!(
                "spawn_async_subagent: registration refused: {error}"
            )));
        }

        let registry = match AgentDefinitionRegistry::for_core_context(
            run_context.dispatch_ctx().into_core(),
        ) {
            Some(registry) => registry,
            None => {
                return Ok(ToolResult::error(
                    "spawn_async_subagent: AgentDefinitionRegistry has not been initialised",
                ));
            }
        };
        let definition = match registry.get(&agent_id).cloned() {
            Some(definition) => definition,
            None => {
                let available: Vec<&str> = registry.list().iter().map(|d| d.id.as_str()).collect();
                return Ok(ToolResult::error(format!(
                    "spawn_async_subagent: unknown agent_id '{agent_id}'. Available: {}",
                    available.join(", ")
                )));
            }
        };

        let fleet = if parent.visible_tool_names.is_empty() {
            FleetToolSet::for_parent(&parent.agent_definition_id)
        } else {
            FleetToolSet::from_visible_tool_names(&parent.visible_tool_names)
        };

        if !parent.allowed_subagent_ids.contains(&definition.id) {
            log::warn!(
                "[spawn_async_subagent] blocked subagent outside allowlist parent={} requested={} allowed={:?}",
                parent.agent_definition_id,
                definition.id,
                parent.allowed_subagent_ids
            );
            return Ok(ToolResult::error(format!(
                "spawn_async_subagent: agent '{}' is not in parent agent '{}' subagents.allowlist",
                definition.id, parent.agent_definition_id
            )));
        }

        let parent_session = parent.session_id.clone();
        let progress_sink = parent.on_progress.clone();
        let parent_thread_id = tool_context
            .and_then(ToolRunContext::thread_id)
            .or(run_context.thread_id.as_deref())
            .map(str::to_owned);

        if let Some(refusal) = refuse_missing_delivery_thread(
            parent_thread_id.as_deref(),
            &parent.agent_definition_id,
            &definition.id,
        ) {
            return Ok(refusal);
        }
        let store = SubagentSessionStore::new(parent.workspace_dir.clone());
        let workspace_descriptor = tool_context
            .and_then(|ctx| ctx.workspace().cloned())
            .or_else(|| run_context.workspace.clone());
        let effective_action_root = workspace_descriptor
            .as_ref()
            .map(|workspace| {
                tracing::debug!(
                    workspace_root = %workspace.root.display(),
                    policy_id = %workspace.policy_id,
                    "[spawn_async_subagent] using ToolExecutionContext workspace root"
                );
                workspace.root.clone()
            })
            .or_else(|| {
                crate::security::live_policy::effective_in(&dispatch)
                    .map(|policy| policy.action_dir.clone())
            });
        let selector = SubagentSessionSelector {
            parent_session: parent_session.clone(),
            parent_thread_id: parent_thread_id.clone(),
            agent_id: definition.id.clone(),
            model: model_override.clone(),
            sandbox_mode: format!("{:?}", definition.sandbox_mode),
            action_root: subagent_sessions::action_root_key(effective_action_root.as_deref()),
            task_key: task_key.clone(),
        };

        let reusable = if force_fresh {
            match subagent_sessions::find_reusable(&store, &selector) {
                Ok(Some(session)) => {
                    let jobs = crate::core::runtime::slot_for::<
                        tinyagents_orchestration::subagent::SubAgentJobRegistry,
                    >(&dispatch);
                    let _ = jobs;
                    let _ = running_subagents::cancel_by_session_in_workspace(
                        &dispatch,
                        &session.subagent_session_id,
                        &parent_session,
                        &parent.workspace_dir,
                    );
                    match subagent_sessions::close_if_current(&store, &session) {
                        Ok(true) => {}
                        Ok(false) => {
                            return Ok(ToolResult::error(
                                "spawn_async_subagent: reusable session changed during fresh start; retry with the current session",
                            ));
                        }
                        Err(err) => {
                            return Ok(ToolResult::error(format!(
                                "spawn_async_subagent: failed to close reusable session: {err}"
                            )));
                        }
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    log::warn!(
                        "[subagent_reuse] fresh lookup failed parent_thread_id={} agent_id={} task_key={} error={}",
                        parent_thread_id.as_deref().unwrap_or("none"),
                        definition.id,
                        task_key,
                        err
                    );
                }
            }
            None
        } else {
            match subagent_sessions::find_reusable(&store, &selector) {
                Ok(session) => session,
                Err(err) => {
                    log::warn!(
                        "[subagent_reuse] lookup failed parent_thread_id={} agent_id={} task_key={} error={}",
                        parent_thread_id.as_deref().unwrap_or("none"),
                        definition.id,
                        task_key,
                        err
                    );
                    None
                }
            }
        };
        let reuse_decision = subagent_sessions::reuse_decision(reusable.as_ref(), force_fresh);
        let follow_up_prompt = reusable_follow_up_message(&prompt, context.as_deref());

        if let Some(session) = reusable.as_ref() {
            if session.status == DurableSubagentStatus::Running {
                if let Some(ref running_task_id) = session.current_task_id {
                    match steer_reusable_session(
                        &detached_parent,
                        &session.subagent_session_id,
                        follow_up_prompt.clone(),
                    )
                    .await
                    {
                        Ok(()) => {
                            log::info!(
                                "[subagent_reuse] parent_thread_id={} subagent_session_id={} task_id={} agent_id={} reuse_decision={}",
                                parent_thread_id.as_deref().unwrap_or("none"),
                                session.subagent_session_id,
                                running_task_id,
                                definition.id,
                                reuse_decision.as_str()
                            );
                            let payload = async_subagent_ref_payload(
                                running_task_id,
                                &session.subagent_session_id,
                                &definition.id,
                                session.worker_thread_id.as_deref(),
                                true,
                                reuse_decision.as_str(),
                                "running",
                                &fleet,
                            );
                            let follow_up = if fleet.can_wait() {
                                "Use the structured reference below to send more input, wait, or perform a short timeout tick."
                            } else {
                                "Its result is delivered to you automatically on a later turn; do not wait or poll for it."
                            };
                            return Ok(ToolResult::success(format!(
                                "Continued reusable async sub-agent `{}`. It is already running and will pick up the new instruction at its next step. \
                                 {}\n\n[async_subagent_ref]\n{}\n[/async_subagent_ref]",
                                payload["agent_id"].as_str().unwrap_or("subagent"),
                                follow_up,
                                serde_json::to_string(&payload)
                                    .unwrap_or_else(|_| "{}".to_string())
                            )));
                        }
                        Err(err) => {
                            log::warn!(
                                "[subagent_reuse] running steer failed parent_thread_id={} subagent_session_id={} task_id={} agent_id={} error={:?}",
                                parent_thread_id.as_deref().unwrap_or("none"),
                                session.subagent_session_id,
                                running_task_id,
                                definition.id,
                                err
                            );
                        }
                    }
                }
            }
        }

        let task_id = format!("sub-{}", uuid::Uuid::new_v4());
        if let Some(thread_id) = parent_thread_id.as_deref() {
            if crate::agent::orchestration::background_completions::mark_stopped_task_if_thread_stopped(
                &dispatch,
                &parent.workspace_dir,
                thread_id,
                &task_id,
            ) {
                return Ok(ToolResult::error(
                    "spawn_async_subagent: parent thread is stopped or deleted",
                ));
            }
        }
        let worker_thread_id = reusable
            .as_ref()
            .and_then(|session| session.worker_thread_id.clone())
            .or_else(|| {
                parent_thread_id.as_ref().and_then(|parent_thread_id| {
                    crate::threads::worker_thread::create_worker_thread(
                        &dispatch,
                        parent.workspace_dir.clone(),
                        parent_thread_id,
                        &definition.id,
                        &task_title,
                        &prompt,
                    )
                    .ok()
                })
            });
        if let (Some(session), Some(worker_thread_id)) =
            (reusable.as_ref(), worker_thread_id.as_ref())
        {
            if session.worker_thread_id.as_deref() == Some(worker_thread_id.as_str()) {
                crate::threads::worker_thread::append_reuse_follow_up(
                    &dispatch,
                    parent.workspace_dir.clone(),
                    worker_thread_id,
                    &definition.id,
                    &task_id,
                    &follow_up_prompt,
                    (parent_thread_id.as_deref(), &session.subagent_session_id),
                );
            }
        }
        let durable_session = match subagent_sessions::upsert_running(
            &store,
            SubagentSessionUpsert {
                selector,
                display_name: Some(definition.display_name().to_string()),
                task_title: task_title.clone(),
                worker_thread_id: worker_thread_id.clone(),
                task_id: task_id.clone(),
                lifecycle_key: Some(
                    tinyagents_orchestration::subagent::SubagentTaskKey::from_context(
                        &detached_parent,
                        task_id.clone(),
                        parent_thread_id.clone(),
                    ),
                ),
            },
            reusable.as_ref(),
        ) {
            Ok(session) => session,
            Err(err) => {
                log::warn!(
                    "[subagent_reuse] upsert failed parent_thread_id={} task_id={} agent_id={} reuse_decision={} error={}",
                    parent_thread_id.as_deref().unwrap_or("none"),
                    task_id,
                    definition.id,
                    reuse_decision.as_str(),
                    err
                );
                return Ok(ToolResult::error(format!(
                    "spawn_async_subagent: failed to persist reusable sub-agent session: {err}"
                )));
            }
        };

        let initial_history = reusable
            .as_ref()
            .and_then(|session| session.latest_history.clone())
            .map(|mut history| {
                history.push(TranscriptMessage::user(follow_up_prompt.clone()));
                history
            });

        log::info!(
            "[subagent_reuse] parent_thread_id={} subagent_session_id={} task_id={} agent_id={} reuse_decision={} task_key={}",
            parent_thread_id.as_deref().unwrap_or("none"),
            durable_session.subagent_session_id,
            task_id,
            definition.id,
            reuse_decision.as_str(),
            task_key
        );

        crate::agent::orchestration::subagent_events::publish_subagent_spawned(
            parent_session.clone(),
            definition.id.clone(),
            "async".to_string(),
            task_id.clone(),
            prompt.chars().count(),
        );
        if let Some(ref tx) = progress_sink {
            let _ = tx
                .send(AgentProgress::SubagentSpawned {
                    agent_id: definition.id.clone(),
                    task_id: task_id.clone(),
                    mode: "async".to_string(),
                    dedicated_thread: worker_thread_id.is_some(),
                    prompt_chars: prompt.chars().count(),
                    prompt: prompt.clone(),
                    worker_thread_id: worker_thread_id.clone(),
                    display_name: Some(definition.display_name().to_string()),
                    parent_call_id: crate::tools::host_extensions::tool_call_id(tool_context),
                })
                .await;
        }

        let background_workspace_dir = parent.workspace_dir.clone();
        let background_dispatch = dispatch.clone();
        let background_agent_id = definition.id.clone();
        let background_task_id = task_id.clone();
        let background_parent_session = parent_session.clone();
        let completion_target =
            crate::agent::orchestration::background_completions::CompletionTarget::new(
                dispatch.clone(),
                parent.workspace_dir.clone(),
                parent_session.clone(),
                parent_thread_id.clone(),
            );
        let background_worker_thread_id = worker_thread_id.clone();
        let background_store = store.clone();
        let background_subagent_session_id = durable_session.subagent_session_id.clone();
        let background_thread_id = background_worker_thread_id
            .clone()
            .unwrap_or_else(|| background_subagent_session_id.clone());
        let background_parent_thread_id = parent_thread_id.clone();
        log::debug!(
            "[spawn_async_subagent] register task_id={} parent_session={} parent_thread_id={}",
            task_id,
            parent_session,
            background_parent_thread_id.as_deref().unwrap_or("none")
        );
        let completion_core = dispatch.clone().into_core();
        let completion_origin = detached_parent.data.origin.clone();
        let completion_worker_thread_id = background_worker_thread_id.clone();
        let registration_progress = progress_sink.clone();
        let registration_completion_target = completion_target.clone();
        let registration_parent_session = parent_session.clone();
        let registration_agent_id = definition.id.clone();
        let registration_task_id = task_id.clone();
        let completion = move |_target: String,
                               result: Result<SubagentRunOutcome, String>,
                               emit_lifecycle_effects: bool|
              -> std::pin::Pin<
            Box<dyn std::future::Future<Output = ()> + Send>,
        > {
            let core = completion_core.clone();
            let origin = completion_origin.clone();
            let progress_sink = progress_sink.clone();
            let background_workspace_dir = background_workspace_dir.clone();
            let background_dispatch = background_dispatch.clone();
            let background_agent_id = background_agent_id.clone();
            let background_task_id = background_task_id.clone();
            let background_parent_session = background_parent_session.clone();
            let completion_target = completion_target.clone();
            let background_worker_thread_id = completion_worker_thread_id.clone();
            let background_store = background_store.clone();
            let background_subagent_session_id = background_subagent_session_id.clone();
            let background_parent_thread_id = background_parent_thread_id.clone();
            Box::pin(async move {
                let finish = async move {
                    let mut abort_report =
                        AbortReport::arm(progress_sink, &background_agent_id, &background_task_id);
                    let result = result.map_err(anyhow::Error::msg);
                    match result {
                        Ok(outcome) => {
                            match outcome.status {
                                SubagentRunStatus::Completed => {
                                    if let Err(err) = subagent_sessions::mark_finished(
                                        &background_store,
                                        &background_subagent_session_id,
                                        &outcome.task_id,
                                        &outcome.status,
                                        outcome.final_history.clone(),
                                    ) {
                                        log::warn!(
                                        "[subagent_reuse] mark_completed failed subagent_session_id={} task_id={} agent_id={} error={}",
                                        background_subagent_session_id,
                                        outcome.task_id,
                                        outcome.agent_id,
                                        err
                                    );
                                    }
                                    // A workflow proposal from the child's tool history is
                                    // durable state, not prose: persist it into the parent chat
                                    // thread (survives reload / reconnect) and carry the full
                                    // payload in the delivery notice for the follow-up turn.
                                    if emit_lifecycle_effects {
                                        let delivery_summary = attach_workflow_proposal(
                                            &background_dispatch,
                                            &background_workspace_dir,
                                            background_parent_thread_id.as_deref(),
                                            &outcome.task_id,
                                            &outcome.agent_id,
                                            &outcome.final_history,
                                            outcome.output.clone(),
                                        );
                                        completion_target
                                            .completed(
                                                &outcome.task_id,
                                                &outcome.agent_id,
                                                delivery_summary,
                                            )
                                            .await;
                                        crate::agent::orchestration::subagent_events::publish_subagent_completed(
                            background_parent_session,
                            outcome.task_id.clone(),
                            outcome.agent_id.clone(),
                            outcome.elapsed.as_millis() as u64,
                            outcome.output.chars().count(),
                            outcome.iterations,
                        );
                                        abort_report
                                            .deliver(AgentProgress::SubagentCompleted {
                                                agent_id: outcome.agent_id,
                                                task_id: outcome.task_id,
                                                elapsed_ms: outcome.elapsed.as_millis() as u64,
                                                iterations: outcome.iterations as u32,
                                                output_chars: outcome.output.chars().count(),
                                                output: outcome.output.clone(),
                                                // Detached by construction: this tool
                                                // takes `detached_child()`, so this child's
                                                // spend never reached the parent turn's
                                                // ledger or `chat_done`. See the field docs.
                                                usage: Some(outcome.usage),
                                                worktree_path: None,
                                                changed_files: Vec::new(),
                                                dirty_status: None,
                                                stop: None,
                                            })
                                            .await;
                                    }
                                }
                                SubagentRunStatus::Incomplete { ref reason } => {
                                    // Async sub-agent stopped short (stuck halt / iteration
                                    // cap). Mark the session finished and deliver the PARTIAL
                                    // progress back to the parent, framed so it is not
                                    // mistaken for a completed result (#4096).
                                    if let Err(err) = subagent_sessions::mark_finished(
                                        &background_store,
                                        &background_subagent_session_id,
                                        &outcome.task_id,
                                        &outcome.status,
                                        outcome.final_history.clone(),
                                    ) {
                                        log::warn!(
                                        "[subagent_reuse] mark_incomplete failed subagent_session_id={} task_id={} agent_id={} error={}",
                                        background_subagent_session_id,
                                        outcome.task_id,
                                        outcome.agent_id,
                                        err
                                    );
                                    }
                                    let framed = format!(
                                    "[SUBAGENT_INCOMPLETE] the sub-agent {reason} and did not finish. \
                             Partial progress:\n{}",
                                    outcome.output
                                );
                                    // An incomplete run may still have produced a full
                                    // proposal before stalling — preserve it durably too.
                                    if emit_lifecycle_effects {
                                        let framed = attach_workflow_proposal(
                                            &background_dispatch,
                                            &background_workspace_dir,
                                            background_parent_thread_id.as_deref(),
                                            &outcome.task_id,
                                            &outcome.agent_id,
                                            &outcome.final_history,
                                            framed,
                                        );
                                        completion_target
                                            .completed(&outcome.task_id, &outcome.agent_id, framed)
                                            .await;
                                        crate::agent::orchestration::subagent_events::publish_subagent_completed(
                            background_parent_session,
                            outcome.task_id.clone(),
                            outcome.agent_id.clone(),
                            outcome.elapsed.as_millis() as u64,
                            outcome.output.chars().count(),
                            outcome.iterations,
                        );
                                        abort_report
                                            .deliver(AgentProgress::SubagentCompleted {
                                                agent_id: outcome.agent_id,
                                                task_id: outcome.task_id,
                                                elapsed_ms: outcome.elapsed.as_millis() as u64,
                                                iterations: outcome.iterations as u32,
                                                output_chars: outcome.output.chars().count(),
                                                output: outcome.output.clone(),
                                                // Detached by construction: this tool
                                                // takes `detached_child()`, so this child's
                                                // spend never reached the parent turn's
                                                // ledger or `chat_done`. See the field docs.
                                                usage: Some(outcome.usage),
                                                worktree_path: None,
                                                changed_files: Vec::new(),
                                                dirty_status: None,
                                                stop: incomplete_stop(reason),
                                            })
                                            .await;
                                    }
                                }
                                SubagentRunStatus::Cancelled => {
                                    let error = "async sub-agent was cancelled".to_string();
                                    if let Err(err) = subagent_sessions::mark_finished(
                                        &background_store,
                                        &background_subagent_session_id,
                                        &outcome.task_id,
                                        &outcome.status,
                                        outcome.final_history.clone(),
                                    ) {
                                        log::warn!(
                                        "[subagent_reuse] mark_cancelled failed subagent_session_id={} task_id={} agent_id={} error={}",
                                        background_subagent_session_id,
                                        outcome.task_id,
                                        outcome.agent_id,
                                        err
                                    );
                                    }
                                    if emit_lifecycle_effects {
                                        completion_target
                                            .failed(&outcome.task_id, &outcome.agent_id, &error)
                                            .await;
                                        crate::agent::orchestration::subagent_events::publish_subagent_failed(
                                        background_parent_session,
                                        outcome.task_id.clone(),
                                        outcome.agent_id.clone(),
                                        error.clone(),
                                    );
                                        abort_report
                                            .deliver(AgentProgress::SubagentFailed {
                                                agent_id: outcome.agent_id,
                                                task_id: outcome.task_id,
                                                error,
                                            })
                                            .await;
                                    }
                                }
                                SubagentRunStatus::AwaitingUser {
                                    ref question,
                                    ref checkpoint,
                                    ..
                                } => {
                                    if let Err(err) = subagent_sessions::mark_finished(
                                        &background_store,
                                        &background_subagent_session_id,
                                        &outcome.task_id,
                                        &outcome.status,
                                        outcome.final_history.clone(),
                                    ) {
                                        log::warn!(
                                        "[subagent_reuse] mark_awaiting_user failed subagent_session_id={} task_id={} agent_id={} error={}",
                                        background_subagent_session_id,
                                        outcome.task_id,
                                        outcome.agent_id,
                                        err
                                    );
                                    }
                                    // #4896: a detached child that pauses for input won't continue
                                    // on its own — queue a framed notice so the parent chat learns it
                                    // needs input, not "Accepted". Same idle-gated path as a success.
                                    if emit_lifecycle_effects {
                                        completion_target
                                            .awaiting_input(
                                                &outcome.task_id,
                                                &outcome.agent_id,
                                                question,
                                                checkpoint.is_some(),
                                            )
                                            .await;
                                        crate::agent::orchestration::subagent_events::publish_subagent_awaiting_user(
                                        background_parent_session,
                                        outcome.task_id.clone(),
                                        outcome.agent_id.clone(),
                                        question.clone(),
                                    );
                                        abort_report
                                            .deliver(AgentProgress::SubagentAwaitingUser {
                                                agent_id: outcome.agent_id,
                                                task_id: outcome.task_id,
                                                question: question.clone(),
                                                worker_thread_id: background_worker_thread_id
                                                    .clone(),
                                                checkpoint_path: checkpoint
                                                    .as_ref()
                                                    .map(|path| path.to_string_lossy().to_string()),
                                            })
                                            .await;
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            let error = err.to_string();
                            if let Err(store_err) = subagent_sessions::mark_failed(
                                &background_store,
                                &background_subagent_session_id,
                                &background_task_id,
                                error.clone(),
                            ) {
                                log::warn!(
                                "[subagent_reuse] mark_failed failed subagent_session_id={} task_id={} agent_id={} error={}",
                                background_subagent_session_id,
                                background_task_id,
                                background_agent_id,
                                store_err
                            );
                            }
                            // #4896: a detached child that errors previously only
                            // published an event — nothing reached chat, so the parent
                            // turn finalized on "Accepted" and the failure was lost.
                            // Queue a framed failure notice so background_delivery
                            // surfaces it as a follow-up chat turn.
                            if emit_lifecycle_effects {
                                completion_target
                                    .failed(&background_task_id, &background_agent_id, &error)
                                    .await;
                                crate::agent::orchestration::subagent_events::publish_subagent_failed(
                                background_parent_session,
                                background_task_id.clone(),
                                background_agent_id.clone(),
                                error.clone(),
                            );
                                abort_report
                                    .deliver(AgentProgress::SubagentFailed {
                                        agent_id: background_agent_id,
                                        task_id: background_task_id,
                                        error,
                                    })
                                    .await;
                            }
                        }
                    }
                    abort_report.disarm();
                };
                match core {
                    Some(core) => {
                        crate::core::runtime::CoreContext::scope_with_turn_origin(
                            core,
                            origin,
                            Box::pin(finish),
                        )
                        .await
                    }
                    None => finish.await,
                }
            })
        };
        let background_prompt = add_background_contract(&prompt);
        let options = SubagentRunOptions {
            skill_filter_override: None,
            context,
            model_override,
            task_id: Some(task_id.clone()),
            thread_id: Some(background_thread_id),
            run_context: detached_parent.data.child(),
            worker_thread_id: background_worker_thread_id,
            initial_history,
            checkpoint_dir: None,
            worktree_action_dir: workspace_descriptor
                .as_ref()
                .map(|descriptor| descriptor.root.clone()),
            workspace_descriptor,
            run_queue: None,
            message_run_queue: None,
        };
        if let Err(error) = crate::agent::subagent_host::spawn_async_via_driver_tool(
            &detached_parent,
            definition.clone(),
            background_prompt,
            options,
            completion,
        )
        .await
        {
            if let Err(store_err) = subagent_sessions::mark_failed(
                &store,
                &durable_session.subagent_session_id,
                &task_id,
                error.to_string(),
            ) {
                log::warn!(
                    "[subagent_reuse] mark_failed after driver registration error subagent_session_id={} task_id={} agent_id={} error={}",
                    durable_session.subagent_session_id,
                    task_id,
                    definition.id,
                    store_err
                );
            }
            let message = error.to_string();
            registration_completion_target
                .failed(&registration_task_id, &registration_agent_id, &message)
                .await;
            crate::agent::orchestration::subagent_events::publish_subagent_failed(
                registration_parent_session,
                registration_task_id.clone(),
                registration_agent_id.clone(),
                message.clone(),
            );
            if let Some(progress) = registration_progress {
                let _ = progress
                    .send(AgentProgress::SubagentFailed {
                        agent_id: registration_agent_id,
                        task_id: registration_task_id,
                        error: message,
                    })
                    .await;
            }
            return Ok(ToolResult::error(format!(
                "spawn_async_subagent: failed to register detached sub-agent: {error}"
            )));
        }

        Ok(accepted_spawn_result(
            &task_id,
            &durable_session.subagent_session_id,
            &definition.id,
            worker_thread_id.as_deref(),
            reusable.is_some(),
            reuse_decision.as_str(),
            &fleet,
        ))
    }
}
