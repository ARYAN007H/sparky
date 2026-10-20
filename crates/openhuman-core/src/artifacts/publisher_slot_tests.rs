use super::*;
use tiny_artifacts::{ArtifactRecord, ArtifactVersion, PublishOptions, PublishedLink};

struct FixedPublisher;

#[async_trait::async_trait]
impl ArtifactPublisher for FixedPublisher {
    async fn publish(
        &self,
        _record: &ArtifactRecord,
        _version: &ArtifactVersion,
        _options: &PublishOptions,
    ) -> anyhow::Result<PublishedLink> {
        anyhow::bail!("not used")
    }
}

#[test]
fn nothing_registered_offers_no_publisher() {
    assert!(artifact_publisher(&Config::default()).is_none());
}

#[test]
fn the_first_factory_that_answers_serves_sharing() {
    register_artifact_publisher("declines", |_| None);
    assert!(artifact_publisher(&Config::default()).is_none());
    let binder: ArtifactPublisherBinder = Arc::new(|_files| Arc::new(FixedPublisher));
    register_artifact_publisher("answers", move |_| Some(binder.clone()));
    assert!(artifact_publisher(&Config::default()).is_some());
    assert!(unregister_artifact_publisher("answers"));
    assert!(artifact_publisher(&Config::default()).is_none());
}

#[tokio::test]
async fn publisher_lookup_uses_the_carried_context_not_the_ambient_profile() {
    use crate::core::runtime::{ContextOverlay, CoreContext, DispatchCtx, DomainSet};
    use crate::tools::toolpacks::ToolGroups;

    let root = CoreContext::for_test(DomainSet::full(), None);
    let overlay = |profile: &'static str| {
        ContextOverlay::new(Config::default(), DomainSet::kernel(), ToolGroups::none())
            .profile(profile)
    };
    let core_a = root.derive_with(overlay("publisher-a"));
    let core_b = root.derive_with(overlay("publisher-b"));
    let ctx_a = DispatchCtx::new(core_a.clone());
    let ctx_b = DispatchCtx::new(core_b.clone());
    let binder: ArtifactPublisherBinder = Arc::new(|_files| Arc::new(FixedPublisher));
    register_artifact_publisher_in(core_a.extensions(), "publisher-context-a", move |_| {
        Some(binder.clone())
    });
    register_artifact_publisher_in(core_b.extensions(), "publisher-context-b-declines", |_| {
        None
    });

    CoreContext::scope(core_b, async {
        assert!(artifact_publisher_in(&Config::default(), &ctx_a).is_some());
        assert!(artifact_publisher_in(&Config::default(), &ctx_b).is_none());
    })
    .await;
}
