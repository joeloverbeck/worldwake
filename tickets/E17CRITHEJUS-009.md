# E17CRITHEJUS-009: Implement fine and exile actions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — two new action handlers in systems crate
**Deps**: E17CRITHEJUS-003 (needs `InstitutionalClaim::Verdict`, `PunishmentKind`), E17CRITHEJUS-008 (needs `CrimeRegister` with `Accusation` entries to supersede)

## Problem

No punishment actions exist. Institutional authority cannot impose consequences for crime. E17 needs `fine` (transfers commodity from accused to faction/office treasury) and `exile` (removes faction membership, marks hostile) actions that supersede accusations with verdicts.

## Assumption Reassessment (2026-03-25)

1. `justice_actions.rs` will be created by E17CRITHEJUS-008. Fine and Exile are added to the same module.
2. `RecordData.supersede_entry()` or equivalent append-with-supersede exists in the institutional record infrastructure (used by `ForceControl` superseding prior entries).
3. `member_of` relation exists in `RelationTables` for faction membership. `hostile_to` relation exists (used by E16b force-legitimacy).
4. `can_exercise_control()` extended with faction/office delegation (S01) is how institutional authority is checked.
5. Conservation must be maintained for Fine: commodity transfer is explicit, not destruction.
6. N/A — no heuristic changes.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatches found.
12. Fine amount: `JusticeDispositionProfile.fine_severity` (Permille) applied to stolen quantity. E.g., Permille(500) on 10 units = 5 units. If accused has < 5, start-fail for Fine.

## Architecture Check

1. Fine and Exile are added to the existing `justice_actions.rs` module created by E17CRITHEJUS-008. Both follow the same registration + handler pattern. Fine is structurally similar to trade (commodity transfer between entities). Exile is structurally similar to office-action faction mutations.
2. No backwards-compatibility aliasing.

## Verification Layers

1. Fine transfers commodity from accused to treasury -> conservation check + authoritative state
2. Exile removes `member_of` relation -> authoritative relation state
3. Exile adds `hostile_to(faction, accused)` relation -> authoritative relation state
4. Both supersede the `Accusation` entry with a `Verdict` -> authoritative record state
5. Fine fails when accused has insufficient commodity -> start-failure
6. Exile fails when accused is not a faction member -> start-failure
7. Both fail when actor lacks institutional authority -> start-failure

## What to Change

### 1. `register_fine_action()` in `justice_actions.rs`

- Action definition: name `"fine"`, domain `ActionDomain::Social`, `TargetSpec::SpecificEntity` (accused), `VisibilitySpec::SamePlace`, tags `[EventTag::Social, EventTag::Crime, EventTag::Transfer]`, duration `Fixed(1)`.

Start handler:
- Actor holds office with jurisdiction over the place
- Unresolved `Accusation` exists in CrimeRegister against the target
- Actor and accused at same place
- Accused possesses sufficient commodity (calculated from `fine_severity` * stolen quantity)

Commit handler:
- Calculate fine: `fine_severity` applied to the commodity kind and quantity from the violation
- Transfer commodity lots from accused to faction/office treasury entity
- Supersede `Accusation` with `Verdict { accused, punishment: Fine { commodity, amount }, effective_tick, supersedes_accusation }`
- Emit event

### 2. `register_exile_action()` in `justice_actions.rs`

- Action definition: name `"exile"`, domain `ActionDomain::Social`, `TargetSpec::SpecificEntity` (accused), `VisibilitySpec::SamePlace`, tags `[EventTag::Social, EventTag::Crime, EventTag::Political]`, duration `Fixed(1)`.

Start handler:
- Actor holds office with jurisdiction
- Unresolved `Accusation` exists in CrimeRegister against the target
- Accused is a member of a faction the office controls

Commit handler:
- Remove `member_of(accused, faction)` relation
- Add `hostile_to(faction, accused)` relation
- Supersede `Accusation` with `Verdict { accused, punishment: Exile { from_faction }, effective_tick, supersedes_accusation }`
- Emit event

### 3. Register both in `action_registry.rs`

Wire `register_fine_action()` and `register_exile_action()` into `register_all_actions()`.

## Files to Touch

- `crates/worldwake-systems/src/justice_actions.rs` (modify — add fine + exile handlers)
- `crates/worldwake-systems/src/action_registry.rs` (modify — wire registration)
- `crates/worldwake-systems/src/lib.rs` (modify — add re-exports if needed)

## Out of Scope

- Accuse action (E17CRITHEJUS-008 — prerequisite)
- AI candidate generation for punishment (E17CRITHEJUS-011)
- Treasury entity creation or treasury management system
- Appeal or contest of verdicts (future spec)
- Guard patrol behavior that enforces punishment (E19)
- Golden tests (E17CRITHEJUS-013)

## Acceptance Criteria

### Tests That Must Pass

1. Fine: commodity transferred from accused to treasury entity
2. Fine: `verify_live_lot_conservation()` passes (conservation maintained)
3. Fine: `Accusation` entry superseded by `Verdict { punishment: Fine { .. } }`
4. Fine: start-fail when accused has insufficient commodity
5. Fine: start-fail when actor lacks institutional authority
6. Exile: `member_of(accused, faction)` relation removed
7. Exile: `hostile_to(faction, accused)` relation added
8. Exile: `Accusation` entry superseded by `Verdict { punishment: Exile { .. } }`
9. Exile: start-fail when accused is not a faction member
10. Exile: start-fail when actor lacks institutional authority
11. Both: start-fail when no unresolved accusation exists
12. Both: events emitted with correct tags and `SamePlace` visibility
13. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Conservation maintained for all Fine outcomes (commodity transfer, not destruction)
2. Ownership/possession relations for fine commodity are correctly transferred
3. Exile adds bidirectional hostility (faction hostile to exile, not just exile hostile to faction — verify which direction `hostile_to` uses)
4. Verdicts are append-only — they supersede accusations but don't delete them
5. Both actions require institutional authority (P21 — offices are world state)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/justice_actions.rs` — focused tests for fine (transfer, conservation, supersede, start-failures) and exile (relations, supersede, start-failures)

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy -p worldwake-systems`
3. `cargo build --workspace`
