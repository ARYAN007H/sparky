use super::*;

#[test]
fn forbidden_edit_error_is_preserved_instead_of_reported_as_unsupported() {
    let outcome = tinychannels_bus::channel::ChannelSendOutcome {
        receipt: Default::default(),
        error: Some(tinychannels_bus::channel::ChannelSendError {
            kind: tinychannels_bus::channel::SendErrorKind::Forbidden,
            message: "edit denied by account policy".into(),
            ..Default::default()
        }),
    };

    let tinychannels_bus::module_progressive::ProgressiveTransportResult::Outcome(mapped) =
        map_edit_outcome(outcome)
    else {
        panic!("a forbidden edit error must remain an outcome");
    };
    let error = mapped.error.expect("the denial details must be preserved");
    assert_eq!(
        error.kind,
        tinychannels_bus::channel::SendErrorKind::Forbidden
    );
    assert_eq!(error.message, "edit denied by account policy");
    assert!(!error.unsupported);
}
