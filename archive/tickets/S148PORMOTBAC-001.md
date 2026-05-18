# S148PORMOTBAC-001: Five-variant SlotKind with core relocation and motive-source mapping

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — relocates `SlotKind` from `worldwake-ai/src/agent_tick/portfolio.rs` to `worldwake-core/src/slot_kind.rs`; expands to five variants (`NeedSurvival`, `PainCare`, `ObligationDuty`, `EconomicOpportunity`, `SocialMotive`); adds `motive_source_slot_map::slot_for` total mapping over `MotiveSourceDiscriminant`
**Deps**: `archive/specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

S112's three-slot portfolio (`Survival`/`Commitment`/`Economic`) collapses safety, care, duty, social, and opportunistic motives into too few buckets; agents miss obligations, fail to investigate suspicions, neglect epistemic work, and skip opportunistic local wins. S148 D1+D4 expands the taxonomy to five slots derived directly from `MotiveSourceDiscriminant` and relocates `SlotKind` to core so the per-motive-class mapping table can live alongside the taxonomy it indexes. The relocation also gives core-resident consumers like `GoalPressureMetrics.candidates_emitted_by_slot` (`crates/worldwake-ai/src/scenario_diagnostics/mod.rs:27`) a single source of truth for `SlotKind` instead of importing it from the AI crate.

## Assumption Reassessment (2026-05-17)

1. Current `SlotKind` lives at `crates/worldwake-ai/src/agent_tick/portfolio.rs:11-16` with three variants (`Survival`, `Commitment`, `Economic`); derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`. Existing focused tests in same file (`#[cfg(test)]` block at line 221+): `slot_kind_round_trips_through_serde:20`, `survival_slot_picks_highest_motive_survival:98`, `commitment_slot_picks_committed_opportunity_when_ranked:120`, `commitment_slot_falls_back_to_highest_obligation_when_commitment_unranked:153`, `self_consume_acquire_populates_survival_slot:193`, `plausible_slots_by_score_applies_weights:249`, `survival_slot_prefers_higher_priority_class_over_higher_motive:299`, `plausible_slots_by_score_prefers_higher_priority_class_over_weighted_score:333`. Existing golden: `crates/worldwake-ai/tests/golden_portfolio_planning.rs` (6 tests; migrated by ticket 010).
2. Spec S148 D1 and D4 specify the new variant set and the `MotiveSourceSlotMap` mapping. `MotiveSourceDiscriminant` already exists at `crates/worldwake-core/src/motive_source.rs:25` with 7 variants (`NeedPressure, Pain, OfficeDuty, Loyalty, Greed, Shame, Revenge`); precedent for the mapping consumer is `MotiveBias.motive_variant: MotiveSourceDiscriminant` at `crates/worldwake-ai/src/htn/method_schema.rs:46`.
3. Shared abstraction under audit: the cross-crate `SlotKind` enum. Current consumers: `crates/worldwake-ai/src/agent_tick/portfolio.rs` (definition + tests), `crates/worldwake-ai/src/agent_tick/planning.rs` (lines 4571, 4597-4598, 4752, 4761), `crates/worldwake-ai/src/decision_trace.rs` (lines 3937, 3945, 3961-3972), `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (2 sites), `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (lines 27, 212), `crates/worldwake-cli/src/bin/observer.rs` (lines 7555-7556). Cross-crate import cycle pressure: the core-side `GoalPressureMetrics` already imports `SlotKind` from ai through the scenario_diagnostics module path — relocating `SlotKind` to core removes the soft layering inversion.
4. Cross-crate variant blast radius: `SlotKind::Survival` 19 sites, `SlotKind::Commitment` 10 sites, `SlotKind::Economic` 14 sites (~43 total across worldwake-ai + worldwake-cli). All rename atomically per FND-28.

## Architecture Check

1. Single source of truth for `SlotKind` lets `GoalPressureMetrics.candidates_emitted_by_slot` (core-resident at `scenario_diagnostics/mod.rs:27`) import the same enum the AI crate constructs. Per FND-28, the relocation removes the implicit cross-crate import that would have grown as more core-side diagnostics consumed `SlotKind`.
2. Variant taxonomy derives from concrete `MotiveSourceDiscriminant` types per FND-3 — no abstract "priority score" decides slot membership. The mapping table is a total exhaustive match: if S141 adds a new motive variant, the match's missing-arm error forces the S148 mapping to update alongside.
3. No backwards-compatibility shims. Legacy variant names (`Survival`, `Commitment`, `Economic`) are removed atomically across the workspace; the local `SlotKind` definition in `agent_tick/portfolio.rs` is deleted (a thin `pub use worldwake_core::SlotKind;` is kept inside portfolio.rs only for in-crate import convenience).

## Verified Layers

1. `SlotKind` variant set + serialization round-trip → focused unit test in `crates/worldwake-core/src/slot_kind.rs::tests`
2. `motive_source_slot_map::slot_for` totality → focused unit test asserting each `MotiveSourceDiscriminant::*` variant maps to a `SlotKind` via explicit enumeration (the test exercises every variant by name; adding a discriminant variant later forces the test to break)
3. Cross-crate consumer migration completeness → workspace compilation under `cargo clippy --workspace --all-targets -- -D warnings` (no orphaned import paths, no leftover legacy variant references)

## Landed Changes

### 1. Relocated `SlotKind` to `worldwake-core`

Added `crates/worldwake-core/src/slot_kind.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SlotKind {
    NeedSurvival,
    PainCare,
    ObligationDuty,
    EconomicOpportunity,
    SocialMotive,
}
```

Re-exported from `crates/worldwake-core/src/lib.rs` (`pub use slot_kind::SlotKind;`). Removed the existing `SlotKind` definition from `crates/worldwake-ai/src/agent_tick/portfolio.rs` and replaced it with a thin re-export at the same site: `pub use worldwake_core::SlotKind;` so in-crate `use crate::agent_tick::portfolio::SlotKind;` references keep working.

### 2. Added `motive_source_slot_map::slot_for` total mapping

Added `crates/worldwake-core/src/motive_source_slot_map.rs`:

```rust
use crate::{MotiveSourceDiscriminant, SlotKind};

pub fn slot_for(discriminant: MotiveSourceDiscriminant) -> SlotKind {
    match discriminant {
        MotiveSourceDiscriminant::NeedPressure => SlotKind::NeedSurvival,
        MotiveSourceDiscriminant::Pain         => SlotKind::PainCare,
        MotiveSourceDiscriminant::OfficeDuty   => SlotKind::ObligationDuty,
        MotiveSourceDiscriminant::Loyalty      => SlotKind::ObligationDuty,
        MotiveSourceDiscriminant::Greed        => SlotKind::EconomicOpportunity,
        MotiveSourceDiscriminant::Shame        => SlotKind::SocialMotive,
        MotiveSourceDiscriminant::Revenge      => SlotKind::SocialMotive,
    }
}
```

Re-exported the mapping from `crates/worldwake-core/src/lib.rs` as `pub use motive_source_slot_map::slot_for as motive_source_slot_for;`.

### 3. Migrated legacy variant references

Atomic rename across the source/test workspace: `Survival` → `NeedSurvival`, `Commitment` → `ObligationDuty`, `Economic` → `EconomicOpportunity`. New variants `PainCare` and `SocialMotive` remain dormant until ticket 004 emits them. The existing `PortfolioSlotWeights` bridge gives those dormant variants zero weight until ticket 002 lifts the five-slot weight profile.

Sites to touch (per Assumption Reassessment item 3 blast radius):
- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (definition site + tests)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (lines 4571, 4597-4598, 4752, 4761)
- `crates/worldwake-ai/src/decision_trace.rs` (lines 3937, 3945, 3961-3972)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (2 sites)
- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (lines 27, 212 — the line-212 fixture in particular constructs a `BTreeMap` literal keyed by the legacy variant)
- `crates/worldwake-cli/src/bin/observer.rs` (lines 7555-7556)

## Landed Files

- `crates/worldwake-core/src/slot_kind.rs` (new)
- `crates/worldwake-core/src/motive_source_slot_map.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add re-exports)
- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (modify — remove local SlotKind def, add `pub use worldwake_core::SlotKind;`, migrate variant references and tests)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — variant rename)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — variant rename)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify — variant rename)
- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (modify — variant rename in fixture map literal)
- `crates/worldwake-ai/tests/golden_portfolio_planning.rs` (modify — existing golden assertion updated for renamed slot debug output)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — variant rename in rendering)
- `archive/tickets/S148PORMOTBAC-001.md` (modify — closeout truthing, then archive move)

## Out of Scope

- `PortfolioWeightsProfile` lifting and per-slot weight field naming (ticket 002 — depends on this ticket for variant names)
- `OperatingMode` enum and per-tick derivation (ticket 003)
- `assemble_portfolio` extension to use the five new variants and emit `PainCare`/`SocialMotive` winners (ticket 004 — slot assembly uses `motive_source_slot_for` to emit these)
- New golden coverage for the five-slot portfolio remains ticket 010. This ticket only updated existing golden assertions that failed because this ticket renamed the live enum variants.

## Acceptance Criteria

### Tests Passed

1. `cargo test -p worldwake-core slot_kind` — added focused tests passed (serde round-trip on all 5 variants; `Ord` ordering matches declaration order)
2. `cargo test -p worldwake-core motive_source_slot_map` — totality test passes (every `MotiveSourceDiscriminant` variant has a slot)
3. Existing portfolio.rs tests passed with renamed variants — semantic intent preserved (e.g., `survival_slot_picks_highest_motive_survival` now tests `NeedSurvival`-slot selection from `NeedPressure`-discriminant candidates)
4. Existing suite: `cargo test --workspace`
5. Lint: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `SlotKind` is defined in exactly one crate (`worldwake-core`); no parallel definition survives anywhere in the workspace.
2. `motive_source_slot_for` (or the chosen public name) is exhaustive over `MotiveSourceDiscriminant` with no `_ =>` catch-all — adding a discriminant variant later breaks compilation, forcing the S148 mapping to update.
3. No legacy variant reference (`SlotKind::Survival`, `SlotKind::Commitment`, `SlotKind::Economic`) appears in source/test code after migration; remaining active spec/ticket mentions are historical migration context.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/slot_kind.rs` — new inline `#[cfg(test)]` module: `slot_kind_serde_round_trips_all_five_variants`, `slot_kind_ord_matches_declaration_order`
2. `crates/worldwake-core/src/motive_source_slot_map.rs` — new inline `#[cfg(test)]` module: `slot_for_is_defined_for_every_motive_source_discriminant`
3. `crates/worldwake-ai/src/agent_tick/portfolio.rs` — modified existing portfolio tests to use new variants while preserving the semantic intent each test asserts
4. `crates/worldwake-ai/tests/golden_portfolio_planning.rs` — updated existing assertion text to check `ObligationDuty`, `EconomicOpportunity`, and absence of `NeedSurvival`

### Commands Run

1. `cargo test -p worldwake-core slot_kind`
2. `cargo test -p worldwake-core motive_source_slot_map`
3. `cargo test -p worldwake-ai --lib agent_tick::portfolio`
4. `cargo test -p worldwake-ai --test golden_portfolio_planning`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-17.

- `SlotKind` now lives in `worldwake-core` with the five S148 variants and is re-exported through `worldwake-ai` for existing imports.
- Added the exhaustive core `motive_source_slot_for` mapping from every current `MotiveSourceDiscriminant` to a slot, with no catch-all arm.
- Renamed legacy slot variant references across AI planning, traces, diagnostics, observer fixtures, and existing portfolio/golden assertions.
- Left `PainCare` and `SocialMotive` dormant in the current three-slot assembly; they receive zero legacy weight until later S148 tickets add `PortfolioWeightsProfile` and five-slot assembly.

## Deviations

- The drafted combined focused command `cargo test -p worldwake-core slot_kind motive_source_slot_map` is not a truthful Cargo selector shape, so verification was split into two commands.
- Broad workspace testing exposed an existing golden assertion that still searched for legacy debug names. That assertion was updated here as enum-rename fallout; new S148 golden coverage remains ticket 010.

## Verification Result

- Passed `cargo test -p worldwake-core slot_kind`
- Passed `cargo test -p worldwake-core motive_source_slot_map`
- Passed `cargo test -p worldwake-ai --lib agent_tick::portfolio`
- Passed `cargo test -p worldwake-ai --test golden_portfolio_planning`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
