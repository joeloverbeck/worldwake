# S34GENEPIACT-006: Candidate generation — emit_verify_belief_goals()

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai: new candidate generation function in candidate_generation.rs
**Deps**: S34GENEPIACT-001 (core types), S34GENEPIACT-005 (GoalKindTag::VerifyBelief and planner ops exist)

## Problem

Agents never generate `VerifyBelief` goal candidates. Without `emit_verify_belief_goals()`, the AI pipeline has no entry point for epistemic behavior — agents cannot decide to verify low-confidence beliefs even when those beliefs underpin costly plans.

## Assumption Reassessment (2026-03-28)

1. `candidate_generation.rs` is in `crates/worldwake-ai/src/candidate_generation.rs` (~10176 lines). The `emit_recorded_violation_candidates()` function (lines 2103-2123) is the structural pattern — it guards on a disposition profile, iterates relevant state, and calls `emit_candidate()`.
2. The spec says `emit_verify_belief_goals()` runs AFTER all other `emit_*` functions. It scans already-emitted `GroundedGoal` candidates for low-confidence belief dependencies. This is a second-pass scan, not a direct belief-store scan.
3. The confidence check uses `belief_confidence(source, staleness_ticks, policy) < profile.belief_verification_threshold`. The `belief_confidence()` function exists in E14's belief infrastructure.
4. Deduplication: skip emission if a `VerifyBelief` candidate with the same `VerificationSubject` (ignoring `generation_tick`) already exists in the candidate list.
5. Conversation memory suppression: when generating candidates, check `HeardBeliefMemory` entries to suppress `AskWitness` as a planner option for recently-asked topics. This is done via the affordance system — the `ask_witness` affordance payload enumerator (ticket 004) skips recently-asked targets. The candidate generation side does NOT filter `AskWitness` — it generates `VerifyBelief` candidates and lets the planner/affordance system handle witness suppression.
6. `emit_candidate()` / `emit_candidate_with_trace()` is the standard emission API. It takes `GoalKind`, `OpportunityAnchor`, and evidence.
7. This ticket does NOT add `GoalFamilyPolicy` — that is ticket 007 (ranking).

## Architecture Check

1. The second-pass scan (scanning already-emitted candidates for belief dependencies) is the spec's design choice. This ties verification to goal-relevant beliefs, not a broad tick-based scan of all beliefs. This is correct per P18 (resource-bounded reasoning) and P5 (carriers of consequence).
2. No backward-compatibility shims. New function only, called at end of candidate generation.

## Verification Layers

1. `VerifyBelief` candidate emitted when belief confidence below threshold -> focused candidate gen test with decision trace
2. `VerifyBelief` candidate NOT emitted when agent lacks `VerificationDispositionProfile` -> focused candidate gen test
3. `VerifyBelief` candidate scans already-emitted candidates for belief dependencies -> focused candidate gen test (emit a needs goal, verify the verification candidate references its belief dependency)
4. `VerifyBelief` deduplicates: same `VerificationSubject` not emitted twice -> focused candidate gen test
5. `VerifyBelief` (SupplyAvailability) emitted when resource source belief is stale -> focused candidate gen test
6. `VerifyBelief` candidate NOT emitted when belief confidence is above threshold -> focused candidate gen test

## What to Change

### 1. Add `emit_verify_belief_goals()` function

In `crates/worldwake-ai/src/candidate_generation.rs`, add:

```rust
pub(crate) fn emit_verify_belief_goals(
    agent: EntityId,
    view: &impl GoalBeliefView,
    current_tick: Tick,
    candidates: &mut Vec<GroundedGoal>,
    trace: &mut Option<CandidateGenerationTrace>,
) {
    // 1. Guard: return if agent lacks VerificationDispositionProfile
    // 2. Scan existing candidates for belief dependencies (evidence_entities)
    // 3. For each evidence entity, check belief_confidence < threshold
    // 4. Determine VerificationSubject based on belief contents
    // 5. Deduplicate by VerificationSubject
    // 6. Emit VerifyBelief { subject, generation_tick: current_tick }
}
```

### 2. Call from generate_candidates()

Add the call to `emit_verify_belief_goals()` AFTER all existing `emit_*` calls in the main `generate_candidates()` function. This ensures the second-pass scan has access to all already-emitted candidates.

### 3. Wire evidence extraction

Implement helper(s) to extract `evidence_entities` and `evidence_places` from existing `GroundedGoal` candidates. For each entity in evidence, look up `BelievedEntityState` and check confidence. Map low-confidence beliefs to `VerificationSubject` variants:
- Entity with `last_known_place: Some(place)` -> `EntityLocation { entity, place }`
- Resource source entity -> `SupplyAvailability { commodity, source, place }`

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add emit_verify_belief_goals function, call from generate_candidates)

## Out of Scope

- `ask_witness` affordance enumeration conversation memory suppression — already in ticket 004
- Ranking and motive scoring — ticket 007
- `GoalFamilyPolicy` for VerifyBelief — ticket 007
- Golden E2E tests — ticket 008
- Changes to `GoalBeliefView` trait (should already expose `verification_disposition_profile()` via component access — if not, add the accessor in this ticket)
- Changes to belief confidence computation
- Planner search (ticket 005)

## Acceptance Criteria

### Tests That Must Pass

1. `VerifyBelief` candidate emitted only when belief confidence below threshold
2. `VerifyBelief` candidate not emitted when agent lacks `VerificationDispositionProfile`
3. `VerifyBelief` candidate scans already-emitted candidates for belief dependencies
4. `VerifyBelief` deduplicates: same `VerificationSubject` not emitted twice regardless of `generation_tick`
5. `VerifyBelief` (SupplyAvailability) emitted when resource source belief is stale
6. `VerifyBelief` NOT emitted when belief confidence is above threshold (no unnecessary verification)
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Candidate generation reads belief state only, never authoritative world state (P12)
2. Verification candidates are goal-relevant (tied to already-emitted candidates), not a broad scan of all beliefs (P5, P18)
3. Deduplication prevents candidate proliferation for the same subject
4. No `HashMap`/`HashSet` in deduplication logic — use sorted iteration or `BTreeSet`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (in-module tests or dedicated test file) — 6 focused candidate generation tests per spec test list items 13-18
2. Verify `GoalBeliefView` exposes `verification_disposition_profile()` accessor — compilation test

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`
3. `cargo build --workspace`
