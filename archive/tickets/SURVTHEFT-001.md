# SURVTHEFT-001: Restore survival-relevant `StealItem` ranking and suppression

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes - worldwake-ai goal dispatch, suppression policy, and ranking/provenance
**Deps**: docs/scenario-roadmap.md row 12 (`survival-theft`)

## Problem

The live AI contract treated `GoalKind::StealItem` as a low-priority social fallback even when theft was the only local survival-relevant food branch. Under self-care stress, theft-family suppression and non-drive ranking left a hungry agent committed to impossible `AcquireCommodity(SelfConsume)` intent instead of admitting a theft-capable local branch.

## Assumption Reassessment (2026-04-24)

1. The live contradiction was inside the AI decision pipeline, not authoritative theft execution. `GoalKind::StealItem` already existed, but its dispatch/ranking contract did not treat survival-relevant theft as a drive-owned self-care branch.
2. The exact shared boundary under audit was the family/policy/ranking contract for `GoalKind::StealItem` across [goal_dispatch_decl.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs), [goal_policy.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs), and [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs).
3. The live `GoalKind` under test remained `StealItem`; the motivating scenario used impossible `AcquireCommodity(SelfConsume)` dominance only as the symptom that exposed the wrong theft-side contract.
4. The truthful invariant for this ticket is narrower than the roadmap row: survival-relevant theft must not be stress-suppressed, and its priority / motive / provenance must derive from the target commodity drive rather than from a generic social-fallback motive.
5. This was a mixed-layer AI ticket, but the strongest honest proof seam is focused `worldwake-ai` coverage at the suppression and ranking layer. A truthful scenario/golden seam still requires additional authored world substrate and is not owned by this completed ticket.
6. The live heuristic removed here was theft-family suppression as applied to `StealItem` plus the stale ranking treatment that ignored the target commodity's drive pressure. No new alias path was added; the canonical theft goal now carries the correct drive-owned policy.
7. Focused verification passed after the repair: `cargo test -p worldwake-ai` and `cargo clippy --workspace --all-targets -- -D warnings`.
8. Mismatch + correction: the original draft incorrectly bundled roadmap-row landing into this ticket. That scenario/golden ownership now belongs to follow-up [SURVTHEFT-002](/home/joeloverbeck/projects/worldwake/archive/tickets/SURVTHEFT-002.md).

## Architecture Check

1. Treating survival-relevant theft as a drive-owned `StealItem` branch is cleaner than teaching `AcquireCommodity(SelfConsume)` a special theft fallback. It preserves one canonical theft path and fixes the actual suppression/ranking contradiction instead of adding a second acquisition identity for the same act.
2. No compatibility shims were introduced. The landed change updates the existing `StealItem` dispatch/policy/ranking path in place.

## Verification Layers

1. `StealItem` is no longer suppressed under self-care stress -> focused `goal_policy` tests in [goal_policy.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs)
2. Survival-relevant theft inherits target-commodity drive priority, motive, and provenance -> focused `ranking` tests in [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
3. The repaired AI contract compiles cleanly across the workspace's test targets -> `cargo clippy --workspace --all-targets -- -D warnings`
4. Scenario-level staged-lot theft, post-theft eating, and concealment witness proof are not claimed here; they are deferred to [SURVTHEFT-002](/home/joeloverbeck/projects/worldwake/archive/tickets/SURVTHEFT-002.md).

## What to Change

### 1. Rebind `StealItem` to the drive-owned theft family

Update the existing `StealItem` dispatch declaration so theft uses the drive provenance family and a theft-family policy that is not stress-suppressed like corpse/social/political opportunism.

### 2. Rank theft from the target commodity drive

Use the target commodity's survival pressure to compute `StealItem` priority, motive, and provenance so the planner can truthfully treat survival theft as a self-care branch.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/goal_policy.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)

## Out of Scope

- Authoring `scenarios/survival-theft.ron`
- Adding `golden_survival_theft.rs` or wiring `.github/workflows/golden-survival.yml`
- Merchant/world-substrate work needed to make row 12 truthful under `docs/FOUNDATIONS.md`

## Acceptance Criteria

### Tests That Must Pass

1. Focused `goal_policy` coverage proves `StealItem` is not stress-suppressed.
2. Focused `ranking` coverage proves survival-relevant theft uses target-commodity drive priority / motive / provenance.
3. Existing suite: `cargo test -p worldwake-ai`
4. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `GoalKind::StealItem` remains the canonical theft goal; acquisition goals do not gain a parallel hidden theft identity.
2. Only the AI-side suppression/ranking repair is claimed complete here; roadmap row 12 remains unlanded until the scenario/golden seam exists.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_policy.rs` - prove `StealItem` is excluded from self-care stress suppression.
2. `crates/worldwake-ai/src/ranking.rs` - prove survival-relevant theft inherits target-commodity drive priority, motive, and provenance.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- 2026-04-24: `GoalKind::StealItem` now uses a drive-owned dispatch policy and is no longer suppressed as a generic opportunistic branch under self-care stress.
- 2026-04-24: Survival-relevant theft now inherits target-commodity drive priority, motive, and provenance in `ranking.rs`.
- 2026-04-24: The broader roadmap-row landing was split out truthfully into [SURVTHEFT-002](/home/joeloverbeck/projects/worldwake/archive/tickets/SURVTHEFT-002.md); row 12 remains `Drafting` until that authored substrate exists.
- 2026-04-24: Verification results: `cargo test -p worldwake-ai` and `cargo clippy --workspace --all-targets -- -D warnings` both passed after the AI-side repair.
- 2026-04-24: Deviation from the original draft: `cargo test --release -p worldwake-ai --test golden_survival_theft survival_theft_proves_concealed_staged_lot_branch -- --ignored --test-threads=1 --nocapture` did not become a truthful closeout seam. The attempted roadmap scenario still lacked a stable staged-lot theft -> post-theft-eat branch, so that work was removed from this ticket and reassigned to `SURVTHEFT-002`.
