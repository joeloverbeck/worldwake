# S51ARTISS-005: Ranking activation for artifact issuance goals

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — posting-goal ranking and selection activation in AI crate
**Deps**: S51ARTISS-003

## Problem

`S51ARTISS-003` made `PostBounty` and `PostNotice` candidates emit lawfully, but the live ranking pipeline still assigns both goal kinds zero motive. As a result, autonomous artifact issuance is not yet selectable, so the S51 stack is not behaviorally live end to end.

## Assumption Reassessment (2026-04-05)

1. `crates/worldwake-ai/src/ranking.rs` still returns `0` for `GoalKind::PostBounty { .. } | GoalKind::PostNotice { .. }` in `motive_score()`, so emitted posting candidates are filtered into `zero_motive` instead of becoming selectable behavior.
2. `S51ARTISS-003` corrected its own scope before implementation: it now owns candidate emission only, and its archived handoff should leave ranking activation to a later slice.
3. The active golden-closeout ticket [`S51ARTISS-004.md`](/home/joeloverbeck/projects/worldwake/tickets/S51ARTISS-004.md) currently assumes posting behavior is already live. Correction applied: that closeout must depend on this ranking-activation slice before its showcase and golden assertions can become honest.
4. The shared posting-goal substrate from archived [`S51ARTISS-002.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/S51ARTISS-002.md) is already lawful: `GoalKind::PostBounty { posting, terms }` and `GoalKind::PostNotice { posting, topic }` carry the concrete world-facing data ranking needs to discriminate first-pass posting motives without inventing planner-only payload defaults.
5. `GroundedGoal` still does not carry separate motive metadata in `crates/worldwake-ai/src/goal_model.rs`, so this ticket must implement ranking from the concrete `GoalKind` payload plus current belief reads rather than relying on missing side-channel fields.
6. The first live posting candidates currently emitted by `S51ARTISS-003` are narrow: institutional accusation-backed `PostBounty` and high-danger `PostNotice`. This ticket should rank exactly those live cases first rather than over-claiming future delivery-bounty or wanted-notice motive families.

## Architecture Check

1. Ranking activation belongs in its own AI slice because candidate emission is already live and lawful, but selection remains suppressed by a distinct zero-motive contract in `ranking.rs`.
2. This approach keeps the canonical pipeline clean: beliefs -> candidate generation -> ranking -> plan selection, without inventing a second activation shortcut in `agent_tick` or golden harness code.
3. No backward-compatibility shims.

## Verification Layers

1. Posted bounty candidates receive non-zero motive only when the live accusation-backed case is present -> focused ranking test
2. Posted notice candidates receive non-zero motive only when the live high-danger case is present -> focused ranking test
3. Zero posting weights or missing live motive substrate keep posting goals at zero motive -> focused ranking test
4. Candidate generation + ranking now make posting behavior behaviorally selectable for downstream showcase/golden work -> focused AI crate coverage; end-to-end proof remains in `S51ARTISS-004`

## What to Change

### 1. Activate posting motive scoring in ranking

In `crates/worldwake-ai/src/ranking.rs`, replace the current hard-coded zero motive for `PostBounty` and `PostNotice` with lawful first-pass ranking that matches the live emitted cases from `S51ARTISS-003`.

- `PostBounty` should use concrete accusation-backed or equivalent live belief inputs already carried by the emitted goal and current view surface.
- `PostNotice` should use the current believed danger/threat substrate already used for the high-danger posting candidate.
- Preserve the explicit zero-weight gates from `UtilityProfile`.

### 2. Add focused ranking proofs

In the ranking test surface, add focused proofs for:
- non-zero motive on live institutional `PostBounty`
- non-zero motive on live danger-warning `PostNotice`
- zero-motive retention when the relevant posting weight is zero or the live motive substrate is absent

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)

## Out of Scope

- Candidate emission — archived `S51ARTISS-003`
- Showcase scenario tuning and golden closeout — `S51ARTISS-004`
- Additional posting motive families beyond the two already emitted live cases

## Acceptance Criteria

### Tests That Must Pass

1. A live accusation-backed `PostBounty` candidate receives non-zero motive and survives zero-motive filtering
2. A live high-danger `PostNotice` candidate receives non-zero motive and survives zero-motive filtering
3. Zero posting weights keep the corresponding posting goal at zero motive
4. Existing suite: `cargo test --workspace`

### Invariants

1. Posting selection remains belief-driven and uses the canonical ranking pipeline
2. Ranking does not invent motive inputs for posting families that are not yet lawfully emitted

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — focused posting-ranking activation tests

### Commands

1. `cargo test -p worldwake-ai -- ranking`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
