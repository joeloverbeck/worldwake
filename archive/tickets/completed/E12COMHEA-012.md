# E12COMHEA-012: Loot action definition + handler

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — primarily `worldwake-systems`; `worldwake-sim` only if implementation proves a missing action-surface hook
**Deps**: E12COMHEA-002 (`DeadAt`), E12COMHEA-003 (`CarryCapacity` already in use), E12COMHEA-004 (`LootActionPayload`), E12COMHEA-005 (`TargetDead` / `TargetIsAgent` / combat constraints), E12COMHEA-006 (wound helpers)

## Problem

The Loot action allows a living agent to strip physically carried inventory from a dead agent at the same place. This is the only corpse interaction supported in Phase 2. It requires co-location, the target having `DeadAt`, and the looter being alive and not incapacitated.

## Assumption Reassessment (2026-03-11)

1. `LootActionPayload { target }` already exists in [`crates/worldwake-sim/src/action_payload.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs), but the current action framework already binds the corpse target through `ActionInstance.targets`.
2. `TargetDead(u8)`, `TargetIsAgent(u8)`, `ActorNotDead`, and `ActorNotIncapacitated` already exist and are already validated in affordance/start-gate code. This ticket should not re-spec those changes as missing work.
3. Combat action registration currently lives in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs), not in `worldwake-sim` registries. Adding Loot should follow that established combat extension point.
4. Carrying is modeled through possession plus nested containment, not through a dedicated inventory container and not through ownership. `CarryCapacity` already exists and transport actions already enforce it.
5. There is no `transfer_ownership()` helper. The relevant primitives are `set_possessor`, `clear_possessor`, `set_owner`, `clear_owner`, `split_lot`, and containment operations on `WorldTxn`.
6. The codebase already distinguishes ownership from possession. For corpse looting, possession transfer is the core mechanic; ownership transfer is a separate design choice, not a prerequisite.
7. There is no existing dynamic duration expression for "sum of accessible loot load". Adding one would expand `worldwake-sim` semantics. That may be justified later for generalized inventory-transfer actions, but it is not currently required for a clean Loot implementation.
8. The relevant reference architecture is closer to transport/trade transfer code than to the existing attack/defend payload path alone.
9. Because loot has no extra user choice beyond the already bound corpse target, requiring a redundant payload would be worse architecture than deriving the target from `ActionInstance.targets`.

## Architecture Decision

Implement Loot as a corpse-possession transfer action in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs), reusing the existing possession/carry-capacity model instead of inventing a parallel corpse-inventory architecture.

This is better than the original ticket framing because:

1. It matches the existing transport model: what an actor physically carries is represented by possession plus nested containment.
2. It avoids introducing a second transfer concept that bypasses `CarryCapacity`, travel propagation, or control semantics.
3. It keeps corpse interaction state-mediated and local: the dead agent's possessions remain concrete world entities at the same place and simply change holder.
4. It is extensible toward future corpse interactions without adding a one-off action-registry or ownership-transfer abstraction that the rest of the engine does not use.

### Payload Handling

1. Loot should derive its target from the bound action target.
2. Do not require `ActionPayload::Loot` for the happy path when the payload carries no information beyond the bound target.
3. The existing `LootActionPayload` type may remain available for future UI/planner plumbing, but this ticket should not force redundant runtime state just to exercise the type.

### Scope Of Looted Inventory

1. Loot should operate on the corpse's physically carried inventory, meaning direct possessions and nested contents under possessed containers.
2. It should not reach out to unpossessed world property merely because the dead agent happens to own it.
3. It should not archive, clean up, or otherwise special-case the corpse itself.

### Duration

1. Use a fixed duration for this ticket unless implementation reveals a compelling, reusable duration abstraction.
2. Do not add a loot-specific dynamic `DurationExpr` just to encode a one-off load-based timing rule.
3. If weighted transfer timing is still desired later, it should be introduced as a generalized inventory-transfer duration mechanism, not as Loot-only semantic debt in `worldwake-sim`.

## What to Change

### 1. Define Loot ActionDef in combat

- Constraints: `ActorAlive`, `ActorNotDead`, `ActorNotIncapacitated`, `ActorNotInTransit`, `ActorHasControl`
- Targets: one target Agent at actor's place
- Preconditions: `TargetAtActorPlace(0)`, `TargetDead(0)`, `TargetIsAgent(0)`
- Duration: `DurationExpr::Fixed` with a minimal deterministic default
- Interruptibility: `FreelyInterruptible`
- Payload: `ActionPayload::None` for the default affordance path; use the bound target as the authoritative source

### 2. Register Loot alongside existing combat actions

- Add `register_loot_action(defs, handlers) -> ActionDefId` in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs)
- Export it from [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs)

### 3. Implement Loot handler in combat

- Validate payload target matches bound target and remains co-located
- Walk the corpse's directly possessed inventory in deterministic order
- Transfer as much as fits under the looter's current `CarryCapacity`
- Reuse existing split-lot behavior when only part of a lot fits
- Preserve nested containment under looted containers instead of flattening it
- Keep the corpse in place even when all possessions have been looted
- Emit the action event through the normal action pipeline with public same-place visibility

### 4. Reuse existing architecture instead of widening `worldwake-sim`

- Prefer local helper reuse/extraction inside `worldwake-systems` if transport/combat need the same load-accounting logic
- Do not modify `crates/worldwake-sim/src/action_def_registry.rs` or `crates/worldwake-sim/src/action_handler_registry.rs` unless a real framework gap is discovered
- Do not introduce ownership aliases, backwards-compatibility shims, or a corpse-specific inventory subsystem

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify — Loot action def/handler/tests)
- `crates/worldwake-systems/src/lib.rs` (modify — export Loot registration)
- `crates/worldwake-systems/src/transport_actions.rs` (optional — only if a shared carry-capacity helper is extracted)
- `crates/worldwake-sim/src/action_payload.rs` (no functional change expected; payload already exists)

## Out of Scope

- Bury action (explicitly deferred per spec)
- Discover/inspect corpse (explicitly deferred per spec)
- Corpse auto-cleanup (explicitly deferred per spec)
- Attack/Defend/Heal actions (separate tickets)
- AI deciding when to loot (E13)
- Selective looting UI/payload for choosing specific items
- Generalized weighted transfer durations for all inventory-moving actions
- Any backward-compatibility layer between possession-based carrying and hypothetical future inventory abstractions

## Acceptance Criteria

### Tests That Must Pass

1. Loot action is registered from the combat module with the expected constraints, preconditions, visibility, and fixed duration
2. Loot action transfers directly possessed corpse inventory to the looter
3. Loot action preserves nested containment when a looted container moves to the looter
2. Cannot loot alive agents (precondition `TargetDead`)
3. Cannot loot if dead (constraint `ActorNotDead`)
4. Cannot loot if incapacitated (constraint `ActorNotIncapacitated`)
5. Must be co-located with target (precondition `TargetAtActorPlace`)
6. Target must be an Agent (precondition `TargetIsAgent`)
7. Carry capacity is respected; loot transfers only what fits
8. Partial transfer splits item lots deterministically when only part fits
9. Corpse retains possessions that were not transferred
10. Conservation holds for all transferred item lots
11. Corpse remains dead and in place after looting
12. Loot event is emitted through the action/event pipeline and is visible at the place
13. Relevant focused suites pass, followed by full workspace tests/lint

### Invariants

1. 9.5: Conservation — items transfer, never created or destroyed
2. Dead agent body persists — not archived by looting
3. Possession remains the authoritative carrier model for physically carried inventory
4. No global or non-local inventory lookup is introduced on behalf of agents
5. No backwards-compatibility shims or alias relations are introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs`
   - Loot action registration metadata
   - successful loot from corpse to looter
   - alive/dead/incapacitated/co-location/target-kind gate coverage
   - carry-capacity-limited loot with deterministic partial split
   - conservation and corpse-retained remainder checks
   - nested-container corpse inventory transfer if implementation supports container looting directly

### Commands

1. `cargo test -p worldwake-systems combat::tests`
2. `cargo test -p worldwake-systems`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completion date: 2026-03-11

What actually changed:

1. Added `register_loot_action` plus Loot action definition/handler in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs).
2. Loot now transfers a corpse's direct possessions using the existing possession model, preserves nested containment under looted containers, and respects `CarryCapacity`.
3. Deterministic partial loot now splits item lots when only part of a corpse-held lot fits.
4. Added a small shared inventory helper module at [`crates/worldwake-systems/src/inventory.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/inventory.rs) so transport and loot share carried-load accounting instead of duplicating it.
5. Exported loot registration from [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs).

Deviations from the original plan:

1. No changes were needed in `crates/worldwake-sim/src/action_def_registry.rs` or `crates/worldwake-sim/src/action_handler_registry.rs`; combat action registration already belongs in `worldwake-systems`.
2. Loot uses the bound action target and `ActionPayload::None` on the default path; the existing `LootActionPayload` type remains available but is not required for the normal affordance flow.
3. Loot transfers possession, not ownership. That keeps corpse looting aligned with the current physical-carry architecture instead of inventing unscoped inheritance/theft semantics in this ticket.
4. Duration stayed fixed rather than adding a loot-specific dynamic duration expression to `worldwake-sim`.

Verification results:

1. `cargo test -p worldwake-systems combat::tests`
2. `cargo test -p worldwake-systems`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
