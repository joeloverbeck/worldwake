# E17CRITHEJUS-001: Core crime types in worldwake-core

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component types, component registration
**Deps**: None (pure type additions)

## Problem

No per-agent crime disposition types exist. E17 requires `TheftDispositionProfile`, `JusticeDispositionProfile`, and `PunishmentKind` before any action or AI work can begin.

## Assumption Reassessment (2026-03-25)

1. `component_schema.rs` and `component_tables.rs` use a macro-generated registration pattern for authoritative components. Agent-only profile components such as `CombatProfile`, `UtilityProfile`, `TradeDispositionProfile`, and `ViolationDispositionProfile` are declared there and projected into `ComponentTables`, `World`, `WorldTxn`, `ComponentKind`, and related tests from the single schema manifest. New crime profiles should follow that manifest path rather than adding ad-hoc storage.
2. `Permille` newtype exists in `crates/worldwake-core/src/numerics.rs` and is used across all profile types.
3. `CommodityKind` and `Quantity` exist in `crates/worldwake-core/src/items.rs`.
4. `World` already has generated component insertion/rejection coverage for agent-only components in `crates/worldwake-core/src/world.rs`, and `WorldTxn` has manifest-consistency coverage in `crates/worldwake-core/src/world_txn.rs`. The live verification surface is those generated APIs, not custom tests in `component_schema.rs`.
5. `PunishmentKind` is not yet consumed by live `GoalKind` ordering in `crates/worldwake-core/src/goal.rs`; that usage lands in later E17 tickets. It is still appropriate here as a shared domain primitive because both the future institutional claim surface (`E17CRITHEJUS-003`) and justice goals (`E17CRITHEJUS-004`) need the same canonical punishment type.
6. N/A — no heuristic changes.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. Corrected mismatch: the original ticket overstated `component_schema.rs` as a direct test target and implied `PunishmentKind` already served `GoalKind` ordering. The scope stays core-only, but verification and rationale are updated to match the current code.
12. N/A.

## Architecture Check

1. A dedicated `crime.rs` module in `worldwake-core` is cleaner than scattering theft/justice primitives across `institutional.rs`, `goal.rs`, or `violation.rs`. Crime is a first-class domain in the E17 spec, and this keeps later theft, accusation, and punishment work dependent on one canonical type surface.
2. Registering `TheftDispositionProfile` and `JusticeDispositionProfile` through the existing component schema is cleaner than hand-writing one-off table/world methods. The schema already owns authoritative component projection, kind checks, and manifest coverage.
3. `PunishmentKind` belongs in the same module as the crime profiles rather than inside institutional records. Fine/exile are domain concepts shared by records, goals, and action handlers; anchoring them in a neutral crime module is more extensible than nesting them under one downstream consumer.
4. No backwards-compatibility aliasing introduced.

## Verification Layers

1. `TheftDispositionProfile` round-trips through serde and satisfies component bounds -> focused unit test in `crime.rs`
2. `JusticeDispositionProfile` round-trips through serde and satisfies component bounds -> focused unit test in `crime.rs`
3. `PunishmentKind` round-trips through serde for both variants and keeps deterministic ordering -> focused unit test in `crime.rs`
4. Schema registration projects the new profiles into live world APIs -> focused `world.rs` tests for insert/query/remove on agents and rejection on non-agents
5. Schema registration remains consistent with generated transaction setter coverage -> existing `world_txn.rs` manifest test plus `cargo test -p worldwake-core`
6. Single-crate ticket; no cross-layer mapping needed.

## What to Change

### 1. New `crime.rs` module in worldwake-core

Create `crates/worldwake-core/src/crime.rs` with:

- `TheftDispositionProfile` struct: `steal_duration_ticks: NonZeroU32`, `theft_motive_weight: Permille`, `witness_risk_penalty: Permille`. Derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Implement `Component` trait.
- `JusticeDispositionProfile` struct: `accusation_motive_weight: Permille`, `fine_severity: Permille`. Same derives + `Component`.
- `PunishmentKind` enum: `Fine { commodity: CommodityKind, amount: Quantity }`, `Exile { from_faction: EntityId }`. Derive `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`.

### 2. Register components in component_schema.rs and component_tables.rs

Add `TheftDispositionProfile` and `JusticeDispositionProfile` as Agent-only components in the schema manifest so the generated storage/world/txn surfaces stay aligned.

### 3. Export from lib.rs

Add `pub mod crime;` and re-export the three types.

## Files to Touch

- `crates/worldwake-core/src/crime.rs` (new)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify test expectations only)
- `crates/worldwake-core/src/lib.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify tests only)

## Out of Scope

- ViolationKind extensions (E17CRITHEJUS-002)
- InstitutionalClaim extensions (E17CRITHEJUS-003)
- GoalKind extensions (E17CRITHEJUS-004)
- Any worldwake-sim, worldwake-systems, or worldwake-ai changes
- Action definitions or handlers
- AI candidate generation or planner integration
- Golden tests

## Acceptance Criteria

### Tests That Must Pass

1. `TheftDispositionProfile` serde round-trip preserves all fields
2. `JusticeDispositionProfile` serde round-trip preserves all fields
3. `PunishmentKind::Fine` and `PunishmentKind::Exile` serde round-trip
4. Both profiles accepted as components on `EntityKind::Agent`
5. Both profiles rejected on non-Agent entity kinds
6. Existing suite: `cargo test -p worldwake-core`
7. Existing suite: `cargo clippy -p worldwake-core`

### Invariants

1. All new types derive the full standard set (`Clone, Debug, Eq, PartialEq, Serialize, Deserialize`)
2. `PunishmentKind` additionally derives `Copy, Ord, PartialOrd` so later goal/record consumers can rely on deterministic value semantics without introducing wrappers or aliases
3. No `HashMap`/`HashSet` introduced (determinism invariant)
4. No floats introduced (determinism invariant)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/crime.rs` — unit tests for trait bounds, construction, deterministic ordering, and serde round-trip
2. `crates/worldwake-core/src/world.rs` — component registration tests for both profiles on Agent vs non-Agent through the generated world API

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy -p worldwake-core`

## Outcome

- Completion date: 2026-03-25
- What actually changed: added `crates/worldwake-core/src/crime.rs` with `TheftDispositionProfile`, `JusticeDispositionProfile`, and `PunishmentKind`; registered both profiles through the authoritative component schema; re-exported the new crime types from `worldwake-core`; added focused unit tests in `crime.rs` and generated-world API tests in `world.rs`.
- Deviations from original plan: verification was corrected to use the live generated `World` API instead of adding bespoke `component_schema.rs` tests; `component_tables.rs` changed only because the schema manifest fan-out requires the new type imports there; `delta.rs` test expectations were updated because schema registration extends `ComponentKind::ALL`.
- Verification results: `cargo test -p worldwake-core` passed; `cargo clippy -p worldwake-core --all-targets -- -D warnings` passed; focused checks for `crime::tests::*` and `world::tests::theft_disposition_profile_component_roundtrip_on_agent` passed during development.
