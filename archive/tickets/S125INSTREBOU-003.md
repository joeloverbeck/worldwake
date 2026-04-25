# S125INSTREBOU-003: Funding authorization helper for office-owned funds

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new helper in `worldwake-systems::artifact_actions`
**Deps**: [S125INSTREBOU-001](S125INSTREBOU-001.md)

## Problem

S125 Deliverable D3 mandates a shared funding authorization helper that gates institutional fund spending: the actor must hold the office whose treasury they propose to spend, and the proposed expenditure must fall within the office's jurisdiction/policy once that policy surface exists. Today, `validate_reward_source` (`crates/worldwake-systems/src/artifact_actions.rs:353-418`) checks institutional authority via `validate_institutional_authority` and quantity via `controlled_commodity_quantity`, but the authorization logic is inline and does not currently include a separate jurisdiction-policy branch. Ticket 005's encumbrance check needs a clean call surface to consume; this ticket extracts the existing authorization step into a private helper without changing observable behavior.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `validate_reward_source` at `crates/worldwake-systems/src/artifact_actions.rs:353-418` calls `validate_institutional_authority(world, actor, treasury_entity)` for the `RewardSource::InstitutionalTreasury` arm, then verifies `world.controlled_commodity_quantity(treasury_entity, payload.reward_commodity) >= payload.reward_quantity`. No standalone authorization helper. Existing tests touching this surface: `post_bounty_commits_social_artifact_with_contention_components` (`artifact_actions.rs:226`) and `post_bounty_fails_when_actor_is_not_colocated_with_posting_place` (`artifact_actions.rs:406`). Their expectations encode the current behavior.
2. S125 §2 (Funding Authorization) specifies the helper signature as a yes/no decision returning `Result<(), ActionError>` to match existing validation idioms. S125 OQ2 keeps the helper private to `artifact_actions` until a second consumer appears (FND-26 system decoupling).
3. Shared abstraction boundary: action validation helpers in `worldwake-systems::artifact_actions`. The helper is a domain service over authoritative state per FND-26.
4. Adjacent contradictions: none. This is a refactor that preserves existing validation behavior exactly; it does not change action semantics. Live reassessment confirmed there is no current jurisdiction-policy branch in the inline validator to extract.

## Architecture Check

1. Extracting authorization keeps responsibility focused: the helper reads existing institutional authority state and produces a yes/no decision; it does not commit or mutate. This separation lets ticket 005's encumbrance check be a sibling concern that runs after authorization passes (FND-26: systems interact through state).
2. No backward compat: replaces the inline check; no shim retained.

## Verification Layers

1. Helper correctness → focused unit tests with explicit office-holder and non-holder cases. No current jurisdiction-policy branch exists in `validate_reward_source`, so an out-of-jurisdiction rejection is deferred to the ticket that introduces that policy surface.
2. Existing post_bounty tests continue to pass → confirms behavioral equivalence with the inlined check (regression guard for the refactor).
3. Single-layer ticket — pure systems-crate unit work; no AI or trace surface required.

## What to Change

### 1. New private helper

Add a private function in `crates/worldwake-systems/src/artifact_actions.rs`:

```rust
fn authorize_office_expenditure(
    world: &World,
    actor: EntityId,
    office: EntityId,
) -> Result<(), ActionError>
```

The body contains the existing `validate_institutional_authority` invocation plus any jurisdiction-policy checks currently inlined in `validate_reward_source` for the `InstitutionalTreasury` arm.

### 2. Refactor `validate_reward_source`

In the `RewardSource::InstitutionalTreasury` arm, call `authorize_office_expenditure(world, actor, treasury_entity)?;` before the `controlled_commodity_quantity` check. Behavior must remain identical: every input that currently passes authorization must still pass; every input that currently fails must still fail with the same `ActionError` variant.

### 3. Focused tests

Add focused tests in the existing `#[cfg(test)]` block at `artifact_actions.rs:1183`:
- `authorize_office_expenditure_accepts_holder_with_current_authority`
- `authorize_office_expenditure_rejects_non_holder`
- `validate_reward_source_uses_authorization_helper_for_institutional_treasury`
- `validate_reward_source_still_rejects_underfunded_institutional_treasury`

No `authorize_office_expenditure_rejects_holder_outside_jurisdiction` test is added in this ticket because no jurisdiction-policy check is currently inlined in `validate_reward_source` to extract. Adding one here would change observable behavior rather than preserve the existing validator contract.

## Files to Touch

- `crates/worldwake-systems/src/artifact_actions.rs` (modify)

## Out of Scope

- Encumbrance check — ticket 005.
- Public/cross-crate exposure of the helper — deferred per S125 OQ2 until a second consumer appears.
- Any behavioral change to existing validations — pure refactor.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `authorize_office_expenditure_accepts_holder_with_current_authority`.
2. New focused test: `authorize_office_expenditure_rejects_non_holder`.
3. New focused tests proving the `validate_reward_source` call surface still gates institutional treasury spending through the helper and still rejects underfunded office treasuries.
4. Existing tests must continue to pass: `post_bounty_commits_social_artifact_with_contention_components` (`artifact_actions.rs:226`), `post_bounty_fails_when_actor_is_not_colocated_with_posting_place` (`artifact_actions.rs:406`), and all 6 `claim_bounty_*` tests at lines 449/535/616/717/770/831.
5. Existing suite: `cargo test -p worldwake-systems`.

### Invariants

1. Behavioral equivalence: the `post_bounty` action's accept/reject decision is unchanged for all existing test inputs.
2. Helper does not mutate world state (read-only validation).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/artifact_actions.rs` (existing `#[cfg(test)]` block at line 1183) — two new `authorize_office_expenditure_*` tests plus two `validate_reward_source_*institutional_treasury` regression tests.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
3. `scripts/verify.sh`

## Outcome

Completed on 2026-04-25.

- Added private `authorize_office_expenditure(...)` in `crates/worldwake-systems/src/artifact_actions.rs` and routed the `RewardSource::InstitutionalTreasury` arm of `validate_reward_source` through it before the existing balance check.
- Added focused systems tests for holder acceptance, non-holder rejection, institutional reward-source routing through the helper, and preservation of the existing insufficient-funds rejection.
- Preserved observable behavior exactly: the extracted helper delegates to the existing institutional-authority validator, including the currently supported faction branch, and does not add a new jurisdiction-policy rejection.

## Deviations

- The drafted out-of-jurisdiction test was not added because the live inline validation had no jurisdiction-policy branch to extract. S125's jurisdiction/policy enforcement remains future work for the ticket that introduces that live policy surface.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib -- --list`.
- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests::authorize_office_expenditure_accepts_holder_with_current_authority -- --exact`.
- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests::authorize_office_expenditure_rejects_non_holder -- --exact`.
- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests::validate_reward_source_uses_authorization_helper_for_institutional_treasury -- --exact`.
- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests::validate_reward_source_still_rejects_underfunded_institutional_treasury -- --exact`.
- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests::post_bounty_commits_social_artifact_with_contention_components -- --exact`.
- Passed `cargo test -p worldwake-systems --lib artifact_actions::tests::post_bounty_fails_when_actor_is_not_colocated_with_posting_place -- --exact`.
- Passed `cargo test -p worldwake-systems`.
- Passed `cargo clippy -p worldwake-systems --all-targets -- -D warnings`.
- Passed `git diff --check`.
- Passed `./scripts/verify.sh` (live script gates included `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`).
