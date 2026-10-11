use super::*;

/// A fresh profile folder under the system temporary directory.
fn folder(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "openhuman-browser-profile-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn a_folder_no_chrome_has_locked_is_free() {
    let path = folder("unlocked");
    assert!(!in_use(&path));
    assert!(!in_use(&path.join("missing")));
    let _ = std::fs::remove_dir_all(&path);
}

#[cfg(not(windows))]
#[test]
fn a_lock_naming_a_live_process_here_holds_the_folder() {
    let here = sysinfo::System::host_name().unwrap_or_default();
    let path = folder("live");
    std::os::unix::fs::symlink(
        format!("{here}-{}", std::process::id()),
        path.join("SingletonLock"),
    )
    .unwrap();
    assert!(in_use(&path));
    let _ = std::fs::remove_dir_all(&path);
}

#[cfg(not(windows))]
#[test]
fn a_lock_a_crashed_chrome_left_behind_is_free_and_another_hosts_is_held() {
    let here = sysinfo::System::host_name().unwrap_or_default();
    // No process has this id: the lock outlived its Chrome.
    let path = folder("stale");
    std::os::unix::fs::symlink(format!("{here}-4294967"), path.join("SingletonLock")).unwrap();
    assert!(!in_use(&path));
    let _ = std::fs::remove_dir_all(&path);

    let path = folder("elsewhere");
    std::os::unix::fs::symlink(
        format!("not-{here}-{}", std::process::id()),
        path.join("SingletonLock"),
    )
    .unwrap();
    assert!(in_use(&path), "a profile another computer holds is held");
    let _ = std::fs::remove_dir_all(&path);

    // A link with no process in it is no lock Chrome wrote.
    assert!(!held_by("no-number-here"));
    assert!(!held_by("nodash"));
}

#[cfg(not(windows))]
#[tokio::test]
async fn waiting_ends_as_soon_as_the_folder_is_let_go() {
    let here = sysinfo::System::host_name().unwrap_or_default();
    let path = folder("closing");
    let lock = path.join("SingletonLock");
    std::os::unix::fs::symlink(format!("{here}-{}", std::process::id()), &lock).unwrap();
    assert!(!free_within(&path, Duration::from_millis(300)).await);
    let letting_go = {
        let lock = lock.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            std::fs::remove_file(lock).unwrap();
        })
    };
    assert!(free_within(&path, PROFILE_WAIT).await);
    letting_go.await.unwrap();
    let _ = std::fs::remove_dir_all(&path);
}
