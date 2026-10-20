use super::*;

fn handler(_ctx: DispatchCtx, _params: Map<String, Value>) -> ControllerFuture {
    panic!("the pure contract runner must never execute handlers");
}

fn other_handler(_ctx: DispatchCtx, _params: Map<String, Value>) -> ControllerFuture {
    panic!("this distinct handler must never be executed");
}

fn fixture() -> (RegisteredController, ControllerParamContract) {
    let controller = RegisteredController {
        schema: ControllerSchema {
            namespace: "fixture",
            function: "read",
            description: "Read a fixture.",
            inputs: vec![crate::core::FieldSchema {
                name: "id",
                ty: crate::core::TypeSchema::String,
                comment: "Fixture identifier.",
                required: true,
            }],
            outputs: vec![],
        },
        handler,
    };
    let contract = ControllerParamContract {
        method: "fixture.read",
        required: &["id"],
        handler,
        examples: vec![serde_json::json!({"id":"example"})],
        parse: |params| match params.get("id").and_then(Value::as_str) {
            Some("example") => Ok(()),
            _ => Err("fixture id must be example".into()),
        },
    };
    (controller, contract)
}

#[test]
fn explicit_example_reaches_pure_parser_without_executing_handler() {
    let (controller, contract) = fixture();
    verify_parameter_contracts(&[controller], &[contract]).unwrap();
}

#[test]
fn contract_inventory_rejects_missing_extra_duplicate_and_empty_rows() {
    let (controller, contract) = fixture();
    assert!(
        verify_parameter_contracts(std::slice::from_ref(&controller), &[])
            .unwrap_err()
            .contains("missing contract")
    );
    assert!(
        verify_parameter_contracts(&[], std::slice::from_ref(&contract))
            .unwrap_err()
            .contains("extra contract")
    );
    assert!(
        verify_parameter_contracts(&[controller], &[contract.clone(), contract])
            .unwrap_err()
            .contains("duplicate contract")
    );
    assert!(verify_parameter_contracts(&[], &[])
        .unwrap_err()
        .contains("empty"));
}

#[test]
fn contract_rejects_each_removed_or_optional_required_declaration() {
    let (controller, contract) = fixture();
    for remove in [true, false] {
        let mut broken = controller.clone();
        if remove {
            broken.schema.inputs.clear();
        } else {
            broken.schema.inputs[0].required = false;
        }
        assert!(
            verify_parameter_contracts(&[broken], std::slice::from_ref(&contract))
                .unwrap_err()
                .contains("required inputs mismatch")
        );
    }
}

#[test]
fn callback_identity_and_explicit_example_failures_are_not_hidden() {
    let (controller, contract) = fixture();
    let mut broken = contract.clone();
    broken.handler = other_handler;
    assert!(
        verify_parameter_contracts(std::slice::from_ref(&controller), &[broken])
            .unwrap_err()
            .contains("handler identity")
    );
    let mut broken = contract.clone();
    broken.examples.clear();
    assert!(
        verify_parameter_contracts(std::slice::from_ref(&controller), &[broken])
            .unwrap_err()
            .contains("no examples")
    );
    let mut broken = contract.clone();
    broken.examples = vec![serde_json::json!({"id":"wrong"})];
    assert!(
        verify_parameter_contracts(std::slice::from_ref(&controller), &[broken])
            .unwrap_err()
            .contains("pure parser")
    );
    let mut broken = contract;
    broken.examples = vec![serde_json::json!({"id":7})];
    assert!(verify_parameter_contracts(&[controller], &[broken])
        .unwrap_err()
        .contains("schema rejects example"));
}

#[test]
fn duplicate_controller_and_schema_identity_failures_are_rejected() {
    let (controller, contract) = fixture();
    assert!(verify_parameter_contracts(
        &[controller.clone(), controller.clone()],
        std::slice::from_ref(&contract)
    )
    .unwrap_err()
    .contains("duplicate controller"));
    let mut broken = controller.schema.clone();
    broken.function = "other";
    assert!(verify_parameter_schema(&broken, &contract)
        .unwrap_err()
        .contains("method identity"));
    let mut broken = controller.schema;
    broken.description = " ";
    assert!(verify_parameter_schema(&broken, &contract)
        .unwrap_err()
        .contains("no description"));
}

#[test]
fn examples_must_be_objects_and_satisfy_schema_before_decoding() {
    let (controller, mut contract) = fixture();
    contract.examples = vec![Value::Null];
    assert!(verify_parameter_schema(&controller.schema, &contract)
        .unwrap_err()
        .contains("not an object"));
    contract.examples = vec![serde_json::json!({})];
    assert!(verify_parameter_schema(&controller.schema, &contract)
        .unwrap_err()
        .contains("schema rejects example"));
}

#[test]
fn real_owner_namespaces_reject_each_controller_namespace_typo() {
    let families = vec![(
        "inference",
        crate::inference::all_inference_controller_schemas(),
    )];
    #[cfg(feature = "mcp")]
    let families = families
        .into_iter()
        .chain([(
            "mcp_clients",
            crate::mcp::registry::all_mcp_registry_controller_schemas(),
        )])
        .collect::<Vec<_>>();
    for (expected, schemas) in families {
        assert!(!schemas.is_empty(), "{expected} owner fixture must exist");
        expect_schema_namespace(&schemas, expected);
        for mut schema in schemas {
            let function = schema.function;
            schema.namespace = "misspelled_owner";
            assert!(
                std::panic::catch_unwind(|| expect_schema_namespace(&[schema], expected)).is_err(),
                "{expected}.{function} namespace mutation escaped"
            );
        }
    }
}
