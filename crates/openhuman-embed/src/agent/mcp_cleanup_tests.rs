use super::*;

fn pending(home: std::path::PathBuf) -> PendingCleanup {
    PendingCleanup {
        cleanup: None,
        instance: Weak::new(),
        home,
        purge: true,
        retry_running: Default::default(),
    }
}

#[test]
fn confirmed_cleanup_purges_before_releasing_a_dead_agent_reservation() {
    let scratch = tempfile::tempdir().unwrap();
    let home = scratch.path().join("worker");
    std::fs::create_dir(&home).unwrap();
    std::fs::write(home.join("kept-until-cleanup"), "fixture").unwrap();
    let pending = pending(home.clone());
    let registry = crate::runtime::AgentMap::default();
    {
        let mut agents = registry.lock().unwrap();
        agents.insert("worker".into(), pending.instance.clone());
        agents.pending.insert("worker".into(), pending.clone());
    }
    finish_removal(&registry, "worker", &pending).unwrap();
    assert!(!home.exists());
    let agents = registry.lock().unwrap();
    assert!(!agents.contains_key("worker"));
    assert!(!agents.pending.contains_key("worker"));
}

#[test]
fn a_failed_purge_keeps_the_dead_agent_reservation_and_purge_intent() {
    let scratch = tempfile::tempdir().unwrap();
    let home = scratch.path().join("worker");
    std::fs::write(&home, "a file cannot be purged as a directory").unwrap();
    let pending = pending(home.clone());
    let registry = crate::runtime::AgentMap::default();
    {
        let mut agents = registry.lock().unwrap();
        agents.insert("worker".into(), pending.instance.clone());
        agents.pending.insert("worker".into(), pending.clone());
    }
    assert!(matches!(
        finish_removal(&registry, "worker", &pending),
        Err(AgentError::Workspace { .. })
    ));
    assert!(home.exists());
    let agents = registry.lock().unwrap();
    assert!(agents.contains_key("worker"));
    assert!(agents.pending.get("worker").unwrap().purge);
}

#[test]
fn an_already_released_reservation_does_not_purge_a_home() {
    let scratch = tempfile::tempdir().unwrap();
    let home = scratch.path().join("worker");
    std::fs::create_dir(&home).unwrap();
    finish_removal(
        &crate::runtime::AgentMap::default(),
        "worker",
        &pending(home.clone()),
    )
    .unwrap();
    assert!(home.exists());
}

#[test]
fn the_last_runtime_guard_keeps_admission_reserved_through_cleanup_completion() {
    let executor = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .thread_stack_size(openhuman_core::core::runtime::AGENT_WORKER_STACK_BYTES)
        .build()
        .unwrap();
    executor.block_on(async {
        tokio::spawn(async {
            let runtime = crate::Runtime::builder()
                .config(openhuman_core::config::Config::default())
                .workspace(crate::Workspace::Ephemeral)
                .build()
                .await
                .expect("a real isolated runtime can be built");
            let guard = runtime.guard();
            let weak = Arc::downgrade(&guard);
            drop(runtime);
            let (entered_tx, entered_rx) = std::sync::mpsc::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let worker = std::thread::spawn(move || {
                let mut guard = Some(guard);
                let result = complete_with_guard(&mut guard, || {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Err::<(), ()>(())
                });
                result.is_err() && guard.is_some()
            });
            entered_rx.recv().unwrap();
            let guard_is_live = weak.upgrade().is_some();
            let admission_is_reserved =
                crate::runtime::RUNTIME_LIVE.load(std::sync::atomic::Ordering::Acquire);
            release_tx.send(()).unwrap();
            let retry_guard_is_retained = worker.join().unwrap();
            assert!(retry_guard_is_retained, "a failed completion may retry and must retain runtime admission until the task ends");
            assert!(
                guard_is_live,
                "completion must retain the last real workspace/runtime guard"
            );
            assert!(
                admission_is_reserved,
                "another runtime must remain excluded while cleanup completion is blocked"
            );
            assert!(weak.upgrade().is_none());
            assert!(!crate::runtime::RUNTIME_LIVE.load(std::sync::atomic::Ordering::Acquire));
        })
        .await
        .unwrap();
    });
}

#[test]
fn exhausted_cleanup_releases_runtime_admission_but_preserves_the_ephemeral_workspace() {
    let executor = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .thread_stack_size(openhuman_core::core::runtime::AGENT_WORKER_STACK_BYTES)
        .build()
        .unwrap();
    executor.block_on(async {
        tokio::spawn(async {
            let runtime = crate::Runtime::builder()
                .config(openhuman_core::config::Config::default())
                .workspace(crate::Workspace::Ephemeral)
                .build()
                .await
                .unwrap();
            let root = runtime.workspace_dir().to_path_buf();
            let home = root.join("agents/unresolved");
            std::fs::create_dir_all(&home).unwrap();
            std::fs::write(home.join("module-state"), "must survive unconfirmed close").unwrap();
            let guard = runtime.guard();
            let weak = Arc::downgrade(&guard);
            drop(runtime);
            let attempts = std::sync::atomic::AtomicUsize::new(0);
            retry_cleanup(
                guard,
                || {
                    attempts.fetch_add(1, Ordering::SeqCst);
                    async {
                        Err(AgentError::McpCleanup(
                            openhuman_core::modules::client::ModuleCallError::ModuleFault,
                        ))
                    }
                },
                || panic!("unconfirmed close must never purge"),
            )
            .await;
            let home_was_preserved = home.join("module-state").exists();
            let _ = std::fs::remove_dir_all(&root);
            assert_eq!(attempts.load(Ordering::SeqCst), 3);
            assert!(
                weak.upgrade().is_none(),
                "exhaustion must release the process guard"
            );
            assert!(!crate::runtime::RUNTIME_LIVE.load(Ordering::Acquire));
            assert!(
                home_was_preserved,
                "failed cleanup must preserve the ephemeral workspace after the last guard drops"
            );
        })
        .await
        .unwrap();
    });
}

fn workspace_cleanup_control(cancel: bool) {
    let executor = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .thread_stack_size(openhuman_core::core::runtime::AGENT_WORKER_STACK_BYTES)
        .build()
        .unwrap();
    executor.block_on(async {
        tokio::spawn(async move {
            let runtime = crate::Runtime::builder()
                .config(openhuman_core::config::Config::default())
                .workspace(crate::Workspace::Ephemeral)
                .build()
                .await
                .unwrap();
            let root = runtime.workspace_dir().to_path_buf();
            let guard = runtime.guard();
            let weak = Arc::downgrade(&guard);
            drop(runtime);
            let cleanup = retry_cleanup(guard, || async { Ok(()) }, || Ok(()));
            if cancel {
                // Executor rejection or an unpolled task must preserve its home.
                drop(cleanup);
            } else {
                cleanup.await;
            }
            let workspace_exists = root.exists();
            let _ = std::fs::remove_dir_all(&root);
            assert!(weak.upgrade().is_none());
            assert!(!crate::runtime::RUNTIME_LIVE.load(Ordering::Acquire));
            assert_eq!(
                workspace_exists, cancel,
                "only confirmed cleanup may remove the ephemeral workspace"
            );
        })
        .await
        .unwrap();
    });
}

#[test]
fn confirmed_cleanup_still_removes_the_last_owners_ephemeral_workspace() {
    workspace_cleanup_control(false);
}

#[test]
fn cancelling_an_unpolled_cleanup_preserves_its_ephemeral_workspace() {
    workspace_cleanup_control(true);
}
