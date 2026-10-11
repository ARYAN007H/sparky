//! User-scoped lifecycle for the module-owned x402 ledger.

use std::path::PathBuf;
use std::sync::OnceLock;

use tinywallet_bus::names::methods;
use tinywallet_bus::wire::{
    X402CloseRequest, X402ConfigResponse, X402ConfigureRequest, X402DetailedSpendingSummary,
    X402ListPaymentsRequest, X402ListPaymentsResponse, X402ServiceHandle, X402UpdateBudgetRequest,
};

use crate::config::Config;

use super::wallet::{self, WalletCallError};

struct X402Service {
    workspace: PathBuf,
    config: X402ConfigResponse,
    handle: X402ServiceHandle,
}

static X402_SERVICE: OnceLock<tokio::sync::Mutex<Option<X402Service>>> = OnceLock::new();

fn service() -> &'static tokio::sync::Mutex<Option<X402Service>> {
    X402_SERVICE.get_or_init(|| tokio::sync::Mutex::new(None))
}

async fn ensure_service(
    config: &Config,
    active: &mut Option<X402Service>,
) -> Result<(), WalletCallError> {
    let workspace = config.workspace_dir.clone();
    let budget = budget_from(|name| std::env::var(name).ok());

    if active
        .as_ref()
        .is_some_and(|current| current.workspace != workspace)
    {
        if let Some(current) = active.as_ref() {
            wallet::call::<_, ()>(
                config,
                methods::X402_CLOSE,
                X402CloseRequest {
                    service: current.handle.clone(),
                },
            )
            .await?;
        }
        *active = None;
    }

    if active.is_none() {
        let service_config = X402ConfigResponse {
            workspace_dir: workspace.display().to_string(),
            session_id: format!("x402-{}", uuid::Uuid::new_v4()),
            per_request_max_atomic: budget.0,
            daily_max_atomic: budget.1,
            monthly_max_atomic: budget.2,
        };
        let handle: X402ServiceHandle = wallet::call(
            config,
            methods::X402_CONFIGURE,
            X402ConfigureRequest {
                config: service_config.clone(),
            },
        )
        .await?;
        *active = Some(X402Service {
            workspace,
            config: service_config,
            handle,
        });
    }
    Ok(())
}

async fn with_service<R, F, Fut>(config: &Config, operation: F) -> Result<R, WalletCallError>
where
    F: FnOnce(X402ServiceHandle) -> Fut,
    Fut: std::future::Future<Output = Result<R, WalletCallError>>,
{
    let mut active = service().lock().await;
    ensure_service(config, &mut active).await?;
    let handle = active
        .as_ref()
        .ok_or_else(|| WalletCallError::Failed("x402 service is unavailable".into()))?
        .handle
        .clone();
    operation(handle).await
}

/// Read the module-owned spending totals and budgets.
pub(crate) async fn summary(
    config: &Config,
) -> Result<X402DetailedSpendingSummary, WalletCallError> {
    let config = config.clone();
    let operation_config = config.clone();
    with_service(&config, move |service| async move {
        wallet::call(
            &operation_config,
            methods::X402_SPENDING_SUMMARY_DETAILED,
            tinywallet_bus::wire::X402SummaryRequest { service },
        )
        .await
    })
    .await
}

/// Read recent module-owned payment records.
pub(crate) async fn list_payments(
    config: &Config,
    limit: u64,
) -> Result<X402ListPaymentsResponse, WalletCallError> {
    let config = config.clone();
    let operation_config = config.clone();
    with_service(&config, move |service| async move {
        wallet::call(
            &operation_config,
            methods::X402_LIST_PAYMENTS,
            X402ListPaymentsRequest { service, limit },
        )
        .await
    })
    .await
}

/// Update live module-owned spending limits without replacing the ledger.
pub(crate) async fn update_budget(
    config: &Config,
    per_request_max_atomic: Option<u64>,
    daily_max_atomic: Option<u64>,
    monthly_max_atomic: Option<u64>,
) -> Result<tinywallet_bus::wire::X402SpendingSummary, WalletCallError> {
    let mut active = service().lock().await;
    ensure_service(config, &mut active).await?;
    let current = active
        .as_mut()
        .ok_or_else(|| WalletCallError::Failed("x402 service is unavailable".into()))?;
    let result: tinywallet_bus::wire::X402SpendingSummary = wallet::call(
        config,
        methods::X402_UPDATE_BUDGET,
        X402UpdateBudgetRequest {
            service: current.handle.clone(),
            per_request_max_atomic,
            daily_max_atomic,
            monthly_max_atomic,
        },
    )
    .await?;
    current.config.per_request_max_atomic = result.per_request_max_atomic;
    current.config.daily_max_atomic = result.daily_max_atomic;
    current.config.monthly_max_atomic = result.monthly_max_atomic;
    Ok(result)
}

fn budget_from(get: impl Fn(&str) -> Option<String>) -> (u64, u64, u64) {
    let mut per_request_max_atomic = 1_000_000;
    let mut daily_max_atomic = 10_000_000;
    let mut monthly_max_atomic = 100_000_000;
    if let Some(value) = get("OPENHUMAN_X402_PER_REQUEST_MAX").and_then(|value| value.parse().ok())
    {
        per_request_max_atomic = value;
    }
    if let Some(value) = get("OPENHUMAN_X402_DAILY_MAX").and_then(|value| value.parse().ok()) {
        daily_max_atomic = value;
    }
    if let Some(value) = get("OPENHUMAN_X402_MONTHLY_MAX").and_then(|value| value.parse().ok()) {
        monthly_max_atomic = value;
    }
    (per_request_max_atomic, daily_max_atomic, monthly_max_atomic)
}

#[cfg(test)]
#[path = "x402_service_tests.rs"]
mod tests;
