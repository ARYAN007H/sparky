use super::*;

#[test]
fn thread_wire_schema_exports_the_core_and_session_records() {
    let schema: serde_json::Value =
        serde_json::from_str(&thread_wire_schema_json().expect("schema serializes"))
            .expect("schema is JSON");
    let properties = schema["properties"].as_object().expect("root properties");
    for property in [
        "thread",
        "message",
        "turnState",
        "agentRun",
        "runEvent",
        "runTelemetry",
    ] {
        assert!(properties.contains_key(property), "schema is missing {property}");
    }
}
