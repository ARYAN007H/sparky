use crate::core::runtime::DispatchCtx;
use super::*;
use crate::core::runtime::{CoreContext, DomainSet, SaasConfig};

#[tokio::test]
async fn a_host_without_a_shared_backend_sweeps_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let host = ProfileHost::new(
        SaasConfig::new(tmp.path()),
        CoreContext::for_test(DomainSet::full(DispatchCtx::process_default()None),
    );
    assert_eq!(sweep(&DispatchCtx::process_default(), &host, Utc::now()).await, SweepReport::default());
}
