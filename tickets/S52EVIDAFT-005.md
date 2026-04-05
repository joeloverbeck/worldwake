# S52EVIDAFT-005: Golden E2E test for theft evidence discovery

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S52EVIDAFT-002, S52EVIDAFT-003, S52EVIDAFT-004

## Problem

Evidence emission (002), decay (003), and perception (004) are implemented but no golden test proves the full E2E chain: theft → evidence creation → guard perception → investigation candidate generation. This ticket adds the golden scenario specified in the spec's Verification section.

## Assumption Reassessment (2026-04-05)

1. `golden_integration.rs` at `crates/worldwake-ai/tests/golden_integration.rs` — exists. Appropriate file for cross-system evidence tests.
2. `GoalKind::InvestigateViolation { violation_id, place }` at `goal.rs:88-91` — exists. Investigation candidates emitted by `emit_expectation_violation_candidates()` at `candidate_generation.rs:3380+`.
3. `ViolationDispositionProfile` exists at `violation.rs` — required on investigating agents.
4. `PerceptionProfile` required on agents that need to observe evidence — per CLAUDE.md golden test note.
5. `commit_steal` at `transport_actions.rs:589` — will emit ContainerTampered evidence after ticket 002.
6. `evidence_decay_system` — will exist after ticket 003. Evidence decays after `decay_ticks`.
7. `BelievedEvidenceState` — will exist after ticket 004. Perception populates this on co-located agents.

## Architecture Check

1. Test-only ticket. No production code changes. The golden test exercises the full cross-crate contract: theft action (systems) → evidence emission (systems) → perception (systems) → belief update (core) → candidate generation (AI) → investigation goal. All interaction through state per P26.
2. No backward-compatibility shims.

## Verification Layers

1. Theft creates ContainerTampered evidence on place → authoritative world state (SceneEvidence present)
2. Guard perceives evidence at same place → belief store assertion (believed_evidence populated)
3. Guard generates InvestigateViolation candidate → decision trace
4. Evidence decays after decay_ticks → authoritative world state (SceneEvidence entries removed)
5. Multi-layer ticket: each invariant mapped to specific proof surface above.

## What to Change

### 1. Add golden scenario: theft evidence discovery

In `crates/worldwake-ai/tests/golden_integration.rs`:

**Setup**:
- 1 place: Market.
- 1 container at Market with items (e.g., 5 Bread owned by a victim agent).
- 1 AI thief at Market with TheftDisposition. Steals from the container.
- 1 AI guard at Market (or arriving shortly after). Has: PerceptionProfile, ViolationDispositionProfile, UtilityProfile.
- Guard has belief about the container's expected contents (prior observation or ownership knowledge) so violation detection can trigger.

**Execution**: Tick until thief commits steal, then tick until guard perceives evidence and generates investigation candidate.

**Assertions**:
- After steal commit: `SceneEvidence` component on Market place contains `ContainerTampered` entry (authoritative world state).
- Guard perceives ContainerTampered evidence (belief store — `believed_evidence` populated with ContainerTampered kind).
- Guard generates `InvestigateViolation` candidate (decision trace).
- After `decay_ticks` (200 ticks): ContainerTampered evidence entry removed from SceneEvidence (authoritative world state).
- Conservation: total item quantities unchanged (theft moved items, not created/destroyed).

### 2. Add deterministic replay companion

Same scenario with identical seed — assert identical world hash and event-log hash.

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)

## Out of Scope

- Combat evidence tests (BloodTrail, CombatAftermath) — can be added in follow-up
- Travel evidence tests (MovementTrace) — can be added in follow-up
- Evidence forging or planting
- Forensic analysis
- Production code changes

## Acceptance Criteria

### Tests That Must Pass

1. Golden: theft → ContainerTampered evidence → guard perception → InvestigateViolation candidate
2. Golden: evidence decays after decay_ticks
3. Deterministic replay companion produces identical outcome
4. Conservation: total quantities unchanged
5. Existing suite: `cargo test --workspace`

### Invariants

1. Evidence created only on action commit — not during planning (P10)
2. Guard perceives evidence locally — no global evidence query (P7)
3. Guard's investigation decision comes from beliefs about evidence — not from authoritative evidence state (P14)
4. Deterministic: same seed → same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs` — Theft evidence discovery golden scenario + replay companion

### Commands

1. `cargo test -p worldwake-ai -- golden_integration`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
