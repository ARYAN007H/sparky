use super::*;

#[tokio::test]
async fn queued_terminal_delivery_does_not_leave_a_completion_entry() {
    let shared = NativeShared::default();
    shared.begin_delivery_wait("queued-request");
    shared.abandon_delivery_wait("queued-request");

    // The queued request returns to its caller before the active runner later
    // reaches its terminal event. The event still has to release the slot.
    shared.with("queued-request", |_| {});
    shared.record_delivery("queued-request", true);

    assert!(shared
        .slots
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_empty());
}

#[tokio::test]
async fn active_terminal_delivery_is_returned_to_its_waiting_caller() {
    let shared = NativeShared::default();
    shared.begin_delivery_wait("active-request");
    shared.record_delivery("active-request", true);

    assert_eq!(shared.take_delivery("active-request"), Some(true));
    assert!(shared
        .slots
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_empty());
}

#[tokio::test]
async fn terminal_delivery_releases_session_profile_and_busy_guards() {
    use crate::agent::host_agents::HostAgent;
    use crate::core::runtime::{ContextOverlay, CoreContext, DispatchCtx, DomainSet};

    fn request(
        dispatch: &DispatchCtx,
        session: &str,
        profile: &str,
        request_id: &str,
    ) -> TurnRequest<QueuedPayload> {
        let core = dispatch.core().expect("profile context").clone();
        let definition = crate::agent::harness::host_definition::AgentDefinitionRegistry::
            builtins_only()
            .get("orchestrator")
            .cloned()
            .expect("built-in orchestrator definition");
        let trusted = Arc::new(crate::agent::turn_host::TrustedAgentTurn {
            host_agent: HostAgent {
                definition,
                config: crate::config::Config::default(),
                host_tools: None,
                context: core,
                hooks: Default::default(),
            },
            channel: "telegram".into(),
            session_key: session.into(),
            event_session_id: format!("event-{session}"),
            origin: crate::agent::turn_origin::AgentTurnOrigin::ExternalChannel {
                sender_name: Some("Ada".into()),
                channel: "telegram".into(),
                sender: Some("sender".into()),
                reply_target: "chat".into(),
                message_id: request_id.into(),
                history_key: None,
                channel_session: None,
            },
            approval: Default::default(),
            relay_thread: None,
            imported_history: Vec::new(),
            native_target: None,
        });
        TurnRequest {
            scope: ScopeKey {
                agent_id: "orchestrator".into(),
                profile_id: Some(profile.into()),
            },
            thread_id: session.into(),
            request_id: request_id.into(),
            text: "hello".into(),
            payload: QueuedPayload {
                ctx: Some(dispatch.clone()),
                trusted_turn: Some(trusted),
                ..Default::default()
            },
            host_authored: false,
            disposition: QueueDisposition::Interrupt,
            deadline: None,
            idle_timeout: None,
        }
    }

    let profile = format!("native-runner-profile-{}", uuid::Uuid::new_v4());
    let session = format!("native-runner-session-{}", uuid::Uuid::new_v4());
    let core = CoreContext::for_test(DomainSet::full(), None).derive_with(
        ContextOverlay::new(
            crate::config::Config::default(),
            DomainSet::full(),
            Default::default(),
        )
        .profile(profile.clone()),
    );
    let dispatch = DispatchCtx::new(core);
    let first_shared = Arc::new(NativeShared::default());
    let first = NativeHost {
        shared: Arc::clone(&first_shared),
    };
    let first_request = request(&dispatch, &session, &profile, "native-first");
    let first_session = first
        .checkout(&first_request)
        .await
        .expect("first native checkout");
    assert_eq!(first_session.session_id, session);

    let second_shared = Arc::new(NativeShared::default());
    let second = NativeHost {
        shared: Arc::clone(&second_shared),
    };
    let second_request = request(&dispatch, &session, &profile, "native-cancelled");
    let waiting = tokio::spawn(async move { second.checkout(&second_request).await });
    tokio::task::yield_now().await;
    assert!(
        !waiting.is_finished(),
        "a second runner instance waits on the checked-out durable session"
    );
    waiting.abort();
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(1), waiting)
            .await
            .expect("cancelled checkout completes")
            .expect_err("checkout task was cancelled")
            .is_cancelled()
    );

    first_shared.record_delivery("native-first", true);
    let third_request = request(&dispatch, &session, &profile, "native-third");
    let third = NativeHost {
        shared: Arc::clone(&second_shared),
    };
    let third_session = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        third.checkout(&third_request),
    )
    .await
    .expect("terminal delivery releases the session/profile guards")
    .expect("a later runner can check out the session");
    assert_eq!(third_session.session_id, session);
    second_shared.record_delivery("native-third", true);
}
