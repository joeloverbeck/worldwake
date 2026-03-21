# E16CINSBELRECCON-006: Institutional Event Projection for Witnesses

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extend perception system in worldwake-systems
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-003, E16CINSBELRECCON-004

## Problem

When an agent witnesses a visible institutional event (office installation, support declaration, faction membership change), the agent should gain corresponding institutional beliefs without needing to consult a record. This is the "witnessing" acquisition path described in spec §7. Currently, the perception system does not project institutional beliefs from witnessed events.

## Assumption Reassessment (2026-03-21)

1. `perception.rs` in worldwake-systems handles event-driven belief updates for witnesses. It processes committed events and updates agent belief stores for entities at the same place.
2. Political events (office installation, support declaration) already emit events with `EventTag::Political`. The perception system must recognize institutionally-legible events and project corresponding `InstitutionalClaim` beliefs.
3. The existing perception system updates `known_entities` in `AgentBeliefStore`. This ticket extends it to also update `institutional_beliefs`.
4. N/A — not a planner ticket.
5. N/A — no ordering.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatch.
12. N/A.

## Architecture Check

1. Extending the existing perception system is cleaner than creating a separate "institutional perception" system — keeps all perception routing in one place, avoids Principle 24 violation.
2. No backward-compatibility shims.

## Verification Layers

1. Witness of office installation gains `InstitutionalClaim::OfficeHolder` belief → integration test with event log check
2. Witness of support declaration gains `InstitutionalClaim::SupportDeclaration` belief → integration test
3. Agent at different place does NOT gain institutional belief from the event → negative test
4. Provenance is `InstitutionalKnowledgeSource::WitnessedEvent` → belief store inspection

## What to Change

### 1. Extend perception system in `perception.rs`

After the existing entity belief projection for same-place witnesses, add institutional belief projection:

- When processing a committed political event, extract the institutional claim content:
  - Office installation → `InstitutionalClaim::OfficeHolder { office, holder, effective_tick }`
  - Support declaration → `InstitutionalClaim::SupportDeclaration { office, supporter, candidate, effective_tick }`
  - Faction membership change → `InstitutionalClaim::FactionMembership { faction, member, active, effective_tick }`
- For each same-place witness of the event, call `WorldTxn::project_institutional_belief()` with `InstitutionalKnowledgeSource::WitnessedEvent`

### 2. Define mapping from event content to institutional claims

Create a helper function that extracts `Option<InstitutionalClaim>` from an event's payload/tag. This keeps the perception system clean and makes the mapping testable in isolation.

## Files to Touch

- `crates/worldwake-systems/src/perception.rs` (modify — add institutional belief projection for political event witnesses)

## Out of Scope

- ConsultRecord action (ticket -005)
- Tell propagation of institutional claims (ticket -008)
- Record entry mutation when political events fire (ticket -007)
- AI reading institutional beliefs (Phase B2 tickets)
- Non-political events (combat, trade, etc.) — no institutional claims to extract
- Force-claim / contested events (deferred to E16b)

## Acceptance Criteria

### Tests That Must Pass

1. Agent at same place as office installation gains `InstitutionalClaim::OfficeHolder` with `WitnessedEvent` source
2. Agent at same place as support declaration gains `InstitutionalClaim::SupportDeclaration` with `WitnessedEvent` source
3. Agent at different place does NOT gain institutional belief from the political event
4. Multiple witnesses at same place each gain independent institutional beliefs
5. `learned_tick` matches the event's tick and `learned_at` matches the event's place
6. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Information locality (Principle 7): only same-place witnesses gain beliefs
2. Provenance traceability: every projected belief has `WitnessedEvent` source
3. Perception system remains stateless — no stored state, only event-driven projection

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` (or integration test) — office installation witness, support declaration witness, non-witness negative case, multi-witness case

### Commands

1. `cargo test -p worldwake-systems perception`
2. `cargo clippy --workspace && cargo test --workspace`
