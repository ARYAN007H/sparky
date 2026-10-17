use super::*;

pub(super) const MIN_POLL_SECONDS: u64 = 5;

/// What the module answers when another run of the job holds its claim.
pub(crate) const ALREADY_RUNNING: &str = "job is already running";

/// Whether the host's cron poll loop is running in this process.
static LOOP_RUNNING: AtomicBool = AtomicBool::new(false);

/// The poll-loop slot: one loop per process, or two would each see the same
/// due rows. Released when dropped.
pub(super) struct LoopSlot;

impl LoopSlot {
    pub(super) fn acquire() -> Option<Self> {
        LOOP_RUNNING
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| Self)
    }
}

impl Drop for LoopSlot {
    fn drop(&mut self) {
        LOOP_RUNNING.store(false, Ordering::SeqCst);
    }
}

/// Whether a cron poll loop is running in this process.
pub fn is_running() -> bool {
    LOOP_RUNNING.load(Ordering::SeqCst)
}

/// Job ids a manual run (Run Now, the run tool) holds in this process.
static MANUAL_RUNS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Mutex::default);

/// A held manual run. Released on drop, including on panic or future
/// cancellation, so an aborted task can never lock a job id.
pub(crate) struct RunGuard(String);

impl Drop for RunGuard {
    fn drop(&mut self) {
        MANUAL_RUNS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.0);
    }
}

/// Claims `job_id` for a manual run, or `None` when another manual run holds
/// it. A scheduled run of the same job is refused later, by the module's own
/// claim (`ExecuteJob` answers [`ALREADY_RUNNING`]).
pub(crate) fn try_acquire_run(job_id: &str) -> Option<RunGuard> {
    MANUAL_RUNS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(job_id.to_string())
        .then(|| RunGuard(job_id.to_string()))
}
