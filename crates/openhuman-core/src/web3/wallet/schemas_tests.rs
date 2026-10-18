use super::*;
use crate::core::all::{expect_schema_fields, expect_schema_input_order};

#[test]
fn wallet_schema_wire_contracts_are_preserved() {
    assert_eq!(all_wallet_controller_schemas().len(), 16);
    assert_eq!(all_wallet_registered_controllers().len(), 16);

    for (function, inputs) in [
        ("generate_mnemonic", &["wordCount"][..]),
        ("validate_mnemonic", &["phrase"][..]),
        ("derive_recovery_material", &["phrase"][..]),
        ("tx_status", &["chain", "hash", "evmNetwork"][..]),
        ("execute_prepared", &["quoteId", "confirmed"][..]),
    ] {
        expect_schema_input_order(&wallet_schemas(function), inputs);
    }
    let status = wallet_schemas("status");
    assert!(status.inputs.is_empty());
    let setup = wallet_schemas("setup");
    assert_eq!(setup.inputs.len(), 6);
    expect_schema_fields(&setup, &[("encryptedMnemonic", Some(true)), ("force", Some(false))]);
    expect_schema_fields(
        &wallet_schemas("validate_mnemonic"),
        &[("phrase", Some(true))],
    );
    expect_schema_fields(
        &wallet_schemas("prepare_transfer"),
        &[("assetSymbol", Some(false))],
    );
    for removed in ["prepare_swap", "prepare_contract_call"] {
        assert_eq!(wallet_schemas(removed).function, "unknown");
    }
}
