# E16OFFSUCFAC-006: Implement Bribe, Threaten, DeclareSupport Action Handlers

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new social action module in `worldwake-systems`, action registry wiring, and a small event/perception classification refinement
**Deps**: E16OFFSUCFAC-001, E16OFFSUCFAC-003, E16OFFSUCFAC-004, E16OFFSUCFAC-005

## Problem

E16 still lacks the action-layer implementation for three social/political actions already modeled elsewhere in the architecture:

- `Bribe`
- `Threaten`
- `DeclareSupport`

The earlier draft of this ticket overstated what was missing in core and sim. Most of the prerequisite data model already exists. The real work is narrower:

1. add the action definitions and handlers in `worldwake-systems`
2. register them in the action catalog
3. make witness/social-observation classification precise enough that the new actions are recorded correctly

Without that last part, implementing these actions naively would produce incorrect belief evidence, because the current perception layer classifies every `EventTag::Social` event as `WitnessedTelling`.

## Assumption Reassessment (2026-03-15)

### Confirmed

1. `ActionDomain::Social` already exists and is already used by Tell.
2. `EventTag::Social` and `EventTag::Political` already exist.
3. `VisibilitySpec::SamePlace` already exists and is the correct visibility boundary for these actions.
4. The action handler pattern (`start`, `tick`, `commit`, `abort`) is established by Tell, Trade, Combat, and other system actions.
5. `support_declarations` already exists in core, with world and `WorldTxn` mutation/query APIs.
6. `UtilityProfile.courage: Permille` already exists.
7. `OfficeData`, `FactionData`, `SuccessionLaw`, `EligibilityRule`, and office/faction ECS registration already exist.
8. `ActionPayload::{Bribe, Threaten, DeclareSupport}` and their typed payload structs/accessors already exist in `worldwake-sim`.
9. `SocialObservationKind::{WitnessedObligation, WitnessedConflict, WitnessedCooperation}` already exist in core.
10. `GoalKind::{ClaimOffice, SupportCandidateForOffice}` already exist, but richer AI candidate generation/planner use is still out of scope here.
11. Low-level social mutation helpers already exist for loyalty, hostility, support declarations, and office assignment in core.

### Incorrect assumptions in the previous draft

1. This ticket does **not** need to add payload structs, `support_declarations`, `courage`, office/faction components, or social-observation enum variants. Those are already present.
2. This ticket does **not** need to touch `worldwake-core` just to add `WitnessedObligation` / `WitnessedConflict` / `WitnessedCooperation`.
3. The original “Files to Touch” list was incomplete. Correct witness classification likely requires touching perception/event classification, not just adding a new action module.
4. The original threat formula referenced a non-existent `attack_skill_base` field. The current combat model exposes `CombatProfile.attack_skill`; any threat comparison must be based on the actual combat fields in the codebase.
5. The original draft implied all three actions could rely on coarse event tags alone. That is not true under the current perception architecture, because `perception::social_kind()` currently maps generic `EventTag::Social` to `WitnessedTelling`.

## Architecture Decision

### Keep

1. Implement these as normal `ActionDef` + `ActionHandler` entries in `worldwake-systems`.
2. Reuse the existing canonical world-state carriers:
   - goods transfer for bribery
   - `loyal_to` for coercive or purchased alignment shifts
   - `hostile_to` for resisted threats
   - `support_declarations` for public office support
3. Keep all mutations event-sourced through `WorldTxn`; no side storage or handler-local pseudo-state.
4. Keep `DeclareSupport` as a direct action over public political state rather than aliasing it to loyalty.

### Refine

1. Tighten event semantics so perception can distinguish:
   - telling
   - obligation/bribery
   - coercion/conflict
   - political cooperation
2. Do this with the smallest durable architecture change, not a special-case hack in the action handlers.

### Benefit vs current architecture

These action implementations are more beneficial than the current architecture because E16 currently has the world-state substrate for offices, factions, loyalty, hostility, support declarations, and courage, but no legal action path that actually drives that state through time. Without these handlers:

- public support is a relation with no embodied social action
- loyalty shifts from bribery/threats have no causal carrier
- AI and human control can reason about office politics conceptually but cannot execute them through the action framework

The one architectural risk is witness classification. The current coarse `EventTag::Social -> WitnessedTelling` shortcut does not scale. Tightening that layer now is cleaner than encoding action-specific inference elsewhere.

## Correct Scope

### 1. Add `worldwake-systems` office/social action module

Create a focused module for:

- `bribe`
- `threaten`
- `declare_support`

This module should follow the existing action style used by `tell_actions.rs` and `trade_actions.rs`.

### 2. Register the actions in the main catalog

Update the full action registry so the three new action defs are part of the canonical system action catalog and `verify_completeness()` still passes.

### 3. Refine social observation classification

Update event/perception classification so witnesses record the right `SocialObservationKind` for:

- bribe -> `WitnessedObligation`
- threaten -> `WitnessedConflict`
- declare support -> `WitnessedCooperation`
- tell -> still `WitnessedTelling`

This refinement should live in the existing event/perception path, not as an out-of-band special case elsewhere.

### 4. Implement Bribe

- Domain: `ActionDomain::Social`
- Target: colocated live agent
- Duration: fixed 2 ticks
- Interruptibility: `FreelyInterruptible`
- Commit requirements:
  - actor alive
  - target alive and colocated
  - actor still controls the offered commodity quantity
- Commit effects:
  - transfer the offered goods from actor to target using the existing lot/control transfer model
  - preserve conservation
  - increase target loyalty toward actor using existing loyalty mutation APIs
- Event semantics:
  - same-place visibility
  - enough event tagging/classification to produce `WitnessedObligation`

### 5. Implement Threaten

- Domain: `ActionDomain::Social`
- Target: colocated live agent
- Duration: fixed 1 tick
- Interruptibility: not freely interruptible
- Commit requirements:
  - actor alive
  - target alive and colocated
  - actor has a `CombatProfile`
  - target has a `UtilityProfile`
- Commit effects:
  - derive a threat pressure from real combat state already present in the codebase
  - compare it against target `courage`
  - on yield: raise target loyalty toward actor
  - on resist: add hostility from target toward actor
- Event semantics:
  - same-place visibility
  - enough event tagging/classification to produce `WitnessedConflict`

### 6. Implement DeclareSupport

- Domain: `ActionDomain::Social`
- Payload-driven action with no fake aliasing to another relation
- Commit requirements:
  - actor alive
  - office exists and is an office entity
  - candidate exists and is alive
  - actor is at the office jurisdiction place
  - office is vacant
- Commit effects:
  - call the existing `declare_support(actor, office, candidate)` transaction API
  - overwrite prior declaration for the same `(actor, office)` pair through the canonical relation
- Event semantics:
  - same-place visibility
  - `Political` classification should result in `WitnessedCooperation`

## Files To Touch

- `crates/worldwake-systems/src/office_actions.rs` (new)
- `crates/worldwake-systems/src/action_registry.rs`
- `crates/worldwake-systems/src/lib.rs`
- `crates/worldwake-systems/src/perception.rs`
- `crates/worldwake-core/src/event_tag.rs` only if the cleanest witness-classification fix requires a more precise tag

## Out Of Scope

- succession resolution logic that consumes support declarations (`E16OFFSUCFAC-007`)
- public-order aggregation (`E16OFFSUCFAC-008`)
- planner-op classification, candidate generation, and office-political AI planning (`E16OFFSUCFAC-009`)
- force-law succession combat orchestration beyond the `Threaten` social action itself
- redesigning the full event taxonomy beyond the smallest durable change needed for correct witness classification

## Acceptance Criteria

### Tests That Must Pass

1. `bribe` registers in the full action catalog with `ActionDomain::Social`.
2. `threaten` registers in the full action catalog with `ActionDomain::Social`.
3. `declare_support` registers in the full action catalog with `ActionDomain::Social`.
4. `verify_completeness()` still passes for the full action registries.
5. Bribe transfers goods from actor to target on commit.
6. Bribe preserves lot conservation.
7. Bribe aborts or fails cleanly when the actor lacks the offered commodity quantity.
8. Bribe increases target loyalty toward actor.
9. Bribe witnesses record `WitnessedObligation`.
10. Threaten yields when actor threat pressure exceeds target courage.
11. Threaten resistance adds hostility from target toward actor.
12. Threaten yield increases target loyalty toward actor.
13. Threaten witnesses record `WitnessedConflict`.
14. DeclareSupport writes the canonical support declaration through the existing relation API.
15. DeclareSupport overwrites the previous declaration for the same `(actor, office)` pair.
16. DeclareSupport requires the actor to be at the office jurisdiction place.
17. DeclareSupport requires the office to be vacant.
18. DeclareSupport witnesses record `WitnessedCooperation`.
19. Existing Tell witness behavior still records `WitnessedTelling`.
20. `cargo test -p worldwake-systems`
21. `cargo clippy --workspace --all-targets -- -D warnings`
22. `cargo test --workspace`

### Invariants

1. Bribery transfers real goods; it does not create or destroy commodities.
2. Threat outcome is derived from existing authoritative combat and utility state, not stored as a separate abstract score.
3. Support declarations remain separate from loyalty; no aliasing or compatibility layer is introduced.
4. Witness knowledge remains same-place and event-mediated; no information teleportation.
5. The event/perception refinement must preserve existing Tell behavior.
6. No backward-compatibility wrappers or duplicate political state are introduced.

## Test Plan

### New / Modified Tests

1. `crates/worldwake-systems/src/office_actions.rs`
   - focused unit tests for registration, payload validation, commit/abort behavior, and relation/inventory outcomes
2. `crates/worldwake-systems/src/perception.rs`
   - witness classification tests proving the new actions map to the correct `SocialObservationKind`
3. `crates/worldwake-systems/src/action_registry.rs`
   - extend the full-catalog test to assert the new action names are present

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - added `bribe`, `threaten`, and `declare_support` action definitions and handlers in `worldwake-systems`
  - registered those actions in the canonical action catalog
  - refined event/perception classification so social witnesses distinguish telling, obligation, conflict, and political cooperation
  - added `EventTag::Coercion` to represent threat-style social conflict cleanly
  - implemented `DeclareSupport` against the existing support-declaration relation and enforced current succession-law eligibility at commit time
- Deviations from original plan:
  - the ticket was narrowed before implementation because payloads, office/faction data, courage, support declarations, and social-observation variants already existed
  - the key missing architecture work was action-layer wiring plus witness-classification refinement, not new core modeling
  - planner-op integration for these actions was not implemented here; instead, planner semantics tests were updated to reflect that this remains owned by `E16OFFSUCFAC-009`
  - bribery item provenance still reuses the existing transfer operation semantics instead of introducing a new lot-history operation in this ticket
- Verification results:
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-ai build_semantics_table_`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
