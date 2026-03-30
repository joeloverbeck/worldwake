# S41BANOFFEME-005: Golden Inventory Update & Cross-Suite Verification

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None in this ticket; live implementation already landed in `worldwake-ai`
**Deps**: `specs/S41-bandit-offensive-emergence-goldens.md`

## Problem

The original ticket assumed the S41 suites were still pending in sibling tickets and that this step was documentation-only. Reassessment against the live code showed the opposite: Scenarios 47-49 and their replay tests already exist in `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`, the generated golden inventory docs were stale, and the ticket needed to be corrected to cover verification, metadata normalization for the inventory parser, and archival.

## Assumption Reassessment (2026-03-30)

1. `cargo test -p worldwake-ai -- --list` confirms the live golden binary already contains eight bandit-camp golden tests in `golden_t22_bandit_camp_destruction.rs`: the two T22 tests plus Scenarios 47-49 and their replay variants.
2. `scripts/golden_inventory.py` exists at `scripts/golden_inventory.py` and `python3 scripts/golden_inventory.py --write --check-docs` regenerates `docs/generated/golden-e2e-inventory.md` and `docs/generated/golden-scenario-map.md`.
3. The exact shared boundary under audit is the structured `// Scenario ...` metadata contract parsed by `parse_source_scenarios()` in `scripts/golden_inventory.py` from source comments in `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`.
4. The live `GoalKind` family under test is `RaidTarget`. Scenario 47 depends on co-located non-faction prey and commodity-visible motive in `ranking.rs`; Scenario 48 depends on belief propagation plus route-threat planning in `route_threat.rs` and `planning_snapshot.rs`; Scenario 49 depends on wound-driven raid deterrence via `pressure.rs`.
5. The ticket’s earlier claim that Scenario 49 still needed a future architecture change was stale. The live architecture already centralizes wound deterrence in `is_bandit_raid_deterred_by_wounds()` and `bandit_raid_wound_threshold()` in `crates/worldwake-ai/src/pressure.rs`, and both `emit_raid_target_goals()` in `crates/worldwake-ai/src/candidate_generation.rs` and `raid_target_motive()` in `crates/worldwake-ai/src/ranking.rs` consume that shared substrate.
6. That live shared-pressure design is cleaner than the stale ticket narrative. It avoids a one-off raid-only patch, keeps deterrence derivation in one canonical place, and makes the same rule visible to candidate generation and ranking without introducing an alias path.
7. The generated docs were genuinely incomplete before this turn: the scenario-map inventory was stale and the S41 comment blocks in `golden_t22_bandit_camp_destruction.rs` needed parser-compliant continuation formatting so `Setup`, `Proves`, and `Chain` render fully in `docs/generated/golden-scenario-map.md`.
8. `cargo clippy --workspace --all-targets -- -D warnings` initially failed on two test-only lint issues: a `map(...).unwrap_or(...)` pattern in `golden_t22_bandit_camp_destruction.rs` and a single-variant wildcard in `crates/worldwake-ai/src/search/tests.rs`. Those were verification blockers, not architectural engine gaps.
9. Mismatch + correction: this ticket is not a precursor to future S41 implementation tickets anymore. Its corrected scope is metadata normalization, generated-doc refresh, focused and full verification, and archival of the completed ticket/spec.

## Architecture Check

1. Keeping wound-based raid deterrence in shared pressure helpers is the right architecture. It is cleaner and more extensible than ticketing or documenting a candidate-generation-only suppression rule because ranking and candidate emission stay aligned on one canonical deterrence substrate.
2. The only source changes needed in this ticket were source-of-truth metadata normalization and behavior-preserving lint cleanups. No backwards-compatibility shims, aliases, or duplicate raid-deterrence paths were introduced.

## Verification Layers

1. Scenario inventory completeness and metadata rendering -> `python3 scripts/golden_inventory.py --write --check-docs` plus inspection of `docs/generated/golden-e2e-inventory.md` and `docs/generated/golden-scenario-map.md`
2. Scenario 47 raid emergence behavior -> `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_pressure_driven_raid_emergence`
3. Scenario 48 belief/economic cascade behavior -> `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_raid_belief_economic_cascade`
4. Scenario 49 wound-dampened raid behavior -> `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_wound_dampened_raid_spiral`
5. Search-test lint cleanup remains behavior-preserving -> `cargo test -p worldwake-ai search_restock_route_preference_follows_believed_combat_threat`
6. Broader AI regression -> `cargo test -p worldwake-ai`
7. Workspace lint health -> `cargo clippy --workspace --all-targets -- -D warnings`

## What to Change

### 1. Normalize scenario metadata at the source

Adjust the S41 scenario comment blocks in `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` to use parser-compliant continuation lines so the generated scenario map renders the full `Setup`, `Proves`, and `Chain` text.

### 2. Refresh generated golden documentation

Regenerate `docs/generated/golden-e2e-inventory.md` and `docs/generated/golden-scenario-map.md` from live test source.

### 3. Clear verification blockers

Apply the smallest behavior-preserving test-code cleanups required for `cargo clippy --workspace --all-targets -- -D warnings` to pass.

### 4. Archive the completed work

Mark this ticket and the S41 spec complete, add outcome sections, and move them into the archive.

## Files to Touch

- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-map.md` (regenerated)
- `specs/S41-bandit-offensive-emergence-goldens.md` (modify, then archive)
- `tickets/S41BANOFFEME-005.md` (modify, then archive)

## Out of Scope

- New engine behavior for bandit raids or route threat
- Reworking the golden inventory script
- Any backwards-compatibility layer around the S41 implementation

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_pressure_driven_raid_emergence`
2. `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_raid_belief_economic_cascade`
3. `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_wound_dampened_raid_spiral`
4. `cargo test -p worldwake-ai search_restock_route_preference_follows_believed_combat_threat`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `python3 scripts/golden_inventory.py --write --check-docs`

### Invariants

1. Generated inventory docs reflect the live S41 tests and scenarios without adding any alias test names or duplicate scenario IDs.
2. The documented wound-dampening story matches the live shared-pressure architecture rather than an obsolete planned patch.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` — modified existing test source metadata to make the generated scenario inventory truthful and adjusted a helper for lint compliance without changing test behavior.
2. `crates/worldwake-ai/src/search/tests.rs` — narrowed an existing helper match arm to satisfy pedantic clippy without changing planner-test semantics.

### Commands

1. `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_pressure_driven_raid_emergence`
2. `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_raid_belief_economic_cascade`
3. `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_wound_dampened_raid_spiral`
4. `cargo test -p worldwake-ai search_restock_route_preference_follows_believed_combat_threat`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

- Completion date: 2026-03-30
- What actually changed: corrected the ticket’s stale assumptions, normalized S41 scenario metadata in the live golden source, regenerated the golden inventory docs, fixed two test-only clippy blockers, verified the focused S41 goldens plus the full `worldwake-ai` crate, and prepared the completed ticket/spec for archival.
- Deviations from original plan: the original ticket assumed sibling S41 implementation tickets were still pending and that this ticket would touch only generated docs. Reassessment showed the S41 implementation had already landed, so the real work was verification, metadata/source-of-truth cleanup, and archival.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_pressure_driven_raid_emergence` ✅
  - `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_raid_belief_economic_cascade` ✅
  - `cargo test -p worldwake-ai --test golden_t22_bandit_camp_destruction golden_wound_dampened_raid_spiral` ✅
  - `cargo test -p worldwake-ai search_restock_route_preference_follows_believed_combat_threat` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `python3 scripts/golden_inventory.py --write --check-docs` ✅
