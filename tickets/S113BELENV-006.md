# S113BELENV-006: Claim-level refutation carriage for live `BeliefStatus::Contradicted`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — belief-store contradiction substrate, envelope derivation consumers
**Deps**: archive/tickets/S113BELENV-001.md

## Problem

`S113BELENV-001` lands the belief-envelope foundation and keeps `BeliefStatus::Contradicted` in the public taxonomy, but the live branch still has no explicit claim-level refutation carrier on `AgentBeliefStore` / `EntityBeliefClaim`. Without that substrate, envelope reads cannot honestly distinguish "stale belief" from "later evidence explicitly refuted this claim", and downstream consumers such as revalidation and remote-target emitters cannot implement their `Contradicted` branches without guessing from unrelated discrepancy memory or authoritative world state.

This ticket adds the minimal authoritative contradiction substrate needed for envelope derivation and wires that substrate into the existing belief-envelope readers so `BeliefStatus::Contradicted` becomes live behavior rather than staged API surface.

## Assumption Reassessment (2026-04-21)

1. `EntityBeliefClaim` in `crates/worldwake-core/src/entity_belief_claim.rs` currently stores `claim_id`, `subject`, `aspect`, `value`, `source`, `acquired_tick`, `claimed_event_tick`, and `confidence`; there is no explicit refutation / contradiction marker on the claim itself.
2. `AgentBeliefStore` in `crates/worldwake-core/src/belief.rs` stores raw claims and derived `known_entities`, but no dedicated per-claim contradiction lane. Existing contradiction-related runtime state such as `DiscrepancyMemory` in `worldwake-ai` is AI recovery state, not honest belief-store provenance.
3. Shared abstraction boundary under audit: authoritative belief-store contradiction carriage -> envelope derivation in `worldwake-sim` -> downstream planner/runtime consumers that branch on `BeliefStatus::Contradicted`.
4. `S113BELENV-003` and `S113BELENV-004` already depend on contradiction-aware envelope reads for their revalidation / emitter skip paths. This ticket is the missing substrate owner for those pending branches.
5. This is a mixed-layer ticket: authoritative belief-store schema plus belief-view projection. Focused proof must separate storage/round-trip coverage from envelope-derivation coverage.

## Architecture Check

1. Adding explicit contradiction carriage to the belief store is cleaner than inferring contradiction from world-state mismatches or AI discrepancy memory after the fact. The envelope should read stored epistemic state, not reconstruct it from downstream consequences.
2. The contradiction carrier should stay claim-scoped or claim-key-scoped, not global per-subject, so later contradictory evidence can coexist with unresolved alternatives without erasing provenance.

## Verification Layers

1. Refutation carrier persists through belief-store serialization / round-trip -> focused `worldwake-core` unit tests.
2. Envelope derivation maps refuted claims to `BeliefStatus::Contradicted` regardless of effective confidence -> focused `worldwake-sim` tests.
3. Existing non-refuted stale/probable/certain paths remain unchanged -> focused regression tests on the same accessors.

## What to Change

### 1. Add explicit contradiction carriage to the belief store

Add the minimal authoritative carrier needed to mark a claim (or claim-key lane) as explicitly refuted by later evidence. Keep the shape deterministic and serializable.

### 2. Wire contradiction carriage into envelope derivation

Update the belief-envelope helpers/accessors from `S113BELENV-001` so refuted claims derive `BeliefStatus::Contradicted` before banded freshness status.

### 3. Unblock contradiction-aware consumers

Reassess `S113BELENV-003` and `S113BELENV-004` after the substrate lands so their contradiction branches can depend on live envelope behavior instead of staged taxonomy.

## Files to Touch

- `crates/worldwake-core/src/entity_belief_claim.rs` (modify or add adjacent carrier)
- `crates/worldwake-core/src/belief.rs` (modify — store/update/round-trip contradiction carriage)
- `crates/worldwake-core/src/lib.rs` (modify — re-export if needed)
- `crates/worldwake-sim/src/belief_view.rs` (modify — derive `BeliefStatus::Contradicted`)

## Out of Scope

- Golden coverage for stale-belief motive scaling (`S113BELENV-005`)
- Decision-event payload snapshots (`S113BELENV-002`)
- New discrepancy enum variants

## Acceptance Criteria

### Tests That Must Pass

1. Focused `worldwake-core` contradiction-carrier tests pass.
2. Focused `worldwake-sim` envelope contradiction-derivation tests pass.
3. Existing `cargo test -p worldwake-core` and `cargo test -p worldwake-sim` pass.

### Invariants

1. `BeliefStatus::Contradicted` is derived from explicit stored contradiction provenance, not guessed from authoritative world state.
2. Non-refuted stale/probable/certain derivation from `effective_claim_confidence` remains unchanged.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — contradiction-carrier storage / round-trip tests.
2. `crates/worldwake-sim/src/belief_view.rs` — contradiction-derivation tests.

### Commands

1. `cargo test -p worldwake-core belief`
2. `cargo test -p worldwake-sim belief_view`
3. `cargo test -p worldwake-core`
4. `cargo test -p worldwake-sim`
