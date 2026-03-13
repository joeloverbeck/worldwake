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
| 004 | `ROUCOMANDJOUPER-004-plan-selection-journey-margin.md` | Controller-level journey switch margin policy | HIGH | Medium | 003 |
| 005 | `ROUCOMANDJOUPER-005-journey-field-advancement.md` | Journey field advancement on arrival and blockage tracking | HIGH | Medium | 002 |
| 006 | `ROUCOMANDJOUPER-006-journey-clearing-conditions.md` | Journey clearing conditions and blocked-intent integration | HIGH | Medium | 001, 002, 004, 005 |
| 007 | `ROUCOMANDJOUPER-007-debug-surface.md` | Observable debug surface for journey state | MEDIUM | Small | 002, 004, 005 |

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
  └── ROUCOMANDJOUPER-006 (Clearing/reprioritization logic assumes shared margin policy boundary)
  └── ROUCOMANDJOUPER-007 (Debug surface should expose controller-level effective margin/source)

ROUCOMANDJOUPER-005 (Field advancement)
  └── ROUCOMANDJOUPER-006 (Blockage counter feeds into clearing)
  └── ROUCOMANDJOUPER-007 (Debug surface reads advancement state)
```

## Recommended Implementation Order

1. ROUCOMANDJOUPER-001 (TravelDispositionProfile) — independent core component
2. ROUCOMANDJOUPER-002 (Journey temporal fields) — independent AI struct change
3. ROUCOMANDJOUPER-003 (Goal switching margin) — depends on 001 + 002
4. ROUCOMANDJOUPER-004 (Controller margin policy) — depends on 003
5. ROUCOMANDJOUPER-005 (Field advancement) — depends on 002
6. ROUCOMANDJOUPER-006 (Clearing conditions) — depends on 001 + 002 + 004 + 005
7. ROUCOMANDJOUPER-007 (Debug surface) — depends on 002 + 004 + 005

Steps 1 and 2 can be done in parallel. Ticket 004 should land before 006 and 007 so controller policy and debug boundaries do not get split across later lifecycle work.

## Crate Impact

| Crate | Tickets | Nature of Change |
|-------|---------|-----------------|
| `worldwake-core` | 001 | New component type + schema registration |
| `worldwake-ai` | 002, 003, 004, 005, 006, 007 | Runtime fields, goal switching, controller policy, lifecycle, debug |
| `worldwake-sim` | 004 | `BeliefView` travel-disposition accessor for controller policy |
| `worldwake-systems` | — | No changes |

## Key Invariants (all tickets)

- Authoritative travel remains adjacent-place and per-leg only
- No continuous multi-edge travel action
- No edge-fraction or abstract progress scalar in authoritative world state
- Journey fields are transient runtime state, never serialized
- No `JourneyCommitment` struct — fields live on `AgentDecisionRuntime`
- No `Vec<EntityId>` or `Vec<TravelEdgeId>` route storage
- Route/destination derived from plan's remaining Travel steps
- No abstract scores — only concrete temporal and threshold fields (Principle 3)
- System decoupling (Principle 12) preserved — no cross-system direct calls
