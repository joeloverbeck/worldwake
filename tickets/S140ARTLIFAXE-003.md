# S140ARTLIFAXE-003: Planner actionability gate, Discrepancy::ArtifactNotActionable, decision-trace axis surface

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — adds `Discrepancy::ArtifactNotActionable { artifact, reason }` variant; extends `decision_trace.rs` with axis-value rendering for ranked candidates' anchored artifacts
**Deps**: archive/tickets/S140ARTLIFAXE-001.md

## Problem

Ticket 001 mechanically migrated `crates/worldwake-ai/src/candidate_generation.rs:650` from `ArtifactState::Active` to `ArtifactActionability::Actionable` so the workspace builds. But the planner has no typed cause for "this candidate was rejected because the artifact is not actionable" — today the rejection is silent (the candidate simply isn't emitted). Per spec D6 and `references/worldwake-validation-patterns.md` "Discrepancy as Failure-Attribution Surface", the spec selects option (1): a new typed `Discrepancy` variant. This ticket adds `Discrepancy::ArtifactNotActionable { artifact: EntityId, reason: BlockerReason }`, threads it through the rejection site at `candidate_generation.rs:650`, and extends `decision_trace.rs` to render the five axis values for any artifact referenced by a ranked candidate so the rejection cause is locally inspectable.

## Assumption Reassessment (2026-05-06)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. After ticket 001 lands, `crates/worldwake-ai/src/candidate_generation.rs:650` reads `header.actionability == ArtifactActionability::Actionable` (the mechanical post-001 substitution). The pre-001 site checked `header.kind != Bounty || header.state != ArtifactState::Active`. Existing tests covering the gate live in the surrounding `candidate_generation.rs` inline test block plus `crates/worldwake-ai/tests/golden_offices.rs` and `golden_survival_justice.rs`. Decision-trace surfaces live in `crates/worldwake-ai/src/decision_trace.rs`; pre-S140 they carry no artifact axis state.
2. Spec deliverable D6 names the substitution and the typed-discrepancy attribution. Spec FND-30 #11 (causal records) mandates that the rejection cause be reconstructable from inspection. `Discrepancy` enum lives at `crates/worldwake-core/src/discrepancy.rs:9-40` with derives `Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`. `BlockerReason` is defined by ticket 001 in `crates/worldwake-core/src/social_artifact.rs` as a `Copy` enum.
3. **Cross-system shared abstraction boundary**: The boundary under audit is the planner's candidate-emission site (gate decision in `worldwake-ai`) and the decision-trace surface (consumed by both goldens and the observer in `worldwake-cli`). The trace surface is dual-use per the "Dual-Use Read-Model Types" pattern; no test-only types are introduced.
5. **Live `GoalKind` under test**: `GoalKind::FulfillBounty { bounty }` (`crates/worldwake-core/src/goal.rs:129`) is the only goal kind today filtered on `ArtifactState`. Per spec Non-Goals, this ticket does not broaden the gate to `Accuse`, `PunishAccused`, `PostBounty`, or `PostNotice` — the gate remains scoped to `FulfillBounty`. The current operator/affordance surface (post-001) reads `actionability` not `state`; this ticket surfaces the rejection cause without changing the gate's domain.
6. **AI-regression layer**: This ticket affects candidate-generation focused/unit coverage AND decision-trace integration coverage. The local needs-only harness is sufficient for the unit-level gate test; the decision-trace assertion uses the integration-level `agent_tick` harness with a single bounty-bearing agent.
13. **Adjacent-contradiction classification**: If implementation discovers that `Discrepancy` has any exhaustive `match` site that doesn't already include a wildcard arm, the new variant forces an arm there. Reassess found ~145 `Discrepancy` use sites total but most are `Err(Discrepancy::X)` construction sites; the genuinely-exhaustive `match d { ... }` sites are the subset requiring new arms. Any such site discovered at implementation time is in-scope for this ticket (not a separate follow-up).

## Architecture Check

1. The new `Discrepancy::ArtifactNotActionable` variant carries `(artifact: EntityId, reason: BlockerReason)`. Both payload types are `Copy`, preserving `Discrepancy`'s `Copy` derive — no `Copy → Clone` ripple.
2. Decision-trace axis-value rendering is dual-use (consumed by both goldens and observer) and lives in `crates/worldwake-ai/src/decision_trace.rs` per the dual-use pattern. No `tests/`-only placement.
3. The gate scope is intentionally narrow per FND-26: broadening to other goal kinds is a separate decision. The narrow scope keeps this ticket's blast radius small and lets goal-kind-specific actionability semantics emerge organically when those goals' domains warrant it.
4. The rejection site emits the `Discrepancy::ArtifactNotActionable` through the existing typed-discrepancy path landed by S109; no new infrastructure is needed.

## Verification Layers

1. Planner candidate suppression on non-actionable artifact → decision trace assertion that `Discrepancy::ArtifactNotActionable { artifact: <id>, reason: <BlockerReason> }` is recorded at the rejection point. Per `docs/precision-rules.md` Rule 6 (Decision-Trace Preference), this is the strongest available proof surface for AI rejection reasoning.
2. Reason payload accuracy → decision trace assertion that `reason` matches the artifact's actual non-actionable axis state (e.g., `LegalEffectExpired` when `actionability == Closed { cause: LegalEffectExpired }`).
3. Decision-trace axis rendering on ranked candidates → focused decision-trace test asserting all 5 axis values appear for any artifact referenced by a ranked candidate.
4. Single-layer scope: this ticket is AI-side observability + a typed-discrepancy addition. The cascade that produces the non-actionable state is verified at S140ARTLIFAXE-002; the E2E behavior is verified at S140ARTLIFAXE-006.

## What to Change

### 1. Add `Discrepancy::ArtifactNotActionable { artifact: EntityId, reason: BlockerReason }` in `crates/worldwake-core/src/discrepancy.rs`

Add the new variant to the `Discrepancy` enum. Import `BlockerReason` from `crate::social_artifact`. Verify Copy derive still holds (both payload types are Copy). Update any exhaustive `match d { ... }` sites discovered during workspace-wide grep (`grep -rn "match.*Discrepancy" crates/`) to add an arm for the new variant.

### 2. Migrate the rejection site at `candidate_generation.rs:650` to record the typed discrepancy

The post-001 site reads `header.actionability`. When the gate rejects, emit `Discrepancy::ArtifactNotActionable { artifact: <bounty_id>, reason: <derive from actionability variant> }` instead of silently skipping. The `reason` derivation:

- `actionability: Closed { cause: BountyFulfilled }` → `BlockerReason::AwaitingAdjudication` is wrong; introduce a `Closed`-flavored mapping. Per spec D1's `BlockerReason` variants (`LegalEffectExpired, LegalEffectRevoked, JurisdictionConflict, AwaitingAdjudication`), map closed-cause to the corresponding blocker reason. If the `BlockerReason` variant set as defined in 001 does not cover all `CloseCause` values, extend `BlockerReason` here as part of the deliverable scope.

### 3. Extend `decision_trace.rs` with axis-value rendering for ranked candidates' anchored artifacts

Add a per-artifact axis snapshot to the decision-trace structure populated for any ranked candidate that anchors on an artifact. The snapshot carries the 5 axis values copied from `ArtifactHeader` at trace-population time. Rendering format follows existing decision-trace conventions in the same module.

### 4. Update existing tests and add new gate / trace tests

Add unit tests for the `Discrepancy::ArtifactNotActionable` recording and the decision-trace axis rendering. Existing tests at `golden_offices.rs:2249` and `golden_survival_justice.rs:329` (post-001) may need extension if their decision-trace assertions have hardcoded the pre-S140 trace shape.

## Files to Touch

- `crates/worldwake-core/src/discrepancy.rs` (modify — add `ArtifactNotActionable` variant)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — emit typed discrepancy at the rejection site)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — axis-value rendering for ranked-candidate-anchored artifacts)
- Likely: `crates/worldwake-core/src/social_artifact.rs` (modify — extend `BlockerReason` if 001's initial variant set does not cover all `CloseCause` mappings); pin during implementation
- Likely: exhaustive-match sites discovered by `grep -rn "match.*Discrepancy" crates/`; pin during implementation

## Out of Scope

- Broadening the actionability gate to other `GoalKind` variants (`Accuse`, `PunishAccused`, `PostBounty`, `PostNotice`) — explicit Non-Goal in spec D6.
- New scoring/priority adjustments based on axis values — ranking is unchanged.
- Observer rendering of axis values — covered by S140ARTLIFAXE-005.
- Discrepancy variant additions for axes outside actionability (e.g., a "credibility too low" discrepancy for ranking suppression) — not in spec scope.
- Authoritative-to-AI Impact Rule sub-points 1, 2, 4–7 — this ticket's gate change is a candidate-emission filter substitution (not a precondition or `validate_*` change), so the rule's full 7-point checklist is N/A. The relevant point is `generate_candidates` (point 2), exercised by existing AI-side coverage at this ticket and by the spec's Validation goldens at S140ARTLIFAXE-006.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core --lib discrepancy` — new variant participates in `Copy` derive; trait-bounds tests pass.
2. `cargo test -p worldwake-ai --lib candidate_generation` — gate rejection on non-actionable artifact records `Discrepancy::ArtifactNotActionable` with correct `BlockerReason`.
3. `cargo test -p worldwake-ai --lib decision_trace` — ranked candidate trace renders all 5 axis values for anchored artifacts.
4. `cargo test -p worldwake-ai --test golden_offices` — passes with extended decision-trace shape.
5. `cargo test -p worldwake-ai --test golden_survival_justice` — passes with extended decision-trace shape.
6. Existing suite: `cargo test --workspace`.

### Invariants

1. `Discrepancy` retains `Copy` derive.
2. Every `Discrepancy::ArtifactNotActionable` recorded at `candidate_generation.rs:650` has both `artifact` and `reason` populated.
3. `BlockerReason` covers every `CloseCause` value the rejection mapping needs (extend in step 2 if 001's variant set is insufficient).
4. The actionability gate's domain is unchanged: only `GoalKind::FulfillBounty` is filtered.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/discrepancy.rs` (modify) — extend or add a unit test asserting the new variant's `Copy` participation and serde round-trip.
2. `crates/worldwake-ai/src/candidate_generation.rs` (modify) — add an inline unit test `bounty_candidate_rejects_with_typed_discrepancy_when_not_actionable` that constructs a non-actionable bounty and asserts the recorded discrepancy.
3. `crates/worldwake-ai/src/decision_trace.rs` (modify) — add a unit test asserting axis-value rendering for ranked candidates anchoring on artifacts.

### Commands

1. `cargo test -p worldwake-core --lib discrepancy`
2. `cargo test -p worldwake-ai --lib candidate_generation decision_trace`
3. `cargo test --workspace`
4. `scripts/verify.sh`
