use super::*;
use crate::core::all::{
    assert_schema_contract, ControllerSchemaContract as Contract, SchemaFieldContract as Field,
};

#[test]
fn wallet_schema_wire_contracts_are_preserved() {
    assert_eq!(all_wallet_controller_schemas().len(), 16);
    assert_eq!(all_wallet_registered_controllers().len(), 16);

    let contracts: &[(&str, Option<&[&str]>, Option<usize>, &[Field])] = &[
        ("generate_mnemonic", Some(&["wordCount"][..]), None, &[]),
        (
            "validate_mnemonic",
            Some(&["phrase"][..]),
            None,
            &[Field {
                name: "phrase",
                required: Some(true),
                type_debug: None,
            }],
        ),
        ("derive_recovery_material", Some(&["phrase"][..]), None, &[]),
        (
            "tx_status",
            Some(&["chain", "hash", "evmNetwork"][..]),
            None,
            &[],
        ),
        ("status", None, Some(0), &[]),
        (
            "setup",
            None,
            Some(6),
            &[
                Field {
                    name: "encryptedMnemonic",
                    required: Some(true),
                    type_debug: None,
                },
                Field {
                    name: "force",
                    required: Some(false),
                    type_debug: None,
                },
            ],
        ),
        (
            "execute_prepared",
            Some(&["quoteId", "confirmed"][..]),
            None,
            &[],
        ),
        (
            "prepare_transfer",
            None,
            None,
            &[Field {
                name: "assetSymbol",
                required: Some(false),
                type_debug: None,
            }],
        ),
    ];
    for (function, input_order, input_count, inputs) in contracts {
        let contract = Contract {
            namespace: "wallet",
            function: *function,
            input_count: *input_count,
            input_order: *input_order,
            required_input_order: None,
            inputs: *inputs,
            output_fields: &[],
            outputs: None,
        };
        let schema = wallet_schemas(function);
        assert_ne!(schema.function, "unknown", "missing {}", contract.function);
        assert_schema_contract(&schema, &contract);
    }

    for removed in ["prepare_swap", "prepare_contract_call"] {
        assert_eq!(wallet_schemas(removed).function, "unknown");
    }
}
