# S11WOULIFAUD-003: Deprivation wound worsening instead of duplication

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — needs system wound creation logic
**Deps**: S11WOULIFAUD-001 (requires `find_deprivation_wound_mut` on `WoundList`)

## Problem

`append_deprivation_wound()` in needs.rs unconditionally creates a new `Wound` each time a deprivation threshold fires. An agent starving repeatedly accumulates multiple starvation wounds with independent severities, cluttering the wound list and violating Principle 9 (aftermath should not be duplicated — it should worsen).

## Assumption Reassessment (2026-03-21)

1. `append_deprivation_wound()` at needs.rs line 272 is private. It creates a new `Wound` with `next_wound_id()`, `BodyPart::Torso`, `WoundCause::Deprivation(kind)`, zero `bleed_rate_per_tick`. Called from the deprivation threshold firing logic.
2. `WoundList::find_deprivation_wound_mut()` does not exist yet — added by S11WOULIFAUD-001.
3. Not an AI ticket. Pure systems-layer behavior change.
4. No ordering dependency.
5. N/A — no heuristic removal.
6. N/A.
7. N/A.
8. N/A.
9. N/A.
10. No mismatch. Spec matches codebase exactly.

## Architecture Check

1. Replacing `append_deprivation_wound` with `worsen_or_create_deprivation_wound` is the minimal change. The function remains private, same call sites, same parameter shape (plus `severity_increase` rename for clarity). WoundId preservation satisfies Principle 4.
2. No backwards-compatibility shims. Old function is removed entirely.

## Verification Layers

1. New wound created when none exists → focused unit test
2. Existing wound severity increased → focused unit test with WoundId assertion
3. Severity capped at Permille max → focused unit test
4. Different DeprivationKinds create separate wounds → focused unit test
5. `inflicted_at` updated on worsening → focused unit test
6. Single-layer ticket (systems crate wound creation). No AI/event/action layers involved.

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

### 3. Add five focused tests in needs.rs `mod tests`

Tests as specified in acceptance criteria below.

## Files to Touch

- `crates/worldwake-systems/src/needs.rs` (modify)

## Out of Scope

- Changing `WoundList` API (done in S11WOULIFAUD-001)
- Changing wound progression/pruning in combat.rs (done in S11WOULIFAUD-002)
- Any AI/ranking changes (done in S11WOULIFAUD-004)
- Golden test hash recapture (done in S11WOULIFAUD-005)
- Changing deprivation threshold logic or `MetabolismProfile`
- Changing `DeprivationExposure` counter reset behavior

## Acceptance Criteria

### Tests That Must Pass

1. `worsen_creates_new_when_no_existing` — empty/None wound list, call function, assert 1 wound created with correct kind, severity, tick
2. `worsen_increases_existing_severity` — existing starvation wound at pm(200), trigger with pm(500), assert severity pm(700) and same WoundId
3. `worsen_caps_at_permille_max` — existing at pm(800), worsen by pm(500), assert severity pm(1000)
4. `different_kinds_create_separate_wounds` — starvation wound exists, trigger dehydration, assert 2 wounds with distinct kinds
5. `worsen_updates_inflicted_at` — existing wound inflicted at Tick(5), worsen at Tick(50), assert inflicted_at is Tick(50)
6. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. At most one deprivation wound per `DeprivationKind` per agent at any time
2. WoundId is preserved when worsening (Principle 4 — persistent identity)
3. `inflicted_at` reflects the tick of last worsening, not original creation
4. `severity` never exceeds `Permille(1000)` — `saturating_add` enforced
5. Conservation: no item/lot state is affected by this change

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs.rs::tests::worsen_creates_new_when_no_existing` — creation path
2. `crates/worldwake-systems/src/needs.rs::tests::worsen_increases_existing_severity` — worsening path + identity
3. `crates/worldwake-systems/src/needs.rs::tests::worsen_caps_at_permille_max` — saturation boundary
4. `crates/worldwake-systems/src/needs.rs::tests::different_kinds_create_separate_wounds` — kind discrimination
5. `crates/worldwake-systems/src/needs.rs::tests::worsen_updates_inflicted_at` — timestamp update

### Commands

1. `cargo test -p worldwake-systems -- worsen_creates_new worsen_increases worsen_caps different_kinds worsen_updates`
2. `cargo clippy -p worldwake-systems`
3. `cargo test -p worldwake-systems`
