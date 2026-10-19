//! Translation from product subagent outcomes to neutral orchestration records.

use super::*;

/// The host's in-flight subagent runs of one agent context.
pub(super) type HostInFlightMap = AsyncMutex<HashMap<HostLifecycleKey, Arc<HostInFlight>>>;

pub(super) const CLASSIFIED_STOP_CAUSE_EXTENSION: &str = "openhuman.classified_stop_cause";

pub(super) fn encode_classified_stop_cause(
    cause: &tinyagents_harness::no_progress::ClassifiedStopCause,
) -> serde_json::Value {
    let stop = crate::agent::turn_projection::breaker_stop_with_cause(cause.clone());
    serde_json::json!({
        "class": stop.failure_class.as_ref().map(|class| class.as_str()),
        "operation": stop.operation,
    })
}

pub(super) fn decode_classified_stop_cause(
    value: &serde_json::Value,
    summary: &str,
) -> Option<tinyagents_harness::no_progress::ClassifiedStopCause> {
    Some(tinyagents_harness::no_progress::ClassifiedStopCause {
        class: value.get("class")?.as_str()?.to_string(),
        operation: value.get("operation")?.as_str()?.to_string(),
        summary: summary.to_string(),
    })
}

pub(super) fn host_outcome_to_neutral(
    outcome: SubagentRunOutcome,
    definition: &crate::agent::harness::host_definition::AgentDefinition,
    options: &SubagentRunOptions,
) -> SubagentOutcome {
    let failure_cause = match &outcome.status {
        SubagentRunStatus::Incomplete { failure_cause, .. } => failure_cause.clone(),
        _ => None,
    };
    let status = match outcome.status {
        SubagentRunStatus::Completed => SubagentOutcomeKind::Completed,
        SubagentRunStatus::AwaitingUser { question, .. } => {
            SubagentOutcomeKind::AwaitingInput(SubagentPause {
                reason: question,
                resume: SubagentResume {
                    history: outcome
                        .final_history
                        .iter()
                        .map(crate::agent::message_convert::chat_message_to_message)
                        .collect(),
                    metadata: std::collections::BTreeMap::from_iter([
                        ("agent_id".into(), definition.id.clone()),
                        (
                            "worker_thread_id".into(),
                            options.worker_thread_id.clone().unwrap_or_default(),
                        ),
                        (
                            "skill_filter_override".into(),
                            options.skill_filter_override.clone().unwrap_or_default(),
                        ),
                        (
                            "model_override".into(),
                            options.model_override.clone().unwrap_or_default(),
                        ),
                    ]),
                    ..SubagentResume::default()
                },
            })
        }
        SubagentRunStatus::Incomplete { reason, .. } => {
            SubagentOutcomeKind::Incomplete(SubagentIncomplete::new(reason))
        }
        SubagentRunStatus::Cancelled => SubagentOutcomeKind::Cancelled,
    };
    let mut neutral = SubagentOutcome {
        task_id: outcome.task_id,
        output: outcome.output,
        history: outcome
            .final_history
            .iter()
            .map(crate::agent::message_convert::chat_message_to_message)
            .collect(),
        status,
        usage: UsageTotals {
            calls: outcome.iterations as u64,
            usage: Usage {
                input_tokens: outcome.usage.input_tokens,
                output_tokens: outcome.usage.output_tokens,
                total_tokens: outcome
                    .usage
                    .input_tokens
                    .saturating_add(outcome.usage.output_tokens),
                cache_read_tokens: outcome.usage.cached_input_tokens,
                // An unknown cost crosses as no charge at all, never as $0.
                charged_amount: outcome
                    .usage
                    .cost()
                    .usd()
                    .map(|usd| ChargedAmount::usd_micros((usd * 1_000_000.0).round() as i64)),
                ..Usage::default()
            },
        },
        artifacts: outcome
            .artifact_paths
            .into_iter()
            .map(|id| ArtifactReference {
                id,
                ..ArtifactReference::default()
            })
            .collect(),
        ..SubagentOutcome::cancelled(String::new())
    };
    if let Some(cause) = failure_cause {
        // Keep only the SDK-normalized class and operation; the human-readable
        // summary may contain private tool or URL details.
        neutral.extensions.insert(
            CLASSIFIED_STOP_CAUSE_EXTENSION.into(),
            encode_classified_stop_cause(&cause),
        );
    }
    neutral
}
