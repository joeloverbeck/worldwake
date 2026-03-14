# E14PERBEL-012: Align Corpse Commodity Acquisition Evidence With Believed Inventory

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` corpse commodity evidence for `AcquireCommodity` candidate generation and related test coverage
**Deps**: `archive/tickets/E14PERBEL-010.md`, `archive/tickets/E14PERBEL-009.md`, `specs/E14-perception-beliefs.md`, `specs/S04-merchant-selling-market-presence.md`

## Problem

The current AI read model is mostly consistent for corpse-related behavior under subjective beliefs:

- `LootCorpse` goal emission accepts believed corpse commodity state
- `BuryCorpse` goal emission relies on perceived corpse plus local grave-plot evidence
- unseen corpses do not leak into planning

But one narrower seam remains in buyer-side commodity acquisition:

- `AcquireCommodity` path discovery still treats corpses as commodity sources only through `corpse_contains_commodity()`
- `corpse_contains_commodity()` currently checks only `direct_possessions(corpse)` and `item_lot_commodity(...)`
- under `PerAgentBeliefView`, non-self direct possession structure may be unavailable even when the agent lawfully believes the corpse carries a commodity quantity

That means corpse-specific opportunistic loot and corpse-specific commodity acquisition are using different standards of subjective sufficiency. The architecture should not keep those two paths diverged.

## Assumption Reassessment (2026-03-14)

1. This is not the same issue as archived `archive/tickets/completed/E14PERBEL-011-add-passive-local-observation-to-perception-pipeline.md`. Passive local observation governs how beliefs are acquired; this ticket governs how already-acquired believed corpse inventory is consumed by `AcquireCommodity` candidate generation.
2. `archive/tickets/E14PERBEL-010.md` correctly established that corpse-loot goal emission is already belief-local: `corpse_has_known_loot()` falls back from direct possessions to believed corpse commodity quantities.
3. The narrower acquisition-side seam is still present in current code. In `crates/worldwake-ai/src/candidate_generation.rs`, `acquisition_path_evidence()` inspects local corpses as possible commodity sources, but it gates them through `corpse_contains_commodity()`, which only checks direct corpse possessions for a matching lot commodity.
4. There is currently no acquisition-side fallback to `commodity_quantity(corpse, commodity)`, so a corpse can be sufficient evidence for `LootCorpse` while still being invisible to `AcquireCommodity` candidate generation for the same believed commodity.
5. Current unit coverage does not already close this gap. Existing corpse-focused tests in `crates/worldwake-ai/src/candidate_generation.rs` cover:
   - `local_corpse_with_possessions_emits_loot_goal`
   - `local_corpse_with_believed_inventory_emits_loot_goal`
   - `local_corpse_with_grave_plot_emits_bury_goal`
   But there is no matching regression asserting corpse-backed `AcquireCommodity` emission from believed aggregate corpse inventory.
6. No active ticket currently owns this exact seam. Archived `E14PERBEL-010` explicitly called it out as a possible future hardening target, and `E14PERBEL-011` does not subsume it.

## Architecture Check

1. The cleanest fix is to make corpse commodity evidence use the same subjective sufficiency rule already accepted for `LootCorpse`: if the agent lawfully believes the corpse contains a commodity, the corpse can count as a candidate evidence source.
2. This is better than preserving the current asymmetry because it removes one more place where planning semantics depend on the shape of observed possession structure rather than on the believed state the AI is supposed to reason from.
3. The fix should stay inside the AI read/model layer. Do not add planner-side authoritative peeking or a compatibility alias to expose hidden corpse possessions.
4. The more robust long-term shape is to share a corpse-commodity knowledge helper between corpse-loot and corpse-acquisition evidence. This ticket should move in that direction without widening scope into planner binding or perception changes.
5. No backwards-compatibility shims are permitted. The old stricter corpse-source rule should be replaced, not preserved beside the new one.

## What to Change

### 1. Unify corpse commodity evidence semantics

Update corpse commodity acquisition-path evidence so that a corpse counts as a source for `AcquireCommodity` when either of the following is true:

- the corpse's direct possessions are known and include a matching commodity lot
- the agent's belief view reports positive believed quantity for that corpse and commodity

The implementation may:

- extend `corpse_contains_commodity()`, or
- replace it with a clearer shared helper used by both corpse-loot and corpse-acquisition logic

The exact helper shape may vary, but the semantic rule must become consistent across corpse-related AI paths.

### 2. Keep corpse evidence belief-local and deterministic

The updated helper must:

- read through the belief-facing AI boundary already in place
- remain deterministic
- not fabricate exact lot identity when only aggregate corpse commodity belief is known

If only aggregate commodity belief is known, that is sufficient for candidate generation and ranking, but exact action targeting must still rely on the later planner/binding layers.

### 3. Add focused regression coverage

Add tests proving that believed corpse commodity state can drive `AcquireCommodity` candidate generation just as it already drives `LootCorpse` goal emission.

At minimum, cover:

- local corpse with believed commodity quantity emits `AcquireCommodity`
- unseen corpse or corpse without believed matching commodity still does not emit the path
- any shared corpse-commodity helper introduced stays aligned with the existing corpse-loot semantics

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `tickets/E14PERBEL-012.md` (modify, then archive)

## Out of Scope

- Passive local observation (`E14PERBEL-011`)
- Exact target binding and affordance matching (`specs/S03-planner-target-identity-and-affordance-binding.md`)
- Seller-listing/market-presence work from `specs/S04-merchant-selling-market-presence.md`
- Reworking the broader belief-view boundary beyond `E14PERBEL-009`
- Any omniscient planner shortcut

## Acceptance Criteria

### Tests That Must Pass

1. A focused regression test proves believed corpse commodity state can emit `AcquireCommodity` candidate evidence without requiring known corpse possession structure.
2. Existing corpse-loot subjective-belief tests still pass.
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

### Invariants

1. Corpse-related AI candidate generation uses one consistent subjective sufficiency rule for believed corpse commodity state.
2. `AcquireCommodity` does not gain omniscient corpse discovery.
3. Aggregate believed commodity state may justify candidate emission, but exact action targeting still remains the responsibility of later planning/binding layers.
4. No compatibility alias or fallback to deleted omniscient behavior is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — add a regression test where a local corpse has believed quantity for a commodity and emits `AcquireCommodity`.
   Rationale: proves the acquisition path stops depending on hidden corpse possession structure.
2. `crates/worldwake-ai/src/candidate_generation.rs` — add or strengthen a negative test where the corpse is local but has no believed matching commodity quantity and therefore does not support the acquisition path.
   Rationale: preserves belief-local gating and avoids broadening corpse discovery incorrectly.
3. Reuse the existing loot-goal corpse tests in `crates/worldwake-ai/src/candidate_generation.rs` as the alignment guard for any shared helper introduced here.
   Rationale: keeps the hardening local to candidate generation while proving the acquisition-side rule now matches the established loot-side rule.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - corrected the ticket assumptions to match the live code, adjacent archived tickets, and the real current test surface
  - introduced a shared corpse-commodity knowledge helper in `crates/worldwake-ai/src/candidate_generation.rs` so corpse-backed `AcquireCommodity` evidence now accepts either known direct corpse lots or believed aggregate corpse commodity quantities
  - added focused candidate-generation regression tests for positive and negative corpse-backed `AcquireCommodity` emission
- Deviations from the corrected plan:
  - no `agent_tick.rs` integration test was needed because unit coverage in `candidate_generation.rs` proved the seam directly and kept the fix local
  - the hardening stayed within candidate generation instead of expanding into planner binding, perception, or broader belief-boundary refactors
- Verification results:
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
