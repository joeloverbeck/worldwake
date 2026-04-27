# S127QUAAWAACQ-004: LastHarvestTrace component, belief-view accessor, decay integration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds `LastHarvestTrace` component to `worldwake-core`, registers it in `component_schema.rs`, adds `GoalBeliefView::last_harvest_trace` accessor, extends `item_decay_system` with retention-based pruning, adds `ScenarioDef.harvest_trace_retention_ticks` tunable, bumps `SAVE_FORMAT_VERSION`
**Deps**: None

## Problem

S127 introduces `LastHarvestTrace` (D5) — a bounded ring of recent harvest events on resource-source entities — so co-located agents can perceive contention pressure (heavily-picked orchard, spent well) directly through FND-14A perception, without a global event-log query. The trace is observable belief substrate, not a hidden tracker; agents who are not co-located learn through `ShareBelief` per existing channels. Decay (entries older than `HARVEST_TRACE_RETENTION_TICKS = 200`) piggybacks on the existing `item_decay_system` maintenance pass per FND-29A. This ticket also lands D9's first half (the `last_harvest_trace(entity)` belief-view accessor with FND-14A co-location gating) and D10's second slice (`ScenarioDef.harvest_trace_retention_ticks` for per-scenario tuning).

## Assumption Reassessment (2026-04-26)

1. No existing `LastHarvestTrace` type — `grep -rn "LastHarvestTrace" crates/` returns 0 matches.
2. `specs/S127-quantity-aware-acquisition.md` D5 prescribes the type shape, retention constant, and decay integration. D9 prescribes the belief-view accessor with FND-14A gating. D10 prescribes the `ScenarioDef` tunable.
3. Shared boundary: `Component` trait registration via `with_component_schema_entries!` macro in `crates/worldwake-core/src/component_schema.rs` (per `tickets/README.md` check #13, the macro generates code using bare type names that must be in scope at every expansion site — `delta.rs`, `world.rs`, `component_tables.rs`).
4. `crates/worldwake-systems/src/item_decay.rs:6-25` defines `item_decay_system` (confirmed during reassessment) — currently iterates resource-source-bearing places to prune `ItemLot` components. Extension point: add a parallel iteration for `LastHarvestTrace` entries, removing entries with `tick < current_tick - HARVEST_TRACE_RETENTION_TICKS`. The system already handles per-tick maintenance and has a `#[cfg(test)]` boundary at line 65.
5. **(corrected 2026-04-27)** Belief-view trait location. The ticket originally proposed `EntityBeliefView` mirroring `resource_source`. Live code disagrees: `resource_source` is declared on **`FacilityBeliefView`** (`belief_view.rs:1211`) and on the consumer-facing `GoalBeliefView` (`belief_view.rs:417`), with a manual `impl<T> GoalBeliefView for T where T: … + FacilityBeliefView + …` blanket impl forwarding (`belief_view.rs:1293+`). The new `last_harvest_trace` accessor follows the same shape — declared on both `FacilityBeliefView` and `GoalBeliefView`, forwarded in the blanket impl. The ticket text is updated below to say `FacilityBeliefView` instead of `EntityBeliefView`. There is no `impl_goal_belief_view!` macro — that prose in spec D9 refers to the manual blanket impl.
6. **(corrected 2026-04-27)** Test-mock proliferation. There are ~10 ad-hoc `FacilityBeliefView` test stubs across `worldwake-sim`, `worldwake-systems`, and `worldwake-ai` (e.g., `commodity_opportunity.rs:672`, `affordance_query.rs:989`, `tell_actions.rs:1456`, `planner_ops.rs:1546`, `ranking.rs:2916`, `feasibility_probe.rs:1282`, `exhaustion.rs:714`, `trade_valuation.rs:473`, `belief_view.rs:2600`). Adding `last_harvest_trace` as a *required* trait method would force changes to all of them. Instead, declare it with a default `None` impl (mirroring `stock_storage_policy` at `belief_view.rs:1207-1210` and `belief_view.rs:413-416`), since these stubs all model facilities that have no traces. `PerAgentBeliefView` overrides with the FND-14A co-located authoritative read; `PlanningState` keeps the `None` default for now (a later ticket — 007, ranking integration — will widen the planner snapshot when ranking actually reads the trace).
7. `crates/worldwake-cli/src/scenario/types.rs` defines `ScenarioDef` with existing `seed`, `compaction_interval`, `scenario_lint_overrides` tunables (confirmed during reassessment per S127 reassessment Agent 1's report). Adding `harvest_trace_retention_ticks: Option<u32>` follows the existing tunable pattern with `#[serde(default)]`.
8. **(corrected 2026-04-27)** Decay-system retention plumbing. To make the override readable from `item_decay_system` (which only receives a `SystemExecutionContext`), mirror the existing `World.commodity_decay` field (`world.rs:129, 149-155`): add `harvest_trace_retention_ticks: u32` on `World` initialized to `HARVEST_TRACE_RETENTION_TICKS`, plus public `harvest_trace_retention_ticks()` getter and `set_harvest_trace_retention_ticks(u32)` setter. `spawn_scenario_inner` resolves the override (or default) onto the world before `assemble_state`, exactly like `set_commodity_decay`.
9. `SAVE_FORMAT_VERSION` after ticket 003 is `50`; this ticket adds a new component to bincode-serialized world state and a new `World` field → bump to `51`.
10. Existing tests exercising `item_decay_system`: `waste_decays_at_threshold_tick`, `multi_commodity_selective_decay`, `no_decay_for_missing_commodity`, `decay_event_has_correct_tags`, `dispatch_table_routes_item_decay_system` (`item_decay.rs:130-330`). All operate on ground items; none touch `LastHarvestTrace`, so adding trace pruning is purely additive coverage.
11. **(corrected 2026-04-27)** Bounded-ring helper test placement. Ticket 006 owns the harvest-commit append site, but the bounded-ring cap is a property of the type itself; the helper `LastHarvestTrace::push` and its eviction test belong here so they ship with the type definition. No coordination with ticket 006 needed beyond the existence of `push`.
12. Adjacent contradictions: ticket 006 (D7 harvest commit) appends to `LastHarvestTrace`. This ticket lands the component; ticket 006 is a downstream consumer — no contradiction.

## Architecture Check

1. `LastHarvestTrace` is a new carrier of consequence (FND-5) — agents reasoning about whether a heavily-picked orchard is worth a trip can read it directly instead of guessing from `available_quantity` alone. Per FND-7, the trace is per-source state observable only by co-located agents through perception; off-place propagation goes through existing `ShareBelief` / report channels.
2. Decay piggybacks on `item_decay_system` rather than introducing a new tick — FND-29A append-only model, same maintenance pass, same scheduling. No new system function added.
3. The belief-view accessor's FND-14A co-location gating mirrors the existing `resource_source` accessor — symmetric perception surface.
4. Per-scenario `harvest_trace_retention_ticks` override is boundary-only (RON authoring); the runtime always reads the resolved constant or the override. No live authority shim.

## Verification Layers

1. `LastHarvestTrace` round-trips via bincode → focused unit test in `production.rs` `#[cfg(test)]`.
2. Component registration: `world.get_component_last_harvest_trace(entity)` and `world.set_component_last_harvest_trace(entity, …)` accessors are generated by the macro → focused test in component_schema or place-level test.
3. Belief-view accessor returns `Some(trace)` when agent is co-located, `None` otherwise → focused test in `belief_view.rs` mirroring the existing `resource_source` test pattern.
4. Decay prunes entries older than `current_tick - HARVEST_TRACE_RETENTION_TICKS` → focused unit test in `item_decay.rs` `#[cfg(test)]` constructing a `LastHarvestTrace` with mixed-age entries and asserting only stale ones are removed.
5. Bounded ring cap: appending a 9th entry drops the oldest by `tick` → focused unit test in `production.rs` (assuming append helper lives there; otherwise in the harvest commit module — to be confirmed during implementation, since this ticket only defines the type, ticket 006 owns the append site).
6. `ScenarioDef.harvest_trace_retention_ticks` override is read at scenario spawn → focused test loading a scenario with the override set.
7. Save format rejects version `50` saves → existing infrastructure.

## What to Change

### 1. Define `LastHarvestTrace` and `HarvestTraceEntry` in `crates/worldwake-core/src/production.rs`

Add per spec D5:

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LastHarvestTrace {
    pub entries: Vec<HarvestTraceEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HarvestTraceEntry {
    pub harvester: EntityId,
    pub tick: Tick,
    pub quantity: u16,
    pub partial: bool,
}

impl Component for LastHarvestTrace {}
```

Define a helper method `LastHarvestTrace::push(&mut self, entry: HarvestTraceEntry)` that enforces the bounded-ring cap (8 entries) by dropping the oldest by `tick` on overflow. Define the constant `pub const HARVEST_TRACE_RETENTION_TICKS: u32 = 200;` in `production.rs` (or a module-local constants file if one exists for this domain — confirm during implementation).

### 2. Register `LastHarvestTrace` in `component_schema.rs`

Add a `with_component_schema_entries!` entry for `LastHarvestTrace` so the macro generates `get_component_last_harvest_trace`, `set_component_last_harvest_trace`, and any related accessors. Follow the existing pattern for role-specific components on resource-source entities. Update macro-expansion-site imports (`delta.rs`, `world.rs`, `component_tables.rs`) per `tickets/README.md` check #13.

### 3. Add `GoalBeliefView::last_harvest_trace` accessor in `crates/worldwake-sim/src/belief_view.rs`

Add a default-`None` trait method on **both** `FacilityBeliefView` (line 1205+, mirroring `stock_storage_policy`'s placement) and `GoalBeliefView` (line 263+, mirroring `stock_storage_policy`/`resource_source`'s placement):

```rust
fn last_harvest_trace(&self, entity: EntityId) -> Option<LastHarvestTrace> {
    let _ = entity;
    None
}
```

Forward `GoalBeliefView::last_harvest_trace` to `FacilityBeliefView::last_harvest_trace` in the manual blanket impl `impl<T> GoalBeliefView for T where T: ... + FacilityBeliefView` (alongside the existing `resource_source` forward at `belief_view.rs:1598`). Implement on `PerAgentBeliefView` (`per_agent_belief_view.rs:1796+`) with FND-14A co-location gating: `if entity == self.agent || self.has_authoritative_local_visibility(entity) { world.get_component_last_harvest_trace(entity).cloned() } else { None }`. `PlanningState` and other trait stubs inherit the `None` default.

### 4. Extend `item_decay_system` in `crates/worldwake-systems/src/item_decay.rs`

Inside the existing tick handler, after the existing item-lot pruning, iterate over `world.iter_last_harvest_traces()` and prune `entries` whose `tick.0 < current_tick.0.saturating_sub(retention_ticks as u64)`. Read `retention_ticks` from `world.harvest_trace_retention_ticks()` (per the new `World` field documented in reassessment note 8). Mutate via a fresh `WorldTxn` writing the filtered trace through `txn.set_component_last_harvest_trace(entity, …)` only when the filter actually drops at least one entry — preserves the no-op-event invariant of the existing decay pass.

### 5. Add `ScenarioDef.harvest_trace_retention_ticks` in `crates/worldwake-cli/src/scenario/types.rs`

```rust
pub struct ScenarioDef {
    // … existing fields …
    #[serde(default)]
    pub harvest_trace_retention_ticks: Option<u32>,
}
```

Plumb the override through `spawn_scenario_inner` (`scenario/mod.rs:143+`): right after `world.set_commodity_decay(...)` (line 150), call `world.set_harvest_trace_retention_ticks(def.harvest_trace_retention_ticks.unwrap_or(HARVEST_TRACE_RETENTION_TICKS))`. Add `LastHarvestTrace` defaults to scenario-test `ScenarioDef` literals where they appear in `lints.rs` and `scenario/mod.rs` test code (these literals enumerate every field — they will not compile after the addition; `harvest_trace_retention_ticks: None` is the safe default). Note: `LintRule` enum, lint inventories, and the canonical lint set may need an update if the new field requires lint coverage; for this slice no lint is added.

### 6. Bump `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs:6` — bump from `50` (after ticket 003) to `51`.

### 7. Add focused tests

- `last_harvest_trace_bincode_roundtrip` in `production.rs`
- `last_harvest_trace_push_evicts_oldest` in `production.rs`
- `belief_view_last_harvest_trace_co_located_only` in `belief_view.rs`
- `item_decay_prunes_stale_harvest_trace` in `item_decay.rs`
- `scenario_def_harvest_trace_retention_override` in `scenario/mod.rs`

## Files to Touch

- `crates/worldwake-core/src/production.rs` (modify — add types, ring helper, retention constant, focused tests)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `LastHarvestTrace`, `HarvestTraceEntry`, `HARVEST_TRACE_RETENTION_TICKS`)
- `crates/worldwake-core/src/component_schema.rs` (modify — register `LastHarvestTrace`)
- `crates/worldwake-core/src/delta.rs` (modify — macro-expansion-site import per check #13)
- `crates/worldwake-core/src/world.rs` (modify — macro-expansion-site import; `harvest_trace_retention_ticks` field/getter/setter)
- `crates/worldwake-core/src/component_tables.rs` (modify — macro-expansion-site import per check #13)
- `crates/worldwake-sim/src/belief_view.rs` (modify — `FacilityBeliefView`/`GoalBeliefView` declaration with `None` default; blanket-impl forwarding)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — `last_harvest_trace` impl with FND-14A co-location gating; focused test)
- `crates/worldwake-systems/src/item_decay.rs` (modify — extend decay pass; focused test)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — `ScenarioDef.harvest_trace_retention_ticks`)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — plumb override into spawn; focused test; update test `ScenarioDef` literals)
- `crates/worldwake-cli/src/scenario/lints.rs` (modify — update test `ScenarioDef` literals for new field)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump SAVE_FORMAT_VERSION)

## Out of Scope

- Appending to `LastHarvestTrace` from the harvest commit handler — ticket 006.
- Reading `LastHarvestTrace` in candidate generation or ranking — ticket 007.
- `ResourceExtractionQueues` component — ticket 005.
- Decision-trace surfacing of `LastHarvestTrace` reads — out of scope; the trace is observable via component-level perception, not a separate trace channel.

## Acceptance Criteria

### Tests That Must Pass

1. `last_harvest_trace_bincode_roundtrip` — round-trip preserves all entries and fields.
2. `last_harvest_trace_push_evicts_oldest` — pushing a 9th entry drops the entry with the smallest `tick`.
3. `belief_view_last_harvest_trace_co_located_only` — non-co-located agent receives `None`.
4. `item_decay_prunes_stale_harvest_trace` — entries older than `current_tick - HARVEST_TRACE_RETENTION_TICKS` are removed; fresh entries are preserved.
5. `scenario_def_harvest_trace_retention_override` — override propagates to runtime decay.
6. Existing item-decay tests still pass (named during reassessment).
7. Existing suite: `cargo test --workspace`.

### Invariants

1. `LastHarvestTrace.entries.len() <= 8` always (bounded ring cap per spec D5).
2. Belief-view accessor returns `Some(_)` only when the requesting agent is co-located with the source (FND-14A).
3. `item_decay_system` is the sole pruning authority for `LastHarvestTrace.entries` — no other path mutates the vector (other than the harvest-commit append in ticket 006).
4. `SAVE_FORMAT_VERSION = 51`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/production.rs` `#[cfg(test)]` — bincode round-trip and ring-eviction tests.
2. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` — co-location gating test.
3. `crates/worldwake-systems/src/item_decay.rs` `#[cfg(test)]` — stale-entry pruning test.
4. `crates/worldwake-cli/src/scenario/mod.rs` `#[cfg(test)]` — override-propagation test.

### Commands

1. `cargo test -p worldwake-core last_harvest_trace`
2. `cargo test -p worldwake-sim belief_view_last_harvest_trace`
3. `cargo test -p worldwake-systems item_decay`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `scripts/verify.sh`

## Outcome

Completed on 2026-04-27.

- Landed `LastHarvestTrace` and `HarvestTraceEntry` in `crates/worldwake-core/src/production.rs` with bounded-ring `push` helper, `HARVEST_TRACE_RETENTION_TICKS = 200`, and `HARVEST_TRACE_MAX_ENTRIES = 8`. Re-exported from `lib.rs`.
- Registered `LastHarvestTrace` via `with_component_schema_entries!` for `EntityKind::Facility | Place`; updated macro-expansion-site imports in `delta.rs`, `world.rs`, `component_tables.rs`. Added a sample in `component_samples()` so `component_value_reports_matching_component_kind` covers the new variant.
- Added a default-`None` `last_harvest_trace(entity)` accessor on both `FacilityBeliefView` and `GoalBeliefView` in `crates/worldwake-sim/src/belief_view.rs`, with forwarding in the manual blanket impl. Implemented FND-14A co-located authoritative read on `PerAgentBeliefView` (`per_agent_belief_view.rs`).
- Extended `item_decay_system` in `crates/worldwake-systems/src/item_decay.rs` to prune entries older than `current_tick - World::harvest_trace_retention_ticks()`. Skips traces with no eligible eviction (preserves the no-op-event invariant).
- Added `World.harvest_trace_retention_ticks` field with public getter/setter (mirrors `commodity_decay`); plumbed `ScenarioDef.harvest_trace_retention_ticks: Option<u32>` through `spawn_scenario_inner`.
- Bumped `SAVE_FORMAT_VERSION` from 50 → 51.
- Added focused tests: `last_harvest_trace_bincode_roundtrip`, `last_harvest_trace_default_is_empty`, `last_harvest_trace_push_evicts_oldest_when_full`, `last_harvest_trace_push_under_capacity_appends`, `last_harvest_trace_trait_bounds` (production.rs); `belief_view_last_harvest_trace_co_located_only` (per_agent_belief_view.rs); `item_decay_prunes_stale_harvest_trace_entries`, `item_decay_skips_traces_without_stale_entries`, `item_decay_honors_world_level_retention_override` (item_decay.rs); `test_spawn_minimal_scenario_uses_default_harvest_trace_retention`, `test_spawn_scenario_applies_harvest_trace_retention_override` (scenario/mod.rs).

## Deviations

- Belief-view trait placement was corrected during reassessment from `EntityBeliefView` → `FacilityBeliefView` to match the live `resource_source` declaration site. Method has a default `None` impl in both `FacilityBeliefView` and `GoalBeliefView`, avoiding fallout across ~10 ad-hoc test mocks (`commodity_opportunity.rs`, `affordance_query.rs`, `tell_actions.rs`, `planner_ops.rs`, `ranking.rs`, `feasibility_probe.rs`, `exhaustion.rs`, `trade_valuation.rs`, `belief_view.rs`, `planning_state.rs`).
- Retention plumbing landed via a new `World.harvest_trace_retention_ticks` field rather than reading from a runtime config registry — mirrors the existing `commodity_decay` precedent. The override resolves to the static default (`HARVEST_TRACE_RETENTION_TICKS = 200`) when `ScenarioDef.harvest_trace_retention_ticks` is `None`.
- Files-to-touch grew during reassessment to include the macro-expansion sites' downstream consumers (`delta.rs` `component_samples` + inventory test, `lib.rs` re-exports, `lints.rs` and handler `ScenarioDef` literals across `worldwake-cli` and `worldwake-ai/tests/golden_survival_baseline.rs`, `scenario_coverage.rs` destructure pattern). All updates are mechanical fallout from the new persisted field.

## Verification Result

- Passed `cargo test -p worldwake-core last_harvest_trace` (5 tests)
- Passed `cargo test -p worldwake-sim belief_view_last_harvest_trace` (1 test)
- Passed `cargo test -p worldwake-systems --lib item_decay` (8 tests, including 3 new)
- Passed `cargo test -p worldwake-cli --lib spawn_scenario` (7 tests, including 2 new)
- Passed `cargo test --workspace` (no failures)
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `./scripts/verify.sh` (fmt + tests + check_active_goal_removed + clippy + clippy-strict + scenario-coverage)
