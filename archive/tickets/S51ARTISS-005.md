# S51ARTISS-005: Ranking activation for artifact issuance goals

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — posting-goal ranking plus live notice suppression-policy correction in AI crate
**Deps**: S51ARTISS-003

## Problem

`S51ARTISS-003` made `PostBounty` and `PostNotice` candidates emit lawfully, but the live ranking pipeline still assigns both goal kinds zero motive. In addition, the current goal-policy surface still suppresses `PostNotice` at `High` stress even though the first live notice candidate is a high-danger `ThreatWarning`. As a result, autonomous artifact issuance is not yet selectable end to end.

## Assumption Reassessment (2026-04-05)

1. `crates/worldwake-ai/src/ranking.rs` still returns `0` for `GoalKind::PostBounty { .. } | GoalKind::PostNotice { .. }` in `motive_score()`, so emitted posting candidates are filtered into `zero_motive` instead of becoming selectable behavior.
2. `S51ARTISS-003` corrected its own scope before implementation: it now owns candidate emission only, and its archived handoff should leave ranking activation to a later slice.
3. The active golden-closeout ticket [`S51ARTISS-004.md`](/home/joeloverbeck/projects/worldwake/tickets/S51ARTISS-004.md) currently assumes posting behavior is already live. Correction applied: that closeout must depend on this ranking-activation slice before its showcase and golden assertions can become honest.
4. The shared posting-goal substrate from archived [`S51ARTISS-002.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/S51ARTISS-002.md) is already lawful: `GoalKind::PostBounty { posting, terms }` and `GoalKind::PostNotice { posting, topic }` carry the concrete world-facing data ranking needs to discriminate first-pass posting motives without inventing planner-only payload defaults.
5. `GroundedGoal` still does not carry separate motive metadata in `crates/worldwake-ai/src/goal_model.rs`, so this ticket must implement ranking from the concrete `GoalKind` payload plus current belief reads rather than relying on missing side-channel fields.
6. The first live posting candidates currently emitted by `S51ARTISS-003` are narrow: institutional accusation-backed `PostBounty` and high-danger `PostNotice`. This ticket should rank exactly those live cases first rather than over-claiming future delivery-bounty or wanted-notice motive families.
7. `crates/worldwake-ai/src/goal_policy.rs` still suppresses all `PostNotice` goals at `High` stress. Because the first live notice candidate is `GoalKind::PostNotice { topic: NoticeTopic::ThreatWarning { .. }, .. }` emitted only under high danger, ranking activation alone cannot make that path selectable. Correction applied: this ticket now owns the matching policy fix for the live threat-warning notice family.

## Architecture Check

1. Ranking activation still belongs in its own AI slice because candidate emission is already live and lawful, but selection remains suppressed by a distinct zero-motive contract in `ranking.rs`.
2. The live `ThreatWarning` notice path also needs a bounded policy correction in `goal_policy.rs`, because the current family-wide suppression rule contradicts the already-lawful high-danger emission case.
3. This corrected approach keeps the canonical pipeline clean: beliefs -> candidate generation -> suppression policy -> ranking -> plan selection, without inventing a second activation shortcut in `agent_tick` or golden harness code.
4. No backward-compatibility shims.

## Verification Layers

1. Posted bounty candidates receive non-zero motive only when the live accusation-backed case is present -> focused ranking test
2. Posted notice candidates receive non-zero motive only when the live high-danger case is present -> focused ranking test
3. Live `ThreatWarning` notice goals remain available under the same high-danger regime that lawfully emits them -> focused goal-policy test
4. Zero posting weights or missing live motive substrate keep posting goals unselectable -> focused ranking test
5. Candidate generation + suppression policy + ranking now make posting behavior behaviorally selectable for downstream showcase/golden work -> focused AI crate coverage; end-to-end proof remains in `S51ARTISS-004`

## What to Change

### 1. Activate posting motive scoring in ranking

In `crates/worldwake-ai/src/ranking.rs`, replace the current hard-coded zero motive for `PostBounty` and `PostNotice` with lawful first-pass ranking that matches the live emitted cases from `S51ARTISS-003`.

- `PostBounty` should use concrete accusation-backed or equivalent live belief inputs already carried by the emitted goal and current view surface.
- `PostNotice` should use the current believed danger/threat substrate already used for the high-danger posting candidate.
- Preserve the explicit zero-weight gates from `UtilityProfile`.

### 2. Correct live notice suppression policy

In `crates/worldwake-ai/src/goal_policy.rs`, stop suppressing the already-live `PostNotice { topic: NoticeTopic::ThreatWarning { .. } }` path under the same `High` danger regime that lawfully emits it. Keep broader posting families on their existing stressed-social suppression path unless this ticket already owns them.

### 3. Add focused proofs

In the ranking and goal-policy test surfaces, add focused proofs for:
- non-zero motive on live institutional `PostBounty`
- non-zero motive on live danger-warning `PostNotice`
- zero/unselectable retention when the relevant posting weight is zero or the live motive substrate is absent
- live `ThreatWarning` notice availability at high danger after the policy correction

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/goal_policy.rs` (modify)

## Out of Scope

- Candidate emission — archived `S51ARTISS-003`
- Showcase scenario tuning and golden closeout — `S51ARTISS-004`
- Additional posting motive families beyond the two already emitted live cases

## Acceptance Criteria

### Tests That Must Pass

1. A live accusation-backed `PostBounty` candidate receives non-zero motive and survives zero-motive filtering
2. A live high-danger `PostNotice` candidate receives non-zero motive and is no longer suppressed by the policy layer that previously blocked it
3. Zero posting weights keep the corresponding posting goal unselectable
4. Existing suite: `cargo test --workspace`

### Invariants

1. Posting selection remains belief-driven and uses the canonical ranking pipeline
2. Threat-warning notices are not emitted-and-suppressed by contradictory high-danger policy
3. Ranking does not invent motive inputs for posting families that are not yet lawfully emitted

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — focused posting-ranking activation tests
2. `crates/worldwake-ai/src/goal_policy.rs` — focused threat-warning availability policy test

### Commands

1. `cargo test -p worldwake-ai -- ranking`
2. `cargo test -p worldwake-ai -- goal_policy`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-04-05

- Activated non-zero ranking for the two already-live posting families in `crates/worldwake-ai/src/ranking.rs`: accusation-backed institutional `PostBounty` and high-danger `ThreatWarning` `PostNotice`.
- Added focused ranking proofs for positive and zero/unselectable cases in `crates/worldwake-ai/src/ranking.rs`.
- Corrected the live suppression contradiction in `crates/worldwake-ai/src/goal_policy.rs` so `PostNotice { topic: NoticeTopic::ThreatWarning { .. } }` is not suppressed under the same high-danger regime that lawfully emits it, and added a focused policy test there.
- Ticket scope was broadened during implementation from ranking-only to ranking plus the bounded `goal_policy.rs` fix once focused verification exposed that the notice path could not become selectable through ranking changes alone.

Verification:
- `cargo test -p worldwake-ai post_bounty_goal_ -- --nocapture`
- `cargo test -p worldwake-ai post_notice_goal_ -- --nocapture`
- `cargo test -p worldwake-ai threat_warning_notice_remains_available_under_high_danger -- --nocapture`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
