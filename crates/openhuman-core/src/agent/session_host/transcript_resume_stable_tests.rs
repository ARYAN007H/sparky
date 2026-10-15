//! A workspace written by the previous release resumes in place: resuming a
//! committed fixture neither renames, rewrites nor re-adopts anything, so the
//! transcripts the store holds are exactly what the import brought in.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;
use tinyinference_llm::model::ModelResponse;

use super::transcript_compat_tests::{
    build_host, model, place_fixture, run_async, store_tree, thread_id, Layout, SCENARIOS,
};

fn tree(root: &Path, thread: &str, host: &crate::agent::OpenHumanSessionHost) -> BTreeMap<String, Value> {
    store_tree(root, thread, host.session.as_ref().expect("bound session"))
}

#[test]
fn resuming_a_committed_workspace_leaves_every_stem_and_row_untouched() {
    for scenario in SCENARIOS
        .iter()
        .filter(|scenario| scenario.layout != Layout::Legacy)
    {
        run_async(async move {
            let root = tempfile::tempdir().expect("tempdir");
            let thread = thread_id(scenario.name);
            let next = model(vec![ModelResponse::assistant("unused")], scenario.native);
            let mut host = build_host(root.path(), next, scenario.native, &thread);
            let stem = host.session_id().expect("bound session id");
            place_fixture(root.path(), scenario, &stem, "");
            let before = tree(root.path(), &thread, &host);
            assert!(!before.is_empty(), "{}: fixture placed", scenario.name);

            assert!(host.resume_bound_session().await.expect("resume"));

            assert_eq!(
                before,
                tree(root.path(), &thread, &host),
                "{}: resume must not rename, create or rewrite transcripts",
                scenario.name
            );
        });
    }
}

#[test]
fn a_legacy_workspace_is_adopted_once_and_the_next_process_resumes_in_place() {
    let scenario = SCENARIOS
        .iter()
        .find(|scenario| scenario.layout == Layout::Legacy)
        .expect("legacy scenario");
    run_async(async move {
        let root = tempfile::tempdir().expect("tempdir");
        let thread = thread_id(scenario.name);
        let legacy_only;
        let after_first;
        {
            let next = model(vec![ModelResponse::assistant("unused")], scenario.native);
            let mut host = build_host(root.path(), next, scenario.native, &thread);
            let stem = host.session_id().expect("bound session id");
            place_fixture(root.path(), scenario, &stem, "");
            let l = super::transcript_compat_tests::store_locator(root.path());
            let sess = host.session.clone().unwrap();
            let t = tree(root.path(), &thread, &host);
            eprintln!("DBGX tree keys={:?}", t.keys().collect::<Vec<_>>());
            eprintln!("DBGX adopt={:?}", l.adopt_legacy(&sess, &thread, &l.thread_transcripts(&thread, None).unwrap().roots[0].transcript.read_session().unwrap().unwrap().meta).map(|a| a.map(|x| x.messages)));
            assert!(host.resume_bound_session().await.expect("first resume"));
            legacy_only = BTreeMap::<String, Value>::new();
            after_first = tree(root.path(), &thread, &host);
        }
        // Adoption writes the session's own transcript and modifies no legacy root.
        for (name, rows) in &legacy_only {
            assert_eq!(
                after_first.get(name),
                Some(rows),
                "legacy root {name} must stay as imported"
            );
        }
        assert!(
            after_first.len() > legacy_only.len(),
            "adopted transcript written"
        );

        // A fresh process over the same workspace resumes without adopting again.
        let next = model(vec![ModelResponse::assistant("unused")], scenario.native);
        let mut host = build_host(root.path(), next, scenario.native, &thread);
        assert!(host.resume_bound_session().await.expect("second resume"));
        assert_eq!(
            after_first,
            tree(root.path(), &thread, &host),
            "second resume must not re-adopt or rewrite anything"
        );
    });
}
