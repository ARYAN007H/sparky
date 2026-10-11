//! Whether a running Chrome holds a profile folder, and waiting a moment for
//! one that is closing to let it go.
//!
//! Chrome lets one browser at a time use a profile folder: a second one
//! started on it hands its window to the first and exits, so a browser task
//! or session given the persistent profile while another Chrome holds it
//! could not start its browser. A task waits up to [`PROFILE_WAIT`] for a
//! Chrome that is closing (the one the task before it closes as it ends), a
//! session up to [`SESSION_PROFILE_WAIT`], and otherwise each runs in a fresh
//! profile.

use std::path::Path;
use std::time::Duration;

/// How long a task waits for the Chrome holding its persistent profile to
/// let it go before it runs in a fresh profile instead.
pub(crate) const PROFILE_WAIT: Duration = Duration::from_secs(5);

/// How long a browser session waits for the same, before it opens in a
/// fresh profile: shorter than a task's, as a person waits on it, and the
/// Chrome holding the profile is most often another conversation's, which
/// does not close.
pub(crate) const SESSION_PROFILE_WAIT: Duration = Duration::from_secs(1);

/// How often the profile is looked at while waiting.
const PROFILE_POLL: Duration = Duration::from_millis(250);

/// Whether a running Chrome holds the profile folder `path`, by the lock it
/// keeps there: on macOS and Linux a `SingletonLock` link naming the host and
/// the process holding it, on Windows a `lockfile` held open. A link naming
/// another host is taken as held, as Chrome takes it; one naming a process
/// that is gone is a lock left by a crash, and free.
pub(crate) fn in_use(path: &Path) -> bool {
    #[cfg(windows)]
    {
        let lock = path.join("lockfile");
        lock.exists() && std::fs::OpenOptions::new().write(true).open(&lock).is_err()
    }
    #[cfg(not(windows))]
    {
        let Ok(target) = std::fs::read_link(path.join("SingletonLock")) else {
            return false;
        };
        held_by(&target.to_string_lossy())
    }
}

/// Whether a `SingletonLock` link's target, `<host>-<pid>`, names a live
/// process on this host, or another host.
#[cfg(not(windows))]
fn held_by(target: &str) -> bool {
    let Some((host, pid)) = target.rsplit_once('-') else {
        return false;
    };
    let Ok(pid) = pid.parse::<u32>() else {
        return false;
    };
    if sysinfo::System::host_name().is_some_and(|here| here != host) {
        return true;
    }
    let pid = sysinfo::Pid::from_u32(pid);
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    system.process(pid).is_some()
}

/// Waits up to `wait` for no Chrome to hold the profile folder `path`;
/// whether it is free.
pub(crate) async fn free_within(path: &Path, wait: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + wait;
    loop {
        if !in_use(path) {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(PROFILE_POLL).await;
    }
}

#[cfg(test)]
#[path = "browser_profile_tests.rs"]
mod tests;
