# S125INSTREBOU-005: post_bounty reservation lifecycle

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — reconcile `RewardEncumbrance` cardinality for multiple active office bounties; action handlers (`validate_reward_source`, `commit_post_bounty`, `claim_bounty`, `withdraw_bounty`), artifact-lifecycle TTL release hook, payload validator extension
**Deps**: [S125INSTREBOU-001](../archive/tickets/S125INSTREBOU-001.md), [S125INSTREBOU-003](../archive/tickets/S125INSTREBOU-003.md), [S125INSTREBOU-008](../archive/tickets/S125INSTREBOU-008.md)

## Problem

Today `post_bounty` validates fund availability at commit but records no encumbrance, so two parallel commits can both succeed against the same balance — directly violating S125 Acceptance Criterion 5 ("Multiple active bounties cannot overpromise the same reserved funds"). `claim_bounty`, `withdraw_bounty`, and the artifact-lifecycle TTL purge path also have no encumbrance to release/consume because none exists. S125 Deliverable D5 mandates: encumbrance creation at commit, release on TTL expiry, release on `withdraw_bounty`, and consumption on successful `claim_bounty` (transferring the reserved lot from office to claimant in the same authoritative transaction).

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `validate_reward_source` at `crates/worldwake-systems/src/artifact_actions.rs:395-430` calls `world.controlled_commodity_quantity(treasury_entity, ...)` (line 410). `commit_post_bounty` at line 988 creates the SocialArtifact entity but does no encumbrance. `validate_post_bounty_payload_override` at line 903 is already wired via `with_payload_override_validator` at `artifact_actions.rs:39` (S125 reassessment Evidence #8) — the payload-override hook exists; this ticket extends the validator's body, no new wiring. Existing tests at lines 1491, 1731, 1838, 1911, 1954, 2040, 2121, 2222, 2275, 2336 cover the bounty/notice lifecycle and need updates after encumbrance lands. `claim_bounty_depleted_source_fails_and_bounty_stays_active` (line 2121) is particularly load-bearing — it covers the "funds disappeared" path which is the analog of "encumbrance is exhausted."
2. S125 §3 (Reward Reservation) specifies: `commit_post_bounty` creates `RewardEncumbrance`; `claim_bounty` transfers the reserved lot from office to claimant in the same authoritative transaction with encumbrance consumption; TTL expiry path releases the encumbrance; `withdraw_bounty` releases the encumbrance. Section H lifecycle: `Active → Released | Claimed`.
3. Shared abstraction boundary: bounty action handlers + artifact-lifecycle TTL purge. The TTL expiry pathway must invoke a release hook; if the existing artifact-lifecycle code lacks a per-artifact-type release hook, this ticket adds it (small extension, not a new SystemFn per S125 SystemFn Integration). Locate the existing TTL purge code by grepping `bounty_ttl` and `ArtifactPostingProfile` (`crates/worldwake-core/src/social_artifact.rs:18-38`) consumers in `worldwake-systems`.
4. Ordering: same-tick contested postings are resolved by the existing scheduler tie-break rules. The second posting's `validate_reward_source` (now also reading encumbrances) sees the first's encumbrance from the action-trace pre-image. The compared branches are not symmetric — the second posting's authoritative validation must consult the encumbrance state mutated by the first commit.
5. Stale-request boundary: when a planner-selected `post_bounty` action starts, `start_post_bounty` must re-run `validate_reward_source` to catch the case where encumbrance state changed between candidate selection and start. First failure boundary is authoritative start (action-trace surface). Verify that the existing start handler already invokes `validate_reward_source`; if not, the rejection path must be added there per the CLAUDE.md Authoritative-to-AI Impact Rule #4.
6. Cumulative arithmetic: the encumbrance check is `controlled_commodity_quantity(office, kind) - sum(active_encumbrances_on_office_for_kind) >= payload.reward_quantity`. The subtraction must be saturating-aware: if encumbrances exceed balance (which should be impossible by construction), the helper must return zero, not underflow.
7. Adjacent contradictions: existing `post_bounty_commits_social_artifact_with_contention_components` test (line 226) asserts the artifact has contention components but doesn't yet assert encumbrance — required consequence of this ticket is to extend the test to assert the new `RewardEncumbrance` component on the office. Existing `claim_bounty_transfers_reward_and_fulfills_bounty` (line 449) asserts the transfer but doesn't yet assert encumbrance consumption — extend.
8. Post-ticket-review update after [S125INSTREBOU-001](../archive/tickets/S125INSTREBOU-001.md): the landed component is a singleton office component, but this ticket's contract requires summing multiple active encumbrance records for one office. Before wiring lifecycle logic, reassess and correct the authoritative cardinality shape so one office can represent more than one active bounty reservation without overwrite. This may require widening the component payload to a per-office collection or moving the record identity to another lawful ECS surface; choose the cleanest live shape during implementation and update this ticket before coding.

## Architecture Check

1. Encumbrance-as-record (FND-18 records are world state) is the natural home for "this bounty's reward is reserved." Release/consumption are explicit world transitions (FND-4 persistent identity / explicit transfer); release on TTL is a lawful world process (the bounty aged out), not a hidden dampener. The authoritative shape must support multiple active records per office; a singleton component that can be overwritten is not sufficient for S125 AC5.
2. No backward compat: pre-encumbrance commit-time-only validation is replaced, not aliased.
3. The existing `with_payload_override_validator(validate_post_bounty_payload_override)` wiring (line 39) is reused — the validator's body extends to enforce the new reservation-aware contract.

## Verification Layers

1. Encumbrance creation at commit → action trace `commit_post_bounty` event + event-log delta showing the new component on the office.
2. Same-tick overlap rejection (S125 AC5) → focused unit test asserting the second posting fails authoritatively at start; action trace shows the rejection at the start boundary, not at planner candidate generation.
3. Encumbrance consumption at successful claim → event-log delta showing component removal + commodity transfer in the same authoritative transaction (matched action trace event).
4. TTL expiry release → action trace at the artifact-lifecycle tick; event-log delta showing component removal without commodity transfer.
5. Withdrawal release → action trace + event-log delta.
6. Authoritative revalidation at start → action trace `start_post_bounty` rejection event when encumbrance state has changed since selection (Authoritative-to-AI Impact Rule #4 + #5).

## What to Change

### 1. Encumbrance-aware `validate_reward_source`

First, reconcile the authoritative `RewardEncumbrance` cardinality shape from ticket 001 so the lifecycle can store and query multiple active reservations for the same office. Keep `EntityKind::Office` as the institutional owner boundary unless live reassessment proves a different ECS attachment surface is cleaner. Update core schema/sample/save tests if the payload shape changes, and keep the save-format policy truthful.

Extend the `RewardSource::InstitutionalTreasury` arm of `validate_reward_source` (`artifact_actions.rs:395-418`) to:
- Preserve the context-aware `authorize_office_expenditure(...)` authorization path landed by [ticket 008](../archive/tickets/S125INSTREBOU-008.md).
- Compute available balance: `controlled_commodity_quantity(treasury_entity, payload.reward_commodity)` minus the sum of active `RewardEncumbrance` quantities matching `(office=treasury_entity, commodity=payload.reward_commodity)`.
- Reject if available < `payload.reward_quantity` with the existing `ActionError` variant for insufficient funds.

### 2. Encumbrance creation at commit

In `commit_post_bounty` (line 939), within the same `WorldTxn` that creates the SocialArtifact, attach a `RewardEncumbrance { bounty_artifact, commodity, quantity, office }` component to the office.

### 3. Encumbrance consumption at claim

In the existing `claim_bounty` handler, on successful claim:
- Transfer the reserved lot quantity from office to claimant (use existing transfer primitives).
- Remove the matching `RewardEncumbrance` from the office.

Both must happen in the same authoritative transaction so observers see consumption + transfer atomically.

### 4. Encumbrance release at withdrawal

In the existing `withdraw_bounty` handler, on successful withdrawal:
- Remove the matching `RewardEncumbrance` from the office.

### 5. Encumbrance release at TTL expiry

Identify the artifact-lifecycle path that ages out bounties (locate via `bounty_ttl` / `ArtifactPostingProfile.bounty_ttl` consumers). When a bounty artifact transitions out of `Active` due to TTL:
- Remove the matching `RewardEncumbrance` from the office.

If no per-artifact-type release hook exists, add one. This is action-handler integration; no new SystemFn per S125 SystemFn Integration.

### 6. Payload override validator extension

Extend `validate_post_bounty_payload_override` (line 854) to require `payload.reward_quantity` matches what the planner-synthesized payload claims and to surface the encumbrance contract at revalidation. The `with_payload_override_validator(validate_post_bounty_payload_override)` wiring at line 39 is unchanged.

### 7. Authoritative re-check at start

Verify `start_post_bounty` calls `validate_reward_source`; if not, add the call. Per CLAUDE.md Authoritative-to-AI Impact Rule #4, the action's start handler must reject if encumbrance state changed between selection and start.

## Files to Touch

- `crates/worldwake-core/src/reward_encumbrance.rs` (modify if cardinality shape changes)
- `crates/worldwake-core/src/component_schema.rs` / `delta.rs` / `world.rs` / `component_tables.rs` (modify if schema shape or tests change)
- `crates/worldwake-sim/src/save_load.rs` (modify if persisted shape changes)
- `crates/worldwake-systems/src/artifact_actions.rs` (modify — primary)
- Likely: an existing artifact-lifecycle module in `worldwake-systems` (modify — TTL release hook). Confirm exact path during implementation by greping `bounty_ttl` consumers; do not assume a path before verification.

## Out of Scope

- AI candidate generation changes — ticket 006.
- Belief-view accessor — ticket 004.
- Conservation rework — ticket 001 confirms reuse; if encumbrance-to-conservation interaction surfaces during implementation, file as a new finding instead of silently extending conservation.
- Faction encumbrance — S125 Non-Goal.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `commit_post_bounty_creates_reward_encumbrance_on_office`.
2. New focused test: `second_post_bounty_with_overlapping_funds_fails_authoritatively` (S125 AC5).
3. New focused test: `claim_bounty_consumes_encumbrance_and_transfers_lot_to_claimant`.
4. New focused test: `withdraw_bounty_releases_encumbrance_without_transfer`.
5. New focused test: `bounty_ttl_expiry_releases_encumbrance`.
6. New focused test: `start_post_bounty_rejects_when_encumbrance_state_changed_since_selection` (Authoritative-to-AI Impact Rule #4 + #5 proof).
7. Extended existing tests must continue to pass with new assertions:
   - `post_bounty_commits_social_artifact_with_contention_components` (`artifact_actions.rs:226`) — extend to assert encumbrance creation.
   - `claim_bounty_transfers_reward_and_fulfills_bounty` (`artifact_actions.rs:449`) — extend to assert encumbrance consumption.
8. Untouched existing tests must continue to pass: `register_artifact_actions_creates_expected_definitions` (line 198), `post_notice_commits_social_artifact_with_notice_content` (line 333), `post_bounty_fails_when_actor_is_not_colocated_with_posting_place` (line 406), `claim_bounty_rejects_second_claimant_in_race_mode` (line 535), `claim_bounty_depleted_source_fails_and_bounty_stays_active` (line 616), `claim_bounty_rejects_when_proof_is_insufficient` (line 717), `claim_bounty_rejects_when_bounty_is_already_fulfilled` (line 770), `claim_bounty_affordance_targets_known_remote_bounty_by_identity` (line 831).
9. Existing suite: `cargo test -p worldwake-systems`.

### Invariants

1. S125 AC5 holds: two same-tick `post_bounty` commits cannot overpromise the same office's funds; the second's `validate_reward_source` (or `start_post_bounty` revalidation) rejects authoritatively.
2. `RewardEncumbrance` lifecycle: `Active` (after commit) → `Claimed` (consumed at claim) | `Released` (at TTL expiry or withdrawal). No silent state.
3. Conservation continues to hold without modification: lots remain conserved through transfer/release; encumbrance is a claim record.
4. Authoritative transactions are atomic: claim either consumes the encumbrance and transfers the lot, or does neither.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/artifact_actions.rs` (existing `#[cfg(test)]` at line 1183) — six new tests + extensions to two existing tests (`post_bounty_commits_social_artifact_with_contention_components`, `claim_bounty_transfers_reward_and_fulfills_bounty`).
2. If a new artifact-lifecycle module is touched for TTL release, add focused tests there for the release hook.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
3. `scripts/verify.sh`
