use super::*;
use crate::core::all::{
    assert_schema_controller_parity, expect_schema_fields, expect_schema_identity,
    expect_schema_output_names, expect_schema_required_inputs,
};
use serde_json::json;

#[test]
fn cron_schema_wire_contracts_are_preserved() {
    assert_schema_controller_parity(
        &all_controller_schemas(),
        &all_registered_controllers(),
        "add",
    );
    let functions: Vec<_> = all_controller_schemas()
        .iter()
        .map(|schema| schema.function)
        .collect();
    assert_eq!(
        functions,
        ["add", "list", "update", "remove", "run", "runs"]
    );

    for (function, required) in [
        ("add", &["schedule"][..]),
        ("update", &["job_id", "patch"][..]),
        ("remove", &["job_id"][..]),
    ] {
        let schema = schemas(function);
        expect_schema_identity(&schema, "cron", function);
        expect_schema_fields(
            &schema,
            &required
                .iter()
                .map(|name| (*name, Some(true)))
                .collect::<Vec<_>>(),
        );
        if function == "update" {
            assert!(schema.inputs.iter().all(|field| field.required));
        }
    }
    expect_schema_required_inputs(&schemas("add"), &["schedule"]);
    let list = schemas("list");
    expect_schema_identity(&list, "cron", "list");
    assert!(list.inputs.is_empty());
    expect_schema_output_names(&list, &["jobs"]);
    let remove = schemas("remove");
    assert_eq!(remove.inputs.len(), 1);
    assert_eq!(remove.outputs[0].name, "result");
    assert_eq!(schemas("add").outputs[0].name, "job");
    expect_schema_fields(&schemas("runs"), &[("limit", Some(false))]);

    let s = schemas("run");
    // `cron_run` spawns the execution and returns immediately, so the declared
    // output is the enqueue acknowledgement — `job_id` plus a `status` that is
    // always "queued". `duration_ms` / `output` belong to the terminal record
    // in `cron_runs` and must not be promised here.
    let TypeSchema::Object { fields } = &s.outputs[0].ty else {
        panic!("expected object output type");
    };
    let names: Vec<_> = fields.iter().map(|f| f.name).collect();
    assert_eq!(names, vec!["job_id", "status"]);

    let status = fields.iter().find(|f| f.name == "status").unwrap();
    assert!(
        matches!(&status.ty, TypeSchema::Enum { variants } if variants.as_slice() == ["queued"]),
        "status must declare exactly the variant the handler emits, got {:?}",
        status.ty
    );
}

#[test]
fn schemas_run_description_points_at_cron_runs_for_the_terminal_record() {
    let s = schemas("run");
    assert!(
        s.description.contains("cron_runs"),
        "the description must tell a client where the terminal outcome lives, got: {}",
        s.description
    );
}

// ── read_required ───────────────────────────────────────────────

#[test]
fn read_required_returns_value_for_present_key() {
    let mut params = Map::new();
    params.insert("job_id".into(), json!("abc"));
    let got: String = read_required(&params, "job_id").unwrap();
    assert_eq!(got, "abc");
}

#[test]
fn read_required_errors_when_key_missing() {
    let params = Map::new();
    let err = read_required::<String>(&params, "job_id").unwrap_err();
    assert!(err.contains("missing required param 'job_id'"));
}

#[test]
fn read_required_errors_when_deserialization_fails() {
    let mut params = Map::new();
    params.insert("job_id".into(), json!(42));
    let err = read_required::<String>(&params, "job_id").unwrap_err();
    assert!(err.contains("invalid 'job_id'"));
}

// ── read_optional_u64 ───────────────────────────────────────────

#[test]
fn read_optional_u64_handles_absent_null_values_and_errors() {
    assert_eq!(read_optional_u64(&Map::new(), "limit").unwrap(), None);
    let mut params = Map::new();
    params.insert("limit".into(), Value::Null);
    assert_eq!(read_optional_u64(&params, "limit").unwrap(), None);
    params.clear();
    params.insert("limit".into(), json!(42));
    assert_eq!(read_optional_u64(&params, "limit").unwrap(), Some(42));

    for (tag, value) in [
        ("negative", json!(-1)),
        ("string", json!("ten")),
        ("bool", json!(true)),
        ("array", json!([1, 2])),
        ("object", json!({"k": 1})),
    ] {
        let mut params = Map::new();
        params.insert("limit".into(), value);
        let err = read_optional_u64(&params, "limit").unwrap_err();
        assert!(
            err.contains("expected unsigned integer"),
            "tag={tag} err={err}"
        );
    }
}

// ── type_name ───────────────────────────────────────────────────
