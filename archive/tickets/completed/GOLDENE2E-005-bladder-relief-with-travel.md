# GOLDENE2E-005: Bladder Relief with Travel

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Possible
**Deps**: None

## Problem

The Relieve goal and PublicLatrine place are completely untested. An agent with high bladder pressure should recognize the need, travel to the latrine, and relieve. This tests the bladder need pathway end-to-end, including the travel-to-facility sub-plan.

**Coverage gap filled**:
- GoalKind: `Relieve` (completely untested)
- Need: Bladder (as driver)
- Topology: PublicLatrine (unused place)
- Cross-system chain: Metabolism → bladder escalation → Relieve goal → travel to PublicLatrine → relieve action → bladder decreases

## Assumption Reassessment (2026-03-12)

1. `GoalKind::Relieve` exists in `crates/worldwake-core/src/goal.rs` (confirmed).
2. `HomeostaticNeeds` has a `bladder` field (confirmed).
3. `MetabolismProfile` has a `bladder_rate` field (confirmed).
4. `PrototypePlace::PublicLatrine` exists in the topology (confirmed).
5. Candidate generation already emits `GoalKind::Relieve` when bladder crosses the low threshold in `crates/worldwake-ai/src/candidate_generation.rs` (confirmed).
6. A relieve action already exists as the `toilet` needs action in `crates/worldwake-systems/src/needs_actions.rs` (confirmed).
7. The current `toilet` action does **not** require a sanitation facility or latrine-tagged place. It can execute anywhere the actor is alive. The original ticket assumption about existing travel-to-latrine behavior was incorrect.
8. The current action/belief surface does not expose a reusable place-tag constraint. If this ticket enforces facility-based relief cleanly, it likely needs a small generic engine addition rather than a `PublicLatrine` special case.

## Architecture Check

1. The original ticket premise is still architecturally desirable: relief should be grounded in concrete world state rather than a context-free self-care action. Requiring a latrine-tagged place is more consistent with the foundations than allowing `toilet` anywhere.
2. The robust implementation is **not** to hardcode `PrototypePlace::PublicLatrine` into the action or planner. The clean design is to require any place tagged with `PlaceTag::Latrine`, letting topology drive the behavior.
3. This keeps the behavior extensible: additional latrines can be added later without changing AI or action code.
4. Fits in `golden_ai_decisions.rs` since it still tests needs-driven AI behavior through the real planner/runtime.
5. No shims or alias paths.

## What to Change

### 1. Add harness constants and helpers

In `golden_harness/mod.rs`:
```rust
pub const PUBLIC_LATRINE: EntityId = prototype_place_entity(PrototypePlace::PublicLatrine);

// In impl GoldenHarness:
pub fn agent_bladder(&self, agent: EntityId) -> Permille {
    self.world
        .get_component_homeostatic_needs(agent)
        .map_or(pm(0), |n| n.bladder)
}
```

Add a location helper only if it removes real duplication in the final test assertions. Prefer `World::effective_place()` directly over a redundant wrapper if the helper adds no reuse.

### 2. Write golden test: `golden_bladder_relief_with_travel`

In `golden_ai_decisions.rs`:
- Agent at Village Square with high bladder pressure (e.g., `pm(800)`), fast bladder metabolism.
- All other needs low.
- `PublicLatrine` is a separate reachable place connected to `VillageSquare` in the prototype topology.
- Run simulation for up to 100 ticks.
- Assert: agent's bladder decreases from initial high value.
- Assert: agent cannot satisfy the goal locally at `VillageSquare`; the scenario should observe either transit or later arrival at `PublicLatrine` before relief completes.
- Assert: agent reaches `PublicLatrine` at some point before or when bladder drops to zero.

**Expected emergent chain**: Bladder pressure → `Relieve` goal → planner finds reachable latrine-tagged place → travel action(s) → `toilet` action at latrine → bladder resets and waste appears there.

### 3. Add focused engine regression coverage if needed

If this ticket requires engine work, add narrow tests around the discovered engine limitation instead of relying only on the golden scenario. The most likely gap is the action-layer inability to express or validate place-tag requirements cleanly.

Expected focused coverage if engine work is needed:
- a needs/action test proving `toilet` is unavailable or invalid away from a latrine
- a planner/runtime or affordance test proving `toilet` becomes available at a latrine-tagged place
### 4. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P5 from Part 3 to Part 1.
- Update Part 2: Bladder need now tested, Relieve GoalKind tested, PublicLatrine used.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add `PUBLIC_LATRINE`, `agent_bladder()`, and only any additional helper that removes real duplication)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify — add test)
- `crates/worldwake-sim/src/*` (modify only if a generic place-tag constraint is needed to support facility-gated relief)
- `crates/worldwake-systems/src/needs_actions.rs` (modify only if the `toilet` action must enforce the new generic facility requirement)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Bladder as interrupt trigger during other actions
- Waste commodity production from relief (if applicable)
- Multiple relief cycles
- Bladder pressure competing with other critical needs (that's GOLDENE2E-010)

## Engine Discovery Protocol

This ticket is a golden e2e test that exercises emergent behavior through the real AI loop.
If implementation reveals that the engine cannot produce the expected emergent behavior,
the following protocol applies:

1. **Diagnose**: Identify the specific engine limitation (missing candidate generation path, planner op gap, action handler deficiency, belief view gap, etc.).
2. **Do not downgrade the test**: The test scenario defines the desired emergent behavior. Do not weaken assertions or remove expected behaviors to work around engine gaps.
3. **Fix forward**: Implement the minimal, architecturally sound engine change that enables the emergent behavior. Document the change in a new "Engine Changes Made" subsection under "What to Change". Each fix must:
   - Follow existing patterns in the affected module
   - Include focused unit tests for the engine change itself
   - Not introduce compatibility shims or special-case logic
4. **Scope guard**: If the required engine change exceeds this ticket's effort rating by more than one level (e.g., a Small ticket needs a Large engine change), stop and apply the 1-3-1 rule: describe the problem, present 3 options, recommend one, and wait for user confirmation before proceeding.
5. **Document**: Record all engine discoveries and fixes in the ticket's Outcome section upon completion, regardless of whether fixes were needed.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_bladder_relief_with_travel` — agent with high bladder pressure travels to a latrine-tagged place and relieves
2. Agent's bladder value decreases from initial high value
3. The scenario observes that relief is not executable at the non-latrine origin and is completed only after reaching a latrine-tagged place
4. If engine work is required, focused regression tests are added in the affected module(s)
5. Simulation completes within 100 ticks
6. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: Bladder need tested, Relieve GoalKind tested, PublicLatrine marked as used
7. Existing suite: `cargo test -p worldwake-ai --test golden_ai_decisions`
8. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Determinism: same seed produces same outcome
3. Agent remains alive throughout

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_bladder_relief_with_travel` — proves bladder relief pathway with travel
2. Additional focused engine regression test(s) only if a concrete action/planner limitation is discovered while implementing this ticket

## Outcome

### Completion date

2026-03-12

### What actually changed

- Added `golden_bladder_relief_with_travel` in `crates/worldwake-ai/tests/golden_ai_decisions.rs`.
- Added `PUBLIC_LATRINE` and `agent_bladder()` to `crates/worldwake-ai/tests/golden_harness/mod.rs`.
- Added `needs_actions::tests::toilet_affordance_requires_latrine_tagged_place`.
- Updated `reports/golden-e2e-coverage-analysis.md` to record the new bladder/relief/latrine coverage.

### Engine changes made

- Added a generic action-layer place constraint: `Constraint::ActorAtPlaceTag(PlaceTag)`.
- Extended the belief/planning surface to answer place tags so affordance queries, planner search, and authoritative validation all respect the same place-tag rule.
- Gated the `toilet` action on `PlaceTag::Latrine` instead of allowing relief anywhere.

### Deviations from the original plan

- The original ticket assumed the engine already required `PublicLatrine` for relief. That was incorrect; `toilet` was previously available at any place.
- The implementation deliberately did **not** hardcode `PrototypePlace::PublicLatrine` into the engine. It uses the generic `PlaceTag::Latrine` constraint instead, which is cleaner and more extensible.
- A downstream integration test in `crates/worldwake-systems/tests/e09_needs_integration.rs` also needed updating because it encoded the old anywhere-toilet assumption.

### Verification results

- `cargo test -p worldwake-systems --lib`
- `cargo test -p worldwake-ai --test golden_ai_decisions`
- `cargo test -p worldwake-systems --test e09_needs_integration scheduler_driven_care_actions_apply_effects_and_preserve_conservation`
- `cargo test --workspace`
- `cargo clippy --workspace`

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_bladder_relief_with_travel`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
