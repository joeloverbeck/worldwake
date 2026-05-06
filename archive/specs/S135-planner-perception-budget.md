# S135: Planner Snapshot Perception Budget and Observation Omission

**Status**: COMPLETED

## Summary

`max_snapshot_entities_per_place = 50` (`crates/worldwake-core/src/cognitive_profile.rs:96`, default 50, used at `crates/worldwake-ai/src/agent_tick/planning.rs:546`) silently truncates the planner's per-place snapshot. If a co-located place holds 80 relevant entities the planner sees only the first 50 in deterministic order. The agent fails to perceive a co-located enemy, corpse, container, workstation, or sale lot — for no in-world reason. This violates FND-7 (locality) and the spirit of FND-14A (same-tick co-located observation is belief-equivalent), because the agent's belief-view shape diverges from what perception would actually deliver.

S105 (Observation Salience Filtering) addressed the *perception* layer with `PerceptionProfile.observation_budget` (`crates/worldwake-core/src/belief.rs:2573`, default 24) and deterministic-priority truncation in `collect_direct_local_observation_batch` (`crates/worldwake-systems/src/perception.rs:639`). The planner snapshot is a *separate* truncation pass over the agent's already-accumulated belief observations, capped per-place at 50 inside `build_planning_snapshot_with_blocked_facility_uses`. The two caps operate on different windows — perception bounds *per-tick incoming* observations, the planner bounds *accumulated* belief entities at a place — but stacking them silently is exactly the FND-12 violation the assessor flagged: performance compressing causality.

S135 collapses the planner-snapshot truncation by removing the per-place cap entirely. Under the live planner contract, the actor's current place remains an authoritative same-tick local surface, while remote/non-local entities enter through accumulated beliefs or explicit evidence. Every entity the perception system dropped becomes an inspectable `ObservationOmission` record naming the entity, the in-world reason it was dropped, and the tick — so observer reports and goldens can verify "the agent could not act on X because their attention budget was full," not "the planner cap was 50."

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Completed 2026-05-06

## Crates

- `worldwake-core` — extends `PerceptionProfile` (`belief.rs:2554`) with `salience_policy: SaliencePolicy` (typed enum, default `PriorityWithNeedBoost`) and `omission_log_capacity: u8` (default 16), both `#[serde(default)]` for back-compat with existing scenarios. Adds new types `ObservationOmission`, `ObservationOmissionLog`, `OmissionReason`, `SaliencePolicy`. Stores `ObservationOmissionLog` as a field on `AgentBeliefStore`, not as a separate component, so omission facts have one canonical belief-store path. Adds `Discrepancy::Omission(OmissionReason)` variant. Removes `CognitiveProfile.max_snapshot_entities_per_place`. Adds paired `omission_log_added` / `omission_log_removed` fields to `BeliefStoreDiff` (`belief.rs:1120`) for delta-compaction parity with the other belief-store sub-stores.
- `worldwake-systems` — perception batch helper (`collect_direct_local_observation_batch` at `perception.rs:639`) records an omission entry whenever an entity is dropped from the per-place observation due to budget or salience floor. Existing truncation logic already uses S105's deterministic priority — S135 adds the omission write alongside the existing `truncate(observation_budget)` call.
- `worldwake-sim` — adds a `GoalBeliefView` accessor for `ObservationOmissionLog` so the AI crate can read the log without violating FND-26 (no direct cross-crate calls); backed by the existing `agent_belief_store` read surface and the live blanket `GoalBeliefView` implementation.
- `worldwake-ai` — `crates/worldwake-ai/src/agent_tick/planning.rs:546` and the snapshot construction path consume the agent's accumulated belief observations directly (the per-place cap argument is removed). `RootCandidateTrace` (`decision_trace.rs:786`) gains `omitted_anchor: Option<OmissionReason>` populated from `search/candidates.rs` when a synthesized root candidate's anchor is absent from the planning snapshot and present in the agent's `ObservationOmissionLog`. Hypothetical-effect-sink revalidation paths (`effect_sink_hypothetical.rs`) gain support for emitting `Discrepancy::Omission(reason)` when revalidation fails because of an omitted entity.
- `worldwake-cli` — `PerceptionProfile` deserialization on `AgentDef.perception_profile` (`scenario/types.rs:447`) gains the two new fields automatically through `#[serde(default)]`. Observer's "Perception Trace Summary" sub-section (`observer.rs:3091`, inside Section 5 "Raw Event Sample") gains a per-agent top-K omissions block grouped by `OmissionReason` discriminant. Default K=5; override via new `--top-omissions <K>` CLI flag. The observer reads the current simulated world's belief store after normal tick/delta application.

## Dependencies

- S105 (Observation Salience Filtering) — completed. Provides `PerceptionProfile.observation_budget` and deterministic priority truncation in `compute_observation_priority` (`crates/worldwake-systems/src/perception.rs:714`). S135 reuses the same budget and the same priority composition; it does not add a second budget.
- S110 (Decision History Events) — completed. The new omission records are not events themselves (FND-27 — they are derived per-tick state, not history). The decision-trace surface that surfaces them is already in place.
- S101 (Activation-Based Belief Decay) — completed. Omission records use the same activation/retention machinery for cleanup.
- S71 (Event Log Delta Compaction) — completed. Omission-log diffs piggyback on the existing `BeliefStoreDiff` compact path through the new paired fields.

## Design Goals

1. **Single perception cap, not two.** The planner snapshot has no second per-place truncation layer. Same-place entities remain planner-visible through the live authoritative local surface; remote/non-local entities enter through accumulated beliefs or explicit evidence.
2. **Every omission has an in-world reason.** Dropped entities produce an `ObservationOmission { reason }` where `reason` names the in-world cause (`OverBudget`, `SalienceBelowFloor`). Never "planner cap was 50."
3. **Ring-buffered, agent-local.** Omission records are per-agent belief-store state, ring-buffered to `omission_log_capacity` (default 16). Per FND-22A (concrete learned state) and FND-27 (caches, not truth) — they are recent perceptual residue, decayed by activation, not authoritative event-log history.
4. **Salience policy is per-agent.** S105 already implements priority-based deterministic truncation with need-pressure boosting. S135 names that policy explicitly via `SaliencePolicy::PriorityWithNeedBoost` and reserves the enum for future genuinely-different policies (e.g., `OcclusionAware` when the lighting/cover substrate exists). Per-agent variation is possible (FND-22) by extending the enum later.
5. **Determinism preserved.** Omission entries emit in `BTreeMap`-stable order (agent-id keyed) with ring-buffer eviction FIFO over entry insertion order.
6. **No new event tag.** Omissions are derived per-tick state and ring-buffered cache, not authoritative history. Observer reports read `AgentBeliefStore.observation_omission_log`; tests assert on that nested store field.
7. **No silent fall-through.** If an action handler revalidates against an entity the agent's belief store does not hold, and that entity has a recent `ObservationOmission` record, the resulting `Discrepancy::Omission(reason)` carries the typed in-world cause so failure is attributable.
8. **Co-located perception remains belief-equivalent under FND-14A.** S135 does not change FND-14A's same-tick read path; it removes the extra planner cap and records perception-budget omissions so local and non-local snapshot behavior remain attributable.

## Non-Goals

- **Full attention model with stress, panic, occlusion-by-cover.** A four-axis `PerceptionBudget { max_observations, salience_policy, occlusion_policy, stress_penalty }` model is broader than S135's scope. S135 lands the *omission record* and the *single-budget collapse*; stress/occlusion/panic axes are deferred to a future spec when the underlying combat/lighting/crowd substrate exists.
- **Cross-agent visibility into omissions.** Omission records are per-agent only. No `ShareBelief`-style propagation of "I missed seeing the dragon" to other agents.
- **Reverse-replay of dropped entities.** Once dropped, the entity is not opportunistically re-perceived later in the same tick. The next tick's perception pass either includes it (budget allowing) or omits it again with a fresh record.
- **Save-format policy.** Omission records are ring-buffered transient state persisted under the existing belief-store delta path with paired `omission_log_added` / `omission_log_removed` fields on `BeliefStoreDiff`; ticket 001 bumped `SAVE_FORMAT_VERSION` for that substrate. Removing `CognitiveProfile.max_snapshot_entities_per_place` in ticket 003 also changes the serialized current `CognitiveProfile` component shape, so ticket 003 bumps `SAVE_FORMAT_VERSION` 67→68. Adding the persisted `Discrepancy::Omission(OmissionReason)` payload variant in ticket 004 bumps the current save format again to 69. Older versions remain rejected per the repo's no-backward-compatibility rule.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `OmissionReason` is a typed enum, never a numeric "attention score." |
| FND-7 (Locality of Motion, Interaction, and Communication) | Omission records make perceptual locality auditable: the agent's observable set is precisely the entities they perceived, not a planner-side superset. |
| FND-12 (Performance May Compress Computation, Never Causality) | The double-cap was a causality compression. Removing it restores the invariant that planner reasoning matches perceived reality. |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | Co-located observation remains belief-equivalent for entities perception did surface; entities perception dropped are recorded as omitted, not silently invisible. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | Omission is a typed first-class state — "I did not see X this tick because Y" — distinct from "X does not exist" or "I have no opinion about X." |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Cross-crate AI reads of `ObservationOmissionLog` go through a new `GoalBeliefView` accessor; perception writes go through the existing belief-store delta path. No direct cross-system calls. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | Omission records are bounded ring buffers, never authoritative truth. Deleting them recovers no lost facts; only the explanation. |
| FND-29 (Debuggability Is a Product Feature) | Observer reports and goldens can answer "why did this agent ignore the dragon next to them?" — the answer is an inspectable omission record. |

## Deliverables

### D1. `worldwake-core::belief::PerceptionProfile` extension

```rust
pub struct PerceptionProfile {
    pub observation_budget: u8,            // existing (S105)

    #[serde(default)]
    pub salience_policy: SaliencePolicy,   // NEW; default PriorityWithNeedBoost

    #[serde(default = "default_omission_log_capacity")]
    pub omission_log_capacity: u8,         // NEW; default 16
    // existing fields preserved
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub enum SaliencePolicy {
    /// Current S105 behavior: priority class composed with `need_salience_boost`
    /// in `compute_observation_priority`. Single composite policy; future
    /// genuinely-different policies (occlusion-aware, stress-modulated) extend
    /// this enum with new variants.
    #[default]
    PriorityWithNeedBoost,
}
```

`PerceptionProfile` carries `#[serde(deny_unknown_fields)]` already; the two new fields use explicit `#[serde(default)]` so existing committed scenarios continue to deserialize unchanged. `SaliencePolicy` derives `Copy` to keep `PerceptionProfile`'s existing `Copy` derive intact.

### D2. `ObservationOmission`, `ObservationOmissionLog`, `OmissionReason` (new types in `worldwake-core`)

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationOmission {
    pub omitted_entity: EntityId,
    pub reason: OmissionReason,
    pub observed_tick: Tick,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OmissionReason {
    OverBudget { budget: u8, candidates_seen: u16 },
    SalienceBelowFloor { policy: SaliencePolicy },
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationOmissionLog {
    pub entries: VecDeque<ObservationOmission>,  // ring-buffered to omission_log_capacity
}
```

`OmissionReason` derives `Copy` so `Discrepancy` (which derives `Copy`) can take it as a payload (see D7). `ObservationOmissionLog` is **runtime-only / scenario-exempt** state owned by `AgentBeliefStore` (analogous to `social_observations`): every agent starts with `ObservationOmissionLog::default()` through `AgentBeliefStore::new()`; no `AgentDef` field, no `*Def` wrapper, no scenario authoring. It is not a separate `EntityKind::Agent` component; this avoids a second lawful transport path for the same omission fact and keeps `BeliefStoreDiff` as the single compact replay path.

### D3. `CognitiveProfile.max_snapshot_entities_per_place` removal

Delete the field. Codebase analysis (workspace-wide grep): 1 active runtime read at `crates/worldwake-ai/src/agent_tick/planning.rs:546` (passed to `build_planning_snapshot_with_blocked_facility_uses` as the `max_per_place` argument); ~6 test-mock `CognitiveProfile` constructors in `worldwake-ai` (`failure_handling.rs:1906`, `decision_runtime.rs:452`, `goal_model.rs:2405`, `agent_tick/planning.rs:2416`, `agent_tick/tests.rs:200`, `search/tests.rs:84`); ~6 default-context references in core/CLI/sim (`cognitive_profile.rs` field/Default/tests, `delta.rs:622`, `scenario/types.rs:1537`). All sites are deletion-by-removal. The corresponding `max_per_place` parameter on `build_planning_snapshot_with_blocked_facility_uses` and the per-place `truncate(max_per_place)` at `planning_snapshot.rs:1264` are removed in the same change. Because `CognitiveProfile` is serialized in the current save format, this deletion bumps `SAVE_FORMAT_VERSION` 67→68.

### D4. Decision-trace annotation

`crates/worldwake-ai/src/decision_trace.rs::RootCandidateTrace` (line 786) gains an in-memory diagnostic field:

```rust
pub omitted_anchor: Option<OmissionReason>,
```

populated from the root trace construction path in `crates/worldwake-ai/src/search/candidates.rs` when a root candidate's anchor entity is absent from the planning snapshot and present in the agent's `ObservationOmissionLog`. `RootCandidateTrace` is not currently a serde-persisted carrier, so this field does not require a save-format bump or serde default proof.

### D5. `GoalBeliefView` accessor for `ObservationOmissionLog`

Add a new method to the `GoalBeliefView` trait (`crates/worldwake-sim/src/belief_view.rs:268`):

```rust
fn observation_omission_log(&self, agent: EntityId) -> Option<&ObservationOmissionLog>;
```

Backed by the existing `agent_belief_store` read surface (reading the nested log from `AgentBeliefStore`) and the live blanket `GoalBeliefView` implementation so the AI crate's planning and decision-trace paths can consult the omission log without violating FND-26.

### D6. Observer "Perception Trace Summary" extension

Inside `crates/worldwake-cli/src/bin/observer.rs` Section 5 "Raw Event Sample", the existing "### Perception Trace Summary" sub-heading (line 3091) gains a per-agent top-K omissions block grouped by `OmissionReason` discriminant (`OverBudget` vs. `SalienceBelowFloor`). Default K=5; override via new `--top-omissions <K>` CLI flag (added to the observer's `ObserverCli` struct alongside its other top-N flags). The omission block reads `ObservationOmissionLog` from the current simulated world's `AgentBeliefStore` after normal tick/delta application.

### D7. `Discrepancy::Omission(OmissionReason)` enum variant

`Discrepancy` (`crates/worldwake-core/src/discrepancy.rs:8`) derives `Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`. Add the variant:

```rust
pub enum Discrepancy {
    // ... 11 existing variants ...
    /// The agent could not revalidate against an entity that perception had
    /// dropped from the belief store under the given salience-budget reason.
    Omission(OmissionReason),
}
```

`OmissionReason: Copy` (D2) keeps the enum's `Copy` derive intact. Implementation requires updating exhaustive-match sites across the workspace (~145 use sites total per pre-implementation grep, of which most are construction `Err(Discrepancy::...)` sites in `effect_sink_hypothetical.rs`, `needs_actions.rs`, `search_actions.rs`; exhaustive-match sites — `match d { Discrepancy::X => ... }` — are the subset requiring new arms). The variant is constructed at hypothetical-effect-sink revalidation sites in `crates/worldwake-ai/src/effect_sink_hypothetical.rs` when a `MissingObservation` would otherwise be returned and the missing entity has a current `ObservationOmission` record in the bounded log. No age threshold is added in this ticket because the live omission substrate has no profile-backed activation window for revalidation; entries stop attributing once they leave the ring buffer.

### D8. Scenario contract for new `PerceptionProfile` fields

`AgentDef.perception_profile: Option<PerceptionProfile>` (`crates/worldwake-cli/src/scenario/types.rs:447`) deserializes via the existing `unwrap_or_default()` path in `spawn_agent()` (`scenario/mod.rs:636-637`). With `#[serde(default)]` on the two new fields (D1), existing committed scenarios deserialize without modification; scenarios that want non-default values author them explicitly under `perception_profile`. No `*Def` wrapper is introduced — `PerceptionProfile` is already directly serde-deserializable.

`ObservationOmissionLog` is runtime-only and absent from the scenario contract entirely (per D2).

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** Omission records do not propagate cross-agent. They are written by the perception batch helper (`worldwake-systems/src/perception.rs:639`) at the same site that already truncates observations (line 665) and read only by:
   - the agent's own decision-trace annotation (via the new `GoalBeliefView::observation_omission_log` accessor),
   - the agent's own hypothetical-effect-sink revalidation path (constructing `Discrepancy::Omission(reason)` when belief revalidation would otherwise fail),
   - the observer's read-only diagnostic surface (replayed from the event-log delta path).
   No new cross-agent path. The planner snapshot itself does not use the omission log for admission; same-place entities come from the authoritative local surface, while remote/non-local entities come from accumulated beliefs or explicit evidence.
2. **Positive-feedback analysis.** No amplifying loop. An agent who omits an entity does not become *more* likely to omit it next tick — perception runs independently each tick under the same budget.
3. **Concrete dampeners.** Not applicable.
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state** (per agent): `AgentBeliefStore.observation_omission_log` (ring-buffered `VecDeque`), `PerceptionProfile.{salience_policy, omission_log_capacity}`.
   - **Derived read-model** (per-tick): the planning snapshot's per-place entity list (now derived directly from the belief store, no per-place cap); the observer's omission summary; the optional `RootCandidateTrace.omitted_anchor` annotation.

## SystemFn Integration

No new `SystemFn`. The omission-record write happens inside `collect_direct_local_observation_batch` (`crates/worldwake-systems/src/perception.rs:639`), which already runs as part of the perception system. Tick ordering unchanged.

## Component Registration

- `ObservationOmissionLog` — not a separate component. It is a field on `AgentBeliefStore` and is default-empty through `AgentBeliefStore::new()`. Runtime-only and scenario-exempt.
- `PerceptionProfile` — already registered (S105). The two new fields ride the existing registration; no schema change.
- `CognitiveProfile.max_snapshot_entities_per_place` — removed; current save format bumped because `CognitiveProfile` is a serialized component.

## Cross-System Interactions

- **Systems → Core**: perception (`worldwake-systems`) writes `AgentBeliefStore.observation_omission_log` through the existing belief-store delta path (`BeliefStoreDiff` paired fields).
- **Sim → Core**: `GoalBeliefView::observation_omission_log` accessor (new) reads the log from `AgentBeliefStore` through the existing `agent_belief_store` read surface and live blanket trait implementation.
- **AI → Sim**: planner reads accumulated beliefs directly (no re-truncation) via existing `GoalBeliefView` accessors; trace annotation and hypothetical-effect-sink revalidation read `ObservationOmissionLog` via the new accessor.
- **CLI → AI/Core**: observer reads `ObservationOmissionLog` from the current simulated world's core belief-store component (no direct call into the AI crate's runtime).

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

`PerceptionProfile` is the per-agent profile. New fields `salience_policy` and `omission_log_capacity` are profile-driven and per FND-22 vary by agent.

## Validation and Falsification

- **Golden coverage**: new `golden_perception_omission.rs` with three scenarios:
  1. Crowded same-place setup with 60 equal-priority entities, budget 24 → expect 36 `OmissionReason::OverBudget` records, deterministic omission ordering, disjoint retained/omitted belief-store sets, and no second same-place planner cap.
  2. Need-weighted policy under hunger pressure → expect food-items retained above waste and omitted entries logged for the lower-priority waste.
  3. Revalidation against an omitted non-snapshot entity → expect `Discrepancy::Omission(reason)` returned from the hypothetical revalidation surface, with `RootCandidateTrace.omitted_anchor` populated for the matching root candidate.
- **Generated inventory/docs**: `python3 scripts/golden_inventory.py --write --check-docs` records the new scenarios in `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, `docs/generated/golden-coverage-matrix.md`, and `docs/generated/golden-scenario-details/perception-omission.md`.
- **Negative test**: an agent's `ObservationOmissionLog` never contains an entity that is also in their `BeliefStore` for the same tick.

## Risks

- **Default-budget regression risk** (low likelihood given current scenarios). If `observation_budget = 24` is more restrictive than the deleted `max_snapshot_entities_per_place = 50` for any place at any tick in any committed scenario, agents will lose access to entities they previously planned with. Mitigation: ticket-001 measures **two** values across every committed `scenarios/*.ron` over a full survival run: (a) max per-tick observed-entity count per place (relevant to the perception budget), and (b) max accumulated-belief-entity count per place (relevant to the previously-binding planner cap). All three survival goldens have ≤4 agents and modest facility density — expected (a) and (b) both well under 24 and 50 respectively. Scenarios where (a) > 24 require an authored profile override; scenarios where (b) > 50 are the ones that actually used the deleted planner cap and may show snapshot-size growth post-S135.
- **Snapshot-size growth at long-lived crowded places.** Removing the per-place planner cap means accumulated beliefs (which can grow beyond perception's per-tick budget over time) are no longer bounded at snapshot construction. For long-running scenarios with high-density places, the snapshot's per-place entity list will be larger. ticket-001's measurement (b) above is the canary; if a real-world scenario exceeds 50 accumulated entities per place, the FND-12 fix is to tighten activation-based decay (S101) rather than re-introduce a snapshot cap.
- **`ObservationOmissionLog` save-format growth.** Ring buffer of 16 entries × N agents could grow event-log delta size. Mitigation: leverage S71 (Event Log Delta Compaction) — omission-log diffs piggyback on the existing `BeliefStoreDiff` compact path through D1's paired `omission_log_added` / `omission_log_removed` fields.
- **Discrepancy variant blast radius.** Adding `Discrepancy::Omission(OmissionReason)` requires updating every exhaustive match on `Discrepancy` in the workspace. Most use sites construct `Err(Discrepancy::X)` and don't pattern-match exhaustively, but the audit must cover the full ~145 use sites to find the genuinely-exhaustive matches. This is a single-commit migration cost, not a runtime risk.

## Outcome

Completed on 2026-05-06.

- Landed the S135 perception-omission substrate across the ticket chain: `PerceptionProfile` omission settings, `ObservationOmissionLog`, omission diffs, the `GoalBeliefView` accessor, `Discrepancy::Omission`, `RootCandidateTrace.omitted_anchor`, observer omission rendering, and removal of `CognitiveProfile.max_snapshot_entities_per_place`.
- Added `crates/worldwake-ai/tests/golden_perception_omission.rs` with scenarios 381-383 proving over-budget omission writes, need-weighted priority retention, and typed omission attribution through decision traces plus hypothetical revalidation.
- Regenerated generated golden inventory/docs, including `docs/generated/golden-scenario-details/perception-omission.md`.
- Truth-synced the spec to the live current-place planner contract: co-located entities remain planner-visible through the authoritative same-tick local surface, while S135 owns explicit omission attribution for perception-budget drops and non-local/absent snapshot anchors.

Deviations from original plan:

- The final no-second-cap proof asserts all 60 co-located local lots remain planner-visible under `docs/planner-contracts.md`, rather than treating the perception budget as a same-place planner admission cap.
- The older-baseline state-hash regression was removed because the repo does not carry a separate older S135 baseline hash. Existing survival goldens continue to own long-run survival behavior.
- Revalidation attribution is proved through `search_plan` root-candidate traces and direct `HypotheticalEffectSink` revalidation, not a fully autonomous action lifecycle.

Verification results:

- `cargo test -p worldwake-ai --test golden_perception_omission -- --list`
- `cargo test -p worldwake-ai --test golden_perception_omission`
- `python3 scripts/golden_inventory.py --write --check-docs`
- `cargo test -p worldwake-ai`
- `./scripts/verify.sh`
