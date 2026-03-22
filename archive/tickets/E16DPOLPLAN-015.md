# E16DPOLPLAN-015: Golden Scenario 18 + 18b — Force succession + deterministic replay

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — law-aware gating for support-based political actions on `SuccessionLaw::Force` offices
**Deps**: E16DPOLPLAN-007

## Problem

No golden test covers `SuccessionLaw::Force` end to end. Reassessing the code also exposes an architectural mismatch: support-based political AI/action paths still treat Force-law offices like Support-law offices even though authoritative succession ignores support counting.

## Assumption Reassessment (2026-03-18)

1. `SuccessionLaw::Force` exists on `OfficeData` — confirmed
2. `succession_system()` handles Force law by calling `resolve_force_succession()`, which installs the sole living eligible present contender — confirmed
3. `DeadAt` exists and dead agents are excluded by `eligible_agents_at()` / `candidate_is_eligible()` — confirmed
4. The golden harness seeds offices as already vacant (`vacancy_since: Some(Tick(0))`) — corrected from the original implicit “vacancy from holder death” assumption
5. Focused authoritative coverage for Force succession already exists in `crates/worldwake-systems/src/offices.rs` — corrected scope: the missing coverage is golden E2E + law-aware AI/action behavior
6. Current AI/action code is law-agnostic for Force offices:
   - `candidate_generation.rs` still emits `ClaimOffice` / `SupportCandidateForOffice` for visibly vacant Force offices
   - `office_actions.rs` still validates support-based political actions without checking `succession_law`
   This contradicts the ticket’s original “no DeclareSupport events” assertion unless production code is corrected first.

## Architecture Check

1. Golden coverage is still valuable: it exercises whole-tick orchestration and deterministic replay for the conservative placeholder Force path.
2. The current architecture is not clean if Force offices continue to accept Support-law political behavior. Those support declarations are semantically meaningless under `resolve_force_succession()` and create dead-state noise.
3. The minimal robust fix is not to invent new force-control mechanics here. It is to make current support-based AI/action paths explicitly Support-law only, leaving the richer Force path to `specs/E16b-force-legitimacy-and-jurisdiction-control.md`.
4. Scenario 18b remains the deterministic replay companion.

## What to Change

### 1. Add to `golden_offices.rs`

- **Scenario 18**: Office with `SuccessionLaw::Force` at VillageSquare. Agent A is the only living eligible present contender. Agent B is colocated but has `DeadAt(Tick(0))`. After the succession period, the Politics system installs A through the Force-law branch.
- **Scenario 18b**: Same seed, verify identical world + event log hashes.
- **Assertions**: A is office holder. No `declare_support` action commits or support-declaration events occur for the Force-law office.

### 2. Add focused regression coverage for the architectural mismatch

- `candidate_generation.rs`: Force-law offices must not generate `ClaimOffice` or `SupportCandidateForOffice`.
- `office_actions.rs`: `declare_support` must fail authoritative validation for a Force-law office instead of committing meaningless support state.

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-systems/src/office_actions.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Support succession law (Scenario 11)
- Multi-candidate force succession (contested force)
- Replacing the placeholder Force branch with the richer E16b control architecture
- Changes to `resolve_force_succession()` itself

## Acceptance Criteria

### Tests That Must Pass

1. `golden_force_succession_sole_eligible` — A installed, no DeclareSupport events
2. `golden_force_succession_deterministic_replay` — identical hashes
3. Focused candidate-generation regression proving Force-law offices do not emit support-based office goals
4. Focused office-action regression proving `declare_support` rejects Force-law offices
5. Existing suites: `cargo test -p worldwake-ai`, targeted `worldwake-systems` tests for `office_actions`

### Invariants

1. Force law never uses support counting
2. Dead agents excluded from eligibility
3. Support-law political goal generation is disabled for Force-law offices, and `declare_support` cannot commit against them
4. Deterministic: same seed → same outcome

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_force_succession_sole_eligible`
2. `golden_offices.rs::golden_force_succession_deterministic_replay`
3. `candidate_generation.rs` Force-law political-goal regression test(s)
4. `office_actions.rs` Force-law `declare_support` validation regression test

### Commands

1. `cargo test -p worldwake-ai golden_offices`
2. `cargo test -p worldwake-ai`
3. `cargo test -p worldwake-systems office_actions`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-18

**What changed**:
- Added Scenario 18 and 18b to `crates/worldwake-ai/tests/golden_offices.rs` for Force-law sole-contender installation plus deterministic replay.
- Added `candidate_generation::tests::political_candidates_skip_force_law_offices`.
- Added `office_actions::tests::declare_support_rejects_force_law_offices`.
- Updated `candidate_generation.rs` so current support-based office goals are emitted only for `SuccessionLaw::Support`.
- Updated `office_actions.rs` so `declare_support` authoritatively rejects Force-law offices.
- Updated `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md`.

**Deviations from original plan**:
- The original ticket claimed `Engine Changes: None`. Reassessment showed an architectural mismatch: Force-law offices still flowed through support-based political AI/action paths. The ticket was corrected and implemented with a minimal production cleanup instead of a tests-only change.
- The original ticket asserted “no `DeclareSupport` events” without accounting for the existing law-agnostic AI path. That invariant only became true after the production gate was added.
- The original scope implied vacancy-from-death setup, but the golden harness seeds offices as already vacant. The completed golden scenario focuses on the actual missing E2E gap: sole living eligible Force-law installation plus dead-rival exclusion and support-path suppression.

**Verification**:
- `cargo test -p worldwake-ai --test golden_offices`
- `cargo test -p worldwake-ai`
- `cargo test -p worldwake-systems office_actions`
- `cargo test --workspace`
- `cargo clippy --workspace`
