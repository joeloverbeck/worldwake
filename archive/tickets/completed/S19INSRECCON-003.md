# S19INSRECCON-003: Scenario 34 — Knowledge Asymmetry Race

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test only
**Deps**: S19INSRECCON-001 (harness helpers) — COMPLETED; S19INSRECCON-002 (Scenario 33 remote-record golden pattern) — COMPLETED; E16c (institutional beliefs, ConsultRecord, record entities) — COMPLETED; E16d (political planning / support succession) — COMPLETED

## Problem

`specs/S19-institutional-record-consultation-golden-suites.md` assigns `S19-003` to Scenario 34, the missing knowledge-asymmetry race. The current ticket was stale: it described Scenario 33 (remote record travel), but that scenario is already implemented in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs#L1140) as `golden_remote_record_consultation_political_action` and `golden_remote_record_consultation_political_action_replays_deterministically`.

What is still missing is a golden that proves two co-located, otherwise similar agents diverge because one already has certain office-vacancy knowledge while the other must spend authoritative time consulting the record first. That is the architecture-level gap: institutional knowledge must be a real competitive resource, not only a planner convenience.

## Assumption Reassessment (2026-03-22)

1. The original ticket scope was wrong. Scenario 33 is already delivered in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs#L1140). `S19INSRECCON-003` must be corrected to Scenario 34 to match [`specs/S19-institutional-record-consultation-golden-suites.md`](/home/joeloverbeck/projects/worldwake/specs/S19-institutional-record-consultation-golden-suites.md).
2. The required harness helpers already exist. [`seed_office_register()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs#L752) and [`seed_office_vacancy_entry()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs#L823) are present, tested, and sufficient. No harness work is needed in this ticket.
3. The live planner contract still matches the spec: unknown office-holder belief with a consultable record produces a `ConsultRecord` prerequisite before `DeclareSupport`, while certain vacancy belief skips it. That is already covered in focused tests at [`search_political_goal_uses_consult_record_as_mid_plan_prerequisite_when_belief_unknown()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs#L5331) and [`search_political_goal_skips_consult_record_when_vacancy_belief_is_already_certain()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs#L5449).
4. The `GoalKind` under test is `ClaimOffice`. The race is not about candidate generation absence; both agents should lawfully generate `ClaimOffice`. The divergence is in selected plan shape: informed agent selects `DeclareSupport`, uninformed agent selects `ConsultRecord -> DeclareSupport`.
5. `consultation_speed_factor: pm(500)` shortens consultation to 50% of base duration in the live engine, not 200%. The authoritative duration formula is in [`consultation_duration_ticks()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs#L268): `floor(consultation_ticks * factor / 1000)`, clamped to at least 1 tick.
6. Because of that live formula, the record used by this scenario must explicitly raise `consultation_ticks`. Using the harness default of `4` would yield a 2-tick consult, which is too short to prove the intended race. A remote helper default is not enough here.
7. The authoritative succession contract does not reward “who declared first” directly. Support-law succession evaluates the current declaration counts when the vacancy timer matures, and ties reset the vacancy clock in [`resolve_support_succession()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs#L214). The scenario therefore needs concrete setup math that ensures the uninformed agent cannot submit a competing declaration before the first lawful evaluation installs the informed agent.
8. With office `succession_period_ticks: 5`, `vacancy_since: Tick(0)`, `declare_support` taking 1 tick, and the uninformed agent’s record set to `consultation_ticks: 12` with `consultation_speed_factor: pm(500)`, the uninformed consult lasts 6 ticks. That makes `ConsultRecord -> DeclareSupport` miss the first support-law evaluation window while the informed agent can declare immediately.
9. This is a golden E2E ticket. The correct verification surfaces are mixed-layer:
   - decision trace for initial candidate/plan divergence
   - action trace for lifecycle ordering (`declare_support` vs `consult_record`)
   - authoritative world state for final office holder
10. Scenario isolation must be explicit. Both agents should be sated, co-located at the office jurisdiction, and free of unrelated economic/combat/social branches. The decisive asymmetry must be institutional knowledge state plus authoritative consultation duration.

## Architecture Check

1. The corrected scope is more valuable than the stale Scenario 33 duplicate because it covers a different architectural promise: knowledge asymmetry creates real competitive outcomes without adding any special-case race logic.
2. The clean architecture is to exercise the existing systems exactly as they stand: seeded certain institutional belief for one agent, `Unknown` plus a consultable record for the other, then let planner selection, action duration, and support-law succession interact through world state. No aliases, no shortcut helpers, no “race resolution” shim.
3. Reusing the established `build_*` + `run_*` + deterministic replay pattern in [`golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs) is preferable to inventing a new harness abstraction. The scenario is a new coverage slice, not a new testing architecture.
4. If reassessment during implementation shows the live succession timing cannot express this race cleanly under current authoritative rules, the right response is to correct the ticket again, not to weaken production behavior or smuggle a test-only shortcut into the harness.

## Verification Layers

1. Both agents generate `ClaimOffice` despite different knowledge states -> decision trace
2. Informed agent selects direct `DeclareSupport` plan; uninformed agent selects `ConsultRecord -> DeclareSupport` plan -> decision trace
3. Informed agent commits `declare_support` before uninformed agent finishes `consult_record` -> action trace ordering via `(tick, sequence_in_tick)`
4. Uninformed agent does not commit `declare_support` before the office is installed -> action trace + authoritative world state
5. Final holder is the informed agent -> authoritative world state (`world.office_holder(office)`)
6. Replay remains deterministic -> world hash + event-log hash comparison

## What to Change

### 1. Add `build_knowledge_asymmetry_race_scenario()` in `golden_offices.rs`

Setup function creating:
- Two sated co-located agents at `VILLAGE_SQUARE` with high `enterprise_weighted_utility(pm(800))`
- Perception profiles on both agents with `consultation_speed_factor: pm(500)`
- Vacant support-law office at `VILLAGE_SQUARE` with `succession_period_ticks: 5`
- Local `OfficeRegister` at `VILLAGE_SQUARE` with a seeded vacancy entry and `consultation_ticks` raised to `12`
- Agent A (“informed”) seeded with `Certain(None)` office-holder belief
- Agent B (“uninformed”) seeded with entity beliefs about the office and record, but no office-holder institutional belief (`Unknown`)
- No rival needs, loyalties, food pressure, combat pressure, or travel branches

### 2. Add `run_knowledge_asymmetry_race()` function

Run enough ticks for:
- Agent A to declare support
- Agent B to spend authoritative time on `consult_record`
- Support-law succession to install the winner

Assert:
1. decision traces show both `ClaimOffice` candidates
2. informed selected plan omits `ConsultRecord`
3. uninformed selected plan includes `ConsultRecord -> DeclareSupport`
4. informed `declare_support` commit occurs before uninformed `consult_record` commit completes or before any uninformed `declare_support` commit
5. authoritative final holder is Agent A
6. replay hashes returned for deterministic companion

### 3. Add primary test

`golden_knowledge_asymmetry_race_informed_wins_office`

### 4. Add replay companion

`golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically`

## Files to Touch

- `tickets/S19INSRECCON-003.md` (this reassessment)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)

## Out of Scope

- No engine behavior changes
- No harness changes in `golden_harness/mod.rs`
- No duplicate Scenario 33 work
- No topology changes
- No documentation updates outside the ticket archival outcome

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office`
2. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The informed agent starts with `InstitutionalBeliefRead::Certain(None)` for the office and therefore does not need `ConsultRecord`
2. The uninformed agent starts with `InstitutionalBeliefRead::Unknown` and therefore does need `ConsultRecord`
3. Both agents remain lawful `ClaimOffice` candidates; the divergence is prerequisite path shape, not candidate suppression
4. Consultation duration is authoritative world state on the record, not a test-only artificial delay
5. The informed agent becomes office holder before the uninformed agent can lawfully convert consulted knowledge into a competing declaration
6. Determinism holds for repeated runs with the same seed

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_offices.rs::golden_knowledge_asymmetry_race_informed_wins_office`
   Rationale: proves the live architecture treats institutional knowledge plus consultation duration as a real competitive advantage in a support-law office race.
2. `crates/worldwake-ai/tests/golden_offices.rs::golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically`
   Rationale: proves the new multi-agent political race remains deterministic under replay.

### Commands

1. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office`
2. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-22
- What actually changed:
  - corrected the stale ticket scope from already-implemented Scenario 33 to the missing Scenario 34 knowledge-asymmetry race
  - added `golden_knowledge_asymmetry_race_informed_wins_office`
  - added `golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically`
  - implemented the scenario in `golden_offices.rs` using the live local office register, authoritative `consultation_ticks = 12`, decision-trace plan-shape assertions, action-trace ordering assertions, and authoritative office-holder verification
- Deviations from original plan:
  - the original ticket narrative was not implemented because Scenario 33 was already present in the codebase
  - no harness or engine changes were needed after reassessment
- Verification results:
  - `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
