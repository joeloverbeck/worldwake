# S32CRIMEMEGOLSUI-001: Scenario 41 — Exile Punishment When Fine Is Not Locally Collectible

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test only
**Deps**: E17 (crime/theft/justice), E16c (institutional beliefs), E16d (political planning), E14 (perception), E15 (social Tell), S27 (expectation-violation goals), `archive/specs/S32-crime-emergence-golden-suites.md`, `docs/golden-e2e-testing.md`

## Problem

The live code has focused coverage for exile punishment selection and exile action commitment, but it still lacks a dedicated golden E2E that proves the full witnessed-theft justice chain ends in `PunishAccused(Exile)` for the live fallback reason: the authority cannot lawfully collect the fine from locally observable stock at punishment-selection time. Scenario 38 already proves the witnessed-theft -> Tell -> Accuse -> PunishAccused chain, but it is intentionally punishment-agnostic at the final step and does not isolate the exile branch as the scenario contract.

## Assumption Reassessment (2026-03-27)

1. The exact shared abstraction boundary under audit is the punishment-selection contract between AI candidate generation and authoritative justice execution:
   `crates/worldwake-ai/src/candidate_generation.rs::candidate_punishment_for_case()`
   chooses `PunishmentKind`, and `crates/worldwake-systems/src/justice_actions.rs::{commit_fine, commit_exile}` enforce the chosen branch.
2. `candidate_punishment_for_case()` exists at `crates/worldwake-ai/src/candidate_generation.rs:533`. The live branch condition is not "accused has no commodities." Fine is emitted only when:
   `fine_amount > 0`, authority place == accused place, and `locally_observed_commodity_quantity(authority, accused, theft.commodity) >= fine_amount`.
   Otherwise the function falls back to `office_governed_faction_for_accused()` and may emit `PunishmentKind::Exile`.
3. The draft ticket's "thief has zero commodities after theft" setup does not match the live architecture. A thief who just stole the fined commodity usually still has at least that commodity unless some later lawful world change removes or relocates it. The ticket scope is corrected to the actual planner contract: no locally collectible fined commodity at punishment-selection time.
4. Focused lower-layer exile coverage already exists and should be named explicitly instead of implying exile is untested:
   `crates/worldwake-ai/src/candidate_generation.rs::justice_candidates_fall_back_to_exile_when_fine_is_not_locally_collectible`
   proves exile candidate generation when the accused's stock is not locally collectible, and
   `crates/worldwake-systems/src/justice_actions.rs::exile_removes_membership_adds_hostility_and_supersedes_exact_accusation`
   proves authoritative exile commitment.
5. Existing golden coverage is narrower than the draft claimed, but not absent. `crates/worldwake-ai/tests/golden_emergent.rs::run_witnessed_theft_accusation_chain` (Scenario 38) already proves the witnessed-theft -> Tell -> Accuse -> PunishAccused chain and is exile-aware in its assertion logic, but its setup is tuned for a fine-paying thief and it does not isolate or require exile as the outcome.
6. The live `GoalKind` under test is
   `GoalKind::PunishAccused { office, accused, accusation_entry, punishment: PunishmentKind::Exile { from_faction } }`.
   The prerequisite surface is: a recorded accusation in a known local `CrimeRegister`, believed office-holder authority, believed faction membership for the governed faction, and punishment-time local observation that fine is not collectible.
7. Golden E2E is still the correct verification layer for this ticket. The intended invariant is a cross-system chain:
   witnessed hidden theft -> witness social observation -> `tell` -> authority violation memory and accusation -> accusation record consultation -> exile candidate generation -> exile action commit.
   Focused/unit tests already cover the lower branch boundaries, but no golden currently proves the full cross-system exile chain end to end.
8. Scenario isolation must be explicit. The intended branch is the exile punishment fallback. Competing lawful branches include:
   Fine when the authority can still locally observe collectible stock, owner-local expectation-violation discovery, and unrelated AI goals on the thief or witness. The scenario should intentionally remove those branches from setup.
9. The draft's hostility wording was imprecise. `commit_exile()` calls `txn.add_hostility(from_faction, accused)`, and the established assertion surface is `world.hostile_towards(accused).contains(&from_faction)`. The ticket is corrected to that exact authoritative query.
10. The draft verification command naming was approximate. Current real test names were confirmed via `cargo test -p worldwake-ai --test golden_emergent -- --list`.
11. The live architecture remains cleaner than adding any special-case "poor thief => exile" rule. The durable branch boundary is local collectibility of the fined commodity, which composes with belief-local planning and avoids introducing wealth aliases or omniscient punishment heuristics.
12. Adjacent contradiction exposed during reassessment: `archive/specs/S32-crime-emergence-golden-suites.md` still narrates Scenario 41 as "zero commodities" and should be corrected in a separate doc/spec cleanup ticket if that narrative is intended to stay authoritative. This implementation ticket corrects only its own scope.

## Architecture Check

1. The clean architecture is to test the live collectibility rule directly, not to force a new "cannot pay because poor" abstraction into either AI or justice code. The current split is robust:
   AI selects Fine only from local observable commodity state; authoritative actions enforce exact office, record, and faction invariants; no new alias path is needed.
2. The scenario should isolate exile by arranging an explicit world-state change that leaves the accused punishable but not fine-collectible, instead of weakening production code or widening Scenario 38 into a mixed-contract test that tries to prove both fine and exile equally.
3. No backwards-compatibility aliasing or shims should be introduced. If the scenario needs a different setup than the S32 draft narrative, the ticket and test should describe the live setup plainly rather than preserving the stale story.

## Verification Layers

1. Witness learns and relays `SuspectedTheft` -> authoritative belief store + action trace for committed `tell`
2. Authority opens a concrete case -> authoritative `ViolationMemory` + `CrimeRegister` accusation entry
3. Exile candidate appears for the recorded case -> decision trace / goal history for `GoalKind::PunishAccused { punishment: Exile }`
4. Fine is not the selected punishment in the exile-isolated scenario -> decision trace and absence of committed `fine` in action trace
5. Exile executes lawfully -> action trace committed `exile`
6. Exile durable aftermath -> authoritative world state (`!factions_of(thief).contains(&faction)`, `hostile_towards(thief).contains(&faction)`)
7. Verdict records the chosen punishment -> authoritative `RecordData` read on `CrimeRegister`
8. Commodity conservation remains intact -> authoritative conservation helper (`verify_live_lot_conservation`)
9. Determinism of the full chain -> replay companion hash equality
10. Additional layers beyond these are unnecessary because focused lower-layer tests already prove the branch internals and the ticket does not need weaker event-log-only proxies.

## What to Change

### 1. Add a dedicated Scenario 41 golden

Add `run_exile_punishment_when_fine_is_not_locally_collectible` to `crates/worldwake-ai/tests/golden_emergent.rs`.

The scenario should follow the proven Scenario 38 witness/tell/accuse setup pattern, but with the punishment branch isolated for exile:
- witnessed theft still happens at `VillageSquare`
- witness still perceives the hidden theft and later tells the authority at `RulersHall`
- authority still files a concrete accusation in the `CrimeRegister`
- before punishment selection, the stolen commodity is no longer locally collectible from the accused even though the accusation remains valid
- the authority and accused are co-located for punishment so the exile action itself can lawfully commit
- assertions require exile specifically, not "fine or exile"

The scenario comment block should use Scenario 41 metadata in the generator format required by `docs/golden-e2e-testing.md`.

### 2. Add dedicated tests

Add:
- `golden_exile_punishment_when_fine_is_not_locally_collectible`
- `golden_exile_punishment_when_fine_is_not_locally_collectible_replays_deterministically`

These tests should be distinct from Scenario 38 rather than broadening Scenario 38 into a dual-contract test.

### 3. Preserve clean scope

Do not modify production AI or justice code unless the new golden exposes a real engine contradiction. The intended work is missing golden coverage for an already-implemented architectural path.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `tickets/S32CRIMEMEGOLSUI-001.md` (modify, then archive on completion)

## Out of Scope

- Changes to `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-systems/src/justice_actions.rs`, or other production crates unless the new golden proves a real contradiction
- Rewriting Scenario 38 into an exile-specific test
- Updating `archive/specs/S32-crime-emergence-golden-suites.md`
- Scenario 42 and Scenario 43
- Golden docs inventory refresh tickets beyond the source metadata added with the new scenario

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_exile_punishment_when_fine_is_not_locally_collectible`
2. `cargo test -p worldwake-ai --test golden_emergent golden_exile_punishment_when_fine_is_not_locally_collectible_replays_deterministically`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The scenario proves `PunishAccused(Exile)` through the live local-collectibility branch, not through a new special-case punishment rule.
2. The accused loses faction membership and becomes a hostility target of the governed faction after exile.
3. The `CrimeRegister` contains a verdict with `PunishmentKind::Exile { from_faction }` superseding the accusation.
4. No committed `fine` occurs in the exile-specific scenario.
5. Commodity totals remain conserved across the scenario.
6. Replay with the same seed yields identical `(world_hash, event_log_hash)`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs::golden_exile_punishment_when_fine_is_not_locally_collectible`
   Rationale: proves the full witnessed-theft -> Tell -> Accuse -> Exile chain at the golden E2E layer under the live punishment-selection contract.
2. `crates/worldwake-ai/tests/golden_emergent.rs::golden_exile_punishment_when_fine_is_not_locally_collectible_replays_deterministically`
   Rationale: proves the new exile golden is deterministic and suitable as stable regression coverage.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_exile_punishment_when_fine_is_not_locally_collectible`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-27
- Actual changes:
  - Added dedicated Scenario 41 coverage in `crates/worldwake-ai/tests/golden_emergent.rs` with:
    - `golden_exile_punishment_when_fine_is_not_locally_collectible`
    - `golden_exile_punishment_when_fine_is_not_locally_collectible_replays_deterministically`
  - Reassessed and corrected the ticket narrative from "accused has zero commodities" to the live architectural branch: fine is unavailable when the fined commodity is not locally collectible at punishment-selection time.
- Deviations from original plan:
  - The implemented scenario does not prove a generic "poor thief" branch, because that is not the live punishment contract.
  - The test isolates exile by removing local collectibility of the stolen bread before punishment selection while preserving the witnessed-theft -> Tell -> Accuse chain.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_emergent golden_exile_punishment_when_fine_is_not_locally_collectible` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
