# S141: Motive Source Ledger

**Status**: Draft

## Summary

Today's goal ranking is a derived utility computation: emitters produce `GoalOffer`s with attached evidence references; `ranking::compare_ranked_goals` (`crates/worldwake-ai/src/ranking.rs`, made file-private by S123) reads needs, drives, learned opportunities, source reliability, and per-agent profile weights to produce a `motive_score: u32`. The score becomes the cross-goal ordering authority. The motive *source* — whether the agent committed because they're hungry, in pain, loyal to another, vengeful, or chasing an opportunity — is implicit in the score's components, not first-class state. Per FND-3 (concrete state over abstract scores), the architectural shape should be inverted: motive sources are the authoritative state; ranking is a derived view over motive sources, not a free-floating utility number.

S141 lands the `MotiveSource` enum and a `MotiveSourceRef` carrier in `worldwake-core`. Each `GoalOffer` (defined in `worldwake-ai`) carries one or more `MotiveSourceRef`s naming the per-agent state that gives the goal weight: `NeedPressure(HomeostaticNeedId)`, `Pain(WoundId)`, `OfficeDuty(EntityId)`, `Loyalty(EntityId)`, `Greed(OpportunityKey)`, `Shame(EntityId)`, `Revenge(ViolationId)`. The existing `motive_score` function in `ranking.rs` is refactored body-only: instead of dispatching on `GoalKind` and reading needs/drives directly, it iterates `offer.motive_sources` and dispatches per `MotiveSource` variant. `compare_ranked_goals` keeps its identity (file-private per S123). S136's always-on decision-event payload gains `decisive_motive_sources: Vec<MotiveSourceRef>` so the post-commit causal record names the load-bearing motive sources rather than only the abstract score.

The seven variants above cover the per-agent state types that exist today (`HomeostaticNeedId`, `WoundId`, `OpportunityKey`, `ViolationId`, and bare `EntityId` for office/loyalty/shame anchors). Variants whose referent substrate has not yet been built — `Fear`, `Obligation`, `Debt`, `Habit`, `Curiosity` — are explicitly deferred to Phase 12 follow-up specs that land the corresponding state (threat-belief substrate, contract artifacts, debt artifacts, habit reinforcement, hypothesis ID surface). The `MotiveSource` enum is open to extension when those substrates exist; until then the spec respects Design Goal 5 (no new authoritative state).

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-core` — adds `motive_source` module owning `MotiveSource` enum and `MotiveSourceRef` carrier. References only types already in core: `HomeostaticNeedId` (`needs.rs:19`), `WoundId` (`wounds.rs:9`), `OpportunityKey` (`goal.rs:201`), `ViolationId` (`violation.rs:20`), `EntityId` (`ids.rs:44`), `Tick` (`ids.rs:57`). Extends the always-on decision-event payload in `decision_event_payload.rs` with `decisive_motive_sources: Vec<MotiveSourceRef>` per the S136 substrate.
- `worldwake-ai` — extends `GoalOffer` (`crates/worldwake-ai/src/goal_model.rs:2038`) with `motive_sources: Vec<MotiveSourceRef>`. Refactors the body of the existing `motive_score` function (`ranking.rs:1007`) to derive the score from `offer.motive_sources` via a per-`MotiveSource`-variant scoring dispatch. `compare_ranked_goals` (file-private per S123) is unchanged in identity. Decision-trace `RankedGoalSummary` (`decision_trace.rs:529`) gains a `motive_source_contributions: Vec<(MotiveSourceRef, u32)>` field.
- `worldwake-systems` — no change. The seven kept motive-source variants reference state already produced and stored by existing systems.
- `worldwake-cli` — observer Section 3b (Decision History, `bin/observer.rs:833`) extends the existing `GoalCommitted` rendering to surface motive-source contributions per commit. `ProfileHomogeneity` lint (per S111, `scenario/lints.rs:62`) extends to detect cloned utility profiles across the 5 new motive-class weight fields.

## Dependencies

- S112 (Portfolio Planning) — completed and archived. The portfolio's three slots (Survival/Commitment/Economic) are aggregations over motive-source classes; S141 makes the aggregation explicit.
- S115 (Agenda Manager) — completed and archived. `AgendaEntry` (`crates/worldwake-ai/src/agenda_types.rs:20`) already carries `offer: GoalOffer` and `motive_score: u32`; motive sources flow through the offer automatically. No agenda-shape change required.
- S123 (Preference Ordering Authority) — completed and archived. `compare_ranked_goals` remains the single file-private comparator. S141 changes the body of its callee `motive_score`, not its identity.
- S136 (Decision Event Payload Extension) — completed and archived at `archive/specs/S136-decision-event-payload-extension.md`. Soft dependency satisfied: because S136 landed first, S141 owns adding `decisive_motive_sources: Vec<MotiveSourceRef>` to the always-on payload (delivered as D6 below).
- S110 (Decision History Events) — completed and archived. Decision events already carry the comparator-decided commit; adding motive-source provenance to the payload is additive.
- S111 (Scenario Homogeneity Lints) — completed and archived. `ProfileHomogeneity` lint extension is the deliverable touchpoint.
- S107, S130, S131 (existing learning state) — `LearnedOpportunityMemory`, `SurveyMemory`, `SourceReliability` continue to feed motive contributions through `OpportunityKey` (the `Greed` variant) and the existing per-need pressure surface; S141 does not duplicate them.

## Design Goals

1. **Motive sources are concrete state references.** Each variant of `MotiveSource` references existing per-agent state via a typed ID that already lives in `worldwake-core`. No score lives in the source — the source names *what* drives the goal; the *strength* derives from reading the referenced state at scoring time.
2. **`motive_score` is a derived view, computed by the same function as today.** The existing `motive_score(candidate: &GoalOffer, context: &RankingContext<'_>) -> u32` keeps its signature. Its body changes from a `match` on `GoalKind` to an iteration over `candidate.motive_sources` with per-variant dispatch. `compare_ranked_goals` continues to read the score; its file-private status (per S123) is preserved.
3. **Per-class scoring weights are profile-driven.** `UtilityProfile` (per-agent, `crates/worldwake-core/src/utility_profile.rs`) gains 5 new `Permille` fields covering the new non-need, non-Pain motive classes (office_duty, loyalty, greed, shame, revenge). The existing per-need weights and the existing `pain_weight` are reused as-is (not re-added). The total weighted sum produces the score deterministically.
4. **Decision-trace shows source contributions.** `RankedGoalSummary` records `(MotiveSourceRef, contribution_score)` pairs for the chosen and top rejected goals. Observer Section 3b surfaces this on each `GoalCommitted` line.
5. **No new authoritative state, no speculative variants.** Every `MotiveSource` variant kept in this spec references state that already exists. Deferred variants (`Fear`, `Obligation`, `Debt`, `Habit`, `Curiosity`) are documented as Phase 12 follow-ups whose substrate must land first.
6. **Determinism.** `Vec<MotiveSourceRef>` iteration is insertion-ordered; per-variant scoring is a fixed `match` dispatch; total score is integer arithmetic; no floats and no `HashMap`/`HashSet` in authoritative state.
7. **Backward-compat-free migration.** Goal offers without explicit motive sources are *invalid* post-S141. Every `GoalOffer` construction site is updated in the same change. A test-build `debug_assert!` catches empty `motive_sources` at offer construction; the conformance test below enforces it at the assembly boundary.
8. **No silent privilege.** Motive sources do not invoke other systems; they are pure references read at scoring time through the existing ranking context.

## Non-Goals

- **A separate `DesireToken` runtime type.** The spec previously proposed both a "MotiveSource" enum and a "DesireToken" carrier. After reassessment, the runtime carrier *is* the extended `GoalOffer` (which already has the lifecycle: offered → ranked → committed → fulfilled/abandoned). No `DesireToken` type is introduced; the conceptual term has been retired from the spec title and prose. `AgendaEntry` continues to carry `offer: GoalOffer` (`agenda_types.rs:20`) and motive sources flow through it automatically.
- **A `MotiveSource` source-of-truth refactor.** The referenced targets (`HomeostaticNeedId`, `WoundId`, `OpportunityKey`, `ViolationId`, etc.) remain authoritative wherever they currently live. S141 only adds the reference layer.
- **Cross-agent motive sharing.** `Loyalty(EntityId)` references a per-agent loyalty target; cross-agent loyalty propagation is out of scope.
- **A new event tag.** Motive sources are payload data on existing decision events (the S136 always-on payload).
- **Substrate for deferred variants.** Threat-belief IDs, contract artifacts, debt artifacts, habit reinforcement state, and per-agent hypothesis IDs are *not* invented here. Phase 12 specs that introduce those substrates will extend `MotiveSource` with the corresponding variants.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `motive_score` becomes a derived view over concrete per-agent state references rather than a free-floating numeric truth. The comparator continues to use the score; the score's provenance is now inspectable per source. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | "Agent X chose Y because they cared about Z" becomes literally inspectable: the goal commits with `motive_sources: [NeedPressure(Hunger), Greed(market_opportunity)]`. |
| FND-22 (Agent Diversity Through Concrete Variation) | Two agents with identical state but different `UtilityProfile` per-class weights rank the same motive sources differently. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Motive sources are state references read at scoring time, not cross-system commands. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `motive_score` is explicitly a derived summary; deleting it and recomputing from motive sources produces the same value. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | Goal offers without explicit motive sources are invalid post-S141. No fallback path. No deferred no-op variants — variants whose substrate doesn't exist are dropped, not stubbed. |
| FND-29 (Debuggability Is a Product Feature) | Decision events carry `decisive_motive_sources`; observer Section 3b renders contributions per commit. The causal answer to "why did this agent commit this goal?" reconstructs from the payload alone. |
| FND-29A (Causal History Is Authoritative, Append-Only, and Queryable) | Decision events carry the motive-source references; history reconstructs the *why* across ticks via the existing append-only event log. |

## Deliverables

### D1: `worldwake-core::motive_source` (new module)

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MotiveSource {
    NeedPressure { need: HomeostaticNeedId },
    Pain { wound: WoundId },
    OfficeDuty { office: EntityId },
    Loyalty { other: EntityId },
    Greed { opportunity: OpportunityKey },
    Shame { reputation_record: EntityId },
    Revenge { violation: ViolationId },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MotiveSourceRef {
    pub source: MotiveSource,
    pub introduced_tick: Tick,
}
```

The derive set matches `GoalOffer`'s current derives (`Clone, Debug, Eq, PartialEq, Serialize, Deserialize` at `crates/worldwake-ai/src/goal_model.rs:2037`) and the always-on decision-payload convention (`crates/worldwake-core/src/decision_event_payload.rs:11`). No `Copy` (variants carry sub-structs in future extensions), no `Hash` (none of the embedding types require it).

Module placement: `crates/worldwake-core/src/motive_source.rs` with `pub mod motive_source;` registered in `crates/worldwake-core/src/lib.rs`.

### D2: `GoalOffer` extension (in `worldwake-ai`)

```rust
// crates/worldwake-ai/src/goal_model.rs:2038
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalOffer {
    // existing fields preserved (key, anchor, evidence_entities, evidence_places,
    // obligation_source, commitment_impact_if_ignored, required_information_gaps,
    // invalidators, learned_expectation_refs, acquisition_quantity)
    pub motive_sources: Vec<MotiveSourceRef>,    // NEW (required, non-empty)
}
```

Required, non-empty post-S141. A test-build `debug_assert!(!offer.motive_sources.is_empty())` is inserted at the three `GoalOffer` construction sites in `crates/worldwake-ai/src/candidate_generation.rs` (lines 554, 4808, 5420) — these are the only production paths that build a `GoalOffer` for the agenda; all 53 `emit_*_candidates` functions route through one of them. The conformance test `every_goal_offer_has_motive_sources()` (D8) enforces the invariant at the assembly boundary.

A single helper `derive_default_motive_sources(goal_kind: &GoalKind, anchor: &OpportunityAnchor) -> Vec<MotiveSourceRef>` lives in `crates/worldwake-ai/src/motive_source_mapping.rs` (new file) and is called from each of the three helper sites. Emitters that already have richer context (e.g., a violation goal with the exact `ViolationId`) may override by passing an explicit `motive_sources` argument. Per-`GoalKind` mapping rules are part of D2.

### D3: `motive_score` body refactor (in `ranking.rs`)

The existing function at `crates/worldwake-ai/src/ranking.rs:1007`:

```rust
fn motive_score(candidate: &GoalOffer, context: &RankingContext<'_>) -> u32 {
    // existing body: `match candidate.key.goal_kind { ... }` dispatching to
    // drive_score / enterprise_score / raid_target_motive / ... helpers.
}
```

is refactored to:

```rust
fn motive_score(candidate: &GoalOffer, context: &RankingContext<'_>) -> u32 {
    candidate
        .motive_sources
        .iter()
        .map(|src| score_motive_source(src, context))
        .sum()
}

fn score_motive_source(src: &MotiveSourceRef, context: &RankingContext<'_>) -> u32 {
    match &src.source {
        MotiveSource::NeedPressure { need } => {
            // need_pressure_for_id (ranking.rs:1322) reads `needs.hunger`, `needs.thirst`,
            // etc. directly via field match — no `.value(need)` method exists.
            let pressure = context
                .needs
                .map(|n| need_pressure_for_id(n, *need))
                .unwrap_or(Permille::zero());
            let weight = utility_weight_for_need(context.utility, *need);
            score_from_pressure_and_weight(pressure, weight)
        }
        MotiveSource::Pain { wound } => {
            // WoundList has wound_load() (wounds.rs:83) and wound_ids() (line 91);
            // per-wound severity is read by indexing into the wound store.
            // Specific accessor is named by D3 ticket; the spec commits to the
            // pattern, not the exact method name.
            score_pain_from_wound(context, *wound, context.utility.pain_weight)
        }
        MotiveSource::OfficeDuty { office } => {
            score_office_duty(context, *office, context.utility.office_duty_weight)
        }
        MotiveSource::Loyalty { other } => {
            score_loyalty(context, *other, context.utility.loyalty_weight)
        }
        MotiveSource::Greed { opportunity } => {
            score_greed(context, opportunity, context.utility.greed_weight)
        }
        MotiveSource::Shame { reputation_record } => {
            score_shame(context, *reputation_record, context.utility.shame_weight)
        }
        MotiveSource::Revenge { violation } => {
            score_revenge(context, *violation, context.utility.revenge_weight)
        }
    }
}
```

`compare_ranked_goals` (`ranking.rs:2615`) is unchanged. The per-variant helpers extract the body fragments of today's `match candidate.key.goal_kind` arms, so the per-commit `motive_score` values are bitwise-identical to pre-S141 for every existing golden (enforced by D8 score parity).

### D4: `UtilityProfile` extension

```rust
// crates/worldwake-core/src/utility_profile.rs
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UtilityProfile {
    // existing fields preserved (hunger_weight, thirst_weight, fatigue_weight,
    // bladder_weight, dirtiness_weight, pain_weight, danger_weight,
    // enterprise_weight, social_weight, activity_awareness_weight,
    // side_benefit_weight, bounty_posting_weight, notice_posting_weight,
    // courage, care_weight)
    #[serde(default = "default_office_duty_weight")]
    pub office_duty_weight: Permille,
    #[serde(default = "default_loyalty_weight")]
    pub loyalty_weight: Permille,
    #[serde(default = "default_greed_weight")]
    pub greed_weight: Permille,
    #[serde(default = "default_shame_weight")]
    pub shame_weight: Permille,
    #[serde(default = "default_revenge_weight")]
    pub revenge_weight: Permille,
}
```

Five new `Permille` fields (one per kept non-need motive class — `NeedPressure` already covered by the existing per-need weights, `Pain` already covered by the existing `pain_weight`). Defaults:

| Field | Default | Rationale |
|-------|---------|-----------|
| `office_duty_weight` | `pm(500)` | balanced default matching existing convention |
| `loyalty_weight` | `pm(500)` | balanced default |
| `greed_weight` | `pm(500)` | balanced default |
| `shame_weight` | `pm(400)` | slight downweight — shame typically does not dominate need pressure |
| `revenge_weight` | `pm(400)` | slight downweight — revenge dampened to prevent runaway feedback loops |

`#[serde(default = "...")]` on each new field preserves save-format compatibility for snapshots created pre-S141 (the snapshot decoder fills missing fields with the per-field default). `SAVE_FORMAT_VERSION` (currently 77 at `crates/worldwake-sim/src/save_load.rs:23`) increments to 78 for the schema bump.

Per FND-22, two agents differ on these weights through scenario authoring. The conformance test `utility_profile_default_for_motive_class()` (D8) ensures every new motive class has a default function.

### D5: Decision-trace extension (`RankedGoalSummary`)

`RankedGoalSummary` at `crates/worldwake-ai/src/decision_trace.rs:529` already carries `motive_score: u32`. Add:

```rust
pub struct RankedGoalSummary {
    // existing fields preserved (motive_score, provenance, discounts,
    // acquisition_quantity, artifact_axes, ...)
    pub motive_source_contributions: Vec<(MotiveSourceRef, u32)>,    // NEW
}
```

Populated by `score_motive_source` (D3) returning a `(MotiveSourceRef, u32)` tuple alongside the sum. The same per-source contribution data feeds the always-on payload (D6) when the goal commits.

### D6: Decision-event payload extension (S136 always-on payload)

In `crates/worldwake-core/src/decision_event_payload.rs`, extend `GoalCommittedPayload` (line 156) with:

```rust
pub struct GoalCommittedPayload {
    // existing fields preserved
    pub decisive_motive_sources: Vec<MotiveSourceRef>,    // NEW
}
```

This is the soft-dep deliverable that the spec's Dependencies section promised. Matches the existing `decisive_beliefs: Vec<BeliefRef>` / `decisive_records: Vec<RecordRef>` / `decisive_world_observations: Vec<ObservationRef>` pattern (lines 346–350) — same collection type (`Vec<T>`), same naming convention. Save-format bump shares the `SAVE_FORMAT_VERSION` increment introduced by D4.

### D7: Observer Section 3b (Decision History) extension

`crates/worldwake-cli/src/bin/observer.rs:833` is the existing **Section 3b — Decision History** that already renders `GoalCommitted` events with `motive_score`. Extend the per-commit rendering to surface motive-source contributions:

```
Tick 412 — Agent A — GoalCommitted: Eat (motive 18420)
  motive sources:
    NeedPressure(Hunger) → 14200 (need_weight=750, pressure=950)
    Greed(market_opportunity#42) → 4220 (greed_weight=500, opportunity_score=420)
```

The `→` and `(weight=…, …)` formatting mirrors existing Section 3a (Opportunities) rendering conventions in observer.rs. No new Section 4 is introduced; Section 4 (Anomaly Flags) and Section 5+ remain unchanged.

### D8: Conformance and golden coverage

- **Conformance test `every_goal_offer_has_motive_sources()`** at `crates/worldwake-ai/tests/conformance_motive_sources.rs` (new file, matching the precedent of `tests/planner_conformance.rs` and `tests/conformance_execution_budget.rs`). Spawns a representative scenario, runs the planner, and asserts every `GoalOffer` constructed during the run carries a non-empty `motive_sources` vector.
- **Conformance test `utility_profile_default_for_motive_class()`** in the same file. Asserts every new `UtilityProfile` field has a `#[serde(default)]` function that returns a non-zero Permille.
- **Golden coverage**: new `crates/worldwake-ai/tests/golden_motive_sources.rs` with five scenarios:
  1. Hunger-only commit → expects `motive_sources: [NeedPressure(Hunger)]` and contribution score == previous-pre-S141 `motive_score` (parity).
  2. Hunger + Greed (market opportunity) commit → expects two motive sources, sum-equals-score, observer renders both.
  3. Pain dominates Hunger under wound profile → expects `Pain(...)` contribution > `NeedPressure(Hunger)` contribution.
  4. `UtilityProfile.greed_weight` variation across two otherwise-identical agents → expects different commit choices for the same opportunity.
  5. Empty `motive_sources` debug-assert in test build → expects panic at offer construction.
- **Score parity regression**: every existing 1440-tick survival golden produces identical `motive_score` values pre/post-S141 for every commit (the score is the same, the provenance is the new layer). This is the strongest regression guard against derivation drift.
- **`ProfileHomogeneity` lint extension** (`crates/worldwake-cli/src/scenario/lints.rs:62`): the lint's per-`UtilityProfile` checker extends to detect cloned values across the 5 new motive-class weight fields, in addition to the existing per-need weights.

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** No new cross-agent path. Motive sources are read references to existing per-agent state. The references propagate through the existing decision-event surface (S110 + S136 always-on payload), now naming the load-bearing motive sources rather than only the score.
2. **Positive-feedback analysis.** No amplification. The score is a deterministic sum over `Vec<MotiveSourceRef>` of bounded natural cardinality (typically 1–3 sources per offer). The `Revenge` and `Shame` weights default to `pm(400)` rather than the balanced `pm(500)` as a soft dampener against feedback (vengeful agent commits more violations → more `Revenge` motive → more violent commits) — the structural dampener is per-agent profile authoring per FND-22, not a numeric cap.
3. **Concrete dampeners.** Not applicable to the motive-source layer itself; the dampeners that limit per-motive growth live in the underlying state (need recovery via eating, wound healing, opportunity expiry, violation TTL via `ViolationMemory` decay in `violation.rs`).
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state**: `MotiveSourceRef`s embedded in the always-on `GoalCommittedPayload` (S136 payload, written to the event log when the goal commits). The carrying `GoalOffer` is a per-tick agenda entry, not authoritative on its own.
   - **Derived read-model**: `motive_score: u32` (existing, body-refactored); per-source contribution scores in `RankedGoalSummary.motive_source_contributions`.

## SystemFn Integration

No new `SystemFn`. Motive-source population happens at the three existing `GoalOffer` construction sites in `candidate_generation.rs`; scoring happens in the existing `motive_score` function called by `compare_ranked_goals` during the existing ranking pass.

## Component Registration

No new ECS components. `UtilityProfile` is the only registered component touched, and only its fields grow — the component itself remains registered at `crates/worldwake-core/src/component_schema.rs` as today. No `AgentDef`/`spawn_agent()` change is required: `UtilityProfile` is already a universal component applied with defaults; new fields acquire their `#[serde(default)]` value when an `AgentDef` doesn't author them.

## Cross-System Interactions

- **AI ↔ Core**: emitters in `worldwake-ai` construct `MotiveSourceRef`s (defined in core) and embed them in `GoalOffer`s (in ai). Ranking reads them from the offer and dispatches scoring against the `RankingContext` aggregator.
- **AI → Sim**: the `GoalCommittedPayload` (in core, used through sim's event-log path) carries `decisive_motive_sources` when the goal commits, written through the existing decision-event emission seam.
- **Sim → CLI**: observer reads decision-event payloads as today; Section 3b extends rendering to surface motive sources.

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

`UtilityProfile` (per-agent) gains 5 new `Permille` fields (`office_duty_weight`, `loyalty_weight`, `greed_weight`, `shame_weight`, `revenge_weight`), one per non-need non-Pain kept motive class. Defaults are documented in D4's table. The `ProfileHomogeneity` lint (S111) extends its `UtilityProfile` comparison to detect cloned values across the new fields.

## Validation and Falsification

See D7 (Observer rendering) and D8 (Conformance, golden coverage, score parity, lint extension). Score parity is the strongest gate: every existing 1440-tick survival golden produces bitwise-identical `motive_score` values pre/post-S141 for every commit. If the body refactor introduces any drift, the parity regression catches it before the new goldens run.

## Deferred Variants (Phase 12 follow-ups)

Five `MotiveSource` variants were proposed in the original draft but require substrate that does not yet exist:

| Deferred variant | Missing substrate | Notes |
|------------------|-------------------|-------|
| `Fear { threat: ThreatBeliefId }` | Threat-belief surface | Today `NoticeTopic::ThreatWarning` exists as an enum variant but there is no per-agent threat-belief ID. A Phase 12 spec introduces the threat-belief substrate; this variant lands with it. |
| `Obligation { contract: ContractId }` | Contract artifact substrate | `obligation.rs` defines `ObligationSatiationProfile` and `ExpectationId` exists in `expectation.rs:9`, but no `ContractId` or contract artifact type exists. Phase 12 introduces contract-as-record (per FND-25 social artifacts). |
| `Debt { debt: DebtId }` | Debt artifact substrate | No debt substrate exists in core today. Phase 12 introduces `DebtId` and the debt artifact. |
| `Habit { habit: HabitId }` | Habit reinforcement substrate (PR-21 rejected in Wave 1 rollup) | The original draft acknowledged this as a stub. Per FND-28 (no dead paths), the variant is dropped, not stubbed. A later Phase 12 spec lands habit reinforcement and the variant. |
| `Curiosity { hypothesis: HypothesisId }` | Per-agent hypothesis ID | `HypothesisKind` exists at `goal.rs:26` but no per-agent hypothesis ID surface. S130's `SurveyMemory` partially overlaps; a Phase 12 spec consolidates the hypothesis-id surface. |

When any of those substrates lands, extending `MotiveSource` is additive: a new variant, a new scoring helper, a new `UtilityProfile` weight field, and updates to the `derive_default_motive_sources` mapping in `motive_source_mapping.rs`.

## Risks

- **Body refactor must produce bitwise-identical motive scores.** The score-parity regression (D8) is the gate. Mitigation: per-variant scoring helpers are direct extractions of the current `motive_score` body's `match` arms; S141MOTSOULED-004 lands the per-`GoalKind`-to-`MotiveSource` mapping that preserves which variant contributes which fragment.
- **Per-`GoalKind` → `MotiveSource` mapping ambiguity.** Some `GoalKind` variants (e.g., enterprise goals, social goals, combat goals) could plausibly emit multiple motive sources. Mitigation: S141MOTSOULED-004 lands the canonical mapping table and a per-emitter override hook for cases where the emitter has richer context (e.g., a `ReportRecordedViolation` emitter knows the exact `ViolationId`). The mapping is part of D2's deliverable scope.
- **`UtilityProfile` save-format growth.** 5 new `Permille` fields. Mitigation: per-field `#[serde(default = "...")]` ensures pre-S141 snapshots deserialize cleanly; `SAVE_FORMAT_VERSION` bumps from 77 → 78.
- **Deferred-variant pressure.** Designers may want `Fear`/`Obligation`/`Debt`/`Habit`/`Curiosity` motives surfaced in goldens before Phase 12 lands the substrates. Mitigation: the deferred-variants table above is the contract. Scenarios that need to express these motives early should author them as `Greed(opportunity)` or similar existing-state proxies until the substrate lands.
