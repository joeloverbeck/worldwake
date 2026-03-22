# E16BFORLEGJURCON-003: Add PressForceClaim and YieldForceClaim action payloads

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — shared action payload contract in `worldwake-sim`
**Deps**: None

## Problem

The spec requires two new action payload types (`PressForceClaim`, `YieldForceClaim`) so later system-layer tickets can add force-claim action defs, handlers, and AI wiring without inventing ad hoc payload shapes.

## Assumption Reassessment (2026-03-22)

1. `crates/worldwake-sim/src/action_payload.rs` currently defines payload structs and typed accessors for `ConsultRecord`, `Tell`, `Bribe`, `Threaten`, `DeclareSupport`, `Transport`, `Harvest`, `Craft`, `Trade`, `Combat`, `Loot`, and `QueueForFacilityUse`. `PressForceClaim` and `YieldForceClaim` do not exist.
2. Action def registration does not live in `worldwake-sim`. The live registrations for political actions are in [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs), and the aggregate catalog is built in [`crates/worldwake-systems/src/action_registry.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs). The original ticket scope incorrectly pulled action-def registration into this payload ticket.
3. `ActionDomain::Social` exists, but it is only relevant to later system-layer tickets that add the actual defs. This ticket should not register placeholder defs in `worldwake-sim`; that would blur the sim/system boundary and create incomplete live actions.
4. There are no `E16BFORLEGJURCON-001` or `-002` ticket files under `tickets/`. The original dependency list is stale and has been removed.
5. Existing payload coverage already lives in `crates/worldwake-sim/src/action_payload.rs` as focused accessor and bincode round-trip tests. The natural extension point is to add the new force-claim payload coverage there rather than inventing a separate registry test.
6. This is not an AI, golden, ordering, heuristic-removal, start-failure, ControlSource, or cumulative-arithmetic ticket. The relevant verification layer is the payload contract itself.
7. Mismatch corrected: this ticket now covers only payload structs, enum variants, accessors, and exports. Action defs and handlers stay with later `worldwake-systems` tickets.

## Architecture Check

1. Keeping this ticket payload-only is cleaner than mixing in action defs. `worldwake-sim` owns the shared serialized action contract; `worldwake-systems` owns concrete political action behavior. Preserving that split avoids an incomplete intermediate architecture where defs exist without authoritative handlers.
2. The change follows the existing `DeclareSupport` pattern exactly where it belongs: struct payload + enum variant + typed accessor + crate re-export. No alias types, no shims, no duplicate source of truth.
3. This is more robust long-term because later force-claim work can depend on a stable payload contract without prematurely coupling `worldwake-sim` to office-specific rule enforcement.

## Verification Layers

1. Payload shape and typed accessors -> focused unit tests in `crates/worldwake-sim/src/action_payload.rs`
2. Serialization compatibility for replay/save boundaries -> focused bincode round-trip tests in `crates/worldwake-sim/src/action_payload.rs`
3. Public API exposure for downstream crates -> compile-time coverage through `worldwake-sim` crate exports plus `cargo test -p worldwake-sim`

## What to Change

### 1. Define payload structs in `action_payload.rs`

```rust
pub struct PressForceClaimActionPayload {
    pub office: EntityId,
}

pub struct YieldForceClaimActionPayload {
    pub office: EntityId,
}
```

### 2. Add enum variants to `ActionPayload`

Add `PressForceClaim(PressForceClaimActionPayload)` and `YieldForceClaim(YieldForceClaimActionPayload)` variants, with `as_press_force_claim()` and `as_yield_force_claim()` accessor methods following the existing pattern.

### 3. Re-export the payload structs from `worldwake-sim`

Update [`crates/worldwake-sim/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/lib.rs) so downstream system and AI tickets can import the new payload types through the crate root, matching the existing public API pattern.

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify — add structs, variants, accessors)
- `crates/worldwake-sim/src/lib.rs` (modify — re-export new payload types)

## Out of Scope

- Action defs and handler registration in `worldwake-systems`
- Action handlers (commit effects) — that's E16BFORLEGJURCON-004
- Force control system — E16BFORLEGJURCON-005
- AI affordance enumeration — E16BFORLEGJURCON-007
- Institutional belief variants — E16BFORLEGJURCON-006
- Precondition validation logic (authoritative checks) — E16BFORLEGJURCON-004

## Acceptance Criteria

### Tests That Must Pass

1. `ActionPayload::PressForceClaim` can be constructed and accessed via `as_press_force_claim()`
2. `ActionPayload::YieldForceClaim` can be constructed and accessed via `as_yield_force_claim()`
3. Both payload variants round-trip through bincode serialization
4. `worldwake_sim::{PressForceClaimActionPayload, YieldForceClaimActionPayload}` are publicly re-exported
5. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Payload structs contain only `office: EntityId` — no redundant claimant/controller identity field
2. This ticket does not introduce placeholder action defs or handlers outside their owning layer
3. No existing tests break

## Tests

### New/Modified Tests

1. `crates/worldwake-sim/src/action_payload.rs` `action_payload_satisfies_required_traits`
Rationale: extends the trait contract coverage to the two new payload structs so they remain cloneable, serializable, and comparable like every other payload type.
2. `crates/worldwake-sim/src/action_payload.rs` `typed_accessors_cover_social_payload_variants`
Rationale: proves each new enum variant is reachable only through its matching typed accessor and does not collide with existing social payload accessors.
3. `crates/worldwake-sim/src/action_payload.rs` `press_force_claim_payload_roundtrips_through_bincode`
Rationale: locks the serialized shape for replay/save boundaries.
4. `crates/worldwake-sim/src/action_payload.rs` `yield_force_claim_payload_roundtrips_through_bincode`
Rationale: locks the serialized shape for replay/save boundaries.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-22
- What changed:
  - Corrected the ticket scope to match the live architecture before implementation.
  - Added `PressForceClaimActionPayload` and `YieldForceClaimActionPayload` to `ActionPayload`, along with typed accessors and `worldwake-sim` crate-root re-exports.
  - Extended existing payload trait/accessor/serialization coverage and updated `ActionTraceDetail::from_payload` to handle the new enum variants exhaustively.
- Deviations from original plan:
  - Did not add action defs or registry coverage here. That original scope was architecturally wrong for this layer because action registration lives in `worldwake-systems`, not `worldwake-sim`.
  - Removed stale nonexistent dependencies `E16BFORLEGJURCON-001` and `E16BFORLEGJURCON-002`.
- Verification results:
  - `cargo test -p worldwake-sim` passed.
  - `cargo clippy --workspace` passed.
  - `cargo test --workspace` passed.
