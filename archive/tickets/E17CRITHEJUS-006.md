# E17CRITHEJUS-006: Implement steal action in worldwake-systems

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action definition + handler in systems crate, plus planner/search registration alignment
**Deps**: E17CRITHEJUS-001 (needs `TheftDispositionProfile`), E17CRITHEJUS-004 (needs `GoalKind::StealItem`)

## Problem

No action allows taking items owned by others. The only acquisition paths are lawful `pick_up` (requires `can_exercise_control`) and `trade`. E17 needs a `steal` action that transfers possession without transferring ownership, with `VisibilitySpec::Hidden` and `EventTag::Crime`.

## Assumption Reassessment (2026-03-26)

Shared abstraction boundary under audit: `GoalKind::StealItem` in `worldwake-core`, transport action registration/handlers in `worldwake-systems`, and `PlannerOpKind::MoveCargo` semantics/search surface in `worldwake-ai`.

1. `transport_actions.rs` in `crates/worldwake-systems/src/transport_actions.rs` contains the live `pick_up` / `put_down` authoritative cargo-transfer path. Steal should reuse the same direct-possession mutation shape (`move_entity_to_direct_possession` / `set_possessor`) while inverting the ownership-control gate.
2. The ticket’s original “new E17 type” assumptions are stale. `TheftDispositionProfile` already exists in [crates/worldwake-core/src/crime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/crime.rs), `GoalKind::StealItem` already exists in [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs), `ViolationKind::SuspectedTheft` already exists in [crates/worldwake-core/src/violation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/violation.rs), and `RecordKind::CrimeRegister` already exists in [crates/worldwake-core/src/institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs). This ticket is now specifically about the missing authoritative steal action and its immediate planner/search landing path.
3. `register_all_actions()` in [crates/worldwake-systems/src/action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs) is still the authoritative registration surface and currently has no steal registration.
4. `ActionDomain::Transport` exists in [crates/worldwake-sim/src/action_domain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_domain.rs), and current planner classification in [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs) maps only `"pick_up"` / `"put_down"` to `PlannerOpKind::MoveCargo`. A systems-only steal action would leave `GoalKind::StealItem` architecturally half-landed.
5. The current AI test surface explicitly encodes the pre-landing gap: `deferred_crime_and_justice_goals_have_no_search_surface_before_actions_land` in [crates/worldwake-ai/src/search/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/tests.rs) asserts that `StealItem` exposes no relevant action defs or candidates. That assumption becomes incorrect once steal lands and must be updated as part of this ticket.
6. `VisibilitySpec::Hidden` exists in [crates/worldwake-core/src/visibility.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/visibility.rs), and `EventTag::Crime` exists in [crates/worldwake-core/src/event_tag.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/event_tag.rs).
7. `TheftDispositionProfile.steal_duration_ticks` in [crates/worldwake-core/src/crime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/crime.rs) is the correct profile-driven duration source. No new numeric duration constant is warranted.
8. Current affordance/precondition enums do not provide a belief-side “owned by other and actor cannot control” inverse-control predicate. Existing patterns already rely on authoritative start validation for richer domain checks, so this ticket should keep that validation in the handler rather than expanding the global precondition language unless multiple action families need the same inverse rule.
9. No competing lawful transport path exists for the same fact. Canonical post-change path: lawful acquisition remains `pick_up`; unlawful possession transfer becomes `steal`; no alias or fallback path is added.
10. Required consequence, not adjacent cleanup: planner/search must recognize the new transport action for `StealItem`. Separate theft candidate generation, ranking, accusation, punishment, and golden coverage remain out of scope.

## Architecture Check

1. Cleanest architecture: model steal as a first-class transport action, not a special case inside `pick_up`. Lawful and unlawful acquisition remain separate action definitions with separate visibility/tags/preconditions, while sharing the same cargo-transfer primitive.
2. Planner/search should classify steal as the same cargo-moving operator family (`PlannerOpKind::MoveCargo`) and reuse the same hypothetical possession-transfer transition shape as `pick_up`. That is cleaner than introducing a one-off theft-only planner op for identical world-state movement.
3. No backwards-compatibility aliasing introduced. `pick_up` remains the lawful path; `steal` is the unlawful path; callers update to the real action instead of using compatibility shims.
4. Ideal future architecture note: if more inverse-control actions appear, the affordance/precondition language may deserve a dedicated negative-control predicate. This ticket should not broaden the shared precondition algebra for a single action family.

## Verification Layers

1. Steal transfers possession without transferring ownership -> authoritative world state check in focused systems tests
2. Conservation survives steal commit/abort -> `verify_live_lot_conservation()` in focused systems tests
3. Crime event metadata is correct -> event-log assertion on `EventTag::Crime`, `EventTag::Transfer`, and `VisibilitySpec::Hidden`
4. Multi-tick duration honors `TheftDispositionProfile.steal_duration_ticks` -> action lifecycle / tick progression test
5. Authoritative rejection surface remains correct when theft is unlawful to start -> focused start-failure assertions (`can_exercise_control == true`, direct possession by another agent, insufficient carry capacity, missing theft profile)
6. `GoalKind::StealItem` search surface lands coherently -> focused AI search/planner tests proving `steal` becomes the relevant `MoveCargo` action for the bound target

## What to Change

### 1. Add steal as a dedicated transport action

- Prefer extending [crates/worldwake-systems/src/transport_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/transport_actions.rs) instead of creating a new parallel module. `pick_up`, `put_down`, and `steal` are one transport family sharing the same cargo-movement primitive.
- Register a third action definition: name `"steal"`, domain `ActionDomain::Transport`, `TargetSpec::EntityAtActorPlace { kind: EntityKind::ItemLot }`, `VisibilitySpec::Hidden`, tags `[EventTag::Crime, EventTag::Transfer]`.
- Duration: profile-driven from `TheftDispositionProfile.steal_duration_ticks`.
- Interruptibility: `Interruptibility::FreelyInterruptible`.

### 2. Start handler

Validate authoritatively:
- Actor and target at same place
- Target is `EntityKind::ItemLot`
- Target has an owner other than actor
- `can_exercise_control(actor, target) == false`
- Actor has `TheftDispositionProfile`
- Target not currently possessed by another agent
- Target not reserved
- Actor has remaining load capacity

Return `StartFailed` on any precondition failure.

### 3. Commit handler

- `txn.set_possessor(target_item, actor)` — transfer possession
- Reuse the existing transport direct-possession helper so travel/placement behavior stays identical to lawful cargo handling
- Ownership relation unchanged
- Emit event with `EventTag::Crime`, `VisibilitySpec::Hidden`, `WitnessData` with actor as sole direct participant

### 4. Abort handler

No-op. Interrupted theft produces no transfer.

### 5. Register in action_registry.rs

If steal remains inside `transport_actions.rs`, extend `register_transport_actions()` and update the full-registry expectation list. Do not add a parallel registration path for the same transport family.

### 6. Align planner/search transport semantics

- In [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs), classify `"steal"` under `PlannerOpKind::MoveCargo`.
- Reuse the same hypothetical possession-transfer transition shape as `pick_up` for `"steal"` so `GoalKind::StealItem` can make search progress without a bespoke planner operator.
- Update the stale deferred search test in [crates/worldwake-ai/src/search/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/tests.rs) to assert the landed behavior for theft while keeping accusation/punishment deferred.

### 7. Export from lib.rs

No new top-level module export is needed if steal stays in `transport_actions.rs`.

## Files to Touch

- `crates/worldwake-systems/src/transport_actions.rs`
- `crates/worldwake-systems/src/action_registry.rs`
- `crates/worldwake-ai/src/planner_ops.rs`
- `crates/worldwake-ai/src/search/tests.rs`

## Out of Scope

- Accuse/Fine/Exile actions (E17CRITHEJUS-008/009)
- Investigate commit extension for SuspectedTheft (E17CRITHEJUS-007)
- AI candidate generation for theft (E17CRITHEJUS-010)
- Perception system changes (none needed per spec)
- Theft motive/ranking/candidate generation beyond enabling the already-existing `StealItem` goal family to bind to the new action
- Refactoring unrelated transport behavior outside the minimal steal/planner landing work
- Golden tests (E17CRITHEJUS-012)

## Acceptance Criteria

### Tests That Must Pass

1. Steal transfers possession: after commit, `possessor_of(item) == actor`
2. Steal does NOT transfer ownership: `owner_of(item)` unchanged
3. Conservation maintained: `verify_live_lot_conservation()` passes before and after steal
4. Event emitted with `EventTag::Crime` tag
5. Event emitted with `VisibilitySpec::Hidden`
6. Abort produces no possession change
7. Start-fail when `can_exercise_control(actor, item) == true` (item is lawfully accessible)
8. Start-fail when item possessed by another agent (robbery is out of scope)
9. Start-fail when actor lacks load capacity
10. Action duration matches `TheftDispositionProfile.steal_duration_ticks`
11. `GoalKind::StealItem` now surfaces `steal` as a relevant search candidate for the exact bound target
12. Accuse / Punish deferred search surface remains absent

### Invariants

1. `pick_up` behavior remains unchanged
2. Conservation invariant holds for all steal outcomes (commit and abort)
3. Ownership relation never mutated by steal
4. `VisibilitySpec::Hidden` on all steal events (no crime event is public)
5. Only agents with `TheftDispositionProfile` can have a steal action started
6. No new transport alias path: lawful cargo stays `pick_up`; theft stays `steal`

## Tests

### New/Modified Tests and Rationale

1. `crates/worldwake-systems/src/transport_actions.rs` — add focused steal tests for commit, abort, duration, and start-failure because the authoritative transport family lives here and shares helper logic with `pick_up`.
2. `crates/worldwake-ai/src/search/tests.rs` — replace the stale theft deferral assertion with a landed `StealItem` search-surface assertion while keeping `Accuse` / `PunishAccused` deferred, proving the scope boundary is still respected.

### Commands

1. `cargo test -p worldwake-systems transport_actions::tests::register_transport_actions_creates_pick_up_put_down_and_steal_defs -- --exact`
2. `cargo test -p worldwake-systems transport_actions::tests::steal_happy_path_transfers_possession_without_transferring_ownership -- --exact`
3. `cargo test -p worldwake-systems transport_actions::tests::steal_requires_theft_profile -- --exact`
4. `cargo test -p worldwake-ai search::tests::steal_goal_surfaces_search_candidates_after_action_lands -- --exact`
5. `cargo test -p worldwake-ai search::tests::accuse_and_punish_goals_remain_deferred_without_actions -- --exact`
6. `cargo test -p worldwake-systems`
7. `cargo test -p worldwake-ai`
8. `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
9. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
10. `cargo build --workspace`

## Outcome

- Completed: 2026-03-26
- Actually changed:
  - Added `steal` as a first-class transport action in `crates/worldwake-systems/src/transport_actions.rs` with hidden crime tagging, profile-driven theft duration, authoritative ownership/control validation, and shared direct-possession movement.
  - Landed the missing shared duration surface via `DurationExpr::ActorTheftDisposition` in `worldwake-sim`, with belief-view support for theft profile duration estimation.
  - Aligned the planner/search layer so `StealItem` now uses `MoveCargo` semantics and `steal` is classified as the relevant transport action while accusation/punishment remain deferred.
  - Updated the stale ticket assumptions and the stale AI search deferral coverage that assumed no theft action existed yet.
- Deviations from original plan:
  - Kept steal inside `transport_actions.rs` instead of creating a new `steal_actions.rs` module because lawful and unlawful cargo transfer belong to one transport family.
  - Did not broaden the shared precondition algebra with a new inverse-control predicate; kept the richer theft-only checks in authoritative handler validation.
  - Did not land accusation/punishment actions or theft candidate generation; those remain separate tickets as planned.
- Verification results:
  - `cargo test -p worldwake-systems` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy -p worldwake-systems --all-targets -- -D warnings` ✅
  - `cargo clippy -p worldwake-ai --all-targets -- -D warnings` ✅
  - `cargo build --workspace` ✅
