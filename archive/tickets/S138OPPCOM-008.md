# S138OPPCOM-008: Interrupt-layer opportunity enrichment

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: No new production code in this ticket — `archive/tickets/S138OPPCOM-006.md` already routes opportunity-derived candidates into `OrderedRanked`; this ticket adds focused interrupt-layer regression proof for that live path
**Deps**: archive/tickets/S138OPPCOM-006.md (Opportunity / PerceivedOpportunityIndex populated per-tick)

## Problem

S138 enriches the existing interrupt layer (`crates/worldwake-ai/src/interrupts.rs:31`, `evaluate_interrupt`) with opportunity-derived candidates so that a panicked agent who sees a corpse, an unattended valuable, or a wounded ally generates interrupt-eligible candidates through the same gate as today's `ranked_candidates`. The interrupt fires when the opportunity's expected-motive-satisfaction clears the active commitment's expected-motive-satisfaction by the existing relative `Permille` `frame_switch_margin` / switch-margin rule. No new interrupt channel is introduced; opportunities populate the existing fire-or-not gate's input set.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage: `crates/worldwake-ai/src/interrupts.rs` has 10+ inline tests starting at line 405 (lines 405, 431, 462, 488, 518, 539, 571, 603, 642, 744). The tests exercise the existing `ranked_candidates`-based fire/no-fire decision. Adding opportunity-derived candidates to the input set must not regress these tests at default (empty) opportunity index.
2. Spec/doc reference: `archive/specs/S138-opportunity-compiler.md` deliverable section "Interrupt-layer enrichment (in `interrupts.rs`)".
3. Shared abstraction boundary: `evaluate_interrupt` already consumes `ranked_candidates: &OrderedRanked<'_>` and per-agent budget margins. Opportunity-derived candidates flow through the same `OrderedRanked` surface — no new fire pathway, no new commitment-loss semantics.
4. AI-regression layer: this is a runtime `agent_tick` change (interrupt evaluation). Existing harness is sufficient — `agent_tick/tests.rs` has integration coverage for the surrounding flow.
5. Heuristic-removal discipline (precision-rules.md §12): this ticket does NOT remove or weaken the existing fire-or-not gate. It extends the gate's input set. The existing tests at empty-opportunity-set default should pass unchanged.
6. Live margin correction: `frame_switch_margin` / `switch_margin` are `Permille` relative margins in `goal_switching::compare_goal_switch`, not absolute score deltas. A current score of 500 with `Permille(100)` requires a challenger score of 550, not 600. Acceptance examples below use the live relative-margin contract.

## Architecture Check

1. Opportunity-derived candidates ride through `OrderedRanked` (the same surface emitter-derived candidates use) — no parallel ranking path, no FND-28 double-truth.
2. The interrupt fires under the same `frame_switch_margin` rule already in use — opportunity-derived candidates are not given priority; they compete on the same motive-satisfaction comparison.
3. FND-21 revisable commitments preserved: the existing "monitor assumptions and revise plans when assumptions break" architecture is enriched, not replaced.
4. No new event tag, no new control source, no new commitment artifact — minimal architectural footprint.

## Verification Layers

1. Empty opportunity index: behavior identical to pre-S138 — covered by the existing `interrupts` module regression suite and by the default empty-opportunity wrapper path.
2. Opportunity-derived challenger clears the relative `frame_switch_margin`: focused unit test proves a same-class anchored opportunity candidate at score 550 interrupts an active score 500 with `Permille(100)`.
3. Marginal opportunity below the relative `frame_switch_margin`: focused unit test proves score 549 does not interrupt the same active score 500 with `Permille(100)`.
4. Per-agent profile effect: upstream profile/ranking differences are represented to the interrupt layer only as ranked motive-score differences; focused unit test proves score 510 vs 800 on the same opportunity shape produces no-interrupt vs interrupt without a new interrupt channel.
5. Interrupt-trigger attribution uses the already-landed `RootCandidateTrace.source` path from `archive/tickets/S138OPPCOM-006.md`; no new `InterruptTrigger` payload or event tag was added.

## What to Change

### 1. Extend `evaluate_interrupt` input surface

Modify `crates/worldwake-ai/src/interrupts.rs:31`:

`archive/tickets/S138OPPCOM-006.md` already routes opportunity-derived candidates through the same `OrderedRanked` produced by ranking. No signature change is needed here; the landed proof keeps the gate signature stable and adds anchored opportunity-shaped regression cases in the existing inline test module.

### 2. Surface opportunity-vs-commitment comparison

Inside the function, when computing the best-challenger comparison, opportunity-derived candidates participate identically. The fire decision uses the existing `compare_goal_switch` relative `Permille` margin arithmetic. No new branch is introduced; the comparison's input set is larger because candidate generation now contributes opportunity-derived `GoalOffer`s to the unified ranked list.

### 3. Attribution in interrupt trigger

When the interrupt fires with an opportunity-derived best-challenger, source attribution remains on the existing planning/root-candidate trace surface as `CandidateSource::OpportunityCompiler`. `InterruptDecision::InterruptForReplan { trigger: InterruptTrigger }` is unchanged; it does not grow a duplicate source payload.

## Files to Touch

- `crates/worldwake-ai/src/interrupts.rs` (modified tests — anchored opportunity-shaped challengers participate in the existing best-challenger comparison)
- No change: `crates/worldwake-ai/src/ranking.rs` — opportunity-derived candidates are already merged into `OrderedRanked` by `archive/tickets/S138OPPCOM-006.md`

## Out of Scope

- New interrupt channels — opportunities ride the existing gate
- Removing/weakening existing fire-or-not heuristics — preserved
- New commitment-loss semantics — preserved (existing `frame_switch_margin` governs)
- New action types — none
- Travel-pruning extension — lands in ticket 007 (independent consumer of `PerceivedOpportunityIndex`)

## Acceptance Criteria

### Tests That Must Pass

1. New test: empty opportunity index produces interrupt decisions identical to today's behavior for the same inputs
2. New test: high-salience opportunity-derived candidate (motive_score 550 or 800, depending on fixture) vs active commitment (motive_score 500) with `frame_switch_margin = Permille(100)` → interrupt fires through the existing `SuperiorSameClassPlan` trigger
3. New test: marginal opportunity (motive_score 549 or 510, depending on fixture) with the same active commitment and margin → interrupt does not fire
4. New test: two upstream ranking outcomes for the same opportunity shape (representing different profile weighting) produce different interrupt decisions without a new interrupt channel
5. Existing 10+ tests at `interrupts.rs:405-744` continue to pass without modification at default empty-opportunity index
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. With `PerceivedOpportunityIndex::default()` (empty), interrupt decisions are byte-identical to pre-S138
2. The `frame_switch_margin` rule governs the fire decision — no new commitment-loss semantics
3. Opportunity-derived triggers are attributable via `CandidateSource::OpportunityCompiler` in the planning/root-candidate decision trace; `InterruptTrigger` remains source-neutral

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/interrupts.rs` (inline `#[cfg(test)]`) — 2 new tests covering anchored opportunity-shaped challengers, margin/no-margin behavior, and upstream score variation

### Commands

1. `cargo test -p worldwake-ai interrupts`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-11.

- Confirmed the live production path from `archive/tickets/S138OPPCOM-006.md`: compiled opportunities become `GoalOffer`s, candidate generation tags them as `CandidateSource::OpportunityCompiler`, ranking places them in the same `OrderedRanked` surface, and `evaluate_interrupt` already consumes that unified ranked list.
- Added focused interrupt regression coverage in `crates/worldwake-ai/src/interrupts.rs` for anchored opportunity-shaped candidates using the existing frame/switch margin gate.
- Corrected the ticket's stale absolute-margin arithmetic to the live relative `Permille` contract.

## Deviations

- No `evaluate_interrupt` signature change landed because the preferred `OrderedRanked` integration path was already live.
- No new `InterruptTrigger` source payload landed. Source attribution remains on the existing root-candidate trace surface as `CandidateSource::OpportunityCompiler`, which avoids duplicating the same fact on a second carrier.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib opportunity_compiler_candidate_uses_existing_frame_switch_margin -- --list` (selector discovery)
- Passed `cargo test -p worldwake-ai --lib opportunity_rank_score_variation_changes_interrupt_decision_without_new_channel -- --list` (selector discovery)
- Passed `cargo test -p worldwake-ai --lib interrupts::tests::opportunity_compiler_candidate_uses_existing_frame_switch_margin -- --exact`
- Passed `cargo test -p worldwake-ai --lib interrupts::tests::opportunity_rank_score_variation_changes_interrupt_decision_without_new_channel -- --exact`
- Passed `cargo test -p worldwake-ai --lib interrupts`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
