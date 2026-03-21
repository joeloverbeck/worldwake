# S11WOULIFAUD-004: Recovery-aware AI priority boost for need goals

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI ranking logic in `worldwake-ai`
**Deps**: `archive/specs/S11-wound-lifecycle-audit.md`

## Problem

The authoritative wound lifecycle already gates natural recovery on `recovery_conditions_met()` in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs): clotted wounds recover only when the agent is not in combat and `hunger`, `thirst`, and `fatigue` are each below their `high` threshold. The AI ranking layer in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) still ignores that gate. As a result, an agent with clotted wounds and a recovery-blocking need at `High` treats eat/drink/sleep as ordinary high-priority self-care rather than the immediate prerequisite for healing.

## Assumption Reassessment (2026-03-21)

1. The ticket's original broader wound-lifecycle framing is stale. The deprivation-wound lookup API already exists in [`crates/worldwake-core/src/wounds.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/wounds.rs), via `WoundList::find_deprivation_wound()` and `find_deprivation_wound_mut()`, with focused tests:
   - `find_deprivation_wound_returns_match`
   - `find_deprivation_wound_mut_updates_severity`
   - `find_deprivation_wound_returns_none_for_empty_list`
2. The deprivation-wound worsening path also already exists in [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs), via `worsen_or_create_deprivation_wound()`, with focused tests:
   - `worsen_creates_new_when_no_existing`
   - `worsen_increases_existing_severity`
   - `worsen_caps_at_permille_max`
   - `different_kinds_create_separate_wounds`
   - `worsen_updates_inflicted_at`
   - `needs_system_second_starvation_threshold_worsens_existing_wound`
3. The wound-pruning / zero-recovery hardening work is also already present in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs), with focused tests:
   - `zero_recovery_rate_wound_persists`
   - `wound_bleed_clot_arithmetic_exact`
   - `pruning_only_at_severity_zero`
   - `progress_wounds_returns_none_when_no_change`
4. The remaining gap is localized to `worldwake-ai` ranking. `RankingContext` in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) currently contains `view`, `agent`, `current_tick`, `utility`, `needs`, `thresholds`, `danger_pressure`, and `decision_context`. It does not currently cache or derive wound-recovery state.
5. `drive_priority()` in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) currently only classifies the passed drive band. `Sleep`, `Relieve`, and `Wash` all use that same helper, so recovery-aware promotion is not expressible without either adding context or adding a separate helper.
6. `relevant_self_consume_factors()` currently returns only `(pressure, weight, band)` tuples for hunger/thirst consumables. It cannot distinguish recovery-relevant self-consume branches from generic factor calculation.
7. `GoalBeliefView::wounds()` already exposes `Vec<Wound>` with `severity` and `bleed_rate_per_tick`, so the ranking layer can detect clotted wounds without any trait change.
8. Existing `worldwake-ai` ranking tests cover ordinary self-care, enterprise, and pain/care ranking, but there is no focused test coverage for the coupling between clotted wounds and recovery-blocking need priorities. I verified this by reading [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) tests and dry-running `cargo test -p worldwake-ai ranking -- --nocapture`.
9. Corrected scope: this ticket should only change `crates/worldwake-ai/src/ranking.rs` plus its focused unit tests. Core and systems wound-lifecycle work is already delivered and should not be reimplemented here.

## Architecture Check

1. The proposed behavior is more beneficial than the current architecture because it closes a real cross-layer contradiction: the authoritative combat system says hunger/thirst/fatigue at `High` block healing, but the planner ranks those needs as if they were independent. Promoting exactly `High -> Critical` for recovery-relevant needs lets AI reason about a concrete causal precondition instead of relying on incidental side-effects.
2. The cleanest implementation is still a small internal ranking-layer change, not a new shared abstraction. `GoalBeliefView::wounds()` already provides the needed concrete state, and the coupling is specifically about AI ranking semantics, not authoritative wound progression.
3. The architecture should keep this local and explicit. No new `GoalKind`, no new compatibility layer, no new profile knob, and no alias path. A tiny helper that promotes a base priority for clotted-wound recovery is cleaner than scattering ad hoc conditionals at each call site.
4. Longer-term ideal architecture: if more systems begin to depend on "what blocks recovery," the cleaner substrate would be a shared derived concept that both AI and UI can query from concrete wound/need state. This ticket does not introduce that substrate because there is only one live consumer today and adding a broader abstraction now would be speculative.

## Verification Layers

1. Authoritative recovery gate still lives in `worldwake-systems::combat::recovery_conditions_met()` -> existing focused combat tests named above.
2. Clotted wound + hunger/thirst commodity at `High` promotes to `Critical` -> focused `worldwake-ai::ranking` unit tests.
3. Clotted wound + fatigue at `High` promotes `Sleep` to `Critical` -> focused `worldwake-ai::ranking` unit test.
4. Bleeding wounds, no wounds, or below-`High` needs do not promote -> focused `worldwake-ai::ranking` unit tests.
5. Non-recovery drives (`Relieve`, `Wash`) remain unchanged -> focused `worldwake-ai::ranking` unit test.
6. No candidate-generation, plan-search, or authoritative execution behavior is changed directly by this ticket. Full `cargo test -p worldwake-ai` is still required because ranking feeds downstream AI selection.

## What to Change

### 1. Add clotted-wound detection in ranking

```rust
fn has_clotted_wounds(view: &dyn GoalBeliefView, agent: EntityId) -> bool {
    view.wounds(agent)
        .iter()
        .any(|w| w.bleed_rate_per_tick.value() == 0 && w.severity.value() > 0)
}
```

### 2. Cache clotted-wound state in `RankingContext`

Compute in `RankingContext::new()` (or wherever the context is constructed) by calling the helper.

### 3. Promote `High` self-care priorities when they unblock recovery

Keep the policy local to ranking. A helper is acceptable if it keeps the rule explicit, for example:

```rust
fn promote_for_clotted_wound_recovery(
    base: GoalPriorityClass,
    context: &RankingContext<'_>,
    recovery_relevant: bool,
) -> GoalPriorityClass {
    if recovery_relevant && context.has_clotted_wounds && base == GoalPriorityClass::High {
        GoalPriorityClass::Critical
    } else {
        base
    }
}
```

`drive_priority()` may grow a `recovery_relevant` parameter, or the promotion may be applied at the call site. Either is acceptable as long as the rule stays internal, explicit, and centralized.

### 4. Update `priority_class()` call sites for direct drives

- `GoalKind::Sleep` → `recovery_relevant: true`
- `GoalKind::Relieve` → `recovery_relevant: false`
- `GoalKind::Wash` → `recovery_relevant: false`

### 5. Update self-consume priority to distinguish recovery-relevant drives

For hunger/thirst commodities, the ranking layer must know whether a factor participates in wound recovery:
- Hunger factors (food commodities) → `true`
- Thirst factors (water commodities) → `true`

Changing `relevant_self_consume_factors()` to return a recovery-relevant marker is acceptable if it is the smallest way to keep `self_consume_priority()` honest.

### 6. Apply the same promotion rule in `self_consume_priority()`

When the 4th element is `true`, `context.has_clotted_wounds` is true, and base class is `High`, boost to `Critical`.

### 7. Add a cross-reference comment

Add a comment near the boost logic referencing `recovery_conditions_met()` in `crates/worldwake-systems/src/combat.rs` to document the coupling.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs`

## Out of Scope

- Changing `recovery_conditions_met()` in `crates/worldwake-systems/src/combat.rs`
- Reworking wound progression, pruning, or deprivation worsening in `worldwake-core` / `worldwake-systems`
- Changing `UtilityProfile` or adding personality parameters
- Changing `GoalBeliefView`
- Adding new `GoalKind` variants
- Any changes to `candidate_generation.rs`, `search.rs`, or authoritative action execution
- Golden-hash recapture unless a current golden test actually fails because of this ranking change

## Acceptance Criteria

### Tests That Must Pass

1. `clotted_wound_boosts_hunger_high_to_critical` — agent with clotted wound, hunger at high → eat goal priority is Critical
2. `bleeding_wound_no_boost` — agent with actively bleeding wound, hunger at high → priority stays High
3. `clotted_wound_no_boost_below_high` — agent with clotted wound, hunger below high → no boost
4. `clotted_wound_boosts_sleep_high_to_critical` — agent with clotted wound, fatigue at high → sleep goal priority is Critical
5. `clotted_wound_no_boost_relieve_or_wash` — agent with clotted wound, bladder/dirtiness at high → priority stays High
6. `no_wounds_no_boost` — agent with no wounds, hunger at high → priority stays High
7. `critical_stays_critical` — agent with clotted wound, hunger at critical → priority stays Critical (no double-boost)
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The boost applies only when: (a) agent has clotted wounds, (b) need is recovery-relevant, (c) base priority is exactly `High`
2. `Critical` is the maximum boost — no further elevation
3. No new fields on `UtilityProfile` or `GoalBeliefView`
4. The coupling with `recovery_conditions_met()` is documented via comment

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs::tests::clotted_wound_boosts_hunger_high_to_critical`
   Rationale: proves a recovery-blocking food need is promoted when the wound is clotted and the base class is exactly `High`.
2. `crates/worldwake-ai/src/ranking.rs::tests::bleeding_wound_no_boost`
   Rationale: proves active bleeding does not masquerade as a recoverable wound and incorrectly trigger the promotion.
3. `crates/worldwake-ai/src/ranking.rs::tests::clotted_wound_no_boost_below_high`
   Rationale: proves the rule is tied to the authoritative `high` recovery gate rather than any positive need pressure.
4. `crates/worldwake-ai/src/ranking.rs::tests::clotted_wound_boosts_sleep_high_to_critical`
   Rationale: proves direct fatigue-driven `Sleep` ranking uses the same recovery-aware rule.
5. `crates/worldwake-ai/src/ranking.rs::tests::clotted_wound_no_boost_relieve_or_wash`
   Rationale: proves unrelated self-care drives are not accidentally pulled into the recovery gate.
6. `crates/worldwake-ai/src/ranking.rs::tests::no_wounds_no_boost`
   Rationale: proves ordinary `High` self-care behavior is unchanged when no clotted wound exists.
7. `crates/worldwake-ai/src/ranking.rs::tests::critical_stays_critical`
   Rationale: proves the promotion is a one-step `High -> Critical` elevation rather than a separate scoring path.

### Commands

1. `cargo test -p worldwake-ai ranking -- --nocapture`
2. `cargo test -p worldwake-ai clotted_wound_boosts_hunger_high_to_critical -- --nocapture`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai`

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - Reassessed the ticket and corrected its scope to the still-missing `worldwake-ai` ranking work.
  - Added clotted-wound awareness to `RankingContext` in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs).
  - Promoted recovery-relevant `High` self-care priorities to `Critical` for clotted wounds, covering both `Sleep` and hunger/thirst self-consume goals.
  - Added a cross-reference comment tying the ranking rule to `recovery_conditions_met()` in combat.
  - Added focused ranking tests for clotted wounds, bleeding wounds, below-`High` needs, non-recovery drives, no wounds, and already-`Critical` needs.
- Deviations from original plan:
  - Did not reimplement deprivation lookup/worsening or wound hardening work because those changes and their focused tests were already present in `worldwake-core` and `worldwake-systems`.
  - Kept the solution local to `worldwake-ai` instead of introducing a broader shared recovery abstraction because there is only one live consumer today.
- Verification results:
  - `cargo test -p worldwake-ai clotted_wound_boosts_hunger_high_to_critical -- --nocapture` ✅
  - `cargo test -p worldwake-ai ranking -- --nocapture` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy -p worldwake-ai` ✅
