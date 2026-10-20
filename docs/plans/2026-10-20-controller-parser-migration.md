# Controller parser contract migration

## Objective and acceptance

Complete the broader version of senamakel/openhuman#228 after #466: every controller in the built-in core registry and every hosted extension must have an independent method/required-input contract and explicit valid example(s) accepted by the same pure decoder/preprocessor used by its runtime handler. The sweep must not call async handlers or cross a configuration, permission, module, network, filesystem, credential, or business-operation boundary. Core and hosted coverage must be exercised with relevant feature sets enabled and disabled.

Preserve the issue's size acceptance: shrink the counted `*schemas_tests.rs` corpus by at least 50% from the agreed 8,401-line baseline (maximum 4,200 lines). Count every replacement test, fixture, helper, macro definition/invocation, and extracted test-only file in the same accounting footprint; do not credit moved code or hide tests outside the glob. Maintain a second conservative accounting measure including all net crate-source lines added outside that glob. Keep behavior, security, defaults, migration, wire, error, and domain-output assertions unless a specific assertion is proven redundant against an independent oracle.

The existing receipts establish that universal parser coverage is feasible as a staged test contract, but do not establish the 50% reduction. The accepted base has a conservative footprint of about 7,920 lines, leaving a 3,720-line gap to 4,200; current audited candidate lists do not justify that deletion budget. Coverage work must therefore proceed alongside an assertion-by-assertion compression ledger. At each wave boundary, stop expansion and report the remaining gap if the only further reductions would remove independent behavior/security oracles. Do not claim the size criterion is met from moved helpers, count reductions, compressed formatting, or deleted meaningful behavior. Root review is required before accepting any proposed tradeoff.

## Architecture

Use the existing private `cfg(test)` contract support in `crates/openhuman-core/src/core/schema_test_support.rs`, extended only as required. Keep `ControllerSchema`, `RegisteredController`, `ControllerExtension`, and registration APIs unchanged. A contract row contains an independently written method literal, independently written required input names, the registered handler function pointer, explicit valid JSON object examples, and an erased callback to the real pure runtime decoder. The runner compares the complete controller and contract method sets, rejects duplicates/empty inventories/missing or extra rows, verifies handler identity, checks nonempty descriptions and exact required-input declarations, then checks every example with both `validate_params` and the real decoder.

The callback is not a second test parser and must not simply return `Ok(())`. Typed controllers use the same `serde` decode function as production. Manual map readers are extracted into a private named pure parser whose decoded result/error is consumed by both the async handler and the contract callback. Keep config loading and gates at their current position; a callback invokes only the parser, never the handler. Existing error/default/value behavior remains independently asserted where it is meaningful. The central meaning of “handler accepts its example” is deliberately bounded to the handler's parameter decoding/preprocessing seam. Business operation success, state-dependent validation, and environment-dependent checks remain covered by focused owner tests.

The core owns built-in and internal controllers. `openhuman-tinyhumans` owns its hosted extension and its own contract sweep; the core must not depend upward or mutate a process-global extension registry in parallel tests. Feature-gated rows are guarded by the same feature and each supported relevant build configuration runs its contract suite. No vendor module implementation is imported to construct a host test.

## Migration rules

For every old test considered for removal, record its asserted behavior, its replacement oracle (or why it is a true duplicate), and old/new line counts. Preserve independent exact identities, required/optional distinctions, output names/shapes, secret redaction, error classes/messages relied on by callers, defaults, aliases, migrations, and security policy. Generic registry consistency does not replace these owner-level contracts. Do not derive fixtures, required names, examples, or expected method inventory from the live schema. Do not run side-effecting handlers as test probes. Do not move tests outside the counted footprint to claim savings.

Required-field mutation checks must run over every contract row and every independently declared required name: remove the field from a cloned schema and separately mark it optional; the runner must reject both. The ordinary positive contract must also fail against a deliberate required-schema regression. Demonstrate red with a nonzero selected test count, restore the exact source, then rerun green. A failing callback or error caused by config/gate setup is not acceptable evidence of parser rejection.

## Bite-sized TDD sequence

Each step is a separately reviewable change on `epic/8089-test-ownership`; keep the branch buildable. For each behavior change, add the test first, run the focused test and record its red result, implement the smallest change, then rerun it green. Keep formatting and focused checks for touched files in that step.

### 1. Lock the test runner's independent contract

Files: `crates/openhuman-core/src/core/schema_test_support.rs`, its sibling `schema_test_support_tests.rs`, and `all_schema_contract_tests.rs`.

Add focused runner tests for missing/extra/duplicate rows, handler mismatch, empty inventory, empty examples, schema rejection, parser rejection, and each required-field mutant. Use tiny synthetic controllers/parsers only to test runner failure modes. Convert the current representative mutant assertion to table-driven checks over every covered required field. Keep current memory and wallet independent fixed assertions and the existing registry shape/serialization check until equivalent broad independent inventory exists. Prove red by breaking a known required schema entry in an isolated local mutation and running the exact test; restore it and record green.

### 2. Complete the existing real-parser vertical slice

Files: `voice/schemas/parameter_contracts.rs`, `voice/schemas_tests.rs`, `voice/live/schemas.rs` and its sibling tests as needed, `mcp/registry/schemas/handlers.rs` and tests, `modules/schemas.rs` and tests, and `core/all_schema_contract_tests.rs`.

Keep the existing voice, `mcp_clients.registry_get`, and `modules.computer_status` path as the pilot. Ensure every voice registered method is represented, examples call the exact shared decoder, and old independent voice behavior/default/provider/error/security checks stay. Add the MCP manual parser seam and preserve missing/null/type/value behavior. Keep `computer_status_load` as a real shared preprocessor; do not move it across its current config/module gate ordering. Make the pilot inventory set equality explicit and run enabled/disabled feature variants. Compress only duplicated catalog scaffolding after replacements are active; compare total counted lines before and after.

### 3. Migrate typed core controllers in domain batches

Files: all core domain `*_schemas_tests.rs` owners plus their schema/handler files only where a private shared decoder is needed; central inventory in `core/all_schema_contract_tests.rs`.

Use batches that follow ownership and minimize review surface: (a) threads, sessions, profiles, agent/agent-runs/workflow-runs; (b) security credentials, encryption, devices, keyring consent, approval; (c) config/workspace, tools, inference/tokenjuice/embedding-host/host-runtime; (d) artifacts, skills/webhooks/runtime, Composio, MCP/server/UI/audit; (e) cron, channels/providers/controllers, voice/live; (f) desktop, platform, storage, web3, flows, memory, sandbox, health, and remaining domains. Before each batch, enumerate registered methods from the authoritative registry and compare against owner-authored contract rows. Typed parsing can share a generic erased callback only when it invokes the same actual runtime deserializer. Manual readers get named parser extraction only when production and the contract use that function. Retain business validation, policy, persistence, network, and error-oracle tests.

After each batch, the global contract inventory must cover the entire selected built-in snapshot; selection is explicit for feature-gated groups, not inferred by filtering only whatever rows happen to have fixtures. Convert the final registry sweep from representative coverage to exact complete coverage only after every method in that registry has an owner row. Run the required-field mutants for every row. Maintain a per-domain old/new line and assertion ledger.

### 4. Migrate hosted extension controllers

Files: `crates/openhuman-tinyhumans/src/hosted/**/schemas.rs` and owner tests, `crates/openhuman-tinyhumans/src/hosted/mod.rs`, plus the hosted crate's test support.

Add a hosted-crate-local snapshot of the extension returned by its composition function and an independently declared complete contract table. Reuse core's generic verifier through the existing test-only facade if that facade is available without changing normal dependency direction; otherwise implement only a thin local adapter that calls the same shared verifier API. Every example uses the hosted handler's real decoder. If a hosted method cannot expose a side-effect-free decoder without moving config/gate behavior, extract a private parser and call it from the handler at its original location. Do not invoke hosted handlers, network clients, or global registration mutation. Test full extension parity, method uniqueness, identity, descriptions, required mutants, and parser acceptance under the hosted feature configuration.

### 5. Remove only proven duplicate schema-test scaffolding

Files: remaining core `*_schemas_tests.rs`, config schema tests, and any test-support files originally in scope.

For each candidate, replace repeated generic name/count/parity/field traversal with compact readable fixed tables using shared helpers. Keep literal expected method names and independent values visible. Retain all behavior/default/security/migration/wire/error/output assertions. Delete compile-only stubs only after confirming they assert no behavior and are not required for module linkage. Remove serde-default tests only when the default is asserted independently elsewhere or has no product/security/migration meaning. Do not mechanically minify or macro-hide fixtures. Review size against both corpus and conservative accounting after every domain batch.

### 6. Close acceptance and verification

Files: final test inventories only; update this plan or PR description with measured results, not source API documentation.

Make the global core contract test cover every built-in and internal registered method exactly once for the enabled feature set, with the disabled-feature variant proving no unavailable contracts leak in. Make hosted coverage equally complete. Run focused crate checks, strict all-target clippy, focused tests, and formatting for touched crates; then the issue gates: `cargo test -p openhuman --lib core::`, `cargo test -p openhuman --lib` with the repository's prescribed `RUST_MIN_STACK`, `pnpm rust:layout`, and `node scripts/check-domain-e2e-coverage.mjs`. Run a hosted crate test/check with its hosted feature enabled and the relevant disabled feature build. Long Cargo gates use `scripts/ci-cancel-aware.sh`; do not set `CARGO_TARGET_DIR`. Record unrelated existing failures separately. Calculate the final 8,401 → counted-line total and conservative total from a clean diff; acceptance requires no more than 4,200 counted lines and honest accounting of all net additions.

## File groups and ownership boundaries

- Core registry and verifier: `crates/openhuman-core/src/core/all.rs`, `all_schema_contract_tests.rs`, `schema_test_support.rs`, sibling `schema_test_support_tests.rs`.
- Core parser and behavior owners: `crates/openhuman-core/src/<domain>/**/schemas*.rs` and sibling `*_tests.rs`; no flat business logic under `core/`.
- Hosted extension and hosted method owners: `crates/openhuman-tinyhumans/src/hosted/**`; its tests stay in that crate.
- Tests remain in their owning crate. Core cannot test hosted controllers by adding a dependency on tinyhumans; tinyhumans cannot reimplement core's parser or controller semantics.
- No `vendor/` production changes are expected. If investigation finds a parser or module behavior owned by a vendor, stop and move that behavior and its tests to the owning repository before changing the OpenHuman adapter.

## Honest line and behavior accounting

Starting basis: original issue baseline 8,401; cap 4,200. Partial #466 is already included in the accepted epic base. The most recent conservative receipt counted approximately 7,920 total equivalent lines and 7,109 raw schema-test lines; receipts vary slightly by accepted-base snapshot, so before source migration recompute once from the exact merged base and preserve 8,401/4,200 as the comparison. For every change, record:

| Domain/batch | Old counted lines | New counted lines | Net counted change | Outside-glob net | Assertions removed | Independent replacement |
| --- | ---: | ---: | ---: | ---: | ---: | --- |

The target is not met until the summed counted total is at most 4,200. If an honest, reviewed assertion map shows that remaining lines are independent behavior/security contracts or necessary explicit examples and the target cannot be reached without dropping them, document the exact remaining gap and ask root to resolve the scope tradeoff before weakening any oracle. Current evidence is insufficient to promise completion of the size target; it is also insufficient to call that target impossible.
