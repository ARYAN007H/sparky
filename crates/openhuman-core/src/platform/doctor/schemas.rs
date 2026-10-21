use crate::core::runtime::DispatchCtx;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::config::Config;
use crate::core::all::{
    BoundRegisteredController, ControllerEntry, ControllerFuture, ControllerTask,
    RegisteredController,
};
use crate::core::Outcome;
use crate::core::{ControllerSchema, FieldSchema, TypeSchema};

pub fn all_controller_schemas() -> Vec<ControllerSchema> {
    vec![schemas("report"), schemas("models")]
}

pub fn all_registered_controllers() -> Vec<RegisteredController> {
    vec![
        RegisteredController {
            schema: schemas("report"),
            handler: handle_report,
        },
        RegisteredController {
            schema: schemas("models"),
            handler: handle_models,
        },
    ]
}

pub(crate) fn all_controller_entries() -> Vec<ControllerEntry> {
    vec![
        ControllerEntry::bound(report_controller()),
        ControllerEntry::bound(models_controller()),
    ]
}

#[cfg(test)]
pub(crate) fn doctor_parameter_contracts() -> Vec<crate::core::all::ControllerParamContract> {
    tests::doctor_parameter_contracts()
}

pub fn schemas(function: &str) -> ControllerSchema {
    match function {
        "report" => ControllerSchema {
            namespace: "doctor",
            function: "report",
            description: "Run diagnostics for workspace and runtime configuration.",
            inputs: vec![],
            outputs: vec![FieldSchema {
                name: "report",
                ty: TypeSchema::Ref("DoctorReport"),
                comment: "Aggregated diagnostics report.",
                required: true,
            }],
        },
        "models" => ControllerSchema {
            namespace: "doctor",
            function: "models",
            description: "Probe provider model availability and auth status.",
            inputs: vec![FieldSchema {
                name: "use_cache",
                ty: TypeSchema::Option(Box::new(TypeSchema::Bool)),
                comment: "Reuse cached provider metadata when available.",
                required: false,
            }],
            outputs: vec![FieldSchema {
                name: "report",
                ty: TypeSchema::Ref("ModelProbeReport"),
                comment: "Model probe summary grouped by provider.",
                required: true,
            }],
        },
        _ => ControllerSchema {
            namespace: "doctor",
            function: "unknown",
            description: "Unknown doctor controller function.",
            inputs: vec![],
            outputs: vec![FieldSchema {
                name: "error",
                ty: TypeSchema::String,
                comment: "Lookup error details.",
                required: true,
            }],
        },
    }
}

fn handle_report(ctx: DispatchCtx, _params: Map<String, Value>) -> ControllerFuture {
    report_controller().invoke(ctx, _params)
}

fn handle_models(ctx: DispatchCtx, params: Map<String, Value>) -> ControllerFuture {
    models_controller().invoke(ctx, params)
}

fn report_controller() -> BoundRegisteredController {
    BoundRegisteredController::after_gate(schemas("report"), load_config, parse_report, run_report)
}

fn models_controller() -> BoundRegisteredController {
    BoundRegisteredController::after_gate(schemas("models"), load_config, parse_models, run_models)
}

fn load_config(ctx: DispatchCtx) -> ControllerTask<Config> {
    Box::pin(async move { ctx.load_config().await })
}

fn parse_report(_params: Map<String, Value>) -> Result<(), String> {
    Ok(())
}

fn parse_models(params: Map<String, Value>) -> Result<bool, String> {
    Ok(read_optional::<bool>(&params, "use_cache")?.unwrap_or(true))
}

fn run_report(_ctx: DispatchCtx, config: Config, (): ()) -> ControllerFuture {
    Box::pin(async move { to_json(crate::platform::doctor::rpc::doctor_report(&config).await?) })
}

fn run_models(_ctx: DispatchCtx, config: Config, use_cache: bool) -> ControllerFuture {
    Box::pin(async move {
        to_json(crate::platform::doctor::rpc::doctor_models(&config, use_cache).await?)
    })
}

fn read_optional<T: DeserializeOwned>(
    params: &Map<String, Value>,
    key: &str,
) -> Result<Option<T>, String> {
    match params.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => serde_json::from_value(value.clone())
            .map(Some)
            .map_err(|e| format!("invalid '{key}': {e}")),
    }
}

fn to_json<T: serde::Serialize>(outcome: Outcome<T>) -> Result<Value, String> {
    outcome.into_cli_compatible_json()
}

#[cfg(test)]
#[path = "schemas_tests.rs"]
mod tests;
