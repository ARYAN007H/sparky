use super::*;
use crate::core::runtime::DispatchCtx;

fn ids(request_id: &str) -> TurnIds {
    TurnIds {
        client_id: "surface-client".into(),
        thread_id: format!("surface-thread-{request_id}"),
        request_id: request_id.into(),
    }
}

async fn next_for(
    rx: &mut tokio::sync::broadcast::Receiver<WebChannelEvent>,
    request_id: &str,
) -> WebChannelEvent {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match rx.recv().await {
                Ok(event) if event.request_id == request_id => return event,
                Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(error) => panic!("bus closed: {error}"),
            }
        }
    })
    .await
    .expect("event within timeout")
}

#[tokio::test]
async fn output_events_keep_the_turns_ids_and_one_increasing_seq() {
    let mut rx = crate::web_chat::subscribe_web_channel_events();
    let surface = Surface::new(
        &DispatchCtx::ambient(),
        ids("surface-seq"),
        crate::config::Config::default(),
    )
    .await;
    for text in ["a", "b"] {
        surface
            .output(ChannelOutputEvent::TextDelta {
                text: text.into(),
                iteration: Some(2),
            })
            .await;
    }
    let first = next_for(&mut rx, "surface-seq").await;
    let second = next_for(&mut rx, "surface-seq").await;
    assert_eq!(first.event, "text_delta");
    assert_eq!(first.client_id, "surface-client");
    assert_eq!(first.thread_id, "surface-thread-surface-seq");
    assert_eq!((first.delta.as_deref(), first.round), (Some("a"), Some(2)));
    assert_eq!((first.seq, second.seq), (Some(1), Some(2)));
    assert!(first.ts.is_some(), "every event carries a wall-clock stamp");
}

#[tokio::test]
async fn an_interim_bubble_carries_the_round_the_canonical_output_lacks() {
    let mut rx = crate::web_chat::subscribe_web_channel_events();
    let surface = Surface::new(
        &DispatchCtx::ambient(),
        ids("surface-interim"),
        crate::config::Config::default(),
    )
    .await;
    surface.interim(3, "Let me check.".into()).await;
    let event = next_for(&mut rx, "surface-interim").await;
    assert_eq!(event.event, "chat_interim");
    assert_eq!(event.full_response.as_deref(), Some("Let me check."));
    assert_eq!(event.round, Some(3));
}

#[tokio::test]
async fn host_authored_events_pass_through_without_a_seq() {
    let mut rx = crate::web_chat::subscribe_web_channel_events();
    let surface = Surface::new(
        &DispatchCtx::ambient(),
        ids("surface-terminal"),
        crate::config::Config::default(),
    )
    .await;
    surface
        .event(bus_web::WebChannelEvent::ChatCancelled {
            context: surface.context(),
            cancel_reason: Some("user_stop".into()),
            superseded_by: None,
        })
        .await;
    let event = next_for(&mut rx, "surface-terminal").await;
    assert_eq!(event.event, "chat_cancelled");
    assert_eq!(event.cancel_reason.as_deref(), Some("user_stop"));
    assert_eq!(event.seq, None);
}

#[tokio::test]
async fn module_projected_cancellation_closes_the_turn() {
    let mut rx = crate::web_chat::subscribe_web_channel_events();
    let surface = Surface::new(
        &DispatchCtx::ambient(),
        ids("surface-cancel"),
        crate::config::Config::default(),
    )
    .await;
    surface
        .output(ChannelOutputEvent::Cancellation {
            reason: Some("user_stop".into()),
        })
        .await;

    let event = next_for(&mut rx, "surface-cancel").await;
    assert_eq!(event.event, "chat_cancelled");
    assert_eq!(event.seq, Some(1));
    let state = surface.state.lock().await;
    assert!(state.closed);
    assert!(state.handle.is_none());
}

#[tokio::test]
async fn dropping_a_surface_ends_its_module_owned_turn() {
    let surface = Surface::new(
        &DispatchCtx::ambient(),
        ids("surface-drop-cleanup"),
        crate::config::Config::default(),
    )
    .await;
    surface
        .output(ChannelOutputEvent::TextDelta {
            text: "active".into(),
            iteration: Some(1),
        })
        .await;
    let (module, handle) = {
        let state = surface.state.lock().await;
        (
            state.module.clone(),
            state.handle.as_ref().unwrap().handle.clone(),
        )
    };
    let cleanup_done = Arc::clone(&surface.cleanup_done);
    drop(surface);
    tokio::time::timeout(std::time::Duration::from_secs(5), cleanup_done.notified())
        .await
        .expect("drop cleanup completes");

    let result: Result<tinychannels_bus::module_web_output::WebOutputEvents, _> = module
        .call_confidential(
            "tinychannels",
            tinychannels_bus::methods::PROJECT_WEB_OUTPUT,
            (ProjectWebOutput {
                handle,
                event: ChannelOutputEvent::TextDelta {
                    text: "late".into(),
                    iteration: Some(1),
                },
            },),
        )
        .await;
    assert!(
        result.is_err(),
        "a dropped turn must reject later projections"
    );
}

#[tokio::test]
async fn the_addressing_context_names_the_turn() {
    let surface = Surface::new(
        &DispatchCtx::ambient(),
        ids("surface-context"),
        crate::config::Config::default(),
    )
    .await;
    let context = surface.context();
    assert_eq!(
        (context.client_id.as_str(), context.request_id.as_str()),
        ("surface-client", "surface-context")
    );
    assert!(context.seq.is_none() && context.ts.is_none());
}
