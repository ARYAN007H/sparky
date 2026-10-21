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

The focused import test reads this checked-in database through the real boot
import pipeline, then deserializes the migrated cost and artifact records into
their owner types and reads the profile through `ProfileRegistry`. It checks
that the source database hash is unchanged and that deleting an imported
record survives a repeated boot import. This proves the TSD document inventory
and owner schema conversions for these records. Secrets, module state, and
workspace artifact files use separate production import paths and are not
covered by `run_all` in this test.

The standalone generator is under `generator/`; it is not a workspace member
or product dependency. Regenerate a fresh database at a caller-selected path:

```sh
cargo run --locked --manifest-path crates/openhuman-core/src/storage/import/fixtures/legacy-v1-profile/generator/Cargo.toml -- /absolute/path/to/storage.db
```

The checked-in database SHA-256 is
`df0d66c4763f7ab95b71242ebcb75479caf56b863e5893dea228124952c90a40`.
