# S57: Golden Gap — Rights Lattice

**Status**: COMPLETED

## Summary

Post-implementation golden gap analysis for S50 (Rights Lattice — Ownership, Access, and Jurisdiction). The live suite now proves typed-rights-backed punishment suppression outside jurisdiction through Scenario 110, and focused coverage proves both live `JurisdictionalAuthority` enumeration and seat-local political actions under the new `seat` / `jurisdiction` split. One meaningful S50 emergence chain still lacks golden E2E proof:

1. an office holder can lawfully punish at a secondary place inside a multi-place jurisdiction even when that place is not the office seat

This is the remaining cross-system contract that distinguishes the S50 seat-vs-jurisdiction split from the earlier single-place office model and from lower-layer focused tests.

## Scenario: Secondary Jurisdiction Punishment Works Away From Office Seat

An office has a distinct seat plus a wider multi-place jurisdiction. The office holder encounters an accused at a secondary jurisdiction place, perceives or retains the needed accusation state there, and lawfully generates and commits punishment without first returning to the office seat.

### Description

1. A justice office has `seat = RulersHall` and `jurisdiction = { RulersHall, GeneralStore }`.
2. The office holder is away from the seat and is currently at `GeneralStore`, which lies inside the office's jurisdiction but is not the seat.
3. An accused agent and the relevant local accusation state are also present or lawfully known at `GeneralStore`.
4. The office holder believes a `JurisdictionalAuthority` right over the accused via that same office.
5. AI generates `PunishAccused` and commits the lawful punishment at `GeneralStore`.
6. The office seat remains `RulersHall`; no office-local political action is required or implied by the punishment branch.

### GoalKinds Exercised

- `PunishAccused`

### ActionDomains Exercised

- `Social` — punishment action chain
- `Travel` — optional prior movement away from the seat if the setup uses a real relocation leg

### Systems Exercised

- **Rights lattice**: `effective_rights()` / `believed_rights()` with `RightKind::JurisdictionalAuthority { via: office }`
- **Office substrate**: `OfficeData { seat, jurisdiction }` split
- **Justice AI**: `emit_punishment_candidates()` office-specific jurisdiction gate
- **Perception / institutional belief**: the accusation and office-holder knowledge path feeding the punishment branch

### Setup Requirements

- One office with a seat distinct from at least one other place in its jurisdiction set
- One office holder with lawful office knowledge
- One accused at the secondary jurisdiction place
- One accusation record or consulted accusation belief that already supports the punishment branch
- Topology or setup that makes the secondary jurisdiction place visibly distinct from the office seat

### What Emergence It Demonstrates

This proves that S50's `seat` / `jurisdiction` split is not just a storage migration. The office remains anchored to one canonical political seat, while the same office's jurisdiction can lawfully authorize justice behavior at another concrete place. The result only emerges when the rights lattice, office substrate, belief-facing rights query, and justice candidate generation all agree on the same office-carried authority.

### Foundation Principle Alignment

- **Principle 7**: legality still depends on locally available accusation and rights knowledge
- **Principle 23**: offices have concrete seat and jurisdiction world state instead of one overloaded scalar
- **Principle 24**: jurisdiction is distinct from office-local presence or generalized control
- **Principle 26**: justice AI reads shared state through the rights lattice instead of a special justice-only office lookup

### Why It Is Not A Duplicate

- **Scenario 110** proves in-jurisdiction versus out-of-jurisdiction punishment, but its positive branch does not require a distinct seat and secondary jurisdiction place.
- Focused tests in `crates/worldwake-systems/src/offices.rs` and `crates/worldwake-systems/src/office_actions.rs` prove `.contains()` membership and seat-local political constraints separately, but there is still no golden that shows the new multi-place jurisdiction value producing lawful punishment away from the office seat.

## Ticket Breakdown

### S57GOLGAP-001: Golden secondary-jurisdiction punishment closeout

- Add a golden scenario plus deterministic replay companion proving:
  - the office seat is distinct from the punishment place
  - the punishment place is still inside the office's jurisdiction set
  - `PunishAccused` generates and commits at the secondary jurisdiction place
  - the result depends on office-specific `JurisdictionalAuthority` rather than seat co-location

**Files**: `crates/worldwake-ai/tests/golden_emergent.rs`
**Effort**: Medium

## Tests

- [ ] secondary-jurisdiction punishment commits away from office seat
- [ ] deterministic replay companion

## Acceptance Criteria

1. The golden proves punishment can commit at a place inside the office's jurisdiction that is not the office seat
2. The scenario asserts the seat/jurisdiction split directly rather than only proving a generic in-jurisdiction branch
3. The assertions use the strongest honest surfaces: authoritative world state for seat and place aftermath, decision trace for `PunishAccused` generation, action trace for punishment commit
4. A deterministic replay companion reproduces the same world and event-log hashes

## Outcome

Completed: 2026-04-05

Implemented via `S57GOLGAP-001`. The delivered proof lives in `crates/worldwake-ai/tests/golden_emergent.rs` as Scenario 111 (`Secondary-Jurisdiction Punishment Away From Office Seat`) plus a deterministic replay companion. The scenario extends the existing Scenario 110 punishment harness so the suite now proves that a single office can keep its canonical seat at `RulersHall` while lawfully punishing at `GeneralStore` through the same office's wider jurisdiction.

The only deviation from the original plan was a lawful fixture correction inside the reused punishment harness: the local crime register had to be placed at the punishment location because authoritative punishment validation resolves against the local register there. No production code changed.

Verification:
- `cargo test -p worldwake-ai golden_jurisdiction_gated_punishment -- --nocapture`
- `cargo test -p worldwake-ai secondary_jurisdiction_punishment -- --nocapture`
- `python3 scripts/golden_inventory.py --write --check-docs`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace -q`
