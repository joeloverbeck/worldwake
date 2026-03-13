# GOLDENE2E-000: Golden E2E Coverage Expansion Index

**Status**: ACTIVE
**Initiative**: Systematically close golden E2E coverage gaps identified in `reports/golden-e2e-coverage-analysis.md`

## Overview

The golden e2e suite currently covers 8/17 GoalKinds, 7/9 ActionDomains, and 4/5 needs. This initiative continues closing the remaining gaps identified in `reports/golden-e2e-coverage-analysis.md`.

On 2026-03-12, living-agent combat was split architecturally from defensive danger mitigation: proactive hostile engagement is now its own goal family, while `ReduceDanger` remains a separate defensive gap to cover.

All tests use the real AI loop (`AgentTickDriver` + `AutonomousControllerRuntime`) with real system dispatch. No manual action queueing. All behavior must be emergent.

## Coverage Targets

| Metric | Current | With Tier 1 | With All |
|--------|---------|-------------|----------|
| GoalKind coverage | 8/17 (47.1%) | 13/17 (76.5%) | 16/17 (94.1%) |
| ActionDomain coverage | 7/9 full | 7/9 full | 9/9 full |
| Needs tested | 4/5 | 4/5 | 5/5 |
| Places used | 7/12 | 7/12+ | 7/12+ |
| Cross-system chains | 12 | 13 | 18 |

## Ticket List

| # | Filename | Title | Priority | Effort | Target File | Tier |
|---|----------|-------|----------|--------|-------------|------|
| 001 | `archive/tickets/completed/GOLDENE2E-001-thirst-driven-acquisition.md` | Thirst-Driven Acquisition | HIGH | Small | `golden_ai_decisions.rs` | 1 |
| 002 | `archive/tickets/completed/GOLDENE2E-002-two-agent-trade-negotiation.md` | Two-Agent Trade Negotiation | HIGH | Large | `golden_trade.rs` | 1 |
| 003 | `archive/tickets/completed/GOLDENE2E-003-multi-hop-travel-plan.md` | Multi-Hop Travel Plan | HIGH | Medium | `golden_ai_decisions.rs` | 1 |
| 004 | `archive/tickets/completed/GOLDENE2E-004-healing-wounded-agent.md` | Healing a Wounded Agent | HIGH | Medium | `golden_care.rs` (new) | 1 |
| 005 | `archive/tickets/completed/GOLDENE2E-005-bladder-relief-with-travel.md` | Bladder Relief with Travel | HIGH | Medium | `golden_ai_decisions.rs` | 1 |
| 006 | `archive/tickets/completed/GOLDENE2E-006-goal-switching-during-multi-leg-travel.md` | Goal Switching During Multi-Leg Travel | HIGH | Medium | `golden_ai_decisions.rs` | 1 |
| 007 | `archive/tickets/completed/GOLDENE2E-007-combat-between-living-agents.md` | Combat Between Living Agents | HIGH | Large | `golden_combat.rs` | 1 |
| 008 | `archive/tickets/completed/GOLDENE2E-008-merchant-restock-return-stock.md` | Merchant Restock and Return Stock Loop | MEDIUM | Large | `golden_trade.rs` | 2 |
| 009 | `archive/tickets/completed/GOLDENE2E-009-carry-capacity-exhaustion.md` | Carry Capacity Exhaustion | MEDIUM | Medium | `golden_production.rs` | 2 |
| 010 | `GOLDENE2E-010-three-way-need-competition.md` | Three-Way Need Competition | MEDIUM | Medium | `golden_ai_decisions.rs` | 2 |
| 011 | `GOLDENE2E-011-wash-action.md` | Wash Action | MEDIUM | Medium | `golden_ai_decisions.rs` | 2 |
| 012 | `GOLDENE2E-012-death-while-traveling.md` | Death While Traveling | MEDIUM | Large | `golden_combat.rs` | 2 |
| 013 | `GOLDENE2E-013-resource-exhaustion-race.md` | Resource Exhaustion Race | MEDIUM | Large | `golden_production.rs` | 2 |

## Dependencies

```
GOLDENE2E-003 (Multi-Hop Travel)
  └── GOLDENE2E-006 (Goal Switch Mid-Travel) -- needs multi-hop working
  └── GOLDENE2E-012 (Death While Traveling) -- needs multi-tick travel

GOLDENE2E-002 (Trade Negotiation)
  └── GOLDENE2E-008 (Merchant Restock Loop) -- builds on trade infrastructure
```

All other tickets are independent.

## Recommended Implementation Order

**Tier 1** (sequential where deps exist):
1. GOLDENE2E-001 (Thirst) — trivial, validates thirst pathway
2. GOLDENE2E-004 (Heal) — new Care domain
3. GOLDENE2E-005 (Bladder/Latrine) — new need + new place
4. GOLDENE2E-003 (Multi-Hop Travel) — tests planner depth
5. GOLDENE2E-006 (Goal Switch Mid-Travel) — depends on 003
6. GOLDENE2E-007 (Combat) — living-agent combat via dedicated hostility goal
7. GOLDENE2E-002 (Trade) — most complex setup

**Tier 2** (order flexible, respecting deps):
- 010, 011, 013 — can start any time
- 008 — after 002
- 012 — after 003

## Harness Additions (incremental, by first ticket needing them)

| Addition | First Needed By |
|----------|-----------------|
| `agent_thirst()` helper | GOLDENE2E-001 |
| `agent_wound_load()` helper | GOLDENE2E-004 |
| `PUBLIC_LATRINE` const + `agent_bladder()` helper | GOLDENE2E-005 |
| `BANDIT_CAMP`, `FOREST_PATH`, `NORTH_CROSSROADS`, `EAST_FIELD_TRAIL` consts + `agent_location()` helper | GOLDENE2E-003 |
| `agent_dirtiness()` helper | GOLDENE2E-011 |

## Engine Discovery Protocol (canonical template)

This initiative exercises emergent behavior through the real AI loop. If implementation reveals that the engine cannot produce the expected emergent behavior, the following protocol applies:

1. **Diagnose**: Identify the specific engine limitation (missing candidate generation path, planner op gap, action handler deficiency, belief view gap, etc.).
2. **Do not downgrade the test**: The test scenario defines the desired emergent behavior. Do not weaken assertions or remove expected behaviors to work around engine gaps.
3. **Fix forward**: Implement the minimal, architecturally sound engine change that enables the emergent behavior. Document the change in a new "Engine Changes Made" subsection under "What to Change". Each fix must:
   - Follow existing patterns in the affected module
   - Include focused unit tests for the engine change itself
   - Not introduce compatibility shims or special-case logic
4. **Scope guard**: If the required engine change exceeds this ticket's effort rating by more than one level (e.g., a Small ticket needs a Large engine change), stop and apply the 1-3-1 rule: describe the problem, present 3 options, recommend one, and wait for user confirmation before proceeding.
5. **Document**: Record all engine discoveries and fixes in the ticket's Outcome section upon completion, regardless of whether fixes were needed.

## Key Files

- `reports/golden-e2e-coverage-analysis.md` — source of truth for gaps
- `crates/worldwake-ai/tests/golden_harness/mod.rs` — shared harness
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` — AI decision tests
- `crates/worldwake-ai/tests/golden_production.rs` — production/economy tests
- `crates/worldwake-ai/tests/golden_combat.rs` — combat/death tests
- `crates/worldwake-ai/tests/golden_determinism.rs` — determinism tests
