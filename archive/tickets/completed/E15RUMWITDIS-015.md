# E15RUMWITDIS-015: Remove Fabricated TellProfile Defaults From Runtime Belief Reads

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` belief-view behavior, Tell affordance enumeration, and planner snapshot/state shape
**Deps**: `archive/tickets/E14PERBEL-006.md`, `archive/tickets/E14PERBEL-009.md`, `archive/tickets/E15RUMWITDIS-012.md`, `specs/E15-rumor-witness-discovery.md`, `docs/FOUNDATIONS.md`

## Problem

`E15RUMWITDIS-012` hardened the authoritative Tell path so missing live-agent Tell information components fail structurally instead of being replaced with synthetic defaults. But the runtime/planner read path still fabricates a missing `TellProfile` as `TellProfile::default()`.

That leaves the architecture inconsistent:

1. authoritative Tell execution rejects a missing live-agent `TellProfile`
2. `PerAgentBeliefView` still reports a missing live-agent `TellProfile` as an apparently valid default profile
3. `PlanningSnapshot` then bakes that fabricated profile into planner state
4. Tell affordance enumeration can therefore advertise Tell behavior that the authoritative path would later reject

This violates the intended invariant that live-agent Tell behavior is driven by attached information components, not by hidden fallback substitution.

## Assumption Reassessment (2026-03-14)

1. `World::create_agent()` still attaches `AgentBeliefStore`, `PerceptionProfile`, and `TellProfile` by default in `crates/worldwake-core/src/world.rs`, so missing `TellProfile` is not a normal live-agent runtime case.
2. `RuntimeBeliefView::tell_profile()` already returns `Option<TellProfile>`, so the current architectural problem is not the trait signature itself; it is the `PerAgentBeliefView` implementation in `crates/worldwake-sim/src/per_agent_belief_view.rs`, which uses `unwrap_or_default()` and turns absence into `Some(TellProfile::default())`.
3. `PlanningSnapshot` in `crates/worldwake-ai/src/planning_snapshot.rs` currently stores `actor_tell_profile: TellProfile` and populates it from `view.tell_profile(actor).unwrap_or_default()`, so the fabricated profile is persisted into planning state rather than remaining a local affordance-query quirk.
4. Tell affordance enumeration in `crates/worldwake-systems/src/tell_actions.rs` still uses `view.tell_profile(actor).unwrap_or_default()`, so a structurally broken live agent can still enumerate Tell affordances through the planner/runtime path.
5. No active ticket in `tickets/` owns this runtime/planner invariant cleanup. `E15RUMWITDIS-013` is event-local perception snapshots, and `E15RUMWITDIS-014` is explicit belief-confidence policy data.
6. Existing tests already cover the current fabricated-default behavior in `crates/worldwake-sim/src/per_agent_belief_view.rs` (`tell_profile_returns_actor_default_when_component_missing`) and the positive Tell affordance path in `crates/worldwake-systems/src/tell_actions.rs` (`tell_affordances_*`), but `crates/worldwake-ai/src/planning_snapshot.rs` does not yet have a direct regression test for `actor_tell_profile`.

## Architecture Check

1. The clean fix is to make missing `TellProfile` remain absent across the runtime/planner read path instead of inventing a default. The trait already models absence with `Option`, so the code should use that honestly.
2. This is cleaner than widening `RuntimeBeliefView` into a `Result`-returning API in this ticket. The current gap is fabricated behavior, not the lack of structural error transport across the whole planner surface.
3. Tell affordance generation should treat missing `TellProfile` as “no legal Tell affordances,” not as “agent behaves like the default profile.” That preserves the existing soft query surface without lying about available behavior.
4. `PlanningSnapshot` and `PlanningState` should preserve the optionality of `TellProfile` rather than collapsing absence into a concrete default value. That keeps hypothetical planning state honest about what the source runtime view actually exposed.
5. No backwards-compatibility alias or dual behavior path should remain. The fabricated-default path should be removed outright once the optional snapshot/state flow exists.

## What to Change

### 1. Stop fabricating `TellProfile` in `PerAgentBeliefView`

Update `crates/worldwake-sim/src/per_agent_belief_view.rs` so:

```rust
fn tell_profile(&self, agent: EntityId) -> Option<TellProfile>
```

returns:

- `None` when `agent != self.agent`
- the actual attached `TellProfile` when present
- `None` when the actor lacks `TellProfile`

Do not use `unwrap_or_default()` here.

Also update the local tests in that module so they prove missing `TellProfile` stays absent instead of being converted into `Some(TellProfile::default())`.

### 2. Preserve Tell-profile absence in planner snapshot/state

Update `crates/worldwake-ai/src/planning_snapshot.rs` and `crates/worldwake-ai/src/planning_state.rs` so the planning layer preserves optional Tell-profile data:

- change `PlanningSnapshot.actor_tell_profile` from `TellProfile` to `Option<TellProfile>`
- populate it directly from `view.tell_profile(actor)` with no default substitution
- have `PlanningState::tell_profile()` return that optional snapshot value directly

This ticket should not redesign unrelated snapshot fields; only the Tell-profile path needs to become honest here.

### 3. Remove Tell affordance fallback defaults

Update `crates/worldwake-systems/src/tell_actions.rs` so `enumerate_tell_payloads()` no longer does:

```rust
let profile = view.tell_profile(actor).unwrap_or_default();
```

Instead:

- if `view.tell_profile(actor)` is `None`, return no Tell affordances
- otherwise continue using the actual profile for relay-depth and candidate-count filtering

Keep authoritative validation and commit behavior unchanged from `E15RUMWITDIS-012`; this ticket is about planner/runtime query honesty, not another authoritative-path redesign.

### 4. Lock the boundary down with regression tests

Add focused tests proving:

1. `PerAgentBeliefView` returns `None` when the actor lacks `TellProfile`
2. `PlanningSnapshot` preserves that absence instead of storing a default profile
3. `PlanningState` reports the same optional value the snapshot carries
4. Tell affordance enumeration returns no Tell affordances when the actor lacks `TellProfile`
5. valid agents with an attached `TellProfile` still enumerate the same Tell affordances as before

Use the existing affordance tests in `tell_actions.rs` as the baseline for the positive-path assertion; extend that module with a missing-profile regression rather than duplicating broad affordance coverage elsewhere.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)

## Out of Scope

- Redesigning `RuntimeBeliefView` to return `Result` for structural invariant failures
- Broad required-component auditing for every runtime/planner profile read
- Authoritative Tell validation or commit changes already completed in `E15RUMWITDIS-012`
- Event-local perception snapshot work in `E15RUMWITDIS-013`
- Belief-confidence policy work in `E15RUMWITDIS-014`
- Reworking unrelated planner snapshot fields into optional forms

## Acceptance Criteria

### Tests That Must Pass

1. `PerAgentBeliefView::tell_profile()` returns `None` when the acting agent lacks `TellProfile`
2. `PlanningSnapshot` stores `None` rather than `TellProfile::default()` when the source view lacks a Tell profile
3. `PlanningState::tell_profile()` returns the same optional value carried by its snapshot
4. Tell affordance enumeration returns no affordances when the actor lacks `TellProfile`
5. Tell affordance enumeration for valid agents is unchanged apart from removal of the fabricated-default path
6. Existing suite: `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Runtime/planner Tell behavior is driven only by actual attached `TellProfile` data, never by synthetic `TellProfile::default()` substitution
2. The runtime/planner read boundary preserves absence honestly: optional Tell-profile data stays optional through belief view, planning snapshot, and planning state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — modify `tell_profile_returns_actor_default_when_component_missing` to assert `None`, and keep coverage for the present-profile path
2. `crates/worldwake-ai/src/planning_snapshot.rs` — add the first direct snapshot regression for missing Tell-profile data so `actor_tell_profile` absence is exercised at snapshot-build time
3. `crates/worldwake-ai/src/planning_state.rs` — adjust `planning_state_preserves_actor_belief_memory_and_tell_profile_from_snapshot` and/or add a companion test so `RuntimeBeliefView::tell_profile(&state, actor)` mirrors the snapshot’s optional value for both present and missing-profile cases
4. `crates/worldwake-systems/src/tell_actions.rs` — keep the existing positive affordance tests and add a focused regression proving missing speaker `TellProfile` yields zero affordances rather than default-profile affordances

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-ai planning_snapshot planning_state`
3. `cargo test -p worldwake-systems tell_actions`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - Removed `TellProfile::default()` substitution from `PerAgentBeliefView::tell_profile()`
  - Changed `PlanningSnapshot.actor_tell_profile` to `Option<TellProfile>` and preserved absence directly from the runtime belief view
  - Updated `PlanningState::tell_profile()` to mirror the optional snapshot value without fabricating a profile
  - Changed Tell affordance enumeration to return no Tell affordances when the speaker lacks a `TellProfile`
  - Added direct snapshot/state regressions and a missing-profile Tell affordance regression
- Deviations from original plan:
  - The ticket reassessment found that `planning_snapshot.rs` had no direct Tell-profile regression test yet, so that test was added from scratch rather than updated
  - `tell_actions.rs` already had strong positive affordance coverage, so the work extended that coverage with a missing-profile regression instead of adding a separate broad “valid agents still enumerate” suite
- Verification results:
  - `cargo test -p worldwake-sim per_agent_belief_view`
  - `cargo test -p worldwake-ai planning_snapshot`
  - `cargo test -p worldwake-ai planning_state`
  - `cargo test -p worldwake-systems tell_actions`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
