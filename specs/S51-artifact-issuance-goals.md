# S51: Social Artifact Issuance Goals

## Summary

Add AI goal generation for *creating* social artifacts (bounties, notices), not just consuming them. Currently agents can fulfill bounties and read notices but the planner never generates goals to post them. This spec adds `GoalKind::PostBounty` and `GoalKind::PostNotice` with candidate generation driven by institutional role, economic motivation, and situational awareness.

## Phase

Phase 6: Architectural Substrates II

## Status

Draft

## Crates

- `worldwake-core` (new GoalKind variants)
- `worldwake-ai` (candidate generation, planner ops, goal dispatch declarations)
- `worldwake-systems` (artifact action enrichment if needed)

## Dependencies

- S45 (unified social artifact model) — completed
- S36 (declarative goal registration) — completed

## Design Goals

- Enable agents to autonomously post bounties when they have motive (e.g., institution wants someone eliminated, merchant wants cargo delivered)
- Enable agents to autonomously post notices when they hold information worth broadcasting (e.g., wanted notice for crime suspect, danger warning)
- Use existing `post_bounty` and `post_notice` action infrastructure — no new actions needed
- Candidate generation must be belief-driven, not omniscient

## Non-Goals

- Artifact maintenance (updating, revoking, contesting) — deferred
- Artifact copying or reposting — deferred
- Operational assignments (patrol orders, escort duties) — separate spec
- Debt/contract artifact types — deferred

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P1 (Maximal Emergence) | Bounty posting emerges from agent motivation, not authored triggers |
| P7 (Locality) | Agent must believe the motive conditions locally — no omniscient bounty generation |
| P14 (Belief ≠ Truth) | Posting decisions based on agent beliefs about threats, needs, resources |
| P18 (Records Are World State) | Posted artifacts are inspectable, persistent world entities |
| P20 (Practical Reasoning) | Goals name desired conditions ("threat eliminated", "information broadcast") |
| P25 (Social Artifacts) | Bounties and notices are first-class world artifacts with identity |

## Deliverables

### New GoalKind Variants

```rust
GoalKind::PostBounty {
    target_kind: BountyTargetKind,  // Elimination or Delivery
    motive: BountyMotive,          // Why the agent wants this
}

GoalKind::PostNotice {
    content_kind: NoticeContentKind,  // Warning, Wanted, Information
    motive: NoticeMotive,
}
```

### New Types

```rust
pub enum BountyMotive {
    InstitutionalEnforcement { office: EntityId, accused: EntityId },
    PersonalVendetta { target: EntityId },
    EconomicNeed { commodity: CommodityKind, destination: EntityId },
}

pub enum NoticeMotive {
    WantedSuspect { accused: EntityId, violation_id: ViolationId },
    DangerWarning { place: EntityId, threat_kind: ThreatKind },
    ResourceAvailability { commodity: CommodityKind, place: EntityId },
}
```

### Candidate Generation

New emission functions:
- `emit_bounty_posting_candidates()` — driven by:
  - Office holder with unresolved accusations (institutional enforcement bounty)
  - Agent with enterprise_weight and unsatisfied delivery needs (economic bounty)
  - Agent with high danger_weight and known threat (personal elimination bounty)
- `emit_notice_posting_candidates()` — driven by:
  - Office holder with unresolved crime cases (wanted notice)
  - Agent with recent danger observation and high social_weight (danger warning)

### Goal Dispatch

Register both new variants in `GoalDispatchDeclaration` with:
- Relevant ops: `PlannerOpKind::PostBounty`, `PlannerOpKind::PostNotice` (new ops wrapping existing actions)
- Feasibility: agent has coin for bounty reward reserve, or posting place is known
- Invalidation: target already eliminated, crime already resolved, etc.

### Planner Integration

- New `PlannerOpKind::PostBounty` and `PlannerOpKind::PostNotice`
- Planner semantics: travel to posting place + post action
- Hypothetical transition: artifact entity created, reward reserved

## Cross-System Interactions

- **Justice system** writes accusation records → read by candidate generation for wanted-notice motivation
- **Perception** updates beliefs about threats/crimes → drives danger-warning notice motivation
- **Trade system** creates unfulfilled demand → drives economic delivery-bounty motivation
- **Social artifact system** handles the actual posting via existing action handlers

## Profile-Driven Parameters

New fields on `UtilityProfile`:
```rust
pub bounty_posting_weight: Permille,  // Motivation to post bounties vs handle threats directly
pub notice_posting_weight: Permille,  // Motivation to post notices vs tell individuals
```

## Component Registration

No new components. New GoalKind variants registered in goal dispatch.

## Section H — Causal Hooks

1. **Information path**: Agent learns about threat/crime/demand through existing perception. Posting decision is belief-driven.
2. **Positive feedback**: Bounty posting → bounty fulfillment → reward payment → reduced resources for future bounties. Self-dampening through resource consumption.
3. **Dampeners**: Bounty requires real reward reserve (coin must exist). Notice posting takes time (action duration). Office holder vacancy stops institutional posting.
4. **Stored vs derived**: `BountyMotive` and `NoticeMotive` are transient candidate-generation artifacts. Posted artifacts are stored as `SocialArtifact` entities.
