# S152COGARCSEE-006: Observer archetype rendering

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (observer/tooling only)
**Deps**: archive/tickets/S152COGARCSEE-002.md, archive/tickets/S152COGARCSEE-005.md

## Problem

Archetypes are only useful if they are inspectable (FND-29). The observer should surface each agent's archetype in the run-metadata agent table and in decision-history narrative context, e.g. "Agent A (Cautious) …".

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The observer binary is `crates/worldwake-cli/src/bin/observer.rs`. Section 1's actual heading is `## Section 1 — Run Metadata` (`observer.rs:4601`) with an agent table rendered as `| Name | EntityId |` (the spec's original "Agent Overview" wording was corrected during reassessment). Section 3b's heading is `## Section 3b — Decision History` (`observer.rs:935`) with context lines appended via producers like `goal_committed_context_lines` (`observer.rs:966`).
2. `CognitiveArchetypeComponent` (ticket 002) is read via the schema-generated getter on the world; archetype values are populated by ticket 005. The observer reads authoritative component state (a co-located read of the agent's own component — no belief layer involved).
3. Tooling-only ticket: items 1-3 suffice. The proof surface is a headless observer render test rather than engine trace/event-log surfaces.

## Architecture Check

1. The observer reads the existing component getter and formats it — no new state, no engine change. Format follows the existing table/context conventions in `observer.rs` so the output stays consistent with sibling sections.
2. No backwards-compatibility concern: purely additive rendering.

## Verification Layers

1. Section 1 agent row includes the archetype -> headless observer render test asserting the `Archetype` column value (rendered-output surface).
2. Section 3b narrative includes archetype context -> headless render test asserting the context substring.
3. Single tooling layer; engine trace/event-log surfaces are not applicable because no simulation state is mutated.

## What to Change

### 1. Section 1 — Run Metadata agent table

Add an `Archetype` column: `| Name | Archetype | EntityId |`, reading `CognitiveArchetypeComponent.archetype` per agent.

### 2. Section 3b — Decision History context

Add archetype context to the per-agent narrative rendering alongside the existing context-line producers (e.g. an `archetype_context_line`-style helper or inline prefix).

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — Section 1 table, Section 3b context, tests)

## Out of Scope

- Rendering the `PersonalityAssigned` event row specifically (the event renders automatically through existing `EventTag` iteration; this ticket adds the per-agent archetype label, not event-row formatting).
- Any engine/state change.

## Acceptance Criteria

### Tests That Must Pass

1. Section 1 agent table renders the archetype for each agent.
2. Section 3b narrative includes the archetype label for an agent's decision lines.
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Observer reads only authoritative component state; it does not infer or mutate archetype (FND-29 inspection, not authority).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (`#[cfg(test)]`) — Section 1 column render + Section 3b context render.

### Commands

1. `cargo test -p worldwake-cli --bin observer`
2. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
3. `./scripts/verify.sh`
