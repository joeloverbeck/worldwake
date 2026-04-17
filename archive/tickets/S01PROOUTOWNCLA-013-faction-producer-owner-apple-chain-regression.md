# S01PROOUTOWNCLA-013: Restore faction-owned producer-owner apple consumption chain

**Status**: 🚫 NOT IMPLEMENTED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — ownership/AI consumption path under producer-owned output
**Deps**: archive/tickets/S01PROOUTOWNCLA-009-fixture-migration-and-golden-tests.md (completed), archive/tickets/S01PROOUTOWNCLA-010-consumption-requires-possession.md (completed), archive/tickets/S01PROOUTOWNCLA-011-consume-owned-goal-search-fix.md (completed)

## Problem

The golden scenario `golden_faction_ownership_producer_owner_delegation` currently fails in `crates/worldwake-ai/tests/golden_production.rs`. The scenario still materializes faction-owned apples and the faction member can pick them up via institutional delegation, but the member does not complete the full `harvest -> pickup -> eat` chain within the scenario budget. This leaves the S01 producer-owner golden contract regressed on the live branch.

## Assumption Reassessment (2026-04-15)

1. The isolated command `cargo test -p worldwake-ai --test golden_production golden_faction_ownership_producer_owner_delegation -- --exact` currently fails locally at `crates/worldwake-ai/tests/golden_production.rs:4030` with `Faction member should complete the full chain: harvest → pickup → eat` — verified.
2. The same scenario still asserts and reaches the earlier milestones `FactionOwnedApplesMaterialized` and `MemberPickedUpFactionApples`, so the live failure is narrower than "ProducerOwner delegation is broken" — verified from the scenario assertions in `crates/worldwake-ai/tests/golden_production.rs:4019-4034`.
3. The exact shared boundary under audit is the producer-owned output path from authoritative ownership/control (`ProductionOutputOwnershipPolicy::ProducerOwner`, pickup legality, possession/ownership state) into AI goal admission and plan completion for consuming the picked-up apples.
4. The motivating invariant is not "preserve the old execution story at any cost." The invariant is that a faction member who lawfully harvests and possesses faction-owned apples in this scenario should still be able to finish the local consumption chain, while the outsider remains blocked and leaves the orchard.
5. Archived ownership tickets already cover earlier substrate:
   - `S01PROOUTOWNCLA-009` owns the golden contract
   - `S01PROOUTOWNCLA-010` tightened consumption to require possession
   - `S01PROOUTOWNCLA-011` fixed owned-consumption goal search
   This ticket owns the remaining live regression rather than re-landing those completed surfaces.
6. The replay companion `golden_faction_ownership_producer_owner_delegation_replays_deterministically` still passes, so the current evidence is a behavior regression in the primary scenario contract, not a replay nondeterminism issue.
7. Root cause is not yet assigned. Candidate generation, search, runtime start validation, or the scenario's local ownership/possession state may still be responsible; implementation must fix the earliest live concrete layer proved by reassessment.

## Architecture Check

1. Keeping this ticket centered on the existing golden contract is cleaner than broad ownership cleanup because the current evidence is one bounded producer-owner scenario regression with a known proof surface.
2. No backward-compatibility shims. The fix should restore the lawful ownership/consumption path at the earliest live failing layer rather than adding scenario-only exceptions.

## Verification Layers

1. Faction-owned apples remain lawfully materialized and pick-up eligible for the faction member -> existing `golden_faction_ownership_producer_owner_delegation` milestone assertions plus strongest lower-layer proof found during implementation
2. The member completes the local `harvest -> pickup -> eat` chain -> `golden_faction_ownership_producer_owner_delegation`
3. The outsider is still blocked from picking up faction-owned apples and leaves the orchard -> `golden_faction_ownership_producer_owner_delegation`
4. Replay fidelity remains intact after the fix -> `golden_faction_ownership_producer_owner_delegation_replays_deterministically`

## What to Change

### 1. Find the earliest failing layer in the producer-owner consume chain

Reassess the live path from harvested faction-owned apples through pickup, consumption candidate admission, planning, start validation, and authoritative consumption completion. Fix the earliest layer that prevents the faction member from eating the apples after pickup.

### 2. Keep the golden contract honest

If the current scenario fixture or milestone logic is stale rather than production behavior, tighten the golden setup or assertions lawfully instead of preserving a false narrative. Keep the core producer-owner contract intact.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify if the scenario contract or fixture needs lawful correction)
- Production files under `crates/worldwake-ai/`, `crates/worldwake-systems/`, or `crates/worldwake-core/` as required by the earliest live failing layer identified during reassessment

## Out of Scope

- New ownership policy variants
- Broad producer-owner refactors outside the failing apple chain
- Reopening already-completed S01 tickets unless reassessment proves one archived contract note is factually wrong

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_production golden_faction_ownership_producer_owner_delegation -- --exact`
2. `cargo test -p worldwake-ai --test golden_production golden_faction_ownership_producer_owner_delegation_replays_deterministically -- --exact`
3. `cargo test -p worldwake-ai --test golden_production`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. A faction member can still lawfully pick up and consume faction-owned producer output in the owned golden scenario
2. Outsiders remain blocked from faction-owned pickup in that scenario
3. The fix restores the canonical ownership/control path without scenario-only shims

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs` — restore the failing faction-owned producer-owner chain at the existing golden proof surface
2. Focused lower-layer test(s) in the earliest owning crate discovered during reassessment — prove the concrete failure boundary instead of relying only on the golden

### Commands

1. `cargo test -p worldwake-ai --test golden_production golden_faction_ownership_producer_owner_delegation -- --exact`
2. `cargo test -p worldwake-ai --test golden_production golden_faction_ownership_producer_owner_delegation_replays_deterministically -- --exact`
3. `cargo test -p worldwake-ai --test golden_production`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Archived as NOT IMPLEMENTED on 2026-04-17 per explicit user direction.

- No production or test changes were made for this ticket.
- The producer-owner regression remained unimplemented and was not reassessed further in this archival pass.
- Existing archived tickets `S01PROOUTOWNCLA-009` through `S01PROOUTOWNCLA-011` still document the earlier producer-owner substrate that landed.
