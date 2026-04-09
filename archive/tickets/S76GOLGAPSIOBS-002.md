# S76GOLGAPSIOBS-002: Golden S76-C — perception forms resource source beliefs

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

No golden test verifies that the perception-to-belief pipeline preserves beliefs about resource source entities (apple sources, water sources) when competing with ground items (waste) for belief capacity. S77 implemented `enforce_capacity()` prioritization, but there is no regression guard verifying resource source beliefs survive the pipeline. Without this, future changes to perception or belief capacity could silently break all resource-seeking behavior.

## Assumption Reassessment (2026-04-09)

1. `AgentBeliefStore.known_entities` exists at `crates/worldwake-core/src/belief.rs:49` as `BTreeMap<EntityId, BelievedEntityState>`. This is the field the assertion targets.
2. `enforce_capacity()` exists at `crates/worldwake-core/src/belief.rs:178` as a method on `AgentBeliefStore` taking `(&mut self, profile: &PerceptionProfile, current_tick: Tick)`. S77 (archived at `archive/specs/S77-belief-capacity-prioritization.md`) implemented the prioritization logic this test guards.
3. Shared boundary: golden harness + perception system. No production code changes.
4. S77 completion is the motivating invariant: resource source beliefs must survive `enforce_capacity()` eviction in the presence of ground items.
5. Ticket/spec setup drift: the live prototype world does not expose a `FarmVillage` place. The closest lawful setup is to use a real prototype place and add facility-backed `ResourceSource` entities there through the golden harness.
6. Ticket/spec wording drift: the harness creates resource-source entities as `EntityKind::Facility` with `WorkstationMarker` + `ResourceSource` via `place_workstation_with_source(...)`. There is no separate source-only entity helper, and no water-specific workstation tag is required for this perception-focused coverage.
7. Scenario isolation: ground waste items are present to create capacity competition. Resource sources are the target. No planning or action execution needed — this tests perception only. `golden_perception_exposure.rs` scenarios 116-119 test perception modulation (fidelity, concealment, fatigue, attention cost) — none verify resource source belief survival.

## Architecture Check

1. Adding to `golden_perception_exposure.rs` (321 lines) is appropriate — same domain (perception/belief pipeline), moderate file size.
2. No backwards-compatibility shims. Tests only.

## Verification Layers

1. Resource source beliefs present in `known_entities` -> authoritative belief state (`AgentBeliefStore.known_entities` contains apple source and water source entity IDs)
2. Belief capacity competition -> authoritative belief state (resource source beliefs survive despite ground item competition)
3. Deterministic replay -> authoritative belief state equality across two runs with same seed
6. Single-layer ticket (golden E2E tests only). No production code changes.

## What to Change

### 1. Implement S76-C scenario runner

Add to `crates/worldwake-ai/tests/golden_perception_exposure.rs`:

Create `run_perception_forms_resource_source_beliefs(seed: Seed)` returning an observation struct:

- Use the live prototype place `OrchardFarm`.
- Place 2 facility-backed resource sources there: 1 apple source and 1 water source.
- Spawn several ground waste items at `OrchardFarm` to create belief capacity competition.
- Spawn 1 AI agent at `OrchardFarm` with an explicit bounded `PerceptionProfile`.
- Run for 50 ticks.
- Collect: whether `known_entities` contains the apple source entity ID and the water source entity ID, plus the retained known-entity IDs for deterministic replay.

### 2. Implement S76-C test and replay companion

```rust
// Scenario S76-C: Perception Forms Beliefs About Resource Sources
#[test]
fn golden_perception_forms_resource_source_beliefs() { ... }

#[test]
fn golden_perception_forms_resource_source_beliefs_replays_deterministically() { ... }
```

Use `Seed([178; 32])`. Assert `known_entities` contains both the apple source and water source entity IDs after 50 ticks. If the belief store contains only waste/ground-item beliefs and no resource source beliefs, the test fails.

## Files to Touch

- `crates/worldwake-ai/tests/golden_perception_exposure.rs` (modify)

## Out of Scope

- Fixing belief capacity prioritization logic (S77, already completed)
- Planner fallback testing (S76GOLGAPSIOBS-001)
- Utility profile diversity testing (S76GOLGAPSIOBS-003)
- Observer tooling enhancements (S78, already completed)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_perception_forms_resource_source_beliefs` — agent's `known_entities` contains apple source and water source entity IDs after 50 ticks at `OrchardFarm`, even though the memory capacity is smaller than the total perceived entities
2. `golden_perception_forms_resource_source_beliefs_replays_deterministically` — identical observations across two runs
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No production code changes — engine behavior is unchanged
2. Deterministic replay: same seed produces identical observation structs
3. Resource source beliefs survive `enforce_capacity()` eviction in the presence of ground items

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_perception_exposure.rs::golden_perception_forms_resource_source_beliefs` — regression guard for perception-to-belief pipeline preserving resource source entities
2. `crates/worldwake-ai/tests/golden_perception_exposure.rs::golden_perception_forms_resource_source_beliefs_replays_deterministically` — determinism guard

### Commands

1. `cargo test -p worldwake-ai golden_perception_forms_resource_source_beliefs`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

Completed on 2026-04-09.

- Implemented the S76-C golden in `crates/worldwake-ai/tests/golden_perception_exposure.rs` with a live `OrchardFarm` setup, two facility-backed `ResourceSource` entities, and six competing waste lots under a bounded `PerceptionProfile`.
- The new observation runner proves both source beliefs survive `enforce_capacity()` eviction, and the replay companion compares the retained known-entity set directly for deterministic coverage.
- Deviations from original plan: corrected the stale `FarmVillage` / generic-source setup to the live `OrchardFarm` + facility-backed `ResourceSource` contract before coding; no production code changes were required.

## Verification

- `cargo test -p worldwake-ai golden_perception_forms_resource_source_beliefs -- --nocapture`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `python3 scripts/golden_inventory.py --write --check-docs`
