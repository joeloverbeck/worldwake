# E14PERBEL-014: Reassess Shared Corpse Belief-Evidence Cleanup Against Current Architecture

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: No additional production or test changes required on 2026-03-14; this ticket records that the intended cleanup target is already satisfied by the current `worldwake-ai` shape
**Deps**: `archive/tickets/E14PERBEL-012.md`, `archive/tickets/E14PERBEL-010.md`, `archive/tickets/E14PERBEL-009.md`, `specs/E14-perception-beliefs.md`, `specs/S03-planner-target-identity-and-affordance-binding.md`

## Problem

This ticket originally assumed that `E14PERBEL-012` fixed the corpse-backed `AcquireCommodity` bug but left a meaningful architecture cleanup still undone.

That assumption is now stale relative to the live code:

- the active corpse sufficiency rule is already centralized around `corpse_has_known_commodity()` in `crates/worldwake-ai/src/candidate_generation.rs`
- `corpse_has_known_loot()` delegates to that helper across all commodity kinds
- `corpse_contains_commodity()` is only a thin wrapper over that same helper, not an independent rule
- corpse-focused tests already cover both believed loot emission and believed corpse-backed acquisition

The remaining question for this ticket is therefore not "extract a new helper module," but "is another extraction materially better than the current architecture?" Based on the current code and tests, the answer is no.

## Assumption Reassessment (2026-03-14)

1. The dependency path in the original ticket was stale. `E14PERBEL-012.md` is already archived under `archive/tickets/`, not active under `tickets/`.
2. Current production code already uses one shared corpse commodity sufficiency rule:
   - `corpse_has_known_loot()` checks `corpse_has_known_commodity()` across `CommodityKind::ALL`
   - `corpse_contains_commodity()` delegates directly to `corpse_has_known_commodity()`
3. The shared helper already implements the correct belief-local rule for the currently active corpse paths:
   - known direct corpse possessions with matching lot commodity count as evidence
   - believed aggregate corpse commodity quantity also counts as evidence
4. The focused regression coverage the original ticket proposed already exists in `crates/worldwake-ai/src/candidate_generation.rs`:
   - `local_corpse_with_believed_inventory_emits_loot_goal`
   - `local_corpse_with_believed_inventory_emits_acquire_commodity`
   - `local_corpse_without_matching_believed_inventory_does_not_emit_acquire_commodity`
5. `cargo test -p worldwake-ai corpse` passes against the current repository state, confirming that the corpse-specific behavior and regressions this ticket wanted to add are already present.
6. No spec requires a separate corpse helper module. `specs/E14-perception-beliefs.md` requires belief-local planning and clean world/belief separation; the current helper arrangement already satisfies that requirement without introducing more file/module surface.

## Architecture Check

1. Introducing another extraction layer is not more beneficial than the current architecture. It would move a tiny, currently coherent helper family into a new module without adding capability, reducing coupling, or improving testability in a meaningful way.
2. The current design is already clean enough for the active scope:
   - the subjective corpse commodity rule is defined once in `corpse_has_known_commodity()`
   - the two public corpse-aware candidate paths route through that rule
   - regression tests already pin the contract
3. A further extraction would only become architecturally justified if corpse evidence semantics start being consumed outside candidate generation by multiple independent modules. That is not the current shape of the codebase.
4. No backward-compatibility aliasing or dual path is warranted. The current code already has the direct shape we want, and there is nothing obsolete here that needs preserving.
5. The broader architectural seam worth watching is not corpse helper placement but the still-broad `GoalBeliefView` / AI read-model surface. That is outside this ticket and should only be revisited when a concrete multi-module pressure appears.

## Scope Correction

This ticket is reduced from an implementation ticket to a verification-and-closure ticket.

### Required work

1. Reassess the ticket assumptions against the live code, specs, and tests.
2. Correct the ticket so it reflects the real current architecture and ownership.
3. Verify the relevant corpse-focused, crate, workspace, and lint commands.
4. Archive the ticket with an accurate `Outcome` section.

### No longer required

1. A new corpse helper module.
2. Additional production edits in `crates/worldwake-ai/src/candidate_generation.rs`.
3. Additional corpse regression tests beyond the ones already present.
4. Any compatibility wrapper, aliasing layer, or omniscient fallback.

## Files To Touch

- `tickets/E14PERBEL-014.md` (modify, then archive)

## Out of Scope

- New behavior changes to corpse candidate semantics beyond what `archive/tickets/E14PERBEL-012.md` already landed
- Planner target binding or exact lot identity work (`specs/S03-planner-target-identity-and-affordance-binding.md`)
- Perception acquisition changes (`E14PERBEL-011`)
- Golden harness cleanup (`E14PERBEL-013`)
- Broad decomposition of `candidate_generation.rs` without a new concrete multi-module need

## Acceptance Criteria

### Verification That Must Pass

1. `cargo test -p worldwake-ai corpse`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Corpse inventory sufficiency under subjective beliefs remains defined by one AI-side rule family, not divergent rule copies.
2. The current helper remains belief-local and deterministic.
3. Aggregate believed commodity state may justify candidate evidence, but exact lot identity is not fabricated.
4. No backwards-compatibility alias or omniscient fallback is introduced.
5. The archived ticket truthfully reflects that the requested cleanup is already satisfied by the current architecture.

## Test Plan

### New/Modified Tests

1. No new or modified tests are required for this closure because the focused corpse regressions the ticket called for already exist in the current codebase.

## Existing Tests Confirming The Behavior

1. `crates/worldwake-ai/src/candidate_generation.rs`
   - `local_corpse_with_believed_inventory_emits_loot_goal`
   - `local_corpse_with_believed_inventory_emits_acquire_commodity`
   - `local_corpse_without_matching_believed_inventory_does_not_emit_acquire_commodity`
2. `crates/worldwake-ai/src/agent_tick.rs`
   - `unseen_death_does_not_create_corpse_reaction_without_reobservation`
3. `crates/worldwake-ai/tests/golden_combat.rs`
   - corpse loot, burial, opportunistic loot, and corpse-related replay coverage

### Commands

1. `cargo test -p worldwake-ai corpse`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - reassessed the ticket against the live code, specs, and tests
  - corrected the ticket scope from "extract another corpse helper layer" to "document and close an already-satisfied cleanup target"
  - verified that the current architecture already routes both corpse loot and corpse-backed acquisition through the same shared sufficiency rule
- Deviations from the original plan:
  - no production code changes were made
  - no tests were added or modified because the relevant regressions already existed and passed
- Verification results:
  - `cargo test -p worldwake-ai corpse`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
