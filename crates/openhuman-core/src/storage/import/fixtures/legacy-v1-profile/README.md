# Legacy v1 profile storage fixture

`storage.db` is a semantic profile fixture in the physical SQLite format of
the historical public `tinystoragedrivers` v1 API at
`0c6dcfce57201fadd830431f36fd759c3baa897a`. The standalone generator writes it
through that API's `StorageBackend::for_scope`, `DocumentStore`, and
`put_with` methods; it does not create or edit TSD tables directly.

The generator's serializable wire structs mirror the historical OpenHuman
owner models and writer transformations from `2d3e22636e^`:

- `platform/cost/types.rs::CostRecord` and `TokenUsage` are written to
  `cost_records`, including the owner timestamp and `ts_ms`.
- `agent/artifacts/types.rs::ArtifactMeta` is written to `artifacts` with the
  old writer's `created_at` to `created_ms` conversion; its
  `artifact_args` document uses the old `{ "args": ... }` wrapper.
- `profiles/types.rs::ProfileMeta` is written to the `cluster` scope's
  `profiles` collection with `created_at` removed from the body and retained
  as TSD `created_ms` metadata, matching `ProfileRegistry::body_of`.
- The isolated desktop profile database stores owner documents in the
  historical `local` scope; the profile directory itself supplied user
  isolation. `security/approval` writes a pending `approvals` record there;
  the boot owner converter must retain it as `approvals.requests`.
- `threads/store` writes a legacy `threads_transcript_keys` record in that
  scope; the boot owner converter must retain it as
  `threads.transcript_keys`.
- The TinyFlows catalog writes a `flows_definitions` record in the same
  per-profile database. Desktop boot must migrate it through the TinyFlows
  catalog owner and the public `flows_list` operation must return it.
- `secrets` contains an encrypted `fixture-secret` document in the same scope
  using the fixture-only master key `[0x5a; 32]`; the integration test reads it
  with TinyStorage's `DocumentSecrets` API after desktop boot.
- The generator also emits the retired workspace conversation log at
  `legacy-workspace/memory/conversations/threads.jsonl`; the host E2E copies it
  to the profile workspace and confirms the thread owner imports it read-only.

The host E2E test copies this database to an existing profile's
`users/u1/workspace/storage/storage.db`, boots the desktop host with layout
auto-migration enabled, and reads threads, approvals, secrets, and flows
through their owner APIs. It compares the legacy database bytes before and
after boot. The focused core import test continues to verify the owner schema
conversions for cost, artifacts, and profile metadata, as well as
non-resurrection after deletion.

The standalone generator is under `generator/`; it is not a workspace member
or product dependency. Regenerate a fresh database at a caller-selected path:

```sh
cargo run --locked --manifest-path crates/openhuman-core/src/storage/import/fixtures/legacy-v1-profile/generator/Cargo.toml -- /absolute/path/to/storage.db
```

The checked-in database SHA-256 is
`0519e2d0d32ec3690182a243f7ea1469f66c6a4c160a94718c391ca3cde701b1`.
Regenerate and verify it with `sha256sum` when updating the fixture.
