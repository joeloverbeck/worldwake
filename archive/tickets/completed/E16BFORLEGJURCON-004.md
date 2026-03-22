# E16BFORLEGJURCON-004: Implement PressForceClaim and YieldForceClaim office actions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — office action defs, authoritative validation, and commit handlers in `worldwake-systems`
**Deps**: E16BFORLEGJURCON-001, E16BFORLEGJURCON-002, E16BFORLEGJURCON-003

## Problem

`PressForceClaim` and `YieldForceClaim` exist only as payload types today. The action catalog still exposes only `bribe`, `threaten`, and `declare_support` from [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs). E16b needs real office-action definitions and commit handlers so force-claim participation becomes explicit authoritative state instead of being inferred later from proximity.

## Assumption Reassessment (2026-03-22)

1. [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs) is the live owner of office political actions. Validation for `declare_support` already lives there via `validate_declare_support_context_in_world()`. There is no separate `crates/worldwake-sim/src/action_validation.rs` surface for this work, so the original file/symbol assumptions were wrong.
2. [`crates/worldwake-systems/src/action_registry.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs) is the aggregate action catalog entry point. There is no `worldwake-sim` action handler registry file to modify. This ticket should extend `register_office_actions()` and let the existing systems-level registry pick the new defs up.
3. [`crates/worldwake-sim/src/action_payload.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs) already contains `ActionPayload::PressForceClaim`, `ActionPayload::YieldForceClaim`, `PressForceClaimActionPayload`, `YieldForceClaimActionPayload`, and both typed accessors. Ticket E16BFORLEGJURCON-003 is already completed and archived. This ticket must not duplicate payload work.
4. The authoritative relation substrate this ticket needs already exists from E16BFORLEGJURCON-002: `WorldTxn::{add_force_claim, remove_force_claim, add_hostility}` and the public query helpers around `contests_office` / `office_controller`. This ticket should reuse those canonical mutation surfaces instead of inventing parallel helpers.
5. `InstitutionalClaim::ForceControl` does **not** exist yet in [`crates/worldwake-core/src/institutional.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs), and [`crates/worldwake-systems/src/perception.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/perception.rs) currently derives institutional beliefs only from `OfficeHolder` and `SupportDeclaration` relation deltas. The original ticket incorrectly pulled force-control institutional metadata into this implementation. That belief/pipeline work belongs to E16BFORLEGJURCON-006.
6. Current force-law succession is still the provisional shortcut in [`resolve_force_succession()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs). Existing focused and golden coverage confirms force-law offices do not yet use `declare_support`, but they also do not yet have explicit force-claim actions. This ticket adds the missing action-layer substrate without replacing the force-control system; that remains E16BFORLEGJURCON-005.
7. The original ticket treated this as "not an AI regression ticket." That is mostly true for planner surfaces, but it still affects the shared action catalog. The correct boundary is: this ticket must add authoritative defs/validation/commit handlers and focused systems tests only; affordance enumeration and planner use remain E16BFORLEGJURCON-007 and later.
8. Architectural mismatch corrected: a recognized current office holder should not be able to `PressForceClaim` against their own already-held office. Recognized title (`office_holder`) and explicit challenger participation (`contests_office`) are distinct authoritative states. Letting the incumbent self-contest would muddy that distinction and weaken the later force-control state machine.
9. Ordering precision: the important contract here is action-lifecycle and authoritative mutation ordering within the action commit. `PressForceClaim` and `YieldForceClaim` are single-tick, non-interruptible social actions whose observable result is relation delta emission in the same commit event. This ticket does not own same-tick cross-agent ordering beyond that normal action contract.
10. Mismatch corrected: "event metadata" in the original wording should be narrowed to the current live surface. This ticket should rely on the action definition's visible `Political` event plus authoritative relation deltas in the committed event record. It should not invent a temporary metadata path before E16BFORLEGJURCON-006 adds the real `ForceControl` institutional claim channel.

## Architecture Check

1. Implementing `PressForceClaim` / `YieldForceClaim` now is better than keeping force participation implicit. The clean architecture is explicit contest state in `contests_office`, explicit office control in `office_controller`, and explicit recognition in `office_holder`, each mutated by the layer that owns it.
2. Keeping validation and handlers in [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs) is cleaner than splitting this across a new sim-layer validator file. The repo already co-locates office-action def registration, authoritative payload validation, and commit behavior there.
3. Deferring `InstitutionalClaim::ForceControl` to E16BFORLEGJURCON-006 is the more robust architecture. Adding an ad hoc temporary metadata path here would create a second, throwaway political-belief mechanism just before the canonical institutional-claim surface lands.
4. Blocking incumbents from pressing a force claim on their own occupied seat is cleaner than allowing redundant self-contests. The later control system needs challenger membership to mean "actively contesting recognized authority," not "anyone related to the office including the lawful holder."

## Verification Layers

1. `PressForceClaim` and `YieldForceClaim` defs are registered in the live action catalog -> focused `office_actions.rs` registry test
2. `PressForceClaim` preconditions reject wrong-place, wrong-law, ineligible, duplicate-claim, and already-holder cases -> focused authoritative validation tests in `office_actions.rs`
3. `YieldForceClaim` preconditions reject non-claimants and wrong-place cases -> focused authoritative validation tests in `office_actions.rs`
4. `PressForceClaim` commit adds `contests_office` and, against a different incumbent, adds hostility -> authoritative world-state assertions in focused action tests
5. `YieldForceClaim` commit removes `contests_office` -> authoritative world-state assertions in focused action tests
6. Commit events are visible same-place political action records with the expected targets/tags -> focused event-log assertions in `office_actions.rs`
7. This ticket does **not** verify force-control institutional belief projection. That belongs to E16BFORLEGJURCON-006 once `InstitutionalClaim::ForceControl` exists.

## What to Change

### 1. Extend `register_office_actions()` in `office_actions.rs`

Add real action defs and handlers for:

- `press_force_claim`
- `yield_force_claim`

They should match the existing office-action registration pattern:

- `ActionDomain::Social`
- `ActorAlive`
- fixed 1-tick duration
- `VisibilitySpec::SamePlace`
- `EventTag::Political` + `EventTag::WorldMutation`
- payload-specific authoritative validators

### 2. Add authoritative payload/context validation in `office_actions.rs`

`PressForceClaim` preconditions:

- actor is alive
- `payload.office` has `OfficeData`
- actor is at the office jurisdiction
- office uses `SuccessionLaw::Force`
- actor is eligible under office rules
- actor does not already contest this office
- actor is not the current recognized `office_holder`

`YieldForceClaim` preconditions:

- actor is alive
- `payload.office` has `OfficeData`
- actor currently contests the office
- actor is at the office jurisdiction

### 3. Add commit handlers in `office_actions.rs`

`PressForceClaim` commit:

- validate authoritative context again at commit
- `txn.add_force_claim(actor, office)`
- if `office_holder(office) == Some(holder)` and `holder != actor`, then `txn.add_hostility(actor, holder)`
- add the office target, and add the incumbent target when hostility is created

`YieldForceClaim` commit:

- validate authoritative context again at commit
- `txn.remove_force_claim(actor, office)`
- add the office target

### 4. Keep belief/perception work out of this ticket

Do **not** add temporary `InstitutionalClaim::ForceControl` stubs, temporary event payload channels, or perception wiring here. Ticket E16BFORLEGJURCON-006 owns that layer.

## Files to Touch

- `crates/worldwake-systems/src/office_actions.rs` (modify — add defs, validators, handlers, and focused tests)
- `crates/worldwake-systems/src/action_registry.rs` (tests only if required by the new action count/name expectations)

## Out of Scope

- `InstitutionalClaim::ForceControl`, `InstitutionalBeliefKey::ForceControllerOf`, perception extraction, and Tell relayability — E16BFORLEGJURCON-006
- Force-control state machine and removal of `resolve_force_succession()` — E16BFORLEGJURCON-005
- AI affordance enumeration and planner/operator wiring — E16BFORLEGJURCON-007 and later
- Record integration for force-control transitions — E16BFORLEGJURCON-005 / E16BFORLEGJURCON-006 as appropriate

## Acceptance Criteria

### Tests That Must Pass

1. `press_force_claim` appears in the registered office-action catalog
2. `yield_force_claim` appears in the registered office-action catalog
3. Pressing a force claim at the correct jurisdiction succeeds and creates `contests_office(actor, office)`
4. Pressing a force claim when not at the jurisdiction fails authoritative validation
5. Pressing a force claim for a support-law office fails authoritative validation
6. Pressing a force claim when ineligible fails authoritative validation
7. Pressing a force claim when already contesting fails authoritative validation
8. Pressing a force claim as the already recognized holder fails authoritative validation
9. Pressing a force claim against a different incumbent adds `hostile_to(actor, holder)`
10. Yielding a force claim removes `contests_office(actor, office)`
11. Yielding when the actor is not contesting fails authoritative validation
12. Commit events remain visible same-place political action records
13. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Force-claim participation is explicit authoritative state, never inferred from co-location alone
2. Recognized office holding and active force-challenge participation remain distinct authoritative concepts
3. Pressing a claim against an incumbent leaves persistent hostility as political aftermath
4. No backwards-compatibility shims or temporary metadata channels are introduced
5. No existing tests break

## Tests

### New/Modified Tests

1. `crates/worldwake-systems/src/office_actions.rs` `register_office_actions_creates_social_defs`
Rationale: extends the live action-catalog assertion so the new office actions are part of the real registry, not just helper functions on disk.
2. `crates/worldwake-systems/src/office_actions.rs` focused `PressForceClaim` validation and commit tests
Rationale: prove the authoritative contract at the layer that owns office-action validation and mutation, including the no-self-contest architectural guard.
3. `crates/worldwake-systems/src/office_actions.rs` focused `YieldForceClaim` validation and commit tests
Rationale: prove claim withdrawal clears only the explicit contest relation and still obeys local-jurisdiction requirements.

## Test Plan

### Commands

1. `cargo test -p worldwake-systems register_office_actions_creates_social_defs`
2. `cargo test -p worldwake-systems press_force_claim`
3. `cargo test -p worldwake-systems yield_force_claim`
4. `cargo test -p worldwake-systems`
5. `cargo clippy --workspace`
6. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-22
- What actually changed:
  - added real `press_force_claim` and `yield_force_claim` action defs, validators, and commit handlers in `crates/worldwake-systems/src/office_actions.rs`
  - extended focused office-action tests to cover registration, authoritative rejection paths, commit-time claim mutation, hostility creation, and visible political event records
  - updated the planner semantics classification test in `crates/worldwake-ai/src/planner_ops.rs` so the workspace reflects the corrected architecture boundary: the new force-claim actions are registered but intentionally remain unclassified until the later AI/planner tickets
- Deviations from original plan:
  - did not add `InstitutionalClaim::ForceControl` or perception wiring here; that was removed from scope as architecturally premature and remains ticket E16BFORLEGJURCON-006
  - did not add a new sim-layer validation file or registry hook; the live architecture already owns office-action validation and registration inside `worldwake-systems`
  - added an explicit guard preventing the recognized office holder from pressing a force claim against their own seat, because that keeps `office_holder` and `contests_office` semantically distinct for the later force-control state machine
- Verification results:
  - `cargo test -p worldwake-systems register_office_actions_creates_social_defs` passed
  - `cargo test -p worldwake-systems press_force_claim` passed
  - `cargo test -p worldwake-systems yield_force_claim` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo clippy --workspace` passed
  - `cargo test --workspace` passed
