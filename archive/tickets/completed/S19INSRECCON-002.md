# S19INSRECCON-002: Scenario 33 — Remote Record Travel + Consultation + Political Action

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test only
**Deps**: `specs/S19-institutional-record-consultation-golden-suites.md`; E16c (ConsultRecord action) — COMPLETED; E16d (political goldens) — COMPLETED; S19INSRECCON-001 assumptions already delivered in `golden_harness`

## Problem

No golden E2E test currently proves the full remote information-locality chain for institutional records: travel to a non-local office register, consult it to acquire vacancy knowledge, travel back to the office jurisdiction, then complete the political action chain. Existing office goldens either start with seeded institutional belief or receive the office fact through direct test seeding, so they do not cover the live ConsultRecord carrier path as a real travel-shaped prerequisite.

## Assumption Reassessment (2026-03-22)

1. The active S19 spec assigns Scenario 33, not Scenario 32, to `S19-002`. The previous ticket body described the wrong scenario. The corrected scope is the remote-record path defined in [`specs/S19-institutional-record-consultation-golden-suites.md`](/home/joeloverbeck/projects/worldwake/specs/S19-institutional-record-consultation-golden-suites.md).
2. The harness assumptions in the older draft are stale. `RULERS_HALL`, `seed_office_register()`, and `seed_office_vacancy_entry()` already exist in [`crates/worldwake-ai/tests/golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs), with focused harness tests proving the helper behavior.
3. Current focused planner coverage already proves the operator family. [`crates/worldwake-ai/src/search.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs) contains `search_political_goal_uses_consult_record_as_mid_plan_prerequisite_when_belief_unknown`, which returns `Travel -> ConsultRecord -> Travel -> DeclareSupport` for a single-edge prerequisite path. [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) also contains `consult_record_step_overrides_unknown_vacancy_belief_and_unblocks_declare_support`. The golden's live selected-plan trace is expected to be more concrete for this scenario because the outbound route to `RULERS_HALL` spans multiple travel legs.
4. The live `GoalKind` under test is `ClaimOffice { office }`, not `SupportCandidateForOffice`. The terminal authoritative action is still `declare_support`, but the golden should assert the selected `ClaimOffice` branch and its selected-plan shape rather than narrating the scenario as a supporter-goal case.
5. Full action registries are required. This is a golden E2E scenario in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs), so the existing `GoldenHarness` runtime is the correct boundary.
6. The authoritative closure boundary is mixed-layer and should stay split: selected AI plan includes `ConsultRecord` before `DeclareSupport`; action traces prove the execution order; authoritative world state proves the durable office-holder mutation. Per [`docs/golden-e2e-testing.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md), the later succession result must not be used as a proxy for earlier action ordering.
7. Current topology math differs from the stale spec narrative. In [`crates/worldwake-core/src/topology.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/topology.rs), `OrchardFarm -> EastFieldTrail` is 2 ticks, `EastFieldTrail -> SouthGate` is 3, `SouthGate -> VillageSquare` is 2, and `VillageSquare -> RulersHall` is 1. Outbound travel to the remote record is therefore 8 ticks, return travel to the jurisdiction is 1 tick, not the earlier 5+1 assumption.
8. ConsultRecord duration math is live in [`crates/worldwake-sim/src/action_semantics.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs): `floor(consultation_ticks * consultation_speed_factor / 1000)`, clamped to at least 1. With the harness defaults (`consultation_ticks = 4`) and `consultation_speed_factor = pm(500)`, consultation takes 2 ticks.
9. Scenario isolation remains single-agent and sated, but the current architecture lawfully creates one empty local office register when `seed_office()` creates the office. The remote scenario therefore must explicitly seed the vacancy entry into the `RULERS_HALL` register and leave the jurisdiction-local register empty so the prerequisite path still belongs to the remote record rather than a test-only local shortcut.
10. Existing golden coverage in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs) already covers a different remote-office contract: `golden_information_locality_for_political_facts` proves that a remote claimant does nothing until an explicit reported belief update arrives. This ticket is still distinct because it proves the record-consultation carrier path and the physical detour through the remote archive.
11. Mismatch + correction: the old ticket title, scenario number, helper assumptions, and travel math were all stale. The corrected ticket scope is a remote-record golden added to `golden_offices.rs`, with no harness work and no engine changes.
12. Reachability math under current code: 8 travel ticks outbound + 2 consult ticks + 1 travel tick back + 1 `declare_support` tick + 5 succession ticks = 17 ticks of expected critical-path work before margins. A 30-tick window is comfortably above the current minimum without depending on accidental scheduler timing.

## Architecture Check

1. The beneficial change is coverage, not architecture churn. Adding the golden strengthens the current causality-first design by proving that institutional knowledge can stay local to a remote record and still drive politics through lawful travel and consultation. That is cleaner than adding shortcut belief seeding or special-case planner aliases because it validates the existing world-state model directly.
2. No backward-compatibility aliasing or shim paths are introduced. The scenario should use the current helper surface and current political planner contract exactly as they exist.

## Verification Layers

1. Initial selected plan is a `ClaimOffice` branch whose step list contains the multi-hop outbound travel legs to `RULERS_HALL`, then `ConsultRecord`, then return travel to `VILLAGE_SQUARE`, then `DeclareSupport` -> decision trace.
2. Remote record consultation commits before support declaration commits -> action trace ordering via `(tick, sequence_in_tick)`.
3. Agent physically reaches the office jurisdiction and ends as office holder -> authoritative world state.
4. Deterministic replay of the scenario -> world hash + event-log hash comparison.
5. This ticket does not need request-resolution proof because the intended contract is lawful execution, not pre-start rejection or start-failure recovery.

## What to Change

### 1. Add a remote-record scenario builder and runner in `golden_offices.rs`

Create a dedicated scenario that:
- seeds one sated claimant at `ORCHARD_FARM`
- seeds a vacant support-law office at `VILLAGE_SQUARE`
- keeps the jurisdiction-local office register empty
- seeds a remote office register at `RULERS_HALL` and appends the vacancy entry there
- seeds the claimant's entity beliefs about the office and the remote record, but no office-holder institutional belief
- enables decision tracing and action tracing for the scenario runner

### 2. Add the primary and replay tests

Add:
- `golden_remote_record_consultation_political_action`
- `golden_remote_record_consultation_political_action_replays_deterministically`

The primary test should assert the selected-plan shape, action ordering, final location, and office-holder result. For the selected-plan shape, assert the concrete multi-hop route through `EastFieldTrail -> SouthGate -> VillageSquare -> RULERS_HALL` before the consult step instead of collapsing the route into one abstract travel step. The replay companion should use the standard non-trivial deterministic replay pattern already used in this file.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)

## Out of Scope

- Any engine or planner refactor
- Changes to `golden_harness/mod.rs`
- Changes to other office scenarios
- Documentation tickets `S19INSRECCON-004` and `S19INSRECCON-005`
- Multi-agent knowledge-race coverage (`S19INSRECCON-003`)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action`
2. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action_replays_deterministically`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The claimant starts without any seeded office-holder institutional belief; vacancy knowledge must come from ConsultRecord execution.
2. The selected AI plan must include the remote-record prerequisite path before `declare_support`, with the outbound route represented as concrete travel legs in the selected-plan trace.
3. `consult_record` must commit before `declare_support`.
4. The claimant must become office holder through the normal succession path.
5. Replay with the same seed must produce identical final world and event-log hashes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_offices.rs::golden_remote_record_consultation_political_action` — proves the remote record carrier path end to end: travel to `RULERS_HALL`, consult for vacancy knowledge, travel to `VILLAGE_SQUARE`, declare support, and win succession.
2. `crates/worldwake-ai/tests/golden_offices.rs::golden_remote_record_consultation_political_action_replays_deterministically` — proves the scenario replays deterministically and that the simulation is non-trivial.

### Commands

1. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action`
2. `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action_replays_deterministically`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-22
- Actual change:
  - Added `golden_remote_record_consultation_political_action`
  - Added `golden_remote_record_consultation_political_action_replays_deterministically`
  - Corrected the ticket itself before implementation because it was describing Scenario 32 instead of the live `S19-002` Scenario 33 contract.
- What changed versus the original stale plan:
  - No harness work was needed; `RULERS_HALL`, `seed_office_register()`, and `seed_office_vacancy_entry()` were already present.
  - The selected-plan assertion had to follow the live runtime surface: the outbound path appears as concrete multi-hop travel legs to `RULERS_HALL`, not one collapsed `Travel` step.
  - The travel math was corrected to the current prototype topology: 8 outbound travel ticks, 1 return travel tick, 2 consult ticks, 1 support tick, 5 succession ticks.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action` passed.
  - `cargo test -p worldwake-ai --test golden_offices golden_remote_record_consultation_political_action_replays_deterministically` passed.
  - `cargo test -p worldwake-ai` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
