# S51ARTISS-003: Candidate generation for bounty and notice posting

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new posting candidate emission functions in AI crate
**Deps**: S51ARTISS-002

## Problem

The planner can now plan for `PostBounty`/`PostNotice` goals (ticket 002) but no candidates are ever emitted. Agents need belief-driven candidate generation that produces lawful posting goals when institutional role or situational awareness justify it.

## Assumption Reassessment (2026-04-05)

1. Candidate generation at `crates/worldwake-ai/src/candidate_generation.rs` uses `generate_candidates_with_travel_horizon()` calling sequential `emit_*_candidates()` helpers. Pattern established for adding new emission functions.
2. `GoalBeliefView` trait provides access to agent beliefs. Used by all emission functions. Believed entity states with `believed_artifact` field are available for checking existing bounties at places.
3. `UtilityProfile.bounty_posting_weight` and `notice_posting_weight` added by ticket 001. In live code they already exist on the authoritative profile surface and should act as explicit zero-check gates for emission.
4. `GroundedGoal` at `crates/worldwake-ai/src/goal_model.rs` does not carry motive metadata, and `crates/worldwake-ai/src/ranking.rs` still scores `PostBounty` / `PostNotice` at zero. Correction applied: this ticket owns candidate emission only, not posting ranking.
5. Office holder detection: agent's believed institutional claims include `InstitutionalClaim::OfficeHolder` entries. Justice office holders can emit enforcement bounties and wanted notices.
6. `ViolationId` at `crates/worldwake-core/src/violation.rs:17-20` exists for accusation references.
7. Agent beliefs about danger come from perceived threats and threat-warning notices. `BelievedEntityState` tracks danger-relevant observations.
8. `BlockedIntentMemory` passed to `generate_candidates()` — used to suppress recently-failed posting goals.

## Architecture Check

1. Candidate generation reads only from beliefs (GoalBeliefView) — never authoritative world state. Per Principle 14.
2. Posting motivation must be derived from existing belief structures only. The strongest live cases are institutional enforcement bounties from believed accusation records and threat-warning notices from believed danger; no new belief types are needed.
3. `bounty_posting_weight == 0` gates emission entirely — agents without this weight never generate posting candidates, preserving existing behavior.
4. No backward-compatibility shims.

## Verification Layers

1. `PostBounty` candidate emitted for office holder with unresolved consulted accusation and non-zero `bounty_posting_weight`
2. `PostBounty` candidate not emitted when `bounty_posting_weight == 0`
3. `PostNotice` candidate emitted for agent with believed high danger and non-zero `notice_posting_weight`
4. `PostNotice` candidate not emitted when `notice_posting_weight == 0`
5. Cross-layer: candidate generation (AI) reads beliefs (core/sim) only. Golden closeout remains in ticket 004.

## What to Change

### 1. Add `emit_bounty_posting_candidates()`

In `crates/worldwake-ai/src/candidate_generation.rs`:

Check `bounty_posting_weight > 0` first and exit early when it is zero.

Use the corrected posting-goal substrate from `S51ARTISS-002`.

Land the first lawful bounty-posting case only:
- Iterate current believed institutional crime-case claims for the agent.
- Require `InstitutionalKnowledgeSource::RecordConsultation`, a believed office holder match for the issuing office, and a believed `JurisdictionalAuthority` right over the accused carried `via` that same office.
- Emit `GoalKind::PostBounty { posting, terms }` where:
  - `posting.posting_place` is the office seat
  - `posting.claim_place` is the office seat
  - `terms.target = BountyTarget::EliminateEntity { target: accused }`
  - `terms.reward_source` is a lawful office-reserved source already modeled by the shared posting substrate
  - the emitted evidence anchors the office, consulted record, accused, and office seat

Do not invent planner- or candidate-only default bounty semantics beyond the shared goal substrate.

### 2. Add `emit_notice_posting_candidates()`

In `crates/worldwake-ai/src/candidate_generation.rs`:

Check `notice_posting_weight > 0` first and exit early when it is zero.

Land the first lawful notice-posting case only:
- If the agent has a current place and its believed danger pressure is at or above its high danger threshold, emit `GoalKind::PostNotice`.
- Use `posting.posting_place = current place`.
- Use `topic = NoticeTopic::ThreatWarning { place: current place }`.
- Evidence should tie the candidate to the current place plus the concrete local-danger inputs already used by the danger-pressure reader where practical.

### 3. Wire into generate_candidates_with_travel_horizon()

Add calls to `emit_bounty_posting_candidates()` and `emit_notice_posting_candidates()` in the sequential emission chain.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- Planner ops and dispatch — ticket 002
- Ranking changes for posting goals — deferred; live ranking still zeros these goals
- CLI showcase and goldens — ticket 004
- Additional posting motives such as delivery-demand bounty issuance, wanted notices, or richer posting-place heuristics
- Duplicate artifact suppression beyond normal `GoalKey` deduplication

## Acceptance Criteria

### Tests That Must Pass

1. Office holder with consulted accusation and matching believed jurisdiction emits `PostBounty`
2. `bounty_posting_weight == 0` suppresses that `PostBounty` candidate
3. High believed local danger with non-zero `notice_posting_weight` emits `PostNotice`
4. `notice_posting_weight == 0` suppresses that `PostNotice` candidate
5. Existing suite: `cargo test --workspace`

### Invariants

1. Candidate generation reads beliefs only — never authoritative state
2. Zero-weight agents never generate posting candidates
3. Emission functions remain belief-driven, so stale beliefs may still produce stale posting desires

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused tests for institutional `PostBounty` emission and zero-weight suppression
2. `crates/worldwake-ai/src/candidate_generation.rs` — focused tests for danger-driven `PostNotice` emission and zero-weight suppression

### Commands

1. `cargo test -p worldwake-ai -- candidate`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-04-05
- What changed:
  - added posting candidate emission in `crates/worldwake-ai/src/candidate_generation.rs` for the first two lawful live cases: accusation-backed institutional `PostBounty` and high-danger `PostNotice`
  - widened the shared AI-facing read surface in `crates/worldwake-sim/src/belief_view.rs` and `crates/worldwake-sim/src/per_agent_belief_view.rs` so candidate generation can lawfully read `UtilityProfile` and apply the explicit zero-weight posting gates
  - added focused candidate-generation tests covering both posting emissions and both zero-weight suppressions
- Deviations from original plan:
  - during reassessment, the ticket was corrected to candidate emission only because `crates/worldwake-ai/src/ranking.rs` still assigns `PostBounty` and `PostNotice` zero motive; ranking activation remains deferred to follow-up ticket `S51ARTISS-005`
  - the original `Files to Touch` list was too narrow because the promised zero-weight gate required a real belief-view read-surface widening, not just AI-local emitter changes
- Verification results:
  - `cargo test -p worldwake-ai posting_candidates -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace -q`
