use super::*;

fn iteration() -> AgentProgress {
    AgentProgress::IterationStarted {
        iteration: 1,
        max_iterations: 5,
    }
}

fn model_replied() -> AgentProgress {
    AgentProgress::ModelCallCompleted {
        model: "m".into(),
        provider_id: "p".into(),
        subagent_task_id: None,
        input: None,
        output: None,
        iteration: 1,
        input_tokens: 0,
        output_tokens: 0,
        cached_input_tokens: 0,
        cache_creation_tokens: 0,
        reasoning_tokens: 0,
        cost_usd: None,
    }
}

fn kind(signal: &TurnSignal) -> &'static str {
    match signal {
        TurnSignal::WorkStarted => "started",
        TurnSignal::WorkFinished => "finished",
        TurnSignal::Progress(event) => match **event {
            AgentProgress::IterationStarted { .. } => "iteration",
            AgentProgress::ModelCallCompleted { .. } => "model",
            AgentProgress::TurnCompleted { .. } => "completed",
            _ => "other",
        },
    }
}

/// The tap forwards every event in order, brackets open model and tool calls
/// with the work signals the runner's idle guard needs, folds each event into
/// the activity record, and releases the drain once the parent completed.
#[tokio::test]
async fn the_tap_forwards_in_order_brackets_open_work_and_signals_the_drain() {
    let (agent_tx, agent_rx) = mpsc::channel(8);
    let (runner_tx, mut runner_rx) = mpsc::channel(16);
    let (drained_tx, mut drained_rx) = watch::channel(false);
    let activity = TurnActivity::default();
    let _tap = tap_progress(
        &DispatchCtx::ambient(),
        agent_rx,
        runner_tx,
        activity.clone(),
        drained_tx,
    );

    for event in [
        iteration(),
        model_replied(),
        AgentProgress::turn_completed(1),
    ] {
        agent_tx.send(event).await.expect("tap listens");
    }
    let mut seen = Vec::new();
    while seen.last() != Some(&"completed") {
        seen.push(kind(&runner_rx.recv().await.expect("signal")));
    }
    assert_eq!(
        seen,
        ["started", "iteration", "finished", "model", "completed"]
    );

    tokio::time::timeout(Duration::from_secs(5), drained_rx.wait_for(|done| *done))
        .await
        .expect("drained once the parent completed")
        .expect("watch open");
    assert_eq!(
        activity.stuck_point(),
        super::super::turn_activity::StuckPoint::AfterRun
    );
    drop(agent_tx);
}

/// A turn whose events stop without `TurnCompleted` (a failed turn) still
/// releases the drain when its stream ends.
#[tokio::test]
async fn the_drain_is_released_when_the_stream_ends_without_a_completion() {
    let (agent_tx, agent_rx) = mpsc::channel(8);
    let (runner_tx, mut runner_rx) = mpsc::channel(16);
    let (drained_tx, mut drained_rx) = watch::channel(false);
    let _tap = tap_progress(
        &DispatchCtx::ambient(),
        agent_rx,
        runner_tx,
        TurnActivity::default(),
        drained_tx,
    );
    agent_tx.send(AgentProgress::TurnStarted).await.unwrap();
    assert_eq!(kind(&runner_rx.recv().await.unwrap()), "other");
    assert!(!*drained_rx.borrow());
    drop(agent_tx);
    tokio::time::timeout(Duration::from_secs(5), drained_rx.wait_for(|done| *done))
        .await
        .expect("the closed stream drains")
        .expect("watch open");
}

#[tokio::test]
async fn a_dropped_runner_stops_the_tap() {
    let (agent_tx, agent_rx) = mpsc::channel(8);
    let (runner_tx, runner_rx) = mpsc::channel(1);
    let (drained_tx, drained_rx) = watch::channel(false);
    let tap = tap_progress(
        &DispatchCtx::ambient(),
        agent_rx,
        runner_tx,
        TurnActivity::default(),
        drained_tx,
    );
    drop(runner_rx);
    agent_tx.send(AgentProgress::TurnStarted).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), tap)
        .await
        .expect("the tap ends with its consumer")
        .expect("no panic");
    assert!(*drained_rx.borrow());
}

#[tokio::test]
async fn the_heartbeat_beats_only_for_a_live_turn() {
    use super::super::surface::TurnIds;
    let _module = crate::channels::module_fixture::client().await;
    let mut bus = crate::web_chat::subscribe_web_channel_events();
    let ids = TurnIds {
        client_id: "hb-client".into(),
        thread_id: "host-hb-thread".into(),
        request_id: "host-hb-request".into(),
    };
    let projection = Arc::new(AsyncMutex::new(
        ProgressProjection::new(
            &DispatchCtx::ambient(),
            ids,
            Default::default(),
            crate::config::Config::default(),
            std::sync::Arc::new(tinyagents_session::port::InMemoryTurnStates::default()),
        )
        .await,
    ));
    let beat = AbortOnDrop(spawn_heartbeat_with_interval(
        &DispatchCtx::ambient(),
        projection.clone(),
        Duration::from_millis(25),
    ));

    // Before `TurnStarted` the turn is not live: no beat, however long it waits.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(bus.try_recv().is_err());

    projection
        .lock()
        .await
        .on_event(AgentProgress::TurnStarted)
        .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let mut names = Vec::new();
    while let Ok(event) = bus.try_recv() {
        if event.context().thread_id == "host-hb-thread" {
            names.push(event.event_name());
        }
    }
    assert_eq!(names[0], "inference_start");
    assert!(names.len() >= 2, "{names:?}");
    assert!(names[1..].iter().all(|name| *name == "inference_heartbeat"));
    drop(beat);
}
