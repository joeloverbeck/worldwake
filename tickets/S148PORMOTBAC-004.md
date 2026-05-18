# S148PORMOTBAC-004: Slot assembly extension composing OperatingMode with PortfolioWeightsProfile

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends `agent_tick/portfolio.rs::assemble_portfolio` signature to consume `&PortfolioWeightsProfile` and `OperatingMode`; adds `primary_motive_slot` and `apply_mode` helpers; wires `derive_operating_mode` call site in the per-tick decision pipeline; emits the new `PainCare` and `SocialMotive` slot winners
**Deps**: `archive/tickets/S148PORMOTBAC-001.md`, `archive/tickets/S148PORMOTBAC-002.md`, `archive/tickets/S148PORMOTBAC-003.md`, `specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

Tickets 001-003 land the substrate: 5-variant `SlotKind` + `motive_source_slot_for` mapping, `PortfolioWeightsProfile` universal component, and `OperatingMode` derivation. Ticket 004 wires them together: `assemble_portfolio` iterates the five slots, picks the primary motive for each candidate via `motive_source_slot_for(highest-weight contribution)`, applies operating-mode-modulated weights (Emergency zeroes `EconomicOpportunity` and `SocialMotive`), and emits one winner per slot through the existing `select_best_candidate_for_slot` mechanism. The existing `Portfolio::plausible_slots_by_score` at `portfolio.rs:80` continues to operate on the assembled portfolio with the new 5-slot `PortfolioWeightsProfile`.

## Assumption Reassessment (2026-05-17)

1. Current `assemble_portfolio` signature at `crates/worldwake-ai/src/agent_tick/portfolio.rs:35`: `pub(crate) fn assemble_portfolio(ranked: &OrderedRanked<'_>, committed: Option<OpportunityKey>, probe: impl Fn(&AgendaEntry) -> FeasibilityVerdict) -> Portfolio`. Iterates pre-sorted `AgendaEntry` candidates; three sequential `select_best_candidate_for_slot` calls at lines 43, 54, 65 (one per legacy slot). After ticket 001, the legacy three-slot iteration must be replaced with a five-slot iteration. After ticket 002, weights come from `PortfolioWeightsProfile` (read via `GoalBeliefView`). After ticket 003, mode comes from `AgentDecisionRuntime.operating_mode` set earlier in the same tick.
2. Spec S148 D5 specifies the extended signature: `assemble_portfolio(ranked, committed, &PortfolioWeightsProfile, OperatingMode, probe) -> Portfolio`. The new `primary_motive_slot` helper picks the highest-weight contribution from `AgendaEntry.motive_source_contributions: Vec<(MotiveSourceRef, u32)>` (`agenda_types.rs:34`); tie-break by `introduced_tick` ascending (newer loses tie). Candidates with no motive sources fall back to `SlotKind::EconomicOpportunity`. The new `apply_mode` zeroes `EconomicOpportunity` and `SocialMotive` when `mode == Emergency`; identity otherwise.
3. Shared abstraction under audit: the per-tick decision pipeline at `crates/worldwake-ai/src/agent_tick/planning.rs:610` (the existing call site reading `cognitive.slot_weights`). After ticket 002, this site reads `belief_view.portfolio_weights_profile(agent)` and threads through to `assemble_portfolio`; this ticket extends the threading to also include `derive_operating_mode(belief, agent, &ranked)` (cached on `runtime.operating_mode` for use by the same-tick planning cap reader in ticket 008).
4. Existing tests in `agent_tick/portfolio.rs::tests` (block at line 221+) covered by ticket 001's variant rename and ticket 010's golden migration — this ticket adds at least one focused test per branch of the new slot-iteration loop (per slot: presence and absence under each operating mode) but defers full coverage to the golden ticket.

## Architecture Check

1. The existing `plausible_slots_by_score(&PortfolioSlotWeights)` mechanism at `portfolio.rs:80` is preserved as-is (its caller at `planning.rs:4571` swaps the weight-source argument, not the method). The change extends `assemble_portfolio` to compose with the new 5-slot taxonomy without inventing a parallel mechanism — FND-28 alignment via single-mechanism extension rather than introduction.
2. Operating-mode degradation is implemented by zeroing weights (not by filtering slots out of the iteration), which keeps the slot taxonomy stable across modes — only the *weight applied* changes, not the slot identity. Per spec design goal 2: "Operating modes adjust slot enablement, not slot identity."
3. Primary-motive selection is deterministic (highest weight wins; ties broken by `introduced_tick` ascending with newer losing the tie) per spec design goal 6, so portfolio composition is replayable.

## Verification Layers

1. `assemble_portfolio` slot iteration correctness → focused unit test asserting each of the 5 slots emits its winner when populated; absent slots emit nothing (no padding)
2. Operating-mode-modulated weights → focused unit test asserting Emergency mode produces zero-weight `EconomicOpportunity` and `SocialMotive` slots that are still iterated but their `weight_for` reads return `Permille::ZERO`
3. Primary motive tie-break → focused unit test constructing two candidates with equal-weight contributions but different `introduced_tick` values and asserting the older `introduced_tick` wins

## What to Change

### 1. Extend `assemble_portfolio` signature and body

In `crates/worldwake-ai/src/agent_tick/portfolio.rs:35`, extend to:

```rust
pub(crate) fn assemble_portfolio(
    ranked: &OrderedRanked<'_>,
    committed: Option<OpportunityKey>,
    weights: &PortfolioWeightsProfile,
    mode: OperatingMode,
    probe: impl Fn(&AgendaEntry) -> FeasibilityVerdict,
) -> Portfolio {
    let effective_weights = apply_mode(weights, mode);
    let mut portfolio = Portfolio::default();
    let mut selected: BTreeSet<OpportunityKey> = BTreeSet::new();
    for slot in [
        SlotKind::NeedSurvival,
        SlotKind::PainCare,
        SlotKind::ObligationDuty,
        SlotKind::EconomicOpportunity,
        SlotKind::SocialMotive,
    ] {
        if effective_weights.weight_for(slot).is_zero() {
            continue;
        }
        if let Some(winner) = select_best_candidate_for_slot(
            ranked,
            slot,
            committed,
            &probe,
            &selected,
            |entry| primary_motive_slot(entry) == slot,
        ) {
            selected.insert(opportunity_key(winner));
            portfolio.insert(slot, winner);
        }
    }
    portfolio
}
```

(The exact `select_best_candidate_for_slot` signature and the `selected` set's deduplication semantics are validated against the existing implementation during reassessment; the structure above mirrors the spec's intent.)

### 2. Add `primary_motive_slot` helper

```rust
fn primary_motive_slot(entry: &AgendaEntry) -> SlotKind {
    entry
        .motive_source_contributions
        .iter()
        .max_by(|(left_ref, left_w), (right_ref, right_w)| {
            left_w
                .cmp(right_w)
                .then_with(|| right_ref.introduced_tick.cmp(&left_ref.introduced_tick))
        })
        .map(|(motive_ref, _)| {
            worldwake_core::motive_source_slot_for(MotiveSourceDiscriminant::from(
                &motive_ref.source,
            ))
        })
        .unwrap_or(SlotKind::EconomicOpportunity)
}
```

The `right_ref.introduced_tick.cmp(&left_ref.introduced_tick)` direction makes newer ticks compare as smaller (lose the tie), so older `introduced_tick` wins per spec design goal 6.

### 3. Add `apply_mode` helper

```rust
fn apply_mode(weights: &PortfolioWeightsProfile, mode: OperatingMode) -> PortfolioWeightsProfile {
    match mode {
        OperatingMode::Emergency => PortfolioWeightsProfile {
            economic_opportunity: Permille::ZERO,
            social_motive: Permille::ZERO,
            ..*weights
        },
        OperatingMode::Normal | OperatingMode::Idle => *weights,
    }
}
```

### 4. Wire the call site

In `crates/worldwake-ai/src/agent_tick/planning.rs:610` (the per-tick planning entry that currently reads `cognitive.slot_weights` and passes into `assemble_portfolio` and `plausible_slots_by_score`):

- Derive operating mode via `let mode = derive_operating_mode(belief_view, agent, &ranked);` and cache on the runtime: `runtime.operating_mode = mode;`.
- Read weights via `let weights = belief_view.portfolio_weights_profile(agent);` (ticket 002 added the accessor).
- Pass both to `assemble_portfolio(&ranked, committed, &weights, mode, probe)`.
- The downstream `plausible_slots_by_score(&weights)` call at line 4571 continues to work with the new `PortfolioWeightsProfile` shape (ticket 002 already extended `plausible_slots_by_score` to accept `&PortfolioWeightsProfile` via the same `weight_for` method).

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (modify — extend `assemble_portfolio` signature and body; add `primary_motive_slot` and `apply_mode` helpers; new focused tests)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — wire call site at line 610: derive mode, read weights, pass to extended `assemble_portfolio`; line 4571 `plausible_slots_by_score` call site uses the new weight source)

## Out of Scope

- `PainCare` and `SocialMotive` golden coverage (ticket 010)
- `max_candidates_to_plan` removal and replacement with `max_plans_for_mode(mode)` reads at planning.rs:660 and sibling sites (ticket 008)
- Observer rendering of slot winners and operating mode (ticket 009)
- Changes to `select_best_candidate_for_slot`'s internal predicate semantics beyond the new per-slot filter (the helper itself is not extended; only its filter argument changes)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai agent_tick::portfolio::tests::*` — new and migrated tests pass: each of the 5 slots produces its winner when populated; Emergency mode skips `EconomicOpportunity` and `SocialMotive`; tie-break by `introduced_tick` ascending favors older
2. Existing suite: `cargo test --workspace`
3. Lint: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `assemble_portfolio` iterates exactly the 5 `SlotKind` variants in declared order (`NeedSurvival, PainCare, ObligationDuty, EconomicOpportunity, SocialMotive`); the iteration is `BTreeMap`-ordered through the array literal — deterministic.
2. Emergency mode zeroes exactly `EconomicOpportunity` and `SocialMotive` weights — no other slot's weight is mutated.
3. `primary_motive_slot` returns the slot of the highest-weight motive contribution; ties broken by older `introduced_tick` winning; motive-less candidates fall back to `EconomicOpportunity`.
4. `derive_operating_mode` runs once per tick before `assemble_portfolio` and the result is cached on `AgentDecisionRuntime.operating_mode` for downstream reads (ticket 008 consumes the cache).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/portfolio.rs::tests` — add: `assemble_portfolio_populates_all_five_slots_under_normal`, `assemble_portfolio_emergency_skips_economic_and_social`, `primary_motive_slot_picks_highest_weight_contribution`, `primary_motive_slot_breaks_ties_with_older_introduced_tick`, `apply_mode_zeroes_economic_and_social_only`
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — verify call-site wiring through an existing test (or add a focused one) confirming `runtime.operating_mode` is set before `assemble_portfolio` is invoked

### Commands

1. `cargo test -p worldwake-ai agent_tick::portfolio agent_tick::planning`
2. `./scripts/verify.sh`
