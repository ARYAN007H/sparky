//! Project module-owned PollRun events onto OpenHuman events and Langfuse.

use serde_json::Value;
use tinyflows_bus::runs::{self, RunEvent};
use tinyflows_catalog::types::FlowRunTrigger;

use crate::core::events::DomainEvent;
use crate::flows::runs_adapter::session::RunSession;

pub(super) async fn project(session: &RunSession, event: &RunEvent) {
    match event.kind.as_str() {
        "started" => {
            if session.mark_started() {
                crate::core::bus::BUS.publish(DomainEvent::FlowRunStarted {
                    flow_id: session.flow_id().to_owned(),
                    run_id: session.run_id.clone(),
                });
            }
        }
        "resumed" => session.set_trigger(FlowRunTrigger::Resume),
        "step_started" => {
            if let Some(node_id) = event.data.get("node_id").and_then(Value::as_str) {
                publish_step(session, node_id, "running");
            }
        }
        "step" => {
            if let (Some(node_id), Some(status)) = (
                event.data.get("node_id").and_then(Value::as_str),
                event.data.get("status").and_then(Value::as_str),
            ) {
                publish_step(session, node_id, status);
            }
        }
        "graph_observation" => match serde_json::from_value::<tinyagents_graph::GraphObservation>(
            event.data.clone(),
        ) {
            Ok(observation) => session.add_observation(observation),
            Err(_) => tracing::warn!(
                target: "flows",
                run_id = %session.run_id,
                "[flows] PollRun graph observation could not be decoded for Langfuse"
            ),
        },
        "agent_progress" => {
            // The UI's existing workflow overlay is node-based. The detailed
            // agent events stay on the module stream; consume them here so the
            // shared cursor remains the sole owner of event ordering.
        }
        "parked" => {
            let parked = match serde_json::from_value::<runs::RunParked>(event.data.clone()) {
                Ok(parked) => parked,
                Err(_) => {
                    tracing::warn!(
                        target: "flows",
                        run_id = %session.run_id,
                        "[flows] PollRun parked outcome could not be decoded"
                    );
                    return;
                }
            };
            let flow = session.flow().await;
            crate::flows::ops::notify_pending_approval(
                &flow,
                &session.run_id,
                &parked.pending_approvals,
            );
            export_segment(session, "pending_approval").await;
        }
        "finished" => {
            let finished = match serde_json::from_value::<runs::RunFinished>(event.data.clone()) {
                Ok(finished) => finished,
                Err(_) => {
                    tracing::warn!(
                        target: "flows",
                        run_id = %session.run_id,
                        "[flows] PollRun terminal outcome could not be decoded"
                    );
                    return;
                }
            };
            if session.mark_finished() {
                crate::core::bus::BUS.publish(DomainEvent::FlowRunFinished {
                    flow_id: session.flow_id().to_owned(),
                    run_id: session.run_id.clone(),
                    status: finished.status.clone(),
                });
            }
            export_segment(session, &finished.status).await;
        }
        _ => {}
    }
}

fn publish_step(session: &RunSession, node_id: &str, status: &str) {
    crate::core::bus::BUS.publish(DomainEvent::FlowRunProgress {
        run_id: session.run_id.clone(),
        node_id: node_id.to_owned(),
        status: status.to_owned(),
    });
}

async fn export_segment(session: &RunSession, status: &str) {
    let observations = session.take_observations();
    let flow = session.flow().await;
    crate::flows::ops::export_run_to_langfuse(
        &session.config,
        &flow.name,
        &flow.id,
        &session.run_id,
        status,
        bus_trigger(session.trigger()),
        &observations,
    )
    .await;
}

fn bus_trigger(trigger: FlowRunTrigger) -> runs::FlowRunTrigger {
    match trigger {
        FlowRunTrigger::Rpc => runs::FlowRunTrigger::Rpc,
        FlowRunTrigger::Schedule => runs::FlowRunTrigger::Schedule,
        FlowRunTrigger::AppEvent => runs::FlowRunTrigger::AppEvent,
        FlowRunTrigger::Resume => runs::FlowRunTrigger::Resume,
    }
}
