use super::*;
use tinywallet_bus::wire::{NativeWalletServiceHandle, WalletAccountInfo};

fn service() -> NativeWalletService {
    NativeWalletService {
        workspace: PathBuf::from("/users/alice/workspace"),
        configuration: NativeWalletConfigRequest {
            solana_cluster: SolanaCluster::Mainnet,
            overridden_endpoints: vec![],
        },
        accounts: WalletAccountsResponse {
            configured: true,
            accounts: vec![WalletAccountInfo {
                chain: tinywallet_bus::Chain::Evm,
                address: "0xabc".into(),
                derivation_path: "m/44'/60'/0'/0/0".into(),
            }],
        },
        response: NativeWalletConfigResponse {
            service: NativeWalletServiceHandle {
                handle: "opaque-service-handle".into(),
            },
            endpoint_references: vec![],
        },
        prepared_quotes: std::collections::HashMap::new(),
    }
}

#[test]
fn native_service_is_replaced_when_the_user_workspace_changes() {
    let active = service();
    assert!(service_requires_replacement(
        &active,
        std::path::Path::new("/users/bob/workspace"),
        &active.configuration,
        &active.accounts,
    ));
}

#[test]
fn native_service_is_replaced_when_wallet_accounts_change() {
    let active = service();
    let changed = WalletAccountsResponse {
        configured: true,
        accounts: vec![WalletAccountInfo {
            chain: tinywallet_bus::Chain::Evm,
            address: "0xdef".into(),
            derivation_path: "m/44'/60'/0'/0/0".into(),
        }],
    };
    assert!(service_requires_replacement(
        &active,
        &active.workspace,
        &active.configuration,
        &changed,
    ));
}

#[test]
fn native_service_is_replaced_when_endpoint_policy_changes() {
    let active = service();
    let changed = NativeWalletConfigRequest {
        solana_cluster: SolanaCluster::Devnet,
        overridden_endpoints: vec![],
    };
    assert!(service_requires_replacement(
        &active,
        &active.workspace,
        &changed,
        &active.accounts,
    ));
}

#[test]
fn native_service_reuses_its_handle_for_identical_user_state() {
    let active = service();
    assert!(!service_requires_replacement(
        &active,
        &active.workspace,
        &active.configuration,
        &active.accounts,
    ));
}

#[tokio::test]
async fn unconfirmed_execution_is_rejected_before_wallet_service_or_secret_access() {
    let error = execute_native_transfer(
        &crate::config::Config::default(),
        None,
        tinywallet_bus::wire::ExecutePreparedParams {
            quote_id: "quote-that-is-never-opened".into(),
            confirmed: false,
        },
    )
    .await
    .expect_err("an unconfirmed transfer must be rejected at the host boundary");

    assert!(matches!(
        error,
        WalletCallError::InvalidInput(message)
            if message == "native wallet execution requires confirmed: true"
    ));
}
