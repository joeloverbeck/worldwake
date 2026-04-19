# S109TYPDISTAX-006: Final cleanup and scenario migration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — remove `BlockingFact::Unknown` and `AssumptionFailed` variants, remove `CognitiveProfile::unknown_block_ticks`, bump `SAVE_FORMAT_VERSION`, update 14 RON files and ~20 Rust literal sites
**Deps**: archive/tickets/S109TYPDISTAX-004.md, archive/tickets/S109TYPDISTAX-005.md

## Problem

After T004 migrates all emission to the new classifier and T005 replaces the diagnostic trace, the `BlockingFact::Unknown` and `BlockingFact::AssumptionFailed` variants no longer have any runtime producer — they survive only in test fixtures, the TTL function, and a handful of match arms. This cleanup ticket removes them along with `CognitiveProfile::unknown_block_ticks` (which loses its last consumer when the TTL arm is deleted), migrates the 14 scenario RON files and ~20 Rust literal construction sites that currently declare the field, bumps `SAVE_FORMAT_VERSION` per FND-28 (old saves are not decodable), and lands the golden test extension (Validation test 9 from the spec).

## Assumption Reassessment (2026-04-19)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Current state after T004/T005: `BlockingFact::Unknown` and `AssumptionFailed` variants still exist at `crates/worldwake-core/src/blocker_memory.rs` (post-T001 rename; line numbers for the enum will have drifted). `CognitiveProfile::unknown_block_ticks` at `crates/worldwake-core/src/cognitive_profile.rs:28` still declared. `blocking_fact_ttl` at `crates/worldwake-ai/src/failure_handling.rs:992` still has arms for both variants. Runtime emission of either variant is zero (T004 proof via invariant check). Test-only references remain at the sites enumerated below. Existing focused tests that assert `BlockingFact::Unknown`/`AssumptionFailed` end up in memory: `blocker_memory.rs` test module `is_blocked_matches_only_live_entries_for_goal_key` (uses `BlockingFact::NoKnownPath`, not affected), `expire_removes_entries_at_or_before_current_tick` (line 402 uses `Unknown`), `sweep_cleared_removes_matching_entries` (line 669 uses `Unknown`), `assumption_failed_blocks_goal_generation` (line 645–657 uses `AssumptionFailed`); `failure_handling.rs` test module sites at lines 1673, 1804, 1812, 2094, 2455, 2531, 2539, 2604, 2678; `agent_tick/tests.rs` at 4124, 6006, 6036; `candidate_generation.rs` at 8005, 15412, 16170; `search/tests.rs` at 2323, 2340. `blocking_fact_ttl_uses_budget_classification`, `unknown_blocker_uses_dedicated_ttl`, and `transient_blockers_unchanged_ttl` tests exercise the TTL function directly and must be updated/removed because `Unknown` is no longer a valid input.
2. Scenario RON files that declare `unknown_block_ticks` (14 total, verified 2026-04-19):
   - `scenarios/survival-baseline.ron` lines 92, 206, 320 (3 occurrences).
   - `scenarios/survival-scattered.ron` lines 104, 219, 334 (3 occurrences).
   - `scenarios/survival-contested.ron` lines 126, 241, 356, 471 (4 occurrences).
   - `scenarios/cli-evaluation.ron` line 191 (1 occurrence).
   - `crates/worldwake-cli/tests/fixtures/observer_anomalies/acute_thirst_spike.ron` line 75 (1 occurrence).
   - `crates/worldwake-cli/tests/fixtures/observer_anomalies/convergence_hub.ron` lines 75, 189, 303 (3 occurrences).
   - `crates/worldwake-cli/tests/fixtures/observer_anomalies/maintenance_starvation_wash_gap.ron` lines 83, 197 (2 occurrences).
   - `crates/worldwake-cli/tests/fixtures/observer_anomalies/recipe_monoculture_apples_vs_grain.ron` lines 75, 189 (2 occurrences).
   - `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_wash_travel_cycle.ron` line 77 (1 occurrence).
3. Shared abstraction boundary: `BlockingFact` enum variants and `CognitiveProfile::unknown_block_ticks` field. Removal is atomic within this ticket — workspace must compile at each intermediate commit if split, but since variant removal and field removal both cascade across many files, this ticket treats the full removal as a single unit. `SAVE_FORMAT_VERSION` bumps from 32 to 33 at `crates/worldwake-sim/src/save_load.rs:6` because the serialized representation of `BlockingFact` and `CognitiveProfile` both change.
13. Adjacent contradiction: `blocking_fact_ttl` currently buckets `AssumptionFailed` into `structural_block_ticks` at `failure_handling.rs:1009`. After removal, that arm is deleted. Similarly, `unknown_block_ticks` loses its consumer (`failure_handling.rs:999`). Both removals are required consequences of this cleanup, not separate bugs.
15. Cumulative-state implication: `SAVE_FORMAT_VERSION = 32` means any save file serialized before this ticket cannot be decoded after the ticket lands. Per FND-28, this is acceptable — Worldwake does not preserve backwards-compatibility across authority-path changes. The ticket bumps to version 33 so loaders correctly reject old saves with a typed error rather than a cryptic deserialization failure.

## Architecture Check

1. Removing the two variants and the field in a single ticket preserves the "workspace builds after each ticket" invariant only when all call sites are updated together. Splitting this into smaller tickets (e.g., "remove field only" then "remove variants only") would either leave the workspace broken mid-sequence or require multiple rounds of `#[allow(dead_code)]` — FND-28 forbids such shims. Therefore this is one Large ticket.
2. No backwards-compatibility aliasing. `Unknown` and `AssumptionFailed` disappear from `BlockingFact`. `unknown_block_ticks` disappears from `CognitiveProfile`. No deprecated stubs, no alias modules. Old saves fail decode with a typed version-mismatch error. FND-28 compliant.

## Verification Layers

1. Variant removal correctness → compile-time proof. Every match on `BlockingFact` is exhaustive; removing two variants forces every match site to drop its `Unknown` / `AssumptionFailed` arms. Missing an arm = compile error.
2. TTL function correctness → focused unit test: `blocking_fact_ttl` for the remaining 15 variants returns the expected bucket; no test input for `Unknown` or `AssumptionFailed` because they no longer exist.
3. Scenario deserialization → RON round-trip tests per migrated scenario file: `scenarios/*.ron` parses cleanly under the new `CognitiveProfile` shape. Existing scenario-loading tests in `crates/worldwake-cli/src/scenario/` cover this; any pre-existing scenario-smoke test is the proof surface.
4. Save-format rejection of v32 files → focused test: attempt to load a byte buffer with `SAVE_FORMAT_VERSION = 32` magic; assert `SaveError::VersionMismatch { expected: 33, actual: 32 }`.
5. Golden extension (Validation test 9) → golden E2E coverage at `crates/worldwake-ai/tests/golden_planner_pathology.rs` or `golden_ai_decisions.rs`: after a target-gone replan, `BlockerMemory` contains a `BlockingFact::TargetGone` entry; after a belief-contradiction replan, `DiscrepancyMemory` contains a `Discrepancy::BeliefContradicted` entry keyed on the relevant `BlockerKey`.

## What to Change

### 1. Remove `BlockingFact::Unknown` and `AssumptionFailed`

In `crates/worldwake-core/src/blocker_memory.rs` (post-T001), delete the two variants from the `BlockingFact` enum at lines ~189–211. The enum now has 15 variants instead of 17.

Update every remaining exhaustive match on `BlockingFact` across the workspace. The compiler enumerates all such sites; expected set:

- `failure_handling.rs::blocking_fact_ttl` — remove the `Unknown => unknown_block_ticks` arm (line 999) and the `AssumptionFailed` arm inside the structural bucket (line 1009).
- `failure_handling.rs::derive_clearing_condition` — remove `Unknown` and `AssumptionFailed` from the `TtlOnly` arm (line 745–748); the remaining arm covers `PatienceExhausted | NoBuyer`.
- `failure_handling.rs::is_blocker_cleared` (line 752) — if it contains an `Unknown | AssumptionFailed` match arm, delete.
- `failure_handling.rs::Blocker::blocks_goal_generation` (now in `blocker_memory.rs` post-T001) — the `!matches!(..., ExclusiveFacilityUnavailable | SourceDepleted)` expression is unaffected because it negates a specific subset; no update needed unless `Unknown` or `AssumptionFailed` was included (verify during implementation).
- Any observer/CLI rendering that matched on specific variants.

### 2. Remove `CognitiveProfile::unknown_block_ticks`

In `crates/worldwake-core/src/cognitive_profile.rs`:

- Remove the `pub unknown_block_ticks: u32,` field at line 28.
- Remove `unknown_block_ticks: 5,` from the `Default` impl at line 59.
- Update the `cognitive_profile_default_matches_split_defaults` test (line 102) to drop the `unknown_block_ticks` assertion.
- Update the `cognitive_profile_roundtrips_through_bincode` test (line 127) to drop the field from the sample profile.

### 3. Update `blocking_fact_ttl`

The function now covers 15 `BlockingFact` variants split across two buckets:

```rust
fn blocking_fact_ttl(fact: BlockingFact, cognitive: &CognitiveProfile) -> u32 {
    match fact {
        BlockingFact::SellerOutOfStock
        | BlockingFact::WorkstationBusy
        | BlockingFact::ReservationConflict
        | BlockingFact::ExclusiveFacilityUnavailable
        | BlockingFact::TargetGone => cognitive.transient_block_ticks,
        BlockingFact::NoKnownPath
        | BlockingFact::NoKnownSeller
        | BlockingFact::TooExpensive
        | BlockingFact::SourceDepleted
        | BlockingFact::MissingTool(_)
        | BlockingFact::MissingInput(_)
        | BlockingFact::DangerTooHigh
        | BlockingFact::CombatTooRisky
        | BlockingFact::PatienceExhausted
        | BlockingFact::NoBuyer => cognitive.structural_block_ticks,
    }
}
```

No `Unknown` arm, no `AssumptionFailed` arm.

### 4. Migrate scenario RON files

In all 14 RON files listed in the reassessment, delete the `unknown_block_ticks: N,` line from each `CognitiveProfile { ... }` block. Do not replace with a different field; the new TTL fields from T003 all carry `#[serde(default)]` and will fall back to their documented defaults.

Optionally, for scenarios that want to override new TTL fields (the spec notes this is the natural place for per-agent personality tuning), authors can add explicit field values — but this ticket does not change semantics, so each scenario just drops the old field.

### 5. Migrate Rust literal sites

At each of the following sites, delete the `unknown_block_ticks: N,` line from the `CognitiveProfile { ... }` literal (per T003's Section 4, these sites already list all fields explicitly):

- `crates/worldwake-core/src/cognitive_profile.rs` test module.
- `crates/worldwake-core/src/delta.rs:582`.
- `crates/worldwake-ai/src/failure_handling.rs:1375`.
- `crates/worldwake-ai/src/decision_runtime.rs:358`.
- `crates/worldwake-ai/src/agent_tick/planning.rs:1382`.
- `crates/worldwake-ai/src/agent_tick/tests.rs:105`.
- `crates/worldwake-ai/src/goal_model.rs:2590`.
- `crates/worldwake-ai/src/search/tests.rs:60`.
- `crates/worldwake-ai/src/lib.rs:132, 150` (the `PlanningBudget::unknown_block_ticks` field on that struct — verify whether it mirrors `CognitiveProfile::unknown_block_ticks` or is independent; if mirror, remove; if independent, leave and note in ticket's classification).
- `crates/worldwake-cli/src/scenario/types.rs:939`.

Verify after editing: `grep -rn "unknown_block_ticks" crates/ scenarios/ docs/profiles/` returns zero matches (aside from historical entries in `docs/profiles/all-profiles.md` which is regenerated from the profile source and will drop the entry automatically).

### 6. Update `docs/profiles/all-profiles.md`

The profile documentation at `docs/profiles/all-profiles.md:85` lists `unknown_block_ticks`. Either regenerate from the updated `CognitiveProfile` source (if the docs are generated) or manually remove the row and add rows for the 9 new TTL fields from T003.

### 7. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:6`, change `pub const SAVE_FORMAT_VERSION: u32 = 32;` to `33`. Update the existing focused test `save_format_rejects_wrong_version` (or equivalent; grep the file's `#[cfg(test)]` block to name it accurately) to assert that a v32 buffer is rejected with `SaveError::VersionMismatch { expected: 33, actual: 32 }`.

### 8. Update tests that reference removed variants

Every test site listed in Assumption Reassessment item 1 must be updated:

- `blocker_memory.rs` test module (post-T001 rename) — tests at ex-lines 402, 645–657, 669 reference `Unknown` or `AssumptionFailed`. Replace `BlockingFact::Unknown` with `BlockingFact::NoKnownPath` (or another concrete variant the test does not otherwise care about) OR convert the test to exercise `DiscrepancyMemory` with `Discrepancy::ImproperPlanningState`. The `assumption_failed_blocks_goal_generation` test loses its contract — delete.
- `failure_handling.rs` test module sites — most assert against the old `derive_blocking_fact` return value. After T004 these already became `classify_discrepancy` assertions; any residual `BlockingFact::Unknown`/`AssumptionFailed` references in test fixtures are either replaced with a specific surviving variant or converted to `Discrepancy::*` assertions on `DiscrepancyMemory`.
- `agent_tick/tests.rs` sites — same pattern.
- `candidate_generation.rs` test sites (8005, 15412, 16170) — the test at 8005 currently seeds `BlockerMemory` with `BlockingFact::AssumptionFailed` to verify candidate suppression; convert to seeding `DiscrepancyMemory` with `Discrepancy::BeliefContradicted`.
- `search/tests.rs` sites (2323, 2340) — same.
- `blocking_fact_ttl_uses_budget_classification` test (failure_handling.rs:2519) — drop the `Unknown` assertion.
- `unknown_blocker_uses_dedicated_ttl` test (failure_handling.rs:2537) — delete entirely; its contract no longer exists.

### 9. Golden test extension (Validation test 9)

In `crates/worldwake-ai/tests/golden_planner_pathology.rs` (or `golden_ai_decisions.rs`, whichever has an existing target-gone replan scenario), extend an existing scenario or add a new test case:

- After a target-gone replan, assert that `world.get_component_blocker_memory(agent)` has an entry with `blocking_fact: BlockingFact::TargetGone` for the relevant `BlockerKey`.
- For a belief-contradiction scenario (target identity mismatch, claim stale), assert that `world.get_component_discrepancy_memory(agent)` has an entry with `discrepancy: Discrepancy::BeliefContradicted` for the relevant `BlockerKey`.

Use `docs/generated/golden-e2e-inventory.md` to locate the closest existing test and add assertions there; avoid creating a brand-new test file if an existing one already exercises the replan behavior.

## Files to Touch

- `crates/worldwake-core/src/blocker_memory.rs` (modify — remove 2 variants from `BlockingFact`; update test module)
- `crates/worldwake-core/src/cognitive_profile.rs` (modify — remove field + default + test references)
- `crates/worldwake-core/src/delta.rs` (modify — remove field from literal at line 582)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — update `blocking_fact_ttl`, `derive_clearing_condition`, `is_blocker_cleared`; update test module)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — line 358)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — line 1382)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — line 105 + test sites 4124, 6006, 6036)
- `crates/worldwake-ai/src/goal_model.rs` (modify — line 2590)
- `crates/worldwake-ai/src/search/tests.rs` (modify — line 60 + test sites 2323, 2340)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — test sites 8005, 15412, 16170)
- `crates/worldwake-ai/src/lib.rs` (modify — lines 132, 150; verify `PlanningBudget::unknown_block_ticks`)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — line 939)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify — line 309 references `cognitive.unknown_block_ticks` for display)
- `crates/worldwake-sim/src/save_load.rs` (modify — version bump)
- `scenarios/survival-baseline.ron` (modify — 3 lines)
- `scenarios/survival-scattered.ron` (modify — 3 lines)
- `scenarios/survival-contested.ron` (modify — 4 lines)
- `scenarios/cli-evaluation.ron` (modify — 1 line)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/acute_thirst_spike.ron` (modify — 1 line)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/convergence_hub.ron` (modify — 3 lines)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/maintenance_starvation_wash_gap.ron` (modify — 2 lines)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/recipe_monoculture_apples_vs_grain.ron` (modify — 2 lines)
- `crates/worldwake-cli/tests/fixtures/observer_anomalies/stuck_detector_wash_travel_cycle.ron` (modify — 1 line)
- `docs/profiles/all-profiles.md` (modify — remove/regenerate)
- `crates/worldwake-ai/tests/golden_planner_pathology.rs` or `golden_ai_decisions.rs` (modify — Validation test 9 assertions)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerate via `python3 scripts/golden_inventory.py --write --check-docs` after the golden test extension)

## Out of Scope

- No new emission-site changes (T004 already did that work).
- No new diagnostic-trace changes (T005 already did that work).
- No new belief-view accessors or TTL fields (T003 already did that work).
- No further migration of `BlockerMemory` API (T001 handled the rename).
- No behavioral changes to `Discrepancy` variants or `BlockerClearingCondition`.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core blocker_memory cognitive_profile` — variant removal + field removal tests pass.
2. `cargo test -p worldwake-ai failure_handling` — updated TTL and classification tests pass.
3. `cargo test -p worldwake-ai golden` — all goldens pass; Validation test 9 extension asserts typed memory entries.
4. `cargo test -p worldwake-cli scenario` — all scenario RON files parse cleanly.
5. `cargo test -p worldwake-sim save_load` — v32 save buffers rejected with `VersionMismatch { expected: 33, actual: 32 }`.
6. Full workspace: `cargo test --workspace`.

### Invariants

1. `grep -rn "BlockingFact::Unknown\|BlockingFact::AssumptionFailed\|unknown_block_ticks" crates/ scenarios/ docs/profiles/` returns zero matches (except inside archived tickets/specs, which live under `archive/`).
2. Every scenario RON file under `scenarios/` and `crates/worldwake-cli/tests/fixtures/` still deserializes and runs its golden/smoke test successfully.
3. `SAVE_FORMAT_VERSION = 33`. Loading a v32 buffer produces `SaveError::VersionMismatch`.
4. Golden test for Validation test 9 asserts typed entries in the correct memory (BlockerMemory for target-gone, DiscrepancyMemory for belief-contradicted).
5. `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
6. Determinism preserved: no new `HashMap`/`HashSet` in authoritative state, no floats introduced, no wall-clock reads.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/blocker_memory.rs` `#[cfg(test)]` — delete `assumption_failed_blocks_goal_generation`; update `expire_removes_entries_at_or_before_current_tick` and `sweep_cleared_removes_matching_entries` to use surviving variants.
2. `crates/worldwake-core/src/cognitive_profile.rs` `#[cfg(test)]` — update default-match and bincode-roundtrip tests to drop `unknown_block_ticks`.
3. `crates/worldwake-ai/src/failure_handling.rs` `#[cfg(test)]` — delete `unknown_blocker_uses_dedicated_ttl`; update `blocking_fact_ttl_uses_budget_classification` and `transient_blockers_unchanged_ttl` to match the new match arms.
4. `crates/worldwake-ai/src/candidate_generation.rs` `#[cfg(test)]` — convert test sites 8005/15412/16170 to seed `DiscrepancyMemory` with `Discrepancy::BeliefContradicted` or `ImproperPlanningState` instead of `BlockerMemory` with `AssumptionFailed`/`Unknown`.
5. `crates/worldwake-ai/src/search/tests.rs` — same pattern for sites 2323/2340.
6. `crates/worldwake-ai/src/agent_tick/tests.rs` — same pattern for sites 4124/6006/6036.
7. `crates/worldwake-sim/src/save_load.rs` `#[cfg(test)]` — update version-mismatch test to assert new version numbers.
8. `crates/worldwake-ai/tests/golden_planner_pathology.rs` (or `golden_ai_decisions.rs`) — Validation test 9 extension: assertions on `BlockerMemory` and `DiscrepancyMemory` after target-gone and belief-contradiction replans respectively.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-ai failure_handling`
3. `cargo test -p worldwake-ai golden`
4. `cargo test -p worldwake-cli scenario`
5. `cargo test -p worldwake-sim save_load`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo test --workspace`
8. `python3 scripts/golden_inventory.py --write --check-docs` (regenerate golden inventory docs after the golden extension)
