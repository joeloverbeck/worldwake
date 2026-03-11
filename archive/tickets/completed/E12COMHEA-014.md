# E12COMHEA-014: Combat system tick function + SystemDispatch wiring

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-systems combat module + system dispatch
**Deps**: `archive/tickets/completed/E12COMHEA-008.md`, `archive/tickets/completed/E12COMHEA-009.md`, `archive/tickets/completed/E12COMHEA-010.md`, `archive/tickets/completed/E12COMHEA-011.md`, `specs/E12-combat-health.md`, `tickets/E12COMHEA-000-index.md`

## Problem

This ticket's original framing is stale. The repository already contains a combat system tick and already wires `SystemId::Combat` to it in the dispatch table. The remaining work for this ticket is to correct the ticket itself, verify the implementation against the E12 spec and current architecture, harden any missing edge-case tests, and archive the ticket with an accurate outcome.

## Assumption Reassessment (2026-03-11)

1. `SystemId::Combat` already exists as position 3 in `SystemManifest` — confirmed.
2. `dispatch_table()` in `crates/worldwake-systems/src/lib.rs` already maps Combat to `combat_system`, not `noop_system`.
3. `SystemExecutionContext` provides `world`, `event_log`, `rng`, `active_actions`, `action_defs`, `tick`, `system_id` — confirmed.
4. Pattern: system function signature is `fn combat_system(SystemExecutionContext) -> Result<(), SystemError>`.
5. System execution order: Needs(0), Production(1), Trade(2), Combat(3), Perception(4), Politics(5).
6. `crates/worldwake-systems/src/combat.rs` already implements:
   - `combat_system()`
   - wound progression via `apply_wound_progression()`
   - death detection / `DeadAt` attachment via `collect_fatalities()`
   - focused tests for dispatch wiring, fatality emission, wound progression, clotting, recovery gating, and same-tick fatality
7. Active combat action processing remains in the action framework handlers (`Attack`, `Defend`, `Heal`, `Loot`). The combat system tick does not and should not duplicate that work.

## Architecture Check

1. Combat system tick runs after Needs, Production, and Trade — correct per system manifest order.
2. Each tick, the combat system:
   a. Progresses all wounds (bleeding, clotting, recovery) for all living agents
   b. Checks for deaths (wound load >= capacity) and attaches `DeadAt`
   c. Active combat actions are processed by their handlers through the action framework — the system tick doesn't re-process them here
3. The system does NOT directly handle individual attack/defend/heal/loot actions — those are handled by the action handler framework during the action phase of the tick. The combat system tick handles wound progression and death detection only.
4. This architecture is better than the original ticket wording. Keeping action execution in handlers and keeping the system tick focused on shared-state progression preserves Principle 12, avoids duplicating action logic, and leaves the combat module extensible as new bodily-harm consequences are added.
5. The one meaningful remaining verification gap is edge-case hardening around entities that carry wounds but lack `CombatProfile`. The current implementation intentionally skips them; that behavior should be explicitly tested rather than inferred.

## Revised Scope

This ticket now covers:

1. Reconcile the ticket with the codebase and spec.
2. Keep the existing combat tick / dispatch architecture unless verification reveals a defect.
3. Add only the missing targeted tests needed to prove the current implementation's edge-case behavior.
4. Run focused and broad verification.
5. Mark the ticket complete and archive it with an accurate `Outcome`.

## What to Change

### 1. Keep the current combat tick architecture

Do not move action execution into the combat system tick. The current split is the clean design:

- action handlers own concrete attack/defend/heal/loot behavior
- `combat_system()` owns wound progression and fatality detection
- both paths communicate only through shared state and emitted events

### 2. Verify dispatch and fatality/progression behavior

Retain the existing production implementation in:

- `crates/worldwake-systems/src/combat.rs`
- `crates/worldwake-systems/src/lib.rs`

Only change production code if verification exposes a real defect.

### 3. Harden edge-case test coverage

Add or strengthen tests for the remaining uncovered invariants, especially:

- agents with wounds but no `CombatProfile` are skipped safely
- dispatch still uses the combat slot correctly
- fatality and wound progression behavior remains deterministic and state-mediated

## Out of Scope

- Re-implementing `combat_system()` or re-wiring dispatch that is already present
- Moving attack/defend/heal/loot handling out of the action framework into the system tick
- Refactoring unrelated combat action code while closing this ticket
- Multi-tick end-to-end combat integration scenarios beyond focused ticket-level coverage (`E12COMHEA-015`)

## Acceptance Criteria

### Tests That Must Pass

1. Existing focused combat-system tests still pass.
2. Combat system progresses bleeding wounds correctly.
3. Combat system detects death and attaches `DeadAt`.
4. Combat system emits death events without archiving corpses or dropping inventory/location context.
5. Combat system does not crash on agents without `CombatProfile` and leaves their wound state unchanged.
6. Dispatch table still uses `combat_system` for `SystemId::Combat`.
7. System execution order remains unchanged with Combat at slot 3.
8. `BodyCostPerTick` / other cross-system consequences are not special-cased here; the combat tick remains focused on combat-owned state.
9. Relevant focused suites pass, followed by `cargo test --workspace` and `cargo clippy --workspace`.

### Invariants

1. Principle 12: system decoupling — combat system depends only on core + sim, not other system modules
2. Principle 6: deterministic execution
3. All mutations go through WorldTxn
4. Events are emitted for combat-owned state changes performed here (wound progression batch and deaths)
5. No backward-compatibility aliasing or duplicate action-processing path is introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs`
   - verify dispatch table wiring remains correct
   - verify fatality event emission and `DeadAt` attachment
   - verify wound progression, clotting, recovery gating, and same-tick fatality
   - add explicit graceful-skip coverage for wounded entities without `CombatProfile`

### Commands

1. `cargo test -p worldwake-systems -- combat`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Reassessed the ticket against the live codebase and corrected the ticket's stale assumptions.
  - Verified that `crates/worldwake-systems/src/combat.rs` already contains the production `combat_system()` implementation and that `crates/worldwake-systems/src/lib.rs` already wires `SystemId::Combat` to it.
  - Added focused regression coverage for the edge case where an entity has wounds but lacks `CombatProfile`; the combat system now has explicit test coverage proving it skips such entities safely without mutating state or emitting events.
- Deviations from original plan:
  - No production combat-system or dispatch implementation work was needed because that work had already been completed in the repository.
  - The only code change for this ticket was test hardening, not new architecture or dispatch rewiring.
  - The architecture assessment favored the current design over the original ticket wording: action handlers own concrete combat actions, while `combat_system()` owns state progression and fatality detection.
- Verification results:
  - `cargo test -p worldwake-systems -- combat` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace` ✅
