# S174RESTFEAT-001: Promote `rest_capacity` to a tracked `FeatureId` in scenario coverage

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None — affects the `scenario-coverage` generator binary and editorial docs only
**Deps**: S174 (landed; `archive/specs/S174-shelter-sleep-surfaces-safe-rest.md`). Companion docs: `docs/scenario-roadmap.md` §5.19, §2 coverage warnings.

## Problem

S174 added `rest_capacity` as an authored place field (rest-site identity / capacity) backing a distinct, fully-implemented mechanic: rest-site scarcity and multi-occupant contention, with five dedicated goldens (`survival_safe_rest`, `survival_sleep_contention`, `survival_rest_interrupted_by_danger`, `survival_failed_rest_cascade`, and the CLI-POV tests in `inspect.rs`).

Before this ticket, the `scenario-coverage` generator did not map `rest_capacity` to any `FeatureDef`, so the generated companion (`docs/generated/scenario-coverage.md`) emitted unmapped-field warnings for it across all five rest scenarios. `docs/scenario-roadmap.md` §6.3 requires every such warning to be *either* permanently classified as editorial *or* given follow-up promotion work. It had been classified as editorial in §2, but unlike the other unmapped fields (`intention_disposition`, `expectation_store`, `last_seen_memory`, `social_observations`), `rest_capacity` backs a landed gameplay mechanic with its own behavioral goldens. This ticket promoted it to a tracked `FeatureId` so structural activation of rest-site contention is visible as its own column, landed via auxiliary coverage (§5.19) — the same shape as `FeatureId::CognitiveArchetypes` (§5.18).

This does **not** claim a survival-coexistence landing. The long-running, collision-proven survival-row landing for rest-site scarcity remains a separate Cluster 1 deepening gap tracked in `docs/gameplay-mechanic-deepening-roadmap.md`.

## Assumption Reassessment (2026-05-28)

1. **Generator structure.** `crates/worldwake-cli/src/bin/scenario_coverage.rs` defines `enum FeatureId` and a data-driven `const FEATURES: &[FeatureDef]`. `FeatureDef` carries `id`, `name`, `covered_agent_fields`, `covered_place_fields`, `covered_scenario_fields`. Activation/warning detection is data-driven over these field lists; a field authored on a scenario but absent from every `FeatureDef`'s covered lists produces the "not mapped by any FeatureDef" warning. Implementation verified the binary's exhaustive `FeatureId` match and added the required `RestSiteContention` status arm.
2. **Sleep mapping kept separate.** `FeatureId::Sleep` ("Basic needs (Sleep)") maps `covered_place_fields: &["sleep_quality", "place_dirtiness"]`. `rest_capacity` is intentionally not in this list; the landed implementation uses a distinct `FeatureId::RestSiteContention` entry instead.
3. **Authored field name.** The scenario field is `rest_capacity` on `PlaceDef` (`crates/worldwake-cli/src/scenario/types.rs`, added by S174 D10). The regenerated companion confirms the exact field token is mapped and appears in five scenarios: `survival-failed-rest-cascade`, `survival-rest-cli`, `survival-rest-interrupted-by-danger`, `survival-safe-rest`, `survival-sleep-contention`.
4. **Adjacent unmapped warnings out of scope.** The same generated section also warns about `portfolio_weights_profile` (agent field, `survival-rest-interrupted-by-danger`) and place-level `contention_policy` (`survival-sleep-contention`). These are pre-existing, not S174 rest-site identity, and are not in scope; this ticket touches only `rest_capacity`. Classify them as separate future cleanup if a maintainer wants them addressed.
5. **Companion verification.** `scripts/verify.sh` runs `cargo run -p worldwake-cli --bin scenario-coverage -- --check`, which fails if `docs/generated/scenario-coverage.md` is stale. The implementer must regenerate with `--write` and commit the regenerated companion.
6. **Name byte-alignment.** `docs/scenario-roadmap.md` §2 requires feature names to stay byte-for-byte aligned with the generated companion. The landed `FeatureDef.name` and roadmap catalog row both use `Rest-site contention / safe rest`.
7. **No engine/behavior change.** This is generator + editorial-doc scope only. No `worldwake-core/sim/systems/ai` change, no scenario `.ron` change, no golden behavior change. The five rest goldens already pass and are unaffected.

## Architecture Check

1. **Distinct `FeatureId` vs. folding into `Sleep`.** Three options were considered:
   - **(a) New `FeatureId::RestSiteContention` mapping `rest_capacity` (recommended).** Gives rest-site scarcity its own activation column, matching the precedent of `FeatureId::CognitiveArchetypes` (a distinct mechanic landed via auxiliary coverage, §5.18). Keeps structural tracking honest: a scenario either authors a rest site or it does not, independent of baseline sleep.
   - (b) Fold `rest_capacity` into `FeatureId::Sleep.covered_place_fields`. Smallest diff, but hides rest-site activation inside the baseline-sleep column — every baseline-sleep scenario would read identically whether or not it authors a rest site, defeating the purpose of the structural inventory and contradicting §5.19's treatment of rest-site contention as a distinct (auxiliary) mechanic.
   - (c) Leave it as a permanent editorial warning (do nothing). Allowed by §6.3, but inconsistent: every other unmapped warning is a supporting/setup field with no dedicated behavioral goldens, whereas `rest_capacity` backs a landed mechanic with five goldens.

   **Recommendation: (a).** It is the only option that both silences the warning and preserves truthful structural tracking, and it mirrors an existing accepted precedent.
2. **No backwards-compatibility shims.** A `FeatureId` variant + `FeatureDef` entry is pure addition; no alias, no dual path, no deprecated mapping.

## Outcome

Completed on 2026-05-28.

The generator now tracks `rest_capacity` through `FeatureId::RestSiteContention` / `Rest-site contention / safe rest`, the generated companion reports the five authored rest-capacity scenarios as active for that feature, and the roadmap catalog/status prose names the auxiliary landing without promoting it to a long-running survival-coexistence row.

## Verified Layers

1. Structural activation of the tracked feature -> regenerated companion (`docs/generated/scenario-coverage.md`): the five scenarios that author `rest_capacity` show `Rest-site contention / safe rest` as active, and non-rest scenarios show it absent.
2. `rest_capacity` mapped status -> generated companion warning section: every `rest_capacity ... not mapped by any FeatureDef` warning was removed, while out-of-scope warnings such as `contention_policy` and `portfolio_weights_profile` remain visible.
3. Companion freshness invariant -> `scenario-coverage --check` passed.
4. Behavior non-regression -> the existing S174 focused goldens and CLI inspect/rest-site POV tests passed through focused `golden_ai` module filters and `cargo test -p worldwake-cli`; no engine, scenario, or golden assertion changed.

## Landed Changes

1. Added `FeatureId::RestSiteContention` in `crates/worldwake-cli/src/bin/scenario_coverage.rs`.
2. Added the `FeatureDef` named `Rest-site contention / safe rest`, mapped to `covered_place_fields: &["rest_capacity"]`.
3. Added `rest_site_contention_status`, which returns `Active` when any authored place has `rest_capacity` and `Absent` otherwise.
4. Regenerated `docs/generated/scenario-coverage.md`.
5. Updated `docs/scenario-roadmap.md` §2, §3, and §5.19 so the hand-authored catalog, status summary, and auxiliary rest row match the generated feature name and mapped-field status.

## Landed Files

- `crates/worldwake-cli/src/bin/scenario_coverage.rs`
- `docs/generated/scenario-coverage.md`
- `docs/scenario-roadmap.md`

## Out of Scope Result

- No new `FeatureId` was added for `portfolio_weights_profile` or place-level `contention_policy`; their generated warnings remain visible.
- No long-running 1440-tick survival-coexistence scenario was authored, and §5.19 remains auxiliary rather than promoted to a survival row.
- No engine behavior, scenario `.ron`, or golden assertion changed.
- `docs/gameplay-mechanic-deepening-roadmap.md` needed no edit because its Cluster 1 rest-site wording already describes the remaining long-running, collision-proven gap truthfully.

## Verification Result

- Passed `cargo fmt --all`.
- Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --write`.
- Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
- Passed `cargo build -p worldwake-cli`.
- Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`.
- Passed `cargo test -p worldwake-ai --test golden_ai -- --list`; this resolved the drafted stale per-file golden command to the live `golden_ai` target.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_safe_rest`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_sleep_contention`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_rest_interrupted_by_danger`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_failed_rest_cascade`.
- Passed `cargo test -p worldwake-cli`.
- Passed `./scripts/verify.sh`; live wrapper gates were `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `bash scripts/check_no_artifact_state.sh`, `bash scripts/check_no_debug_view_in_ai.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
