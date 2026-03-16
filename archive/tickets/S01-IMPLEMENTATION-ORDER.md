**Status**: ✅ COMPLETED

# S01 Production Output Ownership Claims — Implementation Order

## Problem

The original ticket ordering placed fixture migration (S01PROOUTOWNCLA-009) after all engine changes (-004 through -008). This creates broken intermediate states: tickets -004 and -005 make `commit_harvest`/`commit_craft` fail when `ProductionOutputOwnershipPolicy` is missing, but no existing test fixture sets that policy. Every ticket from -004 onward claims "existing suite must pass," which is impossible without migrated fixtures.

## Corrected Order

The fix is to split -009 into two parts:
- **-009a**: Fixture migration only (runs BEFORE engine changes)
- **-009b**: Golden integration tests (runs AFTER engine changes, uses the original -009 ticket)

```
COMPLETED:
  -001  Types (ProductionOutputOwnershipPolicy, ProductionOutputOwner)
  -002  Helper (create_item_lot_with_owner)
  -003  Extend can_exercise_control (faction/office delegation)
  -009a Fixture migration
  -004  Harvest commit ownership (resolve_output_owner + update commit_harvest)

COMPLETED:
  -010  Consumption requires possession

COMPLETED:
  -005  Craft commit ownership (same pattern for commit_craft)

COMPLETED:
  -006  Belief view ownership query (believed_owner_of trait method)

COMPLETED:
  -007  Authoritative pickup validation (can_exercise_control gate)
  -011  Fix ConsumeOwnedCommodity planner search (CONSUME_OPS narrowing, barrier fix, GoalSatisfied preference)

COMPLETED:
  -008  Belief affordance filtering (exclude uncontrollable owned lots)

FINALLY (golden integration — everything wired):
  -009b Golden integration + replay tests (original -009, minus fixture migration)
```

## Why This Order

| Step | Rationale |
|------|-----------|
| -009a first | Decouples infrastructure from features. Every subsequent ticket inherits healthy fixtures. No broken intermediate states. |
| -004 before -005 | -005 reuses `resolve_output_owner` from -004 |
| -010 after -004 | -004 creates actor-owned ground lots; -010 enforces possession for consumption so agents don't bypass pickup. Unblocks golden trade/production tests. |
| -006 before -007/-008 | -007/-008 need belief-layer ownership queries |
| -009b last | Golden tests validate the full lifecycle end-to-end |

## Dependency Graph

```text
-001 ✅ ──→ -002 ✅ ──→ -003 ✅
                              ↓
                           -009a ✅ (fixture migration)
                              ↓
                           -004 ✅ (harvest ownership)
                              ↓
                           -010 ✅ (consumption requires possession)
                              ↓
                           -005 ✅ (craft ownership)
                              ↓
                           -006 ✅ (belief view)
                              ↓
                           -007 ✅ (auth pickup)
                              ↓
                           -011 ✅ (planner search fix)
                              ↓
                           -008 ✅ (affordance filter)
                              ↓
                           -009b ✅ (golden tests)
                              ↓
                           -012 ✅ (trade restock — resolved by -011/-008)
```

## Outcome

- **Completion date**: 2026-03-16
- **What changed**: All 12 tickets plus 2 mid-flight additions (-009a split, -010 insertion, -011 planner fix) completed. The corrected ordering prevented broken intermediate states and enabled incremental validation at each step.
- **Deviations**: -012 required no code changes (resolved by prior work). -009b golden tests validated the full ownership lifecycle end-to-end.
- **Verification**: `cargo test --workspace` — all tests pass.
