# S135: Planner Snapshot Perception Budget and Observation Omission

**Status**: Draft

## Summary

`max_snapshot_entities_per_place = 50` (`crates/worldwake-core/src/cognitive_profile.rs:96`, default 50, used at `crates/worldwake-ai/src/agent_tick/planning.rs:546`) silently truncates the planner's per-place snapshot. If a co-located place holds 80 relevant entities, the planner sees only the first 50 in deterministic order. The agent fails to perceive a co-located enemy, corpse, container, workstation, or sale lot — for no in-world reason. This violates FND-7 (locality) and the spirit of FND-14A (same-tick co-located observation is belief-equivalent), because the agent's belief-view shape diverges from what perception would actually deliver.

S105 (Observation Salience Filtering) addressed the *perception* layer with `PerceptionProfile.observation_budget` (`crates/worldwake-core/src/belief.rs:2573`, default 24) and deterministic-priority truncation in `collect_direct_local_observation_batch`. The planner snapshot is a *separate* truncation pass, downstream of perception, that enforces its own cap. Two truncation layers stacked silently is exactly the FND-12 violation the assessor flagged: performance compressing causality.

S135 collapses the planner-snapshot truncation into the same perception budget S105 already governs and surfaces every dropped entity as an inspectable `ObservationOmission` record. After S135, the planner snapshot is a derived view over the agent's already-truncated belief observations rather than a second silent gate. The omission record names the entity, the reason it was dropped (over-budget, occlusion, salience), and the tick — so observer reports and goldens can verify "the agent could not act on X because their attention budget was full," not "the planner cap was 50."

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-core` — extends `PerceptionProfile` (`belief.rs:2554`) with `salience_policy: SaliencePolicy` (typed enum) and `omission_log_capacity: u8`. Adds new `ObservationOmission { omitted_entity: EntityId, reason: OmissionReason, observed_tick: Tick }` component (per-agent, ring-buffered to `omission_log_capacity`). Removes `CognitiveProfile.max_snapshot_entities_per_place`.
- `worldwake-sim` — perception batch helper records an omission entry whenever an entity is dropped from the per-place observation due to budget. Existing `collect_direct_local_observation_batch` extended.
- `worldwake-ai` — `crates/worldwake-ai/src/agent_tick/planning.rs:546` and the snapshot construction path read the agent's already-truncated observations rather than re-truncating. Decision-trace `RootCandidateTrace` gains an `OmittedAtPerception` annotation surfaced from the new component when the planner would otherwise expect a missing entity.
- `worldwake-cli` — `PerceptionProfileDef` extended with optional `salience_policy` and `omission_log_capacity` fields (omitted-default for back-compat in scenarios). Observer Section 1 (perception summary) renders top-K omissions per agent for the run.

## Dependencies

- S105 (Observation Salience Filtering) — completed. Provides `PerceptionProfile.observation_budget` and deterministic priority truncation. S135 reuses the same budget; it does not add a second budget.
- S110 (Decision History Events) — completed. The new omission records are not events themselves (FND-27 — they are derived per-tick state, not history). The decision-trace surface that surfaces them is already in place.
- S101 (Activation-Based Belief Decay) — completed. Omission records use the same activation/retention machinery for cleanup.

## Design Goals

1. **Single perception cap, not two.** The planner snapshot must consume the same belief observations the perception system produced. No second truncation layer.
2. **Every omission has an in-world reason.** Dropped entities produce an `ObservationOmission { reason }` where `reason` names the in-world cause (`OverBudget`, `OcclusionPolicy`, `SalienceBelowFloor`). Never "planner cap was 50."
3. **Ring-buffered, agent-local.** Omission records are per-agent state, ring-buffered to `omission_log_capacity` (default 16). Per FND-22A (concrete learned state) and FND-27 (caches, not truth) — they are recent perceptual residue, decayed by activation, not authoritative event-log history.
4. **Salience policy is per-agent.** S105 already implements priority-based deterministic truncation; S135 names that policy explicitly via `SaliencePolicy::PriorityRanked` and reserves room for `SaliencePolicy::NeedWeighted` (existing need-pressure boost), `SaliencePolicy::OcclusionAware` (future), so per-agent variation is possible (FND-22).
5. **Determinism preserved.** Omission entries emit in `BTreeMap`-stable order. Ring-buffer eviction is FIFO over entry insertion order.
6. **No new event tag.** Omissions are derived per-tick state and ring-buffered cache, not authoritative history. Observer reports read the component; tests assert on the component.
7. **No silent fall-through.** If an action handler revalidates against an entity the agent's belief store does not hold, and that entity has a recent `ObservationOmission` record, the resulting `Discrepancy` (existing typed taxonomy) carries an `Omission(reason)` annotation so failure is attributable.
8. **Co-located perception remains belief-equivalent under FND-14A** for entities the agent's perception did observe. S135 does not change FND-14A's same-tick read path; it only ensures the planner snapshot and the perception output agree on which entities are visible.

## Non-Goals

- **Full attention model with stress, panic, occlusion-by-cover.** The assessor's `PerceptionBudget { max_observations, salience_policy, occlusion_policy, stress_penalty }` four-axis model is broader than S135's scope. S135 lands the *omission record* and the *single-budget collapse*; stress/occlusion/panic axes are deferred to a future spec when the underlying combat/lighting/crowd substrate exists.
- **Cross-agent visibility into omissions.** Omission records are per-agent only. No `ShareBelief`-style propagation of "I missed seeing the dragon" to other agents.
- **Reverse-replay of dropped entities.** Once dropped, the entity is not opportunistically re-perceived later in the same tick. The next tick's perception pass either includes it (budget allowing) or omits it again with a fresh record.
- **Save-format break.** Omission records are ring-buffered transient state; they are persisted under the existing belief-store delta path with an additive `BeliefStoreDiff::OmissionLog` variant. `SAVE_FORMAT_VERSION` increments by one.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `OmissionReason` is a typed enum, never a numeric "attention score." |
| FND-7 (Locality of Motion, Interaction, and Communication) | Omission records make perceptual locality auditable: the agent's observable set is precisely the entities they perceived, not a planner-side superset. |
| FND-12 (Performance May Compress Computation, Never Causality) | The double-cap was a causality compression. Removing it restores the invariant that planner reasoning matches perceived reality. |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | Co-located observation remains belief-equivalent for entities perception did surface; entities perception dropped are recorded as omitted, not silently invisible. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | Omission is a typed first-class state — "I did not see X this tick because Y" — distinct from "X does not exist" or "I have no opinion about X." |
| FND-27 (Derived Summaries Are Caches, Never Truth) | Omission records are bounded ring buffers, never authoritative truth. Deleting them recovers no lost facts; only the explanation. |
| FND-29 (Debuggability Is a Product Feature) | Observer reports and goldens can answer "why did this agent ignore the dragon next to them?" — the answer is an inspectable omission record. |

## Deliverables

### `worldwake-core::belief::PerceptionProfile` extension

```rust
pub struct PerceptionProfile {
    pub observation_budget: u8,            // existing (S105)
    pub salience_policy: SaliencePolicy,   // NEW
    pub omission_log_capacity: u8,         // NEW (default 16)
    // existing fields preserved
}

pub enum SaliencePolicy {
    PriorityRanked,                  // current S105 behavior; default
    NeedWeighted,                    // current S105 need-pressure boost as explicit policy
}
```

### `ObservationOmission` (new component)

```rust
pub struct ObservationOmission {
    pub omitted_entity: EntityId,
    pub reason: OmissionReason,
    pub observed_tick: Tick,
}

pub enum OmissionReason {
    OverBudget { budget: u8, candidates_seen: u16 },
    SalienceBelowFloor { policy: SaliencePolicy },
}

pub struct ObservationOmissionLog {
    pub entries: VecDeque<ObservationOmission>,  // ring-buffered to omission_log_capacity
}
```

`ObservationOmissionLog` is a per-agent component, registered on `EntityKind::Agent`, defaulted (universal-profile-style — see scenario contract).

### `CognitiveProfile.max_snapshot_entities_per_place` removal

Delete the field. Its 22+ call sites in `crates/worldwake-ai/src/` (and 3 test sites) read the agent's `BeliefStore` truncation result directly. The hard 50-cap goes away.

### Decision-trace annotation

`crates/worldwake-ai/src/decision_trace.rs::RootCandidateTrace` gains `omitted_anchor: Option<OmissionReason>` populated when the planner discards a synthesized root candidate because its anchor entity is in the agent's `ObservationOmissionLog` rather than belief store.

### Observer Section 1 extension

`crates/worldwake-cli/src/bin/observer.rs` Section 1 (perception summary) renders the top-K omissions per agent across the run, grouped by `OmissionReason` variant. Default K=5; override via `--top-omissions <K>`.

### Scenario contract

`AgentDef.perception_profile` continues to accept `omission_log_capacity` as an optional field; default applies when absent (FND-22 universal-profile contract per `docs/spec-drafting-rules.md` Section 5).

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** Omission records do not propagate cross-agent. They are written by the perception batch helper at the same site that already truncates observations and read only by:
   - the agent's own planning snapshot (replacing the deleted re-truncation pass),
   - the agent's own decision-trace annotation,
   - the observer's read-only diagnostic surface.
   No new cross-agent path.
2. **Positive-feedback analysis.** No amplifying loop. An agent who omits an entity does not become *more* likely to omit it next tick — perception runs independently each tick under the same budget.
3. **Concrete dampeners.** Not applicable.
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state** (per agent): `ObservationOmissionLog` (ring-buffered VecDeque), `PerceptionProfile.{salience_policy, omission_log_capacity}`.
   - **Derived read-model** (per-tick): the planning snapshot's per-place entity list (now derived directly from the belief store, not re-truncated); the observer's omission summary.

## SystemFn Integration

No new `SystemFn`. The omission-record write happens inside `collect_direct_local_observation_batch`, which already runs as part of the perception system. Tick ordering unchanged.

## Component Registration

- `ObservationOmissionLog` — register on `EntityKind::Agent` with default-empty (universal). `register_component_schema()` in `crates/worldwake-core/src/component_schema.rs` gains the new entry.
- `PerceptionProfile` — already registered (S105).
- `CognitiveProfile.max_snapshot_entities_per_place` — removed; no schema migration needed since the field is registry-data, not a stored component.

## Cross-System Interactions

- **Sim → Core**: perception writes `ObservationOmissionLog` through the existing belief-store delta path.
- **AI → Core**: planner reads belief store directly (no re-truncation) and reads `ObservationOmissionLog` for trace annotation.
- **CLI → AI**: observer reads `ObservationOmissionLog` via the existing event-log replay surface.

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

`PerceptionProfile` is the per-agent profile. New fields `salience_policy` and `omission_log_capacity` are profile-driven and per FND-22 vary by agent.

## Validation and Falsification

- **Golden coverage**: new `golden_perception_omission.rs` with three scenarios:
  1. Crowded place with 60 entities, budget 24 → expect 36 `OmissionReason::OverBudget` records, no planner-side discard pass.
  2. Need-weighted policy under hunger pressure → expect food-items above non-food-items in the observed set, omitted entries logged for the rest.
  3. Action revalidation against an omitted entity → expect `Discrepancy` annotated with `Omission(reason)`.
- **Determinism regression**: existing 1440-tick survival goldens produce identical canonical state hashes when `max_snapshot_entities_per_place` was the binding cap (i.e., `observation_budget` was already the binding cap, so the regression bound holds: identical state hashes pre/post S135 for `survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron`).
- **Negative test**: an agent's `ObservationOmissionLog` never contains an entity that is also in their `BeliefStore` for the same tick.

## Risks

- **Default-budget regression risk.** If `observation_budget = 24` is more restrictive than the deleted `max_snapshot_entities_per_place = 50` in any committed scenario, agents will lose access to entities they previously planned with. Mitigation: ticket-001 measures the budget gap on every committed `scenarios/*.ron`; scenarios with >24 entities per place at any tick get an authored profile override before deletion.
- **`ObservationOmissionLog` save-format growth.** Ring buffer of 16 entries × N agents could grow event-log delta size. Mitigation: leverage S71 (Event Log Delta Compaction) — omission-log diffs piggyback on the existing `BeliefStoreDiff` compact path.
