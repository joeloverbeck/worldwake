# S143: Static Belief-View Trait Separation

**Status**: Draft

## Summary

FND-14 says world state and belief state are separate layers, with FND-14A allowing same-tick co-located *physical* perception to read authoritative state because a correct perception pipeline would deliver those facts on the same tick anyway. The external assessment in `reports/ai-architecture-improvements.md` calls FND-14A "the single biggest discipline risk" because today's enforcement is comments + runtime co-location assertions, not the type system. The S75 belief-view decomposition (archived) split `RuntimeBeliefView` into 11 domain sub-traits, but observation reads (`locally_observed_*`) and belief reads (`believed_*`) coexist on the same sub-traits — `ControlBeliefView` (`crates/worldwake-sim/src/belief_view.rs:779`) puts ownership/jurisdiction/rights reads next to control reads, and `SpatialBeliefView` (line 905) puts co-located physical observation next to off-place belief reads.

S143 lands a strict static separation. The current 11 sub-traits are refactored so that physical observation accessors live exclusively on `LocalPhysicalObservationView`, authority/relational accessors (ownership, custody, access rights, jurisdiction, office-holder identity) live exclusively on `BelievedAuthorityView`, and authoritative omniscient accessors (currently only reachable via test/observer tooling) are cordoned off in a `#[cfg(any(debug_assertions, test))]`-gated `DebugWorldView` trait. The AI planner crate (`worldwake-ai`) imports only the belief-domain sub-traits and `LocalPhysicalObservationView`; it cannot import the debug trait. This makes a class of FND-14A widening bugs — "I'll just add `believed_jurisdiction` to the observation view" — a compile error rather than a review failure.

This is a correctness refactor; it preserves all current legal reads and changes no live semantic behavior. The behavioral surface only changes when an illegal read is attempted (compile error post-S143; runtime assertion or silent passthrough pre-S143).

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-core` — defines the new `EntityState` debug snapshot struct. `EntityBeliefAspect` and underlying belief-store types are unchanged.
- `worldwake-sim` — defines the new generic `BeliefRead<T>` enum (the unified epistemic-read wrapper) next to the live `BeliefValue<T>` owner, and owns the trait redefinitions in `belief_view.rs`. Existing trait methods are partitioned into new traits (`LocalPhysicalObservationView`, `BelievedAuthorityView`) per the D3 audit table; `RuntimeBeliefView` becomes a thin supertrait composition. The existing `SocialBeliefView` (testimony/rumor/source-reliability — `belief_view.rs:1116`) is untouched; the new `BelievedAuthorityView` covers a disjoint surface.
- `worldwake-ai` — every planner-side `use` site that touches a belief-view trait is examined. Most sites already import `RuntimeBeliefView`; that import becomes a narrower set determined by what the call site actually reads. `worldwake-ai` library code cannot import `DebugWorldView`.
- `worldwake-systems` — strict-narrowing rule applies only to perception-write modules (`crates/worldwake-systems/src/perception.rs` and any future perception-write modules), which import `LocalPhysicalObservationView` for co-location resolution. Action handler modules (e.g., `justice_actions.rs`, `investigate_actions.rs`, `office_actions.rs`, `tell_actions.rs`, `trade_actions.rs`, `report_actions.rs`, `escort_actions.rs`, `consult_record_actions.rs`, `ask_about_person_actions.rs`, `production_actions.rs`, `epistemic_actions.rs`, `bandit_camp_actions.rs`, `patrol_actions.rs`, `search_actions.rs`, `artifact_actions.rs`) legitimately import `RuntimeBeliefView` and domain sub-traits because they perform legality, ownership, and social-state checks while committing actions — no narrowing is enforced on these handlers.
- `worldwake-cli` — observer/test tooling imports `DebugWorldView` gated by `#[cfg(any(debug_assertions, test))]`. Live release-mode CLI handlers may not import it.
- `worldwake-visualizer` — debug/diagnostic tool that consumes `PerAgentBeliefView` and `&World` directly today (`crates/worldwake-visualizer/src/snapshot.rs:13`). Imports `DebugWorldView` under the same cfg-gate as the observer.

## Dependencies

- S75 (Belief View Domain Decomposition, archived at `archive/specs/S75-belief-view-domain-decomposition.md`) — provides the 11-trait substrate that this spec re-partitions. Hard dependency satisfied.
- S109 (Typed Discrepancy Taxonomy, archived at `archive/specs/S109-typed-discrepancy-taxonomy.md`) — `Discrepancy::MissingObservation` (`crates/worldwake-core/src/discrepancy.rs:19`) continues to be the lawful failure mode when a belief read returns absent.
- S136 (Decision Event Payload Extension, archived at `archive/specs/S136-decision-event-payload-extension.md`) — decision-event payloads continue to carry the belief-read provenance unchanged.

## Design Goals

1. **Illegal reads are compile errors.** A planner module attempting to read `believed_owner_of` from a trait it didn't import will not link. The current FND-14A "review/test" enforcement is replaced by type-system enforcement for the legal-belief vs. legal-physical-observation split.
2. **Three orthogonal views.** `LocalPhysicalObservationView` for same-tick co-located physical perception; `BelievedAuthorityView` for ownership/custody/rights/jurisdiction/office (always belief-backed, FND-24 surface); `{Domain}BeliefView` (the existing 11 domain sub-traits, including `SocialBeliefView` for testimony/rumor/source-reliability) for off-place belief reads in their respective domains. No method appears on more than one of these traits.
3. **No legitimate read becomes harder to write.** Every current legal call site continues to work after updating its `use` line.
4. **Debug accessors cordoned.** `DebugWorldView` is `#[cfg(any(debug_assertions, test))]`-gated. Observer rendering, golden harnesses, CLI debug commands, and the visualizer import it; `worldwake-ai` library code cannot. This is a *labeling* discipline — `&World` accessors remain reachable from `worldwake-ai` today as they always were, but the labeled `DebugWorldView` trait gives future debug-only accessors a parking surface that won't accidentally land on the planner's import path.
5. **No new runtime cost.** All separations are at the trait surface; impls share the same body code.
6. **Forward-compatible authority surface.** `BelievedAuthorityView` covers FND-24's enumerated relational domains (ownership, custody, access, jurisdiction, office). Future authority-belief methods (e.g., debts, contracts, succession claims) land on this trait as those domains are modeled.

## Non-Goals

- **No change to belief-store data layout.** `AgentBeliefStore` field set is unchanged.
- **No change to perception output.** What gets observed and stored is determined by the perception pipeline, not by the trait split.
- **No new ECS component.** This spec is a Rust trait refactor.
- **No relaxation of FND-14A's narrow exception.** Same-tick co-located physical reads remain legal exactly where they are legal today.
- **No introduction of `DebtClaim` or other future-domain types.** Authority-belief surfaces beyond ownership/custody/rights/jurisdiction/office land in later specs as their domains are simulated. `BelievedAuthorityView` is forward-extensible but ships lean.
- **No rename of the existing 11 `*BeliefView` sub-traits.** The existing suffix-form names stay; the new traits use prefix-form names (`Believed*View`, `LocalPhysicalObservationView`, `DebugWorldView`) to signal "this is the FND-14/14A-split surface, not a legacy domain trait."

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-14 (World State Is Not Belief State) | The trait surface for "world state read" is now physically separate from the trait surface for "belief read." Planner code cannot accidentally read world state for an off-place entity. |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent; Social Facts Are Not) | The legal world read (co-located physical) is on `LocalPhysicalObservationView`; authority/relational reads (ownership, rights, jurisdiction) are on `BelievedAuthorityView` and have no authoritative-state path. Widening FND-14A becomes a compile error. |
| FND-24 (Ownership, Custody, Access, Obligation, and Jurisdiction Are Distinct) | `BelievedAuthorityView` is the typed surface for these five domains, ensuring agents reason about them only through belief. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | The planner and perception systems share the same belief-store state through narrower, role-specific traits. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | No legacy wrapper traits remain; `RuntimeBeliefView` becomes a thin supertrait alias, and impls are updated in place. |
| FND-29 (Debuggability Is a Product Feature) | `DebugWorldView` provides observer/test/visualizer tooling its needed accessors without leaking them into live planner code. |

## Deliverables

### D1: Read shape types and `LocalPhysicalObservationView` trait

Define `BeliefRead<T>`, `ObservedRead<T>`, and `ObservationSource` in `worldwake-sim` beside the live `BeliefValue<T>` owner; define `EntityState` in `worldwake-core`; and define one new trait in `worldwake-sim` (the physical observation surface).

```rust
// crates/worldwake-sim/src/belief_view.rs

/// Unified epistemic-read wrapper. All belief-view methods returning a single
/// belief-backed value use this type. Symmetrical with ObservedRead<T> below
/// for physical observation reads.
pub enum BeliefRead<T> {
    /// Agent has no belief on the subject.
    Unknown,
    /// Agent currently believes T; carries provenance, confidence, and
    /// acquisition tick via the wrapped BeliefValue.
    Known(BeliefValue<T>),
    /// Agent's last belief on the subject is past its freshness horizon.
    /// The value is still surfaced so callers can choose to use a stale
    /// belief (FND-16 first-class stale).
    Stale(BeliefValue<T>),
}

/// Same-tick co-located physical observation wrapper. The agent is currently
/// co-located with the subject; the read is FND-14A-legal.
pub struct ObservedRead<T> {
    pub value: T,
    pub observed_tick: Tick,
    pub source: ObservationSource,
}

pub enum ObservationSource {
    /// FND-14A: same-tick co-located physical observation. The authoritative
    /// world state is the fact the belief store would hold an instant later.
    CoLocatedSameTick,
    /// The belief store's most recent snapshot of the subject. Used when an
    /// observation pipeline has run and persisted the result.
    BeliefStoreSnapshot,
}
```

```rust
// crates/worldwake-sim/src/belief_view.rs
pub trait LocalPhysicalObservationView {
    fn colocated_entities(&self, actor: EntityId) -> ObservedRead<Vec<EntityId>>;
    fn observed_item_lot_quantity(&self, lot: EntityId) -> ObservedRead<Option<Quantity>>;
    fn observed_workstation_tag(&self, entity: EntityId) -> ObservedRead<Option<WorkstationTag>>;
    fn observed_resource_source(&self, entity: EntityId) -> ObservedRead<Option<ResourceSource>>;
    fn observed_container_contents(&self, container: EntityId) -> ObservedRead<Vec<EntityId>>;
    fn observed_entity_kind(&self, entity: EntityId) -> ObservedRead<Option<EntityKind>>;
}
```

Method-origin classification (audit-table source — see D3 for the cross-trait audit and the migration vs. net-new disposition for each method):

| Method | Origin | Notes |
|--------|--------|-------|
| `colocated_entities` | Migrated from `SpatialBeliefView::locally_observed_entities_at` (`belief_view.rs:909`) and `EntityBeliefView::locally_observed_entities_at` (`belief_view.rs:791`); renamed. | Two existing sites with identical semantics consolidate onto one method. |
| `observed_item_lot_quantity` | Net-new. | Existing `InventoryBeliefView::locally_observed_commodity_quantity` (`belief_view.rs:1030`) is per-(agent, holder, kind) and stays on `InventoryBeliefView` as a belief-backed accessor. The lot-shaped variant on the observation trait is fresh. |
| `observed_workstation_tag` | Net-new. | No existing co-located equivalent; today the planner reads `FacilityBeliefView::workstation_tag` directly. |
| `observed_resource_source` | Net-new. | Today the planner reads `FacilityBeliefView::resource_source` directly. The observation variant returns the existing `ResourceSource` core type wrapped in `ObservedRead`. |
| `observed_container_contents` | Net-new. | Today the planner reads `InventoryBeliefView::direct_possessions` / `direct_container` (`belief_view.rs:394, 435`) directly. |
| `observed_entity_kind` | Net-new. | Today the planner reads `EntityBeliefView::entity_kind` directly. |

`ObservationSource` distinguishes `CoLocatedSameTick` (the FND-14A path) from `BeliefStoreSnapshot` (the persistent belief path). Each impl returns the actual source so observer rendering and decision traces can attribute reads.

### D2: `BelievedAuthorityView` trait

```rust
pub trait BelievedAuthorityView {
    fn believed_owner_of(&self, entity: EntityId) -> BeliefRead<EntityId>;
    fn believed_holder_of(&self, entity: EntityId) -> BeliefRead<EntityId>;
    fn believed_access_right(&self, actor: EntityId, target: EntityId) -> BeliefRead<EffectiveRight>;
    fn believed_jurisdiction(&self, place: EntityId) -> BeliefRead<EntityId>;
    fn believed_office_holder(&self, office: EntityId) -> BeliefRead<EntityId>;
}
```

Method-origin classification:

| Method | Origin | Notes |
|--------|--------|-------|
| `believed_owner_of` | Migrated from `ControlBeliefView::believed_owner_of` (`belief_view.rs:780`). | Signature changes return type from current shape to the unified `BeliefRead<EntityId>`. The current method also lives on the goal-side `RuntimeBeliefView` impl. |
| `believed_holder_of` | Net-new. | No current method named `believed_holder_of`. Existing `direct_possessor`/`direct_container` (`belief_view.rs:435–436`) read authoritative state and stay on `InventoryBeliefView` for the FND-14A-legal co-located case. The belief-backed variant on the authority trait is fresh. |
| `believed_access_right` | Net-new. | Returns the existing `EffectiveRight` core type (`crates/worldwake-core/src/rights.rs:15`) wrapped in `BeliefRead`. |
| `believed_jurisdiction` | Net-new. | No current method named `believed_jurisdiction`. Jurisdiction is presently surfaced as `record_data` / `office_data` on `PoliticalBeliefView`; the dedicated belief-backed accessor here makes the FND-14A wall explicit for jurisdiction reads. |
| `believed_office_holder` | Migrated from `PoliticalBeliefView::believed_office_holder` (`belief_view.rs:1306`). | Current method returns `InstitutionalBeliefRead<Option<EntityId>>`. The migration converts to `BeliefRead<EntityId>`; `Option<T>` collapses into `BeliefRead::Unknown` for absent holders, and `Conflicted(Vec<T>)` from the institutional variant has no analog on `BelievedAuthorityView` because authority-belief callers should treat contradiction as `Unknown` for legality purposes (planner can still consult the underlying institutional read via `PoliticalBeliefView` if it needs the conflict surface). |

All methods return `BeliefRead<T>`. No method on this trait may consult authoritative world state, even for co-located entities — co-location does not tell you who owns the chest. The existing `SocialBeliefView` (`belief_view.rs:1116`, 30 methods covering testimony, rumor, source-reliability, known social observations) is untouched by S143 and continues to expose its current accessor surface; its scope (epistemic propagation) is disjoint from `BelievedAuthorityView`'s scope (relational/institutional normativity).

### D3: Domain sub-trait re-partition and audit table

The 11 existing sub-traits (`ControlBeliefView`, `EntityBeliefView`, `ProfileBeliefView`, `SpatialBeliefView`, `TemporalBeliefView`, `InventoryBeliefView`, `CombatBeliefView`, `EconomicBeliefView`, `SocialBeliefView`, `PoliticalBeliefView`, `FacilityBeliefView`) keep their names. Each sub-trait either retains a method, or hands it off to `LocalPhysicalObservationView` (D1) or `BelievedAuthorityView` (D2). The full per-method audit table is delivered inline in this section so ticket-scoping is accurate.

**Methods migrating out**:

| Source trait (line) | Method | Destination | Rationale |
|---------------------|--------|-------------|-----------|
| `SpatialBeliefView` (909) | `locally_observed_entities_at` | `LocalPhysicalObservationView::colocated_entities` | Co-located physical perception |
| `EntityBeliefView` (791) | `locally_observed_entities_at` | `LocalPhysicalObservationView::colocated_entities` | Duplicate of above; consolidates |
| `ControlBeliefView` (780) | `believed_owner_of` | `BelievedAuthorityView::believed_owner_of` | Ownership is FND-24 authority |
| `PoliticalBeliefView` (1306) | `believed_office_holder` | `BelievedAuthorityView::believed_office_holder` | Office is FND-24 authority |

**Methods staying on their current trait** (with FND-14A-legality rationale):

| Trait | Method(s) | Rationale |
|-------|-----------|-----------|
| `EntityBeliefView` | `locally_observed_is_dead` (791) | Stays; `is_alive`/`is_dead` reads are physical observation, but the existing signature (per-agent, per-target) couples observation to actor identity in a way the consolidation on `LocalPhysicalObservationView` does not capture. The audit ticket may move it later if a `LocalPhysicalObservationView::observed_alive_status(actor, entity)` accessor is added. |
| `ControlBeliefView` | `believed_rights`, `can_control`, `has_control` (781–786) | `believed_rights` is a belief-backed accessor over delegated authority and stays on `ControlBeliefView`. `can_control` and `has_control` are derived control checks that compose ownership + access + force-control state. They stay on `ControlBeliefView` because they cross authority/political/control domains. |
| `PoliticalBeliefView` | `believed_force_controller`, `believed_membership`, `believed_faction_rally_point`, `believed_support_declaration`, `believed_support_declarations_for_office` (1313–1351), `locally_observed_bandit_camp_faction_at` (1272) | All belief-backed and politically scoped. `locally_observed_bandit_camp_faction_at` blends physical observation (bandit camp presence at place) with social attribution (faction identity); stays on `PoliticalBeliefView` per the politically-scoped framing. |
| `InventoryBeliefView` | `locally_observed_commodity_quantity` (1030) | Stays; the (agent, holder, kind) shape is inventory-scoped and the D1 variant `observed_item_lot_quantity(lot)` is the new lot-shaped accessor on the observation trait. Both can coexist because their input shapes are disjoint. |
| `InventoryBeliefView` | `believed_commodity_stock` (1046) | Belief-backed inventory; stays. |
| `InventoryBeliefView` | `direct_possessions`, `direct_container`, `direct_possessor` (394–436) | Authoritative-state inventory reads. Stay; they're the canonical co-located inventory accessor surface and are FND-14A-legal when called for co-located entities. |
| `SocialBeliefView` (all 30 methods) | All | Testimony/rumor/source-reliability surface, disjoint from authority. No migration. |
| `EconomicBeliefView`, `CombatBeliefView`, `FacilityBeliefView`, `TemporalBeliefView`, `ProfileBeliefView` (all methods) | All | Domain-scoped reads; no migration. |

This table is the authoritative per-method classification. If the implementing ticket discovers a method's classification is wrong, the ticket updates the table and the corresponding trait — but the table does not get deferred to a separate audit document.

### D4: `DebugWorldView` trait and `EntityState` struct

```rust
// crates/worldwake-core/src/world.rs (or a sibling module)

/// Snapshot of authoritative world state for one entity, used by debug/observer
/// tooling. Not consumed by the planner.
pub struct EntityState {
    pub kind: Option<EntityKind>,
    pub place: Option<EntityId>,
    pub alive: bool,
    pub container: Option<EntityId>,
    pub possessor: Option<EntityId>,
}
```

```rust
// crates/worldwake-sim/src/belief_view.rs
#[cfg(any(debug_assertions, test))]
pub trait DebugWorldView {
    fn world_entity_state(&self, entity: EntityId) -> EntityState;
    fn world_owner_of(&self, entity: EntityId) -> Option<EntityId>;
    fn world_location_of(&self, entity: EntityId) -> Option<EntityId>;
    fn world_inventory_of(&self, entity: EntityId) -> Vec<EntityId>;
}
```

Implemented for `&World`. Imported by `crates/worldwake-cli/src/bin/observer.rs`, the `worldwake-visualizer` snapshot module, golden harnesses, and integration tests.

`DebugWorldView` is a *labeled* surface for debug/observer/test access, not a type-enforced firewall against existing reads. `worldwake-ai` already depends on `worldwake-core` and can therefore reach `&World` accessors directly today; the only thing stopping it is convention. `DebugWorldView`'s value-add is twofold: (i) the `#[cfg(any(debug_assertions, test))]` gate means future debug-only accessors land on the trait rather than as new methods on `&World`'s primary public API, where they would be reachable from release-mode planner code; (ii) the D7 CI lint forbids `worldwake-ai` from importing the trait, so the discipline becomes an automated check rather than a code-review obligation. The four methods shown here are seed accessors that wrap existing `&World` reads (`World::entity_kind`, `World::effective_place`, `World::owner_of`, `World::possessor_of`, `World::possessions_of`, `World::direct_container`, etc.) so debug tooling can adopt the trait incrementally.

### D5: `RuntimeBeliefView` supertrait composition

```rust
pub trait RuntimeBeliefView:
    ControlBeliefView
    + EntityBeliefView
    + ProfileBeliefView
    + SpatialBeliefView
    + TemporalBeliefView
    + InventoryBeliefView
    + CombatBeliefView
    + EconomicBeliefView
    + SocialBeliefView
    + PoliticalBeliefView
    + FacilityBeliefView
    + BelievedAuthorityView
    + LocalPhysicalObservationView
{}
```

The 11 existing sub-traits remain supertraits of `RuntimeBeliefView`; the two new traits join the composition. `DebugWorldView` is *not* a supertrait — it is deliberately disjoint so that any caller wanting debug reads must import `DebugWorldView` explicitly and accept its cfg-gate.

`RuntimeBeliefView` continues to be the convenience composition for callers that need everything. Callers that need narrower access import only the sub-traits they use, which is what S143 enforces.

### D6: Import audit and rewrite

Every `use ...::RuntimeBeliefView;` in `worldwake-ai/src/**.rs` is examined. Where the file uses only a subset of methods, the import is narrowed. Where the file uses methods spread across multiple sub-traits, the import keeps `RuntimeBeliefView`. The goal is not maximally narrow imports — the goal is that *no* belief-view import in `worldwake-ai` reaches `DebugWorldView`.

Scope check: the broad-blast-radius sites are the 7 production files that import `RuntimeBeliefView` directly (`lib.rs`, `plan_revalidation.rs`, `planning_snapshot.rs`, `planning_state.rs`, `agent_tick/frame.rs`, `agent_tick/planning.rs`, `opportunity_compiler/compile.rs`). Test-side mock impls (~15 files) continue to implement `RuntimeBeliefView` directly — narrowing test mocks is out of scope.

The strict-narrowing rule on `worldwake-systems` applies only to perception-write modules. The ~15 action-handler modules that currently import `RuntimeBeliefView` are not narrowed; they perform legality and social-state checks at commit time and the broader surface is intentional.

### D7: Workspace grep-CI lint

A new `scripts/check_no_debug_view_in_ai.sh` script enforces that no source file under `crates/worldwake-ai/src/**` imports `DebugWorldView`. The script follows the pattern of `scripts/check_active_goal_removed.sh` and `scripts/check_no_artifact_state.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
matches="$(rg -l 'DebugWorldView' crates/worldwake-ai/src 2>/dev/null || true)"
if [ -n "$matches" ]; then
  echo "DebugWorldView illegally imported in worldwake-ai:" >&2
  echo "$matches" >&2
  exit 1
fi
```

The script is wired into `scripts/verify.sh` alongside the existing precedent scripts. No custom clippy lint is introduced — the codebase has zero custom clippy lints, and the grep-CI approach is the established enforcement pattern.

### D8: Golden coverage

`crates/worldwake-ai/tests/golden_belief_wall_trap.rs` adds a "belief-wall trap" regression: an agent stands at a chest in an office building. Expectations:

- Agent observes the chest, its contents, and the building (physical observation, FND-14A legal — `LocalPhysicalObservationView` reads succeed).
- Agent does *not* know who owns the chest, who holds the keys, what office governs the building, who the current office-holder is, or what jurisdiction applies (`BelievedAuthorityView` reads return `BeliefRead::Unknown`).
- Agent's `Steal` candidate is suppressed because the legality predicate requires `believed_owner_of` which returns `BeliefRead::Unknown`.

The new golden composes with the adjacent existing goldens that cover related ground: `crates/worldwake-ai/tests/golden_epistemic_sensing.rs`, `golden_perception_omission.rs`, and `golden_perception_exposure.rs` exercise perception and observation boundaries; the belief-wall trap exercises the authority-belief absence specifically.

A `compile_fail` doctest on the `DebugWorldView` trait definition proves that `worldwake-ai` source files cannot import it. Compile-fail doctests are an established pattern in worldwake (precedents at `crates/worldwake-ai/src/planning_snapshot.rs:402-414` and `crates/worldwake-ai/src/ranking.rs:7-19`).

## FND-01 Section H Analysis

S143 is a Rust trait-surface partition with no new ECS components, perception events, simulation systems, or information channels, and does not alter simulation semantics. The FND-01 Section H sub-sections below are therefore not applicable, retained as placeholders for spec template completeness.

### Information-Path Analysis

Not applicable. S143 is a Rust type-system refactor; it does not introduce new information flows. The information paths preserved are the existing perception → belief-store → belief-view paths.

### Positive-Feedback Analysis

Not applicable. No new feedback loops are introduced or amplified.

### Concrete Dampeners

Not applicable. No new amplifying loops to dampen.

### Stored State vs. Derived Read-Model List

**Stored state**: Unchanged from S75 / S101 / S113. `AgentBeliefStore`, `EntityBeliefAspect`, `BeliefSet<T>`, `BeliefValue<T>` all preserved.

**Derived views**: The new traits (`LocalPhysicalObservationView`, `BelievedAuthorityView`, `DebugWorldView`) are pure trait-surface partitions over existing stored state. The new types `BeliefRead<T>`, `ObservedRead<T>`, `ObservationSource`, and `EntityState` are read-shape wrappers and snapshot structs derived from existing stored state. No new derived authoritative computation.

## SystemFn Integration

Not applicable. S143 introduces no new `SystemFn`.

## Component Registration

Not applicable. S143 introduces no new ECS component.

## Cross-System Interactions

- `worldwake-systems` perception-write modules continue to populate `AgentBeliefStore` exactly as today; they import `LocalPhysicalObservationView` only for observation-time co-location resolution.
- `worldwake-systems` action-handler modules continue to import `RuntimeBeliefView` and domain sub-traits for legality and social-state checks at commit time. The narrowing rule does not apply to them.
- `worldwake-ai` planner code reads through the partitioned trait surface; no behavioral change in legal reads. After D6, no planner source file imports `DebugWorldView`.
- `worldwake-cli` observer and `worldwake-visualizer` debug tooling import `DebugWorldView` under `#[cfg(any(debug_assertions, test))]`.

The interaction shape is unchanged: systems mutate state, the planner reads belief-backed views, and observers read all of it under debug gating. Only the *type-level expression* of these boundaries gets tighter.

## Profile-Driven Parameters

Not applicable. S143 introduces no new profile parameters.

## Test Plan

- Per-method audit table inline in D3 — every accessor classified as migrated to `LocalPhysicalObservationView`, migrated to `BelievedAuthorityView`, or staying on its current domain sub-trait.
- Compile-fail doctest on `DebugWorldView` proving it cannot be imported from `worldwake-ai/src/`.
- Belief-wall trap golden (`crates/worldwake-ai/tests/golden_belief_wall_trap.rs`).
- All existing goldens pass unchanged (`cargo test --workspace`).
- CI grep check (`scripts/check_no_debug_view_in_ai.sh`, wired into `scripts/verify.sh`) verifying no `worldwake-ai` source file imports `DebugWorldView`.
- `cargo clippy --workspace --all-targets -- -D warnings` passes after the import rewrite in D6 (no unused imports, no dead trait imports).
