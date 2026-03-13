# EXCFACACCQUE-000 — Exclusive Facility Access Queues: Index

**Spec**: `specs/DRAFT-exclusive-facility-access-queues.md`
**Phase**: Post-E22, future production/AI hardening
**Dependencies**: E10 (production/transport), E13 (decision architecture)

## Ticket Sequence

| Ticket | Title | Crate(s) | Depends On |
|--------|-------|----------|------------|
| EXCFACACCQUE-001 | Core types, component registration, ActionDefId relocation | core, sim | — |
| EXCFACACCQUE-002 | `queue_for_facility_use` action definition + handler | systems, sim | 001 |
| EXCFACACCQUE-003 | `facility_queue_system` + system manifest registration | systems, sim | 001 |
| EXCFACACCQUE-004 | Grant requirement gate on harvest/craft actions | systems | 001, 003 |
| EXCFACACCQUE-005 | Belief view extensions (queue position, grant query) | sim | 001 |
| EXCFACACCQUE-006 | Planning snapshot + planning state queue/grant support | ai, sim | 001, 005 |
| EXCFACACCQUE-007 | `QueueForFacilityUse` planner op + semantics | ai | 001, 006 |
| EXCFACACCQUE-008 | Candidate generation updates for queue routing | ai | 001, 007 |
| EXCFACACCQUE-009 | Ranking + decision runtime grant detection + replan | ai | 001, 008 |
| EXCFACACCQUE-010 | Failure handling — queue invalidation as explicit blocker | ai | 001, 009 |
| EXCFACACCQUE-011 | Integration tests — multi-agent queue contention | all | 001–010 |

## Critical Design Decisions

- `ActionDefId` must move from `worldwake-sim` to `worldwake-core` so queue types (core components) can reference it without a circular dependency.
- Queue types are authoritative stored state on `EntityKind::Facility`, registered via `component_schema.rs`.
- One queue per facility, one grant at a time, one operation per grant.
- Pruning is permanent-impossibility-only; temporary stock depletion stalls the queue.
- No compatibility layer: exclusive facilities route through queue/grant, not direct contention.
