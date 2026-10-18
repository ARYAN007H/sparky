//! The host's adapter between a web-chat turn and TinyChannels' web projection
//! module. Canonical output crosses TinyBus; the host retains event delivery,
//! tenant routing, and web-chat-specific lifecycle policy.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use sha2::Digest as _;
use tinychannels_bus::module_progressive::ChannelOutputEvent;
use tinychannels_bus::web as bus_web;
use tinychannels_bus::module_web_output::{BeginWebTurn, EndWebTurn, ProjectWebOutput, WebTurnHandle};

use super::event_bus_channel::publish_web_channel_event_in;
use super::WebChannelEvent;
use crate::config::Config;
use crate::core::runtime::DispatchCtx;

/// Addressing of one turn on the web wire.
#[derive(Clone, Debug)]
pub(crate) struct TurnIds {
    pub(crate) client_id: String,
    pub(crate) thread_id: String,
    pub(crate) request_id: String,
}

struct SurfaceState {
    module: tinybus::module::ModuleClient,
    handle: Option<WebTurnHandle>,
    closed: bool,
}

/// One module-owned projection turn plus its host broadcast destination.
#[derive(Clone)]
pub(crate) struct Surface {
    state: Arc<tokio::sync::Mutex<SurfaceState>>,
    ctx: DispatchCtx,
    scope_key: String,
    ids: TurnIds,
    /// The round of the narration the next `chat_interim` carries. The
    /// canonical `Clarification` output has no round, and the frontend keys one
    /// interim bubble per round, so the host supplies it here.
    interim_round: Arc<AtomicU32>,
}

impl Surface {
    /// A surface for the turn `ids`, publishing under `ctx`. Every canonical
    /// projection goes through the compiled TinyChannels module.
    pub(crate) async fn new(ctx: &DispatchCtx, ids: TurnIds, config: Config) -> Self {
        #[cfg(test)]
        let module = {
            let _ = config;
            crate::channels::module_fixture::client().await
        };
        #[cfg(not(test))]
        let module = crate::modules::client(config);

        let tenant = super::ops::tenant_of(ctx);
        let scope_bytes = serde_json::to_vec(&(
            tenant,
            &ids.client_id,
            &ids.thread_id,
            &ids.request_id,
        ))
        .expect("web turn scope tuple is serializable");
        let scope_key = format!("web-chat:{}", hex::encode(sha2::Sha256::digest(scope_bytes)));

        Self {
            state: Arc::new(tokio::sync::Mutex::new(SurfaceState {
                module,
                handle: None,
                closed: false,
            })),
            ctx: ctx.clone(),
            scope_key,
            ids,
            interim_round: Arc::new(AtomicU32::new(0)),
        }
    }

    async fn begin_if_needed(&self, state: &mut SurfaceState) -> Result<String, String> {
        if state.closed {
            return Err("web projection turn is already closed".to_owned());
        }
        if let Some(handle) = &state.handle {
            return Ok(handle.handle.clone());
        }
        let response: WebTurnHandle = state
            .module
            .call_confidential(
                "tinychannels",
                tinychannels_bus::methods::BEGIN_WEB_TURN,
                (BeginWebTurn {
                    scope_key: self.scope_key.clone(),
                    context: self.context(),
                },),
            )
            .await
            .map_err(|error| error.to_string())?;
        let handle = response.handle.clone();
        state.handle = Some(response);
        Ok(handle)
    }

    async fn end_if_needed(&self, state: &mut SurfaceState) {
        state.closed = true;
        let Some(handle) = state.handle.take() else {
            return;
        };
        if let Err(error) = state
            .module
            .call_confidential::<()>(
                "tinychannels",
                tinychannels_bus::methods::END_WEB_TURN,
                (EndWebTurn {
                    handle: handle.handle,
                },),
            )
            .await
        {
            log::warn!(
                "[web-channel] projection turn cleanup failed request_id={}: {error}",
                self.ids.request_id
            );
        }
    }

    fn publish(&self, event: bus_web::WebChannelEvent) {
        let mut flat = WebChannelEvent::from(event);
        if flat.event == "chat_interim" && flat.round.is_none() {
            flat.round = Some(self.interim_round.load(Ordering::Relaxed)).filter(|r| *r > 0);
        }
        publish_web_channel_event_in(&self.ctx, flat);
    }

    /// Project one canonical output event onto the web wire, in module-owned
    /// sequence order. Module errors fail closed; the host never reimplements
    /// the projection locally.
    pub(crate) async fn output(&self, event: ChannelOutputEvent) {
        let mut state = self.state.lock().await;
        if state.closed {
            log::debug!(
                "[web-channel] dropping output after terminal request_id={}",
                self.ids.request_id
            );
            return;
        }
        let handle = match self.begin_if_needed(&mut state).await {
            Ok(handle) => handle,
            Err(error) => {
                log::warn!(
                    "[web-channel] projection turn could not begin request_id={}: {error}",
                    self.ids.request_id
                );
                return;
            }
        };
        let terminal = matches!(
            &event,
            ChannelOutputEvent::FinalMessage { .. }
                | ChannelOutputEvent::ChatError { .. }
                | ChannelOutputEvent::ChatCancelled { .. }
        );
        let projected: tinychannels_bus::module_web_output::WebOutputEvents = match state
            .module
            .call_confidential(
                "tinychannels",
                tinychannels_bus::methods::PROJECT_WEB_OUTPUT,
                (ProjectWebOutput { handle, event },),
            )
            .await
        {
            Ok(projected) => projected,
            Err(error) => {
                log::warn!(
                    "[web-channel] output event not projected request_id={}: {error}",
                    self.ids.request_id
                );
                return;
            }
        };
        for event in projected.events {
            self.publish(event);
        }
        if terminal {
            self.end_if_needed(&mut state).await;
        }
    }

    /// The leading narration of `round`, flushed as an interim bubble.
    pub(crate) async fn interim(&self, round: u32, text: String) {
        self.interim_round.store(round, Ordering::Relaxed);
        self.output(ChannelOutputEvent::Clarification { prompt: text })
            .await;
    }

    /// Publish a host-authored event unchanged (queue, goal, todo, or the
    /// enriched terminal event). It has no canonical projection form, but it
    /// still closes any module-owned projection state for the turn.
    pub(crate) async fn event(&self, event: bus_web::WebChannelEvent) {
        let terminal = matches!(
            &event,
            bus_web::WebChannelEvent::ChatDone { .. }
                | bus_web::WebChannelEvent::ChatError { .. }
                | bus_web::WebChannelEvent::ChatCancelled { .. }
        );
        let mut state = self.state.lock().await;
        self.publish(event);
        if terminal {
            self.end_if_needed(&mut state).await;
        }
    }

    /// The addressing context for an event of this turn.
    pub(crate) fn context(&self) -> bus_web::WebEventContext {
        bus_web::WebEventContext {
            client_id: self.ids.client_id.clone(),
            thread_id: self.ids.thread_id.clone(),
            request_id: self.ids.request_id.clone(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
#[path = "surface_tests.rs"]
mod tests;
