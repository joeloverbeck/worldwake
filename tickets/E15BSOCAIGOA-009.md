# E15BSOCAIGOA-009: Golden social tests T8–T10 (autonomous Tell, suppression, information cascade)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — test file only
**Deps**: E15BSOCAIGOA-001 through E15BSOCAIGOA-005 (all Deliverable 1 implementation), E15BSOCAIGOA-006

## Problem

Tier 2 tests T8–T10 require Deliverable 1 (autonomous social AI goals). These tests validate that agents autonomously generate and execute Tell actions, that survival needs suppress social goals, and that information cascades enable cross-system chains (Tell → belief → enterprise → production → travel → trade).

## Assumption Reassessment (2026-03-15)

1. T8 no longer depends on `GoalKind::ShareBelief` or `PlannerOpKind::Tell` landing; those already exist. The real production prerequisites are `emit_social_candidates()` (E15BSOCAIGOA-004) plus non-placeholder ShareBelief motive scoring (E15BSOCAIGOA-005).
2. T9 depends on the already-present suppression logic continuing to hold once autonomous ShareBelief candidates exist. E15BSOCAIGOA-005 should treat suppression coverage as regression protection, not brand-new functionality.
3. T10 is the most complex golden test — exercises Tell → belief update → enterprise candidate generation (restock goal) → production → travel → trade. Requires all systems working in concert.
4. `golden_social.rs` will have T1–T7 from previous tickets.

## Note

This ticket is the first end-to-end proof that the remaining autonomous-social-behavior gap is actually closed. If T8 or T9 cannot pass after E15BSOCAIGOA-004 and E15BSOCAIGOA-005, then the social AI architecture is still incomplete even if the lower-level implementation tickets are nominally done.

## Architecture Check

1. Appends to existing golden_social.rs.
2. T10 is a cross-system integration test — may need extended tick counts (50+ ticks) for full chain completion.
3. No new production code — exercises existing systems through AI-driven autonomous behavior.

## What to Change

### 1. Add T8–T10 to golden_social.rs

**T8: `golden_agent_autonomously_tells_colocated_peer`**
- Setup: 2 agents, low needs (no survival pressure). Agent A has high social_weight (Permille(900)) and fresh DirectObservation beliefs. Agent B has no beliefs. Both have TellProfile.
- Step simulation — NO InputQueue injection. AI must autonomously generate ShareBelief goal.
- Assert: A generates ShareBelief goal → plans Tell → executes → B receives Report belief.
- Checks: Conservation, determinism.

**T9: `golden_survival_needs_suppress_social_goals`**
- Setup: Agent with high social_weight (Permille(900)) and fresh beliefs, but critically hungry. Food available at current location.
- Step simulation autonomously.
- Assert: Agent eats first (ConsumeOwnedCommodity outranks ShareBelief). Verify ShareBelief does not execute before hunger is addressed.
- Checks: Priority ordering (Critical/High > Low), determinism.

**T10: `golden_information_cascade_enables_trade`**
- Setup: 3-place topology (Market, Farm, Crossroads). Merchant at Market with unmet apple demand. Farmer at Farm with apple production capability. Traveler at Farm with knowledge of Market demand (DirectObservation belief about merchant). Farmer has TellProfile.
- Step simulation — Traveler tells Farmer about Market demand → Farmer generates Restock/enterprise goal → produces apples → travels to Market → trade occurs.
- Assert: Full cross-system chain completes. Commodities and coins conserved across entire chain.
- Checks: Conservation (commodities + coins at every tick), determinism. This test proves information transmission enables economic behavior that was impossible without it.

## Files to Touch

- `crates/worldwake-ai/tests/golden_social.rs` (modify — append tests)

## Out of Scope

- Tests T1–T7 (E15BSOCAIGOA-007, E15BSOCAIGOA-008)
- Tests T11–T13 (E15BSOCAIGOA-010)
- Production code changes (all Deliverable 1 done in E15BSOCAIGOA-001 through E15BSOCAIGOA-005)
- Coverage report (E15BSOCAIGOA-010)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_agent_autonomously_tells_colocated_peer` — AI generates ShareBelief without InputQueue injection, executes Tell, belief transfers
2. `golden_survival_needs_suppress_social_goals` — hungry agent eats before gossiping
3. `golden_information_cascade_enables_trade` — full Tell→belief→enterprise→production→travel→trade chain
4. All 3 tests verify determinism
5. T8, T10 verify conservation per tick
6. T9 verifies priority ordering: ConsumeOwnedCommodity action executes before any ShareBelief action
7. Existing suite: `cargo test -p worldwake-ai --test golden_social` — all T1–T10 pass

### Invariants

1. Autonomous Tell requires no InputQueue injection (purely AI-driven)
2. ShareBelief never outranks Critical/High priority survival goals
3. Conservation holds across multi-system chains (Tell + production + travel + trade)
4. Deterministic replay produces identical hashes for all tests

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — 3 new golden E2E tests (T8–T10)

### Commands

1. `cargo test -p worldwake-ai --test golden_social`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
