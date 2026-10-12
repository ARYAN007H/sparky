# Connected integration cache scope (#7433)

Canonical base: `3dbdd07e5f07d917dabbbc93fb0a9dd2a24a4347`.

The existing credential-only cache key collides when two profile contexts use
one configuration and service identity. Invalidation clears every entry, and
connection reconciliation compares every cached profile with one response.

## Executed baseline

The three sibling regressions use actual `CoreContext` profile overlays, an
identical fake pinned Composio credential, and the production cache read,
invalidation and reconciliation functions. They seed cached snapshots and make
no backend request; this is not an authenticated backend-flow proof.

The supported fixture graph builds successfully in 5m43s and executes all three:
zero passes, three behavioral failures, zero ignored. Alice's snapshot is
visible to Bob; Bob's invalidation removes Alice's snapshot; Bob's reconciliation
removes Alice's snapshot. The failures are recorded in
`.validation/cache-scope-red-supported.log`.

The first minimal-feature attempt fails before execution because three existing
fixtures require the MCP, modules and channels gates. That compilation failure
is not counted as RED.

## Candidate and pending acceptance

The candidate binds cache entries to the profile/operator storage identity,
preserves the existing credential digest, and uses per-scope epochs for fetch
admission. Desktop agents continue to share their operator's catalog. Missing
SaaS profiles fail closed, including before the disabled-mode shortcut.
Settings refresh preserves its caller context with `spawn_scoped`.

The original three assertions remain unchanged. Additional controls cover
in-flight fetch admission, missing SaaS profiles and desktop agent sharing.
The first GREEN attempt catches an obsolete helper import from the retained
branch; the canonical `contract::toolkit_description` path has been restored.
GREEN execution and broader acceptance remain pending.

Feature forwarding passes. Storage-bypass and Rust layout checks fail on files
unchanged from the canonical base: the browser-profile lock probe is reported as
a write, and three security files exceed the line limit. Exact base-to-candidate
diffs of the scripts and reported files are empty.

## Related boundaries

The storage `KEYS` cache holds the process master provider, while native secret
operations derive keys for their captured scopes. Source review found no tenant
data-key cache there. Existing explicit-provider tests do not establish the
complete production `current()` path.

Legacy secret adoption still needs source ownership admission. Serialized
connection events also need trustworthy profile propagation for scoped cache
refresh; a missing profile is refused. This bounded cache change does not
complete the singleton inventory or the cloud-storage epic.
