//! Progress for a running shell call.
//!
//! A shell call can legitimately run for many minutes (a build, a test suite,
//! a download), and until it exits the turn shows nothing. While it runs,
//! [`with_shell_progress`] reports a status line through the harness's
//! [`ToolRunContext::report_progress`] seam every [`HEARTBEAT_INTERVAL`]: the
//! elapsed time, how much output the command has written, and its last output
//! line. The agent loop turns each report into `AgentEvent::ToolProgressDetail`
//! for the call, which the web channel projects as `tool_progress`.
//!
//! The output figures come from [`ShellOutputMeter`], which
//! `shell_exec::collect_process` feeds with every chunk it reads from the
//! process. Paths that do not stream output (the Docker backend) still report
//! the elapsed time.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tinytools::{ToolProgress, ToolRunContext};

/// How often a running shell call reports that it is still working.
pub(crate) const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

/// Longest last-output line carried in a status line, in characters.
const LAST_LINE_CHARS: usize = 120;

#[derive(Debug, Default)]
struct Meter {
    bytes: u64,
    last_line: Option<String>,
}

/// Output written so far by the shell call running on this task. Cheap to
/// clone; every clone sees the same figures.
#[derive(Clone, Debug, Default)]
pub(crate) struct ShellOutputMeter(Arc<Mutex<Meter>>);

impl ShellOutputMeter {
    /// Fold one chunk of stdout or stderr into the figures.
    pub(crate) fn record(&self, chunk: &[u8]) {
        let mut meter = self.0.lock().unwrap_or_else(|p| p.into_inner());
        meter.bytes = meter.bytes.saturating_add(chunk.len() as u64);
        let text = String::from_utf8_lossy(chunk);
        if let Some(line) = text.lines().map(str::trim).rfind(|line| !line.is_empty()) {
            meter.last_line = Some(line.chars().take(LAST_LINE_CHARS).collect());
        }
    }

    fn snapshot(&self) -> (u64, Option<String>) {
        let meter = self.0.lock().unwrap_or_else(|p| p.into_inner());
        (meter.bytes, meter.last_line.clone())
    }
}

tokio::task_local! {
    static CURRENT_METER: ShellOutputMeter;
}

/// The meter of the shell call running on this task, if it reports progress.
/// Read it before spawning a worker task: a task-local does not follow
/// `tokio::spawn`.
pub(crate) fn current_meter() -> Option<ShellOutputMeter> {
    CURRENT_METER.try_with(Clone::clone).ok()
}

/// The status line for a call `elapsed` into its run.
pub(crate) fn progress_message(elapsed: Duration, bytes: u64, last_line: Option<&str>) -> String {
    let secs = elapsed.as_secs();
    let elapsed = if secs >= 60 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else {
        format!("{secs}s")
    };
    if bytes == 0 {
        return format!("running {elapsed} · no output yet");
    }
    let size = if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    };
    match last_line {
        Some(line) => format!("running {elapsed} · {size} output · last line: {line}"),
        None => format!("running {elapsed} · {size} output"),
    }
}

/// Run the shell call `fut`, reporting its progress through `context` every
/// `interval` until it finishes. Without a context there is nowhere to report,
/// so `fut` runs as is.
pub(crate) async fn with_shell_progress<F: std::future::Future>(
    context: Option<&dyn ToolRunContext>,
    interval: Duration,
    fut: F,
) -> F::Output {
    let Some(context) = context else {
        return fut.await;
    };
    let meter = ShellOutputMeter::default();
    let started = tokio::time::Instant::now();
    // Boxed: the shell call's future is large, and this wrapper would
    // otherwise hold it inline a second time on the tool's stack.
    let mut work = Box::pin(CURRENT_METER.scope(meter.clone(), fut));
    let mut ticker = tokio::time::interval_at(tokio::time::Instant::now() + interval, interval);
    loop {
        tokio::select! {
            output = &mut work => return output,
            _ = ticker.tick() => {
                let (bytes, last_line) = meter.snapshot();
                let message = progress_message(started.elapsed(), bytes, last_line.as_deref());
                tracing::debug!(%message, "[shell] reporting progress");
                context.report_progress(ToolProgress::message(message));
            }
        }
    }
}

#[cfg(test)]
#[path = "shell_progress_tests.rs"]
mod tests;
