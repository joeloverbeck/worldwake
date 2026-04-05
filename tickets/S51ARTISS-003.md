# S51ARTISS-003: Candidate generation for bounty and notice posting

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new candidate emission functions in AI crate
**Deps**: S51ARTISS-002

## Problem

The planner can now plan for PostBounty/PostNotice goals (ticket 002) but no candidates are ever emitted. Agents need belief-driven candidate generation that produces posting goals when institutional role, economic motivation, or situational awareness justify it.

## Assumption Reassessment (2026-04-05)

1. Candidate generation at `crates/worldwake-ai/src/candidate_generation.rs` uses `generate_candidates_with_travel_horizon()` calling sequential `emit_*_candidates()` helpers. Pattern established for adding new emission functions.
2. `GoalBeliefView` trait provides access to agent beliefs. Used by all emission functions. Believed entity states with `believed_artifact` field are available for checking existing bounties at places.
3. `UtilityProfile.bounty_posting_weight` and `notice_posting_weight` added by ticket 001 — used for ranking and as zero-check gate.
4. `GroundedGoal` wraps `GoalKind` with motive score and metadata. Posting motive context (institutional enforcement, personal vendetta, etc.) lives in the ranking score, not in GoalKind.
5. Office holder detection: agent's believed institutional claims include `InstitutionalClaim::OfficeHolder` entries. Justice office holders can emit enforcement bounties and wanted notices.
6. `ViolationId` at `crates/worldwake-core/src/violation.rs:17-20` exists for accusation references.
7. Agent beliefs about danger come from perceived threats and threat-warning notices. `BelievedEntityState` tracks danger-relevant observations.
8. `BlockedIntentMemory` passed to `generate_candidates()` — used to suppress recently-failed posting goals.

## Architecture Check

1. Candidate generation reads only from beliefs (GoalBeliefView) — never authoritative world state. Per Principle 14.
2. Posting motivation is derived from existing belief structures (institutional claims, danger observations, demand observations) — no new belief types needed.
3. `bounty_posting_weight == 0` gates emission entirely — agents without this weight never generate posting candidates, preserving existing behavior.
4. No backward-compatibility shims.

## Verification Layers

1. PostBounty candidate emitted for office holder with unresolved accusation → decision trace (candidate list)
2. PostBounty candidate NOT emitted when `bounty_posting_weight == 0` → decision trace absence
3. PostNotice candidate emitted for agent with danger observation → decision trace (candidate list)
4. PostNotice candidate NOT emitted when `notice_posting_weight == 0` → decision trace absence
5. Ranking proportional to posting weight × motive severity → focused unit test on score calculation
6. Cross-layer: candidate generation (AI) reads beliefs (core) from perception (systems). Verified at golden level in ticket 004.

## What to Change

### 1. Add `emit_bounty_posting_candidates()`

In `crates/worldwake-ai/src/candidate_generation.rs`:

Check `bounty_posting_weight > 0` first (early exit if zero).

**Institutional enforcement bounty**: Iterate agent's believed institutional claims. For each office-holder claim where unresolved accusations exist in beliefs:
- Emit `PostBounty { target: EliminateEntity { target: accused }, posting_place }` where `posting_place` is the nearest believed place where posting is lawful.
- Rank: `bounty_posting_weight × accusation_severity`.

**Economic delivery bounty**: If agent has `enterprise_weight > 0` and believes unsatisfied delivery demand:
- Emit `PostBounty { target: DeliverCommodity { commodity, quantity, destination }, posting_place }`.
- Rank: `bounty_posting_weight × demand_urgency`.

**Threat elimination bounty**: If agent has high `danger_weight` and believes a hostile entity threatens a known place:
- Emit `PostBounty { target: EliminateEntity { target: threat }, posting_place }`.
- Rank: `bounty_posting_weight × believed_danger`.

### 2. Add `emit_notice_posting_candidates()`

In `crates/worldwake-ai/src/candidate_generation.rs`:

Check `notice_posting_weight > 0` first (early exit if zero).

**Wanted notice**: For office holders with unresolved crime cases:
- Emit `PostNotice { topic: Institutional { claim: Accusation { ... } }, posting_place }`.
- Rank: `notice_posting_weight × case_severity`.

**Danger warning**: For agents with recent danger observation:
- Emit `PostNotice { topic: ThreatWarning { place: dangerous_place }, posting_place }`.
- Rank: `notice_posting_weight × believed_threat_level`.

### 3. Wire into generate_candidates_with_travel_horizon()

Add calls to `emit_bounty_posting_candidates()` and `emit_notice_posting_candidates()` in the sequential emission chain.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- Planner ops and dispatch — ticket 002
- CLI display — ticket 004
- Golden tests — ticket 004
- Posting-place determination heuristics beyond "nearest believed lawful place"
- Duplicate bounty detection (posting a bounty when one already exists for same target — handled by GoalKey deduplication at goal level)

## Acceptance Criteria

### Tests That Must Pass

1. Office holder with unresolved accusation and `bounty_posting_weight > 0` emits PostBounty candidate
2. Agent with `bounty_posting_weight == 0` emits no PostBounty candidates
3. Agent with danger observation and `notice_posting_weight > 0` emits PostNotice candidate
4. Agent with `notice_posting_weight == 0` emits no PostNotice candidates
5. Ranking score proportional to weight × motive severity
6. Existing suite: `cargo test --workspace`

### Invariants

1. Candidate generation reads beliefs only — never authoritative state (Principle 14)
2. Zero-weight agents never generate posting candidates — no behavior change in unconfigured scenarios
3. Emission functions are belief-driven — stale beliefs may produce stale candidates (correct behavior per P14)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — Unit tests for bounty posting emission: institutional, economic, threat-driven
2. `crates/worldwake-ai/src/candidate_generation.rs` — Unit tests for notice posting emission: wanted, danger warning
3. `crates/worldwake-ai/src/candidate_generation.rs` — Zero-weight gate tests

### Commands

1. `cargo test -p worldwake-ai -- candidate`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
