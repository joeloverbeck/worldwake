# E14PERBEL-010: Reassess Combat And Corpse Goal Behavior Under Subjective Beliefs

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: No additional production changes required on 2026-03-14; this ticket records that the intended behavior is already present in the current `worldwake-ai` architecture
**Deps**: `archive/tickets/E14PERBEL-006.md`, `archive/tickets/completed/E14PERBEL-007.md`, `specs/E14-perception-beliefs.md`, `specs/S02-goal-decision-policy-unification.md`

## Problem

This ticket originally assumed that the E14 migration had left combat/corpse goal behavior materially broken under subjective beliefs and that new production fixes were still required in:

- corpse-loot goal generation
- corpse-burial goal generation
- living combat goal generation and mitigation selection
- `golden_combat` coverage

That assumption is now stale relative to the current codebase. The correct work for this ticket is to reassess the assumption against live code and tests, then either:

1. land the missing production fix if a real architectural gap still exists, or
2. close the ticket if the intended architecture is already implemented and verified

## Assumption Reassessment (2026-03-14)

1. The headline assumption of an active regression is no longer true. `cargo test -p worldwake-ai --test golden_combat`, `cargo test -p worldwake-ai`, and `cargo test --workspace` all pass in the current repository state.
2. The specific corpse-loot bug that motivated this ticket has already been fixed in production code. `crates/worldwake-ai/src/candidate_generation.rs` no longer relies only on `direct_possessions(corpse)` for loot-goal emission; `corpse_has_known_loot()` now falls back to believed corpse commodity quantities.
3. Existing tests already cover the critical belief-local corpse cases:
   - `candidate_generation::tests::local_corpse_with_believed_inventory_emits_loot_goal`
   - `candidate_generation::tests::local_corpse_with_grave_plot_emits_bury_goal`
   - `agent_tick::tests::unseen_death_does_not_create_corpse_reaction_without_reobservation`
   - the `golden_combat` acceptance scenarios for living combat, danger mitigation, burial, and opportunistic loot
4. The living-combat behavior this ticket wanted to restore is also already present. Current candidate generation still emits `EngageHostile` / `ReduceDanger` from visible hostility and current attackers, and the existing combat goldens remain green.
5. Reintroducing omniscient access or adding compatibility aliases would now be strictly worse than the current architecture. The current code already satisfies the relevant E14/FND goals using subjective belief reads.

## Architecture Check

1. The current architecture is better than the originally proposed ticket work because it already achieves the desired behavior without backsliding on Principles 12-15 or adding compatibility layers.
2. `LootCorpse` goal generation is now robust against the original belief-boundary issue because it accepts either:
   - known corpse possessions, or
   - believed corpse commodity quantities
3. `BuryCorpse` generation remains belief-local and concrete: it requires a perceived corpse at the local place plus a local grave-plot workstation.
4. `EngageHostile` / `ReduceDanger` remain the correct long-term shape for combat reaction because they are grounded in current visible hostility, attackers, thresholds, and concrete mitigations rather than a separate combat-policy shim.
5. The proposed broad production audit across ranking, search, goal model, and agent tick is not more beneficial than the current architecture unless a new failing scenario appears. For this ticket, forcing further changes would be churn, not hardening.
6. One possible future hardening seam remains outside this ticket's closure scope: `corpse_contains_commodity()` in acquisition-path evidence still checks only direct corpse possessions, not believed corpse aggregate commodity quantities. That does not break the current combat/corpse goal architecture, but it is a narrower asymmetry worth revisiting only if a future scenario proves it matters.

## Scope Correction

This ticket is reduced from an implementation ticket to a verification-and-closure ticket.

### Required work

1. Reassess the ticket assumptions against current code, specs, and tests.
2. Correct the ticket so it documents the real current state.
3. Verify the relevant narrow, crate, and workspace test/lint commands.
4. Archive the ticket with an accurate `Outcome` section.

### No longer required

1. Additional production edits in `candidate_generation.rs`, `ranking.rs`, `search.rs`, `goal_model.rs`, or `agent_tick.rs`.
2. Golden rebaselining.
3. Any compatibility wrapper, aliasing layer, or omniscient fallback.

## Files To Touch

- `tickets/E14PERBEL-010.md` (modify, then archive)

## Out of Scope

- New production behavior changes to combat/corpse AI unless a failing scenario is discovered
- Splitting the mixed belief/query boundary further than `E14PERBEL-009`
- Rumor/report propagation (`E15`)
- Reintroducing omniscient planning access in any form

## Acceptance Criteria

### Verification That Must Pass

1. `cargo test -p worldwake-ai --test golden_combat`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

### Invariants

1. Combat and corpse goals remain explainable from subjective belief state plus allowed self/public/runtime read surfaces.
2. Corpse and combat behavior do not rely on omniscient entity discovery.
3. No compatibility alias or fallback path to deleted omniscient belief code is introduced.
4. The archived ticket truthfully reflects that the required production behavior is already present.

## Test Plan

### New/Modified Tests

1. No new or modified tests are required for this closure because the current codebase already contains direct unit and acceptance coverage for the originally suspected regression.

### Existing Tests Confirming The Behavior

1. `crates/worldwake-ai/src/candidate_generation.rs`
   - `local_corpse_with_possessions_emits_loot_goal`
   - `local_corpse_with_believed_inventory_emits_loot_goal`
   - `local_corpse_with_grave_plot_emits_bury_goal`
2. `crates/worldwake-ai/src/agent_tick.rs`
   - `unseen_death_does_not_create_corpse_reaction_without_reobservation`
3. `crates/worldwake-ai/tests/golden_combat.rs`
   - corpse loot, burial, living combat, mitigation, deterministic replay, and seed-sensitivity scenarios

### Commands

1. `cargo test -p worldwake-ai --test golden_combat`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - reassessed the ticket against the live code and tests
  - corrected the ticket scope from "implement missing combat/corpse belief fixes" to "document and close an already-landed fix"
  - confirmed that the production AI stack already supports subjective corpse loot, burial, and living combat behavior
- Deviations from original plan:
  - no production code changes were necessary
  - no tests were added or modified because the relevant regression coverage was already present and passing
- Verification results:
  - `cargo test -p worldwake-ai --test golden_combat`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
