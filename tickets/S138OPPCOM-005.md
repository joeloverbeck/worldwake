# S138OPPCOM-005: EffectSchemaIndex module and simulation-startup build

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — adds a startup-time read-model derived from `ActionDefRegistry`
**Deps**: archive/tickets/S138OPPCOM-001.md (defines `EffectFactKey`)

## Problem

S138's compiler needs to answer "which actions produce this effect?" cheaply per tick. `ActionDef.effect_schema` (landed in S134) carries the typed effect declaration per action. This ticket builds a `BTreeMap<EffectFactKey, Vec<ActionDefId>>` index over the registry once at simulation startup and exposes it as `EffectSchemaIndex`. The index becomes a stable read-model the compiler (ticket 006) consults whenever a goal's `relevant_ops` hint is exhausted.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage: `crates/worldwake-sim/src/effect_schema.rs:209` defines `EffectFact` with 6 variants; `crates/worldwake-sim/src/action_def.rs:144` defines `ActionDef.effect_schema: EffectSchema`; `crates/worldwake-sim/src/action_def_registry.rs:6` exposes `ActionDefRegistry` with `defs: Vec<ActionDef>`.
2. Spec/doc reference: `specs/S138-opportunity-compiler.md` deliverable section "`worldwake-ai::effect_schema_index` (new module)".
3. Shared abstraction boundary: ai-side index consumes sim's `ActionDefRegistry` as a read-only input — one-way data flow, no cross-crate mutation.
4. Where to build and store the index: the spec says "built once at simulation startup". `AgentTickDriver` at `crates/worldwake-ai/src/agent_tick/mod.rs:78` is the persistent ai-side driver constructed once per simulation; storing the index as a `AgentTickDriver` field built in `AgentTickDriver::new()` matches the spec's intent and avoids per-tick rebuilds. Confirm `AgentTickDriver::new()` has access to the registry (or accepts it as a constructor parameter).
5. FND-26 (Systems through state): the index reads only the action registry — no cross-system imperative; the registry itself is set up once at simulation construction.

## Architecture Check

1. Startup-once construction matches the spec's no-per-tick-rebuild rule (FND-12, performance compresses computation, never causality) — the index is a derived read-model over registry state that itself never changes during a simulation run.
2. `BTreeMap` key (`EffectFactKey`) and `Vec` of `ActionDefId` preserve determinism: iteration order is `EffectFactKey`'s `Ord` (defined in ticket 001).
3. Empty-index behavior is well-defined: `actions_producing(fact)` returns an empty slice when no action produces the effect, which is the expected case for `EffectFactKey::PartialQuantity` and other effects that don't appear as a primary product of any action.
4. No backward-compatibility shim: the index is brand-new; nothing aliases or wraps it.

## Verification Layers

1. Index build over a known small registry — focused unit test constructing a 2-3 action registry and asserting the index maps each `EffectFactKey` to the right `ActionDefId`s
2. Empty-index lookup for an effect with no producer — focused unit test
3. Iteration order is `BTreeMap`-stable — focused unit test using two different insertion orders (via different `Vec<ActionDef>` orderings) and asserting identical lookup results
4. Determinism — workspace test `cargo test --workspace`

## What to Change

### 1. New module

Create `crates/worldwake-ai/src/effect_schema_index.rs`:

```rust
use crate::opportunity_compiler::EffectFactKey;
use std::collections::BTreeMap;
use worldwake_sim::{ActionDefId, ActionDefRegistry, EffectFact, EffectSchema};

pub struct EffectSchemaIndex {
    pub by_effect: BTreeMap<EffectFactKey, Vec<ActionDefId>>,
}

impl EffectSchemaIndex {
    pub fn build(registry: &ActionDefRegistry) -> Self {
        let mut by_effect: BTreeMap<EffectFactKey, Vec<ActionDefId>> = BTreeMap::new();
        for action_def in registry.defs.iter() {
            for step in &action_def.effect_schema.steps {
                for fact in step.facts.iter() {
                    let key = effect_fact_to_key(fact);
                    by_effect.entry(key).or_default().push(action_def.id);
                }
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

fn effect_fact_to_key(fact: &EffectFact) -> EffectFactKey {
    match fact {
        EffectFact::CommodityTransfer { .. } => EffectFactKey::CommodityTransfer,
        EffectFact::PartialQuantity { .. } => EffectFactKey::PartialQuantity,
        EffectFact::WoundApplied { .. } => EffectFactKey::WoundApplied,
        EffectFact::ExpectationFulfilled { .. } => EffectFactKey::ExpectationFulfilled,
        EffectFact::ContentionGrantConsumed { .. } => EffectFactKey::ContentionGrantConsumed,
        EffectFact::EventEmitted { .. } => EffectFactKey::EventEmitted,
    }
}
```

The exact `EffectSchema` iteration shape (`step.facts.iter()` vs. some other accessor) must be confirmed during implementation against `crates/worldwake-sim/src/effect_schema.rs:9-22`.

### 2. Expose in lib.rs

Modify `crates/worldwake-ai/src/lib.rs`:
- Add `pub mod effect_schema_index;`

### 3. Build and store on AgentTickDriver

Modify `crates/worldwake-ai/src/agent_tick/mod.rs:78` (`AgentTickDriver`):
- Add field `effect_schema_index: EffectSchemaIndex`
- In `AgentTickDriver::new()`, accept or reach the `ActionDefRegistry` and call `EffectSchemaIndex::build(registry)` once
- If `AgentTickDriver::new()` does not currently accept the registry, thread it through the call chain (likely via `SimulationState` or via constructor parameter — confirm during implementation)

`Likely: crates/worldwake-ai/src/agent_tick/mod.rs construction site` — pin the exact insertion point in assumption reassessment during /implement-ticket.

## Files to Touch

- `crates/worldwake-ai/src/effect_schema_index.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — `pub mod`)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — `AgentTickDriver` field + constructor wiring)
- Likely: caller(s) of `AgentTickDriver::new()` — confirm during implementation (grep `AgentTickDriver::new`)

## Out of Scope

- Reading the index in `compile_opportunities` — lands in ticket 006
- Consulting the index from candidate generation when `relevant_ops` is exhausted — lands in ticket 006
- Per-tick rebuild logic (not required; index is registry-time)

## Acceptance Criteria

### Tests That Must Pass

1. New test: `EffectSchemaIndex::build(registry)` over a known 2-3 action registry maps `EffectFactKey::CommodityTransfer` to the action ids that produce it, and returns an empty slice for `EffectFactKey::WoundApplied` when no action produces wounds
2. New test: index build is deterministic — same registry produces byte-identical index across runs
3. New test: per-key `ActionDefId` list is sorted (BTreeMap-stable plus deduplication)
4. Existing suite: `cargo test -p worldwake-ai`
5. Workspace build: `cargo build --workspace`

### Invariants

1. Index is built exactly once per `AgentTickDriver` lifetime — no per-tick rebuild
2. `BTreeMap` iteration order over `EffectFactKey` is deterministic (relies on `EffectFactKey`'s `Ord` derive from ticket 001)
3. `actions_producing(fact)` for an unknown `EffectFactKey` returns an empty slice, not a panic

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/effect_schema_index.rs` (inline `#[cfg(test)]`) — build + lookup + determinism

### Commands

1. `cargo test -p worldwake-ai effect_schema_index`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
