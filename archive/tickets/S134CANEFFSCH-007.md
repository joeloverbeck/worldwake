# S134CANEFFSCH-007: Travel, patrol, and bandit camp schemas

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — replaces empty-placeholder schemas with category-owned `EffectSchema` literals in travel, patrol, and establish_camp actions and switches their commit handler bodies to `apply_effects_with_context(..., Authoritative)`
**Deps**: archive/tickets/S134CANEFFSCH-001.md, archive/tickets/S134CANEFFSCH-002.md

## Problem

S134 deliverable D5 requires migrating movement and positioning actions — `travel` (in `travel_actions.rs`), `patrol` (in `patrol_actions.rs`), and `establish_camp` (in `bandit_camp_actions.rs`) — to declarative `EffectSchema` evaluation. Travel exercises the place-graph traversal substrate (edge-time consumption, arrival event emission). Patrol exercises route-following and commit-time waypoint advancement. Establish_camp exercises bandit-camp creation/reuse and supply-transfer semantics. The planner continues to use the old `apply_hypothetical_transition` path (with `Travel`, `Patrol`, `EstablishCamp` all routing to `GoalModelFallback` per `planner_ops.rs:188–199`); goldens for these actions must preserve behavior.

## Assumption Reassessment (2026-05-04)

1. Movement registrations live at `crates/worldwake-systems/src/travel_actions.rs` via `register_travel_actions`, `crates/worldwake-systems/src/patrol_actions.rs` via `register_patrol_action`, and `crates/worldwake-systems/src/bandit_camp_actions.rs` via `register_establish_camp_action`.
2. After ticket 001, each `ActionDef` literal has `effect_schema: EffectSchema::empty()`. This ticket populates real schemas.
3. Travel is duration-bearing: the existing handler updates `ActionInstance.local_state: Option<ActionState>` with the `Travel { edge_id, origin, destination, departure_tick, arrival_tick }` variant during the action's lifetime, with the commit happening on arrival. The schema must encode the arrival commit effect while preserving start/tick/abort state handling. The duration field already lives on `ActionDef.duration: DurationExpr` — preserved.
4. Patrol is route-following with an imperative start/tick shell and commit-time route-index advancement. The schema owns the commit-time route advance; there is no periodic perception step in the live patrol handler to migrate in this ticket.
5. Establish_camp reuses or creates a camp supply container, sets `BanditCamp` on the place when absent, transfers controlled edible supplies, updates ownership/container relations, and records transfer provenance. This is category-owned domain aftermath, not a generic entity-creation primitive.
6. Existing focused/unit coverage:
   - `travel_actions.rs`, `patrol_actions.rs`, `bandit_camp_actions.rs` `#[cfg(test)]` blocks
   - Goldens — live relevant binaries are `golden_travel_physiology.rs` and `golden_survival_patrol.rs`; there is no current bandit-camp-specific golden binary.
   - Conformance test `conformance_travel` at `planner_conformance.rs:932`.
7. Shared abstraction boundary under audit: category-owned authoritative commit effects behind `ActionDef.effect_schema`. Travel and patrol must produce identical `Place`/route mutations pre- and post-ticket; establish_camp must produce identical `BanditCamp`, supply-container, ownership/container, and provenance effects.

## Architecture Check

1. Live reassessment rejected generic `Move`/`CreateEntity` sketches for this slice. Travel arrival, patrol route advancement, and bandit-camp establishment carry category-specific lifecycle state and aftermath that must remain owned by their action modules.
2. The landed schema language adds `EffectStep::CompleteTravel`, `EffectStep::AdvancePatrolRoute`, and `EffectStep::EstablishBanditCamp`. Unsupported sinks reject these by default with `Discrepancy::ImproperPlanningState`; the local authoritative sinks override only the category they own.
3. Planner hypothetical mode remains on the old `GoalModelFallback` path until ticket 010 owns parity for category-specific staged steps.

## Verification Layers

1. Behavior-preservation invariant → focused action tests and live travel/patrol goldens preserve travel arrival, patrol route advancement, and bandit-camp supply-transfer semantics.
2. Place-mutation invariant → action trace: travel commit produces identical `Place` component delta and arrival event ordering pre- and post-ticket.
3. Bandit-camp invariant → `establish_camp` produces identical camp component, supply-container, ownership/container, and transfer-provenance results.
4. Conformance-tests parity invariant → `conformance_travel` continues to pass, comparing imperative authoritative path (now schema-driven) against `apply_hypothetical_transition` (unchanged) — both must match byte-for-byte.
5. AI integration invariant → relevant travel/patrol golden suites and the full `worldwake-ai` package suite pass.

## What to Change

### 1. Construct `EffectSchema` literal for travel

Use `EffectStep::CompleteTravel`. The category-owned travel sink delegates to the existing arrival mutation seam: clear transit state, move actor and direct possessions to the destination, emit movement evidence at the origin, update route experience from the event log, and reinforce exploration-arrival belief. The duration is on `ActionDef.duration` — not in the schema. The tick-time edge-traversal state is in `ActionInstance.local_state` — not in the schema.

### 2. Construct `EffectSchema` literal for patrol

Use `EffectStep::AdvancePatrolRoute`. The category-owned patrol sink delegates to the existing commit-time validation and `PatrolRoute.current_index` advancement. Start/tick behavior remains imperative.

### 3. Construct `EffectSchema` literal for establish_camp

Use `EffectStep::EstablishBanditCamp`. The category-owned bandit-camp sink delegates to the existing validation/container/supply-transfer seam rather than adding a generic `CreateEntity` step.

### 4. Replace commit handler bodies with `apply_effects` delegation

Each `commit_*` handler delegates through `apply_effects_with_context(..., EffectMode::Authoritative)`. Tick handlers (which carry duration-bearing state for travel and patrol) remain imperative for now — they're not in scope for the schema language unless ticket 010 surfaces a need.

## Files to Touch

- `crates/worldwake-systems/src/travel_actions.rs` (modify)
- `crates/worldwake-systems/src/patrol_actions.rs` (modify)
- `crates/worldwake-systems/src/bandit_camp_actions.rs` (modify)
- `crates/worldwake-sim/src/effect_schema.rs` (modify for category-owned `EffectStep` variants and default-rejecting `EffectSink` methods)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` and `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (no direct edit; default `EffectSink` methods reject the new category-owned steps outside local authoritative sinks)

## Out of Scope

- Migrating non-movement actions (tickets 003, 004, 005, 006, 008, 009).
- Switching the planner search to `apply_effects(..., Hypothetical)` (ticket 010).
- Migrating tick-time handler logic for travel/patrol — only commit-time effects are in scope; tick-time edge-traversal state remains in `ActionInstance.local_state`.
- Changing place-graph traversal semantics or `DurationExpr` evaluation.

## Acceptance Criteria

### Tests That Must Pass

1. Travel-touching and patrol-touching goldens produce unchanged behavior at the live golden surfaces; no separate bandit-camp golden binary exists on the live branch.
2. Conformance test `conformance_travel` continues to pass.
3. Focused `worldwake-systems` module tests for travel, patrol, and bandit_camp pass, plus full `cargo test -p worldwake-systems`.
4. `worldwake-ai` travel/patrol golden binaries and full package tests pass. The package-level `golden_survival` filter is auxiliary only on the live branch because it runs one matching non-ignored test and does not execute ignored scenario-backed survival cases.
5. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Travel arrival produces the same `Place`-component mutation timing as today — duration semantics on `ActionDef.duration` are unchanged.
2. Patrol's tick-time behavior is unchanged; only commit-time route advancement goes through the schema.
3. `establish_camp` produces the same camp/supply transfer state changes.
4. Planner hypothetical behavior remains old-path until ticket 010.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/travel_actions.rs` `#[cfg(test)]` block — modify existing tests to exercise schema-driven commit path; verify duration and tick-time behavior unchanged.
2. `crates/worldwake-systems/src/patrol_actions.rs` and `bandit_camp_actions.rs` — analogous modifications.
3. Existing goldens — no source change.

### Commands

1. `cargo test -p worldwake-systems --lib travel_actions::tests`
2. `cargo test -p worldwake-systems --lib patrol_actions::tests`
3. `cargo test -p worldwake-systems --lib bandit_camp_actions::tests`
4. `cargo test -p worldwake-systems`
5. `cargo test -p worldwake-ai --test planner_conformance conformance_travel`
6. `cargo test -p worldwake-ai --test golden_travel_physiology`
7. `cargo test -p worldwake-ai --test golden_survival_patrol`
8. `cargo test -p worldwake-ai --test golden_survival_patrol -- --ignored`
9. `cargo test -p worldwake-ai`
10. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-05.

- Added category-owned movement/camp schema steps: `CompleteTravel`, `AdvancePatrolRoute`, and `EstablishBanditCamp`.
- Registered non-empty schemas for `travel`, `patrol`, and `establish_camp`.
- Replaced the three commit handlers with authoritative `apply_effects_with_context` delegation through local sinks that call the existing mutation boundaries.
- Added registration assertions that each action definition now carries the expected schema step.

## Deviations

- The draft's generic `Move` and `CreateEntity` sketches were superseded. Travel arrival uses action-local `ActionState` and event-log-derived route experience; establish_camp owns supply-container creation/reuse, `BanditCamp` writes, supply transfer, ownership, and provenance. Those are category-owned effects, not generic effect primitives.
- Patrol has no live periodic perception step to migrate in this ticket; the schema covers commit-time route advancement only.
- `SAVE_FORMAT_VERSION` did not change. The new schema steps live on in-memory `ActionDef` registry data and add no persisted world/runtime state.
- The drafted combined Cargo command with multiple filters was replaced by valid focused module commands. The drafted `golden_patrol`/bandit-camp golden names do not exist as live binaries; verification used the exact live travel/patrol golden binaries and full AI package test instead.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib travel_actions::tests -- --list` (18 listed tests).
- Passed `cargo test -p worldwake-systems --lib travel_actions::tests`.
- Passed `cargo test -p worldwake-systems --lib patrol_actions::tests -- --list` (9 listed tests).
- Passed `cargo test -p worldwake-systems --lib patrol_actions::tests`.
- Passed `cargo test -p worldwake-systems --lib bandit_camp_actions::tests -- --list` (9 listed tests).
- Passed `cargo test -p worldwake-systems --lib bandit_camp_actions::tests`.
- Passed `cargo test -p worldwake-systems`.
- Passed `cargo test -p worldwake-ai --test planner_conformance conformance_travel -- --list` (1 listed test).
- Passed `cargo test -p worldwake-ai --test planner_conformance conformance_travel`.
- Passed `cargo test -p worldwake-ai --test golden_travel_physiology`.
- Passed `cargo test -p worldwake-ai --test golden_survival_patrol`.
- Passed `cargo test -p worldwake-ai --test golden_survival_patrol -- --ignored`.
- Passed `cargo test -p worldwake-ai golden_survival` as an auxiliary live-filter check; it executed one matching non-ignored test.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `cargo test -p worldwake-ai`.
- Passed `git diff --check` after final ticket/spec Markdown edits.
- Passed `cargo fmt --all -- --check`.
