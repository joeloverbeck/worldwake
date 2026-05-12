# S141MOTSOULED-008: Independent multi-source scoring and autonomous motive-source goldens

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — ranking contribution arithmetic and production motive-source decomposition
**Deps**: `archive/tickets/S141MOTSOULED-007.md`, `archive/specs/S141-motive-source-ledger.md`

## Problem

S141MOTSOULED-007 proved the motive-source representation, payload, trace, profile, lint, and assertion surfaces. It did not land independent production scoring for offers with multiple motive sources because the live S141MOTSOULED-004 implementation preserved legacy `motive_score` arithmetic by routing each source through `score_goal_kind_motive`, and trace contributions assigned the aggregate score to the first source.

That left three S141 behaviors unproven as autonomous production behavior: Hunger + Greed summation, Pain dominating Hunger under a wound-heavy profile, and otherwise-identical agents diverging because of `UtilityProfile.greed_weight`.

## Assumption Reassessment (2026-05-12)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ranking::score_motive_source` was the exact scoring boundary under audit; before this ticket it delegated current variants to the legacy goal-kind scoring body to preserve parity.
2. `agent_tick::planning::motive_source_contributions_for_summary` was the trace contribution boundary under audit; before this ticket it used an aggregate-first-source placeholder instead of per-source contribution evidence.
3. `motive_source_mapping::derive_default_motive_sources` was the production source-decomposition boundary under audit; before this ticket most production offers emitted one default source, so autonomous multiple-source branches needed an explicit production decomposition rule rather than a fixture-only vector.
4. The intended invariant is not only "the vector exists." It is that the ranking and trace layers can explain the aggregate score as the deterministic sum of concrete per-source contributions.
5. This ticket is ranking-sensitive: branch divergence must be validated against full live pressure, profile weights, source contributions, priority-class behavior, suppression/filtering, and plan feasibility, not inferred from a single varied field.

## Architecture Check

1. Split per-source scoring at the ranking boundary instead of adding observer-side derivations; the observer must render causal evidence that was already computed by AI.
2. Keep `motive_score` as a derived aggregate cache while making per-source contributions the auditable explanation of that aggregate.
3. Do not add compatibility fallbacks for offers without motive sources; production offers remain invalid without explicit sources.

## Verified Layers

1. Per-source arithmetic invariant -> focused ranking tests prove each source contribution uses the appropriate concrete pressure/profile state and sums to `motive_score`.
2. Trace provenance invariant -> planning tests and the golden motive-source suite prove non-empty source contributions are carried in `RankedGoalSummary.motive_source_contributions`.
3. Autonomous branch invariant -> `golden_motive_sources.rs` now proves Hunger + Greed source summation, Pain-vs-Hunger dominance, and concrete `greed_weight` variation over the live source/contribution carriers.

## Changed Surfaces

### 1. Ranking arithmetic

Replaced the parity-preserving source dispatch with per-source contribution arithmetic for currently supported source variants. Score parity is kept where a production offer has a single source and the old goal-kind formula was already source-equivalent.

### 2. Production source decomposition

Extended production source mapping only where live state provides concrete evidence for multiple motives. The implementation did not manufacture abstract motives or stub deferred variants.

### 3. Golden coverage

Added autonomous goldens for:

1. Hunger + Greed summation for a market opportunity.
2. Pain contribution dominating Hunger under an authored wound/profile setup.
3. `UtilityProfile.greed_weight` divergence between otherwise-identical agents facing the same opportunity.

## Files Touched

- `crates/worldwake-ai/src/ranking.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/motive_source_mapping.rs`
- `crates/worldwake-ai/tests/golden_motive_sources.rs`
- `crates/worldwake-ai/src/agenda_types.rs`
- `crates/worldwake-sim/src/save_load.rs`
- supporting call sites and tests updated for serialized `AgendaEntry.motive_source_contributions`

## Out of Scope

- Deferred motive variants whose substrates do not exist yet: `Fear`, `Obligation`, `Debt`, `Habit`, and `Curiosity`.
- Observer formatting that already renders Section 3b contributions from the trace payload.
- Backward-compatibility aliases for empty `motive_sources`.

## Acceptance Result

### Tests Passed

1. Focused ranking/contribution tests prove per-source arithmetic and aggregate summation.
2. `golden_motive_sources.rs` includes autonomous coverage for Hunger + Greed, Pain dominance, and `greed_weight` branch divergence.
3. Existing suite passed: `cargo test --workspace`.
4. CI lint gate passed: `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. `motive_score` remains a deterministic derived aggregate of concrete `MotiveSourceRef` contributions.
2. Multi-source offers expose every load-bearing source in the decision trace and commit payload.
3. Production offers without `motive_sources` remain invalid.

## Test Plan Result

### Added/Modified Tests

1. Focused ranking or planning tests — prove contribution arithmetic independent of golden fixture shape.
2. `crates/worldwake-ai/tests/golden_motive_sources.rs` — autonomous multi-source behavior coverage.

### Commands Run

1. `cargo test -p worldwake-ai --test golden_motive_sources`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-12.

- `ranking.rs` now computes source-specific motive contributions for the live `MotiveSource` variants and stores contribution vectors on ranked agenda entries so trace summaries carry the same explanation as the aggregate score.
- Source-reliability and production-competition discounts preserve the invariant that stored contributions sum to the final ranked `motive_score`.
- Self-consume commodity acquisition now decomposes into both `NeedPressure` and `Greed` sources when live state can support both motives.
- `golden_motive_sources.rs` and focused ranking/planning tests now cover Hunger + Greed summation, Pain-vs-Hunger dominance, and `greed_weight` branch divergence.
- `SAVE_FORMAT_VERSION` was bumped to 79 because `AgendaEntry` now serializes `motive_source_contributions`.
- Generated golden inventory/docs were refreshed after the motive-source golden metadata changed.

## Deviations

- A `#[cfg(test)]` fallback still derives default motive sources for legacy empty-source ranking fixtures. Production builds continue to assert that offers without explicit motive sources are invalid.
- Some legacy goal families still fall back to the extracted goal-kind scorer inside the `Greed` branch where the exact source-specific substrate is not yet represented. The live S141MOTSOULED-008 proof covers the concrete substrates available today.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib motive_source -- --nocapture`
- Passed `cargo test -p worldwake-ai --lib ranking::tests -- --nocapture`
- Passed `cargo test -p worldwake-ai --test golden_motive_sources`
- Passed `cargo test -p worldwake-ai --test golden_offices`
- Passed `cargo test -p worldwake-sim save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
