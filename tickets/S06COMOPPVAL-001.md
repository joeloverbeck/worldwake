# S06COMOPPVAL-001: CommodityValuationProfile component and registration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-core` (new component type + schema registration)
**Deps**: specs/S06-commodity-opportunity-valuation.md

## Problem

The shared commodity-opportunity valuation layer (S06) requires per-agent reasoning bounds that control how deeply an agent evaluates indirect commodity utility through recipe chains. Without `CommodityValuationProfile`, the downstream tickets (002-007) cannot access these bounds. This ticket delivers the component type and its schema registration with no behavioral changes.

## Assumption Reassessment (2026-04-02)

1. No `CommodityValuationProfile` type exists in the codebase — confirmed via grep (zero matches).
2. Component registration uses the `with_component_schema_entries!` macro. Expansion sites that need imports: `delta.rs`, `world.rs`, `component_tables.rs` in `worldwake-core` (per `tickets/README.md` line 58).
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

Add `CommodityValuationProfile` to the `with_component_schema_entries!` macro invocation. Ensure the type is imported at all expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`).

### 3. Export from `worldwake-core/src/lib.rs`

Add `CommodityValuationProfile` to the crate's public exports.

## Files to Touch

- `crates/worldwake-core/src/valuation.rs` (new — or add to existing module)
- `crates/worldwake-core/src/lib.rs` (modify — export)
- `crates/worldwake-core/src/delta.rs` (modify — import for macro)
- `crates/worldwake-core/src/world.rs` (modify — import for macro)
- `crates/worldwake-core/src/component_tables.rs` (modify — import for macro, if this file exists; otherwise the appropriate macro expansion site)

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
