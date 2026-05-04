# S134CANEFFSCH-005: Production, stock, and transport schemas

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — replaces empty-placeholder schemas with real `EffectSchema` literals across production, stock, and transport actions and switches their commit handler bodies to `apply_effects(..., Authoritative)`. The S127 `partial_quantity` field becomes a typed `EffectFact`.
**Deps**: archive/tickets/S134CANEFFSCH-001.md, archive/tickets/S134CANEFFSCH-002.md

## Problem

S134 deliverable D5 requires migrating the commodity-movement family — production (harvest, craft per-recipe), stock management (store_stock, stage_stock_for_sale, collect_display_stock, unstage_stock), and transport (pick_up, put_down, steal, drop_item) — from imperative handler bodies to declarative `EffectSchema` evaluation. This is the largest single category because (a) production registrations are per-recipe (one `ActionDef` per harvestable resource and per craft recipe, so the count multiplies), (b) the S127 `CommitTraceData::Harvest.partial_quantity` field at `crates/worldwake-sim/src/action_handler.rs:45–47` becomes a typed `EffectFact::PartialQuantity`, and (c) several `EffectStep` variants (`Transfer`, `Consume`, `Produce`, `PartialOnFailure`) get their primary exercise here. The planner continues to use the old `apply_hypothetical_transition` path; the 8 explicit `PlannerTransitionKind` arms covering store/stage/collect/unstage and pick_up/steal/put_down stay intact through ticket 010.

## Assumption Reassessment (2026-05-04)

1. Production registrations live at `crates/worldwake-systems/src/production_actions.rs` via composites `register_harvest_actions` and `register_craft_actions`, each registering one `ActionDef` per recipe. Stock registrations at `crates/worldwake-systems/src/stock_actions.rs` via `register_stock_actions` (store_stock, stage_stock_for_sale, collect_display_stock, unstage_stock). Transport registrations at `crates/worldwake-systems/src/transport_actions.rs` via `register_transport_actions` (pick_up, put_down, steal, drop_item).
2. After ticket 001, every `ActionDef` literal in these three files has `effect_schema: EffectSchema::empty()`. This ticket populates each with the real schema and switches handler bodies.
3. `CommitTraceData::Harvest.partial_quantity: Option<Quantity>` at `crates/worldwake-sim/src/action_handler.rs:45–47` is the existing S127 partial-outcome shape. The schema's `EffectStep::PartialOnFailure { primary, fallback }` (defined in ticket 001) is the declarative form; `EffectFact::PartialQuantity { requested, delivered }` is the typed output. The handler-internal `Option<Quantity>` is replaced by the schema-emitted `EffectFact`.
4. Shared abstraction boundary under audit: 8 of the 9 `PlannerTransitionKind` arms (`PickUpGroundLot`, `StealGroundLot`, `PutDownGroundLot`, `StoreStockIntoLocalFacility`, `StageStoredStockForSale`, `CollectFacilityStockToPossession`, `UnstageDisplayedStock`, `ConsumeMatchingTargetCommodity`) are populated by the actions migrated in this ticket. These dispatch arms remain in `apply_hypothetical_transition` until ticket 010 deletes them; the planner still uses them and produces identical hypothetical outcomes.
5. Existing focused/unit coverage to extend or verify against:
   - `production_actions.rs`/`stock_actions.rs`/`transport_actions.rs` `#[cfg(test)]` blocks
   - Goldens — `golden_harvest_*.rs`, `golden_craft_*.rs`, `golden_partial_quantity.rs` (S127), `golden_merchant_*.rs`, `golden_stock_*.rs`, `golden_pick_up_*.rs`, `golden_steal_*.rs`. Enumerate during reassessment.
   - Conformance tests: `conformance_pick_up`, `conformance_put_down`, `conformance_harvest_noop_coverage_gap`, `conformance_craft_noop_coverage_gap` (currently dual-impl) at `planner_conformance.rs:616, 681, 744, 834`.
6. Recipe-driven multiplication: `register_harvest_actions` and `register_craft_actions` register N actions where N is the recipe count. The `EffectSchema` construction must be parameterized by recipe (each recipe has different inputs and outputs).
7. Bitwise-identical event-log invariant: every harvest/craft/stock/transport event must have identical `EventTag` and identical payload values pre- and post-ticket; the `partial_quantity` reporting must remain numerically identical.

## Architecture Check

1. The `Transfer`/`Consume`/`Produce`/`PartialOnFailure` step variants give the schema the expressive power to encode all production-and-movement semantics declaratively. The S127 partial-quantity feature becomes a structured `EffectFact` rather than a handler-internal `Option<Quantity>`, improving introspection (FND-29 — debuggability is a product feature).
2. Recipe-driven schema construction matches existing recipe-driven `ActionDef` construction — the same `RecipeRegistry` that informs the current handler body informs the schema literal. No new authoritative state is introduced.
3. The 8 explicit `PlannerTransitionKind` arms covered by this ticket's actions remain functional through ticket 010 — they continue to mutate `PlanningState` overlays the same way they do today. Ticket 010's deletion is correct because by then the planner reads schemas instead.

## Verification Layers

1. Bitwise-identical event-log invariant → event-log delta on `golden_harvest_*`, `golden_craft_*`, `golden_partial_quantity`, `golden_merchant_*`, `golden_stock_*`, `golden_pick_up_*`, `golden_steal_*`.
2. Partial-outcome invariant → action trace: harvest with insufficient source produces the same `partial_quantity` value (now surfaced as `EffectFact::PartialQuantity`) as today's handler.
3. Per-recipe schema invariant → focused unit/runtime test: every registered harvest/craft recipe has a non-empty schema; the schema's `EffectStep::Produce` step's commodity matches the recipe's output.
4. `PlannerTransitionKind` arm parity invariant → focused integration test: the existing planner path (`apply_hypothetical_transition` over `StoreStockIntoLocalFacility` etc.) still produces hypothetical outcomes that match the new schema's authoritative outcomes byte-for-byte. The 8 arms are not deleted in this ticket; they remain and are exercised by conformance tests until ticket 010.
5. Canonical state hash invariant → soak: identical `blake3` hashes on the three soak scenarios.

## What to Change

### 1. Construct per-recipe `EffectSchema` literals for harvest and craft

In `production_actions.rs`, replace each empty schema with a recipe-parameterized schema construction. Sketch for harvest:

```rust
EffectSchema {
    preconditions: vec![
        EffectPrecondition::CoLocated { actor, target: source },
        EffectPrecondition::QuantityAvailable { source, commodity: recipe.commodity, min: 1 },
        EffectPrecondition::ContentionGrantHeld { actor, affordance: source },
    ],
    steps: vec![
        EffectStep::PartialOnFailure {
            primary: vec![
                EffectStep::Transfer { source, dest: actor, commodity: recipe.commodity, quantity: recipe.requested },
                EffectStep::ConsumeContentionGrant { grant: source },
                EffectStep::EmitEvent { tag: EventTag::Harvest },
            ],
            fallback: vec![
                EffectStep::Transfer { source, dest: actor, commodity: recipe.commodity, quantity: /* available */ },
                EffectStep::ConsumeContentionGrant { grant: source },
                EffectStep::EmitEvent { tag: EventTag::HarvestPartial },
            ],
        },
    ],
}
```

Craft schema (analogous): preconditions on input ingredient quantities, steps consuming inputs and producing recipe output.

### 2. Construct stock-management `EffectSchema` literals

Each of store_stock, stage_stock_for_sale, collect_display_stock, unstage_stock encodes a specific commodity flow between agent possession and facility storage / display. Use `Transfer` + `EmitEvent` step chains.

### 3. Construct transport `EffectSchema` literals

pick_up, put_down, steal, drop_item all encode commodity transfer between actor possession and ground lot. Schema construction is analogous; `steal` adds a `ContentionGrantHeld` (or analog) precondition or a flag on the `EffectFact::EventEmitted { tag: EventTag::Theft }`.

### 4. Replace handler bodies with `apply_effects` delegation

Each `commit_*` handler in production/stock/transport shrinks to the standard delegation. Remove the imperative bodies entirely. The `CommitTraceData::Harvest` shape at `action_handler.rs:39–47` may need updating — confirm during reassessment whether `partial_quantity` is now derived from the `EffectFact::PartialQuantity` produced by the schema or whether the trace data shape stays the same with the field populated by the new path.

### 5. Surface `EffectFact::PartialQuantity` in commit-trace path

When the schema emits `EffectFact::PartialQuantity { requested, delivered }`, the existing trace consumer (`CommitTraceData::Harvest.partial_quantity`) reads it. Confirm during reassessment that the trace surface preserves the `Option<Quantity>` semantics (`None` when no partial, `Some(delivered)` otherwise) so existing partial-outcome goldens pass unchanged.

## Files to Touch

- `crates/worldwake-systems/src/production_actions.rs` (modify — per-recipe schemas for harvest and craft, commit handler body replacements)
- `crates/worldwake-systems/src/stock_actions.rs` (modify — 4 schemas)
- `crates/worldwake-systems/src/transport_actions.rs` (modify — 4+ schemas)
- `crates/worldwake-sim/src/effect_schema.rs` (modify if `EffectStep` needs new variants for production/stock/transport — likely covered by `Transfer`, `Consume`, `Produce`, `PartialOnFailure`, `EmitEvent`, `ConsumeContentionGrant` already)
- `crates/worldwake-sim/src/action_handler.rs` (modify — `CommitTraceData::Harvest` may need to derive `partial_quantity` from `EffectFact::PartialQuantity`; confirm during reassessment)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` and `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (modify if new sink methods are added)

## Out of Scope

- Migrating non-production/stock/transport actions (tickets 003, 004, 006–009).
- Switching the planner search to `apply_effects(..., Hypothetical)` (ticket 010).
- Deleting `apply_hypothetical_transition` arms or `PlannerTransitionKind` (ticket 010).
- Changing `BindingStrictness` or `RecipeRegistry` shape (preserved per spec Non-Goals).
- Conformance test rewrite (ticket 010).
- Trade actions (ticket 006 — even though trade involves commodity movement, it's a distinct domain with counterparty agreement).

## Acceptance Criteria

### Tests That Must Pass

1. All production/stock/transport-touching goldens (enumerate during reassessment, including `golden_partial_quantity.rs` for S127) produce bitwise-identical event logs.
2. Conformance tests `conformance_pick_up`, `conformance_put_down`, `conformance_harvest_noop_coverage_gap`, `conformance_craft_noop_coverage_gap` continue to pass — `apply_hypothetical_transition` arms are unchanged and the schema-driven authoritative path matches them byte-for-byte.
3. `cargo test -p worldwake-systems production stock transport` — existing inline tests pass.
4. `cargo test -p worldwake-ai golden_survival` — soak goldens produce identical canonical state hashes.
5. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Every registered harvest and craft recipe has a non-empty `EffectSchema` whose `Produce` step's commodity matches the recipe's output.
2. `CommitTraceData::Harvest.partial_quantity` retains its `Option<Quantity>` semantics post-ticket (numeric values preserved exactly).
3. The 8 `PlannerTransitionKind` arms (`PickUpGroundLot`, `StealGroundLot`, `PutDownGroundLot`, `StoreStockIntoLocalFacility`, `StageStoredStockForSale`, `CollectFacilityStockToPossession`, `UnstageDisplayedStock`, `ConsumeMatchingTargetCommodity`) still exist in `planner_ops.rs` after this ticket and produce hypothetical outcomes matching schema-driven authoritative outcomes byte-for-byte.
4. Bitwise-identical canonical state hash on the three soak scenarios.

## Test Plan

### New/Modified Tests

1. Inline tests in `production_actions.rs`, `stock_actions.rs`, `transport_actions.rs` extended to exercise the schema-driven path; add focused tests covering partial-outcome fallback (`PartialOnFailure`'s fallback steps fire when the primary step's preconditions fail mid-execution) and precondition-failure classification (e.g., pick_up with empty ground lot yields a specific `Discrepancy` variant).
2. Existing goldens — no source change.

### Commands

1. `cargo test -p worldwake-systems production stock transport`
2. `cargo test -p worldwake-ai golden_harvest golden_craft golden_partial_quantity`
3. `cargo test -p worldwake-ai conformance_pick_up conformance_put_down conformance_harvest conformance_craft`
4. `cargo test -p worldwake-ai golden_survival`
5. `./scripts/verify.sh`
