use super::*;

#[test]
fn explicit_scratch_workspace_cannot_activate_operator_user() {
    let _guard = crate::config::TEST_ENV_LOCK.blocking_lock();
    let previous = std::env::var_os("OPENHUMAN_WORKSPACE");
    let scratch = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("OPENHUMAN_WORKSPACE", scratch.path());
    }
    assert!(!operator_user_activation_allowed());
    match previous {
        Some(value) => unsafe { std::env::set_var("OPENHUMAN_WORKSPACE", value) },
        None => unsafe { std::env::remove_var("OPENHUMAN_WORKSPACE") },
    }
}

#[test]
fn first_user_activation_queues_prelogin_purge_in_the_removed_workspace() {
    use crate::memory::deletion::{pending, PendingDeletion};
    use crate::threads::store::{ensure_thread, list_threads, CreateConversationThread};

    let _guard = crate::config::TEST_ENV_LOCK.blocking_lock();
    let previous_home = std::env::var_os("HOME");
    let previous_workspace = std::env::var_os("OPENHUMAN_WORKSPACE");
    let tmp = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("HOME", tmp.path());
        std::env::set_var("OPENHUMAN_WORKSPACE", tmp.path());
    }

    let root = crate::config::default_root_openhuman_dir().unwrap();
    let prelogin_workspace = crate::config::pre_login_user_dir(&root).join("workspace");
    let active_workspace = crate::config::user_openhuman_dir(&root, "new-user").join("workspace");
    let ctx = crate::core::runtime::DispatchCtx::unbound();
    ensure_thread(
        &ctx,
        prelogin_workspace.clone(),
        CreateConversationThread {
            id: "prelogin-channel-thread".to_string(),
            title: "Pre-login thread".to_string(),
            created_at: "2026-10-18T00:00:00Z".to_string(),
            parent_thread_id: None,
            labels: None,
            personality_id: None,
            working_dir: None,
        },
    )
    .unwrap();

    activate_user_scope(&ctx, "new-user").unwrap();

    assert!(
        list_threads(&ctx, prelogin_workspace.clone())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        pending(&prelogin_workspace),
        vec![PendingDeletion::Thread {
            thread_id: "prelogin-channel-thread".to_string(),
        }]
    );
    assert!(pending(&active_workspace).is_empty());

    match previous_home {
        Some(value) => unsafe { std::env::set_var("HOME", value) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    match previous_workspace {
        Some(value) => unsafe { std::env::set_var("OPENHUMAN_WORKSPACE", value) },
        None => unsafe { std::env::remove_var("OPENHUMAN_WORKSPACE") },
    }
}
