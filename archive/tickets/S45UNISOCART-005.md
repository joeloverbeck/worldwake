# S45UNISOCART-005: AI elimination-bounty pursuit goals and planner integration

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new `GoalKind`, AI candidate/ranking/planner contract updates
**Deps**: S45UNISOCART-003, S45UNISOCART-004

## Problem

Agents can perceive bounties after `004`, but the AI still has no bounty-pursuit goal family. The original ticket over-claimed a generic bounty plan chain for both elimination and delivery bounties. Reassessment against the live planner showed only elimination bounties fit the current GOAP contract without a broader progress/decomposition expansion: `claim_bounty` in [`artifact_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/artifact_actions.rs) is purely terminal, and delivery progress is not modeled as satisfaction of a higher bounty goal family anywhere in the current planner.

This ticket therefore lands the real, bounded AI slice now:
- add `GoalKind::FulfillBounty { bounty: EntityId }`
- emit and rank pursuit candidates for believed Active elimination bounties
- let the planner root and terminal contract support traveling to the claim place and executing `claim_bounty`
- let combat commitment remain the lawful mid-pursuit barrier for the elimination half of the lifecycle
- invalidate bounty pursuit when the believed artifact stops being Active

Delivery-bounty planner integration becomes an explicit follow-up ticket instead of an unowned promise.

## Assumption Reassessment (2026-04-04)

1. `GoalKind` lives in [`goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) and currently has no bounty variant. `EntityId` is `Copy`, so `FulfillBounty { bounty: EntityId }` fits the existing value-type contract.
2. The live planner contract is not in the stale ticket's `search.rs`/`agent_tick.rs` framing. Exact-goal operator families and root synthesis live across [`goal_dispatch_key.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs), [`goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs), [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), [`search/candidates.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs), and [`exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs).
3. The shared information boundary is already sufficient. `GoalBeliefView::known_entity_beliefs()` in [`belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) exposes `BelievedArtifactState`, so this ticket does not need a new trait method for active bounties.
4. `claim_bounty` already exists authoritatively in [`artifact_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/artifact_actions.rs) and targets a stable `SocialArtifact` identity via `TargetSpec::SpecificEntity(...)`. The action succeeds only when the bounty is still Active, the claimant is at `claim_place`, the target is already satisfied, and proof is valid.
5. The original ticket's generic `Travel(target) -> satisfy target -> Travel(claim_place) -> ClaimBounty` narrative is only partially lawful under the live planner. For `BountyTarget::EliminateEntity`, the existing combat commitment model can carry the hunt/combat side, and the new bounty goal can own the later claim stage. For `BountyTarget::DeliverCommodity`, no remaining ticket owns the broader planner progress/decomposition work needed to treat delivery completion as bounty-goal satisfaction.
6. This ticket is therefore corrected to elimination-bounty AI pursuit only. Delivery-bounty planner integration is a required adjacent consequence and becomes follow-up [`S45UNISOCART-007.md`](/home/joeloverbeck/projects/worldwake/tickets/S45UNISOCART-007.md), not hidden future cleanup.
7. Goal invalidation is not an `agent_tick.rs` special case. Exhaustion and replanning invalidation derive from `GoalDispatchKey` declarations plus [`derive_invalidation_conditions(...)`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs), so `FulfillBounty` needs its own dispatch/invalidation strategy there.
8. Ranking already has an enterprise lane in [`ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs). The live arithmetic should use `enterprise_weight` times a concrete reward-derived signal rather than introducing a bounty-only priority class.
9. CLI display still uses [`format_goal_kind(...)`](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/display.rs), and AI traces have their own formatter in [`decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs). Both are part of the real fallout.
10. `planner_ops.rs` currently treats `"claim_bounty"` as intentionally unclassified. That is stale once `FulfillBounty` becomes planner-visible.
11. The live exact-root synthesis path in [`GroundedGoal::synthesized_root_candidate_targets(...)`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) already supports `SpecificEntity` terminal roots for other exact-bound goals, so `claim_bounty` can join that path without a separate planner API.
12. Mismatch + correction: the ticket no longer claims delivery-bounty AI pursuit in this slice. `006` must be updated to keep Scenario A on the elimination-bounty lifecycle only, and the delivery planner gap must be explicitly ticketed.

## Architecture Check

1. This keeps bounty pursuit on the canonical AI pipeline instead of inventing a dedicated bounty subsystem: candidate generation emits a normal goal, ranking uses normal enterprise motive arithmetic, search uses the normal relevant-op/root-synthesis path, and runtime replanning uses the existing invalidation/exhaustion contract.
2. Narrowing to elimination bounties is cleaner than forcing a fake generic plan chain. It preserves the current spec's hunt-and-claim story where the live planner can actually support it, while ticketing the missing delivery planner substrate explicitly instead of landing decorative goal shells.
3. No backward-compatibility aliasing or shadow goal families are introduced.

## Verification Layers

1. Elimination bounty candidate emission from believed Active bounties -> focused candidate-generation tests and decision-trace candidate presence/absence
2. Ranking uses enterprise-weighted reward motive -> focused ranking tests
3. `claim_bounty` becomes planner-visible and root-synthesizable for exact-bound bounty goals -> focused search/goal-model/planner-op tests
4. FulfillBounty invalidates when the believed artifact is no longer Active -> focused exhaustion/invalidation tests
5. Human-readable goal display stays coherent -> CLI display and AI decision-trace formatter tests
6. Workspace regression surface -> `cargo test --workspace` and CI-matching clippy

## What to Change

### 1. Add `GoalKind::FulfillBounty`

In [`goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs):
- add `FulfillBounty { bounty: EntityId }`
- update `GoalKey::from(...)` so the bounty entity is the canonical bound entity
- update the exhaustive tests accordingly

### 2. Add elimination-bounty candidate emission and ranking

In [`candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs):
- add `emit_bounty_candidates()`
- iterate `known_entity_beliefs(agent)` and filter to believed Active `ArtifactKind::Bounty`
- only emit goals for `BountyTarget::EliminateEntity`
- require a live combat-capability gate consistent with current combat candidate patterns
- anchor the opportunity to the believed bounty entity and carry evidence for the bounty plus relevant places

In [`ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs):
- add enterprise-style motive scoring for `FulfillBounty`
- base the signal on the believed reward quantity/commodity already present on `BelievedArtifactState`

### 3. Wire the planner contract for exact-bound bounty claim goals

Across:
- [`goal_dispatch_key.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs)
- [`goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs)
- [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
- [`planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs)

Make `FulfillBounty` a real goal family:
- add the dispatch key/declaration
- stop treating `claim_bounty` as intentionally unclassified
- let relevant ops include the lawful root/terminal surfaces for elimination-bounty pursuit, including `Travel`, combat commitment where applicable, and `claim_bounty`
- add exact-bound root synthesis for `claim_bounty` on the bounty entity
- add `matches_binding`, `goal_relevant_places`, `is_progress_barrier`, `is_satisfied`, and any bounded `apply_planner_step` support needed for the claim stage

### 4. Add invalidation and display fallout

In [`exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs):
- add a bounty-specific invalidation strategy keyed to believed Active/non-Active state and relevant positional changes

In:
- [`display.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/display.rs)
- [`decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)

add `FulfillBounty` formatting.

### 5. Correct downstream ownership

Before implementation closeout:
- update [`S45UNISOCART-006.md`](/home/joeloverbeck/projects/worldwake/tickets/S45UNISOCART-006.md) so Scenario A is explicitly the elimination-bounty lifecycle
- create [`S45UNISOCART-007.md`](/home/joeloverbeck/projects/worldwake/tickets/S45UNISOCART-007.md) for delivery-bounty planner integration

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-cli/src/display.rs` (modify)
- `tickets/S45UNISOCART-006.md` (modify)
- `tickets/S45UNISOCART-007.md` (new)

## Out of Scope

- Full delivery-bounty planner pursuit and decomposition
- AI-driven bounty posting
- New notice or debt AI beyond the already landed `004` boundary
- Golden/E2E closeout beyond keeping `006` factually aligned

## Acceptance Criteria

### Tests That Must Pass

1. Elimination-bounty candidate emission works and skips non-Active or non-combat-capable cases
2. Ranking gives `FulfillBounty` an enterprise-style reward-driven motive score
3. Search/goal-model tests prove `claim_bounty` is planner-visible for `FulfillBounty`
4. Invalidation tests prove `FulfillBounty` stops surviving once the believed bounty is non-Active
5. CLI/trace formatters render `FulfillBounty`
6. Existing suite: `cargo test --workspace`

### Invariants

1. AI bounty pursuit still plans from beliefs only; it never reads authoritative artifact state directly
2. Delivery-bounty planner integration is not silently implied by this slice; the missing substrate remains explicitly ticketed

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — new goal identity coverage for `FulfillBounty`
2. `crates/worldwake-ai/src/candidate_generation.rs` — candidate emission and omission tests for elimination bounties
3. `crates/worldwake-ai/src/ranking.rs` — motive-score tests for `FulfillBounty`
4. `crates/worldwake-ai/src/goal_model.rs` — exact-bound planner contract tests for bounty goals
5. `crates/worldwake-ai/src/search/tests.rs` — root synthesis / plan-shape tests for `claim_bounty`
6. `crates/worldwake-ai/src/exhaustion.rs` — invalidation-condition coverage
7. `crates/worldwake-cli/src/display.rs` and `crates/worldwake-ai/src/decision_trace.rs` — formatter coverage

### Commands

1. `cargo test -p worldwake-core goal`
2. `cargo test -p worldwake-ai candidate_generation`
3. `cargo test -p worldwake-ai ranking`
4. `cargo test -p worldwake-ai goal_model`
5. `cargo test -p worldwake-ai search`
6. `cargo test -p worldwake-ai exhaustion`
7. `cargo test -p worldwake-cli display`
8. `cargo clippy --workspace --all-targets -- -D warnings`
9. `cargo test --workspace`

## Outcome

Completed on 2026-04-04.

Added `GoalKind::FulfillBounty { bounty: EntityId }` in `crates/worldwake-core/src/goal.rs` and wired the elimination-bounty pursuit slice through the canonical AI pipeline. `crates/worldwake-ai/src/candidate_generation.rs` now emits candidates for believed Active `EliminateEntity` bounties, `crates/worldwake-ai/src/ranking.rs` scores them through the existing enterprise lane, and `crates/worldwake-ai/src/goal_dispatch_key.rs`, `crates/worldwake-ai/src/goal_dispatch_decl.rs`, `crates/worldwake-ai/src/goal_model.rs`, and `crates/worldwake-ai/src/planner_ops.rs` make `claim_bounty` a planner-visible, exact-bound terminal for `FulfillBounty`.

Bounded AI fallout also landed in `crates/worldwake-ai/src/exhaustion.rs`, `crates/worldwake-ai/src/feasibility.rs`, `crates/worldwake-ai/src/goal_policy.rs`, `crates/worldwake-ai/src/failure_handling.rs`, and `crates/worldwake-ai/src/agent_tick/observation.rs` so the new goal family participates coherently in invalidation, feasibility, suppression, failure handling, and read-phase goal handling. Human-readable goal rendering landed in `crates/worldwake-cli/src/display.rs`, and focused planner/candidate coverage landed in `crates/worldwake-ai/src/search/tests.rs`.

Deviation from the original plan: this ticket was corrected to elimination-bounty pursuit only. Delivery-bounty planner integration could not be landed honestly within the current planner contract, so `tickets/S45UNISOCART-007.md` was created as the explicit follow-up and `tickets/S45UNISOCART-006.md` was updated so its golden lifecycle contract remains on the elimination-bounty path.

Verification completed:
- `cargo test -p worldwake-core goal -- --nocapture`
- `cargo test -p worldwake-ai candidate_generation -- --nocapture`
- `cargo test -p worldwake-ai ranking -- --nocapture`
- `cargo test -p worldwake-ai goal_model -- --nocapture`
- `cargo test -p worldwake-ai search -- --nocapture`
- `cargo test -p worldwake-ai exhaustion -- --nocapture`
- `cargo test -p worldwake-cli display -- --nocapture`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
