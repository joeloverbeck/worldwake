# S09INDACTREEVA-006: Complete S09 indefinite-duration removal and finite defend re-evaluation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — remove live indefinite action duration support, make defend finite via `ActorDefendStance`, and prove re-evaluation through focused and golden coverage
**Deps**: archive/tickets/completed/S09INDACTREEVA-001.md

## Problem

The active S09 spec is about removing indefinite action duration from the live authority path, not about `CombatProfile::new()` ergonomics. Current production code still allows `defend` to run with `DurationExpr::Indefinite`, and the runtime, planner, and CLI still carry `Indefinite` branches. That contradicts Principle 8 and preserves the original deadlock failure mode where an agent can remain in defend forever instead of re-entering the decision pipeline.

## Assumption Reassessment (2026-03-20)

1. The previous contents of this ticket were stale. They targeted positional `CombatProfile::new()` cleanup, but the live S09 spec in [S09-indefinite-action-re-evaluation.md](/home/joeloverbeck/projects/worldwake/specs/S09-indefinite-action-re-evaluation.md) is about removing `DurationExpr::Indefinite` and `ActionDuration::Indefinite`, switching defend to a finite profile-driven duration, and proving re-evaluation behavior.
2. `CombatProfile` already has the prerequisite `defend_stance_ticks` field in [combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/combat.rs), and `CombatProfile::new()` already takes the extra argument. The schema prerequisite from ticket 001 is complete, so constructor cleanup is not the current S09 blocker.
3. The current first live production use of indefinite duration is `defend_action_def()` in [combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs), which still sets `duration: DurationExpr::Indefinite`.
4. The authoritative duration layer still exposes indefinite duration in [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) (`DurationExpr::Indefinite` and `resolve_for()`), [action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs) (`ActionDuration::Indefinite`), and [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs) (`reservation_range()` indefinite branch).
5. The belief/planner layer still special-cases indefinite duration in [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) and [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs). This is mixed-layer work, not a systems-only field swap.
6. The CLI still assumes indefinite active durations in [tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/tick.rs) and [world_overview.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/world_overview.rs).
7. Existing test coverage already spans the relevant layers:
   - focused/unit: `worldwake-sim` duration tests, `worldwake-systems` defend tests, `worldwake-ai` search tests
   - golden/E2E: [golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs), including `golden_reduce_danger_defensive_mitigation`
   - real test names were verified with `cargo test -p worldwake-ai -- --list`
8. This ticket changes both authoritative action semantics and AI/planner cost handling, so the S09 spec must be verified across exact layers rather than treated as a single “combat fix”.
9. Ordering is involved, but the contract is action-lifecycle ordering, not strict later-tick separation. The important proof is: defend starts, defend commits after its finite resolved duration, and the agent then re-enters planning. Same-tick follow-on behavior is lawful if traces and state prove the lifecycle correctly.
10. Mismatch corrected: the prior ticket claimed an API-cleanup scope that is not the current architecture need. This ticket now owns the actual remaining S09 implementation slice.

## Architecture Check

1. The spec direction is better than the current architecture. A live `Indefinite` branch lets actions bypass the normal costed lifecycle and can freeze the agent in a stale commitment. Finite, profile-driven defend duration is cleaner because it preserves the same action model as the rest of the engine: explicit duration, explicit completion, and re-entry into planning.
2. `ActorDefendStance` is the right substrate because defend duration is an actor property, not a global constant and not a special scheduler exception. It matches the existing profile-driven `CombatWeapon` pattern and keeps diversity in concrete authoritative state.
3. Removing `Indefinite` entirely is cleaner than keeping it available “just in case”. Principle 26 applies here directly: if indefinite live duration is the wrong model, delete it from the authority path and update all callers now instead of preserving a dormant alias.
4. The old `CombatProfile::new()` positional-constructor concern is real but secondary. It is fixture ergonomics, not an architectural contradiction in the live authority path. It should not displace the S09 invariant fix. If pursued later, it deserves its own ticket.

## Verification Layers

1. `ActorDefendStance` resolves from authoritative `CombatProfile.defend_stance_ticks` and fails cleanly without a combat profile -> focused `worldwake-sim` unit coverage
2. Defend action definition and active action duration are finite and profile-driven -> focused `worldwake-systems` unit/integration coverage
3. Belief-side duration estimation and planner costing use finite defend duration instead of the old indefinite/zero-cost path -> focused `worldwake-ai` search coverage
4. Runtime start/occupancy handling no longer carries indefinite lifecycle branches -> focused `worldwake-sim` runtime coverage
5. Original emergent deadlock is broken end-to-end -> `worldwake-ai` golden coverage using decision trace and action trace where needed
6. CLI presentation remains consistent after live indefinite removal -> targeted `worldwake-cli` tests if touched, plus workspace regression

## What to Change

### 1. Add `DurationExpr::ActorDefendStance` and wire duration resolution

In [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) and [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs):
- add `ActorDefendStance`
- resolve it from `CombatProfile.defend_stance_ticks`
- update focused tests and roundtrip coverage

### 2. Switch defend to finite profile-driven duration

In [combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs):
- change `defend_action_def()` from `DurationExpr::Indefinite` to `DurationExpr::ActorDefendStance`
- update defend-focused assertions so active defend resolves to `ActionDuration::Finite(defend_stance_ticks)`
- strengthen lifecycle coverage so finite defend completion is explicit

### 3. Remove live indefinite duration support

In [action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs), [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs), [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs), [tick_action.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs), [affordance_query.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/affordance_query.rs), [trade_valuation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/trade_valuation.rs), [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs), [tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/tick.rs), and [world_overview.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/world_overview.rs):
- remove `DurationExpr::Indefinite`
- remove `ActionDuration::Indefinite`
- remove fallback logic that manufactured or displayed indefinite duration
- simplify affected matches and tests

### 4. Add end-to-end regression coverage for defend re-evaluation

In [golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs):
- add a golden test that reproduces the original defend deadlock setup with short `defend_stance_ticks`
- prove finite defend commit and subsequent planning re-entry / action resumption using the appropriate trace surfaces
- reassess `golden_reduce_danger_defensive_mitigation` and update only if it relied on indefinite behavior

## Files to Touch

- `crates/worldwake-sim/src/action_semantics.rs`
- `crates/worldwake-sim/src/belief_view.rs`
- `crates/worldwake-sim/src/action_duration.rs`
- `crates/worldwake-sim/src/start_gate.rs`
- `crates/worldwake-sim/src/tick_action.rs`
- `crates/worldwake-sim/src/affordance_query.rs`
- `crates/worldwake-sim/src/trade_valuation.rs`
- `crates/worldwake-systems/src/combat.rs`
- `crates/worldwake-ai/src/search.rs`
- `crates/worldwake-ai/tests/golden_combat.rs`
- `crates/worldwake-cli/src/handlers/tick.rs`
- `crates/worldwake-cli/src/handlers/world_overview.rs`

## Out of Scope

- General `CombatProfile::new()` API ergonomics cleanup
- Refactoring `ActionDuration` from a single-variant enum into a newtype unless it becomes necessary during implementation
- Balance tuning beyond the finite defend-duration contract itself
- Unrelated combat/planner refactors outside the S09 duration model

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-systems`
3. `cargo test -p worldwake-ai`
4. `cargo test -p worldwake-cli`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

### Invariants

1. No live `DurationExpr::Indefinite` variant remains in `crates/`
2. No live `ActionDuration::Indefinite` variant remains in `crates/`
3. Defend duration resolves from `CombatProfile.defend_stance_ticks`
4. Defend commits after a finite duration and the actor re-enters planning
5. Planner duration cost no longer relies on the previous indefinite special-case path
6. The original deadlock scenario is covered by a golden regression

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_semantics.rs` — add/update coverage for `ActorDefendStance` resolution and remove obsolete indefinite assertions; rationale: this is the authoritative duration-expression boundary.
2. `crates/worldwake-sim/src/action_duration.rs` — remove indefinite-only coverage and keep finite lifecycle/roundtrip checks aligned with the new type contract; rationale: the runtime duration type itself must enforce the new invariant.
3. `crates/worldwake-systems/src/combat.rs` — update defend action-definition assertions and strengthen finite-duration lifecycle coverage; rationale: defend is the only production action whose semantics change.
4. `crates/worldwake-ai/src/search.rs` — add or adjust focused planner-cost coverage for finite defend duration; rationale: this proves the planner no longer uses the zero-cost indefinite path.
5. `crates/worldwake-ai/tests/golden_combat.rs` — add a golden defend re-evaluation regression and adjust existing defensive-mitigation expectations only if necessary; rationale: the original bug is an emergent end-to-end failure and needs golden proof.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-systems`
3. `cargo test -p worldwake-ai`
4. `cargo test -p worldwake-cli`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-20
- What changed:
  - removed live `DurationExpr::Indefinite` and `ActionDuration::Indefinite` support from sim, AI, and CLI layers
  - added `DurationExpr::ActorDefendStance` and resolved defend duration from `CombatProfile.defend_stance_ticks`
  - switched the production `defend` action to finite profile-driven duration and strengthened focused lifecycle coverage
  - removed planner and belief-side indefinite special-casing and added focused planner-cost coverage for finite defend
  - added a golden regression proving seeded finite defend commits and the actor re-enters planning afterward
  - updated the living-combat mitigation golden to assert `ReduceDanger` selection through decision traces rather than a brittle derived-pressure threshold
- Deviations from original plan:
  - the archived proof does not require a subsequent non-defend action; current architecture lawfully allows the agent to reselect defend if danger persists, so the regression now asserts the actual invariant: finite defend commits and planning/action flow resumes
  - the living-combat golden setup was explicitly isolated with a seeded attacker action and human-controlled attacker to keep the mitigation branch stable under the new finite-duration model
- Verification results:
  - `cargo test -p worldwake-sim` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test -p worldwake-cli` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
