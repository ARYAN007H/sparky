# Private Rust validation inventory and measurements

This inventory records the test selections before changing execution for #8063.
Coverage replaces equivalent executions inside the private job. Feature sets are
not treated as equivalent merely because they select the same crate.

| Selection                                                          | Earlier execution                       | Private execution                                 |
| ------------------------------------------------------------------ | --------------------------------------- | ------------------------------------------------- |
| `openhuman` product `--lib --bins`                                 | coverage nextest, background unit build | coverage nextest, sequential                      |
| `openhuman-cli` product integration targets                        | coverage nextest `kind(test)`           | same, fails empty selection                       |
| `openhuman-embed`, `openhuman-tinyhumans` product, all targets     | coverage libtest                        | coverage nextest                                  |
| `openhuman-embed`, `openhuman-tinyhumans` default, all targets     | omitted as presumed duplicates          | separate coverage nextest configurations          |
| `openhuman-rpc`, `openhuman-tui` default, all targets              | coverage libtest                        | coverage nextest                                  |
| core no-default lib, existing gate filters                         | plain libtest                           | coverage nextest with equivalent substring unions |
| core no-default plus `modules,mcp,e2e-test-support`, gate filters  | serial plain libtest                    | serial coverage nextest                           |
| Tauri own manifest/default features                                | coverage libtest                        | coverage nextest                                  |
| CLI ten storage E2E targets, `storage-sqlite,storage-file`         | plain libtest                           | coverage nextest, same targets and driver matrix  |
| core storage domain filters excluding cron module tests            | plain libtest                           | coverage nextest, same inclusions/exclusions      |
| core storage cron bus/documents tests                              | serial plain libtest                    | serial coverage nextest                           |
| RPC `session-store,storage-sqlite,storage-file`, `session_store::` | plain libtest                           | coverage nextest                                  |

The storage E2E list and domain filter list remain declared in `lanes-plan.mjs`.
Product features remain in `scripts/ci/product-features.txt`; they are resolved
per package, preserving existing feature forwarding. Checksum-pinned native test
modules, coverage-presence checks and the 80% changed-line gate remain required.
Doctests, clippy, rustdoc, examples and uninstrumented link checks remain separate
because coverage/nextest does not replace them.

The Composio stack-budget regression has `#![cfg(not(coverage))]` and runs once
through uninstrumented nextest in `rust-lint`. Instrumented integration
expectations exclude that target and, outside Windows, the entirely
`#[cfg(windows)]` sandbox target. Present binaries containing only ignored tests
are checked against nextest's compiled inventory; all other missing or empty
expected targets fail validation.

## Full-workflow overlap audit

[CI Full](../.github/workflows/ci-full.yml) calls
[test-reusable.yml](../.github/workflows/test-reusable.yml), whose core job
already enumerates all **46 root Rust integration targets**. The
[Rust E2E runner](../scripts/test-rust-e2e.sh) selects **35 of those same target
names**: `tests/*_e2e.rs` plus `in_process_all`. Job names do not establish
that their test selections are distinct.

| Executions or mode                                                                     | Overlap and configuration differences                                                                                                                                                                                                                                                                                                                                            | Decision                                                                                                           |
| -------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| CI Full's direct Rust E2E job and its `e2e-desktop` call's nested `rust-e2e-linux` job | Both invoke `pnpm test:rust:e2e`, selecting the same 35 targets with product features and the Node mock. The direct job pins its container digest and sets line-table debug information; the reusable job uses a moving image tag and inherits its own workflow's `OPENHUMAN_NODE_ENABLED=0`. Caller workflow environment is not automatically inherited by a reusable workflow. | Retain: repeated selections, but runtime environments are not proven equivalent.                                   |
| Reusable core integrations and Rust E2E                                                | All 35 E2E target names occur in the core job's 46-target inventory. The core job selects `-p openhuman-cli` with product features plus `bin-tools`; E2E selects the workspace with product features and starts an ambient Node mock. Individual tests may also start their own mocks or clear inherited backend settings.                                                       | Retain: package, feature, mock and process setup differ; a shared target name does not prove equivalent execution. |
| `mcp_registry_e2e`                                                                     | The E2E runner explicitly adds `--ignored --test-threads=1`; ordinary integration execution skips these ignored cases.                                                                                                                                                                                                                                                           | Preserve this distinct test mode.                                                                                  |
| `tinymcp_storage_smoke_e2e`                                                            | The E2E runner adds `e2e-test-support` and serial execution.                                                                                                                                                                                                                                                                                                                     | Preserve this distinct feature configuration.                                                                      |
| `tinydocs_linked_e2e`                                                                  | The E2E runner adds non-product `linked-modules`.                                                                                                                                                                                                                                                                                                                                | Preserve this distinct feature configuration.                                                                      |
| Browser/desktop E2E, Windows secrets and MongoDB validation                            | These exercise frontend or native host flows, Windows-specific code, or a separate storage feature and external-service configuration.                                                                                                                                                                                                                                           | Preserve these distinct contexts.                                                                                  |

These full/reusable workflows do not invoke the private coverage lane. No
execution above is removed on the assumption that private coverage replaces an
equivalent selection in the same validation run. Further deduplication would
require checking the resolved package features and normalizing the relevant
runtime settings first.

The current local callers of `test-reusable.yml` are `ci-full.yml` and
[test.yml](../.github/workflows/test.yml); the callers of `e2e-reusable.yml` are
`ci-full.yml` and [e2e.yml](../.github/workflows/e2e.yml). Comments describing
direct staging/production pretest callers are stale. The
[staging](../.github/workflows/release-staging.yml) and
[production](../.github/workflows/release-production.yml) workflows remain
untouched by this change.

## Historical warm baseline

[Run 39006804022](https://github.com/tinyhumansai/openhuman/actions/runs/39006804022),
head `a90ebba82e4bbfe4d454107620b52bf18092932a`, completed its EX63 job on
2026-10-17 from 05:53:35 to 06:22:35 UTC: **29m00s**, excluding approximately
18m15s queue time. This used parallel lanes and the previous sccache backend.

- Cache: 5,373 Rust hits, zero misses. This is a **warm** baseline, not a cold run.
- Core nextest: 10,168 unit tests and 447 integration tests passed; 10 and 19 skipped.
- Remaining core coverage libtest records: 945 passed across 89 result records.
- Tauri: 309 passed, one ignored. Gates-off: 488 passed. Storage: 2,549 passed.
- Coverage: 1,554 eligible files present; changed-line report 86% over 125 measured
  changed lines (17 violations). Four LCOV reports were archived.
- Core coverage check: 20m23s; unit execution 546.719s; integration 316.683s.
- Cache save time is unavailable for this old backend.

These counts are executions, not deduplicated test identities. The historical
commit and runner allocation differ from the implementation benchmark, so this
is context rather than a controlled speed-up claim.

## Measurement protocol

Run the complete selected lane plan twice at the same source revision and image,
using fresh target outputs. Preserve package-manager caches, and identify whether
the signed remote Rust cache already contains those compiler keys. Do not call a
run cold merely because its target directory is new. Record Tinymold hit/miss and
save time, test counts, coverage presence, linkage proof, each check's result and
elapsed time. Preserve failed runs too. After merging, repeat through trusted-main
workflow routing, since pre-merge runs can execute the old workflow.
