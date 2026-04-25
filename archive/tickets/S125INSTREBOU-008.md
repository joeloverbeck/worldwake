# S125INSTREBOU-008: Authorization-policy boundary for office treasury spending

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems::artifact_actions` authorization context and focused validation tests
**Deps**: [S125INSTREBOU-003](S125INSTREBOU-003.md), [S125 spec](../../specs/S125-institutional-treasuries-and-bounty-funding.md)

## Problem

S125 Deliverable D3 requires institutional fund spending to be gated by both office-holder authority and the proposed expenditure's jurisdiction/policy. S125INSTREBOU-003 truthfully extracted the existing holder/faction authority branch into `authorize_office_expenditure(...)`, but live `validate_reward_source` had no jurisdiction-policy branch to extract. The active S125 spec still owns that end-state contract, so the remaining authorization-policy seam needs a bounded implementation ticket instead of being left implicit.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-systems/src/artifact_actions.rs::authorize_office_expenditure(world, actor, office)` currently accepts only `World`, `actor`, and treasury entity. That signature can prove holder authority but cannot represent the proposed expenditure's posting place, claim place, jurisdiction, target, or policy context.
2. `validate_post_bounty_context` in the same file verifies `payload.issuing_authority` and `payload.jurisdiction` exist, then calls `validate_reward_source`, but it does not currently prove that an office-funded bounty is scoped to the office's authoritative jurisdiction.
3. Candidate generation in `crates/worldwake-ai/src/candidate_generation.rs::emit_bounty_posting_candidates` already filters on believed `RightKind::JurisdictionalAuthority` and emits `issuing_authority: Some(office)` with `jurisdiction: Some(posting_place)`, but authoritative validation must not rely on AI-side belief filtering.
4. Shared abstraction boundary: authoritative `post_bounty` validation in `worldwake-systems::artifact_actions`. The AI and belief-view tickets may decide when a candidate is worth emitting, but final legality belongs to this validation boundary.
5. Adjacent contradictions: none requiring production changes in already-completed S125INSTREBOU-003. This ticket owns the remaining branch by widening or wrapping the helper with enough context to validate the proposed office expenditure honestly.

## Architecture Check

1. The clean boundary is an authoritative validation path that receives the proposed expenditure context and checks it against concrete office state such as `OfficeData.jurisdiction`. This keeps social right/policy enforcement in world state, not in AI assumptions or string conventions.
2. No backward compatibility: the narrower helper call should be replaced or wrapped by the context-aware validation path rather than preserved as a parallel authorization alias.

## Verification Layers

1. Office-funded bounty inside office jurisdiction -> focused systems test proving the existing lawful S125 path still passes.
2. Office-funded bounty outside office jurisdiction -> focused systems test proving authoritative validation rejects before commit.
3. Missing or mismatched `issuing_authority` / treasury office context -> focused systems test proving the reward source cannot silently bypass the office-policy check.
4. Single-layer authoritative validation ticket — AI candidate suppression and belief-view funding reads remain owned by S125INSTREBOU-004 and S125INSTREBOU-006.

## What to Change

### 1. Reassess the helper shape

Decide whether to widen `authorize_office_expenditure` with a small context struct or add a private wrapper called from `validate_post_bounty_context`. The resulting call must receive enough context to validate that office treasury spending is lawful for the proposed bounty.

### 2. Enforce office jurisdiction for office treasury spending

For `RewardSource::InstitutionalTreasury { treasury_entity }` where the treasury entity is an `Office`, require the proposed bounty jurisdiction/posting scope to fit the office's authoritative `OfficeData.jurisdiction`. Preserve the existing holder check and insufficient-funds check.

### 3. Keep faction behavior explicit

S125 remains office-scoped. If the existing faction branch must be preserved for pre-existing `RewardSource::InstitutionalTreasury` behavior, keep that preservation explicit in tests and comments; do not broaden S125 into faction treasury design.

## Files to Touch

- `crates/worldwake-systems/src/artifact_actions.rs` (modify)

## Out of Scope

- AI candidate generation changes — S125INSTREBOU-006.
- Belief-view accessor changes — S125INSTREBOU-004.
- Reward encumbrance lifecycle — S125INSTREBOU-005.
- New fiscal-policy profile components beyond the current office-jurisdiction boundary. If implementation proves a richer policy substrate is required, update this ticket before coding.
- Faction treasury design — S125 Non-Goal.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `office_treasury_authorization_accepts_bounty_inside_office_jurisdiction`.
2. New focused test: `office_treasury_authorization_rejects_bounty_outside_office_jurisdiction`.
3. New focused test: `office_treasury_authorization_rejects_mismatched_issuing_authority`.
4. Existing systems suite: `cargo test -p worldwake-systems`.

### Invariants

1. Authoritative validation, not AI candidate generation, is the final legality boundary for spending office treasury funds.
2. The authorization helper or wrapper receives the proposed expenditure context; jurisdiction/policy legality is not inferred from treasury identity alone.
3. Existing valid office-funded bounty cases still pass.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/artifact_actions.rs` — focused `office_treasury_authorization_*` tests in the existing test module.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
3. `scripts/verify.sh`

## Outcome

Completed on 2026-04-25.

- Added a private `OfficeExpenditureContext` in `crates/worldwake-systems/src/artifact_actions.rs` and routed `validate_post_bounty_context` through it so institutional treasury authorization receives the proposed posting place, claim place, issuing authority, and declared jurisdiction.
- `authorize_office_expenditure` now preserves the existing holder/member authority gate and, for office treasuries, requires a matching `issuing_authority`, a declared jurisdiction, and office `OfficeData.jurisdiction` coverage for the posting, claim, and jurisdiction places.
- Added focused `artifact_actions` tests proving in-jurisdiction office-funded bounties pass, out-of-jurisdiction bounties fail authoritatively, and missing or mismatched issuing authority fails before commit.

## Deviations

- The live test helper had to create the same `OfficeRegister` substrate required by `WorldTxn::assign_office`; this is fixture alignment with the current office-register contract, not a production scope expansion.
- Faction treasury behavior remains explicitly preserved by leaving the new jurisdiction-policy branch office-only.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib -- --list`.
- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests::office_treasury_authorization_accepts_bounty_inside_office_jurisdiction -- --exact`.
- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests::office_treasury_authorization_rejects_bounty_outside_office_jurisdiction -- --exact`.
- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests::office_treasury_authorization_rejects_mismatched_issuing_authority -- --exact`.
- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests`.
- Passed `cargo test -p worldwake-systems`.
- Passed `cargo clippy -p worldwake-systems --all-targets -- -D warnings`.
- Passed `git diff --check`.
- Passed `./scripts/verify.sh` (format check, workspace tests, `scripts/check_active_goal_removed.sh`, workspace clippy, workspace all-targets clippy with `-D warnings`, and `scenario-coverage --check`).
