# S52EVIDAFT-005: Golden E2E test for theft evidence discovery

**Status**: COMPLETED
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
4. `PerceptionProfile` required on agents that need to observe evidence — current repo rule is the `Authoritative-To-AI Impact Rule` and golden-harness guidance in `AGENTS.md` / `docs/golden-e2e-testing.md`, not `CLAUDE.md`.
5. `commit_steal` at `transport_actions.rs:589` — will emit ContainerTampered evidence after ticket 002.
6. `evidence_decay_system` — will exist after ticket 003. Evidence decays after `decay_ticks`.
7. `BelievedEvidenceState` — exists after ticket 004. Perception populates this through the explicit current-place observation path, not through generic co-located-entity iteration alone.
8. Existing goldens in `golden_emergent.rs` already prove theft → local investigation and theft → accusation chains, but they do not prove the new `SceneEvidence` authoritative emission/perception/decay substrate end to end. This ticket still owns the evidence-specific golden gap.
9. Live `InvestigateViolation` generation is still driven by stale belief vs current observation in `emit_expectation_violation_candidates()`, not directly by `BelievedEvidenceState`. The golden must therefore prove evidence perception alongside a lawful mismatch-driven investigate candidate, not claim that evidence alone emits the goal.

## Architecture Check

1. Test-only ticket. No production code changes. The golden test exercises the full cross-crate contract: theft action (systems) → evidence emission (systems) → perception (systems) → belief update (core) → mismatch-driven candidate generation (AI) while the same scene evidence remains present and later decays. All interaction through state per P26.
2. No backward-compatibility shims.

## Verification Layers

1. Theft creates ContainerTampered evidence on place → authoritative world state (SceneEvidence present)
2. Guard perceives evidence at same place → belief store assertion (believed_evidence populated)
3. Guard generates `InvestigateViolation` after lawful same-place reobservation exposes a belief-vs-reality mismatch at the evidence scene → decision trace
4. Evidence decays after decay_ticks → authoritative world state (SceneEvidence entries removed)
5. Multi-layer ticket: each invariant mapped to specific proof surface above.

## What to Change

### 1. Add golden scenario: theft evidence discovery

In `crates/worldwake-ai/tests/golden_integration.rs`:

**Setup**:
- theft scene place with a real container holding owned bread.
- 1 thief at the scene with `TheftDispositionProfile`; the scenario drives a real lawful `steal` request against the contained lot and then departs lawfully with the stolen lot. Thief-side AI autonomy is out of scope for this ticket.
- 1 guard/investigator with `PerceptionProfile` and `ViolationDispositionProfile`; not present for the theft itself, but seeded with a stale belief that the owned lot is still at the theft scene.
- guard later returns lawfully to the scene so current-place perception can project `SceneEvidence` and the stale belief mismatch can generate investigation.

**Execution**: Submit the lawful `steal` request, tick until the thief commits it and leaves, then tick until the guard returns, perceives `SceneEvidence`, and generates the investigate branch from the same-place mismatch.

**Assertions**:
- After steal commit: `SceneEvidence` component on Market place contains `ContainerTampered` entry (authoritative world state).
- Guard perceives ContainerTampered evidence (belief store — `believed_evidence` populated with ContainerTampered kind).
- Guard generates `InvestigateViolation` candidate on the return/reobservation tick (decision trace).
- After `decay_ticks` (200 ticks): ContainerTampered evidence entry removed from SceneEvidence (authoritative world state).
- Conservation: total item quantities unchanged (theft moved items, not created/destroyed).

### 2. Add deterministic replay companion

Same scenario with identical seed — assert identical world hash and event-log hash.

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)
- `docs/generated/golden-coverage-matrix.md` (refresh)
- `docs/generated/golden-e2e-inventory.md` (refresh)
- `docs/generated/golden-scenario-map.md` (refresh)

## Out of Scope

- Combat evidence tests (BloodTrail, CombatAftermath) — can be added in follow-up
- Travel evidence tests (MovementTrace) — can be added in follow-up
- Evidence forging or planting
- Forensic analysis
- Production code changes

## Acceptance Criteria

### Tests That Must Pass

1. Golden: theft → ContainerTampered evidence → guard perception → lawful mismatch-driven `InvestigateViolation` candidate
2. Golden: evidence decays after decay_ticks
3. Deterministic replay companion produces identical outcome
4. Conservation: total quantities unchanged
5. Existing suite: `cargo test --workspace`
6. Generated golden inventory/docs refreshed and in sync

### Invariants

1. Evidence created only on action commit — not during planning (P10)
2. Guard perceives evidence locally — no global evidence query (P7)
3. Guard's investigation decision comes from beliefs and local reobservation at the scene — not from authoritative evidence reads in AI planning (P14)
4. Deterministic: same seed → same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs` — theft evidence emission/perception/decay golden scenario + replay companion
2. Generated golden inventory/docs refreshed via `scripts/golden_inventory.py`

### Commands

1. `cargo test -p worldwake-ai -- golden_integration`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

- Completed: 2026-04-05
- Added Scenario 114 in `crates/worldwake-ai/tests/golden_integration.rs`, proving the evidence-specific E2E chain: lawful contained theft commit, authoritative `SceneEvidence` emission, returning-guard evidence perception at the current place, mismatch-driven `InvestigateViolation` selection, decay of the theft residue, and commodity conservation.
- Added deterministic replay coverage for the same scenario in `crates/worldwake-ai/tests/golden_integration.rs`.
- Refreshed `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-e2e-inventory.md`, and `docs/generated/golden-scenario-map.md` so the new scenario is recorded in the generated golden coverage surfaces.
- Deviation from original plan: the final golden does not rely on autonomous thief planning. The theft side is driven by a lawful real external `steal` request, while the owned AI proof surface remains on the returning guard's perception and investigate selection path.
- Verification:
  - `cargo test -p worldwake-ai golden_s52_theft_evidence_discovery -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_integration`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
