# S113: Planner-Facing Belief Envelope

## Summary

Introduce `BeliefValue<T>` and `BeliefSet<T>` wrappers on the highest-impact planner-facing belief queries so the planner sees *beliefs*, not collapsed values. Today `RuntimeBeliefView` returns crisp `Option<EntityId>`, `Vec<EntityId>`, and `Quantity` for queries like believed location, believed stock, and believed target presence — losing confidence, freshness, status (Certain / Probable / Stale / Disputed / Contradicted), and alternatives. Agents cannot currently choose "act now vs. verify first" because the planner sees one answer. This spec scopes down to three query domains (target presence, believed location, believed stock) where the acute decision-quality loss is most visible in existing observer traces. Route / ownership / office-holder / institutional-fact envelope exposure is deferred.

## Phase and Status

Phase 8: Belief-First Continual Planning Foundation. Status: Draft.

## Crates

- `worldwake-sim` — `BeliefValue<T>`, `BeliefSet<T>` types in `belief_view.rs`; new accessor methods on the relevant belief-view sub-traits
- `worldwake-ai` — consumers of the new envelope at candidate-generation and ranking call sites; feasibility probe (S112) upgrades to read the envelope
- `worldwake-core` — belief-store scaffolding for the envelope (confidence/freshness already stored; this spec exposes it)

## Dependencies

- S109 (Typed Discrepancy Taxonomy) — `BeliefSet::contradicted` aligns with `Discrepancy::BeliefContradicted`. Soft.
- S112 (Portfolio Planning) — the information-gathering slot consumes envelope confidence to decide whether to activate. Soft.

## Design Goals

- Planner reads confidence, freshness, and status directly. No more "I see `Some(entity)` but I cannot tell whether the agent is sure."
- `BeliefSet<T>` surfaces alternatives where beliefs genuinely disagree (disputed observations, conflicting reports). Contradictions become first-class, not silently collapsed.
- Scoped rollout. Only three query domains in this spec; extensions come with concrete consumers.
- No change to the underlying belief-store storage. Confidence and freshness are already there (Permille confidence, `observed_at` tick). This spec wraps existing data, not new data.

## Non-Goals

- Provenance-chain plumbing. `BeliefValue` records `observed_at` and `claimed_event_time` but not full source-chain IDs. Source chains are a separate spec when investigative scenarios (S63 warrants, wrongful-accusation) demand it.
- All query sites. Route / ownership / office-holder / institutional-fact surfaces stay unchanged until a concrete consumer proves the need.
- Changing the belief-storage schema. Storage already supports confidence + freshness + source.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-14 (World State Is Not Belief State) | The envelope makes belief-vs-world separation visible at every query site. A planner cannot accidentally treat belief as ground truth because the type signature carries `confidence` and `status`. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | `BeliefValue::observed_at` surfaces when the belief was formed; `claimed_event_time` surfaces when the claimed event occurred. Stale beliefs are visible without special-case code. |
| FND-16 (Ignorance, Uncertainty, Contradiction First-Class) | `BeliefSet::alternatives` keeps disputed reports alive; `BeliefStatus::Contradicted` records when two claims disagree. The crisp-value collapse is the bug this spec fixes. |
| FND-20 (Resource-Bounded Practical Reasoning) | The decision rule "verify first if cost is high and confidence is low" becomes expressible. S112's information slot reads confidence directly. |

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
    pub observed_at: Tick,
    /// When the believed event/state is claimed to have occurred.
    /// May differ from observed_at when the belief was acquired via
    /// testimony about a past event.
    pub claimed_event_time: Option<Tick>,
    pub status: BeliefStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BeliefStatus {
    /// Fresh direct observation or high-confidence testimony.
    Certain,
    /// Inferred or second-hand; still actionable.
    Probable,
    /// Age exceeds the relevant freshness window (per-agent field).
    Stale,
    /// Multiple claims exist and the agent has not resolved the disagreement.
    Disputed,
    /// A later observation refuted this belief; kept for history.
    Contradicted,
}

/// A belief set surfaces the best current belief plus unresolved
/// alternatives. When no disagreement exists, `alternatives` is empty.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefSet<T> {
    pub best: Option<BeliefValue<T>>,
    pub alternatives: Vec<BeliefValue<T>>,
}

impl<T> BeliefSet<T> {
    pub fn certain(value: T, observed_at: Tick) -> Self { /* ... */ }
    pub fn empty() -> Self { /* ... */ }
}
```

### D2: Query-site migration (scoped)

Three planner-facing accessors migrate to the envelope. The underlying belief-store data (already carrying Permille confidence and `observed_at`) is projected into the envelope.

**Target presence** — currently:
```rust
fn believed_target_location(&self, agent: EntityId, target: EntityId) -> Option<EntityId>;
```
After:
```rust
fn believed_target_location(&self, agent: EntityId, target: EntityId)
    -> BeliefValue<Option<EntityId>>;
```

**Believed location of self / others for a category** — currently:
```rust
fn believed_entities_at(&self, agent: EntityId, place: EntityId, kind: EntityKind)
    -> Vec<EntityId>;
```
After (wraps each entity with its per-claim confidence/freshness and surfaces alternatives when multiple claims disagree about which entity is at which place):
```rust
fn believed_entities_at(&self, agent: EntityId, place: EntityId, kind: EntityKind)
    -> BeliefSet<Vec<EntityId>>;
```

**Believed commodity stock at a place** — currently:
```rust
fn believed_commodity_stock(&self, agent: EntityId, place: EntityId, kind: CommodityKind)
    -> Quantity;
```
After:
```rust
fn believed_commodity_stock(&self, agent: EntityId, place: EntityId, kind: CommodityKind)
    -> BeliefValue<Quantity>;
```

The old crisp methods remain temporarily as `*_crisp` shims only at the bridge layer while consumers migrate — **not** as live authority paths (FND-28). Once all in-repo consumers migrate, the shims are deleted in the same spec's wrap-up ticket.

### D3: Staleness policy

`BeliefStatus::Stale` is derived at query time from `observed_at`, the current tick, and a per-agent `PerceptionProfile::freshness_window_ticks` (new field, default 240). A belief older than the window returns `BeliefStatus::Stale` regardless of confidence. Shorter windows make agents more suspicious; longer windows make them more trusting.

`BeliefStatus::Disputed` is derived when `BeliefSet::alternatives` is non-empty. `BeliefStatus::Contradicted` is explicitly set when a new observation refutes a prior claim (the refutation path already exists in belief-store update; this spec wires the status bit through).

### D4: Consumer migration

- `candidate_generation.rs` — emitters that currently check `believed_target_location.is_some()` migrate to checking `.status` and `.confidence`. Target-gone emitters stay active when `status == Stale` (may be outdated but still plausible) and skip when `status == Contradicted`.
- `ranking.rs` — motive scorers that discount for stale beliefs currently lack the signal; with the envelope they multiply motive by `confidence.value() / 1000` (deterministic, Permille-based).
- S112 feasibility probe — reads `BeliefValue::status` and `BeliefValue::confidence`. `Stale` + low confidence → probe returns `Plausible` only if the goal is not identity-bound (S108 `BindingStrictness::ExactIdentity` + stale = probe rejects with `Discrepancy::BeliefStale`).
- `plan_revalidation.rs` — revalidation of identity-bound steps (S108) checks `status != Contradicted` on the target-presence envelope.

### D5: Alternatives population

The belief-store already keeps multiple claim entries per (kind, target) when they come from different sources. Exposing them as `alternatives` requires:

1. The query accessor iterates the agent's claim set for the query, builds a `BeliefValue` per claim, and returns the highest-confidence fresh claim as `best` with the rest as `alternatives`.
2. An alternative is preserved in `alternatives` only when its `value` differs from `best.value` (otherwise it is aggregated into `best.confidence` as the same belief from multiple sources — this aggregation policy matches the existing `effective_claim_confidence` helper in `belief.rs`).

### D6: Decision-trace enrichment

S110's `BlockerRecorded` and `PlanInvalidated` payloads that reference a belief include the envelope summary: `(confidence, status)` at the time of the decision. This makes "why did the agent act on stale belief X?" answerable from the event log alone.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: The envelope is a transformation on the agent's existing belief store. No new information enters. `observed_at` and `claimed_event_time` are already recorded; this spec exposes them to the planner.
2. **Positive-feedback analysis**: None. The envelope is a read-side projection.
3. **Concrete dampeners**: `freshness_window_ticks` (per-agent) controls when observations become `Stale`. Not a clamp — a physical process (memory freshness decay in the agent's cognition).
4. **Stored state vs. derived read-model**: The belief store remains authoritative. `BeliefValue` and `BeliefSet` are derived read-views. `BeliefStatus` is derived at query time from stored `observed_at` + per-agent freshness window + existing contradiction flags.

## SystemFn Integration

No new SystemFn. Envelope projection runs inside the belief-view accessors.

## Component Registration

No new components. `PerceptionProfile` gains `freshness_window_ticks: u32` (serde-default).

## Cross-System Interactions

- **Belief-store ↔ belief-view**: The view projects stored claims into the envelope. State-mediated.
- **Planner consumers ↔ envelope**: Candidate generation, ranking, revalidation, and S112's feasibility probe read the envelope and make typed decisions.
- **S110 event log ↔ envelope snapshots**: Belief-referencing decision events carry a snapshot of `(confidence, status)`.

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `freshness_window_ticks` | `PerceptionProfile` | `u32` | 240 | Ticks past which a belief is tagged `Stale` |

## Validation and Falsification

### Unit tests

1. Fresh direct observation → `BeliefValue { status: Certain, confidence: high }`.
2. Testimony acquired at tick T about an event at tick T-100 → `claimed_event_time = Some(T-100)`, `observed_at = T`.
3. Two sources, same value → `alternatives.is_empty()`, `best.confidence` reflects aggregation.
4. Two sources, different values → `best` is highest-confidence, `alternatives` contains the other.
5. `current_tick - observed_at > freshness_window_ticks` → `status == Stale`.
6. Explicit refutation event → `status == Contradicted`.

### Integration tests

7. Candidate emitter that previously silently used a stale belief now skips when `status == Contradicted`. Existing `golden_planner_pathology` scenarios pass without regression.
8. Ranking: motive score for an acquisition goal tied to a stale belief is scaled down by `confidence`.
9. Feasibility probe (S112): `BeliefStale` + `ExactIdentity` strictness → probe returns `RejectedBeforeSearch { reason: BeliefStale }`.

### Golden test extension

10. Extend an existing rumor-driven scenario (if present) or the `survival-scattered.ron` based golden to assert the envelope surfaces stale observations after `freshness_window_ticks` without any perception update.

## Outcome

To be filled in at completion.
