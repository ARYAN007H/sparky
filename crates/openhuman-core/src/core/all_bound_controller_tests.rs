use super::*;
use crate::core::{runtime::DispatchCtx, FieldSchema, TypeSchema};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
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

fn failing_gate(_ctx: DispatchCtx) -> ControllerTask<()> {
    CALLS.fetch_add(1, Ordering::SeqCst);
    record_event("gate");
    Box::pin(async { Err("gate rejected".to_owned()) })
}

fn failing_parse(_params: Map<String, Value>) -> Result<Parsed, String> {
    CALLS.fetch_add(1, Ordering::SeqCst);
    record_event("parse");
    Err("parser rejected".to_owned())
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
fn bound_controller_gate_error_short_circuits_parser_and_runner() {
    let _lock = TEST_LOCK.lock().expect("test lock is not poisoned");
    CALLS.store(0, Ordering::SeqCst);
    clear_events();
    let controller = BoundRegisteredController::after_gate(
        schema(),
        failing_gate,
        parse_value,
        run_value_after_gate,
    );

    let error = futures::executor::block_on(controller.invoke(DispatchCtx::unbound(), params()))
        .expect_err("gate failure is returned");
    assert_eq!(error, "gate rejected");
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(events(), ["gate"]);
}

#[test]
fn bound_controller_runtime_parser_error_short_circuits_runner() {
    let _lock = TEST_LOCK.lock().expect("test lock is not poisoned");
    CALLS.store(0, Ordering::SeqCst);
    clear_events();
    let controller = BoundRegisteredController::typed(schema(), failing_parse, run_value);

    let error = futures::executor::block_on(controller.invoke(DispatchCtx::unbound(), params()))
        .expect_err("parser failure is returned");
    assert_eq!(error, "parser rejected");
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(events(), ["parse"]);
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
    assert_eq!(
        entries.len(),
        1,
        "threads.upsert has one raw registry entry"
    );
    let entry = entries[0];
    let bound_methods: Vec<_> = registry()
        .iter()
        .chain(internal_registry())
        .filter(|grouped| grouped.controller.is_bound())
        .map(|grouped| grouped.controller.schema.method_name())
        .collect();
    assert_eq!(
        bound_methods,
        ["composio.execute", "threads.upsert"],
        "staged bound inventory"
    );
    assert!(
        entry.is_bound(),
        "threads.upsert must be bound at the production registration site"
    );

    let required = ["id", "title", "created_at"];
    let declared: Vec<_> = entry
        .schema
        .inputs
        .iter()
        .filter(|field| field.required)
        .map(|field| field.name)
        .collect();
    assert_eq!(
        declared, required,
        "required fields are independent contract"
    );

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
        let mut params: Map<String, Value> = serde_json::from_value(json!({
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

#[test]
fn production_registry_binds_composio_execute_to_its_declared_parser() {
    use serde_json::json;

    // Inspect the unfiltered registry so a legacy row cannot hide behind the
    // domain schema catalog or live surface filters.
    let entries: Vec<_> = registry()
        .iter()
        .chain(internal_registry())
        .map(|grouped| &grouped.controller)
        .filter(|entry| entry.schema.method_name() == "composio.execute")
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "composio.execute has one raw registry entry"
    );
    let entry = entries[0];
    let bound_methods: Vec<_> = registry()
        .iter()
        .chain(internal_registry())
        .filter(|grouped| grouped.controller.is_bound())
        .map(|grouped| grouped.controller.schema.method_name())
        .collect();
    assert_eq!(
        bound_methods,
        ["composio.execute", "threads.upsert"],
        "staged bound inventory"
    );
    assert!(
        entry.is_bound(),
        "composio.execute must be bound at registration"
    );
    assert_eq!(entry.rpc_method_name(), "openhuman.composio_execute");

    for name in ["arguments", "connection_id"] {
        assert!(
            entry
                .schema
                .inputs
                .iter()
                .any(|field| field.name == name && !field.required),
            "{name} must remain optional"
        );
    }

    required_input_contract(&entry.schema, &["tool"])
        .expect("independent required-field contract matches production schema");
    let mut missing_required = entry.schema.clone();
    missing_required.inputs.retain(|field| field.name != "tool");
    assert!(required_input_contract(&missing_required, &["tool"])
        .unwrap_err()
        .contains("required inputs mismatch"));
    let mut optional_required = entry.schema.clone();
    optional_required
        .inputs
        .iter_mut()
        .find(|field| field.name == "tool")
        .expect("tool schema field")
        .required = false;
    assert!(required_input_contract(&optional_required, &["tool"])
        .unwrap_err()
        .contains("required inputs mismatch"));

    for example in [
        json!({"tool":"GMAIL_SEND_EMAIL"}),
        json!({"tool":" GMAIL_SEND_EMAIL ","arguments":{"to":"test@example.invalid"},"connection_id":"conn-1"}),
        json!({"tool":"GMAIL_SEND_EMAIL","arguments":null,"connection_id":null}),
        json!({"tool":"GMAIL_SEND_EMAIL","arguments":42,"connection_id":""}),
    ] {
        let params = example.as_object().expect("example is an object").clone();
        validate_params(&entry.schema, &params)
            .expect("actual registered schema accepts the example");
        entry
            .decode_params(params)
            .expect("actual raw registry entry uses the runtime parser");
    }

    let mut missing_tool: Map<String, Value> = serde_json::from_value(json!({
        "arguments":{"to":"test@example.invalid"}
    }))
    .expect("object params");
    assert!(validate_params(&entry.schema, &missing_tool).is_err());
    assert!(entry.decode_params(missing_tool.clone()).is_err());
    missing_tool.insert("tool".into(), json!("GMAIL_SEND_EMAIL"));
    assert!(validate_params(&entry.schema, &missing_tool).is_ok());
    assert!(entry.decode_params(missing_tool).is_ok());
}

#[cfg(feature = "voice")]
#[test]
fn production_registry_binds_every_core_voice_controller_to_its_parser() {
    use serde_json::json;

    struct Contract {
        method: &'static str,
        required: &'static [&'static str],
        optional: &'static [&'static str],
        examples: Vec<Value>,
    }

    let contracts = [
        Contract {
            method: "voice.status",
            required: &[],
            optional: &[],
            examples: vec![json!({})],
        },
        Contract {
            method: "voice.transcribe",
            required: &["audio_path"],
            optional: &["context", "skip_cleanup"],
            examples: vec![
                json!({"audio_path":"fixture.wav"}),
                json!({"audio_path":"fixture.wav","context":"example","skip_cleanup":true}),
            ],
        },
        Contract {
            method: "voice.transcribe_bytes",
            required: &["audio_bytes"],
            optional: &["extension", "context", "skip_cleanup"],
            examples: vec![
                json!({"audio_bytes":[0,1]}),
                json!({"audio_bytes":[0,1],"extension":"wav","context":"example","skip_cleanup":true}),
            ],
        },
        Contract {
            method: "voice.tts",
            required: &["text"],
            optional: &["output_path"],
            examples: vec![
                json!({"text":"Hello"}),
                json!({"text":"Hello","output_path":"fixture.wav"}),
            ],
        },
        Contract {
            method: "voice.reply_synthesize",
            required: &["text"],
            optional: &["voice_id", "model_id", "output_format"],
            examples: vec![
                json!({"text":"Hello"}),
                json!({"text":"Hello","voice_id":"fixture","model_id":"fixture","output_format":"mp3_44100"}),
            ],
        },
        Contract {
            method: "voice.stt_dispatch",
            required: &["audio_base64"],
            optional: &["provider", "model", "mime_type", "file_name", "language"],
            examples: vec![
                json!({"audio_base64":"AA=="}),
                json!({"audio_base64":"AA==","provider":"cloud","model":"fixture","mime_type":"audio/wav","file_name":"fixture.wav","language":"en"}),
            ],
        },
        Contract {
            method: "voice.tts_dispatch",
            required: &["text"],
            optional: &["provider", "voice"],
            examples: vec![
                json!({"text":"Hello"}),
                json!({"text":"Hello","provider":"cloud","voice":"fixture"}),
            ],
        },
        Contract {
            method: "voice.set_providers",
            required: &[],
            optional: &["stt_provider", "tts_provider", "stt_model", "tts_voice"],
            examples: vec![
                json!({}),
                json!({"stt_provider":"cloud","tts_provider":"cloud","stt_model":"fixture","tts_voice":"fixture"}),
            ],
        },
        Contract {
            method: "voice.update_provider_settings",
            required: &[],
            optional: &["voice_providers", "stt_provider", "tts_provider"],
            examples: vec![
                json!({}),
                json!({"voice_providers":[{"slug":"fixture-provider"}],"stt_provider":"cloud","tts_provider":"cloud"}),
            ],
        },
        Contract {
            method: "voice.list_models",
            required: &["provider_id"],
            optional: &["capability"],
            examples: vec![
                json!({"provider_id":"fixture"}),
                json!({"provider_id":"fixture","capability":"stt"}),
            ],
        },
        Contract {
            method: "voice.test_provider",
            required: &["workload", "provider"],
            optional: &["validate_only", "api_key"],
            examples: &[
                json!({"workload":"tts","provider":"cloud"}),
                json!({"workload":"stt","provider":"cloud","validate_only":true,"api_key":"fixture-key"}),
            ],
        },
        Contract {
            method: "voice.live_providers",
            required: &[],
            optional: &[],
            examples: vec![json!({})],
        },
        Contract {
            method: "voice.live_settings_get",
            required: &[],
            optional: &[],
            examples: vec![json!({})],
        },
        Contract {
            method: "voice.live_settings_set",
            required: &[],
            optional: &["default_provider", "gemini", "sarvam", "elevenlabs"],
            examples: vec![
                json!({}),
                json!({"default_provider":"gemini","gemini":{"model":"fixture","voice":null},"sarvam":{"language":"en"},"elevenlabs":{"voice_id":"fixture"}}),
            ],
        },
        Contract {
            method: "voice.live_test_provider",
            required: &["provider"],
            optional: &[],
            examples: vec![json!({"provider":"gemini"})],
        },
    ];

    // Select raw production rows, before DomainSet, surface, or extension filtering.
    let entries: Vec<_> = registry()
        .iter()
        .chain(internal_registry())
        .filter_map(|grouped| {
            grouped
                .controller
                .schema
                .method_name()
                .starts_with("voice.")
                .then_some(&grouped.controller)
        })
        .collect();
    assert_eq!(
        entries.len(),
        contracts.len(),
        "complete core voice inventory"
    );
    let methods: Vec<_> = entries
        .iter()
        .map(|entry| entry.schema.method_name())
        .collect();
    assert_eq!(
        methods,
        contracts
            .iter()
            .map(|contract| contract.method)
            .collect::<Vec<_>>(),
        "independent core voice method inventory and registry order"
    );

    for (entry, contract) in entries.into_iter().zip(contracts) {
        assert!(
            entry.is_bound(),
            "{} must have a production-bound parser",
            contract.method
        );
        let required: Vec<_> = entry
            .schema
            .inputs
            .iter()
            .filter(|input| input.required)
            .map(|input| input.name)
            .collect();
        let optional: Vec<_> = entry
            .schema
            .inputs
            .iter()
            .filter(|input| !input.required)
            .map(|input| input.name)
            .collect();
        assert_eq!(
            required, contract.required,
            "{} required fields",
            contract.method
        );
        assert_eq!(
            optional, contract.optional,
            "{} optional fields",
            contract.method
        );

        for example in &contract.examples {
            let params = example
                .as_object()
                .expect("voice example is an object")
                .clone();
            validate_params(&entry.schema, &params)
                .expect("registered schema accepts voice example");
            entry
                .decode_params(params)
                .expect("actual raw registry parser accepts voice example");
        }

        for name in contract.required {
            for remove in [true, false] {
                let mut broken = entry.schema.clone();
                if remove {
                    broken.inputs.retain(|input| input.name != name);
                } else {
                    broken
                        .inputs
                        .iter_mut()
                        .find(|input| input.name == name)
                        .expect("independent required name is present")
                        .required = false;
                }
                assert!(
                    required_input_contract(&broken, contract.required).is_err(),
                    "{} independent required contract rejects {name} removal/optional mutation",
                    contract.method
                );
            }
        }
    }
}
