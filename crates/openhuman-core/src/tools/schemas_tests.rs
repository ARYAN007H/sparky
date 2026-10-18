use super::*;

use crate::core::all::{
    assert_schema_contract, ControllerSchemaContract as Contract, SchemaFieldContract as Field,
};
use crate::tools::schemas::registry::tools_schemas;
use serde_json::{json, Map};
const BASE_CONTROLLERS: usize = 5;

#[test]
fn all_schemas_covers_the_base_surface() {
    assert_eq!(all_controller_schemas().len(), BASE_CONTROLLERS);
}

#[test]
fn all_controllers_covers_the_base_surface() {
    assert_eq!(all_registered_controllers().len(), BASE_CONTROLLERS);
}

#[test]
fn apify_linkedin_scrape_controller_is_retired() {
    // The backend retired its Apify routes; the controller must not be
    // registered or resolvable any more.
    assert!(!all_controller_schemas()
        .iter()
        .any(|s| s.function == "apify_linkedin_scrape"));
    assert_eq!(
        tools_schemas("tools_apify_linkedin_scrape").function,
        "unknown"
    );
}

#[test]
fn tools_schema_wire_contracts_are_preserved() {
    let contracts: &[(&str, &[Field], &[&str])] = &[
        (
            "composio_execute",
            &[Field {
                name: "action",
                required: Some(true),
                type_debug: None,
            }],
            &[],
        ),
        (
            "web_answer",
            &[
                Field {
                    name: "query",
                    required: Some(true),
                    type_debug: None,
                },
                Field {
                    name: "depth",
                    required: None,
                    type_debug: None,
                },
            ],
            &["citations"],
        ),
        (
            "web_contents",
            &[Field {
                name: "urls",
                required: Some(true),
                type_debug: None,
            }],
            &[],
        ),
        (
            "searxng_search",
            &[
                Field {
                    name: "query",
                    required: Some(true),
                    type_debug: None,
                },
                Field {
                    name: "max_results",
                    required: None,
                    type_debug: None,
                },
            ],
            &[],
        ),
        (
            "web_search",
            &[Field {
                name: "query",
                required: Some(true),
                type_debug: None,
            }],
            &["provider"],
        ),
    ];
    for (function, inputs, outputs) in contracts {
        let contract = Contract {
            namespace: "tools",
            function: *function,
            input_count: None,
            input_order: None,
            required_input_order: None,
            inputs: *inputs,
            output_fields: *outputs,
            outputs: None,
        };
        let schema = tools_schemas(&format!("tools_{function}"));
        assert_schema_contract(&schema, &contract);
    }
}

#[test]
fn optional_string_array_trims_and_drops_blank_entries() {
    let params = Map::from_iter([("categories".to_string(), json!([" web ", "", "  ", "news"]))]);

    let values = optional_string_array(&params, "categories").expect("string array");

    assert_eq!(values, vec!["web", "news"]);
}
