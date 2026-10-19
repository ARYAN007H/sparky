use super::*;

#[cfg(not(feature = "modules"))]
#[tokio::test]
async fn permissions_without_a_loader_are_explicitly_unavailable() {
    assert!(matches!(
        AccessibilityClient::new(&Config::default()).permissions().await,
        Err(AccessibilityError::Bus(ModuleCallError::Unavailable))
    ));
}

#[cfg(feature = "modules")]
#[derive(Clone)]
struct Fixture(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

#[cfg(feature = "modules")]
#[tinybus::interface(name = "test.Accessibility")]
impl Fixture {
    async fn accessibility_permissions(&self) -> tinybus::Result<DesktopResponse> {
        self.0.lock().unwrap().push("permissions".into());
        Ok(DesktopResponse::ok(
            "accessibility-permissions",
            serde_json::to_value(PermissionStatus {
                accessibility: PermissionState::Unsupported,
                input_monitoring: PermissionState::Unsupported,
                microphone: PermissionState::Granted,
            })
            .unwrap(),
        ))
    }

    async fn accessibility_request_permission(
        &self,
        kind: PermissionKind,
    ) -> tinybus::Result<DesktopResponse> {
        assert_eq!(kind, PermissionKind::Microphone);
        self.0.lock().unwrap().push("request".into());
        self.accessibility_permissions().await
    }
}

#[cfg(feature = "modules")]
async fn fixture() -> (AccessibilityClient, Fixture, tinybus::Connection) {
    use tinybus::{broker::Broker, transport::memory::MemoryBus, Connection, ObjectPath};
    let bus = MemoryBus::new();
    Broker::new().spawn(bus.clone());
    let service = Connection::connect(bus.connect().await.unwrap())
        .await
        .unwrap();
    let caller = Connection::connect(bus.connect().await.unwrap())
        .await
        .unwrap();
    let fixture = Fixture(std::sync::Arc::new(std::sync::Mutex::new(Vec::new())));
    service
        .serve_at(
            ObjectPath::new("/test/Accessibility").unwrap(),
            fixture.clone(),
        )
        .unwrap();
    let proxy = caller
        .proxy::<dyn tinybus::Interface>("test.Accessibility", "/test/Accessibility")
        .unwrap();
    (AccessibilityClient::fixture(proxy), fixture, service)
}

#[cfg(feature = "modules")]
#[tokio::test]
async fn permissions_use_the_accessibility_bus_member() {
    let (client, fixture, _service) = fixture().await;
    let status = client.permissions().await.unwrap();
    assert_eq!(status.microphone, PermissionState::Granted);
    assert_eq!(*fixture.0.lock().unwrap(), ["permissions"]);
}

#[cfg(feature = "modules")]
#[tokio::test]
async fn disabled_component_refuses_before_reaching_the_module() {
    let mut config = Config::default();
    config.components.computer = false;
    let client = AccessibilityClient::new(&config);
    assert!(matches!(
        client.permissions().await,
        Err(AccessibilityError::Operation(ref error)) if error.code == crate::core::runtime::components::COMPONENT_DISABLED
    ));
}
