# S138OPPCOM-005: EffectSchemaIndex module and driver-lifetime build

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — adds a driver-lifetime read-model derived from `ActionDefRegistry`
**Deps**: archive/tickets/S138OPPCOM-001.md (defines `EffectFactKey`)

## Problem

S138's compiler needs to answer "which actions produce this effect?" cheaply per tick. `ActionDef.effect_schema` (landed in S134) carries the typed effect declaration per action. This ticket builds a `BTreeMap<EffectFactKey, Vec<ActionDefId>>` index over the registry once per `AgentTickDriver` lifetime and exposes it as `EffectSchemaIndex`. The index becomes a stable read-model the compiler (ticket 006) consults whenever a goal's `relevant_ops` hint is exhausted.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage: `crates/worldwake-sim/src/effect_schema.rs:209` defines `EffectFact` with 6 variants; `crates/worldwake-sim/src/action_def.rs:144` defines `ActionDef.effect_schema: EffectSchema`; `crates/worldwake-sim/src/action_def_registry.rs:6` exposes `ActionDefRegistry` with `iter()`.
2. Spec/doc reference: `specs/S138-opportunity-compiler.md` deliverable section "`worldwake-ai::effect_schema_index` (new module)".
3. Shared abstraction boundary: ai-side index consumes sim's `ActionDefRegistry` as a read-only input — one-way data flow, no cross-crate mutation.
4. Where to build and store the index: the live `AgentTickDriver::new()` constructor intentionally has no action-registry parameter across existing tests/tooling, while `produce_agent_input` receives the canonical `ActionDefRegistry` through `AutonomousControllerContext`. The landed seam stores `EffectSchemaIndex` on `AgentTickDriver`, exposes `AgentTickDriver::new_with_action_defs(registry)` for startup construction, and also performs one-time lazy initialization from `ctx.action_defs` for restored/default-constructed drivers. The index is derived runtime state and is not serialized.
5. Live effect-schema shape: `EffectSchema.steps` contains `EffectStep` declarations, not `EffectFact` records. The index therefore maps declared effect steps to the payload-free `EffectFactKey` categories they can emit (`Transfer -> CommodityTransfer`, `HarvestResource -> PartialQuantity`, etc.) instead of iterating nonexistent `step.facts`.
6. FND-26 (Systems through state): the index reads only the action registry — no cross-system imperative; the registry itself is set up once at simulation construction.

## Architecture Check

1. One-time driver construction matches the spec's no-per-tick-rebuild rule (FND-12, performance compresses computation, never causality) — the index is a derived read-model over registry state that itself never changes during a simulation run.
2. `BTreeMap` key (`EffectFactKey`) and `Vec` of `ActionDefId` preserve determinism: iteration order is `EffectFactKey`'s `Ord` (defined in ticket 001).
3. Empty-index behavior is well-defined: `actions_producing(fact)` returns an empty slice when no action produces the effect, which is the expected case for `EffectFactKey::PartialQuantity` and other effects that don't appear as a primary product of any action.
4. No backward-compatibility shim: the index is brand-new; nothing aliases or wraps it.

## Verification Layers

1. Index build over a known small registry — focused unit test constructing a 2-3 action registry and asserting the index maps each `EffectFactKey` to the right `ActionDefId`s
2. Empty-index lookup for an effect with no producer — focused unit test
3. Iteration order is `BTreeMap`-stable — focused unit test using two different insertion orders (via different `Vec<ActionDef>` orderings) and asserting identical lookup results
4. Determinism — crate-level regression plus workspace build/lint gates

## What to Change

### 1. New module

Create `crates/worldwake-ai/src/effect_schema_index.rs`:

```rust
use crate::opportunity_compiler::EffectFactKey;
use std::collections::BTreeMap;
use worldwake_core::ActionDefId;
use worldwake_sim::{ActionDefRegistry, EffectStep};

pub struct EffectSchemaIndex {
    pub by_effect: BTreeMap<EffectFactKey, Vec<ActionDefId>>,
}

impl EffectSchemaIndex {
    pub fn build(registry: &ActionDefRegistry) -> Self {
        let mut by_effect: BTreeMap<EffectFactKey, Vec<ActionDefId>> = BTreeMap::new();
        for action_def in registry.iter() {
            for key in effect_keys_for_steps(&action_def.effect_schema.steps) {
                by_effect.entry(key).or_default().push(action_def.id);
            }
        }
        // Deduplicate per-key in case an action emits the same effect category in multiple steps
        for ids in by_effect.values_mut() {
            ids.sort();
            ids.dedup();
        }
        Self { by_effect }
    }

    pub fn actions_producing(&self, fact: EffectFactKey) -> &[ActionDefId] {
        self.by_effect.get(&fact).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

fn effect_keys_for_steps(steps: &[EffectStep]) -> Vec<EffectFactKey> { /* maps declaration steps */ }
```

The landed implementation uses `ActionDefRegistry::iter()` and the live `EffectStep` declaration shape; `ActionDefRegistry.defs` is private on the current branch.

### 2. Expose in lib.rs

Modify `crates/worldwake-ai/src/lib.rs`:
- Add `pub mod effect_schema_index;`

### 3. Build and store on AgentTickDriver

Modify `crates/worldwake-ai/src/agent_tick/mod.rs:78` (`AgentTickDriver`):
- Add field `effect_schema_index: EffectSchemaIndex`
- Add `AgentTickDriver::new_with_action_defs(registry)` for startup construction
- Add a one-time lazy initialization fallback in `produce_agent_input` for existing `AgentTickDriver::new()` and `from_saved_runtime` paths
- Expose `AgentTickDriver::effect_schema_index()` as the read accessor for ticket 006

The insertion point is `crates/worldwake-ai/src/agent_tick/mod.rs`: the explicit constructor builds immediately, and `produce_agent_input` initializes restored/default drivers before opportunity consumers can read the index.

## Files to Touch

- `crates/worldwake-ai/src/effect_schema_index.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — `pub mod`)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — `AgentTickDriver` field + constructor wiring)
- `AgentTickDriver::new()` callers compiled unchanged; the driver initializes the derived index on first controller use when constructed without a registry.

## Out of Scope

- Reading the index in `compile_opportunities` — lands in ticket 006
- Consulting the index from candidate generation when `relevant_ops` is exhausted — lands in ticket 006
- Per-tick rebuild logic (not required; index is registry-time)

## Acceptance Criteria

### Tests That Must Pass

1. New test: `EffectSchemaIndex::build(registry)` over a known 2-3 action registry maps `EffectFactKey::CommodityTransfer` to the action ids that produce it, and returns an empty slice for `EffectFactKey::WoundApplied` when no action produces wounds
2. New test: index build is deterministic — same registry produces identical `BTreeMap` contents across runs
3. New test: per-key `ActionDefId` list is sorted (BTreeMap-stable plus deduplication)
4. Existing suite: `cargo test -p worldwake-ai`
5. Workspace build: `cargo build --workspace`

### Invariants

1. Index is built at most once per `AgentTickDriver` lifetime — startup constructor path builds immediately, default/restored drivers build once from the first controller context, and there is no per-tick rebuild
2. `BTreeMap` iteration order over `EffectFactKey` is deterministic (relies on `EffectFactKey`'s `Ord` derive from ticket 001)
3. `actions_producing(fact)` for an unknown `EffectFactKey` returns an empty slice, not a panic

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/effect_schema_index.rs` (inline `#[cfg(test)]`) — build + lookup + determinism

### Commands

1. `cargo test -p worldwake-ai effect_schema_index`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-11.

- Added `worldwake-ai::effect_schema_index::EffectSchemaIndex`, exported from `worldwake-ai`, with deterministic `BTreeMap<EffectFactKey, Vec<ActionDefId>>` lookup.
- Built the index from the live `ActionDefRegistry::iter()` and `EffectSchema.steps` declaration shape. `EffectStep` declarations are mapped to the `EffectFactKey` categories they can emit, including nested `PartialOnFailure` declarations and deduplication of repeated categories per action.
- Stored the derived index on `AgentTickDriver`, added `new_with_action_defs()` for explicit startup construction, added a one-time lazy initialization fallback for existing `new()` / restored-runtime paths, and exposed `effect_schema_index()` for ticket 006.
- Kept the index out of `AgentTickDriverState`: it is a derived registry read-model, not persisted runtime state.

## Deviations

- The drafted `step.facts.iter()` implementation was impossible on the live branch because `EffectSchema.steps` contains `EffectStep` declarations, not `EffectFact` outcomes. The landed index maps from declaration steps to possible `EffectFactKey` categories.
- The existing `AgentTickDriver::new()` call surface was preserved to avoid broad constructor churn. `new_with_action_defs()` is available for startup construction, and the default/restored path initializes once from the first `AutonomousControllerContext`.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib effect_schema_index -- --list`
- Passed `cargo test -p worldwake-ai --lib effect_schema_index`
- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo build --workspace`
