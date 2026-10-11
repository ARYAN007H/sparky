//! Hotkey listener dispatch: rdev everywhere except the macOS Fn/Globe key,
//! which needs the Swift-based globe listener instead.

#[cfg(any(target_os = "macos", test))]
use std::time::Duration;

#[cfg(any(target_os = "macos", test))]
use log::warn;
use tokio_util::sync::CancellationToken;

use crate::voice::hotkey;

#[cfg(any(target_os = "macos", test))]
use super::LOG_PREFIX;

/// Opaque handle that keeps the hotkey listener alive. Drop to stop.
pub(super) enum HotkeyListenerKind {
    Rdev(hotkey::HotkeyListenerHandle),
    #[cfg(any(target_os = "macos", test))]
    Globe {
        cancel: CancellationToken,
        task: tokio::task::JoinHandle<()>,
    },
}

impl HotkeyListenerKind {
    pub(super) async fn stop(self) {
        match self {
            HotkeyListenerKind::Rdev(handle) => handle.stop(),
            #[cfg(any(target_os = "macos", test))]
            HotkeyListenerKind::Globe { cancel, task } => {
                cancel.cancel();
                let _ = task.await;
            }
        }
    }
}

/// Start the appropriate hotkey listener for the current platform and key.
///
/// On macOS, the Fn/Globe key is handled by the Swift-based globe listener
/// inside the TinyComputer module which monitors `NSEvent.flagsChanged`. All other
/// keys return an error on macOS: rdev's CGEventTap callback calls
/// `TSMGetInputSourceProperty` off the main thread; macOS 26 enforces
/// `dispatch_assert_queue(main_queue)` inside that API and kills the process
/// with `EXC_BREAKPOINT` (`dispatch_assert_queue_fail`). Configure
/// `hotkey = "fn"` to avoid this. (#2677)
pub(super) async fn start_hotkey_listener(
    config: &crate::config::Config,
    hotkey_str: &str,
    mode: hotkey::ActivationMode,
    server_cancel: &CancellationToken,
) -> Result<
    (
        HotkeyListenerKind,
        tokio::sync::mpsc::Receiver<hotkey::HotkeyEvent>,
    ),
    String,
> {
    #[cfg(target_os = "macos")]
    {
        if hotkey_str.trim().eq_ignore_ascii_case("fn") {
            return start_globe_hotkey_listener(config, mode, server_cancel).await;
        }
        // rdev calls TSMGetInputSourceProperty off the main thread; macOS 26
        // enforces main-queue-only access and crashes the process. Only the
        // Fn/Globe key is safe via the Swift globe listener. (#2677)
        Err(format!(
            "voice server hotkey '{}' is not supported on macOS — \
             only 'fn' (Fn/Globe key) is safe. rdev calls \
             TSMGetInputSourceProperty off the main thread on macOS 26, \
             causing EXC_BREAKPOINT. Set hotkey = \"fn\" in voice config \
             (issue #2677).",
            hotkey_str
        ))
    }

    // Non-macOS: rdev-based listener for all keys.
    #[cfg(not(target_os = "macos"))]
    {
        // `server_cancel` is only consumed by the macOS Swift-globe listener
        // branch above; the rdev listener manages its own lifecycle, so bind it
        // here to keep the shared signature warning-free on non-macOS.
        let _ = (config, server_cancel);
        let combo = hotkey::parse_hotkey(hotkey_str).map_err(|e| e.to_string())?;
        let (handle, mut source) =
            hotkey::start_listener(combo, mode).map_err(|e| e.to_string())?;
        let (tx, rx) = tokio::sync::mpsc::channel(65);
        tokio::spawn(async move {
            loop {
                let event = tokio::select! {
                    _ = tx.closed() => break,
                    event = source.recv() => match event { Some(event) => event, None => break },
                };
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        });
        Ok((HotkeyListenerKind::Rdev(handle), rx))
    }
}

/// Start an owned task before awaiting the module, so cancellation during startup
/// still leaves an owner to recover a lost start reply and close the native lease.
#[cfg(target_os = "macos")]
async fn start_globe_hotkey_listener(
    config: &crate::config::Config,
    mode: hotkey::ActivationMode,
    server_cancel: &CancellationToken,
) -> Result<
    (
        HotkeyListenerKind,
        tokio::sync::mpsc::Receiver<hotkey::HotkeyEvent>,
    ),
    String,
> {
    start_globe_with_client(
        crate::modules::accessibility::AccessibilityClient::new(config),
        mode,
        server_cancel,
    )
    .await
}

#[cfg(any(target_os = "macos", test))]
async fn start_globe_with_client(
    client: crate::modules::accessibility::AccessibilityClient,
    mode: hotkey::ActivationMode,
    server_cancel: &CancellationToken,
) -> Result<
    (
        HotkeyListenerKind,
        tokio::sync::mpsc::Receiver<hotkey::HotkeyEvent>,
    ),
    String,
> {
    static ADMISSION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let admission = tokio::time::timeout(Duration::from_secs(15), ADMISSION.lock())
        .await
        .map_err(|_| "previous Globe listener cleanup is still pending".to_string())?;
    let (tx, rx) = tokio::sync::mpsc::channel(65);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let cancel = server_cancel.child_token();
    let task_cancel = cancel.clone();
    let task = tokio::spawn(async move {
        let _admission = admission;
        let started = match client.globe_start().await {
            Ok(started) => started,
            Err(error) => {
                let recover = matches!(
                    error,
                    crate::modules::accessibility::AccessibilityError::Bus(
                        crate::modules::client::ModuleCallError::TransportFailed
                            | crate::modules::client::ModuleCallError::ModuleFault
                    )
                );
                let _ = ready_tx.send(Err(error.to_string()));
                if !recover {
                    return;
                }
                // Start is idempotent. Recover its lease before cleanup rather
                // than abandoning a native start whose reply was lost.
                loop {
                    match client.globe_start().await {
                        Ok(started) => {
                            close_globe(&client, started.handle).await;
                            return;
                        }
                        Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
                    }
                }
            }
        };
        let handle = started.handle;
        if !started.status.running {
            let _ = ready_tx.send(Err(started
                .status
                .last_error
                .unwrap_or_else(|| "Globe listener did not start".into())));
            close_globe(&client, handle).await;
            return;
        }
        if ready_tx.send(Ok(())).is_err() {
            close_globe(&client, handle).await;
            return;
        }
        let mut acknowledged_batch = None;
        let mut activation = GlobeActivation::default();
        let mut interval = tokio::time::interval(Duration::from_millis(50));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = task_cancel.cancelled() => break,
                _ = tx.closed() => break,
                _ = interval.tick() => {}
            }
            // One bounded module batch at a time. Wait for the host to consume
            // its queue before accepting another batch (at most 65 events).
            if tx.capacity() != 65 {
                continue;
            }
            let batch = match client
                .globe_read(crate::modules::accessibility::GlobeRead {
                    handle: handle.clone(),
                    acknowledged_batch,
                })
                .await
            {
                Ok(batch) => batch,
                Err(error) => {
                    warn!("{LOG_PREFIX} Globe read failed: {error}");
                    break;
                }
            };
            if batch.handle != handle
                || batch.events.len() > 64
                || batch.batch != acknowledged_batch.unwrap_or(0).saturating_add(1)
            {
                warn!("{LOG_PREFIX} invalid Globe batch");
                break;
            }
            if !batch.status.running {
                break;
            }
            if batch.overflow {
                if let Some(event) = activation.reset() {
                    if tx.send(event).await.is_err() {
                        break;
                    }
                }
            }
            let mut delivered = true;
            for event in batch.events {
                if let Some(event) = activation.event(event, mode) {
                    if tx.send(event).await.is_err() {
                        delivered = false;
                        break;
                    }
                }
            }
            if !delivered {
                break;
            }
            acknowledged_batch = Some(batch.batch);
        }
        close_globe(&client, handle).await;
    });
    // Dropping the startup future cancels the task's receiver; ready_tx then
    // fails and the task closes its lease before releasing admission.
    ready_rx
        .await
        .map_err(|_| "Globe startup task failed".to_string())??;
    Ok((HotkeyListenerKind::Globe { cancel, task }, rx))
}

/// Keep the admission guard while close fails; a successor cannot adopt a
/// listener with uncertain native cleanup. An already-retired lease is closed.
#[cfg(any(target_os = "macos", test))]
async fn close_globe(
    client: &crate::modules::accessibility::AccessibilityClient,
    handle: crate::modules::accessibility::GlobeHandle,
) {
    loop {
        match client.globe_stop(handle.clone()).await {
            Ok(_) => return,
            Err(crate::modules::accessibility::AccessibilityError::Operation(error))
                if error.code == "UNKNOWN_LISTENER" =>
            {
                return
            }
            Err(error) => warn!("{LOG_PREFIX} Globe cleanup pending: {error}"),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Host activation policy; an overflow requires a physical release before rearming.
#[cfg(any(target_os = "macos", test))]
#[derive(Default)]
struct GlobeActivation {
    active: bool,
    wait_release: bool,
}

#[cfg(any(target_os = "macos", test))]
impl GlobeActivation {
    fn reset(&mut self) -> Option<hotkey::HotkeyEvent> {
        self.wait_release = true;
        std::mem::take(&mut self.active).then_some(hotkey::HotkeyEvent::Released)
    }
    fn event(
        &mut self,
        event: tinycomputer_bus::accessibility::GlobeEvent,
        mode: hotkey::ActivationMode,
    ) -> Option<hotkey::HotkeyEvent> {
        use tinycomputer_bus::accessibility::GlobeEvent::{Down, Up};
        if self.wait_release {
            if event == Up {
                self.wait_release = false;
            }
            return None;
        }
        match (event, mode) {
            (Down, hotkey::ActivationMode::Tap) => {
                self.active = !self.active;
                Some(if self.active {
                    hotkey::HotkeyEvent::Pressed
                } else {
                    hotkey::HotkeyEvent::Released
                })
            }
            (Down, hotkey::ActivationMode::Push) if !self.active => {
                self.active = true;
                Some(hotkey::HotkeyEvent::Pressed)
            }
            (Up, hotkey::ActivationMode::Push) if self.active => {
                self.active = false;
                Some(hotkey::HotkeyEvent::Released)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "hotkey_listener_tests.rs"]
mod tests;
