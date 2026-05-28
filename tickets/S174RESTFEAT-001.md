# S174RESTFEAT-001: Promote `rest_capacity` to a tracked `FeatureId` in scenario coverage

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None — affects the `scenario-coverage` generator binary and editorial docs only
**Deps**: S174 (landed; `archive/specs/S174-shelter-sleep-surfaces-safe-rest.md`). Companion docs: `docs/scenario-roadmap.md` §5.19, §2 coverage warnings.

## Problem

S174 added `rest_capacity` as an authored place field (rest-site identity / capacity) backing a distinct, fully-implemented mechanic: rest-site scarcity and multi-occupant contention, with five dedicated goldens (`survival_safe_rest`, `survival_sleep_contention`, `survival_rest_interrupted_by_danger`, `survival_failed_rest_cascade`, and the CLI-POV tests in `inspect.rs`).

The `scenario-coverage` generator does not map `rest_capacity` to any `FeatureDef`, so the generated companion (`docs/generated/scenario-coverage.md`) currently emits unmapped-field warnings for it across all five rest scenarios. `docs/scenario-roadmap.md` §6.3 requires every such warning to be *either* permanently classified as editorial *or* given follow-up promotion work. It is presently classified as editorial in §2, but unlike the other unmapped fields (`intention_disposition`, `expectation_store`, `last_seen_memory`, `social_observations`), `rest_capacity` backs a landed gameplay mechanic with its own behavioral goldens. This ticket promotes it to a tracked `FeatureId` so structural activation of rest-site contention is visible as its own column, landed via auxiliary coverage (§5.19) — the same shape as `FeatureId::CognitiveArchetypes` (§5.18).

This does **not** claim a survival-coexistence landing. The long-running, collision-proven survival-row landing for rest-site scarcity remains a separate Cluster 1 deepening gap tracked in `docs/gameplay-mechanic-deepening-roadmap.md`.

## Assumption Reassessment (2026-05-28)

1. **Generator structure.** `crates/worldwake-cli/src/bin/scenario_coverage.rs` defines `enum FeatureId` (line ~59) and a data-driven `const FEATURES: &[FeatureDef]` (line ~110). `FeatureDef` (line ~102) carries `id`, `name`, `covered_agent_fields`, `covered_place_fields`, `covered_scenario_fields`. Activation/warning detection is data-driven over these field lists; a field authored on a scenario but absent from every `FeatureDef`'s covered lists produces the "not mapped by any FeatureDef" warning. Verify at implementation time that no separate exhaustive `match` on `FeatureId` exists that a new variant would break (the binary is self-contained; `FeatureId::CognitiveArchetypes` was added by S152 as the precedent — follow whatever sites that addition touched).
2. **Current mapping of Sleep.** `FeatureId::Sleep` ("Basic needs (Sleep)") maps `covered_place_fields: &["sleep_quality", "place_dirtiness"]` (line ~146). `rest_capacity` is intentionally *not* in this list. The fold-into-Sleep alternative is evaluated and rejected in Architecture Check.
3. **Authored field name.** The scenario field is `rest_capacity` on `PlaceDef` (`crates/worldwake-cli/src/scenario/types.rs`, added by S174 D10). The generated warnings (`docs/generated/scenario-coverage.md` lines ~23, ~39–45) confirm the exact field token is `rest_capacity` and that it appears in five scenarios: `survival-failed-rest-cascade`, `survival-rest-cli`, `survival-rest-interrupted-by-danger`, `survival-safe-rest`, `survival-sleep-contention`.
4. **Adjacent unmapped warnings out of scope.** The same generated section also warns about `portfolio_weights_profile` (agent field, `survival-rest-interrupted-by-danger`) and place-level `contention_policy` (`survival-sleep-contention`). These are pre-existing, not S174 rest-site identity, and are not in scope; this ticket touches only `rest_capacity`. Classify them as separate future cleanup if a maintainer wants them addressed.
5. **Companion verification.** `scripts/verify.sh` runs `cargo run -p worldwake-cli --bin scenario-coverage -- --check`, which fails if `docs/generated/scenario-coverage.md` is stale. The implementer must regenerate with `--write` and commit the regenerated companion.
6. **Name byte-alignment.** `docs/scenario-roadmap.md` §2 requires feature names to stay byte-for-byte aligned with the generated companion. The new `FeatureDef.name` chosen here MUST match the new roadmap catalog row added in this ticket.
7. **No engine/behavior change.** This is generator + editorial-doc scope only. No `worldwake-core/sim/systems/ai` change, no scenario `.ron` change, no golden behavior change. The five rest goldens already pass and are unaffected.

## Architecture Check

1. **Distinct `FeatureId` vs. folding into `Sleep`.** Three options were considered:
   - **(a) New `FeatureId::RestSiteContention` mapping `rest_capacity` (recommended).** Gives rest-site scarcity its own activation column, matching the precedent of `FeatureId::CognitiveArchetypes` (a distinct mechanic landed via auxiliary coverage, §5.18). Keeps structural tracking honest: a scenario either authors a rest site or it does not, independent of baseline sleep.
   - (b) Fold `rest_capacity` into `FeatureId::Sleep.covered_place_fields`. Smallest diff, but hides rest-site activation inside the baseline-sleep column — every baseline-sleep scenario would read identically whether or not it authors a rest site, defeating the purpose of the structural inventory and contradicting §5.19's treatment of rest-site contention as a distinct (auxiliary) mechanic.
   - (c) Leave it as a permanent editorial warning (do nothing). Allowed by §6.3, but inconsistent: every other unmapped warning is a supporting/setup field with no dedicated behavioral goldens, whereas `rest_capacity` backs a landed mechanic with five goldens.

   **Recommendation: (a).** It is the only option that both silences the warning and preserves truthful structural tracking, and it mirrors an existing accepted precedent.
2. **No backwards-compatibility shims.** A `FeatureId` variant + `FeatureDef` entry is pure addition; no alias, no dual path, no deprecated mapping.

## Verification Layers

1. Structural activation of the new feature -> generated companion (`docs/generated/scenario-coverage.md`): the five rest scenarios show the new feature `Active`; non-rest scenarios show it `Absent`.
2. `rest_capacity` no longer unmapped -> generated companion warning section: the `rest_capacity ... not mapped by any FeatureDef` lines are gone.
3. Companion freshness invariant -> `scenario-coverage --check` exits clean (documentation-only proof surface; no runtime/trace layer applies because no engine behavior changes).

## What to Change

### 1. Add the tracked feature to the generator

In `crates/worldwake-cli/src/bin/scenario_coverage.rs`:
- Add a `FeatureId` variant (proposed `RestSiteContention`).
- Add a `FeatureDef` entry with `name: "Rest-site contention / safe rest"` (or a maintainer-approved name — must match the roadmap row), `covered_place_fields: &["rest_capacity"]`, empty agent/scenario field lists.
- Verify no other exhaustive site in the binary needs the new variant (per Assumption 1).

### 2. Regenerate the companion

Run `cargo run -p worldwake-cli --bin scenario-coverage -- --write` and commit the regenerated `docs/generated/scenario-coverage.md` (new feature row/column, `rest_capacity` warnings removed).

### 3. Update `docs/scenario-roadmap.md`

- §2 Gameplay Feature Catalog: add a row for the new feature with activation signal "any place authored with `rest_capacity`", backing source `facility_queue.rs` + `needs_actions.rs` rest-site lifecycle, and roadmap status "Landed auxiliary in §5.19".
- §2 coverage-warnings list: remove the `rest_capacity` bullet (it is now mapped).
- §3 Status Summary: reword the auxiliary rest row to name the new tracked feature.
- §5.19: note the feature is now structurally tracked (auxiliary landing, analogous to §5.18), not merely an unmapped field.

### 4. Update `docs/gameplay-mechanic-deepening-roadmap.md` (only if wording requires)

Cluster 1 already states rest-site coverage is auxiliary and not collision-proven; adjust only if the new tracked-feature status makes any sentence inaccurate. No change is expected to the "Not Yet Proven Enough" gap, which remains the long-running survival-coexistence landing.

## Files to Touch

- `crates/worldwake-cli/src/bin/scenario_coverage.rs` (modify)
- `docs/generated/scenario-coverage.md` (regenerate)
- `docs/scenario-roadmap.md` (modify)
- `docs/gameplay-mechanic-deepening-roadmap.md` (modify only if wording requires)

## Out of Scope

- Any new `FeatureId` for `portfolio_weights_profile` or place-level `contention_policy` (separate future cleanup).
- Authoring a long-running 1440-tick survival-coexistence scenario for rest-site scarcity, or promoting §5.19 from auxiliary to a landed survival row. That is the Cluster 1 deepening work and must be its own spec/ticket.
- Any engine, scenario `.ron`, or golden behavior change.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo run -p worldwake-cli --bin scenario-coverage -- --check` exits clean (companion is fresh and contains the new feature with no `rest_capacity` unmapped warning).
2. `cargo build -p worldwake-cli` (new `FeatureId` variant compiles; no broken match sites).
3. Existing suite unaffected: `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_safe_rest scenarios::survival_sleep_contention scenarios::survival_rest_interrupted_by_danger scenarios::survival_failed_rest_cascade` still passes.

### Invariants

1. The new feature is `Active` for exactly the five scenarios that author `rest_capacity` and `Absent` elsewhere — structural activation must not be conflated with baseline sleep.
2. Feature names remain byte-for-byte aligned between `scenario_coverage.rs` and `docs/scenario-roadmap.md` §2.
3. No engine behavior, scenario, or golden outcome changes; this is a structural-inventory/editorial promotion only.

## Test Plan

### New/Modified Tests

1. `None — generator + documentation promotion; verification is command-based (`scenario-coverage --check`) and the existing rest goldens named in Acceptance Criteria provide the behavioral coverage.`

### Commands

1. `cargo run -p worldwake-cli --bin scenario-coverage -- --write` then `cargo run -p worldwake-cli --bin scenario-coverage -- --check`
2. `cargo build -p worldwake-cli && cargo clippy -p worldwake-cli --all-targets -- -D warnings`
3. `./scripts/verify.sh`
