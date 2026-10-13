//! Per-job run policy: retry budget and single-flight.
//!
//! The policy is a field of the job itself (`CronJob::policy`,
//! [`JobPolicy`]), owned by `tinyflows-schedule` and stored with the job by
//! `CronDocuments`, so it is read, written and removed with the job. A job
//! whose policy is the default has none stored, so every existing job keeps its
//! behaviour:
//!
//! - `retries: None` uses `reliability.scheduler_retries`; `Some(0)` means
//!   exactly one attempt.
//! - `single_flight: false` is the scheduler's historical behaviour: a job is
//!   never dispatched twice by the scheduler, and a slot it misses while still
//!   running is skipped silently. `true` additionally records each missed slot
//!   as a `skipped` run and refuses a manual "Run now" while a run is in flight.
//!
//! Policies the host used to keep apart from the job (the `cron_job_policies`
//! table of `jobs.db`, then the `cron_job_policies` collection) are folded into
//! the job by the importer and by `CronDocuments` when it opens.

use anyhow::Result;
use tinyflows_schedule::CronJobPatch;
pub use tinyflows_schedule::JobPolicy;

use crate::config::Config;

/// Whether `error` is the store's "no such job" for `job_id`: its exact
/// outermost message, so a backend error that merely mentions "not found"
/// (a missing collection, say) stays on the fail-closed path.
fn is_missing(error: &anyhow::Error, job_id: &str) -> bool {
    error.to_string() == format!("Cron job '{job_id}' not found")
}

/// The policy stored for `job_id`; the default when there is no such job.
///
/// Fails closed: a store that errors is an error, so callers refuse to run the
/// job. The error names the job only; backend error text (which can carry
/// credentials or payloads) is dropped.
pub fn get_policy(config: &Config, job_id: &str) -> Result<JobPolicy> {
    match super::store::get_job(config, job_id) {
        Ok(job) => Ok(job.policy),
        Err(error) if is_missing(&error, job_id) => Ok(JobPolicy::default()),
        Err(_) => {
            tracing::warn!(job_id, "[cron:policy] policy unreadable; refusing to run");
            Err(anyhow::anyhow!(
                "cron policy for job {job_id} is unavailable; refusing to run it"
            ))
        }
    }
}

/// Store `policy` for `job_id`. The default policy is stored as none.
///
/// # Errors
///
/// When the job does not exist or the store fails.
pub fn set_policy(config: &Config, job_id: &str, policy: JobPolicy) -> Result<()> {
    tracing::debug!(job_id, ?policy, "[cron:policy] storing job policy");
    super::store::update_job(
        config,
        job_id,
        CronJobPatch {
            policy: Some(policy),
            ..CronJobPatch::default()
        },
    )?;
    Ok(())
}

/// Revert `job_id` to the default policy. A job that is gone has nothing to
/// revert.
pub fn clear_policy(config: &Config, job_id: &str) -> Result<()> {
    match set_policy(config, job_id, JobPolicy::default()) {
        Err(error) if is_missing(&error, job_id) => Ok(()),
        other => other,
    }
}

/// The retry budget for a job with `policy`.
pub fn effective_retries(config: &Config, policy: &JobPolicy) -> u32 {
    policy
        .retries
        .unwrap_or(config.reliability.scheduler_retries)
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
