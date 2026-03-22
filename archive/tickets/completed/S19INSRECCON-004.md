# S19INSRECCON-004: Scenario 34 — Knowledge Asymmetry Race — Informed Agent Outpaces Consulting Agent

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — existing golden coverage already implements the scenario; this ticket now tracks reassessment, verification, and archival
**Deps**: S19INSRECCON-001 (harness helpers); S19INSRECCON-002 (establishes ConsultRecord golden pattern)

## Problem

The original gap was valid: Worldwake needed a golden proving that institutional knowledge can create a real competitive advantage through lawful consultation duration cost, not just through geography or seeded ranking asymmetry.

The ticket itself is now stale on scope. Live code already contains Scenario 34 and its replay companion in [crates/worldwake-ai/tests/golden_offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs). The remaining work is to reassess that implementation against current planner/runtime/politics contracts, verify it still passes at the intended layers, and archive the ticket accurately.

## Assumption Reassessment (2026-03-22)

1. The planner assumptions are still live. `search_political_goal_uses_consult_record_as_mid_plan_prerequisite_when_belief_unknown` and `search_political_goal_skips_consult_record_when_vacancy_belief_is_already_certain` in [crates/worldwake-ai/src/search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs) still prove the branch split this scenario depends on: `Unknown` inserts `ConsultRecord`, `Certain(None)` skips it.
2. The duration arithmetic is also still live. `consultation_duration_ticks()` in [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) computes `floor(consultation_ticks * consultation_speed_factor / 1000)` and clamps the result to at least 1 tick. With the harness default `consultation_ticks: 4` and `consultation_speed_factor: pm(500)`, consultation is 2 ticks, so Scenario 34 correctly overrides the consulted record to `consultation_ticks: 12`.
3. The ticket's main discrepancy is implementation status, not architecture. The live repo already contains `build_knowledge_asymmetry_race_scenario()` at [crates/worldwake-ai/tests/golden_offices.rs:1450](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs#L1450), `run_knowledge_asymmetry_race()` at [crates/worldwake-ai/tests/golden_offices.rs:1583](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs#L1583), and both Scenario 34 tests at [crates/worldwake-ai/tests/golden_offices.rs:1759](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs#L1759) and [crates/worldwake-ai/tests/golden_offices.rs:1764](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs#L1764).
4. The live `GoalKind` under test is `ClaimOffice { office }` for both agents. The informed claimant selects `DeclareSupport`; the uninformed claimant selects `ConsultRecord -> DeclareSupport`. That matches the intended operator surface for the scenario.
5. The decisive asymmetry is still knowledge state plus authoritative record duration, not ranking asymmetry. Both agents are sated, co-located at `VILLAGE_SQUARE`, and use identical `enterprise_weighted_utility(pm(800))`.
6. The closure boundary is explicitly multi-layer in the live golden: decision trace proves plan-shape divergence, action trace proves early `declare_support` versus late `consult_record`, politics trace proves the support-law install, and authoritative world state proves the final office holder.

## Architecture Check

1. The current design is already the clean architecture for this behavior. The decisive delay lives in authoritative `RecordData.consultation_ticks`, so the golden proves a world-state contract rather than relying on test-only scheduler tricks or planner overrides.
2. No new production abstraction is justified. The existing planner, action semantics, record action, and politics trace surfaces are sufficient and compose cleanly.
3. If verification reveals a real gap, the fix should stay minimal and preserve the existing authoritative-source-of-truth design around record state and support-law succession. No alias paths or backward-compatibility layers should be introduced.

## Verification Layers

1. Selected plan shape differs by knowledge state -> decision trace (`SelectedPlanSource::SearchSelection`, informed `DeclareSupport`, uninformed `ConsultRecord -> DeclareSupport`)
2. Informed claimant acts before the uninformed consult branch completes -> action trace ordering on committed `declare_support` versus committed `consult_record`
3. Support-law installation resolves in favor of the informed claimant -> politics trace (`OfficeSuccessionOutcome::SupportInstalled`)
4. Durable office ownership ends with the informed claimant -> authoritative world state (`world.office_holder(office) == Some(informed_agent)`)
5. Replay remains deterministic -> state-hash replay companion

## What to Change

No net-new Scenario 34 implementation is currently required. The remaining scope is:

1. Verify the existing Scenario 34 tests against current code.
2. Run the broader suites required by the ticket.
3. Add or strengthen tests only if verification exposes a real missing invariant.
4. Mark the ticket completed and archive it with an accurate outcome.

## Files to Touch

- `tickets/S19INSRECCON-004.md` (reassess and finalize)
- `crates/worldwake-ai/tests/golden_offices.rs` only if verification exposes a real gap

## Out of Scope

- No speculative refactor of political planning, action semantics, or record architecture
- No changes to `golden_harness/mod.rs` unless verification proves Scenario 34 is impossible to validate cleanly without it
- No remote-record Scenario 33 work here
- No documentation updates owned by S19INSRECCON-005
- No backward-compatibility shims, aliases, or duplicate scenario helpers

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office`
2. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The informed claimant must not need `ConsultRecord` because it starts with `Certain(None)` office-holder belief.
2. The uninformed claimant must require `ConsultRecord` before `DeclareSupport` because it starts with `Unknown` office-holder belief.
3. The informed claimant must win the office because the explicit record-duration setup makes the consult branch too slow to join the first support-law succession resolution window.
4. Both agents must remain equal on location, need state, and enterprise utility so knowledge state plus authoritative consultation duration remains the decisive asymmetry.
5. Replay with the same seed must produce identical state hashes.

## Test Plan

### New/Modified Tests

1. None expected during reassessment unless verification exposes a real gap.
2. Existing test: `crates/worldwake-ai/tests/golden_offices.rs::golden_knowledge_asymmetry_race_informed_wins_office` — proves knowledge asymmetry plus authoritative record duration yields the office outcome.
3. Existing test: `crates/worldwake-ai/tests/golden_offices.rs::golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically` — proves replay determinism for the same scenario.

### Commands

1. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office`
2. `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-22
- What actually changed: corrected the ticket to match the live repository state. Scenario 34 was already implemented in [crates/worldwake-ai/tests/golden_offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs), so the real work here was reassessment plus verification, not new production or test implementation.
- Deviations from original plan: the original ticket claimed the scenario still needed to be added. Reassessment showed the scenario builder, runtime assertions, primary golden, and replay companion were already present and aligned with the intended architecture.
- Verification results:
- `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office` passed
- `cargo test -p worldwake-ai --test golden_offices golden_knowledge_asymmetry_race_informed_wins_office_replays_deterministically` passed
- `cargo test -p worldwake-ai` passed
- `cargo test --workspace` passed
- `cargo clippy --workspace --all-targets -- -D warnings` passed
