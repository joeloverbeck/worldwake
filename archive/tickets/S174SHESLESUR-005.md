# S174SHESLESUR-005: Sleep goal schema — FeasibilityStrategy::CandidateBacked + two-path sleep_rest_opportunities

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `FeasibilityStrategy::CandidateBacked` variant on existing AI-crate enum; rewritten `DECL_SLEEP` goal schema; existing `emit_sleep_goal` replaced with two-path `sleep_rest_opportunities`
**Deps**: `archive/tickets/S174SHESLESUR-001.md` (RestCapacity/RestOccupancy types), `archive/tickets/S174SHESLESUR-002.md` (rough_sleep_recovery_floor — read by candidate marker), `archive/tickets/S174SHESLESUR-003.md` (belief-view accessors), `archive/tickets/S174SHESLESUR-004.md` (ActionState::Sleep mode carrier + RestOccupancy lifecycle)

## Problem

Before this ticket, the Sleep goal schema used `FeasibilityStrategy::AlwaysLikely` (`crates/worldwake-ai/src/goal_schema.rs:345`) and emitted a single sleep candidate per tick via `emit_sleep_goal` (`candidate_generation.rs:4488`). That collapsed "sleep at a known rest site" and "sleep rough" into one undifferentiated candidate, so the planner could not rank shelter quality against rough-sleep fallback. S174 D4 required splitting Sleep into a two-path enumerator: `KnownRestSite` (belief-backed, higher recovery, possibly contested) and `RoughSleep` (always-available fallback, capped recovery, more interruptible).

The two-path schema required a `FeasibilityStrategy::CandidateBacked` variant (the goal is feasible iff at least one lawful candidate exists). Before this ticket, the enum had 10 variants but none with that semantic.

## Assumption Reassessment (2026-05-26)

1. Verified current code: `FeasibilityStrategy` enum at `crates/worldwake-ai/src/goal_schema.rs:38-49` has variants `OwnedCommodityCheck, EvidencePlaceLocal, AlwaysLikely, CommodityPresenceCheck, ColocationOrDead, NoOpinion, SellCheck, CargoDestinationCheck, CorpseBurialCheck, PlaceMatch` — no `CandidateBacked`. `DECL_SLEEP` at lines 340-351 is a `GoalSchema` static (not `GoalDecl`) with `feasibility_strategy: FeasibilityStrategy::AlwaysLikely` on line 345 and `relevant_ops: SLEEP_OPS` where `SLEEP_OPS = &[PlannerOpKind::Sleep]` at line 99. Existing emitter `emit_sleep_goal` at `candidate_generation.rs:4488-4524` is called from line 1282 (`emit_self_consume_candidates(...)` dispatch block). `PlannerOpKind::QueueForFacilityUse` exists at `planner_ops.rs:23` and is reachable. Live planner contract correction: `SLEEP_OPS` currently also feeds `DECL_SLEEP.progress_barrier_ops`, while `goal_model.rs::GoalKindPlannerExt::is_progress_barrier` and its test `queue_for_facility_use_is_progress_barrier_for_exclusive_goal_families` explicitly exclude `GoalKind::Sleep` from queue progress-barrier treatment. This ticket therefore splits `SLEEP_RELEVANT_OPS = [Sleep, QueueForFacilityUse]` from `SLEEP_PROGRESS_BARRIER_OPS = [Sleep]` instead of reusing one expanded constant for both fields.
2. Spec assumption verified against S174 D4 (rewritten during reassessment to introduce `CandidateBacked` as a prerequisite paragraph). The rewrite confirmed `GoalSchema` (not the spec's earlier `GoalDecl` framing) is the actual type and `emit_sleep_goal` is the function being replaced.
3. Shared abstraction boundary under audit: goal schema declaration (`DECL_SLEEP`) + candidate emission (`sleep_rest_opportunities` replacing `emit_sleep_goal`) + feasibility strategy enum extension (`CandidateBacked`). All three changes land together because (a) `CandidateBacked` is consumed by `DECL_SLEEP`'s new declaration, (b) the new emitter requires `CandidateBacked` semantics to function (otherwise an empty candidate set would still mark the goal feasible), (c) splitting them leaves the workspace either broken (DECL_SLEEP referencing a nonexistent variant) or dead-code (CandidateBacked variant unused).
4. Existing inline tests on the affected functions: `candidate_generation.rs` inline tests at `fatigue_and_bladder_emit_sleep_and_relieve:13002`, `sleep_candidate_emission_at_current_place_only:13182`, `action_specific_place_blocker_with_support_target_suppresses_matching_sleep_candidate:11805`. Implementation updated their expectations to match the two-path emission shape by distinguishing KnownRestSite anchors from targetless RoughSleep anchors.
5. Live `GoalKind` under test: `GoalKind::Sleep` (the existing variant, unchanged). The exact current operator surface is `SLEEP_OPS` containing only `PlannerOpKind::Sleep`; this ticket extends the relevant-operator surface to `[PlannerOpKind::Sleep, PlannerOpKind::QueueForFacilityUse]` to enable rest-site queueing via the existing S44 substrate, while preserving the existing Sleep progress-barrier surface as `[PlannerOpKind::Sleep]`.
6. `FeasibilityStrategy` derive analysis: the enum's current derives need checking before adding `CandidateBacked`. The variant has no payload (unit), so derive compatibility is trivial (it inherits all derives from the enum). Verify at ticket-implementation time.
7. Heuristic removal: `FeasibilityStrategy::AlwaysLikely` is no longer used by `DECL_SLEEP` after this ticket. Search for other consumers of `AlwaysLikely`; if no other goal uses it, the variant could potentially be removed per FND-28. However, removal is out of scope for this ticket — verify first whether any other goal schema uses `AlwaysLikely` and, if not, propose removal as a follow-up cleanup ticket.
8. Reassessment correction (2026-05-26): archived `S174SHESLESUR-004` landed the rough-vs-known discriminator as `ActionState::Sleep { rough, place }`, but `start_sleep_episode` derives the discriminator from the action target: targetless sleep is RoughSleep; targeted sleep at the current rest-capable place is KnownRestSite. There is no separate action payload or `GoalOffer` flag to mark rough sleep at a rest-capable place. The current ticket therefore emits RoughSleep as `OpportunityAnchor::None` with current-place evidence (no action target), and emits KnownRestSite as `OpportunityAnchor::Place(rest_site)`. This preserves S174's same-place rough-sleep fallback without inventing a parallel mode carrier.

## Architecture Check

1. The two-path emitter (KnownRestSite + RoughSleep) preserves flat GOAP — no HTN method is registered. Per S174's Planner-formalism analysis, the two-path split lives in the goal schema and candidate enumerator, not in method decomposition. HTN would over-formalize a two-candidate branch.
2. `FeasibilityStrategy::CandidateBacked` is a generally reusable strategy ("feasibility = the goal entered ranking through a candidate-backed emitter rather than a separate pre-flight world check"). No other current goal needs it, but introducing it cleanly handles any future goal whose feasibility is "anything to do" rather than belief-checked or place-checked. This matches FND-28 — a new strategy variant rather than a special-case branch inside `DECL_SLEEP`'s evaluator.
3. The RoughSleep marker (carried via `ActionState::Sleep { rough: true, ... }`, per `archive/tickets/S174SHESLESUR-004.md`) is selected by the handler from the action's target shape: no target means RoughSleep, while a targeted rest-capable current place means KnownRestSite. This keeps the mode decision in the action lifecycle state rather than a planner-side side channel. This matches FND-26 — systems interact through action state and targets, not direct calls.

## Verified Layers

1. `FeasibilityStrategy::CandidateBacked` was added and covered by exhaustive strategy tests in `goal_schema.rs` plus routing coverage in `feasibility.rs`.
2. `DECL_SLEEP` now uses `FeasibilityStrategy::CandidateBacked`, has relevant ops `[Sleep, QueueForFacilityUse]`, and keeps progress barriers at `[Sleep]`.
3. `sleep_rest_opportunities` emits KnownRestSite candidates only for reachable rest-capable places with known occupancy below capacity.
4. `sleep_rest_opportunities` emits exactly one targetless RoughSleep fallback with current-place evidence when the actor has a current place.
5. Full rest sites are filtered out while the RoughSleep fallback remains available.
6. The rough vs known distinction is carried through target shape: `OpportunityAnchor::None` for RoughSleep and `OpportunityAnchor::Place(rest_site)` for KnownRestSite, matching archived ticket 004's action-start discriminator.
7. Existing sleep candidate and golden sleep-ranking tests pass after fixture truthing for rest capacity and belief-backed occupancy.

## Landed Changes

1. Added `FeasibilityStrategy::CandidateBacked` in `crates/worldwake-ai/src/goal_schema.rs` and routed Sleep through it in `crates/worldwake-ai/src/feasibility.rs`.
2. Split Sleep schema operator constants into `SLEEP_RELEVANT_OPS` and `SLEEP_PROGRESS_BARRIER_OPS`, so `QueueForFacilityUse` is relevant for rest-site planning without becoming a Sleep progress barrier.
3. Replaced `emit_sleep_goal` with `sleep_rest_opportunities` and helper functions in `crates/worldwake-ai/src/candidate_generation.rs`.
4. Updated candidate-generation test stubs with belief-view rest-site capacity and occupant-count accessors.
5. Updated sleep-related focused tests to assert KnownRestSite and targetless RoughSleep candidates separately.
6. Updated golden sleep fixtures to install `RestCapacity` and belief-backed empty contention state where a remote rest-site candidate is expected.
7. Truth-synced `specs/S174-shelter-sleep-surfaces-safe-rest.md` so D4 describes targetless RoughSleep and the Sleep relevant/progress-barrier split.

## Landed Files

- `crates/worldwake-ai/src/goal_schema.rs`
- `crates/worldwake-ai/src/feasibility.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/tests/scenarios/place_dirtiness.rs`
- `crates/worldwake-ai/tests/scenarios/sleep_episode.rs`
- `specs/S174-shelter-sleep-surfaces-safe-rest.md`

## Out of Scope

- No `RestCapacity` / `RestOccupancy` component definitions (ticket 001)
- No belief-view accessor implementations (archived `archive/tickets/S174SHESLESUR-003.md`)
- No `RestOccupancy` writes at sleep action start (`archive/tickets/S174SHESLESUR-004.md`)
- No `rough_sleep_recovery_floor` application (`archive/tickets/S174SHESLESUR-004.md` reads the floor at sleep-tick)
- No `FailedRestOpportunity` records (ticket 006)
- No `ActionTraceDetail::SleepInterrupted` population (ticket 006)
- No removal of `FeasibilityStrategy::AlwaysLikely` even if it becomes unused after this ticket — proposed as a follow-up cleanup if grep confirms zero other consumers
- No HTN method registration for sleep — flat GOAP per spec Planner-formalism analysis

## Acceptance Result

1. Passed: focused schema tests cover `CandidateBacked`, Sleep relevant ops, and Sleep progress-barrier ops.
2. Passed: focused feasibility tests cover Sleep's `CandidateBacked` routing.
3. Passed: focused candidate-generation tests cover KnownRestSite emission, targetless RoughSleep fallback, and full-rest-site filtering.
4. Passed: existing inline candidate-generation tests were updated for targetless RoughSleep and belief-backed rest sites.
5. Passed: `DECL_SLEEP` no longer uses `FeasibilityStrategy::AlwaysLikely`; `Relieve` and `FreeCarryCapacity` still use `AlwaysLikely`, so no zero-consumer cleanup follow-up is needed.
6. Passed: the two-path emitter remains flat GOAP; no HTN method was added.

## Test Plan Result

1. Added `goal_schema.rs` coverage for `FeasibilityStrategy::CandidateBacked` and Sleep declaration shape.
2. Added `candidate_generation.rs` coverage for KnownRestSite plus RoughSleep emission and full-site filtering.
3. Updated `feasibility.rs` coverage for Sleep's candidate-backed hint path.
4. Updated two golden fixture files so remote rest-site candidates have lawful capacity and belief-backed empty contention evidence.

## Outcome

Completed on 2026-05-26.

Sleep goal selection is now candidate-backed. Rest-capable places produce KnownRestSite candidates only when the actor can lawfully see capacity and a non-full occupant count; RoughSleep remains a targetless fallback grounded in the actor's current place evidence.

## Deviations

1. The drafted `SLEEP_OPS = [Sleep, QueueForFacilityUse]` shape was narrowed to `SLEEP_RELEVANT_OPS = [Sleep, QueueForFacilityUse]` plus `SLEEP_PROGRESS_BARRIER_OPS = [Sleep]` because live planner tests explicitly exclude Sleep from queue progress-barrier handling.
2. The drafted RoughSleep `target_place` plus separate flag carrier was replaced with the live archived ticket 004 seam: targetless sleep means RoughSleep, and targeted current rest-capable sleep means KnownRestSite.
3. The drafted `./scripts/verify.sh` row was not run as a wrapper during this per-ticket iteration. Its live subcommands were run directly; the implement-spec-tickets final branch phase still owns the full wrapper gate before push.

## Verification Result

- Passed `cargo test -p worldwake-ai goal_schema::tests`
- Passed `cargo test -p worldwake-ai candidate_generation::tests`
- Passed `cargo test -p worldwake-ai feasibility::tests`
- Passed `cargo test -p worldwake-ai --test golden_ai sleep_ranking_prefers_clean_place_over_dirty_place`
- Passed `cargo test -p worldwake-ai --test golden_ai site_preference_adopts_higher_quality_sleep_place`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test --workspace`
- Passed `cargo fmt --all -- --check`
- Passed `cargo clippy --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Waived `./scripts/verify.sh` wrapper because this per-ticket iteration ran every live wrapper subcommand directly and the final implement-spec-tickets branch phase still owns the full pre-PR wrapper gate before push.
