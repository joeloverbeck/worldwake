# S113BELENV-006: Claim-level refutation carriage for live `BeliefStatus::Contradicted`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `EntityBeliefClaim` contradiction provenance, belief-store refutation helpers, envelope derivation, save format version bump
**Deps**: archive/tickets/S113BELENV-001.md

## Problem

`S113BELENV-001` lands the belief-envelope foundation and keeps `BeliefStatus::Contradicted` in the public taxonomy, but the live branch still has no explicit claim-level refutation carrier on `AgentBeliefStore` / `EntityBeliefClaim`. Without that substrate, envelope reads cannot honestly distinguish "stale belief" from "later evidence explicitly refuted this claim", and downstream consumers such as revalidation and remote-target emitters cannot implement their `Contradicted` branches without guessing from unrelated discrepancy memory or authoritative world state.

This ticket adds the minimal authoritative contradiction substrate needed for envelope derivation and wires that substrate into the existing belief-envelope readers so `BeliefStatus::Contradicted` becomes live behavior rather than staged API surface.

## Assumption Reassessment (2026-04-21)

1. `EntityBeliefClaim` in `crates/worldwake-core/src/entity_belief_claim.rs` currently stores `claim_id`, `subject`, `aspect`, `value`, `source`, `acquired_tick`, `claimed_event_tick`, and `confidence`; there is no explicit refutation / contradiction marker on the claim itself.
2. `AgentBeliefStore` in `crates/worldwake-core/src/belief.rs` stores raw claims and derived `known_entities`, but no dedicated per-claim contradiction lane. The live envelope does expose `BeliefStatus::Contradicted` in `crates/worldwake-sim/src/belief_view.rs`, yet current projection helpers never derive it; the nearby `discrepancy_memory()` accessor is available on the view surface, but envelope projection does **not** currently read it. Existing `DiscrepancyMemory` state in `worldwake-ai` is AI recovery state, not honest belief-store provenance.
3. Shared abstraction boundary under audit: authoritative belief-store contradiction carriage on `EntityBeliefClaim` / `AgentBeliefStore` -> envelope derivation in `worldwake-sim` -> downstream planner/runtime consumers that branch on `BeliefStatus::Contradicted`.
4. Adding contradiction provenance to `EntityBeliefClaim` changes persisted world shape through `AgentBeliefStore`, so this ticket must also bump `SAVE_FORMAT_VERSION` in `crates/worldwake-sim/src/save_load.rs` from `35` to `36` and prove a non-default round-trip with a refuted claim. `#[serde(default)]` on the new field is still useful intra-head, but it is not a cross-version migration path under repo policy.
5. `S113BELENV-003` and `S113BELENV-004` already depend on contradiction-aware envelope reads for their revalidation / emitter skip paths. This ticket is the missing substrate owner for those pending branches, and it should provide both a stored contradiction carrier and a lawful belief-store mutation seam future runtime producers can call.
6. This is a mixed-layer ticket: authoritative belief-store schema plus belief-view projection plus save-format boundary. Focused proof must separate claim/storage behavior, save/load round-trip, and envelope-derivation coverage.

## Architecture Check

1. Adding explicit contradiction carriage to the belief store is cleaner than inferring contradiction from world-state mismatches or AI discrepancy memory after the fact. The envelope should read stored epistemic state, not reconstruct it from downstream consequences.
2. The contradiction carrier should stay claim-scoped or claim-key-scoped, not global per-subject, so later contradictory evidence can coexist with unresolved alternatives without erasing provenance.
3. Contradicted claims must not keep winning ordinary non-envelope summary derivation in `derive_entity_summary`; otherwise `known_entities` would continue to treat explicitly refuted claims as current truth. The read-model split stays clean only if contradiction history remains stored while non-envelope summaries ignore refuted claims.
4. No backward-compatibility shim: persisted-shape change is handled by a save-format bump, not by pretending older saves deserialize into the new claim shape.

## Verification Layers

1. Claim/store refutation carrier persists through `EntityBeliefClaim` / `AgentBeliefStore` serialization and helper mutation -> focused `worldwake-core` unit tests.
2. Persisted world/save shape with a refuted claim round-trips under the bumped format version -> focused `worldwake-sim` save/load test.
3. Envelope derivation maps refuted claims to `BeliefStatus::Contradicted` regardless of effective confidence -> focused `worldwake-sim` tests.
4. Existing non-refuted stale/probable/certain paths remain unchanged -> focused regression tests on the same accessors.

## What to Change

### 1. Add explicit contradiction carriage to `EntityBeliefClaim`

Add the minimal authoritative carrier needed to mark a claim as explicitly refuted by later evidence. Keep the shape deterministic and serializable, and use `#[serde(default)]` on the new field so current-head decode remains robust while the save format still bumps.

### 2. Add belief-store helper(s) that can refute a claim lane honestly

In `crates/worldwake-core/src/belief.rs`:

- Add the narrowest public helper needed to mark claims in a `(subject, aspect)` lane as explicitly refuted at a given tick.
- Also make `record_entity_claim` handle the direct-observation supersession case honestly: when a later direct observation records a conflicting value for the same `(subject, aspect)`, preserve the older claim as contradicted history instead of silently erasing it.
- Keep same-value dominance cleanup in place so the store does not accumulate duplicate non-contradictory history unnecessarily.

### 3. Keep ordinary summary derivation honest

Update `derive_entity_summary`, claim-pruning, and any nearby helper paths in `crates/worldwake-core/src/belief.rs` so explicitly refuted claims do not continue to win `known_entities` summary projection, while contradiction history remains available to the envelope.

### 4. Wire contradiction carriage into envelope derivation

Update the belief-envelope helpers/accessors from `S113BELENV-001` so refuted claims derive `BeliefStatus::Contradicted` before banded freshness status. When non-refuted claims exist, they remain the active candidates; contradicted claims are historical fallbacks rather than ordinary winners.

### 5. Bump `SAVE_FORMAT_VERSION` and prove persisted round-trip

In `crates/worldwake-sim/src/save_load.rs`:

- Bump `SAVE_FORMAT_VERSION` from `35` to `36`.
- Add or update a save/load fixture so a belief store containing a refuted claim survives round-trip under the new version.

### 6. Unblock contradiction-aware consumers

Reassess `S113BELENV-003` and `S113BELENV-004` after the substrate lands so their contradiction branches can depend on stored contradiction provenance instead of staged taxonomy.

## Files to Touch

- `crates/worldwake-core/src/entity_belief_claim.rs` (modify — persisted contradiction provenance on the claim)
- `crates/worldwake-core/src/belief.rs` (modify — contradiction helpers, direct-observation supersession, summary/prune behavior, round-trip tests)
- `crates/worldwake-sim/src/belief_view.rs` (modify — derive `BeliefStatus::Contradicted`)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` bump plus refuted-claim round-trip proof)
- current-ticket fallout in test/sample builders that construct `EntityBeliefClaim` manually across `worldwake-core`, `worldwake-sim`, and `worldwake-ai`

## Out of Scope

- Golden coverage for stale-belief motive scaling (`S113BELENV-005`)
- Decision-event payload snapshots (`S113BELENV-002`)
- New discrepancy enum variants

## Acceptance Criteria

### Tests That Must Pass

1. Focused `worldwake-core` contradiction-carrier / summary-behavior tests pass.
2. Focused `worldwake-sim` save/load round-trip with a refuted claim passes at `SAVE_FORMAT_VERSION = 36`.
3. Focused `worldwake-sim` envelope contradiction-derivation tests pass.
4. Existing `cargo test -p worldwake-core` and `cargo test -p worldwake-sim` pass.

### Invariants

1. `BeliefStatus::Contradicted` is derived from explicit stored contradiction provenance on the claim/store, not guessed from authoritative world state or AI discrepancy memory.
2. Contradicted claims do not remain the winning source of truth for ordinary non-envelope `BelievedEntityState` summary derivation.
3. Non-refuted stale/probable/certain derivation from `effective_claim_confidence` remains unchanged.
4. Persisted saves with the new claim shape are versioned honestly via `SAVE_FORMAT_VERSION = 36`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — contradiction-carrier helper, direct-observation supersession, summary, and round-trip tests.
2. `crates/worldwake-sim/src/belief_view.rs` — contradiction-derivation tests.
3. `crates/worldwake-sim/src/save_load.rs` — refuted-claim save/load round-trip plus version update.

### Commands

1. `cargo test -p worldwake-core belief`
2. `cargo test -p worldwake-sim belief_view`
3. `cargo test -p worldwake-sim save_load`
4. `cargo test -p worldwake-core`
5. `cargo test -p worldwake-sim`
6. `cargo fmt --all`

## Outcome

Completion date: 2026-04-21

Implemented the live contradiction substrate on the authoritative belief-store boundary.

- `EntityBeliefClaim` now carries persisted `refuted_at_tick` provenance.
- `AgentBeliefStore` now exposes `refute_entity_claims(...)`, preserves conflicting older direct-observation claims as contradicted history, keeps same-value dominance cleanup, and excludes refuted claims from ordinary `derive_entity_summary(...)` winners.
- Belief pruning now keeps contradicted claims available as historical envelope inputs even after ordinary summary projection drops them.
- `worldwake-sim` belief-envelope projection now derives `BeliefStatus::Contradicted` from stored refutation provenance and prefers active claims over contradicted history when both exist.
- `SAVE_FORMAT_VERSION` was bumped from `35` to `36`, and save/load coverage now proves a non-default `refuted_at_tick` survives round-trip.
- Manual `EntityBeliefClaim` constructors in `worldwake-core`, `worldwake-sim`, and `worldwake-ai` test/sample helpers were updated for the new persisted field.

Deviations from original plan:

- No additional runtime AI producers beyond direct-observation supersession and the explicit belief-store helper landed here; downstream envelope consumers remain owned by `S113BELENV-003` and `S113BELENV-004`.

## Verification Result

Passed:

1. `cargo test -p worldwake-core belief`
2. `cargo test -p worldwake-sim belief_view`
3. `cargo test -p worldwake-sim save_load`
4. `cargo fmt --all`
5. `cargo test -p worldwake-core`
6. `cargo test -p worldwake-sim`

Not run:

- `cargo clippy --workspace --all-targets -- -D warnings`
- `./scripts/verify.sh`
