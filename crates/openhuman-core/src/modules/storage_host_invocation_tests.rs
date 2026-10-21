use super::*;
use crate::storage::StorageBackend;
use tinystoragedrivers_busproxy::{Object, StorageCall};

fn endpoint() -> (DocumentsOnlyEndpoint, BusName) {
    let backend = crate::storage::test_support::memory_backend();
    let scope = crate::storage::Scope::new("local").unwrap();
    let storage = backend.for_scope(&scope).unwrap();
    let expected_peer = BusName::new(":1.7").unwrap();
    (
        DocumentsOnlyEndpoint {
            expected_peer: expected_peer.clone(),
            active: Arc::new(AtomicBool::new(true)),
            server: StorageServer::new(storage),
        },
        expected_peer,
    )
}

async fn call(
    endpoint: &DocumentsOnlyEndpoint,
    sender: Option<&BusName>,
    object: Object,
    selector: &str,
    request: Value,
) -> Result<Value, HostFailure> {
    endpoint
        .call(StorageCall {
            object,
            selector,
            request,
            sender,
            admission: Admission::Private,
        })
        .await
}

#[tokio::test]
async fn callback_authenticates_before_metadata_and_restricts_ports() {
    let (endpoint, peer) = endpoint();
    let attacker = BusName::new(":1.8").unwrap();

    assert_eq!(
        call(
            &endpoint,
            Some(&attacker),
            Object::Streams,
            "Capabilities",
            json!(null),
        )
        .await,
        Err(HostFailure::Denied("wrong-module-instance"))
    );
    assert_eq!(
        call(&endpoint, None, Object::Blobs, "Capabilities", json!(null),).await,
        Err(HostFailure::Denied("unauthenticated-caller"))
    );

    let stream_caps = call(
        &endpoint,
        Some(&peer),
        Object::Streams,
        "Capabilities",
        json!(null),
    )
    .await
    .unwrap();
    assert_eq!(stream_caps, json!({ "capabilities": [] }));
    assert_eq!(
        call(
            &endpoint,
            Some(&peer),
            Object::Blobs,
            "Get",
            json!({"key": "secret"}),
        )
        .await,
        Err(HostFailure::Denied("documents-only-invocation"))
    );
    assert_eq!(
        call(
            &endpoint,
            Some(&peer),
            Object::Secrets,
            "Capabilities",
            json!(null),
        )
        .await,
        Err(DocumentsOnlyEndpoint::unsupported())
    );
}

#[tokio::test]
async fn callback_documents_route_uses_the_captured_storage() {
    let (endpoint, peer) = endpoint();

    let result = call(
        &endpoint,
        Some(&peer),
        Object::Documents,
        "Get",
        json!({"collection": "juice.savings", "document_id": "current"}),
    )
    .await;

    assert!(matches!(
        result,
        Err(HostFailure::Storage(StorageFailure {
            kind: StorageKind::NotFound
        }))
    ));
}

#[tokio::test]
async fn dropped_invocation_refuses_calls_already_in_flight_at_endpoint() {
    let (endpoint, peer) = endpoint();
    endpoint.active.store(false, Ordering::Release);

    assert_eq!(
        call(
            &endpoint,
            Some(&peer),
            Object::Documents,
            "Capabilities",
            json!(null),
        )
        .await,
        Err(HostFailure::Denied("storage-endpoint-closed"))
    );
}
