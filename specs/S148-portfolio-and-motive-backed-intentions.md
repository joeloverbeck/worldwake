# S148: Portfolio Slot Expansion and Motive-Backed Intentions

**Status**: Draft

## Summary

Folds in PR-3 (Portfolio Slot Expansion replacing top-2 candidates) and PR-1 (BDI deliberation shell — motive-backed intentions) from `reports/ai-architecture-improvements.md`.

S112 (Portfolio Planning, archived) added three slots — Survival, Commitment, Economic — and `max_candidates_to_plan = 2` (`crates/worldwake-ai/src/lib.rs:181`, `crates/worldwake-core/src/cognitive_profile.rs:126`) is still the default planning cap. The assessment identifies a real gap: with hundreds of plausible motives in a dense world, three slots collapse safety/care/duty/social/exploration motives into a single Commitment-or-Economic bucket. Agents miss obligations, fail to investigate suspicions, neglect epistemic work, and skip opportunistic local wins.

S148 expands the portfolio to **seven slots** matching the assessment's motive-class taxonomy: `Survival`, `ImmediateSafety`, `InjuryOrCare`, `ObligationDuty`, `EconomicMaintenance`, `SocialEpistemic`, `OpportunisticLocal`. The legacy `Commitment` and `Economic` slots from S112 fold into `ObligationDuty` and `EconomicMaintenance` respectively, so the existing portfolio code path continues to work for the slots that still exist. Slot weighting uses a new universal `PortfolioWeightsProfile` per agent. The plan-attempt cap rises to `max_candidates_to_plan = 6` by default with operating-mode adjustments: emergency mode (escape/safety motives present) drops to 4; idle mode (no critical needs) stays at 6 across diverse slots.

PR-1's BDI extension is folded in by giving `IntentionFrame` (`crates/worldwake-core/src/intention_frame.rs:138`) the missing fields the assessment flags: `motive_refs: Vec<MotiveSourceRef>` (backed by S141), `resume_conditions: Vec<ResumeCondition>`, `abandon_conditions: Vec<AbandonCondition>`, `explicit_claims: Vec<EntityId>` (artifact references — queue tickets, reservations, contracts), and `causal_links: Vec<EventId>` (which events produced this intention). The agenda manager (S115) already handles `Suspended/Pending` lifecycles; S148 adds the *why* and the *what holds it* alongside.

This spec consumes substrate from S141 (motive sources), S146 (per-goal extractor registry and budgets), and S115 (agenda manager) without changing their identity.

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — extends `agent_tick/portfolio.rs` with new `SlotKind` variants and assembly logic; extends agenda manager to honor enriched intention frames.
- `worldwake-core` — extends `IntentionFrame` with the BDI fields; adds `PortfolioWeightsProfile` universal component; adds `OperatingMode` enum.
- `worldwake-sim` — no change.
- `worldwake-systems` — no change.
- `worldwake-cli` — observer renders the per-slot winners and the motive-backed intention details.

## Dependencies

- S112 (Portfolio Planning, archived, hard dep) — provides the slot-based portfolio infrastructure being extended.
- S115 (Agenda Manager, archived, hard dep) — provides the lifecycle (`committed/pending/suspended`) the enriched `IntentionFrame` plugs into.
- S141 (Motive Source Ledger, archived, hard dep) — provides `MotiveSourceRef` for `IntentionFrame.motive_refs`.
- S140 (Multi-Axis Artifact Lifecycle, archived) — `explicit_claims` references existing artifacts; lifecycle-aware reference invalidation.
- S146 (Goal Schema, Phase 12 wave 2, hard dep) — provides `MotiveSourceVariantId` → slot-kind mapping per `GoalSchema.motive_source_hints`.

## Design Goals

1. **Slot taxonomy matches motive taxonomy.** Each `MotiveSource` variant maps to a deterministic `SlotKind` per `GoalSchema.motive_source_hints` (S146). Survival motives → Survival slot; ImmediateSafety motives → ImmediateSafety slot; Loyalty / OfficeDuty / Revenge → ObligationDuty; Greed → EconomicMaintenance; SocialEpistemic motives → SocialEpistemic; opportunity-driven Local motives → OpportunisticLocal.
2. **Operating modes adjust slot enablement, not slot identity.** Emergency mode disables OpportunisticLocal and EconomicMaintenance; idle mode enables all seven.
3. **Plan-attempt cap rises with breadth.** Default `max_candidates_to_plan = 6`. Single-slot fallback (one winner per slot) keeps planning bounded.
4. **Intentions carry their full evidence record.** `IntentionFrame.motive_refs`, `resume_conditions`, `abandon_conditions`, `explicit_claims`, `causal_links` make every commitment traceable.
5. **No omniscient resolution.** Slot assembly reads only the agent's belief view, motive ledger, and known opportunities.
6. **Deterministic.** Slot tie-breaking is by `MotiveSourceRef.introduced_tick` ascending, then `GoalKindDiscriminant` ordinal.

## Non-Goals

- **No automatic motive-record generation.** Motives are introduced through `MotiveSourceRef` (S141); S148 does not change how motives enter the ledger.
- **No new commitment mechanism.** S115's agenda manager remains the lifecycle authority; S148 enriches the carried data only.
- **No method dispatch.** Methods are S147's scope.
- **No real-time slot mutation.** Slot composition is recomputed each tick; no incremental cache.
- **No "ActiveIntention" slot above the seven.** The assessment proposes a special ActiveIntention slot; S148 represents this via `IntentionFrame.adopted_tick` and agenda-manager continuation logic, not as a distinct slot.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Slot assignment derives from concrete `MotiveSource` types; no abstract "priority score" decides slot membership. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | Seven slots × bounded per-slot winners × per-goal planning budgets is exactly resource-bounded reasoning, not script execution. |
| FND-21 (Intentions Are Revisable Commitments) | Enriched `IntentionFrame` carries `assumptions` (S122), `resume_conditions`, and `abandon_conditions` — every commitment is explicitly revisable. |
| FND-22 (Agent Diversity Through Concrete Variation) | `PortfolioWeightsProfile` per-agent variation produces different slot priorities. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Slot assembly reads existing belief and motive state; no cross-system command. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | S112's `Commitment` and `Economic` slot names fold into `ObligationDuty` and `EconomicMaintenance`; the old enum variants are removed, not aliased. |

## Deliverables

### D1: Expanded `SlotKind`

```rust
// crates/worldwake-ai/src/agent_tick/portfolio.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SlotKind {
    Survival,
    ImmediateSafety,
    InjuryOrCare,
    ObligationDuty,
    EconomicMaintenance,
    SocialEpistemic,
    OpportunisticLocal,
}
```

Migration: S112's `Commitment` → `ObligationDuty`; S112's `Economic` → `EconomicMaintenance`. All references in `portfolio.rs:166-168` and tests are migrated. No alias enum.

### D2: `OperatingMode`

```rust
// crates/worldwake-core/src/operating_mode.rs (new)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OperatingMode {
    Emergency,    // ImmediateSafety motive present at Critical priority
    Normal,       // default
    Idle,         // no goal above Background priority
}
```

Determined per-tick from current motive-source severity. Stored on `AgentSnapshot` (per-tick derivation, not authoritative).

Operating-mode → enabled-slot table:
- `Emergency`: `Survival`, `ImmediateSafety`, `InjuryOrCare` only.
- `Normal`: all seven slots.
- `Idle`: all seven slots (encourages exploration).

### D3: `PortfolioWeightsProfile` (universal)

```rust
// crates/worldwake-core/src/portfolio_weights_profile.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortfolioWeightsProfile {
    pub survival: Permille,
    pub immediate_safety: Permille,
    pub injury_or_care: Permille,
    pub obligation_duty: Permille,
    pub economic_maintenance: Permille,
    pub social_epistemic: Permille,
    pub opportunistic_local: Permille,
    pub max_plans_normal: u8,
    pub max_plans_emergency: u8,
    pub max_plans_idle: u8,
}

impl Default for PortfolioWeightsProfile {
    fn default() -> Self {
        Self {
            survival: Permille::new(1000),
            immediate_safety: Permille::new(950),
            injury_or_care: Permille::new(800),
            obligation_duty: Permille::new(700),
            economic_maintenance: Permille::new(550),
            social_epistemic: Permille::new(400),
            opportunistic_local: Permille::new(300),
            max_plans_normal: 6,
            max_plans_emergency: 4,
            max_plans_idle: 6,
        }
    }
}
```

Universal per FND-22A. Registered on `EntityKind::Agent` with default impl.

### D4: Slot assembly extension

`portfolio.rs::assemble_slots()` extends to:
1. Determine `OperatingMode` from motive severity.
2. For each goal candidate, look up its primary `MotiveSourceRef` and map to `SlotKind` via `GoalSchema.motive_source_hints` (S146).
3. Within each slot, pick one winner via existing ranking (S123's `compare_ranked_goals`).
4. Cap the total to `PortfolioWeightsProfile.max_plans_<mode>`.
5. If fewer than max-plans slots have winners, do not pad — fewer plans is correct.

### D5: `IntentionFrame` extension

```rust
// crates/worldwake-core/src/intention_frame.rs (extended)
pub struct IntentionFrame {
    pub goal: GoalOffer,
    pub domain: GoalDomain,
    pub assumptions: Vec<FrameAssumption>,
    pub state: FrameState,
    pub established_at: Tick,
    pub last_progress_tick: Tick,
    pub stalled_ticks: u32,
    pub patience_limit: u32,
    // New in S148:
    pub motive_refs: Vec<MotiveSourceRef>,
    pub resume_conditions: Vec<ResumeCondition>,
    pub abandon_conditions: Vec<AbandonCondition>,
    pub explicit_claims: Vec<EntityId>,
    pub causal_links: Vec<EventId>,
}

pub enum ResumeCondition {
    BeliefUpdated(BeliefPredicate),
    OpportunityVisible(OpportunityKey),
    LocationReached(EntityId),
    TickElapsed(u32),
    ArtifactValid(EntityId),
}

pub enum AbandonCondition {
    MotiveSourceLost(MotiveSourceVariantId),
    AssumptionPermanentlyBroken(FrameAssumption),
    OpportunityForeverGone(OpportunityKey),
    PatienceExhausted,
}
```

When the agenda manager evaluates a suspended intention, it walks `resume_conditions` (resume if any holds) and `abandon_conditions` (abandon if any holds).

### D6: Explicit claims tracking

`explicit_claims: Vec<EntityId>` references existing world artifacts the intention depends on:
- `ContentionGrant` entities (resource-extraction queue grants).
- `ArtifactBoundary` entities (bounty notices the agent intends to fulfill).
- `OfferRecord` entities (sales offers, contract bids).

The agenda manager invalidates an intention when an `explicit_claim` is in S140 `ArtifactExistence::Removed` or `ArtifactLegalEffect::Suspended` lifecycle states. This makes intent-to-claim revocation lawful per FND-21.

### D7: Causal links

`causal_links: Vec<EventId>` records the events that produced this intention: the perception event that surfaced the motive, the belief-update event, the prior committed-goal completion event that triggered the next chain. Surfaced in decision history (S110/S136) so causal reconstruction across ticks works without ad hoc logging.

### D8: Observer rendering

Observer Section 3b (Decision History) extends `IntentionFrame` rendering:
```
Committed: BakeBread for Granger
  Motive: NeedPressure(Hunger), Greed(SaleOpportunity:bread_lot_42)
  Slot: EconomicMaintenance (weight 550)
  Claims: ContentionGrant#127 (oven queue), OfferRecord#88 (bread sale)
  Resume on: OpportunityVisible(grain_supply_at_market)
  Abandon if: MotiveSourceLost(NeedPressureHunger)
```

`ScenarioDiagnosticsReport.goal_pressure` (S144) gains per-slot occupancy and per-slot winner-vs-rejected counts.

### D9: `max_candidates_to_plan` migration

The default value at `cognitive_profile.rs:126` rises from `2` to `6`. The old `max_candidates_to_plan` field migrates into `PortfolioWeightsProfile.max_plans_normal` for explicit per-agent control. Per `docs/spec-drafting-rules.md` Section 5, the field is universal. `CognitiveProfile.max_candidates_to_plan` is removed (no shim).

### D10: Golden coverage

`golden_portfolio_seven_slots.rs` covers:
- All seven slots populated under normal mode (no operating-mode degradation).
- Emergency mode disables OpportunisticLocal and EconomicMaintenance.
- Survival winner always plans first (priority order).
- `IntentionFrame.motive_refs` matches the committed goal's motive sources.
- `explicit_claims` invalidate on S140 lifecycle transitions.
- `resume_conditions` resume suspended intentions on belief update.
- `abandon_conditions` abandon on motive-source loss.

## FND-01 Section H Analysis

### Information-Path Analysis

Slot assembly reads agent belief view and motive ledger. `explicit_claims` references existing artifact entities visible to the agent (S140 lifecycle gating). `resume_conditions` evaluate against future belief updates that arrive via the existing perception path. No global truth queried.

### Positive-Feedback Analysis

A potential loop: more slots → more committed intentions → more `explicit_claims` → more artifact contention → more frustrated resumes → more replans. The dampener is concrete (D below).

### Concrete Dampeners

- `PortfolioWeightsProfile.max_plans_normal` (default 6) caps per-tick plan attempts.
- `IntentionFrame.patience_limit` (existing per S115) bounds how long a suspended intention can wait before abandonment.
- `AbandonCondition::PatienceExhausted` is the lawful exit path.
- Operating-mode degradation (Emergency drops to 4 plans) is itself a physical dampener — agents under safety pressure plan less broadly.

### Stored State vs. Derived Read-Model List

**Stored state**:
- `PortfolioWeightsProfile` (universal, per-agent).
- Extended `IntentionFrame` fields (`motive_refs`, `resume_conditions`, `abandon_conditions`, `explicit_claims`, `causal_links`) — authoritative per-agent commitment state.

**Derived read-model**:
- `OperatingMode` per-tick derivation.
- Slot composition per-tick derivation.
- Per-slot winner per-tick derivation.

## SystemFn Integration

No new top-level `SystemFn`. Slot assembly and operating-mode derivation run inside the existing agent tick.

## Component Registration

- **New universal component**: `PortfolioWeightsProfile` on `EntityKind::Agent`. Default impl per D3.
- **Extended component**: `IntentionFrame` (already universal; field set extended).

Both per `docs/spec-drafting-rules.md` Section 5.

## Cross-System Interactions

- Reads `MotiveSourceRef` (S141), `GoalSchema.motive_source_hints` (S146), `BeliefView` (S143).
- Writes extended `IntentionFrame` state through agenda manager (S115).
- Surfaced through observer (S110/S136 payload chain) and S144 diagnostics.

State-mediated. No new cross-system calls.

## Profile-Driven Parameters

All slot weights are `Permille`. `max_plans_<mode>` are `u8`. Per FND-22A; per-agent variation.

## Test Plan

- D10 golden coverage (7 scenarios).
- Migration tests proving S112 `Commitment`/`Economic` winners → `ObligationDuty`/`EconomicMaintenance` parity on the existing portfolio golden.
- Unit tests on operating-mode derivation.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
