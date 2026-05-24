# S166OPPCMPSRCFID-002: Derive `required_actions` from registry via `EffectSchemaIndex`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — substrate fidelity fix in `worldwake-ai`; no behavioral change to any current consumer (`Opportunity.required_actions` has zero runtime readers today).
**Deps**: spec `specs/S166-opportunity-compiler-source-fidelity.md` (D2)

## Problem

`crates/worldwake-ai/src/opportunity_compiler/compile.rs:127` constructs every
inventory opportunity with `required_actions: vec![PlannerOpKind::MoveCargo]` —
a hard-coded literal that may not match the lawful producers in the active
scenario's action registry. The compiler already enumerates the producers via
`EffectSchemaIndex::actions_producing(EffectFactKey::CommodityTransfer)` at line
23-28, but uses the result only as an emptiness gate. This ticket extends
`EffectSchemaIndex` to additionally cache the
`ActionDefId → PlannerOpKind` classification, exposes a `planner_ops_producing`
accessor, and replaces the `MoveCargo` literal with the derived set.

With the default action registry the resulting set is
`{Harvest, Craft, Trade, MoveCargo, StockManagement, DropItem, Loot}` — the
intersection of `effect_keys_for_steps` producing `CommodityTransfer`
(`effect_schema_index.rs:60-72`) with the `classify_action_def` arms
(`planner_ops.rs:85-145`). Actions whose classifier returns `None` (e.g., a
generic `transfer` action with no name match) are filtered out — the opportunity
advertises only ops the planner can actually emit.

At reassessment, `Opportunity.required_actions` had **no runtime consumer** (workspace
grep confirms zero non-construction call sites outside `observer.rs:6278` which
constructs an empty `Vec`). This fix is fidelity-preserving substrate work: it
ensures the field's value is truthful for any future consumer, aligning with
FND-3 (concrete state) and FND-29 (debuggability — the field's claim must be
inspectable and correct).

## Assumption Reassessment (2026-05-24)

1. `EffectSchemaIndex::build(registry: &ActionDefRegistry)` at `crates/worldwake-ai/src/effect_schema_index.rs:19` has the action registry in scope; pre-computing the `ActionDefId → Option<PlannerOpKind>` mapping at build time avoids threading `&ActionDefRegistry` through every per-tick caller.
2. `classify_action_def(def: &ActionDef) -> Option<PlannerOpKind>` at `crates/worldwake-ai/src/planner_ops.rs:85` takes `&ActionDef`, not `ActionDefId`. The registry is iterated by value in `EffectSchemaIndex::build` (`for action_def in registry.iter()` at `effect_schema_index.rs:22`), so the classifier can be called directly during build.
3. Shared abstraction boundary under audit: `EffectSchemaIndex`'s public surface — the existing `actions_producing(EffectFactKey) -> &[ActionDefId]` accessor is preserved; the new `planner_ops_producing(EffectFactKey) -> &BTreeSet<PlannerOpKind>` accessor is added alongside.
4. Existing inline tests in `crates/worldwake-ai/src/effect_schema_index.rs::tests` (`build_maps_effect_keys_to_action_defs_and_empty_lookup_is_stable:184`, `build_sorts_and_deduplicates_action_ids_per_effect_key:224`, `build_is_deterministic_for_same_registry:261`) construct `EffectSchemaIndex` via `EffectSchemaIndex::build(&registry)` and inspect `by_effect` — they continue to pass once the new field has a sensible Default. The fixture at `crates/worldwake-ai/src/opportunity_compiler/compile.rs:317-323` uses struct-literal construction (`EffectSchemaIndex { by_effect: BTreeMap::from([(...)]) }`) without `..Default::default()` and **needs the new field added explicitly** when this ticket lands.
5. Existing inline tests in `crates/worldwake-ai/src/opportunity_compiler/compile.rs::tests` exercising `compile_opportunities` and currently asserting against the `MoveCargo`-literal-shaped output: `compile_opportunities_emits_inventory_backed_opportunities:327`, `compile_opportunities_does_not_anchor_acquisition_on_self_inventory:358`, `compile_opportunities_applies_floor_damping_and_cap:407`, `compile_opportunities_skips_confirmed_empty_survey_places:444`, `compile_opportunities_damps_learned_memory_entries:480`. None of these tests currently assert `required_actions` content (verified by grep — no `required_actions` reads in the asserts), so they continue to pass without modification. A new focused test asserting the derived set is added.
6. Information-path classification: the same `(action registry → planner ops producing CommodityTransfer)` derivation today is computed at the emptiness-gate site (`compile.rs:23-28`, used only for an `is_empty()` check) and discarded. After this ticket the same derivation is computed once at index build time, cached in `by_effect_op`, and returned by `planner_ops_producing`. No duplicate transport path survives; the original `actions_producing` accessor returns the raw `ActionDefId` slice and is unchanged.
7. Mismatch + correction: the spec text says "mapped through the `planner_ops` `ActionDefId`→`PlannerOpKind` classifier" — the classifier actually takes `&ActionDef`, not `ActionDefId`. The fix routes through registry iteration at build time (where `&ActionDef` is in scope), preserving the spec's intent (cache `ActionDefId → PlannerOpKind`) without requiring a registry parameter on the per-tick caller. Documented here; no spec text change.

## Architecture Check

1. Caching the `ActionDefId → Option<PlannerOpKind>` mapping at `EffectSchemaIndex::build` time follows the existing precedent of pre-computing the producer set at the same site (the index already iterates the registry once). The per-tick `compile_opportunities` caller gains a single new `BTreeSet` accessor read, not a registry traversal — the work is amortized across all ticks that consume the same index.
2. The new accessor `planner_ops_producing(EffectFactKey) -> &BTreeSet<PlannerOpKind>` returns a borrowed `BTreeSet` rather than allocating a fresh `Vec` per call. The compiler binds the result once before the outer entity loop at `compile.rs:45` and clones into each emitted opportunity's `required_actions` field, so the per-opportunity allocation is bounded by the set's size (≤ N for N classifiable producers).
3. No backward-compatibility shim: the `MoveCargo` literal at `compile.rs:127` is replaced in place. The `actions_producing` accessor remains for callers that need the raw `ActionDefId` slice (none today outside compile.rs's emptiness gate, but the surface is preserved for future use).

## Verified Layers

1. Derived-set correctness — focused unit test in `effect_schema_index.rs::tests` constructing a registry with known producers (transfer, harvest, craft, trade, pick_up, etc.) and asserting `planner_ops_producing(EffectFactKey::CommodityTransfer)` returns the expected `BTreeSet<PlannerOpKind>`.
2. Compile-pass derivation — focused unit test in `opportunity_compiler/compile.rs::tests` constructing a multi-producer registry and asserting an emitted opportunity's `required_actions` matches the derived set rather than `vec![MoveCargo]`.
3. Determinism — the new field is `BTreeMap<EffectFactKey, BTreeSet<PlannerOpKind>>`; iteration order is determinism-stable per the workspace's no-`HashMap` invariant.
4. Existing-test no-regression — the 3 existing `effect_schema_index.rs` tests (build-deterministic, sort/dedup, key mapping) and the 5 existing `opportunity_compiler/compile.rs` tests (listed in Assumption 5) continue to pass.

## Landed Changes

### 1. Extended `EffectSchemaIndex` in `crates/worldwake-ai/src/effect_schema_index.rs`

Add a `by_effect_op: BTreeMap<EffectFactKey, BTreeSet<PlannerOpKind>>` field alongside the existing `by_effect`. In `build()`, alongside the `by_effect` insertion, compute `classify_action_def(action_def)` for each `action_def`; if the result is `Some(op)`, insert into `by_effect_op.entry(key).or_default()` for each `key` in `effect_keys_for_steps(...)`. Drop `None`-classifying actions.

Add the accessor:

```rust
#[must_use]
pub fn planner_ops_producing(&self, fact: EffectFactKey) -> &BTreeSet<PlannerOpKind> {
    static EMPTY: BTreeSet<PlannerOpKind> = BTreeSet::new();
    self.by_effect_op.get(&fact).unwrap_or(&EMPTY)
}
```

Note: `static EMPTY: BTreeSet<PlannerOpKind> = BTreeSet::new()` works because `BTreeSet::new()` is `const`. Confirm during implementation; if it isn't const at the current Rust toolchain version, fall back to `Option<&BTreeSet>` return or a `Cow`. The two existing inline tests construct via `EffectSchemaIndex::build(&registry)` and inspect `by_effect`; they continue to pass with the new field once `Default` provides an empty `by_effect_op`.

Update `impl Default for EffectSchemaIndex` to seed both fields empty.

Update the import block to bring `BTreeSet` and `PlannerOpKind` into scope. Bring `classify_action_def` into scope from `crate::planner_ops`.

### 2. Updated explicit test fixtures

The original compiler fixture:

```rust
fn index() -> EffectSchemaIndex {
    EffectSchemaIndex {
        by_effect: BTreeMap::from([(
            EffectFactKey::CommodityTransfer,
            vec![worldwake_core::ActionDefId(0)],
        )]),
    }
}
```

becomes:

```rust
fn index() -> EffectSchemaIndex {
    EffectSchemaIndex {
        by_effect: BTreeMap::from([(
            EffectFactKey::CommodityTransfer,
            vec![worldwake_core::ActionDefId(0)],
        )]),
        by_effect_op: BTreeMap::from([(
            EffectFactKey::CommodityTransfer,
            BTreeSet::from([PlannerOpKind::MoveCargo]),
        )]),
    }
}
```

The `BTreeSet::from([PlannerOpKind::MoveCargo])` preserves the existing test setup's intent (the registry-of-one has MoveCargo as the only classified producer). The same field was also added to the explicit `EffectSchemaIndex` constructor in `crates/worldwake-ai/src/agent_tick/tests.rs`.

### 3. Replaced the `required_actions` literal in `crates/worldwake-ai/src/opportunity_compiler/compile.rs`

The compile pass now binds before the outer entity loop:

```rust
let required_actions_for_transfer: Vec<PlannerOpKind> = action_index
    .planner_ops_producing(EffectFactKey::CommodityTransfer)
    .iter()
    .copied()
    .collect();
```

The emitted `Opportunity` now uses `required_actions: required_actions_for_transfer.clone()`.

The set is computed once per `compile_opportunities` call; the `.clone()` per emitted opportunity is unavoidable because `Opportunity.required_actions` is `Vec<PlannerOpKind>` owned.

### 4. Added focused tests for the derived set

In `crates/worldwake-ai/src/effect_schema_index.rs::tests`, `planner_ops_producing_returns_classified_set` builds a registry with multiple known classifiable producers (`pick_up`, `harvest:bread`, `trade`) and asserts:

```rust
let expected: BTreeSet<PlannerOpKind> =
    [PlannerOpKind::MoveCargo, PlannerOpKind::Harvest, PlannerOpKind::Trade].into();
assert_eq!(index.planner_ops_producing(EffectFactKey::CommodityTransfer), &expected);
```

In `crates/worldwake-ai/src/opportunity_compiler/compile.rs::tests`, `compile_opportunities_emits_derived_required_actions` constructs an index with multiple classifiable producers and asserts an emitted opportunity's `required_actions` matches the derived set sorted by `BTreeSet`'s natural ordering when collected into `Vec`.

## Landed Files

- `crates/worldwake-ai/src/effect_schema_index.rs` (modify — add field, accessor, classifier integration, focused test)
- `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (modify — fixture update, replace literal, add focused test)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — make `classify_action_def` visible within the crate for the index build)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — update explicit `EffectSchemaIndex` constructor)

## Out of Scope

- Adding consumers for `Opportunity.required_actions`. The field has no current runtime reader and this ticket does not introduce one — it fixes the value's fidelity for any future consumer.
- Changing `Opportunity`'s field shape or removing `required_actions`. Spec D2 explicitly preserves the field's signature.
- Caching at any layer above `EffectSchemaIndex` (e.g., a per-tick `BTreeSet` interned somewhere on the decision runtime). Per-call clone is acceptable for the per-opportunity hot path.

## Acceptance Result

### Tests Passed

1. New focused test `effect_schema_index::tests::planner_ops_producing_returns_classified_set` asserts the multi-producer derivation.
2. New focused test `opportunity_compiler::compile::tests::compile_opportunities_emits_derived_required_actions` asserts the emitted opportunity carries the derived set rather than `vec![MoveCargo]`.
3. Existing inline tests pass unchanged: the 3 in `effect_schema_index.rs` and the 5 in `opportunity_compiler/compile.rs` (named in Assumption Reassessment item 5).
4. `cargo test -p worldwake-ai` — full AI crate suite passes.

### Invariants

1. `Opportunity.required_actions` value at emission is equal to `action_index.planner_ops_producing(EffectFactKey::CommodityTransfer).iter().copied().collect::<Vec<_>>()`. The `MoveCargo` literal is removed from `compile.rs:127`.
2. `EffectSchemaIndex::Default::default()` returns an index whose `by_effect` and `by_effect_op` are both empty — the existing emptiness gate at `compile.rs:23-28` continues to return early when no actions produce `CommodityTransfer`.
3. Iteration of `by_effect_op` is determinism-stable (`BTreeMap` + `BTreeSet` ordering, no `HashMap`).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/effect_schema_index.rs` (inline `tests` module) — `planner_ops_producing_returns_classified_set`: builds a registry with `pick_up`, `harvest:bread`, `trade`, asserts the derived set is `{MoveCargo, Harvest, Trade}`.
2. `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (inline `tests` module) — `compile_opportunities_emits_derived_required_actions`: builds a multi-producer index and asserts an emitted opportunity's `required_actions` matches the derived set in enum-order `Vec` form (`[Trade, Harvest]` for the fixture).
3. Explicit `EffectSchemaIndex` fixtures in `compile.rs::tests::index` and `agent_tick/tests.rs` were updated to include `by_effect_op`.

### Commands Run

1. `cargo test -p worldwake-ai effect_schema_index` — targets the new derivation test.
2. `cargo test -p worldwake-ai opportunity_compiler` — confirms both the new derivation test and existing emission tests pass.
3. `cargo test -p worldwake-ai` — full AI crate suite for no-regression.
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` — clippy gate.
5. `./scripts/verify.sh` — waived for this per-ticket closeout because `$implement-spec-tickets` owns the final full pre-PR gate before branch push.

## Outcome

Completed on 2026-05-24.

- `EffectSchemaIndex` now caches planner operation producers per `EffectFactKey` using the existing action registry and `classify_action_def` during index construction.
- `compile_opportunities` now emits `Opportunity.required_actions` from `planner_ops_producing(CommodityTransfer)` instead of hard-coding `MoveCargo`.
- Explicit test fixtures were updated for the new index field, and focused coverage was added at both the index and compiler seams.

## Deviations

- `classify_action_def` was made `pub(crate)` rather than duplicated or wrapped so `EffectSchemaIndex::build` can reuse the existing classifier directly.
- The compiler-focused expected `Vec` order is `Trade, Harvest` for the test fixture because `BTreeSet<PlannerOpKind>` orders by the enum declaration, not by insertion or prose example order.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib effect_schema_index::tests::planner_ops_producing_returns_classified_set -- --exact`.
- Passed `cargo test -p worldwake-ai --lib opportunity_compiler::compile::tests::compile_opportunities_emits_derived_required_actions -- --exact`.
- Passed `cargo test -p worldwake-ai effect_schema_index`.
- Passed `cargo test -p worldwake-ai opportunity_compiler`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
- Waived `./scripts/verify.sh` for this per-ticket closeout because the spec-ticket harness owns that full gate before pushing the final branch.
