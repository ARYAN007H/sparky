use super::*;

fn reservation() -> ChannelRuntimeReservation {
    let handle = "0123456789abcdef0123456789abcdef".to_string();
    ChannelRuntimeReservation {
        object_path: format!("{OBJECT_PATH}/runtime_{handle}"),
        host_object_path: format!("{HOST_OBJECT_PATH}/runtime_{handle}"),
        handle,
    }
}

#[test]
fn private_paths_must_belong_to_the_reserved_runtime() {
    assert!(validate_reservation(&reservation()).is_ok());
    for path in [
        HOST_OBJECT_PATH.to_string(),
        format!("{HOST_OBJECT_PATH}/runtime_ffffffffffffffffffffffffffffffff"),
    ] {
        let mut value = reservation();
        value.host_object_path = path;
        assert_eq!(
            validate_reservation(&value),
            Err(ModuleCallError::ModuleFault)
        );
    }
    let mut value = reservation();
    value.object_path = OBJECT_PATH.to_string();
    assert_eq!(
        validate_reservation(&value),
        Err(ModuleCallError::ModuleFault)
    );
}

#[test]
fn malformed_handles_cannot_replace_another_callback() {
    for handle in [
        "",
        "../secret",
        "0123456789ABCDEF0123456789ABCDEF",
        "0123456789abcdef0123456789abcdef/other",
    ] {
        let mut value = reservation();
        value.handle = handle.to_string();
        assert_eq!(
            validate_reservation(&value),
            Err(ModuleCallError::ModuleFault)
        );
    }
}

#[tokio::test]
async fn cancelled_close_wait_retains_the_original_cleanup_worker() {
    let closing = Arc::new(Mutex::new(None));
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let acknowledged = Arc::new(AtomicBool::new(false));
    let waiter = tokio::spawn({
        let closing = closing.clone();
        let started = started.clone();
        let release = release.clone();
        let acknowledged = acknowledged.clone();
        async move {
            join_cleanup(&closing, || tokio::spawn(async move {
                started.notify_one();
                release.notified().await;
                acknowledged.store(true, Ordering::Release);
                Ok(())
            })).await
        }
    });
    started.notified().await;
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    assert!(!acknowledged.load(Ordering::Acquire));
    assert!(closing.lock().await.is_some());
    release.notify_one();
    join_cleanup(&closing, || panic!("must reuse the retained worker")).await.unwrap();
    assert!(acknowledged.load(Ordering::Acquire));
    assert!(closing.lock().await.is_none());
}

#[tokio::test]
async fn a_failed_cleanup_can_be_retried_without_aborting_pending_work() {
    let closing = Mutex::new(None);
    assert_eq!(join_cleanup(&closing, || tokio::spawn(async {
        Err(ModuleCallError::TransportFailed)
    })).await, Err(ModuleCallError::TransportFailed));
    assert!(closing.lock().await.is_none());
    join_cleanup(&closing, || tokio::spawn(async { Ok(()) })).await.unwrap();
}
