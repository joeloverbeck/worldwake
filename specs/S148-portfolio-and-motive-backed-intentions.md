# S148: Portfolio Slot Expansion and Motive-Backed Intentions

**Status**: Draft

## Summary

Folds in PR-3 (Portfolio Slot Expansion replacing top-2 candidates) and PR-1 (BDI deliberation shell — motive-backed intentions) from `reports/ai-architecture-improvements.md`.

S112 (Portfolio Planning, archived) introduced the three-slot portfolio — `Survival`, `Commitment`, `Economic` — originally defined as `SlotKind` in `crates/worldwake-ai/src/agent_tick/portfolio.rs`. S148PORMOTBAC-001 relocated `SlotKind` to `crates/worldwake-core/src/slot_kind.rs` and leaves a `worldwake-ai` re-export for existing imports. S148PORMOTBAC-002 lifted slot weighting into `PortfolioWeightsProfile` at `crates/worldwake-core/src/portfolio_weights_profile.rs` and removed the embedded `CognitiveProfile.slot_weights` store. S148PORMOTBAC-008 removed the legacy `CognitiveProfile.max_candidates_to_plan` planning cap and migrated live cap reads to `PortfolioWeightsProfile.max_plans_for_mode(runtime.operating_mode)`. The existing weighted-slot machinery `Portfolio::plausible_slots_by_score()` in `portfolio.rs` already produces a score-ordered slot list, so S148 extends *that* mechanism rather than introducing a parallel one. The assessment identifies a real gap: with hundreds of plausible motives in a dense world, three slots collapse safety/care/duty/social/opportunity motives into a single Commitment-or-Economic bucket. Agents miss obligations, fail to investigate suspicions, neglect epistemic work, and skip opportunistic local wins.

S148 expands the portfolio to **five slots** derived directly from the real `MotiveSourceDiscriminant` taxonomy at `crates/worldwake-core/src/motive_source.rs:25`:

| New SlotKind variant | Source motives (`MotiveSourceDiscriminant`) |
|----------------------|---------------------------------------------|
| `NeedSurvival` | `NeedPressure` |
| `PainCare` | `Pain` |
| `ObligationDuty` | `OfficeDuty`, `Loyalty` |
| `EconomicOpportunity` | `Greed` |
| `SocialMotive` | `Shame`, `Revenge` |

The legacy `Commitment` folds into `ObligationDuty`; the legacy `Economic` folds into `EconomicOpportunity`; the legacy `Survival` becomes `NeedSurvival` (renamed for symmetry). No motive-source taxonomy changes (S141 untouched). Per FND-28 the old enum variants are removed, not aliased.

Slot weighting moves to a new universal component `PortfolioWeightsProfile`. The legacy `PortfolioSlotWeights` struct embedded in `CognitiveProfile` is removed in the same migration so two live authoritative weights stores do not coexist. The plan-attempt cap rises to `max_plans_normal = 5` by default; emergency operating mode drops to 3; idle stays at 5. The legacy `CognitiveProfile.max_candidates_to_plan` and the parallel `ReasoningProfile.max_candidates_to_plan` at `crates/worldwake-ai/src/lib.rs:174` are both removed; their reads migrate to `PortfolioWeightsProfile.max_plans_<mode>`.

PR-1's BDI extension enriches `IntentionFrame` (`crates/worldwake-core/src/intention_frame.rs:138`) with the fields the assessment flags: `motive_refs: Vec<MotiveSourceRef>` (backed by S141 at `motive_source.rs:57`), `resume_conditions: Vec<IntentionResumeCondition>`, `abandon_conditions: Vec<IntentionAbandonCondition>`, `explicit_claims: Vec<EntityId>` (artifact references — queue grants, sale listings, social artifacts), and `causal_links: Vec<EventId>` (which events produced this intention). Lifecycle transitions for `IntentionFrame.state` already run in `crates/worldwake-ai/src/agent_tick/frame.rs` (2381 lines), which consumes `patience_limit` at line 547+; S148 adds the *why* (motives) and *what holds the intention together* (resume/abandon conditions) alongside, and threads the new evaluator into the same module. The S115 agenda manager (`crates/worldwake-ai/src/agenda_manager.rs::tick_agenda`) continues to handle candidate revival.

This spec consumes substrate from archived S141 (motive sources), S146 (per-goal extractor registry and budgets), S115 (agenda manager), and S140 (artifact lifecycle) without changing their identity.

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — extends `agent_tick/portfolio.rs` with the new `SlotKind` variants, the per-tick `OperatingMode` derivation, and the slot-assembly extension that composes with the existing `plausible_slots_by_score`; extends `agent_tick/frame.rs` with the resume/abandon condition evaluator; stores derived `OperatingMode` on `AgentDecisionRuntime` (`decision_runtime.rs:153`).
- `worldwake-core` — adds the new universal `PortfolioWeightsProfile` component; removes `PortfolioSlotWeights` from `CognitiveProfile`; adds `OperatingMode` enum; extends `IntentionFrame` with the five new fields; adds core-residing `IntentionResumeCondition` and `IntentionAbandonCondition` enums; adds the `MotiveSourceSlotMap` mapping table.
- `worldwake-sim` — extends `GoalBeliefView` with a `portfolio_weights_profile` accessor (per the New Component Read by AI Crate pattern; profile components consumed by the AI crate require belief-view forwarding).
- `worldwake-systems` — no change.
- `worldwake-cli` — `scenario/types.rs::AgentDef` gains a `portfolio_weights_profile: Option<PortfolioWeightsProfile>` field; `scenario/mod.rs::spawn_agent()` adds the `set_component_portfolio_weights_profile` call using the canonical `unwrap_or_default()` pattern; `bin/observer.rs` Decision History section renders the per-slot winner, the contributing motive sources, the resume/abandon conditions, and the explicit claims.

## Dependencies

- S112 (Portfolio Planning, archived at `archive/specs/S112-portfolio-planning-three-slots.md`, hard dep) — provides the slot-based portfolio infrastructure (`SlotKind`, `Portfolio::plausible_slots_by_score`, `PortfolioSlotWeights`) being extended.
- S115 (Agenda Manager, archived, hard dep) — `agenda_manager.rs::tick_agenda` handles candidate revival; the resume/abandon evaluator in S148 D10 cooperates with it without replacing it.
- S141 (Motive Source Ledger, archived, hard dep) — provides `MotiveSource`, `MotiveSourceDiscriminant`, `MotiveSourceRef` at `motive_source.rs`. The five-slot taxonomy maps directly onto the existing `MotiveSourceDiscriminant` variants; no motive-source enum changes.
- S140 (Multi-Axis Artifact Lifecycle, archived, hard dep) — `explicit_claims` references existing artifact entities; `IntentionAbandonCondition::ArtifactDestroyed` and `IntentionAbandonCondition::ArtifactLegalEffectLost` consume the lifecycle states declared at `social_artifact.rs:86-108`.
- S146 (Goal Schema, archived at `archive/specs/S146-goal-schema-and-per-goal-budgets.md`, hard dep) — provides the `GoalSchema` registry substrate; this spec attaches the `MotiveSourceSlotMap` table alongside.
- S122 (Frame Assumptions) — already provides `FrameAssumption` at `intention_frame.rs:62`, referenced by `IntentionAbandonCondition::AssumptionPermanentlyBroken`.
- S123 (Goal Ranking) — `compare_ranked_goals` at `ranking.rs:3067` is reused unchanged.
- S143 (BeliefView) — extended with `portfolio_weights_profile` accessor on `GoalBeliefView`.
- S144 (Scenario Diagnostics) — `GoalPressureMetrics.candidates_emitted_by_slot: BTreeMap<SlotKind, u64>` at `scenario_diagnostics/mod.rs:27` already keys by `SlotKind`; the rekeyed five-variant set propagates automatically.

## Design Goals

1. **Slot taxonomy mirrors the real motive taxonomy.** Each `MotiveSourceDiscriminant` variant maps to a deterministic `SlotKind` through the new `MotiveSourceSlotMap` (D4). The mapping is total: every present variant of `MotiveSourceDiscriminant` has a slot. Aggregations (OfficeDuty + Loyalty → ObligationDuty; Shame + Revenge → SocialMotive) are explicit table entries, not implicit collapses.
2. **Operating modes adjust slot enablement through weights, not slot identity.** Emergency mode sets `EconomicOpportunity` and `SocialMotive` weights to `Permille::ZERO` for the tick; the slots remain in the taxonomy and continue to receive emitted candidates. Idle and Normal modes use the agent's configured weights unchanged.
3. **Plan-attempt cap rises with breadth.** `PortfolioWeightsProfile.max_plans_normal = 5` by default; `max_plans_emergency = 3`; `max_plans_idle = 5`. Single-slot fallback (one winner per slot via `plausible_slots_by_score`) keeps planning bounded.
4. **Intentions carry their full evidence record.** `IntentionFrame.motive_refs`, `resume_conditions`, `abandon_conditions`, `explicit_claims`, `causal_links` make every commitment traceable.
5. **No omniscient resolution.** Slot assembly reads only the agent's belief view (`GoalBeliefView` accessor surface), the per-agent motive ledger via `AgendaEntry.motive_source_contributions`, and known opportunities surfaced through candidate generation.
6. **Deterministic.** Slot tie-breaking is by `MotiveSourceRef.introduced_tick` ascending, then `MotiveSourceDiscriminant` ordinal. The "primary motive" for a candidate carrying multiple `motive_source_contributions` is the highest-weight contribution; tie-break by `introduced_tick` ascending.
7. **Single authoritative weights store (FND-28).** `PortfolioSlotWeights` is removed from `CognitiveProfile`; agent weights live exclusively on `PortfolioWeightsProfile`.

## Non-Goals

- **No automatic motive-record generation.** Motives are introduced through `MotiveSourceRef` (S141); S148 does not change how motives enter the ledger.
- **No new commitment mechanism.** S115's agenda manager remains the candidate-revival authority; `frame.rs` remains the `FrameState` lifecycle authority; S148 enriches the carried data and adds the resume/abandon evaluator inside the existing `frame.rs` module.
- **No method dispatch.** Methods are S147's scope.
- **No new motive-source variants.** S141's `MotiveSource` enum is unchanged; S148 maps the existing seven discriminants onto five slots.
- **No new artifact types.** `IntentionFrame.explicit_claims` references existing entities (`ContentionGrant`-bearing facility queues, `SaleListing`-bearing lots, `ArtifactHeader`-bearing social artifacts). S148 does not introduce `ArtifactBoundary` or `OfferRecord` types.
- **No real-time slot mutation.** Slot composition is recomputed each tick; no incremental cache.
- **No "ActiveIntention" slot above the five.** The assessment proposes a special ActiveIntention slot; S148 represents this via `IntentionFrame.established_at` and `frame.rs`'s existing continuation logic, not as a distinct slot.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Slot assignment derives from concrete `MotiveSourceDiscriminant` variants via `MotiveSourceSlotMap`; no abstract "priority score" decides slot membership. |
| FND-7 (Locality) | Slot assembly reads `AgendaEntry.motive_source_contributions` and `PortfolioWeightsProfile` through the per-agent belief view; no global state queries. |
| FND-14 / 14A (World State ≠ Belief State) | `PortfolioWeightsProfile` is accessed via a new `GoalBeliefView` accessor (per the New Component Read by AI Crate pattern); no direct world reads. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | Five slots × bounded per-slot winners × per-goal planning budgets (S146) × OperatingMode-modulated weights is resource-bounded reasoning, not script execution. |
| FND-21 (Intentions Are Revisable Commitments) | Enriched `IntentionFrame` carries `assumptions` (S122), `resume_conditions`, `abandon_conditions`, and `explicit_claims` — every commitment is explicitly revisable via the D10 evaluator. |
| FND-22 (Agent Diversity Through Concrete Variation) | `PortfolioWeightsProfile` is a static per-agent character axis — the canonical universal-component pattern (alongside `MetabolismProfile`, `PerceptionProfile`, `RiskWeightProfile`, etc.). Per-agent variation produces different slot priorities. (FND-22A is *not* cited: weights are static character, not learning state.) |
| FND-26 (Systems Interact Through State) | Slot assembly reads existing belief, motive, and weights state; no cross-system command. The D10 evaluator emits a `Discrepancy::AbandonConditionFired` variant rather than directly invoking other systems. |
| FND-28 (No Backward Compatibility) | S112's `Commitment`/`Economic` variants are removed (renamed in place); `PortfolioSlotWeights` is removed from `CognitiveProfile`; `CognitiveProfile.max_candidates_to_plan` is removed; `ReasoningProfile.max_candidates_to_plan` is removed. No aliases, no shims. |
| FND-29A (Causal History) | `IntentionFrame.causal_links: Vec<EventId>` is bounded by `CognitiveProfile.causal_links_per_step_cap` (already at `cognitive_profile.rs:125`); evictions are FIFO. |
| FND-30 (Causal Hooks Declaration) | See Section H below — 18-point coverage. |

## Deliverables

### D1: Five-variant `SlotKind`

```rust
// crates/worldwake-core/src/slot_kind.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SlotKind {
    NeedSurvival,
    PainCare,
    ObligationDuty,
    EconomicOpportunity,
    SocialMotive,
}
```

**Migration**: S112's `Survival` → `NeedSurvival`; `Commitment` → `ObligationDuty`; `Economic` → `EconomicOpportunity`. New variants: `PainCare`, `SocialMotive`. Per FND-28 the old variant names are removed, not aliased. Every `SlotKind::` match site and constructor across the workspace migrates in lockstep (sites identified at `decision_trace.rs:3937,3945,3961-3972`; `observer.rs:7555-7556`; `planning.rs:4571,4597-4598,4752,4761`; `scenario_diagnostics/mod.rs:212`; `portfolio.rs:46,57,67,167-169,241,334,364,401,441,455,486,500,514-515,547,557,569,585-586` and tests). The legacy `GoalPressureMetrics.candidates_emitted_by_slot: BTreeMap<SlotKind, u64>` rekeys automatically since the field is keyed by the rekeyed enum.

**Implementation note (S148PORMOTBAC-001, 2026-05-17)**: `SlotKind` has been relocated to core with these five variants, and the legacy variant references in source/test code have been renamed. `crates/worldwake-ai/src/agent_tick/portfolio.rs` now re-exports the core enum for existing AI imports. `PainCare` and `SocialMotive` remain dormant until the five-slot assembly ticket makes them emit.

### D2: `PortfolioWeightsProfile` (universal component, lifted from `CognitiveProfile`)

Define the new component in `crates/worldwake-core/src/portfolio_weights_profile.rs`:

```rust
use crate::{Component, Permille};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PortfolioWeightsProfile {
    pub need_survival: Permille,
    pub pain_care: Permille,
    pub obligation_duty: Permille,
    pub economic_opportunity: Permille,
    pub social_motive: Permille,
    pub max_plans_normal: u8,
    pub max_plans_emergency: u8,
    pub max_plans_idle: u8,
}

impl Default for PortfolioWeightsProfile {
    fn default() -> Self {
        Self {
            need_survival: Permille::new_unchecked(1000),
            pain_care: Permille::new_unchecked(900),
            obligation_duty: Permille::new_unchecked(800),
            economic_opportunity: Permille::new_unchecked(600),
            social_motive: Permille::new_unchecked(400),
            max_plans_normal: 5,
            max_plans_emergency: 3,
            max_plans_idle: 5,
        }
    }
}

impl Component for PortfolioWeightsProfile {}
```

**Migration (lift + removal)**:
- Remove `PortfolioSlotWeights` from `cognitive_profile.rs:5-19` and the embedded `slot_weights: PortfolioSlotWeights` field at `cognitive_profile.rs:120`. Update `CognitiveProfile`'s `Default` impl (`cognitive_profile.rs:129-171`) and serde round-trip tests accordingly.
- Update the single runtime consumer at `crates/worldwake-ai/src/agent_tick/planning.rs:610` from `cognitive.slot_weights` to `belief_view.portfolio_weights_profile(agent)` (the new `GoalBeliefView` accessor — see D2 component-read footnote).
- Update test fixtures that construct `PortfolioSlotWeights::default()` (~10 sites including `decision_runtime.rs:469`, `failure_handling.rs:1984`, `goal_model.rs:2668`, `search/tests.rs:93`, `agent_tick/planning.rs:2975`, `agent_tick/tests.rs:212`, `delta.rs:640`, `cognitive_profile.rs:340,540`) to construct `PortfolioWeightsProfile::default()` at the appropriate location.
- Re-export from `core/lib.rs` (replace `pub use cognitive_profile::{CognitiveProfile, PortfolioSlotWeights};` with `pub use cognitive_profile::CognitiveProfile;` and `pub use portfolio_weights_profile::PortfolioWeightsProfile;`).

**Implementation note (S148PORMOTBAC-002, 2026-05-17; updated by S148PORMOTBAC-008, 2026-05-18)**: `PortfolioWeightsProfile` is now a core universal `EntityKind::Agent` component, spawned by default for agents and scenario-authorable through `AgentDef.portfolio_weights_profile`. `GoalBeliefView::portfolio_weights_profile` forwards through `ProfileBeliefView`, and `agent_tick::planning` reads slot weights through that belief-view accessor before calling `Portfolio::plausible_slots_by_score`. `PortfolioSlotWeights`, `CognitiveProfile.slot_weights`, and `CognitiveProfile.max_candidates_to_plan` have been removed from source; planning caps now come from `PortfolioWeightsProfile.max_plans_for_mode(runtime.operating_mode)`.

**Pattern: New Component on EntityKind::Agent** — registration is mandatory:

1. `crates/worldwake-core/src/component_schema.rs`: add `PortfolioWeightsProfile` registration with `|kind| kind == EntityKind::Agent` predicate and the canonical insert/get accessors (`set_component_portfolio_weights_profile`, `get_component_portfolio_weights_profile`).
2. `crates/worldwake-cli/src/scenario/types.rs::AgentDef`: add `pub portfolio_weights_profile: Option<PortfolioWeightsProfile>` with `#[serde(default)]`. No `*Def` wrapper required — the struct contains no `EntityId` references.
3. `crates/worldwake-cli/src/scenario/mod.rs::spawn_agent()`: add `txn.set_component_portfolio_weights_profile(agent_id, agent_def.portfolio_weights_profile.unwrap_or_default())?;` following the canonical universal pattern at `mod.rs:607-678` (matches `metabolism_profile.unwrap_or_default()`).
4. Classification: **(a) universal** — every agent needs portfolio weights to function. `Default` impl required (provided above).

**Pattern: New Component Read by AI Crate** — accessor surface:

5. `crates/worldwake-sim/src/belief_view.rs`: add `fn portfolio_weights_profile(&self, agent: EntityId) -> PortfolioWeightsProfile` on the `GoalBeliefView` trait with a default impl returning `PortfolioWeightsProfile::default()`.
6. `RuntimeBeliefView` impl: backing implementation reads via `World::get_component_portfolio_weights_profile(agent).copied().unwrap_or_default()` — `expect()`-style read on known agents per the universal-profile contract.
7. Belief-view forwarding: explicit delegation following the precedent of `metabolism_profile` (`belief_view.rs:517,2073-2078`).

### D3: `OperatingMode` (per-tick derived, on `AgentDecisionRuntime`)

```rust
// crates/worldwake-core/src/operating_mode.rs (new)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum OperatingMode {
    Emergency, // Pain or NeedPressure motives present with critical urgency
    Normal,    // default
    Idle,      // no motive present above Background priority
}
```

**Derivation site**: `crates/worldwake-ai/src/agent_tick/portfolio.rs::derive_operating_mode(belief: &impl GoalBeliefView, agent: EntityId, ranked: &OrderedRanked) -> OperatingMode`. Reads:
- `Emergency` if any candidate in `ranked` carries a `MotiveSourceRef` whose discriminant is `Pain` or `NeedPressure` AND whose `priority_class` (from `compare_ranked_goals`) is `Critical`.
- `Idle` if all candidates in `ranked` have `priority_class <= Background`.
- `Normal` otherwise.

**Storage**: per-tick derivation, cached on `crates/worldwake-ai/src/decision_runtime.rs::AgentDecisionRuntime` (`decision_runtime.rs:153`) as `pub operating_mode: OperatingMode` field. The cache is refreshed each tick before `assemble_portfolio()` runs and consumed by D5. Storage on `AgentDecisionRuntime` follows the existing "per-agent per-tick runtime state" precedent (`AgendaState`, `current_plan`); not stored as an ECS component because it is derived state per FND-27.

**No `AgentSnapshot` reference** — the spec's earlier draft mistakenly named a non-existent struct; the only `AgentSnapshot` in the codebase is a test profiler at `crates/worldwake-ai/tests/soak_profiler.rs:37`.

**Implementation note (S148PORMOTBAC-003, 2026-05-18)**: `OperatingMode` now exists in core with `Emergency`, `Normal`, and `Idle` variants and defaults to `Normal`. `AgentDecisionRuntime` carries `operating_mode: OperatingMode` with serde defaulting, and `agent_tick/portfolio.rs` provides `derive_operating_mode`. The helper classifies Critical Pain or NeedPressure motive contributions as `Emergency`, all-Background ranked inputs as `Idle`, and all other above-Background ranked inputs as `Normal`. The per-tick call-site wiring and slot-weight consumption remain owned by `S148PORMOTBAC-004`.

### D4: `MotiveSourceSlotMap` (motive-source-to-slot mapping)

```rust
// crates/worldwake-core/src/motive_source_slot_map.rs (new)
use crate::{MotiveSourceDiscriminant, SlotKind};

pub fn slot_for(discriminant: MotiveSourceDiscriminant) -> SlotKind {
    match discriminant {
        MotiveSourceDiscriminant::NeedPressure => SlotKind::NeedSurvival,
        MotiveSourceDiscriminant::Pain         => SlotKind::PainCare,
        MotiveSourceDiscriminant::OfficeDuty   => SlotKind::ObligationDuty,
        MotiveSourceDiscriminant::Loyalty      => SlotKind::ObligationDuty,
        MotiveSourceDiscriminant::Greed        => SlotKind::EconomicOpportunity,
        MotiveSourceDiscriminant::Shame        => SlotKind::SocialMotive,
        MotiveSourceDiscriminant::Revenge      => SlotKind::SocialMotive,
    }
}
```

**Note**: `SlotKind` is referenced from `worldwake-ai`; if the mapping lives in core, it imports the `SlotKind` enum which would also need to relocate to core. Two routings:
- **(a) Recommended**: relocate `SlotKind` from `worldwake-ai/src/agent_tick/portfolio.rs` to `worldwake-core/src/slot_kind.rs`. Re-export from both `core/lib.rs` and `worldwake-ai/src/agent_tick/portfolio.rs` (`pub use worldwake_core::SlotKind;`). All existing `SlotKind` use sites continue to compile unchanged.
- **(b) Alternative**: keep `SlotKind` in `worldwake-ai`, put `slot_for` in `worldwake-ai/src/agent_tick/portfolio.rs` instead. Loses the option of a future core-side belief surface keyed by `SlotKind`.

Adopt (a) so the `MotiveSourceSlotMap` is a pure core-side table colocated with the motive-source taxonomy it indexes; this also lets `GoalPressureMetrics.candidates_emitted_by_slot: BTreeMap<SlotKind, u64>` (already core-resident via `scenario_diagnostics/mod.rs:27`) continue to import `SlotKind` from a single source of truth.

The mapping is **total** over `MotiveSourceDiscriminant`: exhaustive match, no default arm. If S141 ever adds a new motive variant, this match's missing-arm error forces the S148 mapping to be updated alongside.

### D5: Slot assembly extension

Extend `crates/worldwake-ai/src/agent_tick/portfolio.rs::assemble_portfolio` to support the five-slot taxonomy and the operating-mode-modulated weights:

```rust
// portfolio.rs (extended; pseudocode of the runtime flow)
pub(crate) fn assemble_portfolio(
    ranked: &OrderedRanked<'_>,
    committed: Option<OpportunityKey>,
    weights: &PortfolioWeightsProfile,
    mode: OperatingMode,
    probe: impl Fn(&AgendaEntry) -> FeasibilityVerdict,
) -> Portfolio {
    let effective_weights = apply_mode(weights, mode); // mode-degraded weights
    let mut portfolio = Portfolio::default();

    for slot in [
        SlotKind::NeedSurvival,
        SlotKind::PainCare,
        SlotKind::ObligationDuty,
        SlotKind::EconomicOpportunity,
        SlotKind::SocialMotive,
    ] {
        if effective_weights.weight_for(slot).is_zero() {
            continue; // operating-mode disabled this slot
        }
        if let Some(winner) = select_best_candidate_for_slot(
            ranked, slot, committed, &probe,
            |entry| primary_motive_slot(entry) == slot,
        ) {
            portfolio.insert(slot, winner);
        }
    }
    portfolio
}

fn primary_motive_slot(entry: &AgendaEntry) -> SlotKind {
    // entry.motive_source_contributions: Vec<(MotiveSourceRef, u32)> (agenda_types.rs:34)
    // Pick highest-weight contribution; tie-break by introduced_tick ascending.
    let primary = entry.motive_source_contributions.iter()
        .max_by(|(a, w_a), (b, w_b)| w_a.cmp(w_b)
            .then_with(|| b.introduced_tick.cmp(&a.introduced_tick))) // newer loses tie
        .map(|(motive_ref, _)| motive_ref);
    primary
        .map(|m| motive_source_slot_map::slot_for(MotiveSourceDiscriminant::from(&m.source)))
        .unwrap_or(SlotKind::EconomicOpportunity) // fallback for motive-less candidates
}

fn apply_mode(weights: &PortfolioWeightsProfile, mode: OperatingMode) -> PortfolioWeightsProfile {
    match mode {
        OperatingMode::Emergency => PortfolioWeightsProfile {
            economic_opportunity: Permille::ZERO,
            social_motive: Permille::ZERO,
            ..*weights
        },
        OperatingMode::Normal | OperatingMode::Idle => *weights,
    }
}
```

The existing `Portfolio::plausible_slots_by_score(&effective_weights)` at `portfolio.rs:80` continues to operate on the assembled portfolio; the only change at that call site (`planning.rs:4571`) is the source of the weights (now `PortfolioWeightsProfile` instead of `cognitive.slot_weights`).

**Implementation note (S148PORMOTBAC-004, 2026-05-18; updated by S148PORMOTBAC-008, 2026-05-18)**: `assemble_portfolio` now consumes `&PortfolioWeightsProfile` and `OperatingMode`, applies Emergency-mode suppression by zeroing `EconomicOpportunity` and `SocialMotive`, and selects each slot's candidate from the highest-weight motive-source contribution with older `introduced_tick` winning ties. The main planning paths now derive and cache `AgentDecisionRuntime.operating_mode` before assembly. Plan-attempt caps now read from `PortfolioWeightsProfile.max_plans_for_mode(runtime.operating_mode)`.

### D6: `IntentionFrame` extension

```rust
// crates/worldwake-core/src/intention_frame.rs (extended)
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentionFrame {
    pub goal: GoalKey,                            // unchanged
    pub domain: IntentionDomain,                  // unchanged
    pub assumptions: Vec<FrameAssumption>,        // unchanged (S122)
    pub state: FrameState,                        // unchanged
    pub established_at: Tick,                     // unchanged
    pub last_progress_tick: Option<Tick>,         // unchanged (Option<Tick>, not Tick)
    pub stalled_ticks: u32,                       // unchanged
    pub patience_limit: u32,                      // unchanged
    // New in S148:
    pub motive_refs: Vec<MotiveSourceRef>,
    pub resume_conditions: Vec<IntentionResumeCondition>,
    pub abandon_conditions: Vec<IntentionAbandonCondition>,
    pub explicit_claims: Vec<EntityId>,
    pub causal_links: Vec<EventId>,
}
```

Field types `goal: GoalKey` (`core/src/goal.rs:314`) and `domain: IntentionDomain` (`intention_frame.rs:17`) are preserved unchanged — the earlier draft's `GoalOffer`/`GoalDomain` names were drift. The five new fields are appended with `#[serde(default)]` so existing serialized state continues to deserialize (per the spec-drafting-rules.md 5c requirement; `IntentionFrame` is part of save/replay state via `AgentBeliefStore`-adjacent serialization).

**Implementation note (S148PORMOTBAC-006, 2026-05-18)**: `IntentionFrame` now carries the five appended `#[serde(default)]` vectors in `crates/worldwake-core/src/intention_frame.rs`: `motive_refs`, `resume_conditions`, `abandon_conditions`, `explicit_claims`, and `causal_links`. Existing construction sites across core, AI, systems, and golden fixtures initialize those fields explicitly, and focused serde tests prove both non-empty round-trip behavior and pre-S148 deserialization to empty vectors. A scenario RON audit found no authored `IntentionFrame` references. Evaluator semantics and causal-link push-site enforcement were completed by S148PORMOTBAC-007; real motive-ref population remains a follow-on surface.

### D7: `IntentionResumeCondition` and `IntentionAbandonCondition` (core-resident)

```rust
// crates/worldwake-core/src/intention_condition.rs (new)
use crate::{BeliefStatusTag, EntityId, FrameAssumption,
           MotiveSourceDiscriminant, OpportunityAnchor};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum IntentionResumeCondition {
    /// Belief about an entity transitioned to a specific status (e.g., to `Active`).
    BeliefStatusChanged { subject: EntityId, target_status: BeliefStatusTag },
    /// A specific opportunity became visible to the agent again.
    OpportunityVisible(OpportunityAnchor),
    /// Agent reached a specific place (e.g., resume on arrival).
    LocationReached(EntityId),
    /// Wall-clock-equivalent: resume after this many ticks have elapsed since suspension.
    TickElapsed(u32),
    /// Artifact legal effect transitioned to `Active`.
    ArtifactLegalEffectActive(EntityId),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum IntentionAbandonCondition {
    /// The motive that produced this intention is no longer present in the ledger.
    MotiveSourceLost(MotiveSourceDiscriminant),
    /// A frame assumption has been broken in a way that cannot recover.
    AssumptionPermanentlyBroken(FrameAssumption),
    /// The opportunity this intention targeted is gone (consumed by another agent, expired, destroyed).
    OpportunityForeverGone(OpportunityAnchor),
    /// `stalled_ticks` reached `patience_limit` in `frame.rs` (existing mechanism).
    PatienceExhausted,
    /// An explicit-claim artifact transitioned to `ArtifactExistence::Destroyed`.
    ArtifactDestroyed(EntityId),
    /// An explicit-claim artifact's legal effect transitioned out of `Active`
    /// (to `Suspended`, `Expired`, `Revoked`, or `Fulfilled`).
    ArtifactLegalEffectLost(EntityId),
}
```

**Core-residency rationale (Q3=(a))**: `BeliefPredicate` at `worldwake-ai/src/htn/method_schema.rs:72` is HTN-domain-specific (`BountyRecordExists`, `WitnessNamesKnown`, etc.) and lives above the core boundary. Importing it into `IntentionFrame` would violate the `core → sim → systems → ai` dependency graph and conflate two distinct concerns (HTN method preconditions vs. generic intention lifecycle predicates). Defining the new condition enums in core gives `IntentionFrame` self-contained predicates that compose with the existing core-resident `BeliefStatusTag` (`decision_event_payload.rs:281`) and `OpportunityAnchor` (`goal.rs:324`).

**`ArtifactLegalEffectTag` prerequisite**: Add a payload-free discriminant mirror `ArtifactLegalEffectTag` to `worldwake-core/src/social_artifact.rs` following the `BeliefStatusTag` precedent (`decision_event_payload.rs:281`, mechanical mirror of `BeliefStatus`). Variants: `None, Active, Suspended, Expired, Revoked, Fulfilled`. Derives: `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`. Single conversion site `ArtifactLegalEffect → ArtifactLegalEffectTag` lives alongside the existing axis-projection helpers in `social_artifact.rs`. This is the standard Core-Side Mirror pattern from `worldwake-validation-patterns.md`.

**Implementation note (S148PORMOTBAC-005, 2026-05-18)**: `IntentionResumeCondition` and `IntentionAbandonCondition` now live in `crates/worldwake-core/src/intention_condition.rs` and are re-exported from `worldwake-core`. `ArtifactLegalEffectTag` now mirrors `ArtifactLegalEffect` in `social_artifact.rs` with a single `From<&ArtifactLegalEffect>` conversion. `IntentionFrame` field integration was completed by S148PORMOTBAC-006, and evaluator/discrepancy integration was completed by S148PORMOTBAC-007.

### D8: `explicit_claims` tracking (with correct artifact references)

`IntentionFrame.explicit_claims: Vec<EntityId>` references existing world artifacts the intention depends on, scoped to entity kinds the agent might claim against:

- **Facility queue grants**: entities carrying `ContentionGrant` via the `ContentionQueue.granted` field at `crates/worldwake-core/src/contention.rs:43`.
- **Sale listings**: lots carrying `SaleListing` at `crates/worldwake-core/src/trade.rs:25` (the canonical "sales offer" representation; the earlier draft's `OfferRecord` does not exist).
- **Social artifacts**: entities carrying `ArtifactHeader` (bounty notices, contracts) at `crates/worldwake-core/src/social_artifact.rs` (the earlier draft's `ArtifactBoundary` does not exist; `ArtifactHeader` is the actual carrier).
- **Resource-extraction grants**: entities carrying `ContentionGrant` from `production_actions.rs:544,1664` (resource-source grant slots, distinct substrate from facility queues — see Multi-Substrate note in D10).

The resume/abandon evaluator (D10) invalidates an intention when an `explicit_claim` enters `ArtifactExistence::Destroyed` (`social_artifact.rs:86`) or when its `ArtifactLegalEffect` transitions out of `Active` (to `Suspended`, `Expired`, `Revoked`, or `Fulfilled`). Each transition fires `IntentionAbandonCondition::ArtifactDestroyed` or `IntentionAbandonCondition::ArtifactLegalEffectLost` per D7's typed predicates. For non-social-artifact claims (queue grants, sale listings), the evaluator falls back to `IntentionAbandonCondition::OpportunityForeverGone` keyed by the underlying `OpportunityAnchor` when the carrier entity is `is_dead()`-equivalent or the listing has been removed.

### D9: Causal-link cap

`IntentionFrame.causal_links: Vec<EventId>` records the events that produced this intention: the perception event that surfaced the motive, the belief-update event, the prior committed-goal completion event that triggered the next chain. Reuse the existing `CognitiveProfile.causal_links_per_step_cap` (already at `cognitive_profile.rs:125`) as the per-intention cap on `causal_links.len()`. When the cap is reached, evictions are FIFO (drop oldest). This dampens the otherwise unbounded growth flagged by FND-29A.

Surfaced in decision history (S110/S136) so causal reconstruction across ticks works without ad hoc logging.

### D10: Resume/Abandon condition evaluator (host: `agent_tick/frame.rs`)

Add `crates/worldwake-ai/src/agent_tick/frame.rs::evaluate_resume_abandon_conditions(frame: &mut IntentionFrame, belief: &impl GoalBeliefView, agent: EntityId, tick: Tick) -> Option<FrameDecision>`:

```rust
pub(crate) enum FrameDecision {
    Resume,                                 // a resume_condition fired; transition Suspended → Active
    Abandon(IntentionAbandonConditionDiscriminant), // abandon condition fired; mark Exhausted, emit Discrepancy
}
```

**Call site**: invoked from inside `frame.rs` alongside the existing `patience_limit` consumption at line 547+. The existing `FrameState::Suspended → Active` resume path (`frame.rs:519-523`) gains a pre-check against `resume_conditions`; the existing `Exhausted` transition path gains a pre-check against `abandon_conditions`.

**Discrepancy emission**: when an `IntentionAbandonCondition` fires, the evaluator emits a `Discrepancy::AbandonConditionFired(IntentionAbandonConditionDiscriminant)` variant — a new variant added to `worldwake-core/src/discrepancy.rs`. Per the Discrepancy as Failure-Attribution Surface pattern, this is a first-class typed variant while preserving `Discrepancy`'s existing `Copy` derive. `IntentionAbandonCondition` is `Clone + Eq + Ord` but not `Copy` because it carries payloads like `OpportunityAnchor`; the payload-free discriminant mirrors the condition enum one-for-one. The full condition remains recoverable from `frame.abandon_conditions` if needed.

**Multi-substrate hook note**: This evaluator runs in `frame.rs`. It does *not* duplicate the responsibilities of `agenda_manager.rs::tick_agenda` (which revives previously-rejected candidates by re-checking their feasibility). Per the Multi-Substrate Hook Coverage pattern: explicit-claim invalidation that originates from queue grants (`facility_queue.rs`) and from resource-extraction queues (`production_actions.rs`) are both relevant substrates; the evaluator queries `belief.facility_grant(facility)` and the equivalent resource-source-queue belief accessor and treats absence of the grant the same way for both.

**Implementation note (S148PORMOTBAC-007, 2026-05-18)**: `IntentionAbandonConditionDiscriminant` now mirrors every abandon-condition variant in `crates/worldwake-core/src/intention_condition.rs`, and `Discrepancy::AbandonConditionFired` carries that discriminant in `crates/worldwake-core/src/discrepancy.rs`. `crates/worldwake-ai/src/agent_tick/frame.rs` now owns `evaluate_resume_abandon_conditions`, typed predicate helpers, bounded FIFO `causal_links` push, and typed patience-abandon discrepancy emission; `agent_tick/mod.rs` invokes the evaluator during the per-tick frame lifecycle. The live evaluator uses existing `RuntimeBeliefView` surfaces and believed entity artifact snapshots rather than adding new facility/resource grant accessors. Observer rendering remains owned by S148PORMOTBAC-009 and golden E2E coverage remains owned by S148PORMOTBAC-010.

### D11: Observer rendering

`crates/worldwake-cli/src/bin/observer.rs` Decision History section extends per-tick rendering of `IntentionFrame`:

```
Committed: BakeBread for Granger (Slot: EconomicOpportunity, weight 600)
  Motives:
    - Greed(SaleOpportunity:bread_lot_42) introduced t=412
    - NeedPressure(Hunger) introduced t=420
  Claims:
    - ContentionGrant#127 (oven queue)
    - SaleListing on bread_lot_42
  Resume on: OpportunityVisible(grain_supply_at_market)
  Abandon if:
    - MotiveSourceLost(NeedPressure)
    - ArtifactLegalEffectLost(bread_lot_42)
```

Format conventions follow the existing `format_motive_source_ref` at `observer.rs:1194` and the existing "Committed:" header at the Decision History markdown table (currently at `observer.rs:933-941`). `ScenarioDiagnosticsReport.goal_pressure.candidates_emitted_by_slot` at `scenario_diagnostics/mod.rs:27` already keys by `SlotKind` and automatically reflects the renamed five-variant set; no schema change to `GoalPressureMetrics` is required.

**Implementation note (S148PORMOTBAC-009, 2026-05-18)**: The observer Decision History section now appends committed-intention detail rows for `GoalCommitted` events. Slot and weight render from the committed/current-frame motive source mapped through `motive_source_slot_for` and the agent's `PortfolioWeightsProfile`. When the committed goal still matches the agent's current `IntentionFrame`, the observer also renders populated `motive_refs`, `explicit_claims`, `resume_conditions`, `abandon_conditions`, and `causal_links`, skipping empty vectors. The live decision event stream does not carry a historical full-frame snapshot, so full frame sub-bullets are intentionally limited to the matching current frame rather than adding an engine-side trace payload in this observer-only ticket. `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` was updated for the new slot rows.

### D12: Plan-attempt cap migration (`max_candidates_to_plan` removal)

The legacy planning cap `CognitiveProfile.max_candidates_to_plan: u8` (`cognitive_profile.rs:25`, default 2) is removed. The parallel `ReasoningProfile.max_candidates_to_plan` (`crates/worldwake-ai/src/lib.rs:174`) is also removed in the same migration so two cap stores do not coexist (FND-28). The single replacement is `PortfolioWeightsProfile.max_plans_<mode>` selected per-tick by D3's `OperatingMode`.

Reader migration (15+ sites identified at validation time):
- `crates/worldwake-ai/src/agent_tick/planning.rs:660, 2469, 6391`: replace `usize::from(cognitive.max_candidates_to_plan)` with `usize::from(weights.max_plans_for_mode(runtime.operating_mode))`.
- `decision_runtime.rs:426, 469`: remove the `max_candidates_to_plan` assignment from `ReasoningProfile`; remove the field altogether.
- `failure_handling.rs:1941, 1984`, `goal_model.rs:2625, 2668`, `search/tests.rs:53, 93`, `agent_tick/planning.rs:2932, 2975`, `agent_tick/tests.rs:172, 212`: update fixture constructors to drop the field.
- `scenario/types.rs:1753, 1779`, `handlers/persistence.rs:185`, `handlers/inspect.rs:300`: update CLI surfaces to reference `PortfolioWeightsProfile.max_plans_*` instead.

**Golden audit**: existing portfolio and planning golden tests that depend on the cap of 2 (notably `golden_portfolio_planning.rs` and the planning tests at `planning.rs:4501`+) are reviewed in D14 and either rewritten for the new default of 5 or pinned to a per-test `max_plans_normal` value via the `PortfolioWeightsProfile` fixture.

**Implementation note (S148PORMOTBAC-008, 2026-05-18)**: `CognitiveProfile.max_candidates_to_plan` has been removed from core source, CLI scenario fixtures, AI fixtures, and committed scenario RON. The live codebase did not contain a production `ReasoningProfile`; the same-seam stale relay was a test-only `ProfileFixture.max_candidates_to_plan`, which was removed. `PortfolioWeightsProfile::max_plans_for_mode(OperatingMode)` is the single planning-cap accessor, and `agent_tick::planning` uses it for both primary candidate caps and same-goal planning trace caps. CLI inspect output now renders the portfolio max-plan fields instead of the removed cognitive cap.

### D13: Authoritative-to-AI Impact Analysis

Per CLAUDE.md's Authoritative-to-AI Impact Rule, S148 modifies candidate emission (slot assembly determines what gets planned) and adds new abandon-path control flow. All seven checkpoints must hold:

1. `get_affordances` — **N/A** (no affordance change).
2. `generate_candidates` — **flag**: the five-slot taxonomy plus operating-mode-modulated weights changes which candidates win their slot. D14 includes a golden specifically asserting that under each operating mode, candidates emitted at low weight are not silently dropped (they still emit; their winning slot may differ).
3. `search_plan` — **pass** (no precondition change).
4. `BestEffort` action start — **N/A** (no precondition change).
5. `handle_plan_failure` — **flag**: when an `IntentionAbandonCondition` fires inside the D10 evaluator, the produced `Discrepancy::AbandonConditionFired(_)` routes through the existing `handle_plan_failure` path (`agent_tick.rs`). D14 includes a golden asserting that a fired `MotiveSourceLost` condition causes replan with the abandoned intention's motive correctly removed from the contributing set.
6. Payload revalidation — **N/A** (no payload change).
7. Golden tests — **flag**: see D14.

### D14: Golden coverage

Add `crates/worldwake-ai/tests/golden_portfolio_five_slots.rs` covering:
- All five slots populated under `OperatingMode::Normal` (each slot receives a winner derived from the corresponding motive class).
- `OperatingMode::Emergency` weights `EconomicOpportunity` and `SocialMotive` to `Permille::ZERO`; the assertion confirms those slots are skipped while `NeedSurvival`, `PainCare`, and `ObligationDuty` continue to populate.
- `OperatingMode::Idle` populates all five slots when low-priority candidates exist.
- `NeedSurvival` winner is planned first under priority-class ordering.
- `IntentionFrame.motive_refs` matches the committed goal's `motive_source_contributions`.
- `explicit_claims` invalidate on `ArtifactExistence::Destroyed` and on each non-`Active` `ArtifactLegalEffect` transition (Suspended, Expired, Revoked, Fulfilled).
- `resume_conditions` resume a suspended intention on `OpportunityVisible`, `LocationReached`, and `BeliefStatusChanged`.
- `abandon_conditions` cause `Exhausted` transition on `MotiveSourceLost`, `OpportunityForeverGone`, `PatienceExhausted`, `ArtifactDestroyed`, and `ArtifactLegalEffectLost`.
- `causal_links` cap enforcement: when 1+ events beyond `causal_links_per_step_cap` are pushed, the oldest is evicted.

Audit and migrate the existing portfolio goldens at `crates/worldwake-ai/tests/golden_portfolio_planning.rs` (currently 6 tests on the three-slot model) — each test either (a) updates its slot assertions to the new five-slot variants while preserving its scenario, or (b) pins its `PortfolioWeightsProfile` to a fixture isolating the asserted slot.

## FND-01 Section H Analysis (18-point coverage)

### Information-Path Analysis

Slot assembly reads:
- `AgendaEntry.motive_source_contributions` via `OrderedRanked<'_>` produced by `ranking.rs::sort_in_place` (`ranking.rs:327`) — agent-local information.
- `PortfolioWeightsProfile` via the new `GoalBeliefView::portfolio_weights_profile` accessor — universal-profile read through the belief layer.
- `OperatingMode` via `AgentDecisionRuntime.operating_mode` set earlier in the same tick by `derive_operating_mode`.

The D10 evaluator reads the agent's belief view for `is_alive`/`facility_grant`/`artifact_header` checks; transitions are observed through the existing perception channel that updates the agent's belief store. No global state queries.

`IntentionFrame.causal_links: Vec<EventId>` and `motive_refs: Vec<MotiveSourceRef>` are local to the intention; observers (other agents) cannot read another agent's intentions and so do not need an information path for them.

### Positive-Feedback Analysis

Potential loops:
- *Wider portfolio → more committed intentions → more `explicit_claims` → more contention → more abandon firings → more replans.* Dampened by `max_plans_<mode>` and by `IntentionFrame.patience_limit` (existing).
- *More motives recorded → more candidates per tick → larger `OrderedRanked` → more work for `assemble_portfolio`.* Dampened by S141's existing motive-ledger caps and by `max_node_expansions` in the planner.
- *AbandonConditionFired → replan → new abandon condition fires next tick.* Dampened by per-tick FIFO emission semantics: a given `Discrepancy` for the same goal is emitted once per tick, and the per-goal cooldowns in `CognitiveProfile` (`*_backoff_ticks` family at `cognitive_profile.rs:80-119`) gate replan attempts.

### Concrete Dampeners

- `PortfolioWeightsProfile.max_plans_normal` (default 5) caps per-tick plan attempts.
- `PortfolioWeightsProfile.max_plans_emergency` (default 3) caps per-tick plan attempts under emergency mode.
- `IntentionFrame.patience_limit` (existing per S122) bounds suspended-intention wait before abandonment.
- `IntentionAbandonCondition::PatienceExhausted` is the lawful exit path on patience runout.
- `OperatingMode::Emergency` degradation is itself a physical dampener — agents under safety pressure plan less broadly.
- `CognitiveProfile.causal_links_per_step_cap` (existing at `cognitive_profile.rs:125`) caps `IntentionFrame.causal_links.len()`.
- Per-goal `*_backoff_ticks` cooldowns (existing) gate replan after AbandonConditionFired emission.

### Stored State vs. Derived Read-Model List

**Stored state (authoritative, ECS components and per-agent runtime):**
- `PortfolioWeightsProfile` — universal ECS component per agent (D2).
- Extended `IntentionFrame` fields — authoritative per-agent commitment state.

**Stored state (core types referenced by D7/D10):**
- `Discrepancy::AbandonConditionFired(IntentionAbandonConditionDiscriminant)` — typed entry in the discrepancy stream.

**Derived read-model (per-tick, non-authoritative):**
- `AgentDecisionRuntime.operating_mode` — recomputed each tick by `derive_operating_mode`.
- Portfolio composition and per-slot winners — recomputed each tick by `assemble_portfolio`.
- Per-slot weights after operating-mode degradation (`apply_mode`'s output) — recomputed each tick.
- `MotiveSourceSlotMap::slot_for(...)` — pure function over `MotiveSourceDiscriminant`; no state.

### Causal Hooks Declaration (P30, 18 items)

1. **Missing downstream consequence motivating the system**: Three-slot portfolio collapses safety/care/duty/social/opportunistic motives into a single Commitment-or-Economic bucket; agents miss obligations, fail to investigate suspicions, neglect epistemic work, skip opportunistic local wins. Existing systems cannot produce this because the slot taxonomy is the only discriminator between concurrent goals at planning time.
2. **Concrete entities, relations, records introduced**: `PortfolioWeightsProfile` component on Agent; `OperatingMode` enum (runtime, on `AgentDecisionRuntime`); `MotiveSourceSlotMap` mapping (pure table); `IntentionResumeCondition`/`IntentionAbandonCondition` enums; `Discrepancy::AbandonConditionFired` variant; five new `IntentionFrame` fields; `ArtifactLegalEffectTag` discriminant mirror.
3. **Actions/processes that mutate them**: D5's `assemble_portfolio` extension writes derived portfolio composition; D10's `evaluate_resume_abandon_conditions` writes `FrameState` transitions and emits `Discrepancy::AbandonConditionFired`; scenario `spawn_agent` writes the initial `PortfolioWeightsProfile` (universal default or scenario-authored).
4. **Information produced, travel, observability**: Per-intention details (motive_refs, conditions, claims, causal_links) travel through decision-history events into the observer's Decision History section (D11); not directly observable by other agents (private to the agent's runtime). `AbandonConditionFired` discrepancies surface through the existing discrepancy-stream observability.
5. **Quantities conserved/transferred**: None — the portfolio is a derived view; the underlying candidates already exist in `OrderedRanked`. No new quantities introduced.
6. **Scarce capacities/exclusive affordances/reservations/queues introduced; contention rules**: None — `PortfolioWeightsProfile` per-agent; no shared resource introduced.
7. **Partial failures, degraded states, aftermath**: `IntentionAbandonCondition::PatienceExhausted` produces `FrameState::Exhausted` with `Discrepancy::AbandonConditionFired` aftermath. `OperatingMode::Emergency` degradation is a "partial" state where two slots are zeroed.
8. **Positive feedback loops amplified**: see Positive-Feedback Analysis above.
9. **Physical dampeners limiting those loops**: see Concrete Dampeners above.
10. **Agent-local/institutional learning, memory, habit, trust updates**: None — `PortfolioWeightsProfile` is static character (FND-22), not learning state (FND-22A). If a future spec wants weights to adapt, it must add an explicit learning mechanism.
11. **How agents can become wrong / correct errors / provenance/freshness markers**: `IntentionFrame.causal_links: Vec<EventId>` provides per-intention provenance back to the events that produced the intention. `motive_refs` carry `MotiveSourceRef.introduced_tick` for freshness. If the agent's belief view is stale, the D10 evaluator may resume/abandon based on stale data; the existing belief-staleness machinery (S143) is the correction path.
12. **Lifecycle states for new entities/artifacts**: `IntentionFrame.state` already enumerates `Active`/`Suspended`/`Exhausted` (existing per S122). S148 adds explicit transition causes via `IntentionResumeCondition`/`IntentionAbandonCondition`. Visibility unchanged (private to the agent); legality unchanged.
13. **Temporal/spatial resolution, scheduling, simultaneity, tie-breaking**: Each agent's portfolio is recomputed once per tick before planning. Within a tick, the operating-mode derivation runs first, then the slot assembly, then the planning gate. Tie-breaks: primary motive selection is highest-weight contribution, then `introduced_tick` ascending; cross-slot ordering uses `compare_ranked_goals`'s composite ordering. Determinism preserved via `BTreeMap`-ordered iteration of the five `SlotKind` variants in `assemble_portfolio`.
14. **Boundary conditions, external drivers**: None — internal AI-runtime mechanism.
15. **Derived views/caches/optimizations and their source state**: `OperatingMode` is derived per-tick from motive severity (source: `AgendaEntry` contributions); portfolio composition is derived per-tick from `OrderedRanked` + `PortfolioWeightsProfile` + `OperatingMode`. No persisted cache.
16. **Causal records/event identities/provenance links emitted**: `Discrepancy::AbandonConditionFired` (typed) is emitted when an abandon condition fires; the existing decision-history payload chain (S110/S136) carries portfolio composition into per-tick records.
17. **Target patterns, invariants, regression cases, falsification checks**: D14 lists invariants and goldens. Key invariants: motive-source-to-slot mapping is total over `MotiveSourceDiscriminant`; emergency mode zeros exactly `EconomicOpportunity` and `SocialMotive` and nothing else; `causal_links.len() <= causal_links_per_step_cap` at all times; no `IntentionFrame` field carries data that requires global reads to populate.
18. **Save/load/replay/offscreen compression survival**: All new authoritative state derives `Serialize, Deserialize` (component + IntentionFrame fields + condition enums). `OperatingMode` is *not* persisted — it is derived per-tick and so saved/loaded as zero state. The `Discrepancy` stream is already part of save state.

## SystemFn Integration

No new top-level `SystemFn` is added. The work integrates into existing per-tick code:

- `derive_operating_mode` runs inside `agent_tick/portfolio.rs::assemble_portfolio` callers (specifically the planning entry at `agent_tick/planning.rs`) before slot assembly.
- `assemble_portfolio` is the existing entry point; D5 extends its signature and body.
- `evaluate_resume_abandon_conditions` runs inside `agent_tick/frame.rs` alongside the existing `FrameState` transition logic.

## Component Registration

- **New universal component**: `PortfolioWeightsProfile` on `EntityKind::Agent`. Default impl per D2. Registered in `component_schema.rs` with insert/get accessors; threaded through `AgentDef` + `spawn_agent()` per the canonical universal pattern.
- **Extended component**: `IntentionFrame` (already universal; field set extended with `#[serde(default)]` on new fields).
- **Removed component fields**: `CognitiveProfile.slot_weights`, `CognitiveProfile.max_candidates_to_plan`, `ReasoningProfile.max_candidates_to_plan` — single-truth migration per FND-28.

Per `docs/spec-drafting-rules.md` Section 5.

## Cross-System Interactions

- **Reads** (state-mediated):
  - `MotiveSourceRef`, `MotiveSourceDiscriminant` from S141 (core).
  - `PortfolioWeightsProfile` via `GoalBeliefView::portfolio_weights_profile` (sim layer; consumed by ai).
  - `AgendaEntry.motive_source_contributions` from S123 (ai).
  - `ArtifactHeader.existence`/`legal_effect` from S140 (core, read by ai through belief view).
  - `ContentionGrant` from S112's queue substrate (core, read by ai through `belief.facility_grant`).
- **Writes** (state-mediated):
  - Extended `IntentionFrame` state through `agent_tick/frame.rs`.
  - `Discrepancy::AbandonConditionFired` into the existing discrepancy stream.
  - `AgentDecisionRuntime.operating_mode` per-tick.
- **Surfaced** through observer (S110/S136 payload chain) and S144 `GoalPressureMetrics`.

All interactions are state-mediated per FND-26. No new cross-system direct calls.

## Profile-Driven Parameters

- `PortfolioWeightsProfile.{need_survival, pain_care, obligation_duty, economic_opportunity, social_motive}: Permille` — per-agent character weights.
- `PortfolioWeightsProfile.{max_plans_normal, max_plans_emergency, max_plans_idle}: u8` — per-agent planning caps.

All weights are `Permille` per the spec-drafting-rules.md requirement; per-agent variation per FND-22.

## Test Plan

- D14 golden coverage (5-slot scenarios; resume/abandon condition evaluator scenarios; operating-mode degradation scenario; causal_links cap scenario).
- Migration tests proving S112 `Survival/Commitment/Economic` winners → `NeedSurvival/ObligationDuty/EconomicOpportunity` parity on the existing portfolio golden (`golden_portfolio_planning.rs`).
- Unit tests on `derive_operating_mode` decision logic.
- Unit tests on `motive_source_slot_map::slot_for` confirming totality over `MotiveSourceDiscriminant`.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
