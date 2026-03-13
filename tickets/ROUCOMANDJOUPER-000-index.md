# ROUCOMANDJOUPER-000: Route Commitment and Journey Persistence Index

**Status**: ACTIVE
**Spec**: `specs/DRAFT-route-commitment-and-journey-persistence.md`
**Phase**: Future AI/runtime hardening, post-E22 scheduling only

## Overview

Adds explicit journey tracking to the AI runtime layer so agents can persist toward a destination across intermediate arrivals without losing per-leg authoritative travel. Route persistence is profile-driven per agent via `TravelDispositionProfile`. Journey state is transient runtime, not serialized.

## Ticket List

| # | Filename | Title | Priority | Effort | Deps |
|---|----------|-------|----------|--------|------|
| 001 | `ROUCOMANDJOUPER-001-travel-disposition-profile.md` | TravelDispositionProfile component | HIGH | Small | None |
| 002 | `archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-002-journey-temporal-fields.md` | Journey temporal fields on AgentDecisionRuntime | HIGH | Small | None |
| 003 | `ROUCOMANDJOUPER-003-journey-aware-goal-switching.md` | Journey-aware goal switching margin override | HIGH | Small | 001, 002 |
| 004 | `archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-004-plan-selection-journey-margin.md` | Controller-level journey switch margin policy | HIGH | Medium | 003 |
| 005 | `archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-005-journey-field-advancement.md` | Journey field advancement on arrival and blockage tracking | HIGH | Medium | 002, 004 |
| 006 | `ROUCOMANDJOUPER-006-journey-clearing-conditions.md` | Journey clearing conditions and blocked-intent integration | HIGH | Medium | 001, 002, 004, 005, 008, 009 |
| 007 | `ROUCOMANDJOUPER-007-debug-surface.md` | Observable debug surface for journey state | MEDIUM | Small | 002, 004, 005, 008, 009 |
| 008 | `archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-008-explicit-journey-commitment-anchor.md` | Explicit journey commitment anchor on AgentDecisionRuntime | HIGH | Medium | 002, 004, 005 |
| 009 | `ROUCOMANDJOUPER-009-journey-preserving-detour-policy.md` | Journey-preserving detour and abandonment policy | HIGH | Large | 004, 005, 008 |

## Dependencies

```
ROUCOMANDJOUPER-001 (TravelDispositionProfile)
  └── ROUCOMANDJOUPER-003 (Goal switching margin override)
  └── ROUCOMANDJOUPER-006 (Clearing conditions use blocked_leg_patience_ticks)

ROUCOMANDJOUPER-002 (Journey temporal fields)
  └── ROUCOMANDJOUPER-003 (Needs journey_established_at for "active journey" check)
  └── ROUCOMANDJOUPER-005 (Needs fields to advance/reset)
  └── ROUCOMANDJOUPER-006 (Needs fields to clear)
  └── ROUCOMANDJOUPER-007 (Needs fields to expose)

ROUCOMANDJOUPER-003 (Goal switching)
  └── ROUCOMANDJOUPER-004 (Controller computes one effective margin for both selection and interrupts)

ROUCOMANDJOUPER-004 (Controller margin policy)
  └── ROUCOMANDJOUPER-008 (Needs a durable runtime commitment anchor to outlive individual plans)
  └── ROUCOMANDJOUPER-009 (Relation-based detour vs abandonment policy builds on the controller margin seam)
  └── ROUCOMANDJOUPER-006 (Clearing/reprioritization logic assumes shared margin policy boundary)
  └── ROUCOMANDJOUPER-007 (Debug surface should expose controller-level effective margin/source)

ROUCOMANDJOUPER-005 (Field advancement)
  └── ROUCOMANDJOUPER-008 (Blocked-step replanning now preserves commitment longer than a single plan instance)
  └── ROUCOMANDJOUPER-009 (Detour policy builds on recoverable journey advancement/blockage tracking)
  └── ROUCOMANDJOUPER-006 (Blockage counter feeds into clearing)
  └── ROUCOMANDJOUPER-007 (Debug surface reads advancement state)

ROUCOMANDJOUPER-008 (Explicit commitment anchor)
  └── ROUCOMANDJOUPER-009 (Detour policy needs durable committed goal/destination state)
  └── ROUCOMANDJOUPER-006 (Clearing semantics must act on commitment, not just the current plan)
  └── ROUCOMANDJOUPER-007 (Debug surface must expose commitment state and destination)

ROUCOMANDJOUPER-009 (Detour/abandonment policy)
  └── ROUCOMANDJOUPER-006 (Clearing should distinguish suspension from abandonment)
  └── ROUCOMANDJOUPER-007 (Debug should expose commitment relation and suspend/active state)
```

## Recommended Implementation Order

1. ROUCOMANDJOUPER-001 (TravelDispositionProfile) — independent core component
2. ROUCOMANDJOUPER-002 (Journey temporal fields) — independent AI struct change
3. ROUCOMANDJOUPER-003 (Goal switching margin) — depends on 001 + 002
4. ROUCOMANDJOUPER-004 (Controller margin policy) — depends on 003
5. ROUCOMANDJOUPER-005 (Field advancement) — depends on 002
6. ROUCOMANDJOUPER-008 (Explicit commitment anchor) — separates durable commitment from the current plan
7. ROUCOMANDJOUPER-009 (Detour/abandonment policy) — applies relation-based controller behavior on top of the commitment anchor
8. ROUCOMANDJOUPER-006 (Clearing conditions) — should land after the anchor/policy split so clearing semantics operate on true abandonment
9. ROUCOMANDJOUPER-007 (Debug surface) — should expose the final commitment model, not the interim plan-derived one

Steps 1 and 2 can be done in parallel. Ticket 004 should land before the later controller/runtime lifecycle tickets. Tickets 008 and 009 should land before 006 and 007 so commitment state, detour policy, clearing, and debug all describe the same architecture.

## Crate Impact

| Crate | Tickets | Nature of Change |
|-------|---------|-----------------|
| `worldwake-core` | 001 | New component type + schema registration |
| `worldwake-ai` | 002, 003, 004, 005, 006, 007, 008, 009 | Runtime fields, goal switching, controller policy, lifecycle, debug |
| `worldwake-sim` | 004 | `BeliefView` travel-disposition accessor for controller policy |
| `worldwake-systems` | — | No changes |

## Key Invariants (all tickets)

- Authoritative travel remains adjacent-place and per-leg only
- No continuous multi-edge travel action
- No edge-fraction or abstract progress scalar in authoritative world state
- Journey fields are transient runtime state, never serialized
- No `JourneyCommitment` struct — fields live on `AgentDecisionRuntime`
- No `Vec<EntityId>` or `Vec<TravelEdgeId>` route storage
- Route remains derived from plan travel steps; committed destination may be retained transiently across planless replanning seams after ROUCOMANDJOUPER-008
- No abstract scores — only concrete temporal and threshold fields (Principle 3)
- System decoupling (Principle 12) preserved — no cross-system direct calls
