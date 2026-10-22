use super::*;
use crate::agent::orchestration::delivery_drain::DrainFence;
use std::sync::Arc;
use std::time::Duration;
use tinyagents_tasks::{
    CompletionRecord, CompletionResult, CompletionStatus, InMemoryCompletionStore,
};

fn delivery_target(router: &tinyagents_tasks::CompletionRouter) -> DeliveryTarget<'_> {
    DeliveryTarget {
        router,
        parent_key: "parent-thread",
        tenant_key: "profile",
        thread_key: "profile:parent-thread",
    }
}

fn immediate_delivery(router: &tinyagents_tasks::CompletionRouter) -> CompletionDelivery {
    CompletionDelivery::new(
        CompletionDeliveryConfig {
            debounce: Duration::ZERO,
            recovery_delay: Duration::ZERO,
            retry_backoff: Duration::ZERO,
            busy_poll: Duration::ZERO,
            max_busy_polls: 0,
            batch_size: 32,
        },
        ThreadDeliveryLeases::default(),
    )
}

#[tokio::test]
async fn upstream_delivery_claims_and_acknowledges_one_parent_batch() {
    let router = tinyagents_tasks::CompletionRouter::new(Arc::new(InMemoryCompletionStore::new()));
    for (task_id, text) in [("child-a", "A"), ("child-b", "B")] {
        router
            .record(CompletionRecord::new(
                task_id,
                "parent-thread",
                "researcher",
                CompletionStatus::Success,
                CompletionResult::text(text),
            ))
            .await
            .unwrap();
    }

    let delivery = immediate_delivery(&router);
    let mut delivered = Vec::new();
    let outcome = delivery
        .deliver_once(
            delivery_target(&router),
            || false,
            || true,
            |batch| {
                delivered = batch.iter().map(|record| record.task_id.clone()).collect();
                async { Ok::<(), &'static str>(()) }
            },
            |_| {},
        )
        .await;

    assert_eq!(outcome, DeliveryOutcome::Delivered { count: 2 });
    assert_eq!(delivered, ["child-a", "child-b"]);
    assert!(router.pending_for("parent-thread").is_empty());
}

#[tokio::test]
async fn upstream_delivery_reports_parent_busy_without_claiming_completions() {
    let router = tinyagents_tasks::CompletionRouter::new(Arc::new(InMemoryCompletionStore::new()));
    router
        .record(CompletionRecord::new(
            "child-busy",
            "parent-thread",
            "researcher",
            CompletionStatus::Success,
            CompletionResult::text("done"),
        ))
        .await
        .unwrap();

    let delivery = immediate_delivery(&router);
    let outcome = delivery
        .deliver_once(
            delivery_target(&router),
            || true,
            || true,
            |_| async { Ok::<(), &'static str>(()) },
            |_| {},
        )
        .await;

    assert_eq!(outcome, DeliveryOutcome::ParentBusy);
    assert_eq!(router.pending_for("parent-thread")[0].attempts, 0);
}

#[tokio::test]
async fn production_drain_uses_the_upstream_path_for_a_thread_without_pending_work() {
    let ctx = crate::core::runtime::DispatchCtx::ambient();
    let thread_id = format!("unmapped-{}", uuid::Uuid::new_v4());

    let outcome = try_deliver(ctx, thread_id, DrainFence::none(), false).await;

    assert_eq!(outcome, DeliveryOutcome::Empty);
}
