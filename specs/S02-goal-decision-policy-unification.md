**Status**: PENDING

# Goal Decision Policy Unification

## Summary
Define a single goal-family decision-policy layer for `worldwake-ai` so the AI no longer splits closely related behavior rules across ranking-time suppression and interrupt-time special cases.

Today, corpse opportunism is the clearest example of the architectural split:
- `ranking.rs` suppresses `LootCorpse` / `BuryCorpse` under high self-care or danger
- `interrupts.rs` separately hardcodes when opportunistic loot may interrupt

That shape is serviceable for Phase 2, but it is not the long-term architecture we want. As more goal families arrive, special-case branches in multiple modules will drift. The correct direction is:
- one shared decision-policy surface per goal family
- one shared current-context derivation
- ranking and interrupt evaluation both consume that same policy output
- no compatibility wrappers preserving both the old and new paths

This is a hardening draft for the archived E13 decision architecture. It does not introduce new world simulation state. It cleans the AI read model so later epics can add new goal families without copying policy across modules.

## Why This Exists
The current E13 implementation has a real architectural seam:

1. Ranking decides whether some goals are suppressed entirely.
2. Interrupt evaluation separately decides whether some goals are allowed to preempt.
3. Some goal-family rules, especially corpse opportunism, now exist in both places.

That creates three risks:

1. **Policy drift**
   - a rule is tightened in ranking but not interrupts
   - or interrupts are broadened while ranking still assumes the old world

2. **Poor extensibility**
   - new goal families such as evidence handling, theft, patrol response, escort detours, or companion overrides will be tempted to add new one-off branches in multiple modules

3. **Weak explainability**
   - the system can answer "this goal ranked low" and "this action interrupted" separately, but not from one coherent policy source

Foundational alignment:
- Principle 18: decisions must be explainable as what this agent, with this belief state and these priorities, would try to do
- Principle 19: intentions are revisable commitments, so interruption policy must be explicit and coherent
- Principle 24: systems interact through state, not through hidden cross-calls
- Principle 26: no backward compatibility; do not preserve parallel policy paths
- Principle 27: debuggability is a product feature

## Spec Ownership Scan
No active spec currently owns this cleanup.

Reviewed and not sufficient:
- [E20-companion-behaviors.md](/home/joeloverbeck/projects/worldwake/specs/E20-companion-behaviors.md)
  - describes behavior-level priority overrides during travel
  - does not define a general AI policy model for suppression vs interruption
- [S06-commodity-opportunity-valuation.md](/home/joeloverbeck/projects/worldwake/specs/S06-commodity-opportunity-valuation.md)
  - centralizes commodity value, not goal admissibility or interrupt posture
- [S04-merchant-selling-market-presence.md](/home/joeloverbeck/projects/worldwake/specs/S04-merchant-selling-market-presence.md)
  - discusses ranking effects but does not unify decision policy across ranking and interrupts
- [S03-planner-target-identity-and-affordance-binding.md](/home/joeloverbeck/projects/worldwake/specs/S03-planner-target-identity-and-affordance-binding.md)
  - centralizes exact target matching, not timing/suppression/preemption rules

This draft is therefore the correct ownership point.

## Phase
Phase 3: Information & Politics, Step 10 (parallel after E14)

## Crates
- `worldwake-ai`

## Dependencies
- E14

## Design Goals
1. Keep one authoritative AI-layer policy surface for goal-family decision behavior.
2. Separate three concepts cleanly:
   - whether a goal family is currently available for consideration
   - how it ranks if available
   - whether it may interrupt a current action
3. Remove goal-family-specific policy drift between `ranking.rs` and `interrupts.rs`.
4. Preserve belief locality: all policy inputs come from belief-facing read models and current AI runtime state, never authoritative world shortcuts.
5. Keep the policy declarative per goal family, not encoded as scattered `if GoalKind::X` branches across modules.
6. Improve debugability so the AI can report why a goal was suppressed, capped, deferred, or denied as an interrupt challenger.
7. Do not add compatibility wrappers or preserve both old and new policy paths.

## Non-Goals
- redesigning planner search
- changing authoritative world state or event schema
- replacing `GoalPriorityClass` or motive scoring with a different model
- moving AI transient policy into authoritative components
- adding new gameplay goal families in this draft
- changing `priority_class()` derivation (already centralized in ranking.rs, not policy drift)
- changing `motive_score()` derivation (already centralized in ranking.rs, not policy drift)
- changing `GoalKindTag` (serves planner ops, a different purpose)

## Deliverables

### 1. Shared `DecisionContext` Read Model
Add one derived AI-layer context object, built once per agent decision pass, that exposes the currently relevant local pressure classes needed by both ranking and interrupts.

```rust
pub struct DecisionContext {
    pub max_self_care_class: GoalPriorityClass,
    pub danger_class: GoalPriorityClass,
}

impl DecisionContext {
    /// True if either self-care or danger is at or above the given class.
    pub fn is_stressed_at_or_above(&self, threshold: GoalPriorityClass) -> bool {
        self.max_self_care_class >= threshold || self.danger_class >= threshold
    }
}
```

The context contains ONLY the shared pressure state that both ranking and interrupts need. Interrupt-specific parameters (`current_goal`, `current_action_interruptibility`, `plan_valid`) are already passed directly to `evaluate_interrupt()` and do NOT belong here.

This avoids one module classifying "high danger / high self-care" one way while another reconstructs it differently.

Note: suppression uses a `>= High` threshold while opportunistic interrupt gating uses `>= Medium`. The `DecisionContext` exposes raw class values so each consumer can compare at its own threshold via `is_stressed_at_or_above()`.

### 2. Goal-Family Decision Policy Declaration

Extend the AI goal-semantics layer with explicit decision policy per goal family. Policy is keyed on `&GoalKind` (not `GoalKindTag`), because some GoalKindTag values have multiple behaviors depending on payload — e.g., `AcquireCommodity` with `CommodityPurpose::SelfConsume` is CriticalSurvival, while `AcquireCommodity` with other purposes is enterprise. `GoalKindTag` cannot discriminate these.

The interrupt posture is two-dimensional, not a single enum. The codebase has two independent interrupt dimensions:

1. **Penalty interrupt eligibility**: Can this goal interrupt `InterruptibleWithPenalty` actions? (Only critical survival + ReduceDanger, and only at Critical priority)
2. **Free interrupt role**: How does this goal behave as a challenger in freely-interruptible context? (Reactive/margin-free, Opportunistic, Normal)

These are orthogonal: `Heal` is reactive but NOT penalty-eligible. `EngageHostile` is neither.

```rust
pub struct GoalFamilyPolicy {
    pub suppression: SuppressionRule,
    pub penalty_interrupt: PenaltyInterruptEligibility,
    pub free_interrupt: FreeInterruptRole,
}

pub enum SuppressionRule {
    /// Never suppressed regardless of stress level.
    Never,
    /// Suppressed when agent stress (max self-care or danger) is at or above this class.
    WhenStressedAtOrAbove(GoalPriorityClass),
}

pub enum PenaltyInterruptEligibility {
    /// Can interrupt InterruptibleWithPenalty actions when this goal reaches Critical priority.
    WhenCritical { trigger: InterruptTrigger },
    /// Cannot interrupt penalty actions.
    Never,
}

pub enum FreeInterruptRole {
    /// Reactive: can interrupt via HigherPriorityGoal (margin-free) and SameClassMargin.
    Reactive,
    /// Opportunistic: can interrupt freely only when no medium+ self-care/danger pressure.
    Opportunistic,
    /// Normal: can only interrupt via SameClassMargin; HigherPriorityGoal alone → NoInterrupt.
    Normal,
}
```

No `priority_cap` field. Current priority is *derived* from needs/danger/enterprise pressure via `priority_class()`, not capped. The derivation logic is already centralized in one function in ranking.rs and is genuinely varied per goal kind (not policy drift). It stays.

### 3. Goal Families Own Their Policy

Policy is declared by goal family via a single lookup function that takes `&GoalKind` and pattern-matches including purpose fields. All 16 goal families are migrated in one pass.

```rust
pub fn goal_family_policy(kind: &GoalKind) -> GoalFamilyPolicy {
    match kind {
        // Self-care: critical survival interrupt, reactive
        GoalKind::ConsumeOwnedCommodity { .. }
        | GoalKind::AcquireCommodity { purpose: CommodityPurpose::SelfConsume, .. }
        | GoalKind::Sleep | GoalKind::Relieve | GoalKind::Wash => GoalFamilyPolicy {
            suppression: SuppressionRule::Never,
            penalty_interrupt: PenaltyInterruptEligibility::WhenCritical {
                trigger: InterruptTrigger::CriticalSurvival,
            },
            free_interrupt: FreeInterruptRole::Reactive,
        },

        // Danger response: critical danger interrupt, reactive
        GoalKind::ReduceDanger => GoalFamilyPolicy {
            suppression: SuppressionRule::Never,
            penalty_interrupt: PenaltyInterruptEligibility::WhenCritical {
                trigger: InterruptTrigger::CriticalDanger,
            },
            free_interrupt: FreeInterruptRole::Reactive,
        },

        // Healing: reactive but NOT penalty-eligible
        GoalKind::Heal { .. } => GoalFamilyPolicy {
            suppression: SuppressionRule::Never,
            penalty_interrupt: PenaltyInterruptEligibility::Never,
            free_interrupt: FreeInterruptRole::Reactive,
        },

        // Combat engagement: NOT reactive, NOT penalty-eligible
        // (is_reactive_goal does NOT include EngageHostile;
        //  interrupt_with_penalty does NOT include EngageHostile)
        GoalKind::EngageHostile { .. } => GoalFamilyPolicy {
            suppression: SuppressionRule::Never,
            penalty_interrupt: PenaltyInterruptEligibility::Never,
            free_interrupt: FreeInterruptRole::Normal,
        },

        // Corpse looting: suppressed in stress, opportunistic interrupt
        GoalKind::LootCorpse { .. } => GoalFamilyPolicy {
            suppression: SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High),
            penalty_interrupt: PenaltyInterruptEligibility::Never,
            free_interrupt: FreeInterruptRole::Opportunistic,
        },

        // Corpse burial, social, political: suppressed in stress, normal interrupt
        // (BuryCorpse gets Normal, NOT Opportunistic — different from LootCorpse)
        GoalKind::BuryCorpse { .. }
        | GoalKind::ShareBelief { .. }
        | GoalKind::ClaimOffice { .. }
        | GoalKind::SupportCandidateForOffice { .. } => GoalFamilyPolicy {
            suppression: SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High),
            penalty_interrupt: PenaltyInterruptEligibility::Never,
            free_interrupt: FreeInterruptRole::Normal,
        },

        // Enterprise, treatment, recipe input, other acquisition: never suppressed, normal
        GoalKind::AcquireCommodity { .. }
        | GoalKind::ProduceCommodity { .. }
        | GoalKind::SellCommodity { .. }
        | GoalKind::RestockCommodity { .. }
        | GoalKind::MoveCargo { .. } => GoalFamilyPolicy {
            suppression: SuppressionRule::Never,
            penalty_interrupt: PenaltyInterruptEligibility::Never,
            free_interrupt: FreeInterruptRole::Normal,
        },
    }
}
```

This is the central rule:
- ranking and interrupts consume declared policy
- they do not own goal-family policy themselves

Future Principle 20 (agent diversity) extension: change signature to `goal_family_policy(&GoalKind, &AgentPolicyProfile)` to support per-agent policy variation.

### 4. Suppression Evaluation

```rust
pub fn evaluate_suppression(
    kind: &GoalKind,
    context: &DecisionContext,
) -> GoalPolicyOutcome {
    let policy = goal_family_policy(kind);
    match policy.suppression {
        SuppressionRule::Never => GoalPolicyOutcome::Available,
        SuppressionRule::WhenStressedAtOrAbove(threshold) => {
            if context.is_stressed_at_or_above(threshold) {
                GoalPolicyOutcome::Suppressed {
                    threshold,
                    max_self_care: context.max_self_care_class,
                    danger: context.danger_class,
                }
            } else {
                GoalPolicyOutcome::Available
            }
        }
    }
}
```

### 5. Ranking Consumes Shared Policy
`rank_candidates()` must:
- evaluate family policy against the shared `DecisionContext`
- drop suppressed candidates through `evaluate_suppression()`

`RankingContext` embeds a `DecisionContext` instead of recomputing pressure independently:
```rust
struct RankingContext<'a> {
    view: &'a dyn GoalBeliefView,
    agent: EntityId,
    current_tick: Tick,
    utility: &'a UtilityProfile,
    needs: Option<HomeostaticNeeds>,
    thresholds: Option<DriveThresholds>,
    danger_pressure: Permille,
    decision_context: DecisionContext,  // NEW: shared context
}
```

`ranking.rs` continues to own generic deterministic ordering, `priority_class()`, and `motive_score()`, but family-specific suppression branches are replaced by shared policy consumption.

### 6. Interrupt Evaluation Consumes Shared Policy
`evaluate_interrupt()` receives `decision_context: &DecisionContext` as a new parameter.

**Penalty interrupt migration**: Replace `is_critical_survival_goal()` + `ReduceDanger` check with policy lookup:
```rust
let policy = goal_family_policy(&challenger.grounded.key.kind);
match policy.penalty_interrupt {
    PenaltyInterruptEligibility::WhenCritical { trigger }
        if challenger.priority_class == GoalPriorityClass::Critical =>
    {
        InterruptDecision::InterruptForReplan { trigger }
    }
    _ => InterruptDecision::NoInterrupt,
}
```

**Free interrupt migration**: Replace hardcoded `LootCorpse` check with policy lookup:
```rust
let policy = goal_family_policy(&challenger.grounded.key.kind);
match policy.free_interrupt {
    FreeInterruptRole::Opportunistic => {
        if !decision_context.is_stressed_at_or_above(GoalPriorityClass::Medium) {
            InterruptDecision::InterruptForReplan {
                trigger: InterruptTrigger::OpportunisticLoot,
            }
        } else {
            InterruptDecision::NoInterrupt
        }
    }
    FreeInterruptRole::Reactive => { /* existing HigherPriorityGoal + SameClassMargin logic */ }
    FreeInterruptRole::Normal => { /* existing SameClassMargin-only logic */ }
}
```

Replace `is_reactive_goal()` check in switch_kind matching with `policy.free_interrupt == Reactive`.

**Semantic note**: The current `no_medium_or_above_self_care_or_danger()` scans *ranked candidates* for medium+ self-care/danger goals. The new approach uses `DecisionContext.is_stressed_at_or_above(Medium)` which checks raw pressure classes directly. These should be equivalent (same pressure → same classification), but golden tests will verify.

This preserves the good part of E13DECARC-015 (interrupt evaluation remains pure and cheap) while removing goal-family interrupt exceptions living outside the policy declaration surface.

### 7. Explainable Decision Diagnostics
Add a runtime-accessible diagnostic surface for policy outcomes.

```rust
pub enum GoalPolicyOutcome {
    Available,
    Suppressed {
        threshold: GoalPriorityClass,
        max_self_care: GoalPriorityClass,
        danger: GoalPriorityClass,
    },
}
```

This data remains transient, but the architecture supports answers such as:
- "loot corpse suppressed because hunger is High"
- "loot corpse not allowed to interrupt because medium-or-above stress exists"
- "bury corpse suppressed because danger is High"

This directly supports Principle 27.

### 8. Migration Requirement
Do not preserve both the old branchy logic and the new policy layer. All 16 goal families are migrated in one pass.

When this lands:
- `is_suppressed()` is removed from ranking.rs
- `is_critical_survival_goal()` is removed from interrupts.rs
- `is_reactive_goal()` is removed from interrupts.rs
- `no_medium_or_above_self_care_or_danger()` is removed from interrupts.rs
- duplicated pressure-threshold logic must not survive in two places

## Component Registration
No new authoritative components.

This draft introduces AI transient read-model and policy types only. They must not be registered in `worldwake-core` component schema.

## SystemFn Integration

### `worldwake-ai`
- add `goal_policy.rs` module with shared decision-context derivation, policy types, lookup function, and suppression evaluation
- update ranking to embed `DecisionContext` in `RankingContext` and consume shared policy
- update interrupt evaluation to accept `&DecisionContext` and consume shared policy
- update `agent_tick.rs` to build `DecisionContext` once per agent tick and thread it to both ranking and interrupts

### `worldwake-sim`
- no new authoritative simulation system is required
- continue supplying `Interruptibility` and belief-facing reads to the AI layer

### `worldwake-core`
- no new authoritative world state is required
- existing `GoalKind`, `GoalKey`, `DriveThresholds`, and related types remain canonical

## Cross-System Interactions (Principle 12)
- world systems write concrete needs, wounds, danger-relevant state, and action state
- belief/runtime surfaces expose those facts locally to the AI
- shared AI decision policy derives availability and interrupt posture from those read models
- ranking and interrupt logic both consume that derived policy

No direct system-to-system calls are introduced. Influence remains state-mediated through:
- homeostatic needs
- thresholds
- active action interruptibility
- current goal/runtime plan state
- local corpse/evidence availability

## FND-01 Section H

### Information-Path Analysis
- suppression and interrupt posture are derived from belief-facing local state already available to the acting agent
- no policy rule may depend on omniscient world access that bypasses `BeliefView` / AI runtime state
- corpse opportunism remains local because corpse candidates already come from local corpse evidence
- danger gating remains local because it is derived from currently believed threats / active attackers, not a hidden global danger manager

### Positive-Feedback Analysis
- a fragmented decision-policy architecture can create design feedback loops where each new goal family adds more local exceptions, making future change harder and drift more likely
- that is an architectural positive-feedback loop: complexity creates more exceptions, which create more complexity

### Concrete Dampeners
- one policy declaration per goal family
- one shared decision-context derivation
- one shared switch-margin comparison path
- explicit migration removing old branches instead of layering new ones beside them
- debug diagnostics that expose policy reasons before more exceptions accrete invisibly

These are concrete architectural dampeners, not numeric clamps.

### Stored vs Derived State
Stored authoritative state:
- homeostatic needs
- drive thresholds
- wounds / danger-relevant world state
- active action / interruptibility in scheduler state
- current AI runtime plan state where already stored in runtime

Derived transient state:
- `DecisionContext`
- goal-family policy outcomes
- suppression reasons

## Invariants
- goal-family decision policy is declared in one AI-layer place (`goal_family_policy()`)
- ranking and interrupts do not carry diverging family-specific suppression/preemption rules
- all policy evaluation remains deterministic
- all policy inputs are belief-facing or AI-runtime-facing, never authoritative-world shortcuts
- no compatibility wrappers preserve both old and new policy paths
- debug diagnostics can explain why a goal was suppressed or denied as an interrupt challenger
- policy is keyed on `&GoalKind`, not `GoalKindTag`, to discriminate purpose-dependent behavior

## Tests
- [ ] `goal_family_policy()` returns correct policy for every `GoalKind` variant
- [ ] `evaluate_suppression()` with various `DecisionContext` stress levels
- [ ] `LootCorpse` suppression uses `High` threshold; opportunistic interrupt gate uses `Medium` threshold
- [ ] `BuryCorpse` suppression uses `High` threshold; free interrupt role is `Normal` (not `Opportunistic`)
- [ ] penalty interrupt eligibility: self-care goals + ReduceDanger → `WhenCritical`; Heal/EngageHostile → `Never`
- [ ] free interrupt role: self-care/ReduceDanger/Heal → `Reactive`; LootCorpse → `Opportunistic`; EngageHostile/enterprise → `Normal`
- [ ] ranking drops suppressed candidates via `evaluate_suppression()`, not local hardcoded branches
- [ ] interrupt evaluation rejects opportunistic loot via shared policy when stress is `>= Medium`
- [ ] normal same-class and higher-class switch-margin comparisons still route through shared switch-policy logic
- [ ] deterministic ranking/interrupt behavior is unchanged for scenarios not covered by special family policy
- [ ] existing golden corpse-opportunism and suppression scenarios continue to pass after migration

## Acceptance Criteria
- the active corpse-opportunism split between `ranking.rs` and `interrupts.rs` is removed
- one shared goal-family policy declaration surface exists in `goal_policy.rs`
- ranking and interrupts both consume that surface
- `is_suppressed()`, `is_critical_survival_goal()`, `is_reactive_goal()`, `no_medium_or_above_self_care_or_danger()` are completely removed
- no new world components or compatibility shims are introduced
- the resulting architecture is easier to extend for future goal families than the current branch-per-module approach
- all 16 goal families are migrated, not just corpse families

## Suggested Implementation Sequence
1. create `goal_policy.rs` module with types, `goal_family_policy()`, `evaluate_suppression()`, `DecisionContext`
2. migrate `RankingContext` to embed `DecisionContext`; replace `is_suppressed()` with `evaluate_suppression()`
3. thread `DecisionContext` into `evaluate_interrupt()`; migrate `interrupt_with_penalty()` to use policy
4. migrate `interrupt_freely()` to use policy; remove `is_reactive_goal()`, `no_medium_or_above_self_care_or_danger()`
5. update `agent_tick.rs` to build `DecisionContext` once and pass to both ranking and interrupts
6. add focused unit coverage for policy module, then verify all existing goldens pass

## Files Modified

| File | Change |
|------|--------|
| `crates/worldwake-ai/src/goal_policy.rs` | **NEW** — policy types, lookup, evaluation, DecisionContext |
| `crates/worldwake-ai/src/lib.rs` | Add `goal_policy` module, re-exports |
| `crates/worldwake-ai/src/ranking.rs` | Embed DecisionContext in RankingContext, replace `is_suppressed()` |
| `crates/worldwake-ai/src/interrupts.rs` | Consume policy for penalty/free interrupt, remove hardcoded functions |
| `crates/worldwake-ai/src/agent_tick.rs` | Build DecisionContext, thread to `evaluate_interrupt()` |

## Relationship to Existing and Future Specs
- supersedes the Phase 2 narrow corpse-opportunism carve-out documented by archived E13 interrupt work
- complements [S06-commodity-opportunity-valuation.md](/home/joeloverbeck/projects/worldwake/specs/S06-commodity-opportunity-valuation.md), which centralizes value calculation but not decision posture
- should be completed before broadening behavior-specific overrides in [E20-companion-behaviors.md](/home/joeloverbeck/projects/worldwake/specs/E20-companion-behaviors.md)
- leaves planner target identity to [S03-planner-target-identity-and-affordance-binding.md](/home/joeloverbeck/projects/worldwake/specs/S03-planner-target-identity-and-affordance-binding.md)

## Open Questions (Resolved)
1. **BuryCorpse interrupt**: BuryCorpse gets `Normal` free_interrupt, not `Opportunistic`. Different from LootCorpse.
2. **Diagnostics**: Runtime-accessible (not test-only).
3. **Non-corpse migration**: All 16 families in one pass.
