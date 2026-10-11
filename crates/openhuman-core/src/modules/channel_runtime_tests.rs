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
    for path in [HOST_OBJECT_PATH.to_string(), format!("{HOST_OBJECT_PATH}/runtime_ffffffffffffffffffffffffffffffff")] {
        let mut value = reservation(); value.host_object_path = path;
        assert_eq!(validate_reservation(&value), Err(ModuleCallError::ModuleFault));
    }
    let mut value = reservation(); value.object_path = OBJECT_PATH.to_string();
    assert_eq!(validate_reservation(&value), Err(ModuleCallError::ModuleFault));
}

#[test]
fn malformed_handles_cannot_replace_another_callback() {
    for handle in ["", "../secret", "0123456789ABCDEF0123456789ABCDEF", "0123456789abcdef0123456789abcdef/other"] {
        let mut value = reservation(); value.handle = handle.to_string();
        assert_eq!(validate_reservation(&value), Err(ModuleCallError::ModuleFault));
    }
}
