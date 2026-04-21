# S113BELENV-001: Envelope foundation — `BeliefValue<T>`, `BeliefSet<T>`, and three new planner-facing accessors

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — belief-view trait surface (`worldwake-sim/src/belief_view.rs`), public confidence helper (`worldwake-core/src/belief.rs`)
**Deps**: None

## Problem

The planner today has no belief-store accessor for "where do I believe target X is?", "which entities do I believe are at remote place P?", or "how much commodity Q do I believe is at place P?". Existing accessors (`entities_at`, `locally_observed_entities_at`, `commodity_quantity`, `locally_observed_commodity_quantity` in `crates/worldwake-sim/src/belief_view.rs`) read authoritative world state or same-tick co-located perception (FND-14A); `pursuit_belief.rs::last_known_place` is a single-target pursuit slot, not a general belief query. As a result, agents cannot plan from remote rumor or stale testimony, and the planner cannot distinguish "high-confidence recent observation" from "decayed stale claim" because every belief projection today is crisp.

S113 introduces `BeliefValue<T>`, `BeliefSet<T>`, and `BeliefStatus` as read-model wrappers and adds three new envelope-typed accessors. This ticket lands the types, the three accessors, the staleness derivation, and the alternatives aggregation rule together because none of them are independently usable — the types are inert without accessors, and the accessors cannot be written without the derivation rule.

## Assumption Reassessment (2026-04-21)

1. `EntityBeliefClaim` (`crates/worldwake-core/src/entity_belief_claim.rs:47-56`) already stores `acquired_tick: Tick`, `claimed_event_tick: Option<Tick>`, `confidence: Permille`, `source: PerceptionSource`, and `value: ClaimValue`. The envelope exposes existing data; no new storage is added. `AgentBeliefStore.entity_claims: BTreeMap<EntityId, Vec<EntityBeliefClaim>>` (`crates/worldwake-core/src/belief.rs:45-46`) already supports multi-source disagreement per entity.
2. `effective_claim_confidence` lives at `crates/worldwake-core/src/belief.rs:2280` as a private `fn`. Cross-crate use from `worldwake-sim` requires promoting it to `pub`. Signature: `fn effective_claim_confidence(claim: &EntityBeliefClaim, current_tick: Tick, policy: &BeliefConfidencePolicy) -> u16`. Semantics: single-claim per-tick staleness decay; **no multi-claim aggregation** — the alternatives rule here does not claim this helper performs aggregation.
3. The belief-view hierarchy in `crates/worldwake-sim/src/belief_view.rs` has sub-traits at lines 464 (`EntityBeliefView`), 544 (`SpatialBeliefView`), 614 (`InventoryBeliefView`), plus a composite `GoalBeliefView` that sees the agent's accessible belief state. Shared abstraction boundary under audit: the belief-view trait surface (not world state) and the `EntityBeliefClaim` read model. Methods added on sub-traits must propagate to `GoalBeliefView` so candidate-generation (T004) and ranking/revalidation/probe (T003) can read them through the goal-belief interface they already use.
6. This is an AI-planning-layer ticket — intended layer is belief-view projection, exercised by focused unit tests in the target file. No `agent_tick` or golden coverage is required at this stage (T003/T005 cover the downstream integrations).
8. No heuristic is being removed, weakened, or bypassed. The envelope is additive read-only infrastructure.
13. No adjacent contradictions surfaced during reassessment. The spec's original migration framing (deleted during `/reassess-spec`) asserted three accessors existed today; grep workspace-wide confirms they do not. The ticket frames them as net-new.

## Architecture Check

1. The envelope is a read-only projection over existing stored claim data — no new authoritative state, no new information path (P14, P15, P27). Status derivation runs at query time from `effective_claim_confidence` + existing `claim_confidence_threshold`; nothing is cached, nothing is stored (P27).
2. No backward-compatibility shims. The original migration framing that proposed `*_crisp` aliases was deleted during `/reassess-spec` because the methods being "migrated" never existed (P28).
3. The three accessors fill genuine gaps without overlapping `entities_at` / `commodity_quantity` (world reads) or `locally_observed_*` (same-tick perception): the envelope queries remote/stale belief-store content that neither existing accessor surfaces. Non-envelope accessors continue to filter by `claim_confidence_threshold` as today; the envelope accessors intentionally surface below-threshold claims with `status: Stale` so the planner can reason about verification.

## Verification Layers

1. Types, trait-method signatures, and `GoalBeliefView` forwarding → focused unit tests in `crates/worldwake-sim/src/belief_view.rs` (`#[cfg(test)]` module).
2. Staleness band derivation (`Certain`/`Probable`/`Stale` from `effective_claim_confidence`) → focused unit test against a claim fixture with controlled `acquired_tick` and `current_tick`.
3. Alternatives selection (highest-effective-confidence claim wins `best`, different-value claims preserved as `alternatives`) → focused unit test with two conflicting claims.
4. Contradicted flag passthrough → focused unit test asserting `status == Contradicted` wins over band-derivation.
5. This is a single-layer ticket (belief-view projection). Downstream verification layers — ranking arithmetic, decision-trace capture, golden E2E — are covered by T003, T002, and T005 respectively.

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

### 3. Add three new belief-envelope accessor methods

Each accessor lands on the domain-appropriate sub-trait with a default implementation returning an "empty" envelope. The `RuntimeBeliefView` impl (later in the same file) provides the real belief-store reads. Forward each through `GoalBeliefView`'s composite surface.

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
    profile: &PerceptionProfile,
    forced_status: Option<BeliefStatus>,
) -> BeliefValue<T> {
    let effective = effective_claim_confidence(claim, current_tick, &profile.confidence_policy);
    let threshold = profile.claim_confidence_threshold.value();
    let certain_floor = threshold.saturating_mul(2).min(1000);
    let status = forced_status.unwrap_or_else(|| {
        if claim_is_contradicted(claim) {
            BeliefStatus::Contradicted
        } else if effective >= certain_floor {
            BeliefStatus::Certain
        } else if effective >= threshold {
            BeliefStatus::Probable
        } else {
            BeliefStatus::Stale
        }
    });
    BeliefValue {
        value,
        confidence: Permille::new(effective).unwrap_or(Permille::new(0).unwrap()),
        acquired_tick: claim.acquired_tick,
        claimed_event_tick: claim.claimed_event_tick,
        status,
    }
}
```

The `claim_is_contradicted` predicate taps the existing contradiction-flag mechanism in `AgentBeliefStore` (implementer to locate the current refutation record and wire it through — if no explicit flag exists today, surface that as an in-scope finding and add a minimal refutation marker, or descope `Contradicted` derivation until a later ticket).

For `BeliefSet`-returning accessors, iterate relevant claims, compute per-claim effective confidence, select highest-effective as `best`, preserve claims with **different `value`** in `alternatives`. Same-value claims from different sources are **not** aggregated — `best.confidence` is the single-claim effective confidence of the winning claim. When `alternatives` is non-empty, set `best.status = Disputed`.

### 5. Unit tests

Add to the existing `#[cfg(test)]` block in `belief_view.rs` (or open one if absent at the expected location):

1. Fresh direct observation (age 0, confidence 950) with threshold 50 → `status: Certain`, `confidence == 950`.
2. Testimony about past event: `acquired_tick = 100`, `claimed_event_tick = Some(50)` → envelope carries both.
3. Decayed claim: effective confidence in `(threshold, 2*threshold)` band → `status: Probable`.
4. Heavily decayed claim: effective < threshold → `status: Stale`, and the accessor still returns the claim (below-threshold surfacing).
5. Two sources with different `value` → `best` is highest-effective, `alternatives` contains the other, `best.status == Disputed`.
6. Two sources with same `value` → `alternatives.is_empty()`; `best.confidence` equals the higher of the two single-claim effective confidences (no aggregation).
7. Contradicted flag set → `status == Contradicted` regardless of effective confidence.
8. `believed_entities_at` with three claimed entities and one contradicted → returns three `BeliefValue<EntityId>`, one with `status: Contradicted`.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — promote `effective_claim_confidence` to `pub`)
- `crates/worldwake-core/src/lib.rs` (modify — re-export if needed)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add `BeliefValue`, `BeliefSet`, `BeliefStatus` types, three new trait methods with default and `RuntimeBeliefView` impls, `GoalBeliefView` forwarding, derivation helper, unit tests)

## Out of Scope

- `BeliefSnapshot` on decision-event payloads (T002)
- Consumer integration in ranking/revalidation/probe (T003)
- Candidate-generation integration (T004)
- Golden-test extension (T005)
- Multi-source confidence aggregation (explicitly deferred by D5 — future spec if decision-quality evidence warrants)
- Source-chain projection into the envelope (Non-Goal; `PerceptionSource` stays on the underlying claim)
- Changes to `PerceptionProfile` fields (Q2b removed the originally-proposed `freshness_window_ticks`)

## Acceptance Criteria

### Tests That Must Pass

1. All 8 new unit tests listed in §5 above pass.
2. Existing `belief_view.rs` `#[cfg(test)]` tests pass unchanged.
3. Existing `worldwake-core` and `worldwake-sim` suites: `cargo test -p worldwake-core` and `cargo test -p worldwake-sim`.

### Invariants

1. The envelope accessors are read-only over `AgentBeliefStore` and do not mutate any belief state (P27).
2. `BeliefStatus` is never stored as authoritative state — it is always derived at query time (P3, P27).
3. Non-envelope belief-view accessors (`entities_at`, `commodity_quantity`, etc.) retain their existing filtering and semantics unchanged (P28 — no alternate authority paths introduced).
4. `effective_claim_confidence`'s single-claim decay semantics are unchanged; promotion to `pub` does not alter its behavior.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` — 8 new unit tests per §5 covering band derivation, alternatives, contradiction, and the three accessor methods.
2. No modifications to existing tests expected; if any existing test on a neighboring method breaks due to trait-signature changes (new default methods), fix the test to include the new defaults rather than weakening the method surface.

### Commands

1. `cargo test -p worldwake-sim belief_view` (targeted — runs the new unit tests).
2. `cargo test -p worldwake-core` (ensures `effective_claim_confidence` promotion did not break core tests).
3. `cargo clippy -p worldwake-sim --all-targets -- -D warnings` (catches unused imports on `BeliefSet<T>::empty`, etc.).
4. `./scripts/verify.sh` before opening a PR.
