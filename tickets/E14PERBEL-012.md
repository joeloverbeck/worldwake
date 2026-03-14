# E14PERBEL-012: Align Corpse Commodity Acquisition Evidence With Believed Inventory

**Status**: PENDING
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

1. This is not the same issue as `E14PERBEL-011`. Passive local observation concerns how beliefs are acquired; this ticket concerns how already-acquired believed corpse inventory is consumed by `AcquireCommodity` candidate generation.
2. `archive/tickets/E14PERBEL-010.md` confirmed that corpse-loot goal emission was already fixed by making `corpse_has_known_loot()` fall back to believed corpse commodity quantities.
3. The remaining helper `corpse_contains_commodity()` in `crates/worldwake-ai/src/candidate_generation.rs` still checks only corpse possession structure, so the acquisition path remains narrower than the loot-goal path.
4. This mismatch is real current code, not a speculative future concern:
   - `acquisition_path_evidence()` inspects local corpses as commodity sources
   - `corpse_contains_commodity()` is the gating helper for that path
   - there is no matching fallback to `commodity_quantity(corpse, commodity)`
5. No active ticket in `tickets/` currently owns this exact seam. `E14PERBEL-011` is adjacent but not sufficient.

## Architecture Check

1. The cleanest fix is to make corpse commodity evidence use the same subjective sufficiency rule already accepted for `LootCorpse`: if the agent lawfully believes the corpse contains a commodity, the corpse can count as a candidate evidence source.
2. This is better than preserving the current asymmetry because it removes one more place where planning semantics depend on the shape of observed possession structure rather than on the believed state the AI is supposed to reason from.
3. The fix should stay inside the AI read/model layer. Do not add planner-side authoritative peeking or a compatibility alias to expose hidden corpse possessions.
4. The more robust long-term shape is to centralize corpse commodity knowledge rules so `LootCorpse` and `AcquireCommodity` do not drift again. This ticket should move in that direction with minimal scope.
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
- any shared helper introduced stays aligned with the existing corpse-loot semantics

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify if an integration-style belief test is the clearest place to prove the behavior)
- `tickets/E14PERBEL-012.md` (new)

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
3. `crates/worldwake-ai/src/agent_tick.rs` — add an integration-style belief test only if unit coverage alone cannot prove the end-to-end candidate effect cleanly.
   Rationale: keeps the test surface minimal unless runtime integration is actually needed.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

