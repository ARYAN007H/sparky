use super::*;

// Fixed expectations carried over from the pre-existing memory specification
// tests and wallet setup regression. Never derive this oracle from live schemas.
fn independent_registered_schema_contract(schema: &ControllerSchema) -> Result<(), String> {
    let required: &[&str] = match (schema.namespace, schema.function) {
        ("memory", "engine_set") => &["engine"],
        ("memory", "recall") => &["question"],
        ("memory", "fetch" | "brain_search") => &["query"],
        ("memory", "learn") => &["text"],
        ("memory", "forget") => &["ids"],
        ("memory", "sources_add") => &["kind", "target"],
        ("memory", "sources_remove") => &["id"],
        ("memory", "import_start") => &["consent"],
        ("memory", "brain_forget") => &["source"],
        ("memory", "policy_set") => &[],
        ("wallet", "setup") => return required_fields_contract(schema, &["encryptedMnemonic"]),
        _ => return Ok(()),
    };
    required_input_contract(schema, required)
}

#[test]
fn registered_memory_contract_rejects_missing_or_optional_question() {
    let actual = all_registered_controllers()
        .into_iter()
        .map(|controller| controller.schema)
        .find(|schema| schema.method_name() == "memory.recall")
        .expect("memory.recall is registered");
    assert_eq!(independent_registered_schema_contract(&actual), Ok(()));
    for remove in [true, false] {
        let mut broken = actual.clone();
        if remove {
            broken.inputs.retain(|input| input.name != "question");
        } else {
            broken
                .inputs
                .iter_mut()
                .find(|input| input.name == "question")
                .expect("real recall schema declares question")
                .required = false;
        }
        assert!(independent_registered_schema_contract(&broken)
            .unwrap_err()
            .contains("memory.recall required inputs mismatch"));
    }
}

#[test]
fn registered_controller_schemas_are_consistent_and_serializable() {
    use std::collections::BTreeSet;

    let controllers = builtin_controller_snapshot();
    assert!(!controllers.is_empty(), "the controller registry is empty");
    let mut methods = BTreeSet::new();

    for controller in controllers {
        let schema = &controller.schema;
        let method = schema.method_name();
        assert!(methods.insert(method.clone()), "duplicate method {method}");
        assert!(
            !schema.namespace.trim().is_empty(),
            "{method} has no namespace"
        );
        assert!(
            !schema.function.trim().is_empty(),
            "{method} has no function name"
        );
        assert!(
            !schema.description.trim().is_empty(),
            "{method} has no description"
        );
        assert!(!schema.outputs.is_empty(), "{method} has no output fields");

        assert_eq!(
            independent_registered_schema_contract(schema),
            Ok(()),
            "{method} independent schema contract"
        );

        let serialized = serde_json::to_value(schema).expect("controller schema serializes");
        let serialized_inputs = serialized["inputs"]
            .as_array()
            .unwrap_or_else(|| panic!("{method} inputs are not serialized as an array"));
        assert_eq!(
            serialized_inputs.len(),
            schema.inputs.len(),
            "{method} serialized input count differs from its registered schema"
        );
        let mut params = Map::new();
        for input in &schema.inputs {
            let matching = serialized_inputs
                .iter()
                .find(|field| field["name"] == input.name);
            let matching = matching.unwrap_or_else(|| {
                panic!(
                    "{method} input `{}` is missing from serialized schema",
                    input.name
                )
            });
            assert_eq!(
                matching["required"], input.required,
                "{method} input `{}` requiredness differs in serialized schema",
                input.name
            );
            if input.required {
                // `validate_params` accepts null for every declared type, so this
                // safely checks required-key acceptance without executing a handler.
                params.insert(input.name.to_owned(), Value::Null);
            }
        }
        assert!(
            validate_params(schema, &params).is_ok(),
            "{method} rejects a map containing all required schema inputs"
        );
        for input in schema.inputs.iter().filter(|input| input.required) {
            let mut missing = params.clone();
            missing.remove(input.name);
            assert!(
                validate_params(schema, &missing).is_err(),
                "{method} accepts params without required input `{}`",
                input.name
            );
        }
    }
}

#[cfg(feature = "web3")]
#[test]
fn registered_wallet_schema_contract_rejects_missing_encrypted_mnemonic() {
    let actual = all_registered_controllers()
        .into_iter()
        .map(|controller| controller.schema)
        .find(|schema| schema.method_name() == "wallet.setup")
        .expect("wallet.setup is registered with web3 enabled");
    assert_eq!(independent_registered_schema_contract(&actual), Ok(()));

    let mut schema = actual;
    schema
        .inputs
        .retain(|input| input.name != "encryptedMnemonic");

    assert!(independent_registered_schema_contract(&schema)
        .unwrap_err()
        .contains("is missing required input `encryptedMnemonic`"));
}

/// Built-in/internal snapshots avoid ambient DomainSet filtering and global
/// hosted-extension mutation. Hosted/RPC contracts belong to their own crates.
fn builtin_controller_snapshot() -> Vec<RegisteredController> {
    registry()
        .iter()
        .chain(internal_registry())
        .map(|grouped| grouped.controller.clone())
        .collect()
}

fn representative_parameter_contracts() -> (Vec<RegisteredController>, Vec<ControllerParamContract>)
{
    let controllers = builtin_controller_snapshot()
        .into_iter()
        .filter(|controller| {
            (cfg!(feature = "voice") && controller.schema.namespace == "voice")
                || (cfg!(feature = "mcp")
                    && controller.schema.method_name() == "mcp_clients.registry_get")
                || controller.schema.method_name() == "modules.computer_status"
        })
        .collect();
    let contracts = vec![crate::modules::schemas::computer_status_parameter_contract()];
    #[cfg(feature = "mcp")]
    let contracts = contracts
        .into_iter()
        .chain([crate::mcp::registry::registry_get_parameter_contract()])
        .collect::<Vec<_>>();
    #[cfg(feature = "voice")]
    let contracts = contracts
        .into_iter()
        .chain(crate::voice::parameter_contracts())
        .collect::<Vec<_>>();
    (controllers, contracts)
}

#[test]
fn voice_and_representative_handler_examples_match_real_pure_parsers() {
    let (controllers, contracts) = representative_parameter_contracts();
    verify_parameter_contracts(&controllers, &contracts).unwrap();
}

#[test]
fn every_covered_required_declaration_rejects_removal_and_optional_mutants() {
    let (controllers, contracts) = representative_parameter_contracts();
    verify_parameter_contracts(&controllers, &contracts).unwrap();
    let mut mutations = 0;
    for contract in &contracts {
        let actual = controllers
            .iter()
            .find(|controller| controller.schema.method_name() == contract.method)
            .unwrap();
        for required in contract.required {
            for remove in [true, false] {
                let mut broken = actual.schema.clone();
                if remove {
                    broken.inputs.retain(|field| field.name != *required);
                } else {
                    broken
                        .inputs
                        .iter_mut()
                        .find(|field| field.name == *required)
                        .expect("positive contract requires this field")
                        .required = false;
                }
                assert!(
                    verify_parameter_schema(&broken, contract)
                        .unwrap_err()
                        .contains("required inputs mismatch"),
                    "{} required {required}",
                    contract.method
                );
                mutations += 1;
            }
        }
    }
    // Voice has ten independently required fields and MCP has one. The
    // computer-status adapter remains covered when both optional domains are
    // disabled, but has no required field to mutate. Synthetic runner tests
    // still exercise mutation rejection in that feature configuration.
    let expected_mutations =
        20 * usize::from(cfg!(feature = "voice")) + 2 * usize::from(cfg!(feature = "mcp"));
    assert_eq!(
        mutations, expected_mutations,
        "covered required-field mutations"
    );
}

#[test]
fn real_contract_inventory_rejects_a_missing_and_an_extra_row() {
    let (controllers, mut contracts) = representative_parameter_contracts();
    let removed = contracts
        .pop()
        .expect("representative contracts are nonempty");
    assert!(verify_parameter_contracts(&controllers, &contracts)
        .unwrap_err()
        .contains("missing contract"));
    contracts.push(removed.clone());
    contracts.push(ControllerParamContract {
        method: "unregistered.fixture",
        ..removed
    });
    assert!(verify_parameter_contracts(&controllers, &contracts)
        .unwrap_err()
        .contains("extra contract"));
}
