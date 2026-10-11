//! Agent tool adapters that send wallet operations through TinyWallet's bus.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tinytools::{Tool, ToolExposure, ToolResult};
use tinywallet_bus::tools::ToolDeclaration;
use tinywallet_bus::wire::{EvmNetwork, WalletChain};

#[derive(Debug)]
struct WalletModuleTool {
    declaration: &'static ToolDeclaration,
}

impl WalletModuleTool {
    const fn new(declaration: &'static ToolDeclaration) -> Self {
        Self { declaration }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxQuery {
    chain: WalletChain,
    #[serde(default)]
    evm_network: Option<EvmNetwork>,
    hash: String,
}

#[async_trait]
impl Tool for WalletModuleTool {
    fn exposure(&self) -> ToolExposure {
        ToolExposure::Deferred
    }

    fn name(&self) -> &'static str {
        self.declaration.name
    }

    fn description(&self) -> &'static str {
        self.declaration.description
    }

    fn parameters_schema(&self) -> Value {
        self.declaration.parameters.clone()
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let result = match self.name() {
            "wallet_status" => crate::web3::wallet::status()
                .await
                .map(|outcome| serde_json::to_value(outcome.value))
                .map_err(anyhow::Error::msg),
            "wallet_chain_status" => crate::web3::wallet::chain_status()
                .await
                .map(|outcome| serde_json::to_value(outcome.value))
                .map_err(anyhow::Error::msg),
            "wallet_prepare_transfer" => {
                let params = match serde_json::from_value(args) {
                    Ok(params) => params,
                    Err(error) => return Ok(ToolResult::error(format!("invalid arguments: {error}"))),
                };
                crate::web3::wallet::prepare_transfer(params)
                    .await
                    .map(|outcome| serde_json::to_value(outcome.value))
                    .map_err(anyhow::Error::msg)
            }
            "wallet_tx_status" | "wallet_tx_receipt" | "wallet_lookup_tx" => {
                let query: TxQuery = match serde_json::from_value(args) {
                    Ok(query) => query,
                    Err(error) => return Ok(ToolResult::error(format!("invalid arguments: {error}"))),
                };
                let result = match self.name() {
                    "wallet_tx_status" => crate::web3::wallet::tx_status(
                        query.chain,
                        query.evm_network,
                        &query.hash,
                    )
                    .await
                    .map(|outcome| serde_json::to_value(outcome.value)),
                    "wallet_tx_receipt" => crate::web3::wallet::tx_receipt(
                        query.chain,
                        query.evm_network,
                        &query.hash,
                    )
                    .await
                    .map(|outcome| serde_json::to_value(outcome.value)),
                    _ => crate::web3::wallet::lookup_tx(
                        query.chain,
                        query.evm_network,
                        &query.hash,
                    )
                    .await
                    .map(|outcome| serde_json::to_value(outcome.value)),
                };
                result.map_err(anyhow::Error::msg)
            }
            _ => return Ok(ToolResult::error("unknown wallet tool")),
        };

        match result {
            Ok(Ok(value)) => Ok(ToolResult::success(serde_json::to_string_pretty(&value)?)),
            Ok(Err(error)) => Ok(ToolResult::error(format!("failed to serialize wallet result: {error}"))),
            Err(error) => Ok(ToolResult::error(error.to_string())),
        }
    }
}

/// The six established wallet tools, backed by OpenHuman's module adapters.
pub fn all_agent_tools() -> Vec<Box<dyn Tool>> {
    tinywallet_bus::tools::wallet_tool_declarations()
        .iter()
        .map(|declaration| {
            Box::new(WalletModuleTool::new(
                tinywallet_bus::tools::find_wallet_tool_declaration(declaration.name)
                    .expect("the declaration came from the shipped Wallet list"),
            )) as Box<dyn Tool>
        })
        .collect()
}
