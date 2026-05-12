# S141MOTSOULED-008: Independent multi-source scoring and autonomous motive-source goldens

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — ranking contribution arithmetic and production motive-source decomposition
**Deps**: `archive/tickets/S141MOTSOULED-007.md`, `specs/S141-motive-source-ledger.md`

## Problem

S141MOTSOULED-007 proved the current motive-source representation, payload, trace, profile, lint, and assertion surfaces. It did not land independent production scoring for offers with multiple motive sources because the live S141MOTSOULED-004 implementation still preserves legacy `motive_score` arithmetic by routing each source through `score_goal_kind_motive`, and trace contributions still assign the aggregate score to the first source.

That leaves three S141 behaviors unproven as autonomous production behavior: Hunger + Greed summation, Pain dominating Hunger under a wound-heavy profile, and otherwise-identical agents diverging because of `UtilityProfile.greed_weight`.

## Assumption Reassessment (2026-05-12)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ranking::score_motive_source` is the exact scoring boundary under audit; today it delegates current variants to the legacy goal-kind scoring body to preserve parity.
2. `agent_tick::planning::motive_source_contributions_for_summary` is the trace contribution boundary under audit; today it gives the full aggregate score to the first source and zero to later sources.
3. `motive_source_mapping::derive_default_motive_sources` is the production source-decomposition boundary under audit; today most production offers emit one default source, so autonomous multiple-source branches need an explicit production decomposition rule rather than a fixture-only vector.
4. The intended invariant is not only "the vector exists." It is that the ranking and trace layers can explain the aggregate score as the deterministic sum of concrete per-source contributions.
5. This ticket is ranking-sensitive: branch divergence must be validated against full live pressure, profile weights, source contributions, priority-class behavior, suppression/filtering, and plan feasibility, not inferred from a single varied field.

## Architecture Check

1. Split per-source scoring at the ranking boundary instead of adding observer-side derivations; the observer must render causal evidence that was already computed by AI.
2. Keep `motive_score` as a derived aggregate cache while making per-source contributions the auditable explanation of that aggregate.
3. Do not add compatibility fallbacks for offers without motive sources; production offers remain invalid without explicit sources.

## Verification Layers

1. Per-source arithmetic invariant -> focused ranking tests prove each source contribution uses the appropriate concrete pressure/profile state and sums to `motive_score`.
2. Trace provenance invariant -> decision-trace tests prove every non-empty source contribution is carried in `RankedGoalSummary.motive_source_contributions`.
3. Autonomous branch invariant -> golden E2E tests prove full candidate generation, ranking, search, commit, event-log payload, and observer-visible trace behavior for the three S141 autonomous cases.

## What to Change

### 1. Ranking arithmetic

Replace the current parity-preserving source dispatch with per-source contribution arithmetic for currently supported source variants. Keep score parity where a production offer has a single source and the old goal-kind formula was already source-equivalent.

### 2. Production source decomposition

Extend production source mapping only where live state provides concrete evidence for multiple motives. Do not manufacture abstract motives or stub deferred variants.

### 3. Golden coverage

Add autonomous goldens for:

1. Hunger + Greed summation for a market opportunity.
2. Pain contribution dominating Hunger under an authored wound/profile setup.
3. `UtilityProfile.greed_weight` divergence between otherwise-identical agents facing the same opportunity.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/motive_source_mapping.rs` (modify if production decomposition changes)
- `crates/worldwake-ai/tests/golden_motive_sources.rs` (modify)
- focused ranking or planning tests as needed

## Out of Scope

- Deferred motive variants whose substrates do not exist yet: `Fear`, `Obligation`, `Debt`, `Habit`, and `Curiosity`.
- Observer formatting that already renders Section 3b contributions from the trace payload.
- Backward-compatibility aliases for empty `motive_sources`.

## Acceptance Criteria

### Tests That Must Pass

1. Focused ranking/contribution tests prove per-source arithmetic and aggregate summation.
2. `golden_motive_sources.rs` includes autonomous coverage for Hunger + Greed, Pain dominance, and `greed_weight` branch divergence.
3. Existing suite: `cargo test --workspace`.
4. CI lint gate: `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. `motive_score` remains a deterministic derived aggregate of concrete `MotiveSourceRef` contributions.
2. Multi-source offers expose every load-bearing source in the decision trace and commit payload.
3. Production offers without `motive_sources` remain invalid.

## Test Plan

### New/Modified Tests

1. Focused ranking or planning tests — prove contribution arithmetic independent of golden fixture shape.
2. `crates/worldwake-ai/tests/golden_motive_sources.rs` — autonomous multi-source behavior coverage.

### Commands

1. `cargo test -p worldwake-ai --test golden_motive_sources`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
