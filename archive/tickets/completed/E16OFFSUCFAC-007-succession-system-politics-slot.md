# E16OFFSUCFAC-007: Implement Succession System (Politics Slot)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new system function in `worldwake-systems`, system dispatch wiring
**Deps**: Archived dependencies already landed: `archive/tickets/E16OFFSUCFAC-002.md`, `archive/tickets/completed/E16OFFSUCFAC-003-support-declarations-social-relation.md`, `archive/tickets/completed/E16OFFSUCFAC-006.md`

## Problem

The E16 office substrate already exists in core and social action handling already exists in `worldwake-systems`, but the `Politics` slot in the per-tick dispatcher is still wired to a noop. The remaining missing work is the authoritative succession system that turns office vacancy and public support declarations into actual office transfers.

## Assumption Reassessment (2026-03-15)

1. `SystemManifest` already contains the `Politics` slot in canonical order, after `Perception` and before the tick ends — confirmed.
2. `worldwake-systems::dispatch_table()` still maps `Politics` to `noop_system` — confirmed. Replacing that is the main wiring change.
3. `OfficeData`, `SuccessionLaw`, `EligibilityRule`, `support_declarations`, `office_holder`, and `WorldTxn` office/support mutation APIs already exist — confirmed.
4. Bribe, Threaten, and DeclareSupport action handlers already exist in [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs) and their tests already pass — confirmed. They are out of scope for this ticket.
5. `GoalKind::ClaimOffice` and `GoalKind::SupportCandidateForOffice` already exist in core, and the AI layer has placeholder goal-tag support, but political candidate generation and planner-op wiring are still unfinished — confirmed. That remains E16OFFSUCFAC-009, not this ticket.
6. The current public `World` APIs do **not** distinguish “office never had a living holder” from “office still has a stale relation to a dead holder.” `world.office_holder(office)` only reports a live holder. This ticket therefore must key vacancy activation off current authoritative vacancy state, not off a guaranteed “holder died this tick” signal.
7. The current authoritative state does **not** track “agent has held the jurisdiction uncontested for N ticks.” There is no explicit occupation-duration or jurisdiction-control state to support the original force-succession timing assumption.
8. The event layer does **not** have bespoke `VacancyEvent` or `InstallationEvent` structs. Political world changes are represented through ordinary `EventPayload` records with tags, targets, place, observed entities, and state deltas.

## Architecture Reassessment

1. Implementing the politics slot is beneficial and necessary. Leaving succession as a noop preserves dead architecture: offices, support declarations, and political actions exist, but they cannot change office ownership.
2. The cleanest current implementation is a small office-domain system module in `worldwake-systems` that reads authoritative office state and mutates only through `WorldTxn`.
3. The original ticket’s proposed Force logic was too optimistic for the current architecture. Requiring “alone at jurisdiction for >= succession_period_ticks / 2” would need new authoritative occupation-history state. Adding hidden ad hoc history just for this ticket would be a poor fit.
4. For now, Force succession should stay conservative and state-based: once the office has been vacant for its succession period, exactly one eligible live agent present at the jurisdiction wins. Multiple contenders block installation until ordinary combat or movement resolves the contention. This preserves locality and extensibility without inventing opaque timers.
5. If later design work wants richer “hold the seat by force” behavior, the right architecture is an explicit jurisdiction-control or contested-occupation state model, not event-log archaeology and not hidden helper caches.
6. No backward-compatibility shims or alias paths.

## Scope

### In Scope

1. Add `crates/worldwake-systems/src/offices.rs` with the `succession_system` and office-domain helpers.
2. Replace the `Politics` noop in [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs) with the real succession system.
3. Emit ordinary political event records for visible vacancy activation and office installation.
4. Normalize office invariants the system can authoritatively repair:
   - filled offices clear stale `vacancy_since`
   - installed winners clear stale support declarations for that office
5. Add focused tests for support succession, force succession under the current architecture, invariant normalization, and dispatch wiring.

### Out of Scope

1. Reimplementing bribe, threaten, or declare-support action handlers.
2. AI candidate generation, planner-op wiring, or belief-mediated office goals. That is E16OFFSUCFAC-009.
3. `public_order()` and hostile-faction aggregation. That is E16OFFSUCFAC-008.
4. New occupation-duration state or jurisdiction-control state.
5. Changing `SystemManifest` ordering.

## What To Change

### 1. Create `crates/worldwake-systems/src/offices.rs`

Add:

```rust
pub fn succession_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>
```

Behavior:

1. Iterate all offices with `OfficeData`.
2. If the office currently has a live holder:
   - do nothing if `vacancy_since` is already `None`
   - otherwise clear stale `vacancy_since` in a hidden system txn
3. If the office has no live holder and `vacancy_since` is `None`:
   - mark `vacancy_since = Some(tick)`
   - vacate the office relation through `WorldTxn`
   - emit a visible political event at the office jurisdiction with `SamePlace` visibility
4. If the office has no live holder and `vacancy_since` is `Some(start_tick)`:
   - if the succession period has not elapsed, do nothing
   - otherwise resolve by `succession_law`

Support law:

1. Count support declarations for this office.
2. Ignore dead or ineligible candidates even if a stale declaration row exists.
3. If there is exactly one top candidate, install them atomically:
   - `assign_office`
   - set `vacancy_since = None`
   - clear support declarations for the office
   - emit a visible political event at the jurisdiction
4. If there is a tie for top support, extend the contest by resetting `vacancy_since` to the current tick.
5. If there are no valid declarations, also extend the contest by resetting `vacancy_since` to the current tick.

Force law:

1. Gather eligible live agents currently present at the jurisdiction.
2. If exactly one eligible agent is present and the succession period has elapsed, install them atomically and emit a visible political event.
3. If zero or multiple eligible agents are present, leave the office vacant. Combat and movement resolve the contest elsewhere in the normal system pipeline.

### 2. Add helper functions

Helpers should be small, deterministic, and reusable:

- `pub fn offices_with_jurisdiction(place: EntityId, world: &World) -> Vec<EntityId>`
- `pub fn office_is_vacant(office: EntityId, world: &World) -> bool`
- `pub fn eligible_agents_at(office: EntityId, place: EntityId, world: &World) -> Vec<EntityId>`

`candidate_is_eligible` should remain shared logic, not reimplemented ad hoc per branch.

### 3. Replace the politics noop

Update [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs):

1. Add `pub mod offices;`
2. Re-export `succession_system`
3. Map the `Politics` slot to `succession_system`

## Files To Touch

- [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) (new)
- [`crates/worldwake-systems/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs) (modify)
- Ticket archival path after completion

## Acceptance Criteria

### Tests That Must Pass

1. A vacant office with `vacancy_since = None` is activated into succession and records `vacancy_since = current_tick`.
2. Vacancy activation clears any stale office-holder relation through `WorldTxn`.
3. Vacancy activation emits a visible political event at the jurisdiction with `SamePlace` visibility.
4. A living holder prevents succession resolution and clears stale `vacancy_since` if present.
5. Support succession installs the unique top-supported eligible candidate once the period elapses.
6. Support succession ignores stale/ineligible candidates when counting declarations.
7. Tied top support resets the vacancy timer instead of installing anyone.
8. No valid declarations also resets the vacancy timer instead of installing anyone.
9. Successful installation clears `vacancy_since`, sets the new `office_holder`, and clears support declarations for the office atomically.
10. Installation emits a visible political event at the jurisdiction with `SamePlace` visibility.
11. Force succession installs exactly one eligible live contender present at the jurisdiction after the period elapses.
12. Force succession does not install anyone when multiple eligible contenders are present.
13. The `Politics` dispatch slot is no longer a noop.
14. `cargo clippy --workspace --all-targets -- -D warnings`
15. Relevant tests pass first, then `cargo test --workspace`

### Invariants

1. Office uniqueness is preserved: the system never leaves two simultaneous holders for one office.
2. The system reads authoritative office state and writes through `WorldTxn` only.
3. No bespoke compatibility layer, alias, or fallback noop remains for politics dispatch.
4. Determinism: iteration and winner selection remain stable under `BTreeMap`/`BTreeSet` ordering.
5. The system does not invent unsupported occupancy-duration history.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs`
   - vacancy activation
   - stale `vacancy_since` normalization for filled offices
   - support winner installation
   - support tie/no-declaration timer reset
   - stale/ineligible declaration filtering
   - force winner installation with exactly one contender
   - force stalemate with multiple contenders
2. `crates/worldwake-systems/src/lib.rs` or `crates/worldwake-systems/src/offices.rs`
   - dispatch wiring test proving `Politics` now invokes the succession system

### Commands

1. `cargo test -p worldwake-systems succession`
2. `cargo test -p worldwake-systems office_actions -- --nocapture`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Added `crates/worldwake-systems/src/offices.rs` with the per-tick `succession_system`
  - Replaced the `Politics` noop in `worldwake-systems::dispatch_table()`
  - Extracted shared office eligibility logic so succession and office actions use the same rule path
  - Added succession tests for vacancy activation, support resolution, force resolution, timer reset, invariant normalization, and dispatch wiring
- Deviations from original plan:
  - Did not introduce bespoke `VacancyEvent` or `InstallationEvent` types; used ordinary political event records with tags, place, visibility, and deltas
  - Did not implement force “hold location for N ticks” logic because the current architecture has no authoritative occupation-duration state
  - Force succession was implemented as conservative uncontested present-tense installation after the vacancy period elapses
- Verification results:
  - `cargo test -p worldwake-systems succession` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
