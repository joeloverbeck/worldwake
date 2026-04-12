# S97POSNOTART-003: Runtime candidate generation + test fixture updates

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate generation sets `expires_at` from profile
**Deps**: archive/tickets/S97POSNOTART-002.md

## Problem

Currently, all `ArtifactPostingContext` constructions in candidate generation hardcode `expires_at: None`, meaning posted artifacts never expire. This ticket reads the agent's `ArtifactPostingProfile` via `GoalBeliefView` to compute profile-driven `expires_at` values at the 2 runtime sites, and updates ~15 test fixtures for consistency.

## Assumption Reassessment (2026-04-12)

1. Runtime `ArtifactPostingContext` constructions with `expires_at: None` exist at exactly 2 locations in `crates/worldwake-ai/src/candidate_generation.rs`:
   - Line 642: `PostBounty` in `emit_bounty_posting_candidates` — needs `bounty_ttl`
   - Line 726: `PostNotice` (ThreatWarning) in `emit_notice_posting_candidates` — needs `threat_warning_ttl`
   Both are before `#[cfg(test)]` at line 4899. All other `expires_at: None` in this file (lines 6198, 11034, 11180, 11245) are test code.
2. `GenerationContext` at line 146 has `view: &dyn GoalBeliefView` and `current_tick: Tick` — both needed for `ctx.view.artifact_posting_profile(ctx.agent)` and `ctx.current_tick + ttl`.
3. Test `ArtifactPostingContext` fixtures (past `#[cfg(test)]` boundaries) exist in:
   - `candidate_generation.rs`: lines 11034, 11180, 11245
   - `goal_dispatch_decl.rs`: lines 803, 819, 828 (test at line 664)
   - `ranking.rs`: lines 2712, 2809, 2894, 2939, 2994, 3189, 3233, 3265 (test at line 1715)
   - `feasibility.rs`: lines 963, 981 (test at line 267)
   - `goal_policy.rs`: line 704 (test at line 121)
4. `BelievedArtifactState` locations (route_threat.rs, exhaustion.rs, goal_model.rs, goal_dispatch_key.rs, search/tests.rs, plan_revalidation.rs) do NOT need changes — they represent observed artifact state that will naturally carry non-None `expires_at` once artifacts are posted with TTL.

## Architecture Check

1. Reading TTL from the belief view accessor (not directly from authoritative state) respects the belief-only planning invariant (FND-14). The profile is agent-local configuration accessed through the standard planning surface.
2. No backward-compatibility shims — `expires_at: None` is replaced with profile-derived values at construction time. No fallback path needed since the profile is universal with defaults.

## Verification Layers

1. Runtime candidate generation produces `expires_at: Some(...)` → focused unit test on `emit_notice_posting_candidates` and `emit_bounty_posting_candidates`
2. Posted artifacts expire via `artifact_lifecycle_system` → golden test (ticket 005)
3. Single-layer change (AI candidate generation) — cross-system lifecycle verified in ticket 005.

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

### 3. Update test fixtures — `ArtifactPostingContext` locations

Update all test `ArtifactPostingContext` constructions to use `expires_at: Some(Tick(...))` with values consistent with the profile defaults and the test's `current_tick`. Affected files:
- `candidate_generation.rs` (3 test locations)
- `goal_dispatch_decl.rs` (3 test locations)
- `ranking.rs` (8 test locations)
- `feasibility.rs` (2 test locations)
- `goal_policy.rs` (1 test location)

For test helpers that construct representative goals without a tick context (e.g., `representative_goal_for` in `goal_dispatch_decl.rs`), use `expires_at: Some(Tick(48))` as a representative value matching the default `threat_warning_ttl`.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — 2 runtime + 3 test)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — 3 test)
- `crates/worldwake-ai/src/ranking.rs` (modify — 8 test)
- `crates/worldwake-ai/src/feasibility.rs` (modify — 2 test)
- `crates/worldwake-ai/src/goal_policy.rs` (modify — 1 test)

## Out of Scope

- `BelievedArtifactState` locations — these represent observation-side state, not posting-side construction
- `plan_revalidation.rs` — its `expires_at: None` is on `PostBountyActionPayload`, not `ArtifactPostingContext`
- CLI scenario support (ticket 004)
- Golden test (ticket 005)
- Artifact lifecycle system changes (none needed)

## Acceptance Criteria

### Tests That Must Pass

1. `emit_notice_posting_candidates` produces candidates with `expires_at: Some(Tick(current_tick + 48))` (default threat_warning_ttl)
2. `emit_bounty_posting_candidates` produces candidates with `expires_at: Some(Tick(current_tick + 144))` (default bounty_ttl)
3. All updated test fixtures compile and pass
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No runtime `ArtifactPostingContext` construction in candidate_generation.rs uses `expires_at: None` after this ticket
2. Belief-only planning preserved — TTL read via `GoalBeliefView`, not authoritative state
3. `BelievedArtifactState` fixtures remain unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — modify existing emission tests to assert `expires_at` is `Some`
2. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — update `representative_goal_for` fixtures
3. `crates/worldwake-ai/src/ranking.rs` — update test fixture constructions
4. `crates/worldwake-ai/src/feasibility.rs` — update test fixture constructions
5. `crates/worldwake-ai/src/goal_policy.rs` — update test fixture constructions

### Commands

1. `cargo test -p worldwake-ai -- candidate_generation`
2. `cargo test -p worldwake-ai -- goal_dispatch`
3. `cargo test -p worldwake-ai -- ranking`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`
