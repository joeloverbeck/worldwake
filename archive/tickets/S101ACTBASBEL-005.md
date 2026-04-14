# S101ACTBASBEL-005: Soak golden profile migration for inventory refresh

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: tickets/S101ACTBASBEL-003.md

## Problem

`S101ACTBASBEL-003` migrated `PerceptionProfile` away from `entity_memory_capacity`, `entity_claim_capacity`, `memory_retention_ticks`, and `infrastructure_retention_ticks`, but `crates/worldwake-ai/tests/golden_long_scenarios.rs` still contains two stale helper constructors using the removed fields. That target is only compiled when the golden inventory/doc generator runs `cargo test -p worldwake-ai --features soak --test golden_long_scenarios -- --list`, so the normal workspace proof stayed green while the owned generated-doc refresh failed. As a result, `python3 scripts/golden_inventory.py --write --check-docs` cannot complete, and the generated golden docs for the migrated scenarios cannot be refreshed honestly.

## Assumption Reassessment (2026-04-14)

1. The blocking command is reproducible on the current branch: `cargo test -p worldwake-ai --features soak --test golden_long_scenarios -- --list` fails with `E0560` because `PerceptionProfile` no longer has the four removed fields.
2. The stale literals are concentrated in two helper constructors: `t21_default_perception()` at `crates/worldwake-ai/tests/golden_long_scenarios.rs:275` and `t33_default_perception()` at `crates/worldwake-ai/tests/golden_long_scenarios.rs:1219`.
3. The generated-doc blocker is downstream of those stale helpers, not of the inventory script itself. `scripts/golden_inventory.py` shells out to the same `cargo test ... -- --list` command for that target before writing docs.
4. This is test/doc fallout, not production fallout. The live runtime contract already migrated in ticket 003, and the blocker is limited to soak-feature golden helpers plus the generated markdown produced from that test inventory.
5. The exact owned proof surface is: `golden_long_scenarios` compiles under `--features soak` -> `python3 scripts/golden_inventory.py --write --check-docs` succeeds -> generated golden docs reflect the post-migration scenario wording.

## Architecture Check

1. The clean fix is to migrate the two stale helper constructors to the new activation-based `PerceptionProfile` fields and then regenerate the docs. This keeps the generated golden inventory truthful without reopening production code.
2. No backward-compatibility shim is allowed for the removed profile fields; the test helpers must move to the real live contract.

## Verification Layers

1. Soak golden helper schema compatibility -> `cargo test -p worldwake-ai --features soak --test golden_long_scenarios -- --list`
2. Generated golden doc refresh -> `python3 scripts/golden_inventory.py --write --check-docs`
3. Generated wording alignment for the affected scenario surfaces -> direct read of the updated files under `docs/generated/`
4. Single-layer ticket: no additional production-layer mapping is applicable because the runtime migration already landed in ticket 003

## What to Change

### 1. Migrate stale soak helper constructors

Update `t21_default_perception()` and `t33_default_perception()` in `crates/worldwake-ai/tests/golden_long_scenarios.rs` to use the activation-based `PerceptionProfile` fields instead of the removed capacity/retention fields.

### 2. Refresh generated golden docs

Rerun `python3 scripts/golden_inventory.py --write --check-docs` once the soak target compiles, then verify the generated scenario docs no longer mention the removed `entity_memory_capacity` contract for the affected perception-exposure scenario.

## Files to Touch

- `crates/worldwake-ai/tests/golden_long_scenarios.rs` (modify)
- `docs/generated/golden-scenario-index.md` (modify)
- `docs/generated/golden-scenario-details/perception-exposure.md` (modify)
- `docs/generated/golden-scenario-details/*.md` (modify) — broader generated inventory refresh fallout from `scripts/golden_inventory.py --write --check-docs`

## Out of Scope

- Any production/runtime belief-decay logic
- New activation-decay golden scenarios (owned by `tickets/S101ACTBASBEL-004.md`)
- Broad cleanup of unrelated generated golden docs

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --features soak --test golden_long_scenarios -- --list`
2. `python3 scripts/golden_inventory.py --write --check-docs`

### Invariants

1. No test helper in `golden_long_scenarios.rs` uses removed `PerceptionProfile` fields
2. Generated golden docs describe the live activation-based profile contract rather than the removed capacity-field contract

## Test Plan

### New/Modified Tests

1. `None — documentation/tooling handoff ticket; verification is command-based and existing golden inventory generation coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai --features soak --test golden_long_scenarios -- --list`
2. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

Completed on 2026-04-14.

- Migrated the two stale soak-feature helper constructors in `crates/worldwake-ai/tests/golden_long_scenarios.rs` (`t21_default_perception()` and `t33_default_perception()`) from removed `PerceptionProfile` capacity/retention fields to the live activation-based fields, using the same long-horizon low-threshold profile shape already used by the soak/golden harnesses.
- Restored the golden inventory/doc refresh path by making `cargo test -p worldwake-ai --features soak --test golden_long_scenarios -- --list` compile and enumerate successfully again.
- Regenerated the owned golden docs with `python3 scripts/golden_inventory.py --write --check-docs`, updating the affected perception-exposure scenario text from the removed `entity_memory_capacity = 4` wording to the live `observation_buffer_capacity = 4` contract.

## Deviations

- The ticket's original file list under-claimed the generated fallout surface. The required inventory refresh rewrote the global golden index plus multiple files under `docs/generated/golden-scenario-details/`, not just the two perception-exposure pages originally cited.
- `tickets/S101ACTBASBEL-005.md` began this session as an untracked active draft, so its completion evidence lives in the file content and current worktree state rather than ordinary tracked-ticket diff history.

## Verification Result

- Passed `cargo test -p worldwake-ai --features soak --test golden_long_scenarios -- --list`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
