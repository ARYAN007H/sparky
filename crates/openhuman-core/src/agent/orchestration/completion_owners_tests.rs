use super::*;

fn owner(task_id: &str) -> DriverTaskOwner {
    let workspace_dir = PathBuf::from("/workspace");
    DriverTaskOwner {
        task_id: task_id.to_owned(),
        profile: None,
        workspace: tinytools::WorkspaceDescriptor::new(workspace_dir.clone()),
        workspace_dir,
        parent_session: "parent".into(),
        parent_thread_id: Some("thread".into()),
    }
}

#[test]
fn terminal_tasks_leave_the_non_evictable_live_owner_set() {
    let mut st = Owners::default();
    st.driver_tasks.insert("task".into(), owner("task"));

    terminalize_driver_task(&mut st, "task", 2);

    assert!(!st.driver_tasks.contains_key("task"));
    assert!(st.terminal_driver_tasks.contains_key("task"));
}

#[test]
fn active_owner_capacity_refuses_new_tasks_without_evicting_live_scope() {
    let mut st = Owners::default();
    for index in 0..ACTIVE_DRIVER_TASKS_CAP {
        let key = format!("task-{index}");
        st.driver_tasks.insert(key.clone(), owner(&key));
    }

    let result = remember_active_driver_task(
        &mut st,
        "new-task".into(),
        owner("new-task"),
        ACTIVE_DRIVER_TASKS_CAP,
    );

    assert!(result.is_err());
    assert!(st.driver_tasks.contains_key("task-0"));
    assert!(!st.driver_tasks.contains_key("new-task"));
}
