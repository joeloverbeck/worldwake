# S113: Planner-Facing Belief Envelope

## Summary

Introduce `BeliefValue<T>` and `BeliefSet<T>` read-model wrappers and expose three new planner-facing belief-store accessors that surface confidence, freshness, status (`Certain` / `Probable` / `Stale` / `Disputed`), with `Contradicted` retained in the end-state taxonomy once claim-level refutation carriage lands. Today the planner has no belief-store accessor for "where do I believe target X is?", "which entities do I believe are at remote place P?", or "how much commodity Q do I believe is at place P?" — existing accessors (`entities_at`, `locally_observed_entities_at`, `commodity_quantity`, `locally_observed_commodity_quantity`) read world state or same-tick perception (FND-14A), and `pursuit_belief.rs::last_known_place` is a single-slot pursuit target, not a general query. Agents therefore cannot plan from remote rumor or stale testimony, and they cannot reason "act now vs. verify first" because the signals the planner sees do not carry confidence. This spec adds three scoped belief-envelope accessors; route / ownership / office-holder / institutional-fact envelope exposure is deferred.

## Phase and Status

Phase 8: Belief-First Continual Planning Foundation. Status: Draft.

## Crates

- `worldwake-sim` — `BeliefValue<T>`, `BeliefSet<T>` types in `belief_view.rs`; three new accessor methods on `EntityBeliefView`, `SpatialBeliefView`, and `InventoryBeliefView` with forwarding through `GoalBeliefView`
- `worldwake-ai` — consumers of the new envelope at candidate-generation, ranking, and plan-revalidation call sites; feasibility probe (S112) gains envelope-aware rejection reasons
- `worldwake-core` — `BeliefSnapshot` addition on belief-referencing decision-event payloads (`decision_event_payload.rs`)

Implementation note: foundational ticket `S113BELENV-001` lands the non-contradicted envelope projection plus staged `BeliefStatus::Contradicted` API surface. Claim-level refutation carriage needed to derive `Contradicted` honestly is deferred to `S113BELENV-006`.

## Dependencies

- S109 (Typed Discrepancy Taxonomy, archived) — reuses `Discrepancy::BeliefStale` and `Discrepancy::BeliefContradicted` for feasibility-probe and revalidation rejection reasons. Soft.
- S112 (Portfolio Planning, archived) — S112's `FeasibilityVerdict::RejectedBeforeSearch { reason }` gains envelope-driven triggers; the information-gathering slot consumes envelope confidence to decide whether to activate. Soft.
- S108 (Per-Action Binding Strictness, archived) — revalidation of identity-bound steps gains a `BeliefStatus::Contradicted` short-circuit. Soft.
- S110 (Decision History Events, archived) — `BlockerRecordedPayload` and `PlanInvalidatedPayload` (in `crates/worldwake-core/src/decision_event_payload.rs`) gain an optional `belief_snapshot` field so belief-driven invalidations/blockers can carry frozen envelope metadata. Soft.

## Design Goals

- Planner reads confidence, freshness, and status directly. No more "I see `Some(entity)` but I cannot tell whether the agent is sure."
- Planner gains belief-store queries for remote targets, remote entities-at-place, and remote commodity stock — queries the current belief-view surface cannot answer.
- `BeliefSet<T>` and `Vec<BeliefValue<T>>` return shapes surface alternatives where beliefs genuinely disagree (disputed observations, conflicting reports). Contradictions become first-class, not silently collapsed.
- Scoped rollout. Only three query domains in this spec; extensions come with concrete consumers.
- No change to the underlying belief-store storage. Confidence and freshness are already there (`EntityBeliefClaim.confidence: Permille`, `EntityBeliefClaim.acquired_tick: Tick`, `EntityBeliefClaim.claimed_event_tick: Option<Tick>`, `EntityBeliefClaim.source: PerceptionSource`). This spec wraps and exposes existing data, not new data.

## Non-Goals

- Full source-chain projection. `BeliefValue` records `acquired_tick` and `claimed_event_tick`. `PerceptionSource` (with `Report { from, chain_len }` and `Rumor { chain_len }`) is *already stored* on every `EntityBeliefClaim`; this spec does not project the full source chain into the envelope. Exposing source-chain identifiers belongs to a later spec when investigative scenarios (S63 warrants, wrongful-accusation) demand them.
- All query sites. Route / ownership / office-holder / institutional-fact surfaces stay unchanged until a concrete consumer proves the need.
- Changing the belief-storage schema. Storage already supports confidence, freshness, source, and multi-claim disagreement.
- New per-agent profile parameters. Staleness derivation reuses the existing `PerceptionProfile::claim_confidence_threshold` and `PerceptionProfile::confidence_policy.staleness_penalty_per_tick`.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-14 (World State Is Not Belief State) | The envelope makes belief-vs-world separation visible at every new query site. A planner cannot accidentally treat belief as ground truth because the type signature carries `confidence` and `status`. The three new accessors are belief-store reads; they sit alongside (not replacing) the existing world-state and same-tick perception reads, which remain correct under FND-14A for co-located observation. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | `BeliefValue::acquired_tick` surfaces when the belief was formed; `claimed_event_tick` surfaces when the claimed event occurred. Stale beliefs are visible without special-case code. `PerceptionSource` remains available on the underlying claim for source-chain reasoning. |
| FND-16 (Ignorance, Uncertainty, Contradiction First-Class) | `BeliefSet::alternatives` and `Vec<BeliefValue<T>>` keep disputed reports alive; `BeliefStatus::Contradicted` records when a later observation refutes a prior claim. The crisp-value collapse is the bug this spec fixes. |
| FND-20 (Resource-Bounded Practical Reasoning) | The decision rule "verify first if cost is high and confidence is low" becomes expressible. S112's information slot reads confidence directly. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `BeliefStatus` is derived at query time from existing stored fields (`acquired_tick`, `claimed_event_tick`, `confidence`, contradiction flags). No derived value is promoted to authoritative state. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The three new accessors are introduced cleanly alongside existing world/perception accessors. No deprecated shims or dual authority paths. |

## Deliverables

### D1: `BeliefValue<T>` and `BeliefSet<T>` types

New types in `crates/worldwake-sim/src/belief_view.rs`:

```rust
/// A single planner-visible belief with provenance metadata. Wraps a
/// crisp value with confidence, freshness, and status.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefValue<T> {
    pub value: T,
    pub confidence: Permille,
    /// Tick at which the agent acquired this belief. Matches the
    /// underlying `EntityBeliefClaim.acquired_tick`.
    pub acquired_tick: Tick,
    /// When the believed event/state is claimed to have occurred.
    /// May differ from `acquired_tick` when the belief was acquired
    /// via testimony about a past event.
    pub claimed_event_tick: Option<Tick>,
    pub status: BeliefStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BeliefStatus {
    /// Effective confidence (after per-tick staleness decay) is well
    /// above `claim_confidence_threshold`. Fresh observation or
    /// high-confidence testimony.
    Certain,
    /// Effective confidence is above threshold but not by a wide
    /// margin. Still actionable, worth considering verification.
    Probable,
    /// Effective confidence has decayed to or below
    /// `claim_confidence_threshold`. The belief is too eroded to act
    /// on without verification; the planner may still read it to
    /// decide between acting and first gathering evidence.
    Stale,
    /// Multiple claims exist and the agent has not resolved the
    /// disagreement. Derived when `BeliefSet::alternatives` is
    /// non-empty for the `best` entry.
    Disputed,
    /// A later observation refuted this belief; kept for history.
    /// Set explicitly when a refutation event is recorded on the
    /// belief store.
    Contradicted,
}

/// A belief set surfaces the best current belief plus unresolved
/// alternatives. Used for queries with a single conceptual slot
/// (e.g., "where is target X?") that may have conflicting claims.
/// For queries returning a set of entities with independent
/// confidences (e.g., "who is at place P?"), use
/// `Vec<BeliefValue<EntityId>>` instead.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefSet<T> {
    pub best: Option<BeliefValue<T>>,
    pub alternatives: Vec<BeliefValue<T>>,
}

impl<T> BeliefSet<T> {
    pub fn certain(value: T, acquired_tick: Tick) -> Self { /* ... */ }
    pub fn empty() -> Self { /* ... */ }
}
```

### D2: Three new belief-envelope accessors

All three are **new accessors**, not migrations of existing methods. Grep confirms no method named `believed_target_location`, `believed_entities_at`, or `believed_commodity_stock` currently exists in the belief-view surface. Each accessor lands on the domain-appropriate sub-trait and is forwarded through `GoalBeliefView` via the existing trait hierarchy.

**Target presence** — new method on `EntityBeliefView` (`belief_view.rs:464`):
```rust
fn believed_target_location(&self, agent: EntityId, target: EntityId)
    -> BeliefValue<Option<EntityId>>;
```
Reads entity-location claims (`EntityBeliefAspect::Location` on `EntityBeliefClaim`) from the agent's belief store. Returns the highest-confidence fresh claim as `value: Some(place)` or `value: None` if no claim exists; `status: Contradicted` when a refutation is recorded. Complements (does not replace) the existing pursuit-target pipeline in `pursuit_belief.rs`, which remains authoritative for single-target pursuit state.

**Believed entities at a remote place** — new method on `SpatialBeliefView` (`belief_view.rs:544`):
```rust
fn believed_entities_at(&self, agent: EntityId, place: EntityId, kind: EntityKind)
    -> Vec<BeliefValue<EntityId>>;
```
Per-entity confidences inside the set. Each believed-to-be-present entity carries its own `BeliefValue` (status, confidence, acquired_tick). This models "I heard Bob is at the tavern from source A, but source B didn't mention him" — Bob gets a `Probable` status while co-mentioned entities get `Certain`. Complements the existing `entities_at` (world-state, FND-14A) and `locally_observed_entities_at` (same-tick co-located perception) accessors on the same trait. Those remain for co-located queries; the new method is for remote/belief-based queries.

**Believed commodity stock at a place** — new method on `InventoryBeliefView` (`belief_view.rs:614`):
```rust
fn believed_commodity_stock(&self, agent: EntityId, place: EntityId, kind: CommodityKind)
    -> BeliefValue<Quantity>;
```
Reads `EntityBeliefAspect::Inventory(CommodityKind)` claims for the place. Complements `commodity_quantity` (world-state) and `locally_observed_commodity_quantity` (same-tick perception) on the same trait. Value `Quantity::zero()` + `status: Stale` is distinct from value `Quantity::zero()` + `status: Certain` (the former means "I haven't heard of any recently," the latter means "I have solid evidence the place is empty").

### D3: Staleness derivation (no new profile parameter)

`BeliefStatus` is derived at query time from stored fields plus existing per-agent parameters:

1. Compute `effective = effective_claim_confidence(claim, current_tick, &profile.confidence_policy)` — the existing helper at `crates/worldwake-core/src/belief.rs:2280` applies per-tick staleness decay (`staleness_penalty_per_tick` on `BeliefConfidencePolicy`) to the stored confidence.
2. `Contradicted` wins first once explicit claim-level refutation carriage exists. Foundational ticket `S113BELENV-001` does not invent this flag; follow-up ticket `S113BELENV-006` owns that derivation substrate.
3. If multiple claims disagree in `best` vs. `alternatives`, status is `Disputed` (and the decision rule for which becomes `best` is described in D5).
4. Otherwise, bands derived from the agent's `PerceptionProfile::claim_confidence_threshold`:
   - `effective >= claim_confidence_threshold * 2` → `Certain` (well above threshold — cap the multiplier at 1000 permille).
   - `effective >= claim_confidence_threshold` → `Probable` (above retention threshold).
   - `effective < claim_confidence_threshold` → `Stale` (effective confidence has decayed below the retention threshold, but the envelope accessor intentionally surfaces these below-threshold claims to let the planner reason about verification).

No new `PerceptionProfile` field is introduced. The existing `claim_confidence_threshold` and `confidence_policy.staleness_penalty_per_tick` fully parameterize the decay and band boundaries per agent. Shorter agent-local `staleness_penalty_per_tick` makes the agent slower to mark beliefs stale; higher `claim_confidence_threshold` makes the agent more suspicious of eroded claims.

**Note on envelope vs. non-envelope reads**: Non-envelope accessors (e.g., `entities_at`, or any future non-envelope belief read) continue to filter by `claim_confidence_threshold` as today. The envelope accessors surface below-threshold claims with `status: Stale` so the planner can reason "I once believed X, but the belief has decayed — verify or act?" This is the new capability.

### D4: Consumer integration (net-new, not migration)

None of the three accessors have current consumers. All integration sites are new:

- `candidate_generation.rs` — existing emitters that need belief-based target-presence or remote-stock signals gain new envelope reads. Emitters use `status == Contradicted` to skip (the belief is refuted), `status == Stale` to still emit (the belief is eroded but plausible — the agent may want to plan a verification step), and `status == Certain`/`Probable` for normal emission. The specific emitters that gain new reads are identified per-GoalKind during ticket decomposition; the envelope infrastructure is introduced first.
- `ranking.rs` — `motive_score` gains envelope-confidence-aware scaling at the strongest honest live seam: `RaidTarget` motive via target-location confidence. The scaling formula is
  ```rust
  let scaled = (motive as u64)
      .saturating_mul(confidence.value() as u64)
      / 1000;
  u32::try_from(scaled).unwrap_or(u32::MAX)
  ```
  multiplying before dividing to preserve precision. `Permille::value()` returns `u16` in [0, 1000]; the `u64` lift avoids overflow on large motive values. Deterministic integer arithmetic throughout.
- `plan_revalidation.rs` — `revalidate_exact_target_step` (around `plan_revalidation.rs:101-117`) gains a new predicate: when the step is identity-bound (S108 `BindingStrictness::ExactIdentity`), read the target-presence envelope via `believed_target_location`. If `status == Contradicted`, return `false` from the boolean revalidation seam. The discrepancy classification (`Discrepancy::BeliefContradicted` / `BeliefStale`) remains downstream in failure handling. This is a new predicate insertion, not a modification of existing logic.
- S112 feasibility probe (`crates/worldwake-ai/src/feasibility_probe.rs`) — the probe gains envelope-aware rejection: when the target step is identity-bound and the envelope returns `status == Stale`, the probe returns `FeasibilityVerdict::RejectedBeforeSearch { reason: Discrepancy::BeliefStale }`, letting the information-gathering slot activate. `FeasibilityVerdict` at `crates/worldwake-ai/src/agent_tick/portfolio.rs:29-31` already carries the `Discrepancy` reason; no new variants are needed.

Under the **Authoritative-to-AI Impact Rule** (CLAUDE.md): emitter changes in D4 modify candidate emission. Ticket decomposition must check that `get_affordances`, `generate_candidates`, `search_plan`, `BestEffort` action start, `handle_plan_failure`, and payload revalidation all remain correct; new goldens or extensions of `golden_planner_pathology` exercise the `Stale`/`Contradicted` paths.

### D5: Alternatives population and aggregation

The belief store (`AgentBeliefStore::entity_claims: BTreeMap<EntityId, Vec<EntityBeliefClaim>>`) already keeps multiple claim entries per `(subject, aspect, source_kind)` key. Exposing them through the envelope:

1. The envelope accessor iterates the agent's relevant claim set (filtered by the query's entity/place/commodity), runs `effective_claim_confidence` on each, and selects the claim with the highest *effective* confidence as `best`.
2. A claim is preserved in `alternatives` only when its `value` differs from `best.value`. Same-value claims do **not** get aggregated into a composite confidence boost — `best.confidence` is the single-claim effective confidence of the winning claim. (A later spec may introduce a multi-source aggregation rule if decision-quality evidence warrants; the existing `effective_claim_confidence` helper is single-claim staleness decay and does not perform aggregation.)
3. When `alternatives` is non-empty, `best.status` is set to `Disputed` instead of `Certain`/`Probable`/`Stale`.
4. For `believed_entities_at`, the return type `Vec<BeliefValue<EntityId>>` carries per-entity confidences directly; alternatives within a single entity's claim history follow the same rule (different-value claims preserved).

### D6: Decision-trace enrichment via `BeliefSnapshot`

Add a new type `BeliefSnapshot` in `crates/worldwake-core/src/decision_event_payload.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefSnapshot {
    pub confidence: Permille,
    pub status: BeliefStatusTag,
    pub acquired_tick: Tick,
}

/// Serializable projection of `BeliefStatus` — lives in core because
/// `BeliefStatus` itself lives in sim. Kept as a transparent tag;
/// variants mirror `BeliefStatus` exactly.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BeliefStatusTag {
    Certain,
    Probable,
    Stale,
    Disputed,
    Contradicted,
}
```

Extend the two belief-referencing payloads:

- `BlockerRecordedPayload` (at `decision_event_payload.rs:250-256`) gains an optional `belief_snapshot: Option<BeliefSnapshot>`. `S113BELENV-002` owns the schema addition plus save-format bump; `S113BELENV-003` now populates this field on the target-belief `BeliefStale` / `BeliefContradicted` blocker/discrepancy branches it wired. Other runtime emitters still lawfully write `None` until their producer sites are updated.
- `PlanInvalidatedPayload` (at `decision_event_payload.rs:144-178`) gains an optional `belief_snapshot: Option<BeliefSnapshot>`. `S113BELENV-002` owns the schema addition plus save-format bump; live population for belief-driven invalidation variants lands later once the producer sites are wired. Until then, runtime emitters lawfully write `None`.

Because Worldwake's save format is positionally serialized with `bincode`, adding either field requires a save-format bump rather than relying on `#[serde(default)]` for old-save compatibility. The `#[serde(default)]` remains useful for any intra-head decode path that omits the field, but it is not a cross-version migration mechanism.

This makes "why did the agent act on stale belief X?" and "why did the plan invalidate?" answerable from the event log alone.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: The envelope is a transformation on the agent's existing belief store. No new information enters. `acquired_tick`, `claimed_event_tick`, `confidence`, and contradiction flags are already recorded on `EntityBeliefClaim`; this spec exposes them to the planner through new accessors. For the new `BeliefSnapshot` field on decision-event payloads, the information path is: authoritative belief store → envelope accessor at the blocker/invalidation site → snapshot copied into payload → append-only event log. No cross-agent information transfer.
2. **Positive-feedback analysis**: None. The envelope is a read-side projection; it neither writes belief-store state nor creates new world state.
3. **Concrete dampeners**: Staleness decay is governed by the existing physical process — `BeliefConfidencePolicy::staleness_penalty_per_tick` applies per-tick decay to every stored claim via `effective_claim_confidence`. Per-agent `claim_confidence_threshold` determines when effective confidence crosses into `Stale`. No new dampener introduced.
4. **Stored state vs. derived read-model**: The belief store (`AgentBeliefStore.entity_claims`) remains authoritative. `BeliefValue`, `BeliefSet`, and `BeliefStatus` are derived read-views. `BeliefStatusTag` is a serializable tag on decision-event payloads — a historical snapshot, not authoritative live state. `BeliefSnapshot` values in the append-only event log are frozen at the time of the blocker/invalidation event and are never re-derived.

## SystemFn Integration

No new SystemFn. Envelope projection runs inside the belief-view accessors during planning; no tick-level system operation is added.

## Component Registration

No new components and no field additions to existing components. `PerceptionProfile`, `BeliefConfidencePolicy`, and all other agent-behavior-affecting components are unchanged. The spec-drafting-rules §5 scenario-contract checklist is therefore N/A for this spec.

## Cross-System Interactions

- **Belief-store ↔ belief-view**: The belief-view accessors read stored `EntityBeliefClaim` entries from `AgentBeliefStore` and project them into the envelope. State-mediated. No direct cross-system calls.
- **Planner consumers ↔ envelope**: Candidate generation, ranking, plan revalidation, and S112's feasibility probe read the envelope through the `GoalBeliefView` trait surface and make typed decisions on `status` and `confidence`. State-mediated reads.
- **Envelope ↔ S110 event log**: Belief-referencing blockers and invalidations capture a `BeliefSnapshot` into the append-only event log. Read-once, frozen-at-event. No retroactive mutation.
- **S108 binding strictness ↔ envelope**: `plan_revalidation.rs`'s identity-bound revalidation consults `BindingStrictness::ExactIdentity` and the target-presence envelope together; the combination produces a `Discrepancy::BeliefContradicted` or `Discrepancy::BeliefStale` when appropriate.
- **S112 feasibility probe ↔ envelope**: The probe reads the envelope before committing to search; stale or contradicted beliefs short-circuit to `RejectedBeforeSearch { reason }` so the information-gathering slot can activate.

## Profile-Driven Parameters

No new profile parameters introduced. The spec reuses existing `PerceptionProfile` fields:

| Reused Parameter | Profile | Type | Purpose in S113 |
|------------------|---------|------|-----------------|
| `claim_confidence_threshold` | `PerceptionProfile` | `Permille` | Boundary between `Probable` and `Stale` in envelope status derivation |
| `confidence_policy.staleness_penalty_per_tick` | `BeliefConfidencePolicy` (nested in `PerceptionProfile`) | `Permille` | Per-tick confidence decay applied by `effective_claim_confidence` |

Agent diversity (FND-22) continues to flow through these existing parameters: agents with higher `claim_confidence_threshold` mark beliefs `Stale` sooner; agents with higher `staleness_penalty_per_tick` decay beliefs faster.

## Validation and Falsification

### Unit tests

1. Fresh direct observation → `BeliefValue { status: Certain, confidence: high }`.
2. Testimony acquired at tick T about an event at tick T-100 → `claimed_event_tick = Some(T-100)`, `acquired_tick = T`.
3. Single-source claim whose effective confidence (after decay) is well above `claim_confidence_threshold` → `status == Certain`.
4. Single-source claim whose effective confidence is above but within 2× `claim_confidence_threshold` → `status == Probable`.
5. Single-source claim whose effective confidence has decayed below `claim_confidence_threshold` → `status == Stale`, and the envelope accessor surfaces the claim (non-envelope accessors would filter it out).
6. Two sources, different values → `best` is highest-effective-confidence, `alternatives` contains the other, `best.status == Disputed`.
7. Explicit refutation flag set on claim → `status == Contradicted` regardless of other signals.
8. Ranking formula precision test: `motive_score = 500`, `confidence = Permille(500)` → scaled score `= 250` (verifies multiply-before-divide preserves precision). `motive_score = 500`, `confidence = Permille(1000)` → scaled score `= 500` (full motive preserved at max confidence).

### Integration tests

9. Candidate emitter with a new envelope read skips emission when `status == Contradicted` on the target-presence envelope; existing `golden_planner_pathology` scenarios pass without regression.
10. Ranking: motive score for an acquisition goal tied to a `Stale` belief is scaled down proportional to effective confidence.
11. Feasibility probe (S112): identity-bound target with `status == Stale` → probe returns `FeasibilityVerdict::RejectedBeforeSearch { reason: Discrepancy::BeliefStale }`.
12. Plan revalidation: identity-bound step whose target-presence envelope returns `status == Contradicted` fails revalidation with `Discrepancy::BeliefContradicted`.
13. Decision-trace payload: a `Stale`-driven target-belief blocker emits a `BlockerRecordedPayload` with `belief_snapshot: Some(...)` carrying the captured `confidence` and `BeliefStatusTag::Stale`. `S113BELENV-002` lands the payload schema; `S113BELENV-003` lands the first live producer population on the affected AI branches.

### Golden test extension

14. Extend an existing rumor-driven scenario (if present) or the `survival-scattered.ron`-based golden to assert the envelope surfaces stale observations after sufficient staleness decay (ticks elapsed such that `effective_claim_confidence` drops below the agent's `claim_confidence_threshold`) without any perception update.

## Outcome

To be filled in at completion.
