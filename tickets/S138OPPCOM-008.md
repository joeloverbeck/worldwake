# S138OPPCOM-008: Interrupt-layer opportunity enrichment

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `evaluate_interrupt` ranks opportunity-derived candidates against the active commitment's expected-motive-satisfaction; behavior activates only when the opportunity index is non-empty
**Deps**: archive/tickets/S138OPPCOM-006.md (Opportunity / PerceivedOpportunityIndex populated per-tick)

## Problem

S138 enriches the existing interrupt layer (`crates/worldwake-ai/src/interrupts.rs:31`, `evaluate_interrupt`) with opportunity-derived candidates so that a panicked agent who sees a corpse, an unattended valuable, or a wounded ally generates interrupt-eligible candidates through the same gate as today's `ranked_candidates`. The interrupt fires when the opportunity's expected-motive-satisfaction exceeds the active commitment's expected-motive-satisfaction by at least the existing `frame_switch_margin`. No new interrupt channel is introduced; opportunities populate the existing fire-or-not gate's input set.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage: `crates/worldwake-ai/src/interrupts.rs` has 10+ inline tests starting at line 405 (lines 405, 431, 462, 488, 518, 539, 571, 603, 642, 744). The tests exercise the existing `ranked_candidates`-based fire/no-fire decision. Adding opportunity-derived candidates to the input set must not regress these tests at default (empty) opportunity index.
2. Spec/doc reference: `specs/S138-opportunity-compiler.md` deliverable section "Interrupt-layer enrichment (in `interrupts.rs`)".
3. Shared abstraction boundary: `evaluate_interrupt` already consumes `ranked_candidates: &OrderedRanked<'_>` and per-agent budget margins. Opportunity-derived candidates flow through the same `OrderedRanked` surface — no new fire pathway, no new commitment-loss semantics.
4. AI-regression layer: this is a runtime `agent_tick` change (interrupt evaluation). Existing harness is sufficient — `agent_tick/tests.rs` has integration coverage for the surrounding flow.
5. Heuristic-removal discipline (precision-rules.md §12): this ticket does NOT remove or weaken the existing fire-or-not gate. It extends the gate's input set. The existing tests at empty-opportunity-set default should pass unchanged.

## Architecture Check

1. Opportunity-derived candidates ride through `OrderedRanked` (the same surface emitter-derived candidates use) — no parallel ranking path, no FND-28 double-truth.
2. The interrupt fires under the same `frame_switch_margin` rule already in use — opportunity-derived candidates are not given priority; they compete on the same motive-satisfaction comparison.
3. FND-21 revisable commitments preserved: the existing "monitor assumptions and revise plans when assumptions break" architecture is enriched, not replaced.
4. No new event tag, no new control source, no new commitment artifact — minimal architectural footprint.

## Verification Layers

1. Empty opportunity index: behavior identical to pre-S138 — focused unit test (regression guard against existing 10+ tests)
2. High-salience opportunity exceeds active commitment's motive-satisfaction by `frame_switch_margin`: interrupt fires with the opportunity as the trigger — focused unit test
3. Marginal opportunity (below `frame_switch_margin` advantage): interrupt does not fire — focused unit test
4. Per-agent profile effect: two agents with different `RiskWeightProfile` see different opportunity rankings, producing different interrupt decisions on the same perceived entity — focused unit test
5. Interrupt-trigger attribution surfaces opportunity-derived triggers in the decision trace (via `RootCandidateTrace.source` from ticket 001) — runtime trace coverage

## What to Change

### 1. Extend `evaluate_interrupt` input surface

Modify `crates/worldwake-ai/src/interrupts.rs:31`:

Either (a) accept an additional `opportunity_candidates: &[GoalOffer]` parameter, OR (b) ensure `ranked_candidates: &OrderedRanked<'_>` already includes opportunity-derived candidates as part of the unified ranking. The latter is preferred — it keeps the gate signature stable. If archive/tickets/S138OPPCOM-006.md routes opportunity-derived candidates through the same `OrderedRanked` produced by ranking, no signature change is needed here. Confirm during implementation.

### 2. Surface opportunity-vs-commitment comparison

Inside the function, when computing the best-challenger comparison (line 61 area in current code), opportunity-derived candidates participate identically — the expected-motive-satisfaction comparison uses the same arithmetic. The fire decision compares `best_challenger.motive_score >= active_commitment.motive_score + frame_switch_margin`. No new branch is introduced; the comparison's input set is larger.

### 3. Attribution in interrupt trigger

When the interrupt fires with an opportunity-derived best-challenger, the `InterruptTrigger` payload (or sibling trace surface) names the trigger's source as `CandidateSource::OpportunityCompiler` (field from ticket 001). Inspect `InterruptDecision::InterruptForReplan { trigger: InterruptTrigger }` to confirm where the source-of-trigger should be recorded.

## Files to Touch

- `crates/worldwake-ai/src/interrupts.rs` (modify — best-challenger comparison reads opportunity-derived candidates; trigger source attribution)
- Likely: `crates/worldwake-ai/src/ranking.rs` — confirm whether opportunity-derived candidates are already merged into `OrderedRanked` by archive/tickets/S138OPPCOM-006.md or need explicit handling here

## Out of Scope

- New interrupt channels — opportunities ride the existing gate
- Removing/weakening existing fire-or-not heuristics — preserved
- New commitment-loss semantics — preserved (existing `frame_switch_margin` governs)
- New action types — none
- Travel-pruning extension — lands in ticket 007 (independent consumer of `PerceivedOpportunityIndex`)

## Acceptance Criteria

### Tests That Must Pass

1. New test: empty opportunity index produces interrupt decisions identical to today's behavior for the same inputs
2. New test: high-salience opportunity-derived candidate (motive_score 800) vs active commitment (motive_score 500) with `frame_switch_margin = 100` → interrupt fires, trigger source = `OpportunityCompiler`
3. New test: marginal opportunity (motive_score 510) with same active commitment and margin → interrupt does not fire
4. New test: two agents with different `RiskWeightProfile.theft_aversion` rank the same opportunity differently → different interrupt decisions
5. Existing 10+ tests at `interrupts.rs:405-744` continue to pass without modification at default empty-opportunity index
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. With `PerceivedOpportunityIndex::default()` (empty), interrupt decisions are byte-identical to pre-S138
2. The `frame_switch_margin` rule governs the fire decision — no new commitment-loss semantics
3. Opportunity-derived triggers are attributable via `CandidateSource::OpportunityCompiler` in the decision trace

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/interrupts.rs` (inline `#[cfg(test)]`) — 4 new tests per Acceptance Criteria

### Commands

1. `cargo test -p worldwake-ai interrupts`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
