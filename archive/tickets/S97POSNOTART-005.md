# S97POSNOTART-005: Golden test — artifact expiry bounds entity count

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — test only
**Deps**: archive/tickets/S97POSNOTART-001.md, archive/tickets/S97POSNOTART-002.md, archive/tickets/S97POSNOTART-003.md, archive/tickets/S97POSNOTART-004.md

## Problem

No existing test verifies the end-to-end loop: agent posts notice with `expires_at` → lifecycle system expires it → agent re-posts. Without this test, the TTL provisioning could silently break if any link in the chain regresses.

## Assumption Reassessment (2026-04-12)

1. `crates/worldwake-ai/tests/golden_integration.rs` already owns the strongest existing artifact lifecycle and autonomous posting coverage for this spec family (`S45` bounty expiry, `S45` notice route effects, `S51` autonomous bounty posting, `S58` autonomous notice posting). `golden_planner_pathology.rs` proves repeated posting pressure, but not authoritative artifact lifecycle bounds.
2. `artifact_lifecycle_system` at `crates/worldwake-systems/src/artifact_lifecycle.rs:4` transitions `Active` artifacts with `expires_at <= current_tick` to `Expired`. Confirmed: the system reads `ArtifactHeader.expires_at` and sets `ArtifactState::Expired`.
3. `UtilityProfile.notice_posting_weight` at `crates/worldwake-core/src/utility_profile.rs:21` is type `Permille`. The golden test scenario needs this set high to keep the autonomous warning path live through repeated re-posting.
4. Agents that need to observe post-production output require `PerceptionProfile` — this is a known golden test requirement per CLAUDE.md.
5. Existing golden tests in `golden_integration.rs` already use the right helpers for this ticket: scenario setup, tick advancement, action-trace assertions, and authoritative social-artifact inspection.

## Architecture Check

1. End-to-end golden test is the right verification layer for cross-system emergent behavior (posting system + lifecycle system + planner re-posting). Unit tests in ticket 003 verify the candidate generation change in isolation; this test verifies the composed behavior.
2. No backward-compatibility shims — pure test addition.

## Verification Layers

1. Agent autonomously commits repeated `post_notice` actions with non-`None` `expires_at` payloads → action trace plus authoritative artifact-header inspection
2. Earlier notices transition to `ArtifactState::Expired` on the authoritative lifecycle schedule → authoritative world state inspection
3. Active notice population at the posting place stays bounded over time because expired notices leave the active set → authoritative world state count assertion sampled across the run
4. Multi-layer ticket: candidate generation (AI) → action execution (systems) → artifact lifecycle (systems). The golden test spans all three, which is appropriate for E2E coverage.

## What to Change

### 1. Add golden test

In `crates/worldwake-ai/tests/golden_integration.rs`, add a new scenario in the existing social-artifact block near the S45/S51/S58 tests:

**Setup**:
- Single AI issuer at Market with non-zero `notice_posting_weight`
- Explicit short `ArtifactPostingProfile` override so the threat-warning TTL is short enough to prove multiple post→expire→re-post loops quickly
- Persistent hostile belief at Warned Road so the same lawful warning keeps regenerating
- Existing S45 notice topology and perception helpers, with authoritative artifact-state sampling over the run

**Assertions**:
- Agent posts at least 2 ThreatWarning notices (enough ticks for re-posting)
- Every created notice has `expires_at: Some(...)`
- Earlier notices become `ArtifactState::Expired`
- Active notice count at the posting place never exceeds the lawful ceiling implied by the short TTL and the live tick ordering
- The run proves the same notice family can reappear after prior instances expire, rather than accumulating only-active artifacts forever

**Emergence justification**: Tests the cross-system interaction between goal-driven posting (AI crate), artifact creation (systems crate), and artifact expiry (systems crate). The interplay determines the artifact population trajectory.

**Why not duplicate**: Existing artifact lifecycle unit tests verify expiry in isolation, and `golden_planner_pathology.rs` already proves pressure-driven repeated posting. This ticket closes the missing composed proof: autonomous notice issuance plus authoritative expiry plus bounded active artifact population in the owning integration suite.

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify — add scenario/test)
- `tickets/S97POSNOTART-005.md` (modify — reassessment correction and closeout)

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

1. `crates/worldwake-ai/tests/golden_integration.rs` — new golden scenario verifying TTL provisioning, authoritative expiry, and bounded active notice population

### Commands

1. `cargo test -p worldwake-ai golden_s97_autonomous_notice_expiry_bounds_active_population`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
5. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

- Completion date: 2026-04-12
- Added `golden_s97_autonomous_notice_expiry_bounds_active_population` to `crates/worldwake-ai/tests/golden_integration.rs` with a short `ArtifactPostingProfile` override on the existing S45 notice topology.
- The new golden proves repeated autonomous `post_notice` commits, non-`None` notice expiry ticks, authoritative transition of earlier notices to `ArtifactState::Expired`, and a bounded active notice population at the posting place.
- Scenario metadata regeneration updated the generated golden inventory/docs under `docs/generated/`.

## Deviations

1. Reassessment corrected the owning suite from `golden_planner_pathology.rs` to `golden_integration.rs`. The pathology suite already proves repeated posting pressure, but the strongest existing home for composed artifact lifecycle + autonomous posting proof is the integration suite.
2. The active-count bound is not enforced by duplicate suppression. The live code allows repeated notice posting; the bound proven here comes from TTL-driven expiry plus the current authoritative tick ordering.

## Verification Result

1. `cargo test -p worldwake-ai golden_s97_autonomous_notice_expiry_bounds_active_population`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
5. `python3 scripts/golden_inventory.py --write --check-docs`
