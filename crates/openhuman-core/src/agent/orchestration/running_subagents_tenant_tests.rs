use super::*;
use crate::core::runtime::{ContextOverlay, CoreContext, DomainSet, Mode};
use crate::tools::toolpacks::ToolGroups;

const CHILD: &str = "OPENHUMAN_DETACHED_TENANT_TEST_CHILD";
const PARENT: &str = "same-parent-session";

// Mode is process-owned and irreversible. Each assertion runs in a fresh
// libtest process, without changing the mode of unrelated desktop tests.
fn child(name: &str, mode: Mode) -> bool {
    let module = module_path!().split_once("::").expect("crate module").1;
    let test = format!("{module}::{name}");
    if std::env::var(CHILD).as_deref() == Ok(test.as_str()) {
        crate::core::runtime::mode::lock_mode(mode).expect("lock child mode");
        return true;
    }
    let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", &test, "--nocapture", "--test-threads=1"])
        .env(CHILD, &test)
        .output()
        .expect("run isolated mode test");
    assert!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "isolated test {test} failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

fn profile(name: &str, workspace: &std::path::Path) -> Arc<CoreContext> {
    CoreContext::for_test(DomainSet::full(), None).derive_with(
        ContextOverlay::new(
            crate::config::Config {
                workspace_dir: workspace.to_path_buf(),
                ..crate::config::Config::default()
            },
            DomainSet::kernel(),
            ToolGroups::none(),
        )
        .profile(name)
        .session_agent("same-agent"),
    )
}

struct Worker {
    task: String,
    session: String,
    queue: Arc<RunQueue<QueuedTurn>>,
    abort: AbortHandle,
    _status: watch::Sender<DetachedSubagentStatus>,
}

impl Worker {
    fn register(workspace: &std::path::Path) -> Self {
        let task = uuid::Uuid::new_v4().to_string();
        let session = uuid::Uuid::new_v4().to_string();
        let queue = run_queue();
        let (tx, rx) = status_channel();
        let task_handle = tokio::spawn(std::future::pending::<()>());
        let abort = task_handle.abort_handle();
        register(
            task.clone(),
            "researcher".into(),
            PARENT.into(),
            None,
            Some(session.clone()),
            workspace.to_path_buf(),
            None,
            queue.clone(),
            abort.clone(),
            rx,
        );
        Self {
            task,
            session,
            queue,
            abort,
            _status: tx,
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        prune(&self.task);
        self.abort.abort();
    }
}

struct Pair {
    _a_dir: tempfile::TempDir,
    _b_dir: tempfile::TempDir,
    alice: Arc<CoreContext>,
    bob: Arc<CoreContext>,
    a: Worker,
    b: Worker,
}

impl Pair {
    fn new() -> Self {
        let a_dir = tempfile::tempdir().unwrap();
        let b_dir = tempfile::tempdir().unwrap();
        let alice = profile("detached-alice", a_dir.path());
        let bob = profile("detached-bob", b_dir.path());
        let a = CoreContext::sync_scope(alice.clone(), || Worker::register(a_dir.path()));
        let b = CoreContext::sync_scope(bob.clone(), || Worker::register(b_dir.path()));
        Self {
            _a_dir: a_dir,
            _b_dir: b_dir,
            alice,
            bob,
            a,
            b,
        }
    }
}

async fn control(function: &str, task: &str) -> serde_json::Value {
    let controller = crate::agent::orchestration::subagent_control::all_registered_controllers()
        .into_iter()
        .find(|c| c.schema.function == function)
        .expect("public controller");
    let mut params = serde_json::Map::new();
    params.insert("taskId".into(), serde_json::json!(task));
    if function == "steer" {
        params.insert("message".into(), serde_json::json!("ownership regression"));
    }
    (controller.handler)(params)
        .await
        .expect("control response")
}

fn field(value: &serde_json::Value, key: &str) -> Option<bool> {
    value
        .get("data")
        .and_then(|v| v.get(key))
        .or_else(|| value.get(key))
        .and_then(serde_json::Value::as_bool)
}

#[tokio::test]
async fn same_parent_rosters_are_profile_isolated() {
    if !child("same_parent_rosters_are_profile_isolated", Mode::Saas) {
        return;
    }
    let pair = Pair::new();
    for (ctx, own) in [(&pair.alice, &pair.a), (&pair.bob, &pair.b)] {
        let tasks = CoreContext::sync_scope(ctx.clone(), || snapshot_for_parent(PARENT));
        assert_eq!(
            tasks.iter().map(|t| t.task_id.as_str()).collect::<Vec<_>>(),
            [own.task.as_str()]
        );
    }
}

#[tokio::test]
async fn same_parent_resolution_and_wait_refuse_foreign_workers() {
    if !child(
        "same_parent_resolution_and_wait_refuse_foreign_workers",
        Mode::Saas,
    ) {
        return;
    }
    let pair = Pair::new();
    CoreContext::scope(pair.bob.clone(), async {
        assert!(matches!(
            task_id_for_session(&pair.a.session, PARENT),
            Err(WaitError::NotOwned)
        ));
        assert!(matches!(
            resume_ref_for_task(&pair.a.task, PARENT),
            Err(WaitError::NotOwned)
        ));
        assert!(matches!(
            wait(&pair.a.task, PARENT, Duration::ZERO).await,
            Err(WaitError::NotOwned)
        ));
        assert_eq!(
            task_id_for_session(&pair.b.session, PARENT).unwrap(),
            pair.b.task
        );
    })
    .await;
}

#[tokio::test]
async fn same_parent_agent_steering_refuses_foreign_worker() {
    if !child(
        "same_parent_agent_steering_refuses_foreign_worker",
        Mode::Saas,
    ) {
        return;
    }
    let pair = Pair::new();
    CoreContext::scope(pair.bob.clone(), async {
        assert_eq!(
            steer(&pair.a.task, PARENT, "foreign".into(), QueueLane::Steer).await,
            Err(SteerError::NotOwned)
        );
        assert_eq!(pair.a.queue.status().await.steers, 0);
        steer(&pair.b.task, PARENT, "own".into(), QueueLane::Steer)
            .await
            .unwrap();
        assert_eq!(pair.b.queue.status().await.steers, 1);
    })
    .await;
}

#[tokio::test]
async fn public_steering_refuses_another_profiles_task() {
    if !child("public_steering_refuses_another_profiles_task", Mode::Saas) {
        return;
    }
    let pair = Pair::new();
    CoreContext::scope(pair.bob.clone(), async {
        assert_eq!(
            field(&control("steer", &pair.a.task).await, "steered"),
            Some(false)
        );
        assert_eq!(pair.a.queue.status().await.steers, 0);
        assert_eq!(
            field(&control("steer", &pair.b.task).await, "steered"),
            Some(true)
        );
        assert_eq!(pair.b.queue.status().await.steers, 1);
    })
    .await;
}

#[tokio::test]
async fn public_cancellation_refuses_another_profiles_task() {
    if !child(
        "public_cancellation_refuses_another_profiles_task",
        Mode::Saas,
    ) {
        return;
    }
    let pair = Pair::new();
    CoreContext::scope(pair.bob.clone(), async {
        assert_eq!(
            field(&control("cancel", &pair.a.task).await, "cancelled"),
            Some(false)
        );
        assert!(resume_ref_for_task(&pair.b.task, PARENT).is_ok());
        assert_eq!(
            field(&control("cancel", &pair.b.task).await, "cancelled"),
            Some(true)
        );
    })
    .await;
    CoreContext::scope(pair.alice.clone(), async {
        assert!(
            resume_ref_for_task(&pair.a.task, PARENT).is_ok(),
            "foreign cancel leaves original worker live"
        );
    })
    .await;
}

#[tokio::test]
async fn missing_saas_scope_cannot_observe_or_control_workers() {
    if !child(
        "missing_saas_scope_cannot_observe_or_control_workers",
        Mode::Saas,
    ) {
        return;
    }
    let pair = Pair::new();
    assert!(CoreContext::scoped().is_none());
    assert!(snapshot_for_parent(PARENT).is_empty());
    assert!(matches!(
        resume_ref_for_task(&pair.a.task, PARENT),
        Err(WaitError::NotOwned)
    ));
    assert_eq!(
        steer(&pair.a.task, PARENT, "unscoped".into(), QueueLane::Steer).await,
        Err(SteerError::NotOwned)
    );
    assert_eq!(
        field(&control("steer", &pair.a.task).await, "steered"),
        Some(false)
    );
    assert_eq!(
        field(&control("cancel", &pair.a.task).await, "cancelled"),
        Some(false)
    );
    assert_eq!(pair.a.queue.status().await.steers, 0);
    CoreContext::sync_scope(pair.alice.clone(), || {
        assert!(resume_ref_for_task(&pair.a.task, PARENT).is_ok())
    });
    // A scoped kernel context without an admitted profile is not authority.
    CoreContext::scope(CoreContext::for_test(DomainSet::full(), None), async {
        assert!(snapshot_for_parent(PARENT).is_empty());
        assert!(matches!(
            resume_ref_for_task(&pair.a.task, PARENT),
            Err(WaitError::NotOwned)
        ));
        assert_eq!(
            field(&control("steer", &pair.a.task).await, "steered"),
            Some(false)
        );
        assert_eq!(
            field(&control("cancel", &pair.a.task).await, "cancelled"),
            Some(false)
        );
    })
    .await;
}

#[tokio::test]
async fn desktop_public_controls_remain_available_without_parent_scope() {
    if !child(
        "desktop_public_controls_remain_available_without_parent_scope",
        Mode::SingleUser,
    ) {
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let worker = Worker::register(directory.path());
    assert_eq!(snapshot_for_parent(PARENT).len(), 1);
    assert_eq!(
        field(&control("steer", &worker.task).await, "steered"),
        Some(true)
    );
    assert_eq!(worker.queue.status().await.steers, 1);
    assert_eq!(
        field(&control("cancel", &worker.task).await, "cancelled"),
        Some(true)
    );
}
