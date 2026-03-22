# E16DPOLPLAN-020: Occupied Office Blocks ClaimOffice

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Focused coverage only
**Deps**: `specs/E16d-political-planning-and-golden-coverage.md`

## Problem

The original ticket assumed support-based office politics lets a challenger replace a living incumbent by building a larger coalition. That is not how the current architecture works. The real uncovered edge is narrower: candidate generation should continue to suppress `ClaimOffice` and `SupportCandidateForOffice` when an office still has a holder, even if stale vacancy metadata exists.

## Assumption Reassessment (2026-03-18)

1. The active spec path is [specs/E16d-political-planning-and-golden-coverage.md](/home/joeloverbeck/projects/worldwake/specs/E16d-political-planning-and-golden-coverage.md), not `specs/E16d-political-deliberation.md`.
2. `ClaimOffice` is vacancy-gated in AI candidate generation. `office_is_visibly_vacant()` requires both `office_data.vacancy_since.is_some()` and `view.office_holder(office).is_none()` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs#L373).
3. `DeclareSupport` is vacancy-gated in the authoritative action layer. `validate_declare_support_context_in_world()` rejects any office whose `vacancy_since` is `None` or whose `office_holder` is still present in [office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs#L582).
4. The succession system resolves only vacancies. `succession_system()` activates vacancy state when a holder disappears, then resolves support or force succession from that vacancy state in [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs#L13) and [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs#L145).
5. Coalition-building for vacant offices is already covered:
   `golden_bribe_support_coalition` in [golden_offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs#L312)
   `golden_threaten_with_courage_diversity` in [golden_offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs#L513)
6. Existing focused coverage already checks the obvious non-vacant case (`vacancy_since = None`) via `political_candidates_require_visible_vacancy_and_skip_existing_declaration` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs#L3664), but it does not explicitly lock down the stale-metadata edge where `vacancy_since` is present while an incumbent still occupies the office.

## Architecture Check

The current vacancy-only office model is cleaner than the original proposal.

1. It keeps offices as explicit world state with concrete transitions: holder lost -> office becomes vacant -> succession resolves vacancy.
2. It avoids smuggling in a hidden recall/impeachment mechanic where support declarations silently depose a living incumbent.
3. If Worldwake later needs incumbent challenges, that should be a separate institution-level mechanism with explicit artifacts and rules, not an overload of `ClaimOffice` or `DeclareSupport`.

## What to Change

### 1. Strengthen focused AI coverage in `candidate_generation.rs`

- Extend `political_candidates_require_visible_vacancy_and_skip_existing_declaration` to cover the occupied-office edge explicitly.
- **Setup**: `vacancy_since = Some(Tick(_))` but `office_holder(office) = Some(holder)`.
- **Expected**: candidate generation emits neither `ClaimOffice` nor `SupportCandidateForOffice`, and records `PoliticalCandidateOmissionReason::OfficeNotVisiblyVacant`.
- Keep the existing same test's already-declared-support branch, since that still proves the vacancy path when the office is truly open.

### 2. Do not add a new golden office scenario

- `golden_offices.rs` already covers coalition building for vacant offices.
- The docs in [golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md), [golden-e2e-scenarios.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md), and [golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) remain accurate after this correction.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs`

## Out of Scope

- Replacing a living incumbent through support counting
- New recall, impeachment, or challenge mechanics
- New golden office scenarios
- Changes to succession system behavior
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `political_candidates_require_visible_vacancy_and_skip_existing_declaration`
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. A living office holder blocks political claim/support candidate emission, even if vacancy metadata is stale.
2. Support-based succession remains vacancy-driven rather than challenger-driven.
3. Existing vacant-office coalition goldens remain the place where bribe/threaten political planning is exercised.

## Test Plan

### New/Modified Tests

1. `candidate_generation.rs::political_candidates_require_visible_vacancy_and_skip_existing_declaration`
   Rationale: prove the exact occupied-office edge that the original ticket mis-modeled as challenger replacement.

### Commands

1. `cargo test -p worldwake-ai political_candidates_require_visible_vacancy_and_skip_existing_declaration`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets`

## Outcome

- Completion date: 2026-03-18
- What actually changed: corrected the ticket from an invalid incumbent-replacement golden scenario to the real architectural invariant, then strengthened focused AI coverage so an occupied office with stale `vacancy_since` still suppresses `ClaimOffice` and `SupportCandidateForOffice`.
- Deviations from original plan: no new golden office scenario was added, no docs were changed, and no production behavior was changed. The original proposed behavior conflicts with the current vacancy-driven office architecture.
- Verification results:
  - `cargo test -p worldwake-ai political_candidates_require_visible_vacancy_and_skip_existing_declaration` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace --all-targets` ✅
