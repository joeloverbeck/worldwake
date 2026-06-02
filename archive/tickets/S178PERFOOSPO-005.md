# S178PERFOOSPO-005: GoalBeliefView lot condition accessors and perception write

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `GoalBeliefView` and `InventoryBeliefView` gained `lot_condition`/`lot_freshness_band`/`commodity_perish_profile` accessors; `PerAgentBeliefView` implements co-located authoritative gating through `LocalPhysicalObservationView::observed_lot_condition`; `BelievedEntityState`/`ObservedEntitySnapshot` gained `last_observed_condition: Option<Permille>`; perception stores the field through the existing observed-entity snapshot path when an agent observes a perishable lot.
**Deps**: `archive/tickets/S178PERFOOSPO-001.md`, `archive/tickets/S178PERFOOSPO-003.md`

## Problem

D8 exposes lot freshness to the AI crate through the belief view (never directly through authoritative world state for remote lots). Co-located/possessed lots return the lot's authoritative `PerishableState.condition` (FND-14A); remote lots return the agent's belief-store `last_observed_condition` written by perception when the agent was last co-located with the lot. Without D8, candidate generation (ticket 006) and forensics (ticket 007) cannot read freshness in an FND-14B-compliant way. Mirrors S177's `ReliabilityRecord.last_observed_quality` precedent for belief-stored last-observed perception axes.

## Assumption Reassessment (2026-05-31)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Before this ticket, `GoalBeliefView` exposed `item_lot_commodity` and `item_lot_consumable_profile` for lot reads, but no condition/freshness accessors. `PerAgentBeliefView::observed_item_lot_quantity` was the canonical precedent for co-located authoritative reads — `has_authoritative_local_visibility(lot)` gated a `world.get_component_item_lot(lot)` read, returning `ObservedRead` with `ObservationSource::CoLocatedSameTick`. S177's `ReliabilityRecord.last_observed_quality` remained the precedent for belief-store-stored last-observed-value on a perception axis.
2. Spec D8 verified against current `specs/S178-perishable-food-spoilage.md`. FND-14B mandates belief-backing for remote planner reads; FND-14A allows authoritative reads for co-located/possessed physical properties; FND-7 mandates information locality.
3. Shared abstraction boundary (precision-rules §2 — Layer Precision): the `GoalBeliefView` trait surface (the AI-facing accessor) and the `AgentBeliefStore` lot-belief structure (the belief-storage layer). Both must be updated atomically — adding the accessor without the belief-store field leaves remote reads always returning `None` even after observation; adding the field without the accessor leaves perception writes unreadable. This is a mixed-layer ticket — separate proof surfaces for each layer per Verification Layers below.
4. Information-path contract (precision-rules §16): the lot-condition fact travels through (a) FND-14A co-located authoritative read at action commit (ticket 004's Eat handler, direct `world.get_component_perishable_state`) and (b) perception → belief-store `last_observed_condition` → `GoalBeliefView::lot_condition` for planning-time remote reads (this ticket). Both paths are canonical end-state; the FND-14A path is not removable because Eat at action commit needs authoritative truth, not a stale belief.

## Architecture Check

1. The split between FND-14A authoritative co-located read (ticket 004's Eat at action commit) and FND-14B belief-backed remote read (this ticket's candidate-generation surface) is architecturally clean: action-commit reads see truth (Eat outcome must reflect actual lot condition); planning-time reads see belief (planning a trip to a remote cache must not benefit from omniscient freshness). FND-26 system-via-state — perception writes belief; AI reads belief view; no direct cross-system calls.
2. Following S177's `last_observed_quality` precedent keeps the belief-store extension minimal — one new field on the existing lot-belief entry, written by perception when the agent observes a perishable lot. No new belief-store entry type. `Option<Permille>` is the lawful unknown signal for non-perishable lots and pre-observation cases.

## Outcome

S178 D8 landed. AI-facing belief views now expose perishable lot condition and derived freshness through belief-backed accessors, co-located reads remain authoritative, remote reads use `BelievedEntityState.last_observed_condition`, perception records observed perishable condition through the existing snapshot path, and the save format advanced to 119 for the persisted belief field.

## Verification Result

1. Passed — `GoalBeliefView::lot_condition` returns authoritative `PerishableState.condition` for a co-located perishable lot through `PerAgentBeliefView::observed_lot_condition`; `lot_condition_returns_authoritative_for_co_located_lot` covers this FND-14A path.
2. Passed — remote reads use `BelievedEntityState.last_observed_condition` and do not fall back to authoritative world state; `lot_condition_returns_belief_for_remote_lot_with_prior_observation` and `lot_condition_returns_none_for_remote_lot_with_no_belief` cover the FND-14B path.
3. Passed — `GoalBeliefView::lot_freshness_band` derives `Freshness` from believed condition, lot commodity, and the world perish profile; `lot_freshness_band_derives_from_condition_and_profile` and `lot_freshness_band_returns_none_when_condition_unknown` cover derivation and unknown propagation.
4. Passed — perception stores perishable lot condition through `build_observed_entity_snapshot` and `record_entity_snapshot_claims`, not through a parallel lot-only writer; `perception_writes_last_observed_condition_when_co_located_with_perishable_lot` covers the end-to-end write.
5. Passed — the new persisted belief field required a save-format bump from 118 to 119; `save_format_version_is_119_after_perishable_lot_condition_belief` and the existing full non-default save roundtrip cover the save boundary.

## Implemented Changes

1. `crates/worldwake-sim/src/belief_view.rs` adds `lot_condition`, `commodity_perish_profile`, and default-derived `lot_freshness_band` to `GoalBeliefView` and `InventoryBeliefView`; the blanket `GoalBeliefView` implementation forwards those methods through `InventoryBeliefView`.
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` implements `LocalPhysicalObservationView::observed_lot_condition` with a strict co-location gate for authoritative `PerishableState` reads and a remote fallback only to `AgentBeliefStore`/`BelievedEntityState.last_observed_condition`.
3. `crates/worldwake-core/src/belief.rs` adds `last_observed_condition: Option<Permille>` to `ObservedEntitySnapshot` and `BelievedEntityState`, projects `PerishableState.condition` into observed snapshots, records a `LotCondition` belief claim, and restores the value from claim-derived summaries.
4. `crates/worldwake-core/src/entity_belief_claim.rs` adds `EntityBeliefAspect::LotCondition` and `ClaimValue::LotCondition`; `crates/worldwake-core/src/topic_scope.rs` classifies it as `ResourceAvailability`.
5. `crates/worldwake-systems/src/perception.rs` adds the focused perception regression for co-located perishable lots; no separate production writer was needed because the existing observed-snapshot path now carries the condition field.
6. `crates/worldwake-sim/src/save_load.rs` bumps `SAVE_FORMAT_VERSION` to 119 and updates the version assertion.

## Touched Files

- `crates/worldwake-core/src/belief.rs`
- `crates/worldwake-core/src/entity_belief_claim.rs`
- `crates/worldwake-core/src/event_record.rs`
- `crates/worldwake-core/src/topic_scope.rs`
- `crates/worldwake-sim/src/belief_view.rs`
- `crates/worldwake-sim/src/per_agent_belief_view.rs`
- `crates/worldwake-sim/src/save_load.rs`
- `crates/worldwake-systems/src/perception.rs`

## Out of Scope

- Candidate generation reads (ticket 006).
- Forensic record (ticket 007).
- Eat handler changes (ticket 004 already landed the direct action-commit read).

## Acceptance Result

1. `lot_condition_returns_authoritative_for_co_located_lot` passed.
2. `lot_condition_returns_belief_for_remote_lot_with_prior_observation` passed.
3. `lot_condition_returns_none_for_remote_lot_with_no_belief` passed.
4. `lot_freshness_band_derives_from_condition_and_profile` passed.
5. `perception_writes_last_observed_condition_when_co_located_with_perishable_lot` passed.
6. `lot_freshness_band_returns_none_when_condition_unknown` passed.

### Invariants

1. `lot_condition` never reads authoritative `PerishableState` for a remote lot — co-location gate is strict (FND-14B).
2. `last_observed_condition` is `Option<Permille>` — non-perishable lots and pre-observation cases lawfully signal "unknown" with `None`.
3. `lot_freshness_band` returns `None` whenever `lot_condition` returns `None` (consistent unknown propagation; no fallback to direct world read).
4. The macro-forwarded methods compile and resolve correctly across all `GoalBeliefView` blanket-impl consumers (`worldwake-ai` crate consumes via macro).

## Added/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` adds five tests covering local authoritative condition, remote belief-backed condition, unknown remote condition, freshness-band derivation, and unknown propagation.
2. `crates/worldwake-systems/src/perception.rs` adds `perception_writes_last_observed_condition_when_co_located_with_perishable_lot`.
3. `crates/worldwake-sim/src/save_load.rs` updates the save-format version assertion to 119.

## Commands Result

1. `cargo test -p worldwake-sim per_agent_belief_view::tests::lot_condition` — passed.
2. `cargo test -p worldwake-sim per_agent_belief_view::tests::lot_freshness_band` — passed.
3. `cargo test -p worldwake-systems perception::tests::perception_writes_last_observed_condition_when_co_located_with_perishable_lot` — passed.
4. `cargo test -p worldwake-sim save_format_version_is_119_after_perishable_lot_condition_belief` — passed.
5. `cargo test -p worldwake-sim save_to_bytes_roundtrip_preserves_full_nondefault_state` — passed.
6. `cargo check --workspace` — passed.
7. `cargo test -p worldwake-sim` — passed.
8. `cargo test -p worldwake-systems` — passed.
9. `cargo test --workspace` — passed.
10. `cargo fmt --all` — run before the final workspace test.
11. `./scripts/verify.sh` — not run for this per-ticket closeout; final queued spec closeout owns the full pre-PR wrapper.
