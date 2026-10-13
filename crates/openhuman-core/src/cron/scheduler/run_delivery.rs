//! The host's `RunDelivery` and `TriggerSink` ports for the schedule runtime.
//!
//! Delivery sends a finished run's output to its chat, channel or origin and
//! raises the alert (`delivery.rs`), then announces the completion on the bus.
//! The trigger sink fires a `flow` row's schedule trigger directly in the
//! flows domain, or runs a host-owned system job, with no domain event in
//! between.

use super::delivery::deliver_run;
use super::host::Binding;
use crate::core::bus::BUS;
use crate::core::events::DomainEvent;
use anyhow::Result;
use async_trait::async_trait;
use tinyflows_schedule::runtime::{Attempt, RunDelivery, TriggerSink};
use tinyflows_schedule::types::{CronJob, DeliveryStatus};

pub(super) struct HostDelivery {
    binding: Binding,
}

impl HostDelivery {
    pub(super) fn new(binding: Binding) -> Self {
        Self { binding }
    }
}

#[async_trait]
impl RunDelivery for HostDelivery {
    async fn deliver(
        &self,
        job: &CronJob,
        run_id: &str,
        attempt: &Attempt,
    ) -> Result<DeliveryStatus> {
        self.binding
            .within(async {
                let config = self.binding.config();
                let result =
                    deliver_run(&config, job, run_id, &attempt.output, attempt.success).await;
                // A failed required delivery fails the run, as the runtime
                // records it.
                let delivered = match &result {
                    Ok(status) => *status != DeliveryStatus::Failed || job.delivery.best_effort,
                    Err(_) => job.delivery.best_effort,
                };
                let success = attempt.success && delivered;
                // The job ran as its agent (when it had one); a one-shot job
                // is deleted once the run completes, so subscribers learn the
                // owner here, not from the store.
                crate::cron::completion_owner::note(&job.id);
                BUS.publish(DomainEvent::CronJobCompleted {
                    job_id: job.id.clone(),
                    success,
                    output: crate::util::truncate_with_ellipsis(&attempt.output, 512),
                });
                BUS.publish(DomainEvent::HealthChanged {
                    component: "scheduler".to_string(),
                    healthy: success,
                    message: (!success).then(|| {
                        if attempt.output.is_empty() {
                            format!("job {} failed", job.id)
                        } else {
                            crate::util::truncate_with_ellipsis(&attempt.output, 256)
                        }
                    }),
                });
                result
            })
            .await
    }
}

pub(super) struct HostTriggers {
    binding: Binding,
}

impl HostTriggers {
    pub(super) fn new(binding: Binding) -> Self {
        Self { binding }
    }
}

#[async_trait]
impl TriggerSink for HostTriggers {
    async fn schedule(&self, flow_id: &str, run_id: &str) -> Result<Attempt> {
        self.binding
            .within(async {
                if let Some(name) = crate::cron::system_jobs::system_command_name(flow_id) {
                    return Ok(run_system_job(flow_id, name, run_id).await);
                }
                tracing::info!(
                    target: "flows",
                    %flow_id,
                    %run_id,
                    "[cron] flow schedule trigger fired"
                );
                #[cfg(feature = "flows")]
                crate::flows::bus::dispatch_schedule_tick(&self.binding.config(), flow_id).await;
                #[cfg(not(feature = "flows"))]
                tracing::warn!(%flow_id, "[cron] built without `flows`; schedule trigger dropped");
                Ok(Attempt {
                    success: true,
                    output: format!("flow schedule tick emitted for flow {flow_id}"),
                    retryable: true,
                })
            })
            .await
    }
}

/// A host-owned system job (`system:<name>`): announced on the bus, then its
/// registered handler is awaited and its result is the run's result. Without a
/// handler the job is announce-only and `ok` on dispatch.
async fn run_system_job(command: &str, name: &str, run_id: &str) -> Attempt {
    tracing::info!(%command, job = %name, %run_id, "[cron] system job due — publishing CronSystemJobDue");
    BUS.publish(DomainEvent::CronSystemJobDue {
        job: name.to_string(),
    });
    let ctx = crate::cron::system_job_handlers::SystemJobContext {
        job_id: run_id.to_string(),
        name: name.to_string(),
    };
    let (success, output) = match crate::cron::system_job_handlers::dispatch(ctx).await {
        Some(Ok(())) => (true, format!("system job {name} completed")),
        Some(Err(error)) => (false, format!("system job {name} failed: {error}")),
        None => (true, format!("system job {name} dispatched")),
    };
    Attempt {
        success,
        output,
        retryable: true,
    }
}
