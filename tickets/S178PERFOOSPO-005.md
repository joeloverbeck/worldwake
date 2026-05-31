# S178PERFOOSPO-005: GoalBeliefView lot condition accessors and perception write

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `GoalBeliefView` gains `lot_condition`/`lot_freshness_band`/`commodity_perish_profile` accessors; `RuntimeBeliefView` + `PerAgentBeliefView` implement co-located authoritative gating; `AgentBeliefStore` lot-belief gains `last_observed_condition: Option<Permille>`; perception writes the new field when the agent is co-located with a perishable lot.
**Deps**: 001

## Problem

D8 exposes lot freshness to the AI crate through the belief view (never directly through authoritative world state for remote lots). Co-located/possessed lots return the lot's authoritative `PerishableState.condition` (FND-14A); remote lots return the agent's belief-store `last_observed_condition` written by perception when the agent was last co-located with the lot. Without D8, candidate generation (ticket 006) and forensics (ticket 007) cannot read freshness in an FND-14B-compliant way. Mirrors S177's `ReliabilityRecord.last_observed_quality` precedent for belief-stored last-observed perception axes.

## Assumption Reassessment (2026-05-31)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:481-482` currently exposes `item_lot_commodity` and `item_lot_consumable_profile` for lot reads; no condition/freshness accessors. `PerAgentBeliefView::observed_item_lot_quantity` at `crates/worldwake-sim/src/per_agent_belief_view.rs:525-538` is the canonical precedent for co-located authoritative reads — `has_authoritative_local_visibility(lot)` (lines 282-299) gates a `world.get_component_item_lot(lot)` read, returning `ObservedRead` with `ObservationSource::CoLocatedSameTick`. S177's `ReliabilityRecord.last_observed_quality` is the precedent for belief-store-stored last-observed-value on a perception axis (located in survival forensics + perception).
2. Spec D8 verified against current `specs/S178-perishable-food-spoilage.md`. FND-14B mandates belief-backing for remote planner reads; FND-14A allows authoritative reads for co-located/possessed physical properties; FND-7 mandates information locality.
3. Shared abstraction boundary (precision-rules §2 — Layer Precision): the `GoalBeliefView` trait surface (the AI-facing accessor) and the `AgentBeliefStore` lot-belief structure (the belief-storage layer). Both must be updated atomically — adding the accessor without the belief-store field leaves remote reads always returning `None` even after observation; adding the field without the accessor leaves perception writes unreadable. This is a mixed-layer ticket — separate proof surfaces for each layer per Verification Layers below.
4. Information-path contract (precision-rules §16): the lot-condition fact travels through (a) FND-14A co-located authoritative read at action commit (ticket 004's Eat handler, direct `world.get_component_perishable_state`) and (b) perception → belief-store `last_observed_condition` → `GoalBeliefView::lot_condition` for planning-time remote reads (this ticket). Both paths are canonical end-state; the FND-14A path is not removable because Eat at action commit needs authoritative truth, not a stale belief.

## Architecture Check

1. The split between FND-14A authoritative co-located read (ticket 004's Eat at action commit) and FND-14B belief-backed remote read (this ticket's candidate-generation surface) is architecturally clean: action-commit reads see truth (Eat outcome must reflect actual lot condition); planning-time reads see belief (planning a trip to a remote cache must not benefit from omniscient freshness). FND-26 system-via-state — perception writes belief; AI reads belief view; no direct cross-system calls.
2. Following S177's `last_observed_quality` precedent keeps the belief-store extension minimal — one new field on the existing lot-belief entry, written by perception when the agent observes a perishable lot. No new belief-store entry type. `Option<Permille>` is the lawful unknown signal for non-perishable lots and pre-observation cases.

## Verification Layers

1. `GoalBeliefView::lot_condition` returns authoritative `condition` for co-located lot → focused unit test on `PerAgentBeliefView::observed_lot_condition` with `has_authoritative_local_visibility(lot) == true`.
2. `GoalBeliefView::lot_condition` returns belief-store `last_observed_condition` for remote lot when perception has written it → focused unit test with a fixture that writes belief and then queries with the agent at a different place.
3. `GoalBeliefView::lot_condition` returns `None` for remote lot when no belief exists → focused unit test (FND-14B compliance — no fallback to authoritative remote read).
4. Perception writes `last_observed_condition` when agent is co-located with a perishable lot → perception trace assertion (mirror S177's `last_observed_quality` perception-write test pattern).
5. `lot_freshness_band` returns `None` whenever `lot_condition` returns `None` → derivation consistency unit test.

## What to Change

### 1. `GoalBeliefView` trait method additions

In `crates/worldwake-sim/src/belief_view.rs`, add to the `GoalBeliefView` trait:

```rust
fn lot_condition(&self, lot: EntityId) -> Option<Permille>;
fn commodity_perish_profile(&self, commodity: CommodityKind) -> Option<CommodityPerishProfile>;

fn lot_freshness_band(&self, lot: EntityId) -> Option<Freshness> {
    let condition = self.lot_condition(lot)?;
    let commodity = self.item_lot_commodity(lot)?;
    let profile = self.commodity_perish_profile(commodity)?;
    Some(Freshness::derive_from(condition, &profile))
}
```

`lot_freshness_band` is a default trait method derived from `lot_condition` + `commodity_perish_profile`.

### 2. `RuntimeBeliefView` impl

Implement `lot_condition` and `commodity_perish_profile` on `RuntimeBeliefView`, routing through `PerAgentBeliefView`:

```rust
fn lot_condition(&self, lot: EntityId) -> Option<Permille> {
    self.per_agent_view().observed_lot_condition(lot).into_value()
}

fn commodity_perish_profile(&self, commodity: CommodityKind) -> Option<CommodityPerishProfile> {
    self.world().commodity_perish_profiles().get(&commodity).copied()
}
```

### 3. `PerAgentBeliefView::observed_lot_condition`

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, mirror the `observed_item_lot_quantity` pattern at lines 525-538:

```rust
impl PerAgentBeliefView<'_> {
    fn observed_lot_condition(&self, lot: EntityId) -> ObservedRead<Option<Permille>> {
        if self.has_authoritative_local_visibility(lot) {
            return ObservedRead {
                value: self.world.get_component_perishable_state(lot).map(|s| s.condition),
                observed_tick: self.current_tick,
                source: ObservationSource::CoLocatedSameTick,
            };
        }
        let belief_value = self.agent_belief_store(self.agent)
            .and_then(|store| store.lot_belief(lot))
            .and_then(|b| b.last_observed_condition);
        ObservedRead {
            value: belief_value,
            observed_tick: self.current_tick,
            source: ObservationSource::BeliefStore,
        }
    }
}
```

### 4. `AgentBeliefStore` lot-belief `last_observed_condition` field

Locate the lot-belief entry type in `crates/worldwake-core/src/belief.rs` (the canonical `AgentBeliefStore` location). Add:

```rust
#[serde(default)]
pub last_observed_condition: Option<Permille>,
```

`Option<Permille>` because lot-belief entries for non-perishable commodities never get a condition write; `None` is the lawful unknown signal. The `#[serde(default)]` annotation lets ticket 001's `SAVE_FORMAT_VERSION=116` bump cover this field without a separate bump.

### 5. Perception write of `last_observed_condition`

Locate the perception write path that updates lot-belief entries (the S177 analog at `crates/worldwake-systems/src/perception.rs` or wherever the perception system lives — grep `last_observed_quality` to find the parallel write site). When the agent observes a lot that has a `PerishableState` component, write the lot's current `condition` to `last_observed_condition` in the belief store. The write occurs at the same perception step that writes `last_observed_quality` for water lots.

### 6. `impl_goal_belief_view!` macro forwarding

Extend the `impl_goal_belief_view!` macro (or equivalent blanket-impl helper in `crates/worldwake-sim/src/belief_view.rs`) to forward `lot_condition`, `lot_freshness_band`, and `commodity_perish_profile` to the underlying `BeliefView` so blanket impls compile across consumer crates.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait methods + macro forwarding + `RuntimeBeliefView` impl)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — add `observed_lot_condition` mirroring `observed_item_lot_quantity` at lines 525-538)
- `crates/worldwake-core/src/belief.rs` (modify — `last_observed_condition: Option<Permille>` field on lot-belief entry)
- Likely: `crates/worldwake-systems/src/perception.rs` (modify — perception write site for `last_observed_condition`; discover via `rg 'last_observed_quality' crates/worldwake-systems/` at implementation time)
- To be confirmed: `crates/worldwake-core/src/belief.rs` `BeliefStoreDiff` may need a `last_observed_condition` diff field; check during implementation per spot-check (d) sub-rule

## Out of Scope

- Candidate generation reads (ticket 006).
- Forensic record (ticket 007).
- Eat handler (ticket 004 — uses direct authoritative read at action commit, not belief view).
- `SAVE_FORMAT_VERSION` bump (rides ticket 001's 115→116 via `#[serde(default)]` on the new belief-store field).

## Acceptance Criteria

### Tests That Must Pass

1. `lot_condition_returns_authoritative_for_co_located_lot` — agent at the same place as a perishable lot reads `world.get_component_perishable_state(lot).condition` value via `lot_condition`.
2. `lot_condition_returns_belief_for_remote_lot_with_prior_observation` — agent who previously observed the lot (perception wrote `last_observed_condition`) reads belief-stored value via `lot_condition` after moving to a different place.
3. `lot_condition_returns_none_for_remote_lot_with_no_belief` — agent who never observed the lot gets `None` (FND-14B compliance).
4. `lot_freshness_band_derives_from_condition_and_profile` — derived band matches `Freshness::derive_from(condition, profile)` across the three band boundaries.
5. `perception_writes_last_observed_condition_when_co_located_with_perishable_lot` — perception write site test asserts belief-store field updated.
6. `lot_freshness_band_returns_none_when_condition_unknown` — consistent unknown propagation.

### Invariants

1. `lot_condition` never reads authoritative `PerishableState` for a remote lot — co-location gate is strict (FND-14B).
2. `last_observed_condition` is `Option<Permille>` — non-perishable lots and pre-observation cases lawfully signal "unknown" with `None`.
3. `lot_freshness_band` returns `None` whenever `lot_condition` returns `None` (consistent unknown propagation; no fallback to direct world read).
4. The macro-forwarded methods compile and resolve correctly across all `GoalBeliefView` blanket-impl consumers (`worldwake-ai` crate consumes via macro).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]` — 4 unit tests for the four condition-read cases (co-located, remote-with-belief, remote-without-belief, non-perishable-lot).
2. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` — 2 unit tests for `lot_freshness_band` derivation and unknown propagation.
3. Perception write test in `crates/worldwake-systems/src/perception.rs` `#[cfg(test)]` (or the file located via grep) — 1 unit test for the write site.

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view::tests::lot_condition`
2. `cargo test -p worldwake-sim belief_view::tests::lot_freshness_band`
3. `cargo test --workspace`
4. `./scripts/verify.sh`
