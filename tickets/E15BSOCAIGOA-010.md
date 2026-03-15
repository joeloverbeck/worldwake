# E15BSOCAIGOA-010: Golden social tests T11–T13 and coverage report update

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — tests and report only
**Deps**: E15BSOCAIGOA-001 through E15BSOCAIGOA-005 (Deliverable 1), E15BSOCAIGOA-006, E15BSOCAIGOA-009

## Problem

Tests T11–T13 validate chain-length gossip cutoff, agent diversity in social behavior, and the full information lifecycle (rumor → wasted trip → discovery → replan). The coverage report also needs updating to reflect all 13 social tests.

## Assumption Reassessment (2026-03-15)

1. T11 needs emit_social_candidates() chain_len filtering (E15BSOCAIGOA-004). Agent D with max_relay_chain_len=1 should not receive a chain_len=2 rumor from C.
2. T12 tests Principle 20 (agent diversity) via different social_weight values (900, 200, 0).
3. T13 exercises the full information lifecycle: rumor received → travel → arrival → passive observation contradicts rumor → InventoryDiscrepancy Discovery → replan with corrected belief.
4. `reports/golden-e2e-coverage-analysis.md` exists and needs social coverage section.

## Architecture Check

1. Appends 3 final tests to golden_social.rs, completing the 13-test suite.
2. Coverage report is documentation-only — no code impact.
3. T12 may need extended tick simulation (20+ ticks) to verify all 3 agents' behavior patterns.

## What to Change

### 1. Add T11–T13 to golden_social.rs

**T11: `golden_chain_length_filtering_stops_gossip`**
- Setup: 4 agents (A, B, C, D) co-located. A has DirectObservation. B, C have max_relay_chain_len=3. D has max_relay_chain_len=1.
- Step simulation autonomously.
- Assert: A→B (Report, chain 1). B→C (Rumor, chain 2). C→D blocked (chain would be 3, D's max is 1). D has NO belief about the subject.
- Checks: Determinism, no infinite propagation.

**T12: `golden_agent_diversity_in_social_behavior`**
- Setup: 3 agents: Gossip (social_weight=900), Normal (social_weight=200), Loner (social_weight=0). All have fresh beliefs, low needs, TellProfile.
- Step simulation for extended ticks (20+).
- Assert: Gossip generates ShareBelief goals early and executes Tell. Normal generates ShareBelief eventually. Loner never generates ShareBelief (motive score = 0).
- Checks: Determinism, diversity verification (Principle 20).

**T13: `golden_rumor_leads_to_wasted_trip_then_discovery`**
- Setup: Agent receives Rumor (via autonomous Tell from co-located peer) about apples at Orchard Farm. Orchard actually depleted (Quantity(0)).
- Step simulation autonomously.
- Assert: Agent plans Travel → arrives at Orchard Farm → passive observation → InventoryDiscrepancy Discovery → belief updated from Rumor to DirectObservation of empty state → agent replans.
- Checks: Conservation, determinism, belief source upgrade (Rumor → DirectObservation).

### 2. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Add Part section for social information scenarios (T1–T13)
- Update coverage matrix with Social domain
- Remove backlog item "stale belief → travel to depleted source → re-observation → replan" (now covered by T6)
- Add new GoalKind coverage (ShareBelief)
- Add new ActionDomain coverage note (Social)
- Note deferred: InvestigateMismatch goal as future backlog item

## Files to Touch

- `crates/worldwake-ai/tests/golden_social.rs` (modify — append tests)
- `reports/golden-e2e-coverage-analysis.md` (modify)

## Out of Scope

- Tests T1–T10 (E15BSOCAIGOA-007 through E15BSOCAIGOA-009)
- Production code changes
- GoalKind::InvestigateMismatch (future spec, only mentioned as deferred in report)
- Harness modifications

## Acceptance Criteria

### Tests That Must Pass

1. `golden_chain_length_filtering_stops_gossip` — gossip stops at agent D's chain_len filter
2. `golden_agent_diversity_in_social_behavior` — 3 agents show distinct social behavior based on social_weight
3. `golden_rumor_leads_to_wasted_trip_then_discovery` — full lifecycle: rumor → travel → discovery → corrected belief → replan
4. All 3 tests verify determinism
5. T13 verifies conservation
6. T12 verifies Loner (social_weight=0) never generates ShareBelief across entire simulation
7. Full suite: `cargo test -p worldwake-ai --test golden_social` — all 13 tests pass
8. Full workspace: `cargo test --workspace` — no regressions

### Invariants

1. Chain length filtering prevents infinite gossip propagation
2. social_weight=0 produces zero motive score → no ShareBelief candidates ranked (Principle 20 diversity)
3. Discovery events replace rumor-sourced beliefs with DirectObservation-sourced beliefs
4. Conservation holds across all tick steps
5. Coverage report accurately reflects implemented test state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — 3 new golden E2E tests (T11–T13)
2. `reports/golden-e2e-coverage-analysis.md` — updated coverage documentation

### Commands

1. `cargo test -p worldwake-ai --test golden_social`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
