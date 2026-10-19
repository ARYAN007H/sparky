# Agent goals

TinyAgents owns goal types, lifecycle, persistence, budgets, and prompt
rendering. This directory contains OpenHuman runtime adapters for explicit
thread selection, tool registration, and turn accounting.

Goals are controlled by the agent tools and runtime. The conversation UI reads
them through the `threads.goal_get` RPC and renders the thread goal panel.

## Further reading

- [Parent module README](../README.md)
- [Agent harness architecture](../../../../../docs/gitbooks/en/developing/architecture/agent-harness.md)
- [Goals and todos](../../../../../docs/gitbooks/en/features/goals-and-todos.md)
