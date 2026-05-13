# S139EPISENSUB-001: Foundation goal-layer wiring for AskWitness

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` (`goal.rs`), `worldwake-ai` (`goal_dispatch_key.rs`, `goal_dispatch_decl.rs`, `goal_model.rs`, `planning_state.rs`, `ranking.rs`, `search/transition.rs`), `worldwake-sim` (`belief_view.rs`), `worldwake-cli` (`display.rs`)
**Deps**: specs/S139-epistemic-sensing-subgoals.md

## Problem

Before this ticket, the action layer for asking a witness was fully shipped (`crates/worldwake-systems/src/epistemic_actions.rs`, `crates/worldwake-sim/src/action_payload.rs:364`), but no `GoalKind` variant existed for an agent to *adopt the intent* to ask a witness. The repair search (S137 `RepairKind::InsertVerification`) could not splice an ask step before a guard that depends on a low-confidence belief. This foundation ticket added the goal-layer surface — the variant, its dispatch key, its `GoalKindPlannerExt` integration, and the belief-view accessor that the satisfaction predicate reads — so that subsequent tickets can wire the candidate emitter, ranking, and golden coverage on top.

The foundation landed in one ticket because `GoalKindPlannerExt` performs an exhaustive match across all `GoalKind` variants at `crates/worldwake-ai/src/goal_model.rs:546+`; adding the variant in one ticket and the match arms in another would leave the workspace uncompilable between them.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalKind` is defined at `crates/worldwake-core/src/goal.rs:62` with `#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]` at line 61. `TellTopic` is `Copy` at `crates/worldwake-core/src/belief.rs:1736`, and `EntityId` is `Copy`, so `AskWitness { witness: EntityId, topic: TellTopic }` preserves the existing `Copy` derive without payload-derive widening. `GoalDispatchKey` was bumped from `[Self; 40]` to `[Self; 41]`.
2. `GoalKindPlannerExt` trait is defined at `goal_model.rs:40-82` with 11 methods (verified by direct grep). This ticket added the private helpers that translate `TellTopic::EntityBelief` into an `AskWitnessPayload`, reject unsupported topics, read the entity subject, and evaluate the report-provenance satisfaction predicate.
3. Shared abstraction boundary under audit: the `GoalKind` enum (worldwake-core) — every consumer in worldwake-ai's planning pipeline pattern-matches on it. Exhaustive match sites included the `GoalKindPlannerExt` impl in `goal_model.rs:546+`, `GoalDispatchKey::from_goal_kind` in `goal_dispatch_key.rs:99+`, and the ranking pipeline's priority and motive_score functions in `ranking.rs`. All gained `AskWitness` arms in this ticket, with placeholder priority/motive in ranking to be replaced by ticket 005.
4. Live `GoalKind` under test: this ticket added a variant whose planner behavior mirrors the existing structural analogs exercised by `ShareBelief`, `ExploreLocation`, and `InvestigateViolation` goldens. Existing operator surface for `PlannerOpKind::AskWitness` is at `planner_ops.rs:47` and classified at line 136; no operator change was required.
5. Existing inline tests exercising the modified surfaces (named per docs/precision-rules.md Rule 3):
   - `goal_model.rs:2518 goal_priority_class_satisfies_required_bounds`
   - `goal_model.rs:2527 grounded_goal_satisfies_required_bounds`
   - `goal_model.rs:2715 ranked_goal_provenance_family_is_payload_aware`
   - `goal_model.rs:2774 consume_goal_relevant_ops_include_consumption_and_pickup_only`
   - `goal_dispatch_key.rs:377 test_goal_dispatch_key_exhaustive_coverage`
   - `goal_dispatch_key.rs:483 test_goal_dispatch_key_all_lists_each_dispatch_key_once`
   These tests assert exhaustive coverage over `GoalKind` / `GoalDispatchKey` and were extended for `AskWitness`.
6. Mismatch + correction: the spec's earlier draft proposed a new `EpistemicProfile` component and `register_component_schema()` function; both were corrected during reassessment (the existing `EpistemicDispositionProfile` is reused; `with_component_schema_entries!` is the actual macro name). This ticket's surface area is unaffected because no component registration is required here.
7. Mismatch + correction: live `GoalKindPlannerExt::ranked_goal_provenance_family()` and `relevant_op_kinds()` delegate through `GoalDispatchKey::declaration()`. Adding `GoalDispatchKey::AskWitness` without a declaration left the workspace uncompilable, so this ticket landed a minimal `DECL_ASK_WITNESS` using the existing share-belief testimony policy. Ticket 003 now owns the dedicated `EPISTEMIC_SENSING_POLICY` and any declaration-policy refinement, not the first declaration entry.

## Architecture Check

1. The goal-layer wiring stayed decoupled from the action layer (FND-26). `GoalKind::AskWitness` references only `EntityId` and `TellTopic` (both core types), and the satisfaction predicate reads belief state through the existing `GoalBeliefView` facade — no direct cross-system calls.
2. No backwards-compat shims. The new variant lands directly on `GoalKind` with no parallel "legacy ask intent" type. Existing pattern-match sites are updated in place; no `#[deprecated]` markers, no shim wrappers.
3. The placeholder priority/motive_score for `AskWitness` in `ranking.rs` is named explicitly here as "placeholder, replaced by ticket 005" so reviewers do not mistake it for the final contract. Ticket 005 references this placeholder back.

## Verified Layers

1. New `GoalKind::AskWitness` variant exists and routes through dispatch → unit test in `goal_dispatch_key.rs`'s `test_goal_dispatch_key_all_lists_each_dispatch_key_once` extended to assert `Self::AskWitness` is present in `ALL`.
2. `GoalKindPlannerExt::is_satisfied` for `AskWitness` reads `entity_beliefs_sourced_from_witness` → focused unit test in `goal_model.rs` exercising the satisfaction predicate against a stubbed `GoalBeliefView` impl.
3. `GoalBeliefView::entity_beliefs_sourced_from_witness` returns `BTreeMap`-ordered entries → focused unit test in `belief_view.rs` against a fixture `AgentBeliefStore` with mixed `PerceptionSource` variants, asserting only `Report { from: witness, .. }` entries surface.
4. Single-layer ticket additional-layer-mapping was not applicable for the placeholder ranking arms: their proof surface was "workspace compiles + existing `ranking.rs` tests stay green"; the real ranking contract remains ticket 005's test surface.

## Landed Changes

### 1. Add `GoalKind::AskWitness` variant

In `crates/worldwake-core/src/goal.rs` (enum at line 62), added the variant:

```rust
AskWitness {
    witness: EntityId,
    topic: TellTopic,
},
```

`TellTopic` was already imported (line 6). No new derives were needed.

### 2. Extend `GoalDispatchKey`

In `crates/worldwake-ai/src/goal_dispatch_key.rs`:
- Added the `AskWitness` variant.
- Bumped `ALL: [Self; 40]` to `[Self; 41]` and added `Self::AskWitness` in deterministic order.
- Added the `GoalKind::AskWitness { .. } => Self::AskWitness` arm to `from_goal_kind`.
- Extended `test_goal_dispatch_key_exhaustive_coverage` and `test_goal_dispatch_key_all_lists_each_dispatch_key_once` to cover `AskWitness`.

### 3. Add `GoalKindPlannerExt` match arms

In `crates/worldwake-ai/src/goal_model.rs`, added arms for `GoalKind::AskWitness { witness, topic }` to the trait method impls at line 546+:

1. `ranked_goal_provenance_family` → `Some(RankedGoalProvenanceFamily::EpistemicSensing)`. `EpistemicSensing` was added as a `RankedGoalProvenanceFamily` variant.
2. `relevant_op_kinds` → `&[PlannerOpKind::Travel, PlannerOpKind::AskWitness]`.
3. `target_commodity` → `None`.
4. `relevant_observed_commodities` → `None`.
5. `build_payload_override` → constructs `AskWitnessPayload { target: witness, topic_entity: Some(subject), topic_commodity: None }` from `TellTopic::EntityBelief { subject }`; returns `Err(GoalPayloadOverrideError::UnsupportedTopic)` for `SocialObservation` / `InstitutionalClaim` variants.
6. `is_progress_barrier` → `step.op == PlannerOpKind::AskWitness && step.target_binding == witness`.
7. `is_satisfied` → reads `state.view.entity_beliefs_sourced_from_witness(agent, *witness)` and returns `true` when a belief on `topic`'s subject exists with `source = PerceptionSource::Report { from: *witness, .. }` and confidence ≥ `stale_evidence_barrier_threshold`. The landed helper carries a `TODO(S139EPISENSUB-002)` for the later witness-recency preference refinement.
8. `goal_relevant_places` → `vec![state.view.effective_place(*witness).unwrap_or(state.agent_place)]`.
9. `prerequisite_places` → same as `goal_relevant_places`.
10. `matches_binding` → for `PlannerOpKind::AskWitness`: `authoritative_targets.contains(witness)`; for `PlannerOpKind::Travel`: standard travel-binding (place reach).
11. `candidate_is_available` → `true` when the belief envelope holds a topic-subject entry with confidence below `stale_evidence_barrier_threshold` AND the `AskWitnessMemoryKey { counterparty: *witness, topic_entity, topic_commodity }` cooldown window has elapsed.

### 4. Add `GoalBeliefView::entity_beliefs_sourced_from_witness`

In `crates/worldwake-sim/src/belief_view.rs`, added the belief-view method:

```rust
fn entity_beliefs_sourced_from_witness(
    &self,
    agent: EntityId,
    witness: EntityId,
) -> Vec<(EntityId, BelievedEntityState)>;
```

The backing social belief view iterates `AgentBeliefStore.known_entities` (`BTreeMap`) and filters entries whose `source` matches `PerceptionSource::Report { from, .. }` with `from == witness`. The goal-view facade forwards through the same helper, and `PlanningState` overrides the method against the actor snapshot used by planner tests.

### 5. Add placeholder ranking arms for `GoalKind::AskWitness`

In `crates/worldwake-ai/src/ranking.rs`, added arms to the priority-class and `motive_score` exhaustive matches:

- Priority class: `GoalPriorityClass::Background` (placeholder — replaced by ticket 005 to `Low`).
- `motive_score`: returns `0` (placeholder — replaced by ticket 005 with the witness-recency-weighted formula).

Both paths are marked with `TODO(S139EPISENSUB-005)` placeholder comments.

### 6. Minimal dispatch declaration and display fallout

Live declaration lookup required a minimal `DECL_ASK_WITNESS`, representative fixture, and exhaustive declaration-test coverage in `crates/worldwake-ai/src/goal_dispatch_decl.rs`. `search/transition.rs` maps unsupported AskWitness topics to the existing unsupported-goal trace reason, and `worldwake-cli/src/display.rs` formats the new goal variant.

### 7. Verified `PlannerOpKind::AskWitness` classification

Confirmed `PlannerOpKind::AskWitness` at `planner_ops.rs:47` and the `classify_action_def` arm at line 136 (`(ActionDomain::Epistemic, "ask_witness") => Some(PlannerOpKind::AskWitness)`) were unchanged.

## Files Touched

- `crates/worldwake-core/src/goal.rs` (modify — D1 variant)
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify — D3 variant + ALL bump + match arm)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — minimal live declaration required by metadata lookup)
- `crates/worldwake-ai/src/goal_model.rs` (modify — D6 match arms in 11 methods)
- `crates/worldwake-ai/src/planning_state.rs` (modify — planner snapshot override for report-provenance belief lookup)
- `crates/worldwake-ai/src/search/transition.rs` (modify — trace reason mapping for unsupported topic payloads)
- `crates/worldwake-sim/src/belief_view.rs` (modify — D8 trait method + RuntimeBeliefView impl + `impl_goal_belief_view!` forwarding)
- `crates/worldwake-ai/src/ranking.rs` (modify — placeholder priority + motive_score arms for AskWitness)
- `crates/worldwake-cli/src/display.rs` (modify — exhaustive goal formatter arm)
- `crates/worldwake-ai/src/planner_ops.rs` (verified only — D10)

## Out of Scope

- `EPISTEMIC_SENSING_POLICY` constant in `goal_policy.rs` — ticket 003.
- Dedicated `DECL_ASK_WITNESS` policy refinement to `EPISTEMIC_SENSING_POLICY` — ticket 003.
- Real priority class (Low) and `motive_score` formula for `AskWitness` — ticket 005 replaces the placeholders left here.
- `emit_ask_witness_candidates` in `candidate_generation.rs` — ticket 004.
- Extending `EpistemicDispositionProfile` with `witness_recency_preference` — ticket 002. The `is_satisfied` predicate uses `stale_evidence_barrier_threshold` only until 002 lands; the recency-window refinement is the TODO marker.
- Golden coverage for `AskWitness` end-to-end — ticket 006.

## Acceptance Result

### Tests Passed

1. New focused unit test in `goal_model.rs`'s `#[cfg(test)]` block asserting `GoalKindPlannerExt::is_satisfied` for `GoalKind::AskWitness` returns `true` when a `BelievedEntityState` exists with matching `PerceptionSource::Report` provenance and confidence ≥ `stale_evidence_barrier_threshold`.
2. New focused unit test in `goal_model.rs` asserting `build_payload_override` for `GoalKind::AskWitness { witness, topic: TellTopic::EntityBelief { subject } }` returns `AskWitnessPayload { target: witness, topic_entity: Some(subject), topic_commodity: None }`.
3. New focused unit test in `goal_model.rs` asserting `build_payload_override` returns `Err(GoalPayloadOverrideError::UnsupportedTopic)` for `TellTopic::SocialObservation` / `InstitutionalClaim`.
4. New focused unit test in `belief_view.rs` asserting `entity_beliefs_sourced_from_witness` returns only entries whose `PerceptionSource::Report { from, .. }` matches the named witness.
5. Existing `test_goal_dispatch_key_exhaustive_coverage` and `test_goal_dispatch_key_all_lists_each_dispatch_key_once` pass with `AskWitness` arms.
6. Existing suite: `cargo test -p worldwake-ai`, `cargo test -p worldwake-sim`, and `cargo test -p worldwake-core` all passed.

### Invariants

1. `GoalKind` derives `Copy` unchanged after the new variant lands (all variant fields are `Copy`).
2. `GoalDispatchKey::ALL` covers every `GoalKind` variant in `from_goal_kind`'s match — verified by `test_goal_dispatch_key_exhaustive_coverage`.
3. `GoalKindPlannerExt` impls exhaustively match all `GoalKind` variants (workspace compiles with `-D warnings`; no `_` fallback arms introduced).
4. `RuntimeBeliefView::entity_beliefs_sourced_from_witness` iterates `AgentBeliefStore.known_entities` in `BTreeMap` order (determinism — AGENTS.md Critical Invariants).

## Test Plan Result

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — added 3 focused unit tests per Acceptance Result #1-#3.
2. `crates/worldwake-sim/src/belief_view.rs` — added 1 focused unit test per Acceptance Result #4.
3. `crates/worldwake-ai/src/goal_dispatch_key.rs` — extended fixture arrays/asserts to cover `AskWitness`.

### Commands

1. Passed `cargo test -p worldwake-ai --lib ask_witness_goal`.
2. Passed `cargo test -p worldwake-ai --lib goal_dispatch_key::tests`.
3. Passed `cargo test -p worldwake-ai --lib goal_dispatch_decl::tests`.
4. Passed `cargo test -p worldwake-sim --lib entity_beliefs_sourced_from_witness_filters_report_provenance_in_key_order`.
5. Passed `cargo test -p worldwake-core`.
6. Passed `cargo test -p worldwake-sim`.
7. Passed `cargo test -p worldwake-ai`.
8. Passed `cargo clippy --workspace --all-targets -- -D warnings`.
9. Passed `./scripts/verify.sh`.

## Outcome

Completed on 2026-05-13.

- Added `GoalKind::AskWitness`, `GoalDispatchKey::AskWitness`, payload override support for `TellTopic::EntityBelief`, report-provenance satisfaction logic, relevant-place/prerequisite-place routing to the witness, exact AskWitness binding, and availability checks gated by stale-confidence plus ask-witness cooldown memory.
- Added the witness-sourced belief-view accessor through `SocialBeliefView`, `GoalBeliefView`, and `PlanningState`, with deterministic report-source filtering covered by a sim unit test.
- Added the staged ranking placeholders (`Background` priority and zero motive) with `TODO(S139EPISENSUB-005)` markers, leaving real ranking/motive behavior to ticket 005.
- Landed a minimal `DECL_ASK_WITNESS` because live planner metadata delegates through declarations; ticket 003 now owns the dedicated `EPISTEMIC_SENSING_POLICY` and policy refinement.

## Deviations

- The original split said `DECL_ASK_WITNESS` was entirely out of scope for ticket 001, but live declaration lookup made a minimal declaration required for compilation. The declaration currently reuses `SHARE_BELIEF_TESTIMONY_POLICY`; ticket 003 was narrowed to the dedicated policy/refinement work.
- CLI goal display and root-candidate payload-error mapping were touched only as exhaustive shared-shape fallout from adding the new goal variant and unsupported-topic payload error.
- The attempted combined selector `cargo test -p worldwake-ai --lib goal_dispatch_key::tests goal_dispatch_decl::tests` was invalid and is not counted as verification; the two selectors were rerun separately and passed.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib ask_witness_goal`.
- Passed `cargo test -p worldwake-ai --lib goal_dispatch_key::tests`.
- Passed `cargo test -p worldwake-ai --lib goal_dispatch_decl::tests`.
- Passed `cargo test -p worldwake-sim --lib entity_beliefs_sourced_from_witness_filters_report_provenance_in_key_order`.
- Passed `cargo test -p worldwake-core`.
- Passed `cargo test -p worldwake-sim`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh`.
