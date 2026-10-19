use super::*;

pub(crate) fn expect_schema_identity(schema: &ControllerSchema, namespace: &str, function: &str) {
    assert_eq!(schema.namespace, namespace);
    assert_eq!(schema.function, function);
}

pub(crate) fn expect_schema_fields(schema: &ControllerSchema, fields: &[(&str, Option<bool>)]) {
    for (name, required) in fields {
        let field = schema
            .inputs
            .iter()
            .find(|field| field.name == *name)
            .unwrap_or_else(|| {
                panic!(
                    "{}.{} is missing input `{name}`",
                    schema.namespace, schema.function
                )
            });
        if let Some(required) = required {
            assert_eq!(
                field.required, *required,
                "{}.{} input `{name}`",
                schema.namespace, schema.function
            );
        }
    }
}

pub(crate) fn expect_schema_required_inputs(schema: &ControllerSchema, required: &[&str]) {
    assert_eq!(
        required_input_contract(schema, required),
        Ok(()),
        "{}.{} required inputs",
        schema.namespace,
        schema.function
    );
}

pub(crate) fn required_input_contract(
    schema: &ControllerSchema,
    required: &[&str],
) -> Result<(), String> {
    let actual: Vec<_> = schema
        .inputs
        .iter()
        .filter(|field| field.required)
        .map(|field| field.name)
        .collect();
    if actual == required {
        Ok(())
    } else {
        Err(format!(
            "{}.{} required inputs mismatch: expected {required:?}, got {actual:?}",
            schema.namespace, schema.function
        ))
    }
}

pub(crate) fn required_fields_contract(
    schema: &ControllerSchema,
    required: &[&str],
) -> Result<(), String> {
    for name in required {
        let Some(field) = schema.inputs.iter().find(|field| field.name == *name) else {
            return Err(format!(
                "{}.{} is missing required input `{name}`",
                schema.namespace, schema.function
            ));
        };
        if !field.required {
            return Err(format!(
                "{}.{} input `{name}` is not required",
                schema.namespace, schema.function
            ));
        }
    }
    Ok(())
}

pub(crate) fn expect_schema_input_order(schema: &ControllerSchema, inputs: &[&str]) {
    let actual: Vec<_> = schema.inputs.iter().map(|field| field.name).collect();
    assert_eq!(
        actual, inputs,
        "{}.{} input order",
        schema.namespace, schema.function
    );
}

pub(crate) fn expect_schema_output_names(schema: &ControllerSchema, outputs: &[&str]) {
    let actual: Vec<_> = schema.outputs.iter().map(|field| field.name).collect();
    assert_eq!(
        actual, outputs,
        "{}.{} output names",
        schema.namespace, schema.function
    );
}

pub(crate) fn expect_schema_output_fields(schema: &ControllerSchema, outputs: &[&str]) {
    for name in outputs {
        assert!(
            schema.outputs.iter().any(|field| field.name == *name),
            "{}.{} is missing output `{name}`",
            schema.namespace,
            schema.function
        );
    }
}

/// Shared test helper: assert a domain's controller-schema list and
/// registered-controller list stay in lockstep, and that a known function is
/// present. Replaces the brittle `assert_eq!(schemas().len(), N)` magic-number
/// pattern repeated across ~15 domains (plan.md §3/§6) — a legitimate new
/// controller no longer breaks the count, but a schema/handler desync or a
/// dropped op still fails.
pub(crate) fn assert_schema_controller_parity(
    schemas: &[ControllerSchema],
    controllers: &[RegisteredController],
    known_function: &str,
) {
    assert_eq!(schemas.len(), controllers.len(), "schema/controller count");
    for (schema, controller) in schemas.iter().zip(controllers) {
        assert_eq!(
            schema.method_name(),
            controller.schema.method_name(),
            "schema/controller registration order diverged"
        );
    }
    assert!(
        schemas.iter().any(|s| s.function == known_function),
        "expected a `{known_function}` controller schema to be registered",
    );
}
