# S11WOULIFAUD-003: Deprivation wound worsening instead of duplication

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — needs system wound creation logic
**Deps**: `archive/tickets/completed/S11WOULIFAUD-001-woundlist-deprivation-lookup-methods.md`, `archive/specs/S11-wound-lifecycle-audit.md`

## Problem

`append_deprivation_wound()` in needs.rs unconditionally creates a new `Wound` each time a deprivation threshold fires. An agent starving repeatedly accumulates multiple starvation wounds with independent severities, cluttering the wound list and violating Principle 9 (aftermath should not be duplicated — it should worsen).

## Assumption Reassessment (2026-03-21)

1. `append_deprivation_wound()` is still the exact private authoritative construction site in [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs). `apply_deprivation_consequences()` calls it for both starvation and dehydration threshold firings, and the helper always allocates a fresh `WoundId`, pushes a new torso wound, and records `bleed_rate_per_tick = pm(0)`.
2. The ticket's original prerequisite is stale. `WoundList::find_deprivation_wound()` and `find_deprivation_wound_mut()` already exist in [`crates/worldwake-core/src/wounds.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/wounds.rs), and their focused core tests were already delivered in `archive/tickets/completed/S11WOULIFAUD-001-woundlist-deprivation-lookup-methods.md`.
3. Existing focused/unit coverage is broader than the ticket claimed, but it does not yet prove the desired "worsen instead of duplicate" behavior. Current needs tests in [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs) already cover single starvation creation, single dehydration creation, and tolerance-counter reset behavior via `needs_system_adds_starvation_wound_and_resets_hunger_exposure`, `needs_system_adds_dehydration_wound_and_resets_thirst_exposure`, and `needs_system_requires_another_full_tolerance_period_before_second_wound`.
4. Existing broader systems integration coverage also exists in [`crates/worldwake-systems/tests/e09_needs_integration.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/tests/e09_needs_integration.rs), specifically `scheduler_applies_starvation_and_dehydration_consequences_after_tolerance_windows`, which proves both deprivation kinds can coexist but does not exercise repeated firings of the same kind.
5. This is not an AI-pipeline ticket. The relevant architectural layer is the authoritative needs-system mutation path only; no candidate generation, ranking, plan search, action start, or plan-failure behavior changes here.
6. No ordering contract is involved. The invariant is authoritative wound-state mutation and identity preservation, not event-log, action-lifecycle, or planner ordering.
7. No heuristic/filter is being removed. The change replaces a too-thin write path with a cleaner authoritative mutation that preserves wound identity instead of duplicating it.
8. No mismatch in high-level direction: the spec still points at the right architectural improvement, but the ticket scope and dependency references were stale relative to the current repo. This ticket should target `needs.rs` only and must not re-open already completed core API work.

## Architecture Check

1. Replacing `append_deprivation_wound()` with a private `worsen_or_create_deprivation_wound()` helper in [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs) is cleaner than trying to deduplicate deprivation wounds later in combat, UI, or read-model code. The authoritative write path should encode the one-wound-per-kind invariant at creation time.
2. Preserving the existing wound and worsening its severity is architecturally stronger than aggregating duplicates at read time. It keeps `WoundId` stable, preserves Principle 4 object identity, avoids parallel representations of the same bodily state, and makes future wound-specific reasoning or treatment logic easier to extend.
3. A broader refactor into a generic "merge wounds by cause" abstraction would be over-designed for the current schema. Combat wounds are intentionally distinct and do not share this invariant. A deprivation-specific helper is the smaller, more robust change.
4. No backwards-compatibility aliasing or shim path is needed. The old helper should be replaced, not retained.

## Verification Layers

1. No existing deprivation wound for that kind -> focused needs unit test calling the helper directly in [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs)
2. Existing same-kind deprivation wound worsens in place and preserves `WoundId` -> focused needs unit test calling the helper directly in [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs)
3. Worsening saturates at `Permille(1000)` -> focused needs unit test calling the helper directly in [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs)
4. Different deprivation kinds still produce distinct wounds -> focused needs unit test calling the helper directly in [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs)
5. System-level repeated deprivation firing reuses the same wound record instead of appending a duplicate -> focused `needs_system` test in [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs)
6. Single-layer ticket: no additional AI, action-trace, or decision-trace verification layer is applicable because this ticket changes only authoritative needs-system wound mutation

## What to Change

### 1. Replace `append_deprivation_wound` with `worsen_or_create_deprivation_wound`

In `crates/worldwake-systems/src/needs.rs`, replace the existing function (lines 272-289) with:

```rust
fn worsen_or_create_deprivation_wound(
    wound_list: &mut Option<WoundList>,
    existing: Option<&WoundList>,
    kind: DeprivationKind,
    severity_increase: Permille,
    tick: Tick,
) {
    let list = wound_list.get_or_insert_with(|| existing.cloned().unwrap_or_default());
    if let Some(wound) = list.find_deprivation_wound_mut(kind) {
        wound.severity = wound.severity.saturating_add(severity_increase);
        wound.inflicted_at = tick;
    } else {
        let wound_id = list.next_wound_id();
        list.wounds.push(Wound {
            id: wound_id,
            body_part: BodyPart::Torso,
            cause: WoundCause::Deprivation(kind),
            severity: severity_increase,
            inflicted_at: tick,
            bleed_rate_per_tick: Permille::new(0).expect("zero is valid"),
        });
    }
}
```

### 2. Update all call sites

Replace every `append_deprivation_wound(...)` call with `worsen_or_create_deprivation_wound(...)`. The parameter names change (`severity` → `severity_increase`) but the positional signature is identical.

### 3. Add focused helper and system tests in `needs.rs` `mod tests`

Add the helper-level cases from acceptance criteria plus one `needs_system` regression that proves a second starvation threshold firing worsens the existing wound instead of appending a duplicate entry.

## Files to Touch

- `crates/worldwake-systems/src/needs.rs` (modify)

## Out of Scope

- Changing `WoundList` API in `worldwake-core` (already completed in `archive/tickets/completed/S11WOULIFAUD-001-woundlist-deprivation-lookup-methods.md`)
- Changing wound progression/pruning in `combat.rs` (already covered by `archive/tickets/completed/S11WOULIFAUD-002-wound-pruning-regression-coverage.md`)
- Any AI/ranking changes (`tickets/S11WOULIFAUD-004.md`)
- Golden hash recapture or broader golden-scenario updates (`archive/tickets/completed/S11WOULIFAUD-005.md`)
- Changing deprivation threshold logic or `MetabolismProfile`
- Changing `DeprivationExposure` counter reset behavior

## Acceptance Criteria

### Tests That Must Pass

1. `worsen_creates_new_when_no_existing` — empty/None wound list, call function, assert 1 wound created with correct kind, severity, tick
2. `worsen_increases_existing_severity` — existing starvation wound at pm(200), trigger with pm(500), assert severity pm(700) and same WoundId
3. `worsen_caps_at_permille_max` — existing at pm(800), worsen by pm(500), assert severity pm(1000)
4. `different_kinds_create_separate_wounds` — starvation wound exists, trigger dehydration, assert 2 wounds with distinct kinds
5. `worsen_updates_inflicted_at` — existing wound inflicted at Tick(5), worsen at Tick(50), assert inflicted_at is Tick(50)
6. `needs_system_second_starvation_threshold_worsens_existing_wound` — repeated starvation consequence firing through `needs_system` keeps one starvation wound, preserves `WoundId`, and increases severity
7. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. At most one deprivation wound per `DeprivationKind` per agent at any time
2. WoundId is preserved when worsening (Principle 4 — persistent identity)
3. `inflicted_at` reflects the tick of last worsening, not original creation
4. `severity` never exceeds `Permille(1000)` — `saturating_add` enforced
5. Conservation: no item/lot state is affected by this change

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs) `worsen_creates_new_when_no_existing` — proves the creation path still constructs an explicit deprivation wound when no same-kind wound exists
2. [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs) `worsen_increases_existing_severity` — proves same-kind deprivation damage accumulates on the existing wound and preserves `WoundId`
3. [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs) `worsen_caps_at_permille_max` — locks in the saturation boundary so the single wound cannot exceed max severity
4. [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs) `different_kinds_create_separate_wounds` — proves the one-wound invariant is per `DeprivationKind`, not global
5. [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs) `worsen_updates_inflicted_at` — proves the wound records the latest worsening tick
6. [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs) `needs_system_second_starvation_threshold_worsens_existing_wound` — proves the real system path now reuses the same wound on repeated starvation consequence firings

### Commands

1. `cargo test -p worldwake-systems worsen_creates_new_when_no_existing`
2. `cargo test -p worldwake-systems worsen_increases_existing_severity`
3. `cargo test -p worldwake-systems worsen_caps_at_permille_max`
4. `cargo test -p worldwake-systems different_kinds_create_separate_wounds`
5. `cargo test -p worldwake-systems worsen_updates_inflicted_at`
6. `cargo test -p worldwake-systems needs_system_second_starvation_threshold_worsens_existing_wound`
7. `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
8. `cargo test -p worldwake-systems`

## Outcome

- Completed: 2026-03-21
- What actually changed:
  - corrected the ticket first so it reflects the current repo: `WoundList::find_deprivation_wound[_mut]` already existed in `worldwake-core`, existing needs/integration coverage was named explicitly, and scope was narrowed to the authoritative `needs.rs` mutation path
  - replaced `append_deprivation_wound()` with `worsen_or_create_deprivation_wound()` in `crates/worldwake-systems/src/needs.rs`
  - starvation/dehydration threshold firings now worsen the existing same-kind deprivation wound in place, preserve `WoundId`, update `inflicted_at`, and saturate severity instead of appending duplicate wounds
  - added six focused `needs.rs` tests covering helper creation, in-place worsening, saturation, per-kind separation, timestamp update, and repeated starvation firing through `needs_system`
- Deviations from original plan:
  - no `worldwake-core` API work was needed because the lookup helpers had already been implemented and archived under S11WOULIFAUD-001
  - scope stayed entirely within `crates/worldwake-systems/src/needs.rs`; no combat, AI, or golden changes were required for this ticket
- Verification results:
  - `cargo test -p worldwake-systems worsen_`
  - `cargo test -p worldwake-systems different_kinds_create_separate_wounds`
  - `cargo test -p worldwake-systems needs_system_second_starvation_threshold_worsens_existing_wound`
  - `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
  - `cargo test -p worldwake-systems`
