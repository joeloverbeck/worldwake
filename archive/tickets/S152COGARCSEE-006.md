# S152COGARCSEE-006: Observer archetype rendering

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (observer/tooling only)
**Deps**: archive/tickets/S152COGARCSEE-002.md, archive/tickets/S152COGARCSEE-005.md

## Problem

Archetypes are only useful if they are inspectable (FND-29). Before this ticket, the observer did not surface each agent's archetype in the run-metadata agent table or in decision-history narrative context, e.g. "Agent A (Cautious) …".

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The observer binary is `crates/worldwake-cli/src/bin/observer.rs`. Before this ticket, Section 1's actual heading was `## Section 1 — Run Metadata` (`observer.rs:4601`) with an agent table rendered as `| Name | EntityId |` (the spec's original "Agent Overview" wording was corrected during reassessment). Section 3b's heading was `## Section 3b — Decision History` (`observer.rs:935`) with context lines appended via producers like `goal_committed_context_lines` (`observer.rs:966`).
2. `CognitiveArchetypeComponent` (ticket 002) is read via the schema-generated getter on the world; archetype values are populated by ticket 005. The observer reads authoritative component state (a co-located read of the agent's own component — no belief layer involved).
3. Tooling-only ticket: items 1-3 suffice. The proof surface is a headless observer render test rather than engine trace/event-log surfaces.

## Architecture Check

1. The observer reads the existing component getter and formats it — no new state, no engine change. Format follows the existing table/context conventions in `observer.rs` so the output stays consistent with sibling sections.
2. No backwards-compatibility concern: purely additive rendering.

## Verified Layers

1. Section 1 agent row includes the archetype -> `tests::format_report_renders_archetype_in_agent_table` asserts the `Archetype` column value (rendered-output surface).
2. Section 3b narrative includes archetype context -> `tests::render_decision_history_section_appends_archetype_to_agent_name` asserts the agent label substring.
3. Existing observer decision-history fixture includes archetype-qualified agent labels for `survival-baseline.ron`.
4. Single tooling layer; engine trace/event-log surfaces are not applicable because no simulation state is mutated.

## Landed Changes

### 1. Section 1 — Run Metadata agent table

Added an `Archetype` column: `| Name | Archetype | EntityId |`, reading `CognitiveArchetypeComponent.archetype` per agent.

### 2. Section 3b — Decision History context

Added archetype context to decision-history agent labels, e.g. `Agent A (Opportunistic)`, using the same component read path as the Section 1 table.

## Landed Files

- `crates/worldwake-cli/src/bin/observer.rs` — Section 1 table, Section 3b agent labels, and bin-local tests.
- `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` — refreshed expected decision-history output.
- `docs/generated/scenario-coverage.md` — regenerated after `./scripts/verify.sh` detected scenario-coverage drift from the live cognitive-archetype feature.

## Out of Scope

- Rendering the `PersonalityAssigned` event row specifically (the event renders automatically through existing `EventTag` iteration; this ticket adds the per-agent archetype label, not event-row formatting).
- Any engine/state change.

## Acceptance Result

### Tests That Passed

1. Section 1 agent table renders the archetype for each agent.
2. Section 3b narrative includes the archetype label for an agent's decision lines.
3. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. Observer reads only authoritative component state; it does not infer or mutate archetype (FND-29 inspection, not authority).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (`#[cfg(test)]`) — Section 1 column render + Section 3b context render.
2. `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` — existing observer fixture refreshed for archetype-qualified agent labels.

## Outcome

Completed on 2026-05-20.

- Observer Section 1 now renders `| Name | Archetype | EntityId |`.
- Observer Section 3b now renders decision-event agent labels with archetype context when the agent has a `CognitiveArchetypeComponent`.
- The existing observer decision-history golden fixture was refreshed to the new rendered output.
- `docs/generated/scenario-coverage.md` was regenerated after the full verification wrapper detected drift from the live cognitive-archetype feature.

## Deviations

- The decision-history rendering landed as an agent-label suffix rather than a separate context row because the ticket's example was `Agent A (Cautious)` and this keeps the table compact.
- Scenario-coverage generated-doc refresh was not part of the original file list, but the live `./scripts/verify.sh` wrapper requires it and the drift is in the same S152 cognitive-archetype feature surface.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer -- --list`
- Passed `cargo test -p worldwake-cli --bin observer tests::format_report_renders_archetype_in_agent_table -- --exact`
- Passed `cargo test -p worldwake-cli --bin observer tests::render_decision_history_section_appends_archetype_to_agent_name -- --exact`
- Passed `cargo test -p worldwake-cli --bin observer`
- Passed `cargo test -p worldwake-cli --test observer_decision_history survival_baseline_decision_history_section_matches_golden -- --exact`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
- Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --write`
- Passed `./scripts/verify.sh`
