use super::*;
use crate::core::{FieldSchema, TypeSchema};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

static CALLS: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());
static EVENTS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

fn record_event(event: &'static str) {
    EVENTS
        .lock()
        .expect("event lock is not poisoned")
        .push(event);
}

fn clear_events() {
    EVENTS.lock().expect("event lock is not poisoned").clear();
}

fn events() -> Vec<&'static str> {
    EVENTS.lock().expect("event lock is not poisoned").clone()
}

#[derive(Debug)]
struct Parsed(String);

fn parse_value(params: Map<String, Value>) -> Result<Parsed, String> {
    CALLS.fetch_add(1, Ordering::SeqCst);
    record_event("parse");
    params
        .get("value")
        .and_then(Value::as_str)
        .map(|value| Parsed(value.to_owned()))
        .ok_or_else(|| "`value` is required".to_owned())
}

fn run_value(_ctx: DispatchCtx, parsed: Parsed) -> ControllerFuture {
    CALLS.fetch_add(1, Ordering::SeqCst);
    record_event("run");
    Box::pin(async move { Ok(Value::String(parsed.0)) })
}

fn gate(_ctx: DispatchCtx) -> ControllerTask<()> {
    CALLS.fetch_add(1, Ordering::SeqCst);
    record_event("gate");
    Box::pin(async { Ok(()) })
}

fn schema() -> ControllerSchema {
    ControllerSchema {
        namespace: "bound_test",
        function: "echo",
        description: "Echo one value through a bound parser.",
        inputs: vec![FieldSchema {
            name: "value",
            ty: TypeSchema::String,
            comment: "Value to echo.",
            required: true,
        }],
        outputs: vec![FieldSchema {
            name: "value",
            ty: TypeSchema::String,
            comment: "Echoed value.",
            required: true,
        }],
    }
}

fn params() -> Map<String, Value> {
    serde_json::from_value(serde_json::json!({"value":"fixture"})).expect("fixture is an object")
}

#[test]
fn bound_controller_uses_its_parser_for_decode_and_runtime_invocation() {
    let _lock = TEST_LOCK.lock().expect("test lock is not poisoned");
    CALLS.store(0, Ordering::SeqCst);
    clear_events();
    let controller = BoundRegisteredController::typed(schema(), parse_value, run_value);

    controller
        .decode_params(params())
        .expect("the bound parser accepts the fixture");
    assert_eq!(CALLS.load(Ordering::SeqCst), 1, "decoder only parses");
    assert_eq!(events(), ["parse"]);

    let output = futures::executor::block_on(controller.invoke(DispatchCtx::unbound(), params()))
        .expect("runtime invocation succeeds");
    assert_eq!(output, Value::String("fixture".to_owned()));
    assert_eq!(
        CALLS.load(Ordering::SeqCst),
        3,
        "invocation parses then runs"
    );
    assert_eq!(events(), ["parse", "parse", "run"]);
}

#[test]
fn bound_controller_decode_does_not_run_gate_or_business_handler() {
    let _lock = TEST_LOCK.lock().expect("test lock is not poisoned");
    CALLS.store(0, Ordering::SeqCst);
    clear_events();
    let controller =
        BoundRegisteredController::after_gate(schema(), gate, parse_value, |_ctx, (), parsed| {
            run_value(DispatchCtx::unbound(), parsed)
        });

    controller
        .decode_params(params())
        .expect("the bound parser accepts the fixture");
    assert_eq!(CALLS.load(Ordering::SeqCst), 1, "only the parser ran");
    assert_eq!(events(), ["parse"]);

    let output = futures::executor::block_on(controller.invoke(DispatchCtx::unbound(), params()))
        .expect("gated runtime invocation succeeds");
    assert_eq!(output, Value::String("fixture".to_owned()));
    assert_eq!(CALLS.load(Ordering::SeqCst), 4, "gate, parser, then runner");
    assert_eq!(events(), ["parse", "gate", "parse", "run"]);
}

#[test]
fn bound_controller_runs_gate_before_parser() {
    let _lock = TEST_LOCK.lock().expect("test lock is not poisoned");
    CALLS.store(0, Ordering::SeqCst);
    clear_events();
    let controller =
        BoundRegisteredController::after_gate(schema(), gate, parse_value, run_value_after_gate);

    let output = futures::executor::block_on(controller.invoke(DispatchCtx::unbound(), params()))
        .expect("gated runtime invocation succeeds");
    assert_eq!(output, Value::String("fixture".to_owned()));
    assert_eq!(CALLS.load(Ordering::SeqCst), 3);
    assert_eq!(events(), ["gate", "parse", "run"]);
}

fn run_value_after_gate(_ctx: DispatchCtx, _gate: (), parsed: Parsed) -> ControllerFuture {
    run_value(DispatchCtx::unbound(), parsed)
}

#[test]
fn bound_controller_preserves_parser_errors() {
    let _lock = TEST_LOCK.lock().expect("test lock is not poisoned");
    clear_events();
    let controller = BoundRegisteredController::typed(schema(), parse_value, run_value);
    let error = controller
        .decode_params(Map::new())
        .expect_err("missing value is rejected");
    assert_eq!(error, "`value` is required");
}

#[test]
fn production_registry_binds_threads_upsert_to_its_declared_parser() {
    use serde_json::json;

    // Read the unfiltered production registry directly. The domain catalog is
    // useful for wire-shape checks, but it can remain correct if production
    // registration accidentally falls back to a legacy handler.
    let entries: Vec<_> = registry()
        .iter()
        .chain(internal_registry())
        .map(|grouped| &grouped.controller)
        .filter(|entry| entry.schema.method_name() == "threads.upsert")
        .collect();
    assert_eq!(entries.len(), 1, "threads.upsert has one raw registry entry");
    let entry = entries[0];

    let required = ["id", "title", "created_at"];
    let declared: Vec<_> = entry
        .schema
        .inputs
        .iter()
        .filter(|field| field.required)
        .map(|field| field.name)
        .collect();
    assert_eq!(declared, required, "required fields are independent contract");

    for example in [
        json!({"id":"t1", "title":"Hello", "created_at":"2026-04-22T00:00:00Z"}),
        json!({"id":"t2", "title":"Thread", "created_at":"2026-04-22T00:00:00Z", "labels":["work"]}),
    ] {
        let params = example.as_object().expect("example is an object").clone();
        validate_params(&entry.schema, &params)
            .expect("actual registered schema accepts the example");
        entry
            .decode_params(params)
            .expect("actual raw registry entry uses the runtime parser");
    }

    for missing in ["id", "title", "created_at"] {
        let mut params = serde_json::from_value(json!({
            "id":"t1", "title":"Hello", "created_at":"2026-04-22T00:00:00Z"
        }))
        .expect("valid upsert params");
        params.remove(missing);
        assert!(
            validate_params(&entry.schema, &params).is_err(),
            "registered schema requires `{missing}`"
        );
        assert!(
            entry.decode_params(params).is_err(),
            "runtime parser rejects missing `{missing}`"
        );
    }
}
