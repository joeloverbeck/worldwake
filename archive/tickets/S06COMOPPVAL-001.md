# S06COMOPPVAL-001: CommodityValuationProfile component and registration

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-core` (new component type + schema registration)
**Deps**: specs/S06-commodity-opportunity-valuation.md

## Problem

The shared commodity-opportunity valuation layer (S06) requires per-agent reasoning bounds that control how deeply an agent evaluates indirect commodity utility through recipe chains. Without `CommodityValuationProfile`, the downstream tickets (002-007) cannot access these bounds. This ticket delivers the component type and its schema registration with no behavioral changes.

## Assumption Reassessment (2026-04-02)

1. No `CommodityValuationProfile` type exists in the codebase — confirmed via grep (zero matches).
2. Component registration uses the `with_component_schema_entries!` macro plus the canonical schema declaration in `component_schema.rs`. Live expansion/import sites that need the new type in scope are `delta.rs`, `world.rs`, `component_tables.rs`, and `world_txn.rs` in `worldwake-core`; `component_schema.rs` itself also needs the new authoritative entry.
3. `Permille` at `crates/worldwake-core/src/numerics.rs` is `Copy + Clone + Eq + PartialEq + Ord + PartialOrd + Hash + Debug + Serialize + Deserialize`. All proposed fields (`NonZeroU8`, `u8`, `Permille`) are `Copy`.
4. Existing profile components follow the pattern: struct with derives → `impl Component for T {}` → registration in schema macro → export from `lib.rs`. Examples: `TradeDispositionProfile`, `UtilityProfile`, `MetabolismProfile`.
5. The spec proposes `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize` derives. Adding `Ord, PartialOrd, Hash` would be consistent with most components (e.g., `TradeDispositionProfile` pattern) and is harmless for a profile type.

## Architecture Check

1. A dedicated profile component for valuation reasoning bounds is cleaner than overloading `UtilityProfile` (which governs AI motive weights, not valuation depth). Separation of concerns: `UtilityProfile` = what the agent cares about, `CommodityValuationProfile` = how deeply the agent reasons about indirect value.
2. No backward-compatibility shims. This is a new type — all construction sites are new.

## Verification Layers

1. `CommodityValuationProfile` satisfies required trait bounds -> focused unit test (trait assertion)
2. Component registration compiles at all macro expansion sites -> compiler
3. Single-layer ticket (type addition only, no runtime behavior change).

## What to Change

### 1. Add `CommodityValuationProfile` to `worldwake-core`

Create or extend the appropriate module (e.g., `crates/worldwake-core/src/valuation.rs` or add to an existing module like `trade.rs` if that's the natural home for valuation-adjacent types):

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CommodityValuationProfile {
    pub recipe_opportunity_depth: NonZeroU8,
    pub recipe_place_horizon: u8,
    pub indirect_value_decay_per_step: Permille,
}

impl Component for CommodityValuationProfile {}
```

### 2. Register in component schema

Add `CommodityValuationProfile` to the authoritative `with_component_schema_entries!` declaration in `component_schema.rs`. Ensure the type is imported at all live expansion/import sites (`delta.rs`, `world.rs`, `component_tables.rs`, `world_txn.rs`).

### 3. Export from `worldwake-core/src/lib.rs`

Add `CommodityValuationProfile` to the crate's public exports.

### 4. Add representative test fixture support

Add a `sample_commodity_valuation_profile()` helper in `worldwake-core/src/test_utils.rs` so downstream schema/delta/world-transaction tests can construct the new component deterministically without ad hoc duplicates.

## Files to Touch

- `crates/worldwake-core/src/valuation.rs` (new — or add to existing module)
- `crates/worldwake-core/src/lib.rs` (modify — export)
- `crates/worldwake-core/src/component_schema.rs` (modify — authoritative schema entry)
- `crates/worldwake-core/src/delta.rs` (modify — import for macro)
- `crates/worldwake-core/src/world.rs` (modify — import for macro)
- `crates/worldwake-core/src/component_tables.rs` (modify — import for macro)
- `crates/worldwake-core/src/world_txn.rs` (modify — import for macro expansion site)
- `crates/worldwake-core/src/test_utils.rs` (modify — representative fixture)

## Out of Scope

- Belief view extension (ticket 002)
- Commodity opportunity module (ticket 003)
- Recipe propagation logic (ticket 004)
- Trade valuation integration (ticket 005)
- AI ranking integration (ticket 006)

## Acceptance Criteria

### Tests That Must Pass

1. `CommodityValuationProfile` satisfies `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned` (trait assertion test)
2. Bincode roundtrip for `CommodityValuationProfile` produces identical output
3. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `CommodityValuationProfile` is `Copy` — enforced by derive macro.
2. All fields use proper newtypes: `NonZeroU8`, `u8`, `Permille` — no bare floats or unconstrained integers.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/valuation.rs` (new test module) — trait assertion and bincode roundtrip for `CommodityValuationProfile`

### Commands

1. `cargo test -p worldwake-core -- valuation` — targeted tests
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite

## Outcome

- **Completed**: 2026-04-02
- **What changed**:
  - Added `CommodityValuationProfile` in `crates/worldwake-core/src/valuation.rs` with the required bounded-reasoning fields and focused trait/bincode tests.
  - Exported the new component from `crates/worldwake-core/src/lib.rs`.
  - Registered the component in the authoritative schema in `crates/worldwake-core/src/component_schema.rs` and wired the generated schema surfaces in `crates/worldwake-core/src/delta.rs`, `crates/worldwake-core/src/world.rs`, and `crates/worldwake-core/src/component_tables.rs`.
  - Added `sample_commodity_valuation_profile()` in `crates/worldwake-core/src/test_utils.rs` and updated the explicit component inventory/sample assertions in `crates/worldwake-core/src/delta.rs`.
- **Deviations from original plan**:
  - The reassessment found that `component_schema.rs` and the explicit `delta.rs` component manifest were part of the real registration boundary, while `world_txn.rs` did not require a lasting new top-level import after implementation.
  - No behavioral runtime changes were introduced; the ticket remained a type-and-schema slice.
- **Verification results**:
  - `cargo test -p worldwake-core valuation -- --nocapture`
  - `cargo test -p worldwake-core component_kind_variants_match_authoritative_components -- --nocapture`
  - `cargo test -p worldwake-core`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
