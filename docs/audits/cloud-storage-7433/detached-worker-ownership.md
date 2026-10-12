# Detached-worker profile ownership

This bounded host correction starts from canonical
`3dbdd07e5f07d917dabbbc93fb0a9dd2a24a4347`. It addresses the process-local
detached-worker ownership boundary in #7433. It does not consume new TinyAgents
capabilities or change storage declarations, credentials, dependencies, or
the connected-integration cache.

## Boundary

The host adapts TinyAgents' opaque registry owner string using the existing
injective profile/session encoding. Registry ownership includes the original
profile and parent session. Durable task records, completion routing and
public responses retain the original unencoded session ID.

Registration captures the actual scoped profile. SaaS work without a task
context or admitted profile is refused; an unowned registration aborts the
spawned task before recording it. Live rosters, resolution, wait and agent
steering use the caller's profile/session key. Durable fallback paths also
require the caller's actual captured workspace.

Public user controls do not require a parent-session argument. They compare
the worker's captured profile with the caller's profile, then use TinyAgents'
owned cancellation/steering APIs with the snapshot's exact owner. Thread
stop/delete/purge filters retain their workspace restriction and also check
the captured profile. Desktop public controls retain the process user's
authority over all workers.

TinyAgents continues to own the registry, status channels, abort/cancellation
mechanics and steering handles. The host change supplies ownership metadata
and admission checks around those APIs.

## Regression evidence

Test-only checkpoint `26a7611387` executed against the uncorrected production
source through cancel-aware session `38040`. Compilation succeeded in 8m06s;
the seven selected tests finished with **1 passed, 6 failed, 0 ignored**.
The desktop control passed. All six tenant tests failed on actual behavior:

- identical parent-session IDs exposed both profiles' live workers;
- a foreign worker resolved under the other profile;
- foreign agent steering succeeded;
- foreign public steering and cancellation returned success;
- a missing SaaS task scope could read the live roster.

Each SaaS case uses a fresh libtest subprocess because process mode is fixed
by a `OnceLock`. The fixtures use actual `CoreContext` profile overlays,
distinct workspaces, live pending Tokio tasks, status watches and public
registered controller handlers. The parent requires both child exit success
and a one-test success result, preventing an empty test filter from passing.

Corrected source `9b1191ee4e` ran through session `65302`. Compilation succeeded
in 2m46s; the complete detached-worker suite finished with **29 passed,
0 failed, 0 ignored**, including the original seven cases and existing
desktop cancellation, steering, wait, roster, durable fallback and wire-shape
controls. The original failing assertions were retained. The missing-scope
case additionally checks a scoped kernel context without a profile.

The separate public-controller compatibility suite finished with **5 passed,
0 failed, 0 ignored**. These 34 results are distinct top-level tests; the seven
subprocess repetitions are not added to the count. Both runs used:

```sh
scripts/ci-cancel-aware.sh cargo test --locked -j 1 -p openhuman \
  --no-default-features --features storage-sqlite,mcp,modules,channels \
  --lib agent::orchestration::running_subagents::tests \
  -- --nocapture --test-threads=1

scripts/ci-cancel-aware.sh cargo test --locked -j 1 -p openhuman \
  --no-default-features --features storage-sqlite,mcp,modules,channels \
  --lib agent::orchestration::subagent_control::tests \
  -- --nocapture --test-threads=1
```

Complete outputs were read and retained outside disposable build output in
`.validation/detached-tenant-red.log`,
`.validation/detached-tenant-green-and-compat.log` and
`.validation/detached-control-compat.log`. Independent source review found no
Critical or Important findings in the bounded ownership correction.

## Remaining limits

These results cover the supported feature graph above. They do not assert a
fresh full-workspace, all-feature, MSRV, coverage or hosted CI result.

Abort handles, steering queues and live status remain process-local. A shared
durable ledger does not route a request to another executor or provide a
distributed turn claim. Cross-core routing, native module releases, host
consumption, storage migration and complete cloud acceptance remain separate
work.
