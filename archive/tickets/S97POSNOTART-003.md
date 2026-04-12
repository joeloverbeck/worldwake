# S97POSNOTART-003: Runtime candidate generation + test fixture updates

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — candidate generation sets `expires_at` from profile; local candidate-generation test harness carries the profile
**Deps**: archive/tickets/S97POSNOTART-002.md

## Problem

Currently, the two runtime `ArtifactPostingContext` constructions in candidate generation hardcode `expires_at: None`, meaning planner-emitted posting goals do not provision TTL. This ticket reads `ArtifactPostingProfile` via `GoalBeliefView` at those runtime sites and updates the local candidate-generation harness/expected goals so focused proof matches the live contract.

## Assumption Reassessment (2026-04-12)

1. Runtime `ArtifactPostingContext` constructions with `expires_at: None` exist at exactly 2 locations in `crates/worldwake-ai/src/candidate_generation.rs`:
   - Line 642: `PostBounty` in `emit_bounty_posting_candidates` — needs `bounty_ttl`
   - Line 726: `PostNotice` (ThreatWarning) in `emit_notice_posting_candidates` — needs `threat_warning_ttl`
   Both are before `#[cfg(test)]` at line 4899. All other `expires_at: None` in this file (lines 6198, 11034, 11180, 11245) are test code.
2. `GenerationContext` at line 146 has `view: &dyn GoalBeliefView` and `current_tick: Tick` — both needed for `ctx.view.artifact_posting_profile(ctx.agent)` and `ctx.current_tick + ttl`.
3. The local `TestBeliefView` harness in `candidate_generation.rs` does not yet carry `ArtifactPostingProfile`, so focused candidate-generation proof would still observe `expires_at: None` until that harness adds the new profile accessor/state.
4. Other `ArtifactPostingContext` literals in `goal_dispatch_decl.rs`, `ranking.rs`, `feasibility.rs`, and `goal_policy.rs` are synthetic representative goals or policy fixtures, not runtime-produced candidate-generation outputs. They do not currently need to change to make this ticket's owned runtime contract truthful.
5. Existing same-domain autonomous posting goldens in `crates/worldwake-ai/tests/golden_integration.rs` assert exact selected `GoalKey` payloads for bounty and notice posting. Once runtime candidate generation provisions TTL, those expectations also need to move from `expires_at: None` to `Some(Tick(...))`.
6. `BelievedArtifactState` locations (route_threat.rs, exhaustion.rs, goal_model.rs, goal_dispatch_key.rs, search/tests.rs, plan_revalidation.rs) do NOT need changes — they represent observed artifact state that will naturally carry non-None `expires_at` once artifacts are posted with TTL.

## Architecture Check

1. Reading TTL from the belief view accessor (not directly from authoritative state) respects the belief-only planning invariant (FND-14). The profile is agent-local configuration accessed through the standard planning surface.
2. No backward-compatibility shims — `expires_at: None` is replaced with profile-derived values at construction time. No fallback path needed since the profile is universal with defaults.

## Verification Layers

1. Runtime candidate generation produces `expires_at: Some(...)` for posting goals → focused unit test on `emit_notice_posting_candidates` and `emit_bounty_posting_candidates`
2. Posted artifacts expire via `artifact_lifecycle_system` → golden test (ticket 005)
3. Existing autonomous posting goldens that pin exact selected goal payloads must be updated to the new lawful `expires_at: Some(...)` shape.
4. Single-layer production change in AI candidate generation; local harness parity is proved in the same focused candidate-generation tests, and cross-system lifecycle remains verified in ticket 005.

## What to Change

### 1. Update `emit_notice_posting_candidates`

In `crates/worldwake-ai/src/candidate_generation.rs` at line 726, replace `expires_at: None` with:

```rust
let posting_profile = ctx.view.artifact_posting_profile(ctx.agent);
let expires_at = posting_profile.map(|p| ctx.current_tick + p.threat_warning_ttl);
```

Use `expires_at` in the `ArtifactPostingContext` construction.

### 2. Update `emit_bounty_posting_candidates`

At line 642, replace `expires_at: None` with profile-derived computation using `bounty_ttl`.

### 3. Update candidate-generation harness and expected-goal fixtures

Add `ArtifactPostingProfile` storage/accessor support to the local `TestBeliefView` in `candidate_generation.rs`, then update the focused expected goals in that file to assert `expires_at: Some(Tick(current_tick + ttl))` for the runtime-generated `PostBounty` and `PostNotice` candidates.

### 4. Update existing autonomous posting golden expectations

Adjust the existing exact `GoalKey` expectations in `crates/worldwake-ai/tests/golden_integration.rs` for autonomous bounty/notice posting so they match the new runtime-produced posting payload shape with profile-derived TTL.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — 2 runtime sites + local test harness/expected-goal assertions)
- `crates/worldwake-ai/tests/golden_integration.rs` (modify — existing autonomous posting goal expectations only)

## Out of Scope

- `BelievedArtifactState` locations — these represent observation-side state, not posting-side construction
- Synthetic representative-goal fixtures outside `candidate_generation.rs` (`goal_dispatch_decl.rs`, `ranking.rs`, `feasibility.rs`, `goal_policy.rs`) unless focused proof shows they encode the runtime posting contract
- `plan_revalidation.rs` — its `expires_at: None` is on `PostBountyActionPayload`, not `ArtifactPostingContext`
- CLI scenario support (ticket 004)
- New bounded-count post/expire/re-post golden coverage (ticket 005)
- Artifact lifecycle system changes (none needed)

## Acceptance Criteria

### Tests That Must Pass

1. `emit_notice_posting_candidates` produces candidates with `expires_at: Some(Tick(current_tick + 48))` (default threat_warning_ttl)
2. `emit_bounty_posting_candidates` produces candidates with `expires_at: Some(Tick(current_tick + 144))` (default bounty_ttl)
3. Local candidate-generation harness exposes `ArtifactPostingProfile` so focused candidate tests prove the real runtime contract
4. Existing autonomous posting goldens that assert exact `GoalKey` payloads pass with updated `expires_at: Some(...)` expectations
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No runtime `ArtifactPostingContext` construction in candidate_generation.rs uses `expires_at: None` after this ticket
2. Belief-only planning preserved — TTL read via `GoalBeliefView`, not authoritative state
3. Non-runtime synthetic goal fixtures outside `candidate_generation.rs` remain unchanged unless later tickets decide to normalize representative values

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — modify existing emission tests to assert `expires_at` is `Some`
2. `crates/worldwake-ai/tests/golden_integration.rs` — update existing autonomous posting exact-goal assertions
3. No other files required unless focused proof reveals an additional owned runtime contract site

### Commands

1. `cargo test -p worldwake-ai posting_candidates_emit_institutional_bounty_from_consulted_accusation`
2. `cargo test -p worldwake-ai posting_candidates_emit_threat_warning_notice_for_high_local_danger`
3. `cargo test -p worldwake-ai posting_candidates_emit_threat_warning_notice_for_remote_warned_place_from_belief`
4. `cargo test -p worldwake-ai golden_s51_autonomous_bounty_posting`
5. `cargo test -p worldwake-ai golden_s58_autonomous_notice_reroutes_later_travel`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed as a focused candidate-generation slice. Runtime posting candidates now derive `expires_at` from `ArtifactPostingProfile` through `GoalBeliefView`, the local candidate-generation harness carries that profile so focused tests prove the live contract, and the existing autonomous posting goldens were updated where they pin the exact selected `GoalKey` payload.

## Deviations

1. The original draft overclaimed unrelated synthetic fixture fallout in `goal_dispatch_decl.rs`, `ranking.rs`, `feasibility.rs`, and `goal_policy.rs`. Reassessment narrowed the owned production surface to `candidate_generation.rs`.
2. Focused proof exposed an additional same-domain verification dependency: existing autonomous posting goldens in `crates/worldwake-ai/tests/golden_integration.rs` assert exact posting payloads and therefore needed expectation updates in-scope. The separate new bounded-count post/expire/re-post golden scenario remains ticket 005.

## Verification Result

1. Passed `cargo test -p worldwake-ai posting_candidates_emit_institutional_bounty_from_consulted_accusation`
2. Passed `cargo test -p worldwake-ai posting_candidates_emit_threat_warning_notice_for_high_local_danger`
3. Passed `cargo test -p worldwake-ai posting_candidates_emit_threat_warning_notice_for_remote_warned_place_from_belief`
4. Passed `cargo test -p worldwake-ai golden_s51_autonomous_bounty_posting`
5. Passed `cargo test -p worldwake-ai golden_s58_autonomous_notice_reroutes_later_travel`
6. Passed `cargo test -p worldwake-ai`
7. Passed `cargo clippy --workspace --all-targets -- -D warnings`
