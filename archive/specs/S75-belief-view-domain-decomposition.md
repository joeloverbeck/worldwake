# S75: Belief View Domain Decomposition

## Summary

Decompose the monolithic `RuntimeBeliefView` trait (113 methods spanning 11 domains) and its `GoalBeliefView` subset (92 methods) into domain-specific sub-traits composed via a supertrait. Restructure `SnapshotEntity` into matching domain sub-structs. This reduces the shotgun surgery pattern (currently 4 files, 2 crates minimum per new belief surface method) and enables test mocks to implement only the domains they exercise.

## Phase

Phase 7: Consequence Carriers (infrastructure refactor — no dependency on or from other Phase 7 specs)

## Status

COMPLETED

## Crates

- `worldwake-sim` (trait definitions, `PerAgentBeliefView` implementation)
- `worldwake-ai` (PlanningState, PlanningSnapshot implementations, test mocks)

## Dependencies

- None. Pure structural refactor with no behavioral changes.

## Design Goals

- **Reduce shotgun surgery**: Adding a new belief surface method should require changes only to the relevant domain sub-trait and its implementors, not the entire 113-method surface.
- **Preserve all existing call sites**: The 128 `&dyn RuntimeBeliefView` occurrences across 27 source files remain unchanged — `RuntimeBeliefView` becomes a supertrait composing all domain sub-traits.
- **Improve test mock ergonomics**: Test mocks implement only the sub-traits they need, with `unimplemented!()` defaults or blanket impls for unused domains.
- **Mirror sub-traits in SnapshotEntity**: Domain sub-structs in `SnapshotEntity` correspond 1:1 to sub-traits, making the projection relationship explicit and auditable.
- **Incremental migration**: Sub-traits can be extracted one domain at a time, with each step producing a compilable intermediate state.

## Non-Goals

- Changing any belief-view semantics or adding/removing methods.
- Introducing new ECS components, goal kinds, or actions.
- Changing the `GoalBeliefView` → `RuntimeBeliefView` delegation pattern.
- Performance optimization — this is a structural change only. The supertrait approach preserves the existing single-vtable dispatch.
- Splitting the trait across multiple files or modules (that is a possible follow-on but not in scope here).

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P12 (Performance ≠ Causality) | Supertrait composition preserves single `&dyn RuntimeBeliefView` dispatch — no additional vtable indirection. No causal paths change. |
| P14 (World State ≠ Belief State) | Each sub-trait inherits the belief-only contract. No sub-trait may expose authoritative world state. |
| P26 (Systems Through State) | No cross-system calls introduced. Sub-traits are query-only interfaces, same as today. |
| P28 (No Backward Compat) | The monolithic trait is replaced, not wrapped. No compatibility layer or alias. |

## FND-01 Section H

Not applicable. This spec changes no causal hooks, information paths, stored state, or world semantics. It restructures trait definitions and struct layout only. All existing tests must pass without modification (modulo import path changes for sub-trait names).

## Deliverables

### 1. Domain Sub-Traits

Eleven sub-traits extracted from `RuntimeBeliefView`. Each sub-trait is defined in `worldwake-sim/src/belief_view.rs` (or a new `belief_view/` module if the file grows unwieldy — a follow-on decision).

| Sub-Trait | Methods | Key Methods |
|-----------|---------|-------------|
| `EntityBeliefView` | 8 | `is_alive`, `is_dead`, `is_incapacitated`, `entity_kind`, `corpse_entities_at`, `bandit_flee_wound_threshold`, `bandit_camp_establishment_ticks`, `locally_observed_is_dead` |
| `SpatialBeliefView` | 12 | `effective_place`, `is_in_transit`, `in_transit_state`, `entities_at`, `locally_observed_entities_at`, `adjacent_places`, `adjacent_places_with_travel_ticks`, `place_has_tag`, `place_has_any_tag_in`, `route_exists`, `patrol_route`, `route_experience` |
| `InventoryBeliefView` | 13 | `direct_possessions`, `commodity_quantity`, `locally_observed_commodity_quantity`, `item_lot_commodity`, `item_lot_consumable_profile`, `direct_container`, `direct_possessor`, `carry_capacity`, `load_of_entity`, `knows_recipe`, `recipe_definition`, `unique_item_count`, `known_recipes` |
| `CombatBeliefView` | 10 | `combat_profile`, `courage`, `consultation_speed_factor`, `wounds`, `hostile_targets_of`, `visible_hostiles_for`, `current_attackers_of`, `patrol_profile`, `pursuit_profile`, `has_wounds` |
| `SocialBeliefView` | 18 | `agent_belief_store`, `known_entity_beliefs`, `known_social_observations`, `believed_activity_of`, `agents_active_at`, `tell_profile`, `told_belief_memories`, `told_belief_memory`, `recipient_knowledge_status`, `ask_witness_memory`, `belief_confidence_policy`, `observation_fidelity`, `source_reliability`, `expectation_store`, `last_seen_memory`, `epistemic_disposition_profile`, `theft_disposition_profile`, `intention_disposition_profile` |
| `EconomicBeliefView` | 9 | `trade_disposition_profile`, `commodity_valuation_profile`, `controlled_commodity_quantity_at_place`, `local_controlled_lots_for`, `listed_sale_lots_at`, `seller_for_sale_lot`, `has_sale_listing`, `demand_memory`, `merchandise_profile` |
| `PoliticalBeliefView` | 18 | `known_institutional_beliefs`, `factions_of`, `bandit_factions_of`, `locally_observed_bandit_camp_faction_at`, `violation_disposition_profile`, `active_violation_records`, `record_data`, `office_data`, `believed_office_holder`, `believed_force_controller`, `believed_membership`, `believed_faction_rally_point`, `offices_contested_by`, `loyalty_to`, `believed_support_declaration`, `believed_support_declarations_for_office`, `institutional_belief_claims`, `justice_disposition_profile` |
| `TemporalBeliefView` | 10 | `current_tick`, `has_contention_policy`, `facility_queue_position`, `facility_grant`, `contention_queue_is_full`, `facility_queue_join_tick`, `facility_queue_patience_ticks`, `reservation_conflicts`, `reservation_ranges`, `estimate_duration` |
| `ProfileBeliefView` | 5 | `homeostatic_needs`, `drive_thresholds`, `metabolism_profile`, `preference_profile`, `utility_profile` |
| `FacilityBeliefView` | 6 | `workstation_tag`, `stock_storage_policy`, `resource_source`, `has_production_job`, `matching_workstations_at`, `resource_sources_at` |
| `ControlBeliefView` | 4 | `believed_owner_of`, `believed_rights`, `can_control`, `has_control` |

**Note**: Some categorization boundaries are debatable (e.g., `patrol_profile` could be Combat or Profile; `justice_disposition_profile` could be Social or Political). The ticket decomposition phase should finalize these assignments based on which consumers actually call each method together.

### 2. RuntimeBeliefView Supertrait

```rust
pub trait RuntimeBeliefView:
    EntityBeliefView
    + SpatialBeliefView
    + InventoryBeliefView
    + CombatBeliefView
    + SocialBeliefView
    + EconomicBeliefView
    + PoliticalBeliefView
    + TemporalBeliefView
    + ProfileBeliefView
    + FacilityBeliefView
    + ControlBeliefView
{
    // No additional methods — RuntimeBeliefView is purely compositional.
}
```

All existing `&dyn RuntimeBeliefView` call sites (128 occurrences across 27 source files) continue to work unchanged. Rust's vtable for a supertrait composed of sub-traits includes all methods from all sub-traits in a single dispatch table when used via `&dyn RuntimeBeliefView`. Trait upcasting (e.g., `&dyn RuntimeBeliefView` → `&dyn SpatialBeliefView`) is supported on the project's Rust 1.93.0 toolchain if needed post-refactor.

### 3. GoalBeliefView Supertrait

`GoalBeliefView` is a planning-time subset of `RuntimeBeliefView` (92 of 113 methods). It deliberately excludes queue/reservation helpers, duration estimation, and broader affordance/runtime helpers. After decomposition, `GoalBeliefView` should compose a planning-relevant subset of the same domain sub-traits. The exact sub-trait composition will be determined during ticket decomposition by auditing which GoalBeliefView methods map to which sub-traits — it likely includes all sub-traits except portions of `TemporalBeliefView` and `FacilityBeliefView`. The existing `impl_goal_belief_view!` macro (defined at `belief_view.rs:745`) continues to generate the delegation from `GoalBeliefView` methods to `RuntimeBeliefView` methods.

### 4. SnapshotEntity Domain Sub-Structs

`SnapshotEntity` fields are reorganized into domain sub-structs mirroring the sub-traits:

```rust
pub struct SnapshotEntity {
    pub entity: SnapshotEntityCore,      // EntityBeliefView fields (incl. alive/dead/incapacitated from former SnapshotLifecycle)
    pub spatial: SnapshotSpatial,        // SpatialBeliefView fields
    pub inventory: SnapshotInventory,    // InventoryBeliefView fields
    pub combat: SnapshotCombat,          // CombatBeliefView fields
    pub social: SnapshotSocial,          // SocialBeliefView fields (if cached)
    pub economic: SnapshotEconomic,      // EconomicBeliefView fields
    pub political: SnapshotPolitical,    // PoliticalBeliefView fields
    pub temporal: SnapshotTemporal,      // TemporalBeliefView fields
    pub profiles: SnapshotProfiles,      // ProfileBeliefView fields
    pub facility: SnapshotFacility,      // FacilityBeliefView fields (incl. has_production_job from former SnapshotActionFlags)
    pub control: SnapshotControl,        // ControlBeliefView fields (incl. controllable_by_actor, has_control from former SnapshotActionFlags)
}
```

The current `SnapshotActionFlags` (3 fields: `has_production_job`, `controllable_by_actor`, `has_control`) and `SnapshotLifecycle` (3 fields: `alive`, `dead`, `incapacitated`) are dissolved into their respective domain sub-structs to maintain 1:1 correspondence between sub-traits and sub-structs. Each sub-struct's fields correspond to the methods of its matching sub-trait, making the RuntimeBeliefView ↔ SnapshotEntity projection relationship explicit and auditable.

### 5. impl_goal_belief_view! Macro Update

The existing macro generates `GoalBeliefView` → `RuntimeBeliefView` delegation. After decomposition, it should generate delegation per sub-trait. The generated code is mechanically identical; only the organizational grouping changes.

### 6. Test Mock Simplification

Test mocks currently must implement all 113 methods (2 test files: `search/tests.rs` and `agent_tick/tests.rs` in worldwake-ai). After decomposition, mocks can:
- Implement only the sub-traits their test exercises
- Use a helper macro or blanket impl that provides `unimplemented!()` for all sub-trait methods
- Selectively override specific domain sub-traits with real implementations

## Cross-System Interactions

None new. This is a structural change to trait organization. All cross-system interactions remain state-mediated (P26) via the same belief-view query interface.

## Migration Strategy

The decomposition can proceed incrementally, one domain sub-trait at a time:

1. Extract one sub-trait (e.g., `CombatBeliefView` — smallest at 10 methods)
2. Move methods from `RuntimeBeliefView` to the sub-trait
3. Add the sub-trait as a supertrait bound on `RuntimeBeliefView`
4. Implement the sub-trait for `PerAgentBeliefView` and `PlanningState`
5. Verify all tests pass
6. Repeat for the next domain

This produces a compilable intermediate state at every step. The `RuntimeBeliefView` supertrait grows one bound per extraction, while the monolithic method list shrinks.

**Blast radius**: ~20 files import `RuntimeBeliefView` or `GoalBeliefView` directly and will need updated import paths when sub-trait names are introduced. The 128 `&dyn RuntimeBeliefView` occurrences across 27 source files remain unchanged — they dispatch through the supertrait as before.

## Stored State vs. Derived Read-Model

No change. Sub-traits are query interfaces over existing state. No new stored state is introduced. The SnapshotEntity sub-struct reorganization is a layout change, not a semantic change.

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Rust trait object coherence — `dyn RuntimeBeliefView` with many supertraits | Rust supports this; the supertrait compiles to a single vtable. Verify with a prototype extraction of one sub-trait. |
| GOAP search performance — SnapshotEntity sub-struct field access adds indirection | Profile before and after. Sub-struct fields are still inline (not heap-allocated). Cache locality impact should be negligible for struct-of-structs. |
| Method categorization disagreement | Finalize during ticket decomposition. Some methods may move between domains. The important constraint is: no method appears in two sub-traits. |
| Macro complexity — `impl_goal_belief_view!` may need restructuring | The macro already generates mechanical delegation. Grouping by sub-trait is additive complexity, not multiplicative. |

## Outcome

Completed on 2026-04-09.

Implemented the staged `RuntimeBeliefView` domain decomposition across tickets `S75BELVDECOM-001` through `S75BELVDECOM-008`, including domain sub-traits, supertrait composition, `SnapshotEntity` domain sub-structs, and the final `GoalBeliefView` cleanup.

Deviation from the original draft: the finished implementation preserved `GoalBeliefView` as the stable AI-facing facade and removed `impl_goal_belief_view!` in favor of blanket impls plus goal-only bridge traits, instead of keeping the old macro delegation pattern.

Verification passed with:
- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
