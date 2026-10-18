use super::*;
use tinyflows::graph::ids;
use tinyflows::graph::GraphEvent;

#[test]
fn graph_observations_are_adapted_without_changing_their_wire_shape() {
    let thread_id = "flow:flow-1:uuid-1";
    let engine = FlowObservation {
        event_id: ids::EventId::new("evt-1"),
        run_id: ids::RunId::new("run-9"),
        root_run_id: ids::RunId::new("run-9"),
        parent_run_id: None,
        thread_id: Some(ids::ThreadId::new(thread_id)),
        graph_id: ids::GraphId::new("workflow"),
        checkpoint_id: None,
        namespace: Vec::new(),
        step: 1,
        offset: 2,
        ts_ms: 1_020,
        event: GraphEvent::NodeStarted {
            node: ids::NodeId::new("fetch"),
            step: 1,
        },
    };

    let converted = exporter_observations(&[engine.clone()]);
    assert_eq!(converted.len(), 1);
    assert_eq!(
        serde_json::to_value(&engine).unwrap(),
        serde_json::to_value(&converted[0]).unwrap()
    );
    assert_eq!(converted[0].thread_id.as_ref().map(|id| id.as_str()), Some(thread_id));
    assert_eq!(converted[0].event_id.as_str(), "evt-1");
    assert_eq!(converted[0].step, 1);
    assert_eq!(converted[0].ts_ms, 1_020);
}

#[test]
fn catalog_flow_triggers_map_to_the_serde_bus_contract() {
    let cases = [
        (FlowRunTrigger::Rpc, BusFlowRunTrigger::Rpc),
        (FlowRunTrigger::Schedule, BusFlowRunTrigger::Schedule),
        (FlowRunTrigger::AppEvent, BusFlowRunTrigger::AppEvent),
        (FlowRunTrigger::Resume, BusFlowRunTrigger::Resume),
    ];
    for (catalog, bus) in cases {
        assert_eq!(flow_trigger(catalog), bus);
    }
}
