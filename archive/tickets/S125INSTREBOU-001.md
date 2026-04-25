# S125INSTREBOU-001: RewardEncumbrance component + treasury ownership conventions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new ECS component on `EntityKind::Office`; component schema + delta + world + component_tables registration; current save-format version bump for the persisted component-table shape
**Deps**: [S125 spec](../../specs/S125-institutional-treasuries-and-bounty-funding.md)

## Problem

Bounty posting today validates `RewardSource::InstitutionalTreasury` against `controlled_commodity_quantity(treasury_entity, kind)` at start/commit only. There is no encumbrance/reservation state, so two parallel `post_bounty` commits in the same tick can both validate against the same coin balance and both succeed — violating S125 Acceptance Criterion 5 ("Multiple active bounties cannot overpromise the same reserved funds"). Additionally, scenario authoring of office funds today must use loose item lots, which perturbs co-located scene perception (the `survival-justice` failure case noted in S125 Evidence #6). This ticket lands the foundational `RewardEncumbrance` ECS component on `EntityKind::Office` and codifies the treasury-container ownership conventions that subsequent tickets build on.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `EntityKind::Office` exists at `crates/worldwake-core/src/entity.rs:8-19`. The component schema macro `forward_authoritative_components!` at `crates/worldwake-core/src/component_schema.rs:3-100` accepts kind-check expressions per the New Component on EntityKind pattern (component_schema.rs has existing kind-check tests at lines 217-248). No existing test touches `RewardEncumbrance` because the component does not yet exist. Existing focused test for office-component infrastructure: `register_artifact_actions_creates_expected_definitions` (`crates/worldwake-systems/src/artifact_actions.rs:198`).
2. S125 Section H "Stored state vs. derived read models" mandates `RewardEncumbrance` as authoritative state with `Active → Released | Claimed` lifecycle. S125 conservation-integration subsection states encumbrance is a claim record (analogous to `SaleListing` at `crates/worldwake-core/src/trade.rs:25-29`), not a conserved quantity, so `verify_authoritative_conservation` and `verify_live_lot_conservation` (`crates/worldwake-core/src/conservation.rs:20-48`) do not need extension.
3. Shared abstraction boundary: ECS component schema + `OwnedBy`/`PossessedBy` relations. New component must register at all three macro expansion sites: `crates/worldwake-core/src/delta.rs`, `crates/worldwake-core/src/world.rs`, `crates/worldwake-core/src/component_tables.rs` (per `tickets/README.md` check #13).
4. Adjacent contradictions: none; treating encumbrance as a claim record (not a conserved quantity) is a required consequence of this ticket and avoids a new conserved domain in `verify_*_conservation`.
5. Live persistence check: `ComponentTables` is serialized inside saves, so adding `RewardEncumbrance` changes the current persisted shape. Per the repo's no-backward-compatibility rule, this ticket also bumps `SAVE_FORMAT_VERSION` and extends the existing non-default save/load roundtrip to include a persisted encumbrance.

## Architecture Check

1. `RewardEncumbrance` is a record analogous to `SaleListing` — a claim against existing conserved item lots, not a conserved quantity itself. This avoids a new conserved domain in `verify_*_conservation` (FND-3 + FND-4). The treasury-container convention (container `OwnedBy(office)`, lots inside `OwnedBy(office)`) reuses existing `controlled_item_lots_for(office)` and `controlled_commodity_quantity(office, kind)` (`crates/worldwake-core/src/world/ownership.rs:69-80`) without new helpers.
2. No backward-compatibility shims: pre-encumbrance behavior (commit-time-only validation) is replaced by ticket 005, not aliased.

## Verification Layers

1. Component registration → focused unit test exercising round-trip insert/query through the World API plus kind-check rejection on non-Office attachment.
2. Conservation reuse → existing `verify_authoritative_conservation` + `verify_live_lot_conservation` invariants continue to hold (no extension).
3. Single-layer ticket — no AI or trace surface; this is ECS infrastructure. Higher-layer assertions (decision/action traces, golden) belong to tickets 005, 006, 007.
4. Save-shape proof → existing `save_to_bytes_roundtrip_preserves_full_nondefault_state` includes a non-default `RewardEncumbrance`, and `SAVE_FORMAT_VERSION` advances once.

## What to Change

### 1. New `RewardEncumbrance` struct

Add `crates/worldwake-core/src/reward_encumbrance.rs` defining:

```rust
pub struct RewardEncumbrance {
    pub bounty_artifact: EntityId,
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub office: EntityId,
}
```

Derives: `Clone`, `Debug`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`. No `Hash` derive unless an existing component pattern requires it — verify against neighbouring components (e.g., `SaleListing`).

### 2. Schema registration

Register through `with_component_schema_entries!` in `crates/worldwake-core/src/component_schema.rs` with kind-check `|kind| kind == EntityKind::Office`. Propagate to `crates/worldwake-core/src/delta.rs`, `crates/worldwake-core/src/world.rs`, and `crates/worldwake-core/src/component_tables.rs` per `tickets/README.md` check #13.

### 3. Module export

Re-export `RewardEncumbrance` from `crates/worldwake-core/src/lib.rs` consistent with how sibling components like `SaleListing` are exported.

### 4. Treasury-container convention doc-comment

Add a doc comment on the `RewardEncumbrance` struct describing the ownership chain ticket 002 will materialize: treasury container is `EntityKind::Container` with `OwnedBy(container, office)`; lots inside are `OwnedBy(lot, office)` so existing `controlled_item_lots_for(office)` continues to enumerate them. This is documentation only — no relation-layer code changes in this ticket.

### 5. No conservation changes

`crates/worldwake-core/src/conservation.rs` is intentionally untouched. Encumbrance is a claim record, not a conserved quantity. If any downstream ticket discovers a conservation interaction, surface as a new finding rather than silently extending conservation here.

### 6. Save-format bump

Because the new component is part of persisted `ComponentTables`, bump `crates/worldwake-sim/src/save_load.rs::SAVE_FORMAT_VERSION` and extend the current-format roundtrip fixture with a non-default encumbrance.

## Files to Touch

- `crates/worldwake-core/src/reward_encumbrance.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module declaration + re-export)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify — save-format version bump + non-default roundtrip fixture)

## Out of Scope

- Encumbrance creation, release, or consumption logic — ticket 005.
- AI-layer reading of encumbrance state through the belief view — ticket 004.
- Scenario authoring of treasury containers — ticket 002.
- Conservation integration changes — confirmed unnecessary per S125 conservation-integration subsection; if a downstream ticket discovers otherwise, file as a new finding.
- Faction-side support — S125 Non-Goal (no scenario spawn surface for factions today).

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `world_inserts_and_queries_reward_encumbrance_on_office`.
2. New focused test: `reward_encumbrance_attachment_rejected_on_non_office_entity_kinds` (mirroring the existing pattern at `component_schema.rs:217-248`).
3. Existing suite: `cargo test -p worldwake-core`.

### Invariants

1. `RewardEncumbrance` can only be attached to `EntityKind::Office`.
2. `verify_authoritative_conservation` and `verify_live_lot_conservation` continue to pass without modification.
3. Component is registered at all three macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`).
4. Current-format save/load preserves a non-default `RewardEncumbrance`; older save formats remain rejected.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/reward_encumbrance.rs` (within `#[cfg(test)]`) — round-trip insert/query and kind-check rejection.
2. `crates/worldwake-core/src/component_schema.rs` (extend existing kind-check tests at lines 217-248) — add `RewardEncumbrance`-on-non-Office rejection case.
3. `crates/worldwake-sim/src/save_load.rs` — extend the existing current-format non-default roundtrip fixture with a `RewardEncumbrance`.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy -p worldwake-core --all-targets -- -D warnings`
3. `scripts/verify.sh`

## Outcome

Completed on 2026-04-25.

- Added `RewardEncumbrance` as an office-only authoritative ECS component and re-exported it from `worldwake-core`.
- Registered the component through the shared schema macro surfaces in `component_schema.rs`, `delta.rs`, `world.rs`, `world_txn.rs` projection, and `component_tables.rs`.
- Documented the treasury-container ownership convention on the component without changing relation or conservation logic.
- Bumped `SAVE_FORMAT_VERSION` from 45 to 46 because the new component is persisted through `ComponentTables`, and extended the current-format save/load roundtrip fixture with a non-default encumbrance.

## Deviations

- The drafted ticket did not name save-format fallout. Live reassessment showed the component-table shape is persisted, so the implementation includes the required current-format version bump and save/load proof.
- No conservation code changed; `RewardEncumbrance` remains a claim record against conserved item lots, not a conserved quantity.

## Verification Result

- Passed `cargo test -p worldwake-core --lib -- --list`
- Passed `cargo test -p worldwake-core --lib reward_encumbrance::tests::world_inserts_and_queries_reward_encumbrance_on_office -- --exact`
- Passed `cargo test -p worldwake-core --lib reward_encumbrance::tests::reward_encumbrance_attachment_rejected_on_non_office_entity_kinds -- --exact`
- Passed `cargo test -p worldwake-core --lib component_schema::tests::reward_encumbrance_is_registered_for_offices_only -- --exact`
- Passed `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact`
- Passed `cargo test -p worldwake-core`
- Passed `cargo clippy -p worldwake-core --all-targets -- -D warnings`
- Passed `./scripts/verify.sh`
