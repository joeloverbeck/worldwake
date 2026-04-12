# S97POSNOTART-005: Golden test — artifact expiry bounds entity count

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — test only
**Deps**: archive/tickets/S97POSNOTART-001.md, S97POSNOTART-002, S97POSNOTART-003, S97POSNOTART-004

## Problem

No existing test verifies the end-to-end loop: agent posts notice with `expires_at` → lifecycle system expires it → agent re-posts. Without this test, the TTL provisioning could silently break if any link in the chain regresses.

## Assumption Reassessment (2026-04-12)

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs` exists (32KB). This is the appropriate location for pathological behavior tests like unbounded artifact accumulation.
2. `artifact_lifecycle_system` at `crates/worldwake-systems/src/artifact_lifecycle.rs:4` transitions `Active` artifacts with `expires_at <= current_tick` to `Expired`. Confirmed: the system reads `ArtifactHeader.expires_at` and sets `ArtifactState::Expired`.
3. `UtilityProfile.notice_posting_weight` at `crates/worldwake-core/src/utility_profile.rs:21` is type `Permille`. The golden test scenario needs this set high (e.g., 900) to ensure the agent prioritizes notice posting.
4. Agents that need to observe post-production output require `PerceptionProfile` — this is a known golden test requirement per CLAUDE.md.
5. Existing golden tests in `golden_planner_pathology.rs` and `golden_integration.rs` use established patterns for scenario setup, tick advancement, and state assertion.

## Architecture Check

1. End-to-end golden test is the right verification layer for cross-system emergent behavior (posting system + lifecycle system + planner re-posting). Unit tests in ticket 003 verify the candidate generation change in isolation; this test verifies the composed behavior.
2. No backward-compatibility shims — pure test addition.

## Verification Layers

1. Agent posts notices with non-None `expires_at` → event-log delta (artifact creation events have `expires_at` set)
2. Expired artifacts transition to `ArtifactState::Expired` → authoritative world state inspection
3. Active artifact count at location is bounded → authoritative world state count assertion
4. Multi-layer ticket: candidate generation (AI) → action execution (systems) → artifact lifecycle (systems). The golden test spans all three, which is appropriate for E2E coverage.

## What to Change

### 1. Add golden test

In `crates/worldwake-ai/tests/golden_planner_pathology.rs`, add a test:

**Setup**:
- Single agent with `notice_posting_weight: Permille(900)` and `ArtifactPostingProfile { threat_warning_ttl: 12, office_vacancy_ttl: 96, bounty_ttl: 144 }`
- Agent at a location with a persistent threat belief (believed dangerous entity at a reachable place)
- `PerceptionProfile` on the agent so it can observe posted artifacts
- Run for 100 ticks

**Assertions**:
- Agent posts at least 2 ThreatWarning notices (enough ticks for re-posting)
- All posted notices have `expires_at` set (not `None`)
- After `threat_warning_ttl` (12) ticks, earlier notices are `ArtifactState::Expired`
- Total active (non-expired) notice count at the location never exceeds a bounded ceiling (e.g., `ceil(100 / 12) + 1`)

**Emergence justification**: Tests the cross-system interaction between goal-driven posting (AI crate), artifact creation (systems crate), and artifact expiry (systems crate). The interplay determines the artifact population trajectory.

**Why not duplicate**: Existing artifact lifecycle tests verify the expiry mechanism in isolation. This test verifies the planner provides `expires_at` values and the end-to-end post→expire→re-post loop works.

## Files to Touch

- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify — add test)

## Out of Scope

- Modifying artifact lifecycle system (already correct)
- Perception throttling (deferred per spec non-goals)
- Testing OfficeVacancy or Bounty TTL specifically (the mechanism is identical; ThreatWarning exercises the code path)

## Acceptance Criteria

### Tests That Must Pass

1. Golden test: agent posts notices with `expires_at: Some(...)`, notices expire after TTL, active count is bounded
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No posted artifact has `expires_at: None` when the agent has an `ArtifactPostingProfile`
2. Active artifact count at a location is bounded over time (not monotonically increasing)
3. FND-11 satisfied: artifact accumulation loop is dampened by expiry

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_planner_pathology.rs` — new golden test verifying TTL provisioning and artifact population bounding

### Commands

1. `cargo test -p worldwake-ai -- golden_planner_pathology`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
