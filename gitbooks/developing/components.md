---
description: >-
  The [components] config table: one switch per tiny dependency, how it narrows
  a host, and how legacy flags migrate.
icon: toggle-on
---

# Components

`config.toml` has a `[components]` table with one boolean per tiny dependency. All default to `true`.

```toml
[components]
flows = true       # tinyflows: saved workflows
memes = true       # tinymemes
documents = true   # tinydocs
search = true      # tinysearch
mcp = true         # tinymcp
connectors = true  # tinyconnectors (Composio)
hosting = true     # tinyhosts
juice = true       # tinyjuice
memory = true      # tinymemory
wallet = true      # tinywallet (web3, x402)
computer = true    # tinycomputer
sandbox = true     # tinybox
channels = true    # tinychannels
skills = true      # tinyskills
voice = true       # tinyvoice
```

Each switch also reads `OPENHUMAN_COMPONENT_<NAME>=0|1` (for example `OPENHUMAN_COMPONENT_VOICE=0`); the environment wins over the file.

## What a switch does

The switches only narrow. The core resolves them into a component set and computes `effective = host choice ∩ components` for the domain families, background services and tool groups. A host preset (`harness`, `kernel`, `saas`, ...) decides what may exist; a switch can remove something from it, never add something back.

A method whose component is off answers `COMPONENT_DISABLED:<name>` (JSON-RPC `error.message`, with `error.data = {"kind": "component_disabled", "component": "<name>"}`) instead of `unknown method`, so the UI and CLI can explain why. A method that the host simply left out still looks unregistered.

Loadable modules follow the same switches. A module whose component is off is never downloaded, mapped or registered: `modules.list` reports its state as `disabled`, boot skips it (including a matching artifact on the module search path), and loading it returns `COMPONENT_DISABLED:<name>`. `about_app` marks capabilities backed by a disabled component with status `disabled`.

A flag change takes effect on the next core start. There is no hot toggling.

Each component's domain wires itself to its switch (tools, services and RPC) in its own change; until then a switch removes only the families, services and tool groups listed in the core's component set.

## Not toggleable

The kernel dependencies (tinyagents, tinytools, tinyinference, tinybus) are always on. `[modules] enabled` remains the global kill switch for loadable native modules.

## Legacy flags

The first load of an older config moves these once into `[components]`, and the next save writes the result:

| Legacy setting | Becomes |
| --- | --- |
| `[hosting] enabled = false` | `components.hosting = false` |
| `[search] enabled = false` | `components.search = false` |
| `[mcp_client] enabled = false` | `components.mcp = false` |
| `[memory] engine = "none"` | `components.memory = false` (`engine` is kept) |
| `OPENHUMAN_TINYMEMES` set to an off value | `components.memes = false` |
| `[composio] enabled` | dropped; it was never read |

A `components.*` key already present in the file always wins over its legacy flag, so the migration is idempotent and turning a component back on sticks.

## Wallet

`wallet = false` removes the `web3` domain family and tool group, so `wallet.*`, `web3.*` and `x402.*` answer `COMPONENT_DISABLED:wallet`. The x402 payment hook behind `http_request` declines before any quote or signing (the 402 response comes back unpaid), and `tinywallet` is never downloaded, attested or loaded. Keychain material, spending limits (`OPENHUMAN_X402_*_MAX`) and RPC endpoints (`OPENHUMAN_WALLET_RPC_*`) are left as they are, so setting `wallet = true` again restores the wallet on the next core start.
