use super::*;

#[test]
fn legacy_controller_handler_identities_and_order_are_preserved() {
    use crate::core::all::ControllerHandler;

    let controllers = all_profiles_registered_controllers();
    assert_eq!(controllers.len(), FUNCTIONS.len());
    let expected: [(&str, ControllerHandler); 7] = [
        ("provision", handle_provision),
        ("deprovision", handle_deprovision),
        ("list", handle_list),
        ("status", handle_status),
        ("set_credential", handle_set_credential),
        ("clear_credential", handle_clear_credential),
        ("release", handle_release),
    ];
    for ((function, handler), controller) in expected.iter().zip(&controllers) {
        assert_eq!(controller.schema.namespace, "profiles");
        assert_eq!(controller.schema.function, *function);
        assert!(std::ptr::fn_addr_eq(controller.handler, *handler));
    }
}

pub(crate) fn profile_parameter_contracts() -> Vec<crate::core::all::ControllerParamContract> {
    use serde_json::json;

    vec![
        crate::core::all::ControllerParamContract {
            method: "profiles.provision",
            required: &["user_id"],
            optional: &[],
            examples: vec![json!({"user_id":"gateway-user-1"})],
        },
        crate::core::all::ControllerParamContract {
            method: "profiles.deprovision",
            required: &["profile_id"],
            optional: &[],
            examples: vec![json!({"profile_id":"profile-1"})],
        },
        crate::core::all::ControllerParamContract {
            method: "profiles.list",
            required: &[],
            optional: &[],
            examples: vec![json!({})],
        },
        crate::core::all::ControllerParamContract {
            method: "profiles.status",
            required: &["profile_id"],
            optional: &[],
            examples: vec![json!({"profile_id":"profile-1"})],
        },
        crate::core::all::ControllerParamContract {
            method: "profiles.set_credential",
            required: &["profile_id", "kind", "token"],
            optional: &["expires_at"],
            examples: vec![json!({
                "profile_id":"profile-1",
                "kind":"api_key",
                "token":"fixture-only-token"
            })],
        },
        crate::core::all::ControllerParamContract {
            method: "profiles.clear_credential",
            required: &["profile_id"],
            optional: &[],
            examples: vec![json!({"profile_id":"profile-1"})],
        },
        crate::core::all::ControllerParamContract {
            method: "profiles.release",
            required: &["profile_id"],
            optional: &[],
            examples: vec![json!({"profile_id":"profile-1"})],
        },
    ]
}

#[test]
fn the_operator_plane_is_its_own_domain_family() {
    use crate::core::all::DomainGroup;
    use crate::core::runtime::DomainSet;
    assert!(DomainSet::saas().allows(DomainGroup::Operator));
    for preset in [
        DomainSet::full(),
        DomainSet::harness(),
        DomainSet::embedded(),
        DomainSet::kernel(),
        DomainSet::none(),
    ] {
        assert!(
            !preset.allows(DomainGroup::Operator),
            "a single-user preset never serves the operator plane"
        );
    }
}

#[tokio::test]
async fn outside_saas_the_controllers_refuse() {
    let controllers = all_profiles_registered_controllers();
    let list = controllers
        .iter()
        .find(|c| c.schema.function == "list")
        .unwrap();
    let err = (list.handler)(DispatchCtx::process_default(), Default::default())
        .await
        .unwrap_err();
    assert!(err.contains("SaaS"), "{err}");
}
