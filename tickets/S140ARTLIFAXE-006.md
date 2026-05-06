# S140ARTLIFAXE-006: Golden artifact lifecycle E2E + no-shim regression guard

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No new engine code. Adds `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` (5 scenarios), supporting `.ron` scenario files, and `scripts/check_no_artifact_state.sh` regression grep guard wired into `scripts/verify.sh`.
**Deps**: archive/tickets/S140ARTLIFAXE-001.md, archive/tickets/S140ARTLIFAXE-002.md, S140ARTLIFAXE-003, S140ARTLIFAXE-004, S140ARTLIFAXE-005

## Problem

After 001-005 land, the architecture is in place but no E2E golden proves the contract holds across the action commit → lifecycle cascade → planner gate → decision-trace path. Per the spec's Validation and Falsification section, this ticket lands the 5 golden scenarios that exercise the architecture end-to-end (bounty fulfilled, warrant revoked, expired-but-still-posted bounty, suspended legal effect, false rumor refuted), plus the `\bArtifactState\b` no-shim regression grep guard. The grep guard exists as a CI-side check parallel to `scripts/check_active_goal_removed.sh`; without it, future tickets could silently reintroduce the symbol.

## Assumption Reassessment (2026-05-06)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. After 001-005 land: `ArtifactHeader` carries 5 axis fields, `EventTag::ArtifactTransition` is emitted, `artifact_lifecycle_system` runs 5 ordered stages with event-driven cross-axis cascades, `Discrepancy::ArtifactNotActionable` is recorded at the planner gate, decision-trace renders axis values, and the observer renders Section 11. The only remaining work is E2E golden coverage and the symbol-absence regression guard. Existing golden infrastructure: `crates/worldwake-ai/tests/golden_*.rs` files (per `cargo test -p worldwake-ai -- --list`); `scripts/check_active_goal_removed.sh` precedent at the workspace root; `scripts/verify.sh` already invokes precedent guards in fixed order.
2. Spec Validation section names the 5 scenarios. The grep guard is named in spec D-section "Validation and Falsification" → "No-shim regression". Per `docs/golden-e2e-testing.md`, golden tests assert end-to-end behavior with action-trace and event-log delta surfaces; they are the canonical E2E proof surface.
3. **Cross-system shared abstraction boundary**: The boundary under audit is the full action-commit → lifecycle-cascade → planner-gate chain. Each golden scenario exercises a distinct slice of that chain.
5. **Live `GoalKind` under test**: `GoalKind::FulfillBounty` (for bounty-fulfilled and revoked-warrant scenarios; the warrant scenario uses notice-flavored institutional artifacts that participate in the same axis lifecycle). Operator surface: the existing claim_bounty/withdraw_bounty action paths post-002. Affordances: post-001 reads of `actionability` axis.
6. **AI-regression layer**: Golden E2E coverage. Each scenario uses the full `agent_tick` integration harness (full action registries required for the action-commit handlers), not the local needs-only harness.
7. **Ordering layer**: Action lifecycle ordering (action commit → lifecycle cascade in same tick). Event-log delta ordering proves the cascade. Per `docs/precision-rules.md` Rule 4, the contract is action lifecycle + event-log ordering; assertions name `(tick, sequence_in_tick)` keys for the cascade pair.
8. **Scenario isolation**: Each golden scenario isolates one branch of the spec contract. Lawful competing affordances are excluded by scenario design (e.g., the bounty-fulfilled scenario does not include other agents that could withdraw or contest the bounty mid-flight).
12. **Scenario isolation choices**: The 5 scenarios are intended to prove the 5 axis transitions independently. Lawful competing branches (e.g., a fulfillment that races with revocation) are intentionally excluded from setup; those compositions are out of scope and would require a 6th E2E golden if they ever became part of the contract.
15. **Cumulative arithmetic**: `expires_at` for the expired-but-still-posted bounty scenario is reachable under the existing `ArtifactPostingProfile.bounty_ttl: 144` default (S97); the scenario seeds an artifact with explicit `expires_at` and runs forward to verify the TTL transition emits the correct cascade.

## Architecture Check

1. Golden E2E goldens are the canonical proof surface for spec sign-off (per `docs/golden-e2e-testing.md`). They consume the full architecture rather than mock surfaces.
2. The grep guard at `scripts/check_no_artifact_state.sh` parallels the existing `scripts/check_active_goal_removed.sh` pattern. Wiring it into `scripts/verify.sh` makes the check part of the CI gate; no special-case behavior.
3. The grep guard scope is `\bArtifactState\b` (word boundary) so `BelievedArtifactState` (which 001 may have renamed away from, but the guard is conservatively scoped) is unaffected.

## Verification Layers

1. Bounty fulfilled scenario → action trace (commit_claim_bounty), event-log delta (legal_effect Fulfilled + actionability Closed cascade in same tick, treasury reward release per S125), authoritative state (post-tick `ArtifactHeader` axis values).
2. Warrant revoked scenario → event-log delta (legal_effect Revoked + actionability Closed cascade), planner refusal via decision trace (Discrepancy::ArtifactNotActionable at next planner tick).
3. Expired-but-still-posted bounty scenario → event-log delta (legal_effect Expired emitted by lifecycle_system, actionability Closed cascade, visibility Posted preserved).
4. Suspended legal effect scenario → event-log delta (legal_effect Active → Suspended, restoration on resolution event).
5. False rumor refuted scenario → event-log delta (credibility Credible → Refuted from evidence-against event, actionability Closed via stage-5 cascade observing the credibility transition).
6. No-shim regression guard → CI command output (`scripts/check_no_artifact_state.sh` exit code 0 + zero matches reported).

## What to Change

### 1. Create `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs`

Land 5 golden test functions following the existing `golden_*.rs` conventions:

- `bounty_fulfilled_emits_legal_effect_and_actionability_cascade`
- `warrant_revoked_blocks_subsequent_planner_emission`
- `expired_bounty_retains_posted_visibility_with_closed_actionability`
- `suspended_legal_effect_restores_on_resolution_event`
- `refuted_false_rumor_cascades_to_closed_actionability_via_credibility_handler`

Each test loads or constructs a scenario, runs the simulation forward enough ticks to exercise the contract, and asserts on event-log deltas, action traces, and authoritative state per its Verification Layer mapping.

### 2. Add supporting `.ron` scenario fixtures (if needed)

If any of the 5 scenarios are best authored declaratively, add minimal `.ron` files under `scenarios/` (e.g., `scenarios/golden-artifact-lifecycle-bounty-fulfilled.ron`). For tests that require non-default axis state (suspended, refuted), use the unified `ArtifactDef` from S140ARTLIFAXE-004 with explicit axis fields. Tests that can be expressed programmatically (constructing artifacts inline) skip this step.

### 3. Add `scripts/check_no_artifact_state.sh`

Model after `scripts/check_active_goal_removed.sh`. Run a workspace-wide grep `grep -rn "\bArtifactState\b" crates/` and exit non-zero if any matches are returned. Make the script executable (`chmod +x`).

### 4. Wire the guard into `scripts/verify.sh`

Add the line `bash scripts/check_no_artifact_state.sh` to `scripts/verify.sh` parallel to the existing `bash scripts/check_active_goal_removed.sh` invocation. Place it in fixed order so the verify gate fails early if a regression is introduced.

### 5. Regenerate golden inventory documentation

Per `tickets/README.md`, run `python3 scripts/golden_inventory.py --write --check-docs` to update `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/` with the new test names and scenarios.

## Files to Touch

- `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` (new — 5 golden tests)
- Likely: `scenarios/golden-artifact-lifecycle-*.ron` (new — scenario fixtures, count discovered during implementation; some scenarios may stay programmatic)
- `scripts/check_no_artifact_state.sh` (new — grep guard, executable)
- `scripts/verify.sh` (modify — wire guard invocation in fixed order)
- `docs/generated/golden-e2e-inventory.md` (regenerate via `scripts/golden_inventory.py`)
- `docs/generated/golden-scenario-index.md` (regenerate)
- `docs/generated/golden-scenario-details/` (regenerate — per-file detail)

## Out of Scope

- Engine code changes for new lifecycle triggers — covered by 002 (5-stage handler is the substrate; this ticket exercises it).
- Observer rendering correctness — covered by 005's render tests (this ticket's goldens may incidentally touch observer output but do not extend Section 11 logic).
- Composed scenarios (e.g., race between fulfillment and revocation) — explicit Non-Goal in scenario isolation per assumption-reassessment item 12.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_artifact_lifecycle` — all 5 golden tests pass.
2. `bash scripts/check_no_artifact_state.sh` — exits 0; zero matches in `crates/`.
3. `scripts/verify.sh` — full CI gate passes (includes the new guard).
4. `python3 scripts/golden_inventory.py --check-docs` — confirms generated docs are in sync with the new tests.
5. Existing suite: `cargo test --workspace`.

### Invariants

1. The exact symbol `\bArtifactState\b` does not appear anywhere under `crates/`. Enforced by the grep guard.
2. Each of the 5 golden scenarios proves a distinct axis transition or cascade pattern; no two goldens duplicate the same contract.
3. `scripts/verify.sh` invokes `scripts/check_no_artifact_state.sh` in a deterministic order parallel to the existing `check_active_goal_removed.sh` invocation.
4. Generated docs (`docs/generated/golden-e2e-inventory.md` etc.) list the 5 new test names.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_artifact_lifecycle.rs` (new) — 5 golden tests per Step 1 above.

### Commands

1. `cargo test -p worldwake-ai --test golden_artifact_lifecycle`
2. `bash scripts/check_no_artifact_state.sh`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test --workspace`
5. `scripts/verify.sh` (full CI gate; the new guard runs in order)
