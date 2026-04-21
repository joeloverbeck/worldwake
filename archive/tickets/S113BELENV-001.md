# S113BELENV-001: Envelope foundation — `BeliefValue<T>`, `BeliefSet<T>`, and three new planner-facing accessors

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — belief-view trait surface (`worldwake-sim/src/belief_view.rs`), runtime belief implementations (`worldwake-sim/src/per_agent_belief_view.rs`, `worldwake-ai/src/planning_state.rs`, `worldwake-ai/src/planning_snapshot.rs`), public confidence helper (`worldwake-core/src/belief.rs`)
**Deps**: None

## Problem

The planner today has no belief-store accessor for "where do I believe target X is?", "which entities do I believe are at remote place P?", or "how much commodity Q do I believe is at place P?". Existing accessors (`entities_at`, `locally_observed_entities_at`, `commodity_quantity`, `locally_observed_commodity_quantity` in `crates/worldwake-sim/src/belief_view.rs`) read authoritative world state or same-tick co-located perception (FND-14A); `pursuit_belief.rs::last_known_place` is a single-target pursuit slot, not a general belief query. As a result, agents cannot plan from remote rumor or stale testimony, and the planner cannot distinguish "high-confidence recent observation" from "decayed stale claim" because every belief projection today is crisp.

S113 introduces `BeliefValue<T>`, `BeliefSet<T>`, and `BeliefStatus` as read-model wrappers and adds three new envelope-typed accessors. This ticket lands the types, the three accessors, the staleness derivation, and the alternatives aggregation rule together because none of them are independently usable — the types are inert without accessors, and the accessors cannot be written without the derivation rule.

## Assumption Reassessment (2026-04-21)

1. `EntityBeliefClaim` (`crates/worldwake-core/src/entity_belief_claim.rs:47-56`) already stores `acquired_tick: Tick`, `claimed_event_tick: Option<Tick>`, `confidence: Permille`, `source: PerceptionSource`, and `value: ClaimValue`. The envelope exposes existing data; no new storage is added. `AgentBeliefStore.entity_claims: BTreeMap<EntityId, Vec<EntityBeliefClaim>>` (`crates/worldwake-core/src/belief.rs:45-46`) already supports multi-source disagreement per entity.
2. `effective_claim_confidence` lives at `crates/worldwake-core/src/belief.rs:2280` as a private `fn`. Cross-crate use from `worldwake-sim` requires promoting it to `pub`. Signature: `fn effective_claim_confidence(claim: &EntityBeliefClaim, current_tick: Tick, policy: &BeliefConfidencePolicy) -> u16`. Semantics: single-claim per-tick staleness decay; **no multi-claim aggregation** — the alternatives rule here does not claim this helper performs aggregation.
3. The belief-view hierarchy in `crates/worldwake-sim/src/belief_view.rs` has sub-traits at lines 464 (`EntityBeliefView`), 544 (`SpatialBeliefView`), 614 (`InventoryBeliefView`), plus a composite `GoalBeliefView` that sees the agent's accessible belief state. The concrete runtime implementation boundary is broader than the draft: `PerAgentBeliefView` lives in `crates/worldwake-sim/src/per_agent_belief_view.rs`, and planner snapshot parity lives in `crates/worldwake-ai/src/planning_snapshot.rs` + `crates/worldwake-ai/src/planning_state.rs`. Shared abstraction boundary under audit: the belief-view trait surface (not world state), the `EntityBeliefClaim` read model, and the planner snapshot carriage needed to keep the same accessors available through `RuntimeBeliefView`.
4. The current belief-view surface exposes `belief_confidence_policy` but **not** the actor's `claim_confidence_threshold`, while S113's status bands are defined against that threshold. T001 therefore must add a threshold read to the social/goal belief surface and carry it into `PlanningSnapshot`; otherwise `Certain` vs `Probable` vs `Stale` cannot be derived honestly in planner state.
5. `AgentBeliefStore` currently has no explicit per-claim refutation / contradiction flag. `EntityBeliefClaim` stores `claim_id`, `aspect`, `value`, source metadata, event/acquisition ticks, and confidence only. The drafted `Contradicted` passthrough is therefore not implementable on the live branch without widening into new authoritative substrate. This ticket narrows to the honest foundational slice: land `BeliefStatus::Contradicted` as staged API surface, but derive only `Certain` / `Probable` / `Stale` / `Disputed` here and create a follow-up ticket for claim-level refutation carriage.
6. `BeliefSet<T>` remains useful staged substrate for disagreement-preserving projection, but none of the three current accessors expose `BeliefSet<T>` directly. The live slice is: add the type now, use it internally to preserve the alternatives/disputed rule where applicable, and return the winning `BeliefValue<T>` (or `Vec<BeliefValue<EntityId>>`) from the three accessor signatures already consumed by sibling tickets.
7. This is an AI-planning-layer ticket — intended proof surfaces are focused helper tests in `belief_view.rs`, runtime tests in `per_agent_belief_view.rs`, and snapshot-parity tests in `planning_state.rs`. No `agent_tick` or golden coverage is required at this stage (T003/T005 cover the downstream integrations).
8. No heuristic is being removed, weakened, or bypassed. The envelope is additive read-only infrastructure.
13. No adjacent contradictions surfaced during reassessment. The spec's original migration framing (deleted during `/reassess-spec`) asserted three accessors existed today; grep workspace-wide confirms they do not. The ticket frames them as net-new.

## Architecture Check

1. The envelope is a read-only projection over existing stored claim data — no new authoritative state, no new information path (P14, P15, P27). Status derivation runs at query time from `effective_claim_confidence` + existing `claim_confidence_threshold`; nothing is cached, nothing is stored (P27).
2. No backward-compatibility shims. The original migration framing that proposed `*_crisp` aliases was deleted during `/reassess-spec` because the methods being "migrated" never existed (P28).
3. The three accessors fill genuine gaps without overlapping `entities_at` / `commodity_quantity` (world reads) or `locally_observed_*` (same-tick perception): the envelope queries remote/stale belief-store content that neither existing accessor surfaces. Non-envelope accessors continue to filter by `claim_confidence_threshold` as today; the envelope accessors intentionally surface below-threshold claims with `status: Stale` so the planner can reason about verification.
4. `BeliefStatus::Contradicted` remains part of the public envelope taxonomy, but claim-level contradiction derivation is explicitly deferred to follow-up ticket `S113BELENV-006`. T001 must not invent a fake contradiction path from unrelated discrepancy memory or world-state checks.

## Verification Layers

1. Types, trait-method signatures, and `GoalBeliefView` forwarding → focused unit tests in `crates/worldwake-sim/src/belief_view.rs` (`#[cfg(test)]` module).
2. Staleness band derivation (`Certain`/`Probable`/`Stale` from `effective_claim_confidence`) → focused unit test against a claim fixture with controlled `acquired_tick` and `current_tick`.
3. Alternatives selection (highest-effective-confidence claim wins `best`, different-value claims preserved as `alternatives`) → focused unit test with two conflicting claims.
4. `BeliefSet` dispute projection (highest-effective-confidence claim wins `best`, different-value alternatives preserved, `best.status == Disputed`) -> focused unit test in `crates/worldwake-sim/src/belief_view.rs`.
5. Runtime and planning-snapshot parity for the three accessor methods -> focused tests in `crates/worldwake-sim/src/per_agent_belief_view.rs` and `crates/worldwake-ai/src/planning_state.rs`.
6. This is a single-layer ticket (belief-view projection + planner-visible carriage). Downstream ranking arithmetic, decision-trace capture, contradiction-driven rejection, and golden E2E remain covered by T003, T002, T006, and T005 respectively.

## What to Change

### 1. Expose `effective_claim_confidence` cross-crate

In `crates/worldwake-core/src/belief.rs` at line 2280, change `fn effective_claim_confidence(...)` to `pub fn effective_claim_confidence(...)`. Update `crates/worldwake-core/src/lib.rs` if a re-export is needed for the symbol to be visible from `worldwake-sim`. Document that this helper performs *single-claim* per-tick decay only.

### 2. Add envelope types to `belief_view.rs`

Add to `crates/worldwake-sim/src/belief_view.rs` (top-of-file, near other pub types):

```rust
/// A single planner-visible belief with provenance metadata. Wraps a
/// crisp value with confidence, freshness, and status.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefValue<T> {
    pub value: T,
    pub confidence: Permille,
    pub acquired_tick: Tick,
    pub claimed_event_tick: Option<Tick>,
    pub status: BeliefStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BeliefStatus {
    Certain,
    Probable,
    Stale,
    Disputed,
    Contradicted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefSet<T> {
    pub best: Option<BeliefValue<T>>,
    pub alternatives: Vec<BeliefValue<T>>,
}

impl<T> BeliefSet<T> {
    pub fn certain(value: T, acquired_tick: Tick) -> Self { /* single-claim factory */ }
    pub fn empty() -> Self { /* ... */ }
}
```

`T` is constrained by whatever `Clone`/`Copy`/derive bounds are already used by the belief-view surface; for `BeliefSet<T>`, `Clone` is sufficient since it holds `Vec<BeliefValue<T>>`. For `BeliefValue<T>`, `Copy` requires `T: Copy` — the three accessors return `Option<EntityId>`, `EntityId`, and `Quantity`, all of which are `Copy`, so the `Copy` derive is valid at usage.

### 3. Add three new belief-envelope accessor methods plus threshold carriage

Each accessor lands on the domain-appropriate sub-trait with a default implementation returning an "empty" envelope or empty list. Forward each through `GoalBeliefView`'s composite surface. Also add a new `claim_confidence_threshold(&self, agent: EntityId) -> Permille` read on `SocialBeliefView`/`GoalBeliefView` and carry it into `PlanningSnapshot` so planner-state accessors can derive the same status bands from snapshot-backed state.

- **`EntityBeliefView::believed_target_location`** — `fn believed_target_location(&self, agent: EntityId, target: EntityId) -> BeliefValue<Option<EntityId>>`. Reads `EntityBeliefAspect::Location` claims for `target` from `agent`'s belief store; returns envelope with `value: Some(place)` / `value: None`. If no claim exists, return `BeliefValue { value: None, confidence: Permille(0), acquired_tick: Tick(0), claimed_event_tick: None, status: BeliefStatus::Stale }`.
- **`SpatialBeliefView::believed_entities_at`** — `fn believed_entities_at(&self, agent: EntityId, place: EntityId, kind: EntityKind) -> Vec<BeliefValue<EntityId>>`. Iterates agent's claims matching `(place, kind)`, emits one `BeliefValue<EntityId>` per claimed entity with the entity's own confidence/freshness/status.
- **`InventoryBeliefView::believed_commodity_stock`** — `fn believed_commodity_stock(&self, agent: EntityId, place: EntityId, kind: CommodityKind) -> BeliefValue<Quantity>`. Reads `EntityBeliefAspect::Inventory(CommodityKind)` claims for `place`; returns envelope with `value: Quantity`. No claim → `BeliefValue { value: Quantity::zero(), ..stale_defaults }`.

Add matching blanket forwarding through `GoalBeliefView` (or its composite) so callers that hold `&dyn GoalBeliefView` can invoke these directly.

### 4. Staleness derivation and alternatives population

Implement a module-private helper (in `belief_view.rs`):

```rust
fn project_claim_into_belief_value<T>(
    claim: &EntityBeliefClaim,
    value: T,
    current_tick: Tick,
    threshold: Permille,
    policy: &BeliefConfidencePolicy,
) -> BeliefValue<T> {
    let effective = effective_claim_confidence(claim, current_tick, policy);
    let threshold = threshold.value();
    let certain_floor = threshold.saturating_mul(2).min(1000);
    let status = if effective >= certain_floor {
        BeliefStatus::Certain
    } else if effective >= threshold {
        BeliefStatus::Probable
    } else {
        BeliefStatus::Stale
    };
    BeliefValue {
        value,
        confidence: Permille::new(effective).unwrap_or(Permille::new(0).unwrap()),
        acquired_tick: claim.acquired_tick,
        claimed_event_tick: claim.claimed_event_tick,
        status,
    }
}
```

T001 does **not** derive `Contradicted`; no explicit contradiction flag exists on the live claim store. Keep `BeliefStatus::Contradicted` as staged enum surface and defer its derivation to `S113BELENV-006`.

For internally projected `BeliefSet` values, iterate relevant claims, compute per-claim effective confidence, select highest-effective as `best`, preserve claims with **different `value`** in `alternatives`. Same-value claims from different sources are **not** aggregated — `best.confidence` is the single-claim effective confidence of the winning claim. When `alternatives` is non-empty, set `best.status = Disputed`.

### 5. Unit tests

Add to the existing `#[cfg(test)]` block in `belief_view.rs` (or open one if absent at the expected location):

1. Fresh direct observation (age 0, confidence 950) with threshold 50 → `status: Certain`, `confidence == 950`.
2. Testimony about past event: `acquired_tick = 100`, `claimed_event_tick = Some(50)` → envelope carries both.
3. Decayed claim: effective confidence in `(threshold, 2*threshold)` band → `status: Probable`.
4. Heavily decayed claim: effective < threshold → `status: Stale`, and the accessor still returns the claim (below-threshold surfacing).
5. Two sources with different `value` projected through `BeliefSet` → `best` is highest-effective, `alternatives` contains the other, `best.status == Disputed`.
6. Two sources with same `value` projected through `BeliefSet` → `alternatives.is_empty()`; `best.confidence` equals the higher of the two single-claim effective confidences (no aggregation).
7. `believed_entities_at` with three claimed entities at one place returns three `BeliefValue<EntityId>` with per-entity freshness/status derived from the winning location claim for each subject.
8. Planning-state parity: the same raw actor belief-store claims produce the same envelope accessors through `PerAgentBeliefView` and `PlanningState`.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — promote `effective_claim_confidence` to `pub`)
- `crates/worldwake-core/src/lib.rs` (modify — re-export if needed)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add `BeliefValue`, `BeliefSet`, `BeliefStatus` types, threshold read, three new trait methods with default implementations, `GoalBeliefView` forwarding, derivation helpers, unit tests)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — real runtime threshold read + runtime accessor tests)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — carry actor claim-confidence threshold into snapshot state)
- `crates/worldwake-ai/src/planning_state.rs` (modify — planner-state threshold read + snapshot parity tests)

## Out of Scope

- `BeliefSnapshot` on decision-event payloads (T002)
- Consumer integration in ranking/revalidation/probe (T003)
- Candidate-generation integration (T004)
- Golden-test extension (T005)
- Multi-source confidence aggregation (explicitly deferred by D5 — future spec if decision-quality evidence warrants)
- Claim-level refutation carriage and live `BeliefStatus::Contradicted` derivation (`S113BELENV-006`)
- Source-chain projection into the envelope (Non-Goal; `PerceptionSource` stays on the underlying claim)
- Changes to `PerceptionProfile` fields (Q2b removed the originally-proposed `freshness_window_ticks`)

## Acceptance Criteria

### Tests That Must Pass

1. The focused helper/runtime/snapshot tests added for §5 pass.
2. Existing `belief_view.rs`, `per_agent_belief_view.rs`, and `planning_state.rs` focused tests pass unchanged.
3. Existing `worldwake-core` and `worldwake-sim` suites: `cargo test -p worldwake-core` and `cargo test -p worldwake-sim`.

### Invariants

1. The envelope accessors are read-only over `AgentBeliefStore` and do not mutate any belief state (P27).
2. `BeliefStatus` is never stored as authoritative state — it is always derived at query time (P3, P27).
3. Non-envelope belief-view accessors (`entities_at`, `commodity_quantity`, etc.) retain their existing filtering and semantics unchanged (P28 — no alternate authority paths introduced).
4. `effective_claim_confidence`'s single-claim decay semantics are unchanged; promotion to `pub` does not alter its behavior.
5. `BeliefStatus::Contradicted` is not emitted by T001; the variant exists as staged API surface only until `S113BELENV-006` lands explicit claim-level refutation carriage.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` — focused helper tests for band derivation and `BeliefSet` dispute projection.
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]` — focused runtime tests for the three new accessors on live belief stores.
3. `crates/worldwake-ai/src/planning_state.rs` `#[cfg(test)]` — snapshot-parity tests proving the same accessors remain available through planning state.

### Commands

1. `cargo test -p worldwake-sim belief_view` (targeted helper coverage).
2. `cargo test -p worldwake-core` (ensures `effective_claim_confidence` promotion did not break core tests).
3. `cargo test -p worldwake-sim` (runtime/accessor coverage).
4. `cargo test -p worldwake-ai planning_state` (snapshot-parity coverage).
5. `cargo clippy -p worldwake-sim --all-targets -- -D warnings`.
4. `./scripts/verify.sh` before opening a PR.

## Outcome

Completed: 2026-04-21

- Added `BeliefValue<T>`, `BeliefSet<T>`, and `BeliefStatus` to `crates/worldwake-sim/src/belief_view.rs`, plus shared projection helpers for claim-to-envelope derivation.
- Promoted `effective_claim_confidence` in `crates/worldwake-core/src/belief.rs` and re-exported it from `crates/worldwake-core/src/lib.rs` so envelope derivation can reuse the live single-claim decay rule across crates.
- Added new belief-envelope reads on the belief-view surface and real runtime/planner implementations:
  - `claim_confidence_threshold`
  - `believed_target_location`
  - `believed_entities_at`
  - `believed_commodity_stock`
- Carried `actor_claim_confidence_threshold` through `crates/worldwake-ai/src/planning_snapshot.rs` so planner snapshots preserve the same band derivation as runtime belief views.
- Landed focused helper/runtime/snapshot tests covering status-band derivation, dispute projection, runtime accessor behavior, and planning-state parity.
- Kept `BeliefStatus::Contradicted` in the public taxonomy but deferred live derivation to `S113BELENV-006`; the live branch still lacks explicit claim-level refutation carriage.
- Updated `specs/S113-belief-envelope.md` plus active sibling tickets `S113BELENV-003` and `S113BELENV-004` to record that `Contradicted` remains staged API surface until `S113BELENV-006` lands.

Deviations from original plan:

- The live branch lacked any explicit claim-level refutation carrier on `AgentBeliefStore` / `EntityBeliefClaim`, so this ticket did not implement live `BeliefStatus::Contradicted` derivation.
- To keep the ticket honest, the implementation landed the non-contradicted envelope foundation and created follow-up ticket `S113BELENV-006` for claim-level refutation carriage.

## Verification Result

Passed:

1. `cargo fmt --all`
2. `cargo test -p worldwake-core`
3. `cargo test -p worldwake-sim`
4. `cargo test -p worldwake-ai planning_state::tests::`
5. Focused checks during implementation:
   - `cargo test -p worldwake-sim belief_view::tests::`
   - `cargo test -p worldwake-sim per_agent_belief_view::tests::believed_`
   - `cargo test -p worldwake-ai planning_state_projects_actor_belief_store_location_claims`
