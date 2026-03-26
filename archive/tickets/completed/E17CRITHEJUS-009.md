# E17CRITHEJUS-009: Complete punishment actions on a case-bound crime contract

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — core institutional claim contract, sim action payloads, systems punishment handlers, planner/operator wiring
**Deps**: `specs/E17-crime-theft-justice.md`, E17CRITHEJUS-003, E17CRITHEJUS-008, E17CRITHEJUS-011

## Problem

`accuse` now exists, but punishment does not. More importantly, the current ticket scope is based on assumptions that no longer match the code or a clean long-term architecture:

1. The live code already has `crates/worldwake-systems/src/justice_actions.rs` with `register_accuse_action()`.
2. `PunishAccused` is present in `worldwake-core` / `worldwake-ai`, but planner support is explicitly deferred and no executable punishment operators exist.
3. The current target-only Fine/Exile design is ambiguous when an accused has multiple active accusations.
4. Fine cannot be derived robustly from the current institutional record contract because `InstitutionalClaim::Accusation` only stores `violation_id`, and `ViolationId` comes from per-agent `ViolationMemory`, not shared institutional state.

Implementing the original ticket literally would add actions on top of an under-specified crime-case contract and would fossilize that ambiguity into both systems and AI.

## Assumption Reassessment (2026-03-26)

1. `justice_actions.rs` already exists and already implements `register_accuse_action()` with focused tests in [crates/worldwake-systems/src/justice_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs). The original ticket assumption that this file/module would be created by E17CRITHEJUS-008 is stale.
2. Crime record infrastructure already exists in [crates/worldwake-core/src/institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs): `RecordKind::CrimeRegister`, `InstitutionalClaim::Accusation`, `InstitutionalClaim::Verdict`, and `RecordData::supersede_entry()`. The ticket’s sample `Verdict` shape is stale: the live type includes `violation_id` and uses record-entry supersession through `InstitutionalRecordEntry.supersedes`, not a `supersedes_accusation` field on the claim.
3. The mixed-layer abstraction boundary under audit is the crime-case identity contract spanning [crates/worldwake-core/src/institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs), [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs), [crates/worldwake-sim/src/action_payload.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs), [crates/worldwake-systems/src/justice_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs), and [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs).
4. The live goal under test is `GoalKind::PunishAccused`. The current operator surface does not exist yet: [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) intentionally routes it to `DEFERRED_CRIME_JUSTICE_OPS`, and [crates/worldwake-ai/src/search/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/tests.rs) asserts that punishment remains deferred until verdict actions exist.
5. The first authoritative failure boundary for the original ticket shape is not in the action handler body; it is the missing case identity. A Fine/Exile action with only `TargetSpec::SpecificEntity(accused)` cannot uniquely resolve which accusation to supersede when one accused has multiple active accusations for distinct `violation_id` values. `register_accuse_action()` only prevents duplicates for the same `(accused, violation_id)` pair, not for the same `accused` globally.
6. `can_exercise_control()` already includes faction and office delegation in [crates/worldwake-core/src/world/ownership.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/ownership.rs). The original ticket is correct that this is the current institutional control surface.
7. `member_of` and `hostile_to` relations already exist and are mutated through `WorldTxn::remove_member()` / `WorldTxn::add_hostility()` in [crates/worldwake-core/src/world_txn.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs). Exile can be authoritative with existing relation machinery.
8. The original “transfer fine to treasury entity” assumption is not backed by a live dedicated treasury abstraction. What the code does support cleanly today is office-owned institutional property via `can_exercise_control()` delegation. The canonical no-shim destination for a fine in the current architecture is the jurisdictional office entity itself, not an invented alias treasury entity.
9. The original fine arithmetic assumption is not currently satisfiable from authoritative shared state. `InstitutionalClaim::Accusation` stores only `violation_id`; [crates/worldwake-core/src/violation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/violation.rs) makes clear that `ViolationId` is allocated inside per-agent `ViolationMemory`. That means Fine cannot robustly derive “stolen quantity” later from a shared institutional source without first extending the crime-case record contract.
10. Existing focused verification already covers the adjacent delivered layers: `cargo test -p worldwake-systems -- --list` shows accuse focused tests in `justice_actions::tests::*`, investigate theft tests in `investigate_actions::tests::*`, and the full current `worldwake-systems` suite passes listing resolution. There is currently no focused systems test for `fine` or `exile` because those actions do not exist.
11. Adjacent contradictions exposed by reassessment:
    - Required consequence of this ticket: punishment must become case-bound, not accused-bound.
    - Required consequence of this ticket: crime records must carry enough authoritative offense detail to support fine computation later.
    - Future cleanup, not required for this ticket: if the project eventually wants explicit treasuries rather than office-owned institutional assets, that should be a separate ticket introducing a first-class treasury/world-state contract instead of an alias.
12. Mismatch + correction: this is not a “two new handlers in the systems crate” ticket anymore. The minimal clean scope is: harden the case contract first, then add Fine/Exile handlers, then expose punishment operators to planner semantics. Full punishment candidate generation remains in E17CRITHEJUS-011.

## Architecture Check

1. The cleaner architecture is case-bound punishment, not target-only punishment. A punishment action should bind to a specific active accusation case, because verdict supersession is defined over record entries and because one accused can lawfully have multiple active accusations.
2. The cleaner authoritative source for Fine arithmetic is the crime record itself, not a later lookup into per-agent `ViolationMemory`. The institutional record must carry the offense facts needed for punishment, otherwise Fine depends on subjective/local state that other actors cannot authoritatively inspect.
3. Within the current architecture, routing confiscated goods to the office entity is cleaner than inventing a treasury alias. The office is already a first-class institutional world object with jurisdiction and delegated control semantics.
4. No backwards-compatibility aliasing or shim paths should be introduced. If `PunishAccused` needs additional identity or if `Accusation` needs richer offense detail, update the canonical types directly and fix affected callers/tests.

## Verification Layers

1. Punishment binds to exactly one active accusation case -> focused unit/runtime coverage over case lookup in `justice_actions` plus authoritative record state
2. Verdict supersedes the intended accusation entry and leaves append-only history intact -> authoritative `RecordData` entry inspection
3. Fine preserves conservation while moving goods to institutional custody -> authoritative world state plus `verify_live_lot_conservation()`
4. Exile removes faction membership and adds hostility in the intended direction -> authoritative relation state
5. Planner/operator exposure for punishment is no longer deferred once actions exist -> focused `worldwake-ai` goal-model/search tests
6. Full punishment candidate generation remains out of scope here because that proof belongs to E17CRITHEJUS-011

## What to Change

### 1. Harden the crime-case contract

Update the canonical crime-case identity so punishment can resolve a single accusation deterministically and Fine can compute from authoritative shared record state.

Required scope correction:

- Extend the punishment identity beyond just `accused`. The likely shape is `violation_id` on `GoalKind::PunishAccused` plus a dedicated punishment payload in `worldwake-sim`, but the exact field set must be chosen together with the crime-record extension.
- Extend `InstitutionalClaim::Accusation` so it records the offense details Fine needs later from institutional state rather than from per-agent `ViolationMemory`.
- Keep `InstitutionalClaim::Verdict` case-bound to the same accusation/violation identity and continue using record-entry supersession as the append-only resolution path.

### 2. Implement Fine and Exile on top of that contract

In [crates/worldwake-systems/src/justice_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs):

- Add `register_fine_action()` and `register_exile_action()`.
- Require institutional authority at the jurisdiction place.
- Require a uniquely identified unresolved accusation case.
- For Fine: transfer the computed commodity amount from accused-controlled goods into office-controlled institutional custody, preserving conservation.
- For Exile: remove the accused from the relevant faction and add faction-to-accused hostility.
- Supersede the resolved accusation entry with the appropriate `Verdict`.

### 3. Wire punishment into the non-deferred planner/operator layer

Once actions exist, remove the intentional punishment deferral in `worldwake-ai`:

- expose planner op kinds for Fine / Exile
- map `PunishAccused` to those operators
- update focused goal-model/search tests that currently assert punishment remains deferred

This does not include new punishment goal generation from consulted crime registers; that remains E17CRITHEJUS-011.

## Files to Touch

- [crates/worldwake-core/src/institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs) (modify)
- [crates/worldwake-core/src/goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) (modify)
- [crates/worldwake-sim/src/action_payload.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs) (modify)
- [crates/worldwake-systems/src/justice_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs) (modify)
- [crates/worldwake-systems/src/action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs) (modify)
- [crates/worldwake-systems/src/lib.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs) (modify)
- [crates/worldwake-ai/src/planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs) (modify)
- [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) (modify)
- [crates/worldwake-ai/src/search/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/tests.rs) (modify)

## Out of Scope

- New punishment candidate generation from consulted crime registers (`E17CRITHEJUS-011`)
- Golden justice-chain coverage (`E17CRITHEJUS-013`)
- Introducing a new standalone treasury entity abstraction
- Reworking faction roster record synchronization beyond what punishment itself requires

## Acceptance Criteria

### Tests That Must Pass

1. Fine resolves exactly one active accusation case and supersedes that exact entry
2. Fine transfers commodity into office-controlled institutional custody without violating conservation
3. Fine start-fails when the accused lacks sufficient commodity
4. Exile resolves exactly one active accusation case and supersedes that exact entry
5. Exile removes the accused from the punished faction
6. Exile adds hostility from the faction toward the exiled agent
7. Both start-fail when actor lacks institutional authority
8. Planner-focused tests no longer treat `PunishAccused` as deferred once punishment actions are registered
9. Existing focused suites remain green

### Invariants

1. Punishment is case-bound, not merely accused-bound
2. Verdict resolution remains append-only via record-entry supersession
3. Fine arithmetic comes from authoritative shared crime-case state, not a per-agent memory lookup
4. No backwards-compatibility aliases are introduced for treasury or punishment identity

## Test Plan

### New/Modified Tests

1. [crates/worldwake-systems/src/justice_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs) — add focused Fine/Exile coverage for exact case resolution, conservation, relation mutation, and start-failure paths
2. [crates/worldwake-ai/src/search/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/tests.rs) — replace the current “punishment remains deferred” assertion with concrete operator exposure checks once actions exist
3. [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) — update focused goal/operator binding tests for the revised punishment identity contract

### Commands

1. `cargo test -p worldwake-systems justice_actions::tests::`
2. `cargo test -p worldwake-ai search::tests::crime_goal_relevant_action_defs_follow_live_registry`
3. `cargo test -p worldwake-ai goal_model`
4. `cargo test -p worldwake-systems`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace`
7. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-26
- What actually changed:
  - Added a concrete `TheftFacts` value type and threaded it through violation memory, social observations, and institutional accusations so punishment reads authoritative shared offense facts instead of reconstructing them from subjective memory.
  - Hardened punishment identity to the actual active accusation case via `GoalKind::PunishAccused { office, accusation_entry, punishment, accused }` and `PunishActionPayload { office, accusation_entry, punishment }`.
  - Implemented `fine` and `exile` in `justice_actions.rs`, registered them in the live action registry, and made Fine transfer confiscated goods into office-controlled institutional custody while Exile removes membership and adds faction hostility.
  - Removed planner deferral for punishment by adding `PlannerOpKind::Fine` / `PlannerOpKind::Exile`, wiring payload synthesis, root candidate synthesis, and progress-barrier semantics for `PunishAccused`.
  - Updated downstream tests and stale contract assumptions across AI/sim/core to the new accusation shape.
- Deviations from original plan:
  - The final case identity is not just `violation_id`; it is the institutional accusation entry itself. That is stronger and matches the append-only record architecture better.
  - Fine does not route to a new treasury abstraction. It routes to the office entity because that is the current first-class institutional control surface.
  - A small unrelated `clippy::large_enum_variant` failure in `worldwake-core/src/delta.rs` had to be handled with a targeted allow so the repo’s requested workspace lint pass could complete.
- Verification results:
  - `cargo test -p worldwake-systems justice_actions::tests::`
  - `cargo test -p worldwake-ai search::tests::crime_goal_relevant_action_defs_follow_live_registry -- --nocapture`
  - `cargo test -p worldwake-ai goal_model -- --nocapture`
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
