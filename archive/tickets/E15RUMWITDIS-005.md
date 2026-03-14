# E15RUMWITDIS-005: Add Tell Action Payload and Register Tell Action Definition

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new Tell payload variant in sim, new Tell action registration in systems
**Deps**: `archive/tickets/E15RUMWITDIS-002.md`, `archive/tickets/completed/E15RUMWITDIS-003.md`, `specs/E15-rumor-witness-discovery.md`

## Problem

E15 needs a first-class Tell action in the action framework so later tickets can add commit-time belief transfer and AI affordance generation without introducing stringly typed action logic or ad hoc social special cases.

This ticket should establish the action-framework surface only:

- `ActionPayload::Tell(...)`
- a registered `tell` `ActionDef` in the `Social` domain
- handler wiring and completeness coverage
- start-gate payload validation for the dynamic Tell-specific validity checks the current `ActionDef` precondition vocabulary cannot represent cleanly

It should not try to implement the actual belief-transfer commit semantics or planner exposure yet.

## Assumption Reassessment (2026-03-14)

1. `ActionDomain::Social` already exists in [`crates/worldwake-sim/src/action_domain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_domain.rs), so this ticket must not re-add or redefine it.
2. `EventTag::Social`, `MismatchKind`, `SocialObservationKind::WitnessedTelling`, and `TellProfile` already exist in core. The original ticket overstated how much E15 groundwork was still missing.
3. `ActionPayload` lives in [`crates/worldwake-sim/src/action_payload.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs) and currently has no Tell variant.
4. Action registration still flows through [`crates/worldwake-systems/src/action_registry.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs) and `build_full_action_registries()`, but its current test still describes a "phase two catalog". This ticket should update that test to reflect the live catalog rather than preserving stale naming.
5. The current action semantics surface cannot express "actor has a belief about subject entity" or "belief chain depth is relayable" as static `Precondition` variants. Those checks belong in Tell-specific payload validation at start time, not in the generic `ActionDef` precondition list.
6. The current `RuntimeBeliefView` does not expose `TellProfile` or the actor's known belief subjects. That means Tell affordance enumeration cannot be implemented cleanly in this ticket without broadening the read-model boundary. That remains later E15 work.
7. The existing framework already supports dynamic validation hooks through `with_payload_override_validator(...)` and `with_authoritative_payload_validator(...)`. That is the right place for Tell's subject/listener-specific validity checks at this stage.
8. The E15 spec's payload shape still names both `listener` and `subject_entity`. That duplicates the listener already bound as target 0. This is not ideal architecture, but the existing framework already uses the same pattern in other actions, so this ticket should stay consistent with the current system rather than silently inventing a one-off alternative.

## Architecture Check

1. Adding a real `tell` action definition is still the correct direction. Social transmission should be a first-class action with explicit duration, visibility, and event tags, not hidden inside perception or AI code.
2. The original ticket's static-precondition plan was not a good fit for the current architecture. Tell-specific dynamic checks should live in handler validators, because the generic precondition enum intentionally stays narrow and reusable.
3. A stub handler is acceptable here because it lets the action exist in the registry without faking commit behavior. Ticket `E15RUMWITDIS-006` should own the real world mutation.
4. The clean long-term architecture would remove duplicated entity ids from payloads generally and rely on bound targets plus payload-only extra parameters. That broader action-framework cleanup is out of scope for this ticket and should not be done piecemeal here.
5. No backward-compatibility shims or alias actions.

## What to Change

### 1. Add `TellActionPayload`

In [`crates/worldwake-sim/src/action_payload.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs), add:

```rust
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TellActionPayload {
    pub listener: EntityId,
    pub subject_entity: EntityId,
}
```

Also add:

- `ActionPayload::Tell(TellActionPayload)`
- `ActionPayload::as_tell()`
- serde/roundtrip/accessor test coverage
- public re-export from [`crates/worldwake-sim/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/lib.rs)

### 2. Add `tell_actions` module in `worldwake-systems`

Create [`crates/worldwake-systems/src/tell_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/tell_actions.rs) with:

- Tell `ActionDef`
- Tell handler registration function
- stub `start`, `tick`, `commit`, and `abort` handlers
- Tell payload extraction helper
- Tell payload validators

The Tell `ActionDef` should use:

- name: `tell`
- domain: `ActionDomain::Social`
- targets: one co-located `Agent`
- static preconditions:
  - actor alive
  - target exists
  - target is at actor place
  - target kind is `Agent`
  - target alive
- duration: fixed 2 ticks
- body cost per tick: zero
- interruptibility: `FreelyInterruptible`
- commit conditions:
  - actor alive
  - target exists
  - target is at actor place
  - target kind is `Agent`
  - target alive
- visibility: `VisibilitySpec::SamePlace`
- causal event tags: `{ EventTag::Social, EventTag::WorldMutation }`

### 3. Use validator hooks for Tell-specific dynamic validity

Tell-specific checks that depend on payload content or the actor's belief store should be enforced through handler validators, not generic `Precondition` entries.

At minimum:

- payload override validator should accept only `ActionPayload::Tell(...)`
- authoritative payload validator should reject:
  - missing/non-Tell payload
  - payload `listener` that does not match bound target 0
  - attempts to tell oneself
  - missing actor `AgentBeliefStore`
  - no belief for `subject_entity`
  - subject belief whose source chain depth exceeds actor `TellProfile.max_relay_chain_len`

This gives the action a correct start-gate contract without pulling commit logic forward.

### 4. Wire Tell into the system exports and registry

Update:

- [`crates/worldwake-systems/src/action_registry.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs)
- [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs)

so Tell is part of the normal action catalog and handler completeness verification.

### 5. Update catalog tests to reflect the live registry

The existing full-registry test should stop describing the catalog as "phase two" and should assert Tell is present in the action set.

## Files to Touch

- [`crates/worldwake-sim/src/action_payload.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs)
- [`crates/worldwake-sim/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/lib.rs)
- [`crates/worldwake-systems/src/tell_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/tell_actions.rs)
- [`crates/worldwake-systems/src/action_registry.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs)
- [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs)
- [`tickets/E15RUMWITDIS-005.md`](/home/joeloverbeck/projects/worldwake/tickets/E15RUMWITDIS-005.md)

## Out of Scope

- Tell commit semantics that actually transfer beliefs (`E15RUMWITDIS-006`)
- Tell affordance enumeration and any `RuntimeBeliefView` expansion needed for it (`E15RUMWITDIS-007`)
- Discovery-event emission or mismatch detection
- AI goal generation for Tell
- global action-framework cleanup around payload/target duplication

## Acceptance Criteria

### Tests That Must Pass

1. `ActionPayload::Tell(TellActionPayload { .. })` constructs, roundtrips through bincode, and is returned by `as_tell()`.
2. Tell action is registered in the full action catalog with `ActionDomain::Social`.
3. Tell `ActionDef` uses fixed 2-tick duration, zero body cost, `FreelyInterruptible`, `VisibilitySpec::SamePlace`, and `{Social, WorldMutation}` tags.
4. `verify_completeness()` still passes with Tell included.
5. Tell authoritative payload validation rejects:
   - non-Tell payloads
   - listener/target mismatch
   - self-targeting
   - unknown subject beliefs
   - over-depth relay chains
6. Relevant narrow suites pass before workspace-wide verification.
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test --workspace`

### Invariants

1. `ActionPayload` remains `Default` with `None`.
2. Tell registration uses the existing action framework and validator hooks rather than ad hoc special cases.
3. No existing action registrations change behavior.
4. No planner/AI omniscience is introduced.

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-sim/src/action_payload.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs) — add Tell payload accessor and serde roundtrip tests.
   Rationale: locks down the new payload contract at the enum boundary.
2. [`crates/worldwake-systems/src/tell_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/tell_actions.rs) — add focused tests for Tell `ActionDef` shape and authoritative payload validation.
   Rationale: this is where the ticket's real behavior lives now that dynamic Tell validity is handled through validators instead of static preconditions.
3. [`crates/worldwake-systems/src/action_registry.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs) — update the full registry test to assert `tell` is present.
   Rationale: prevents silent omission from the standard action catalog.

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - added `TellActionPayload`, `ActionPayload::Tell(...)`, `ActionPayload::as_tell()`, and the corresponding sim re-export
  - added `crates/worldwake-systems/src/tell_actions.rs` with Tell `ActionDef` registration, stub lifecycle handlers, Tell payload extraction, and start-gate payload validation
  - wired Tell into the shared action catalog and updated the registry test so the live catalog now asserts `tell` is present
  - updated AI planner-op tests so Tell remains intentionally unclassified until later E15 AI work instead of breaking on the expanded action catalog
- Deviations from original plan:
  - the ticket was corrected before implementation because the original assumptions were stale: `ActionDomain::Social`, `TellProfile`, `EventTag::Social`, `MismatchKind`, and `WitnessedTelling` were already implemented
  - Tell-specific validity checks were implemented through payload validators rather than static `Precondition` entries because the current action semantics surface does not model "actor knows subject" or relay-depth checks cleanly
  - the implementation deliberately stopped at framework registration and validation; no belief-transfer commit logic or affordance enumeration was pulled forward
- Verification results:
  - `cargo test -p worldwake-sim` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
