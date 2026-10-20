use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static CALLS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct Parsed(String);

fn parse_value(params: Map<String, Value>) -> Result<Parsed, String> {
    CALLS.fetch_add(1, Ordering::SeqCst);
    params
        .get("value")
        .and_then(Value::as_str)
        .map(|value| Parsed(value.to_owned()))
        .ok_or_else(|| "`value` is required".to_owned())
}

fn run_value(_ctx: DispatchCtx, parsed: Parsed) -> ControllerFuture {
    CALLS.fetch_add(1, Ordering::SeqCst);
    Box::pin(async move { Ok(Value::String(parsed.0)) })
}

fn gate(_ctx: DispatchCtx) -> ControllerTask<()> {
    CALLS.fetch_add(1, Ordering::SeqCst);
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
    serde_json::from_value(serde_json::json!({"value":"fixture"}))
        .expect("fixture is an object")
}

#[test]
fn bound_controller_uses_its_parser_for_decode_and_runtime_invocation() {
    CALLS.store(0, Ordering::SeqCst);
    let controller = BoundRegisteredController::typed(schema(), parse_value, run_value);

    controller
        .decode_params(params())
        .expect("the bound parser accepts the fixture");
    assert_eq!(CALLS.load(Ordering::SeqCst), 1, "decoder only parses");

    let output = futures::executor::block_on(controller.invoke(DispatchCtx::unbound(), params()))
        .expect("runtime invocation succeeds");
    assert_eq!(output, Value::String("fixture".to_owned()));
    assert_eq!(CALLS.load(Ordering::SeqCst), 3, "invocation parses then runs");
}

#[test]
fn bound_controller_decode_does_not_run_gate_or_business_handler() {
    CALLS.store(0, Ordering::SeqCst);
    let controller = BoundRegisteredController::after_gate(
        schema(),
        gate,
        parse_value,
        |_ctx, (), parsed| run_value(DispatchCtx::unbound(), parsed),
    );

    controller
        .decode_params(params())
        .expect("the bound parser accepts the fixture");
    assert_eq!(CALLS.load(Ordering::SeqCst), 1, "only the parser ran");

    let output = futures::executor::block_on(controller.invoke(DispatchCtx::unbound(), params()))
        .expect("gated runtime invocation succeeds");
    assert_eq!(output, Value::String("fixture".to_owned()));
    assert_eq!(CALLS.load(Ordering::SeqCst), 4, "gate, parser, then runner");
}

#[test]
fn bound_controller_runs_gate_before_parser() {
    CALLS.store(0, Ordering::SeqCst);
    let controller = BoundRegisteredController::after_gate(
        schema(),
        gate,
        parse_value,
        run_value_after_gate,
    );

    let output = futures::executor::block_on(controller.invoke(DispatchCtx::unbound(), params()))
        .expect("gated runtime invocation succeeds");
    assert_eq!(output, Value::String("fixture".to_owned()));
    assert_eq!(CALLS.load(Ordering::SeqCst), 3);
}

fn run_value_after_gate(_ctx: DispatchCtx, _gate: (), parsed: Parsed) -> ControllerFuture {
    run_value(DispatchCtx::unbound(), parsed)
}

#[test]
fn bound_controller_preserves_parser_errors() {
    let controller = BoundRegisteredController::typed(schema(), parse_value, run_value);
    let error = controller
        .decode_params(Map::new())
        .expect_err("missing value is rejected");
    assert_eq!(error, "`value` is required");
}
