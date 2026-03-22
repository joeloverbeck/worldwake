# E16BFORLEGJURCON-005: Implement office force-control system (replace resolve_force_succession)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — office system in worldwake-systems
**Deps**: E16BFORLEGJURCON-001, E16BFORLEGJURCON-002, E16BFORLEGJURCON-003, E16BFORLEGJURCON-004

## Problem

The current `resolve_force_succession()` in `worldwake-systems/src/offices.rs` is a thin placeholder: if exactly one eligible agent is present after the vacancy period, they are installed. This must be replaced with the full state machine: explicit control tracking, contested state, departure clears control, uncontested hold period before installation. The old function is removed, not kept alongside (Principle 26).

## Assumption Reassessment (2026-03-22)

1. `resolve_force_succession()` exists in `offices.rs` (lines ~288-333). It checks for exactly one eligible contender present and installs them. It will be fully replaced.
2. The politics system flow in `offices.rs` dispatches to `resolve_force_succession()` for `SuccessionLaw::Force` offices. The dispatch point needs to call the new logic instead.
3. `OfficeForceProfile` and `OfficeForceState` will exist from ticket -001. `contests_office/contested_by` and `office_controller/offices_controlled` will exist from ticket -002.
4. N/A — not an AI regression ticket.
5. N/A — no ordering dependency beyond tick-level system execution.
6. Removing `resolve_force_succession` entirely. The missing substrate it stood in for (explicit contest/control state) is now being added by tickets -001 and -002. This does not reopen regressions because the old behavior was intentionally conservative and no golden tests depend on its exact behavior (golden political tests use support-law offices).
7. N/A — not a start-failure ticket.
8. Closure boundary: the force-control system resolves `office_controller` mutations per tick and, when the installation gate is met, atomically installs `office_holder`. The authoritative symbols are `office_controller` (new relation from -002), `OfficeForceState` (new component from -001), and `install_office_holder()` (existing).
9. N/A — no ControlSource manipulation.
10. N/A — no golden scenario in this ticket (golden coverage is ticket -009).
11. No mismatches found. The old `resolve_force_succession` is confirmed to be the only force branch.
12. Installation requires `control_since + uncontested_hold_ticks <= current_tick` with `last_uncontested_tick == current_tick - 1` (no gap). The math is simple integer comparison on `Tick` values.

## Architecture Check

1. The state machine is directly derived from the spec's lifecycle diagram. Four concrete situations (no claimants, one controller, same controller continues, multiple claimants) map to branches in a per-tick scan. Installation is a fifth branch gated on temporal continuity. This is cleaner than the presence-only heuristic.
2. `resolve_force_succession()` is removed entirely (Principle 26). No wrapper or compatibility shim.

## Verification Layers

1. One claimant becomes controller → authoritative world state (relation check)
2. Controller continuity breaks on second claimant → `OfficeForceState.contested_since` set
3. Controller leaves → `office_controller` cleared → authoritative state
4. Controller dies → claim removed, control cleared → authoritative state
5. Uncontested hold period met → `office_holder` installed → authoritative state + event log
6. Multiple claimants block installation → `office_controller` stays None
7. Installation clears all claims → relation state check
8. Installation emits visible event with `InstitutionalClaim::OfficeHolder` → event log + metadata
9. Installation appends to office register record → record data check

## What to Change

### 1. Replace `resolve_force_succession()` in `offices.rs`

Remove the old function entirely. Add new per-tick force-control logic:

For each force office with `OfficeForceProfile`:
1. Gather live claimants from `contested_by(office)`, filter to those present at jurisdiction
2. **Departure rule**: if current `office_controller` is not present at jurisdiction, `clear_office_controller(office)` and reset `control_since`
3. Derive situation:
   - **No claimants present**: clear controller, preserve holder
   - **One claimant, office uncontrolled**: `set_office_controller`, set `control_since = tick`, clear `contested_since`
   - **Same sole controller remains**: keep continuity, set `last_uncontested_tick = tick`
   - **Multiple claimants present**: clear controller, set `contested_since` if absent
4. **Installation gate**: if controller uncontested for `uncontested_hold_ticks` AND no other live claimants in `contested_by`:
   - `install_office_holder(office, controller)` (existing helper)
   - Clear `vacancy_since` on `OfficeData`
   - Clear all `contests_office` entries for this office
   - Emit visible installation event with `InstitutionalClaim::OfficeHolder` metadata
   - Append to jurisdiction's office register record

### 2. Emit political events for state transitions

Each state transition (control established, control lost, office contested, installation) emits a visible `EventTag::Political` event at the jurisdiction with `VisibilitySpec::SamePlace` and appropriate `InstitutionalClaim::ForceControl` metadata.

### 3. Handle dead claimants

During the per-tick scan, remove dead claimants from `contests_office` (using `txn.remove_force_claim`). Dead claimants cannot hold control.

## Files to Touch

- `crates/worldwake-systems/src/offices.rs` (modify — remove `resolve_force_succession`, add force-control state machine)

## Out of Scope

- Public order degradation from contested offices — deferred to E19
- Guard responses to coups — deferred to E19
- Patrol escalation around disputed seats — deferred to E19
- Additional installation gates (guard acquiescence, faction support thresholds) — deferred to E19
- AI integration (affordances, planner ops) — E16BFORLEGJURCON-007/008
- Institutional belief queries — E16BFORLEGJURCON-006

## Acceptance Criteria

### Tests That Must Pass

1. One uncontested claimant becomes controller but NOT immediately recognized holder
2. Controller continuity breaks when another claimant arrives
3. Controller continuity breaks when controller dies
4. Controller loses control immediately upon leaving jurisdiction
5. Returning to jurisdiction after departure restarts control clock (`control_since` resets)
6. After `uncontested_hold_ticks`, sole controller with no other live claimants is installed as `office_holder`
7. Multiple simultaneous claimants keep office contested and block installation
8. `office_controller` and `office_holder` never diverge into invalid multiplicity (both 1:1)
9. Dead claimants are removed from `contests_office`
10. Installation clears all active force claims for the office
11. Installation emits visible event with `InstitutionalClaim::OfficeHolder` metadata
12. Installation appends entry to office register record (if one exists)
13. Control-established and control-lost events emit `InstitutionalClaim::ForceControl` metadata
14. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. No office has more than one recognized holder (`office_holder` is 1:1)
2. No office has more than one current controller (`office_controller` is 1:1)
3. Controller and recognized holder are distinct concepts stored in separate relations
4. Physical presence at jurisdiction is required to hold control; departure clears control immediately (Principle 8)
5. No hidden "time at place" heuristic substitutes for stored control state
6. The provisional `resolve_force_succession` is fully removed (Principle 26)
7. All values remain deterministic and integer-based
8. No existing golden tests break (current goldens use support-law offices)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs` test module — comprehensive focused tests covering all 4 situations plus installation gate, departure, death, and event emission

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
