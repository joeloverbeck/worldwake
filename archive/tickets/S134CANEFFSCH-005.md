# S134CANEFFSCH-005: Production, stock, and transport schemas

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — replaces empty-placeholder schemas across production, stock, and transport actions with category-owned `EffectStep` variants interpreted by local authoritative sinks. Harvest partial quantities are emitted as `EffectFact::PartialQuantity` and converted back into the existing `CommitTraceData::Harvest.partial_quantity` surface.
**Deps**: archive/tickets/S134CANEFFSCH-001.md, archive/tickets/S134CANEFFSCH-002.md

## Problem

S134 deliverable D5 requires migrating the commodity-movement family — production (`harvest`, `craft`), stock management (`store_stock`, `stage_stock_for_sale`, `collect_display_stock`, `unstage_stock`), and transport (`pick_up`, `put_down`, `steal`, `drop_item`) — from empty placeholder schemas toward canonical `EffectSchema` evaluation.

The planner continues to use the old `apply_hypothetical_transition` path through ticket 010. This ticket owns the authoritative commit schema surface only.

## Assumption Reassessment (2026-05-05)

1. Production registrations live in `crates/worldwake-systems/src/production_actions.rs`, stock registrations in `crates/worldwake-systems/src/stock_actions.rs`, and transport registrations in `crates/worldwake-systems/src/transport_actions.rs`. All three files had `EffectSchema::empty()` placeholders after ticket 001.
2. The draft ticket overclaimed that generic `Transfer`/`Consume`/`Produce`/`PartialOnFailure` steps were sufficient. Live handler behavior includes resource-source depletion and harvest trace updates, craft job cleanup, stock assignment/listing changes, split-lot materialization, unique-item contention cleanup, and theft evidence. Those are concrete domain aftermaths, not generic commodity transfers.
3. The truthful boundary follows the already-landed S134 combat/needs pattern: add typed category-specific `EffectStep` variants and interpret them through category-owned authoritative sinks. This keeps the effect schema canonical at the action boundary without pretending the generic sink can express domain-specific ECS mutations.
4. `CommitTraceData::Harvest.partial_quantity` remains the public trace shape. The schema path emits `EffectFact::PartialQuantity { requested, delivered }`; the production delegation helper converts that typed fact into the existing trace field.
5. The planner hypothetical arms for stock and transport remain in place until S134CANEFFSCH-010. Category-specific steps intentionally reject in the generic hypothetical sink for now.
6. The drafted verification selectors were stale. `golden_craft` and `golden_partial_quantity` match no live AI tests; the live S127 witness is `golden_quantity_aware_acquisition::golden_partial_success_emits_partial_quantity`. `golden_harvest_to_consume` is an ignored test and must be run with `-- --ignored`.

## Architecture Check

1. Shared abstraction boundary: `EffectSchema` now names production, stock, and transport commit effects explicitly. The category sink owns how each typed step mutates authoritative ECS state.
2. Information path: no new planner privilege is introduced. Authoritative commit still uses scheduler transactions and existing action payload validation; planner mode remains on the old forward-model path until ticket 010.
3. Stored-vs-derived state: no persistent shape changes. `ActionDef.effect_schema` is registry-time data, so `SAVE_FORMAT_VERSION` is unchanged.
4. Outcome granularity: harvest partial delivery is now an `EffectFact`, while the existing trace field is preserved as a consumer-facing projection.

## What Changed

1. `crates/worldwake-sim/src/effect_schema.rs`
   - Added typed effect steps for `HarvestResource`, `FinishCraft`, stock assignment/listing actions, and transport pickup/drop/steal actions.
   - Extended `EffectSink` with default category methods that reject unsupported steps as `Discrepancy::ImproperPlanningState`.
   - Routed the new variants through `apply_step`; `HarvestResource` can emit `EffectFact::PartialQuantity`, and `PickUp` can return split-lot materialization data to the local transport sink.

2. `crates/worldwake-systems/src/production_actions.rs`
   - Registered non-empty harvest and craft schemas.
   - Replaced `commit_harvest` and `commit_craft` bodies with `apply_effects_with_context(...)` delegation through `ProductionEffectSink`.
   - Preserved resource-source depletion, extraction-slot release, craft-job cleanup, item-lot creation, and harvest trace semantics.

3. `crates/worldwake-systems/src/stock_actions.rs`
   - Registered non-empty schemas for `store_stock`, `collect_display_stock`, `stage_stock_for_sale`, and `unstage_stock`.
   - Replaced commit bodies with schema delegation through `StockEffectSink`.
   - Preserved stock assignment, display/listing, and possession/storage component mutations.

4. `crates/worldwake-systems/src/transport_actions.rs`
   - Registered non-empty schemas for `pick_up`, `put_down`, `drop_item`, and `steal`.
   - Replaced commit bodies with schema delegation through `TransportEffectSink`.
   - Preserved split-lot materialization, ground placement, unique-item contention cleanup, theft stock marker clearing, and theft evidence emission.

## Deviations From Draft

1. Generic `Transfer`/`Consume`/`Produce` steps were not used for this category because they do not express the live domain aftermath listed above.
2. `EffectStep::PartialOnFailure` was not used for harvest. The authoritative sink does not have a generic rollback/snapshot substrate, and the live partial-harvest behavior is an intentional domain branch. `HarvestResource` emits `EffectFact::PartialQuantity` directly.
3. The drafted command `cargo test -p worldwake-systems production stock transport` is not a valid Cargo selector shape. The focused system tests were run as separate filters.
4. The planner parity invariant remains deferred to S134CANEFFSCH-010. This ticket keeps existing hypothetical arms alive and only migrates authoritative commit effects.

## Files Touched

- `crates/worldwake-sim/src/effect_schema.rs`
- `crates/worldwake-systems/src/production_actions.rs`
- `crates/worldwake-systems/src/stock_actions.rs`
- `crates/worldwake-systems/src/transport_actions.rs`
- `archive/specs/S134-canonical-effect-schema.md`
- `archive/tickets/S134CANEFFSCH-010.md`

## Out of Scope

- Switching planner search to `apply_effects(..., Hypothetical)` (S134CANEFFSCH-010).
- Deleting `apply_hypothetical_transition`, `PlannerTransitionKind`, or old planner conformance tests (S134CANEFFSCH-010).
- Migrating trade, queue, escort, social, patrol, office, or artifact actions (sibling tickets).
- Save-format changes.

## Verification Result

Passed:

1. `cargo test -p worldwake-systems --lib --no-run`
2. `cargo test -p worldwake-systems production`
3. `cargo test -p worldwake-systems stock`
4. `cargo test -p worldwake-systems transport`
5. `cargo test -p worldwake-systems`
6. `cargo test -p worldwake-ai --test golden_simulation_gaps golden_harvest_to_consume -- --ignored`
7. `cargo test -p worldwake-ai --test golden_quantity_aware_acquisition golden_partial_success_emits_partial_quantity`
8. `cargo test -p worldwake-ai --test planner_conformance`
9. `cargo test -p worldwake-ai golden_survival`
10. `cargo clippy --workspace --all-targets -- -D warnings`
11. `./scripts/verify.sh`

Selector reassessment:

- `cargo test -p worldwake-ai golden_craft -- --list` found no live matching tests.
- `cargo test -p worldwake-ai golden_partial_quantity -- --list` found no live matching tests.
- `cargo test -p worldwake-ai --test planner_conformance conformance_pick_up -- --list`, `conformance_put_down`, `conformance_harvest`, and `conformance_craft` all resolved inside the live `planner_conformance` target; the full target passed.

`./scripts/verify.sh` completed the repo gate: format check, workspace tests, active-goal removal check, both clippy lanes including `--all-targets -- -D warnings`, and scenario coverage check.

## Outcome

Completed on 2026-05-05.

- Landed production, stock, and transport authoritative effect schemas using category-owned `EffectStep` variants and local authoritative sinks.
- Preserved the existing production, stock, and transport commit semantics while routing commits through `apply_effects_with_context(...)`.
- Preserved harvest partial trace projection by emitting `EffectFact::PartialQuantity` from `HarvestResource` and converting it back into `CommitTraceData::Harvest.partial_quantity`.
- Corrected the S134 spec and S134CANEFFSCH-010 handoff so the planner-switch ticket owns hypothetical parity and does not assume this slice used generic `PartialOnFailure`.
- Verified with focused systems/AI commands, CI-shaped clippy, and `./scripts/verify.sh` as recorded above.
