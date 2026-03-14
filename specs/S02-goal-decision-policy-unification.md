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
- `worldwake-sim`
- `worldwake-core`

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

## Deliverables

### 1. Shared `DecisionContext` Read Model
Add one derived AI-layer context object, built once per agent decision pass, that exposes the currently relevant local pressure classes and runtime facts needed by both ranking and interrupts.

Suggested shape:

```rust
struct DecisionContext {
    max_self_care_class: GoalPriorityClass,
    danger_class: GoalPriorityClass,
    current_goal: Option<GoalKey>,
    current_action_interruptibility: Option<Interruptibility>,
    plan_valid: bool,
}
```

The exact fields may vary, but the context must remain:
- transient
- belief-facing
- deterministic
- shared by ranking and interrupt evaluation

This avoids one module classifying "high danger / high self-care" one way while another reconstructs it differently.

### 2. Goal-Family Decision Policy Declaration
Extend the AI goal-semantics layer with explicit decision policy per goal family.

Suggested shape:

```rust
struct GoalDecisionPolicy {
    availability: GoalAvailabilityPolicy,
    priority_cap: Option<GoalPriorityClass>,
    interrupt: GoalInterruptPolicy,
}
```

Suggested subtypes:

```rust
enum GoalAvailabilityPolicy {
    AlwaysConsider,
    SuppressWhenSelfCareAtOrAbove(GoalPriorityClass),
    SuppressWhenDangerAtOrAbove(GoalPriorityClass),
    SuppressWhenAny(Vec<GoalSuppressionRule>),
}

enum GoalInterruptPolicy {
    NeverVoluntary,
    CompareNormally,
    OpportunisticOnlyWhen(Vec<GoalInterruptGate>),
}
```

The exact type names may differ, but the policy must express:
- whether a goal family is currently considered at all
- whether it is capped in priority
- whether it is interrupt-eligible, and under what gate

### 3. Goal Families Own Their Policy
Policy must be declared by goal family in the goal-model / semantics layer, not in ranking or interrupts.

Examples:
- self-care goals: always considered, compare normally
- `ReduceDanger`: always considered, compare normally
- `LootCorpse`:
  - suppressed when self-care is `High+`
  - suppressed when danger is `High+`
  - interrupt-eligible only when no medium-or-above self-care/danger challenger exists
- `BuryCorpse`:
  - suppressed when self-care is `High+`
  - suppressed when danger is `High+`
  - no opportunistic interrupt special case unless explicitly justified later

This is the central rule:
- ranking and interrupts consume declared policy
- they do not own goal-family policy themselves

### 4. Ranking Consumes Shared Policy
`rank_candidates()` must:
- evaluate family policy against the shared `DecisionContext`
- drop suppressed candidates through the shared policy layer
- apply any declared priority caps through the same policy layer

`ranking.rs` may still own generic deterministic ordering and motive scoring, but not family-specific suppression branches once this draft lands.

### 5. Interrupt Evaluation Consumes Shared Policy
`interrupts.rs` must:
- stop hardcoding opportunistic `LootCorpse` branches directly
- ask goal-family policy whether a challenger is interrupt-eligible under current context
- continue reusing the shared switch-margin comparison logic for normal same-class / higher-class comparison

This preserves the good part of E13DECARC-015:
- interrupt evaluation remains pure and cheap

But it removes the bad part:
- goal-family interrupt exceptions living outside the policy declaration surface

### 6. Explainable Decision Diagnostics
Add a small AI-layer diagnostic surface for policy outcomes.

Suggested shape:

```rust
enum GoalPolicyOutcome {
    Available,
    Suppressed { reason: GoalSuppressionReason },
    AvailableButInterruptBlocked { reason: GoalInterruptBlockReason },
}
```

This data remains transient and optional, but the architecture should support answers such as:
- "loot corpse suppressed because hunger is High"
- "loot corpse not allowed to interrupt because medium-or-above self-care pressure exists"
- "bury corpse suppressed because danger is High"

This directly supports Principle 27.

### 7. Migration Requirement
Do not preserve both the old branchy logic and the new policy layer.

When this lands:
- ranking-side corpse suppression branches must be removed
- interrupt-side corpse-opportunity special casing must be rewritten to consume goal-family policy
- duplicated pressure-threshold logic must not survive in two places

## Component Registration
No new authoritative components.

This draft introduces AI transient read-model and policy types only. They must not be registered in `worldwake-core` component schema.

## SystemFn Integration

### `worldwake-ai`
- add the shared decision-context derivation helper
- add goal-family decision-policy declarations in the goal semantics layer
- update ranking to consume shared policy outcomes
- update interrupt evaluation to consume shared policy outcomes
- expose optional debug snapshots or helper methods for policy diagnostics if useful

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
- interrupt-block reasons
- capped priority application

## Invariants
- goal-family decision policy is declared in one AI-layer place
- ranking and interrupts do not carry diverging family-specific suppression/preemption rules
- all policy evaluation remains deterministic
- all policy inputs are belief-facing or AI-runtime-facing, never authoritative-world shortcuts
- no compatibility wrappers preserve both old and new policy paths
- debug diagnostics can explain why a goal was suppressed or denied as an interrupt challenger

## Tests
- [ ] goal-family policy declaration exists for every currently emitted Phase 2 goal family
- [ ] `LootCorpse` suppression and interrupt gating are both driven by the same policy declaration
- [ ] `BuryCorpse` suppression is driven by the same policy declaration surface as `LootCorpse`
- [ ] ranking drops suppressed corpse candidates via shared policy, not local hardcoded branches
- [ ] interrupt evaluation rejects opportunistic loot via shared policy when self-care or danger gates fail
- [ ] normal same-class and higher-class switch-margin comparisons still route through shared switch-policy logic
- [ ] deterministic ranking/interrupt behavior is unchanged for scenarios not covered by special family policy
- [ ] debug diagnostics report suppression and interrupt-block reasons deterministically
- [ ] existing golden corpse-opportunism and suppression scenarios continue to pass after migration

## Acceptance Criteria
- the active corpse-opportunism split between `ranking.rs` and `interrupts.rs` is removed
- one shared goal-family policy declaration surface exists
- ranking and interrupts both consume that surface
- no new world components or compatibility shims are introduced
- the resulting architecture is easier to extend for future goal families than the current branch-per-module approach

## Suggested Implementation Sequence
1. introduce shared decision-context derivation
2. introduce goal-family policy declaration types in the goal semantics layer
3. migrate corpse families (`LootCorpse`, `BuryCorpse`) first
4. migrate any other existing family-specific suppression/preemption carve-outs to the same model where lawful
5. remove legacy branches
6. add focused unit coverage, then preserve behavior-level proof through existing goldens

## Relationship to Existing and Future Specs
- supersedes the Phase 2 narrow corpse-opportunism carve-out documented by archived E13 interrupt work
- complements [S06-commodity-opportunity-valuation.md](/home/joeloverbeck/projects/worldwake/specs/S06-commodity-opportunity-valuation.md), which centralizes value calculation but not decision posture
- should be completed before broadening behavior-specific overrides in [E20-companion-behaviors.md](/home/joeloverbeck/projects/worldwake/specs/E20-companion-behaviors.md)
- leaves planner target identity to [S03-planner-target-identity-and-affordance-binding.md](/home/joeloverbeck/projects/worldwake/specs/S03-planner-target-identity-and-affordance-binding.md)

## Open Questions
1. Should `BuryCorpse` remain non-interrupt-eligible even when `LootCorpse` stays opportunistic, or should both corpse interactions eventually share one opportunistic family policy?
2. Should the eventual diagnostic surface be test-only, or exposed through a lightweight runtime snapshot for CLI/debug tooling?
3. Are there any current non-corpse goal families whose suppression rules should migrate in the same first pass, or should the initial implementation stay intentionally narrow to corpse opportunism plus policy infrastructure?
