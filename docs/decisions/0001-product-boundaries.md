# ADR 0001 — Product boundaries

- Status: accepted
- Date: 2026-10-06
- Phase: Fase 0

## Context

The ecosystem already has chatbots, IDEs, agents and repo-packers. A new generic wrapper would not defend a position.

## Decision

Build a **local context layer**, not an agent: discovery → secure scan → parse → index → hybrid retrieval → budgeted, provenanced context, consumed via CLI / JSON / MCP. Core never depends on a provider, a model, a cloud, or an agent implementation.

## Consequences

- No chat UI, no autonomous edits, no mandatory embeddings/LLM in v1.
- Adapters (OpenCode, Claude Code, Cursor, …) convert `Context API → consumer format`; they never own ranking.
- Success is measured by retrieval quality + token savings, not feature count.
