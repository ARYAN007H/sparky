use super::*;
use crate::core::all::{
    assert_schema_controller_parity, ControllerHandler, ControllerParamContract,
};

#[test]
fn legacy_registered_handlers_keep_their_function_identity() {
    let controllers = all_registered_controllers();
    let schemas = all_controller_schemas();
    let expected_handlers: [ControllerHandler; 2] = [handle_encrypt_secret, handle_decrypt_secret];
    assert_eq!(controllers.len(), 2);
    assert_eq!(controllers.len(), expected_handlers.len());
    assert_eq!(schemas.len(), 2);
    assert_eq!(
        schemas
            .iter()
            .map(|schema| schema.method_name())
            .collect::<Vec<_>>(),
        ["encrypt.secret", "decrypt.secret"]
    );
    assert_schema_controller_parity(&schemas, &controllers, "secret");
    for (controller, expected) in controllers.iter().zip(expected_handlers) {
        assert!(std::ptr::fn_addr_eq(controller.handler, expected));
    }
}

pub(super) fn parameter_contracts() -> Vec<ControllerParamContract> {
    use serde_json::json;

    vec![
        ControllerParamContract {
            method: "encrypt.secret",
            required: &["plaintext"],
            optional: &[],
            examples: vec![
                json!({"plaintext":""}),
                json!({"plaintext":"  keep whitespace  "}),
            ],
        },
        ControllerParamContract {
            method: "decrypt.secret",
            required: &["ciphertext"],
            optional: &[],
            examples: vec![
                json!({"ciphertext":""}),
                json!({"ciphertext":"  keep whitespace  "}),
            ],
        },
    ]
}

#[test]
fn read_required_parses_values_and_preserves_error_categories() {
    let mut m = Map::new();
    m.insert("key".into(), Value::String("value".into()));
    let result: String = read_required(&m, "key").unwrap();
    assert_eq!(result, "value");
    assert!(read_required::<String>(&Map::new(), "key")
        .unwrap_err()
        .contains("missing required param"));
    let wrong = Map::from_iter([("key".into(), Value::Bool(true))]);
    assert!(read_required::<String>(&wrong, "key")
        .unwrap_err()
        .contains("invalid"));
}
