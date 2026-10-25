# Epic 7964: upstream/main merged into epic/7964-channel-v2 (2026-10-24/25)

Receipts for `epic-7964-upstream-merge` (merge commits of `upstream/main` up to `51e12c53` /
current tip at merge time). Raw logs live in the session scratchpad; the numbers below are
copied from them. Nothing was pushed to `tinyhumansai/*`.

## Fork epic tips (all merged with their upstream default branch, merge commits, no rebase)

| Repo | Fork epic tip | Notes |
| --- | --- | --- |
| senamakel/tinychannels | `7278130d` | upstream main 337d7137 merged; later PRs: context_compacting event (wire contract 33), typed inbound preparation + turn-lease cap, final-message handle end, inbound voice ingress |
| senamakel/tinyagents | `3020f2e2` | upstream main 7346fabd merged; restored `run_progress_ledger` export and `TurnRunnerConfig::cancel_grace` |
| senamakel/tinyflows | `5953af94` | upstream main merged (clean) |
| senamakel/tinyvoice | `fca28c70` | upstream main merged (clean) |

Every other gitlink is upstream's pin; `vendor/tinymemes` is gone with upstream; `.gitmodules` is upstream's.

## Gates (product features, `--test-threads=1` for unit suites)

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy -p openhuman --all-targets --features <product> -- -D warnings` | exit 0 |
| `cargo clippy -p openhuman -p openhuman-cli -p openhuman-tinyhumans -- -D warnings` | exit 0 |
| `pnpm --filter openhuman-app rust:clippy` | exit 0 |
| `pnpm rust:layout` | exit 0 |
| `channels::` | 338 passed, 0 failed |
| `web_chat::` | 310 passed, 0 failed |
| `cron::` | 330 passed, 0 failed (needs `RUST_MIN_STACK=16777216`) |
| `flows::` | 518 passed, 1 failed: `flows_build_copilot_toolset_unhides_the_live_run_tools`, fails identically on upstream/main (pre-existing) |
| JSON-RPC e2e (`json_rpc_*_e2e`, 10 suites) | 9 exit 0; `json_rpc_auth_security_e2e` (telegram bot token) exit 101, also fails on upstream/main (module staging / pin skew) |
| `pnpm docs:generate` / `docs:check` / `docs:test` | exit 0 / 0 / 0 |
| `pnpm test` (vitest) | 9306 passed; `src/lib/attachments.test.ts` gzip case fails on Node 22, also on upstream/main |
| `pnpm typecheck`, `lint`, `format:check`, `i18n:check`, `i18n:english:check`, `chat-wire:check` | exit 0 |
| Coverage lane (`OH_COV_RUNNER=nextest scripts/ci/rust-coverage.sh`) + `diff-cover` vs upstream/main | 86% of 11048 changed lines (threshold 80) |

## Known red / not pre-existing

- `modules::registry::tests::release_manifest_tests::tinychannels_registry_matches_scoped_runtime_release`:
  the registry pins the published TinyChannels release (contract 23); the fork bus is contract 33.
  Needs a release + re-pin; pins were deliberately not touched.
- `native_channel_session_restart_e2e::telegram_voice_note_runs_scoped_stt_agent_and_mirror_tts_through_modules`:
  inbound STT works; the outbound mirrored voice reply does not. The fork TinyChannels module (already
  before this merge, 5bb0f0c4) has no outbound voice synthesis (`ModuleSynthesizer` /
  `prepare_voice_reply` are gone, nothing emits `OutboundVoice::Synthesize`).
- Module channels have no typing indicator: the module's adapter path does not implement
  `StartTyping`; the host no longer calls it for module channels.
- Several saas/embed e2e suites need the fork TinyChannels module staged
  (`node scripts/modules/build.mjs tinychannels --out <dir> --print-env`, plus
  `OPENHUMAN_TEST_MODULE_SOURCE_DIR`); with the published module they fail until re-pin. Two timing
  flakes (`lease fence ... expired before renewal`, `inline_permissions`) were also seen on upstream/main.
- Pre-existing on upstream/main: `cargo fmt --check` drift in two test files (fixed here), two clippy lints
  (fixed here), 6 files with no coverage records (undeclared upstream modules), 24 nextest failures.

## Decisions taken in the merge

- Upstream per-conversation inbound debounce was ported onto the epic dispatch (debouncer only delays/merges
  intake; the TinyAgents `TurnRunner` still owns queueing).
- Voice-note audio now lives in the storage port's BlobStore (the storage-bypass ratchet forbids new fs writers).
- Storage-schema lint exempts TinyChannels' declared legacy migration source collection names.
- `app/.prettierignore` lists the generated `threadWire.generated.ts` like `chatWire.generated.ts`.
- The wiki patch (`docs/handoffs/7964-wiki-handoff.patch`) stays deferred to the final upstream stage.
