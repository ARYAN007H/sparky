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
        "queuedChatAck",
        "startedChatAck",
        "threadsList",
        "threadMessages",
        "threadDelete",
        "purgeResult",
        "threadSearchHit",
        "threadSearch",
        "getTurnState",
        "listTurnStates",
        "clearTurnState",
        "agentRun",
        "runEvent",
        "runTelemetry",
    ] {
        assert!(
            properties.contains_key(property),
            "schema is missing {property}"
        );
    }
    let ack = &schema["$defs"]["QueuedChatAck"];
    for field in [
        "queued",
        "queue_mode",
        "client_id",
        "thread_id",
        "request_id",
        "queue_depth",
        "queued_attachments",
    ] {
        assert!(
            ack["required"].as_array().unwrap().contains(&field.into()),
            "ack schema must require serialized field {field}"
        );
    }
}

#[test]
fn started_chat_ack_schema_requires_every_serialized_field() {
    let schema: serde_json::Value =
        serde_json::from_str(&thread_wire_schema_json().expect("schema serializes"))
            .expect("schema is JSON");
    let ack = &schema["$defs"]["StartedChatAck"];
    for field in ["accepted", "client_id", "thread_id", "request_id"] {
        assert!(
            ack["required"].as_array().unwrap().contains(&field.into()),
            "started ack schema must require serialized field {field}"
        );
    }
}
