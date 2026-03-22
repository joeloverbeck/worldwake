**Status**: COMPLETED

# Wound Lifecycle Overhaul

## Summary

Expand the wound lifecycle from its current minimal implementation to a robust, well-tested subsystem with three improvements:

1. **Investigation & pruning hardening**: Diagnose an observed wound disappearance anomaly (wounds with `natural_recovery_rate: pm(0)` vanishing), harden pruning contract with explicit tests.
2. **Deprivation wound worsening**: When a deprivation threshold fires and the agent already has a wound of the same `DeprivationKind`, worsen that wound instead of creating a duplicate.
3. **Recovery-aware AI priority boost**: When an agent has clotted (non-bleeding) wounds and a recovery-relevant need (hunger, thirst, fatigue) is at or above the `high` threshold — blocking natural wound recovery — boost that need goal's priority class from `High` to `Critical`.

Originally an investigation-only spec (wound disappearance anomaly). Expanded after codebase reassessment revealed the anomaly's `no_recovery_combat_profile()` workaround has been removed, and architectural gaps exist in how wounds interact with deprivation accumulation and AI decision-making.

## Discovered Via

Golden E2E emergent tests (S07 care interaction coverage). A fighter wounded during combat had `natural_recovery_rate: pm(0)` set via combat profile override, yet `wound_load` returned to 0 within ~100 ticks. The wound was observed (`fighter_wounded=true`) at an intermediate tick but absent at the scenario's end.

Codebase reassessment (2026-03-21) identified additional gaps:
- Deprivation wounds stack as duplicates instead of worsening, cluttering the wound list
- AI has zero awareness of `recovery_conditions_met()` — wound recovery is an accidental side-effect of satisfying needs

## Foundation Alignment

- **Principle 3** (Concrete State Over Abstract Scores): Wounds are concrete state. Pruning, worsening, and recovery must be fully deterministic and traceable. The AI recovery boost derives from concrete wound state (bleed rates, severities), not abstract scores.
- **Principle 9** (Outcomes Are Granular and Leave Aftermath): Combat and deprivation wounds are aftermath. If they silently vanish or stack as duplicates, the aftermath is lost or cluttered. Worsening a single wound preserves identity while accumulating consequences.
- **Principle 12** (World State Is Not Belief State): The AI recovery boost reads wounds from the belief view, not world state. The agent's perceived wound state drives its care priority.
- **Principle 20** (Agent Diversity Through Concrete Variation): Recovery boost emerges differently per agent through their individual `DriveThresholds`, metabolisms, and wound states. No new uniform personality parameter is needed.
- **Principle 4** (Persistent Identity): Worsened deprivation wounds preserve their `WoundId`. Identity is stable across severity increases.

## Phase

Phase 3: Information & Politics (investigation + targeted enhancements, no phase dependency)

## Crates

- `worldwake-core` (wound types, `WoundList`, `WoundCause`)
- `worldwake-systems` (wound processing in `needs_system`, wound progression in `combat_system`)
- `worldwake-ai` (ranking priority boost in `ranking.rs`)

## Dependencies

None. All required infrastructure exists: `WoundList`, `CombatProfile`, `GoalBeliefView::wounds()`, `DriveThresholds`, `UtilityProfile`, `classify_band()`.

## Engine Changes

### A. Investigation & Pruning Hardening

#### Root Cause Analysis

The `progress_wounds()` function in `crates/worldwake-systems/src/combat.rs` (line 192):
- Bleeding: `severity += bleed_rate`, then `bleed_rate -= clot_resistance`, skip recovery
- Recovery: if `can_recover` AND `severity > 0`: `severity -= recovery_rate`
- Pruning (line 224): `next.wounds.retain(|w| w.severity.value() > 0)`

With `natural_recovery_rate: pm(0)`, `Permille::saturating_sub(pm(0))` is a no-op — severity cannot reach 0 through the recovery path. The anomaly likely came from a test setup issue (e.g., combat profile override ordering) or code that has since been refactored. The `no_recovery_combat_profile()` workaround referenced in the original spec no longer exists in the codebase.

#### Four Hypotheses (retained from original spec)

**H1: Wound pruning has a severity floor** — Wounds may be pruned when severity reaches 0 or epsilon, even with recovery_rate pm(0). Test: wound with severity pm(200), bleed_rate pm(0), recovery_rate pm(0). Tick 50 times. Assert severity unchanged and wound not pruned.

**H2: Bleed/clot arithmetic underflow** — Clot resistance applied to severity instead of bleed_rate, or subtraction ordering issue. Test: wound with severity pm(100), bleed_rate pm(50), clot_resistance pm(25), recovery_rate pm(0). Tick until clotted. Assert severity = initial + total accumulated bleed.

**H3: Combat profile override not taking effect** — Component table merge ordering drops the override. Test: override profile in transaction, read back immediately, assert recovery_rate matches override.

**H4: Wound system has minimum recovery rate** — Recovery code clamps to minimum of 1 or applies recovery regardless of profile value. Test: inspect `progress_wounds()` source for any floor on recovery_rate.

#### Pruning Contract

Harden the pruning contract: a wound is pruned if and only if its `severity` reaches `Permille(0)`. Add `#[cfg(debug_assertions)]` contract check before the retain line in `progress_wounds()` to catch contract violations during testing.

### B. Deprivation Wound Worsening

#### Current Behavior

`append_deprivation_wound()` in `crates/worldwake-systems/src/needs.rs` (line 272) unconditionally creates a NEW `Wound` each time a deprivation threshold fires. An agent starving repeatedly accumulates multiple starvation wounds with independent severities.

#### New Behavior

If the agent already has a deprivation wound of the same `DeprivationKind` (checked via a new `WoundList::find_deprivation_wound_mut()` lookup), increase that wound's severity via `saturating_add` instead of creating a duplicate.

**WoundList API additions** (`crates/worldwake-core/src/wounds.rs`):

```rust
/// Returns an immutable reference to the first wound caused by the given DeprivationKind.
pub fn find_deprivation_wound(&self, kind: DeprivationKind) -> Option<&Wound>

/// Returns a mutable reference to the first wound caused by the given DeprivationKind.
pub fn find_deprivation_wound_mut(&mut self, kind: DeprivationKind) -> Option<&mut Wound>
```

**Replacement function** (`crates/worldwake-systems/src/needs.rs`):

Replace `append_deprivation_wound` with `worsen_or_create_deprivation_wound`:

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
            bleed_rate_per_tick: Permille::new(0).expect("zero is a valid permille"),
        });
    }
}
```

**Design decisions**:
- **`inflicted_at` updated to current tick**: Records when last worsened. More useful for AI reasoning and display than the original creation time.
- **WoundId preserved**: The existing wound keeps its original ID. Identity is stable (Principle 4).
- **`saturating_add` caps at 1000**: A single wound cannot exceed `Permille` maximum.
- **Handles partially-healed wounds**: If wound was at severity 100 (recovered from 400) and worsening adds 800, result is `saturating_add(100, 800) = 900`.

### C. Recovery-Aware AI Priority Boost

#### Problem

The combat system recovers clotted wounds only when `recovery_conditions_met()` (`crates/worldwake-systems/src/combat.rs`, line 230): not in combat AND `hunger < thresholds.hunger.high()` AND `thirst < thresholds.thirst.high()` AND `fatigue < thresholds.fatigue.high()`. The AI ranking system has zero awareness of these conditions — wound recovery is an accidental side-effect of satisfying needs independently.

#### Solution

When ranking need goals (eat, drink, sleep), if the agent has clotted wounds AND the corresponding need is at or above the `high` threshold (the recovery-blocking level), boost the priority class from `High` to `Critical`.

**File: `crates/worldwake-ai/src/ranking.rs`**

1. Add helper to detect clotted wounds from beliefs:
```rust
fn has_clotted_wounds(view: &dyn GoalBeliefView, agent: EntityId) -> bool {
    view.wounds(agent).iter().any(|w| w.bleed_rate_per_tick.value() == 0 && w.severity.value() > 0)
}
```

2. Add `has_clotted_wounds: bool` field to `RankingContext`. Compute in `RankingContext::new()`.

3. Add `recovery_relevant: bool` parameter to `drive_priority()`:
```rust
fn drive_priority(
    context: &RankingContext<'_>,
    pressure: impl Fn(HomeostaticNeeds) -> Permille,
    band: impl Fn(DriveThresholds) -> ThresholdBand,
    recovery_relevant: bool,
) -> GoalPriorityClass {
    let base = match (context.needs, context.thresholds) {
        (Some(needs), Some(thresholds)) => classify_band(pressure(needs), &band(thresholds)),
        _ => GoalPriorityClass::Background,
    };
    if recovery_relevant && context.has_clotted_wounds && base == GoalPriorityClass::High {
        GoalPriorityClass::Critical
    } else {
        base
    }
}
```

4. Update call sites in `priority_class()`:
   - `GoalKind::Sleep` → `recovery_relevant: true`
   - `GoalKind::Relieve` → `recovery_relevant: false` (bladder is not a recovery condition)
   - `GoalKind::Wash` → `recovery_relevant: false` (dirtiness is not a recovery condition)

5. Update `relevant_self_consume_factors()` return type to include a 4th `bool` element:
   - Hunger factors (food) → `true`
   - Thirst factors (water) → `true`

6. Update `self_consume_priority()` to apply the boost when the 4th element is `true`, `context.has_clotted_wounds`, and base class is `High`.

7. Add comment referencing `recovery_conditions_met()` in `combat.rs` to document the coupling.

**No changes needed elsewhere**:
- `UtilityProfile` — no new field. The boost is a logical consequence ("my wound cannot heal until I eat"), not a personality trait. Per-agent diversity already comes from different thresholds and metabolisms.
- `GoalBeliefView` — `wounds()` already returns `Vec<Wound>` with full `bleed_rate_per_tick` data.
- No new `GoalKind`.

## FND-01 Section H Analysis

### 1. Information-Path Analysis

- **Wound worsening**: Needs system reads `HomeostaticNeeds`, `DeprivationExposure`, `MetabolismProfile`, `WoundList` from `World`. Writes updated `WoundList` via `WorldTxn`. All per-agent, local. No cross-agent information flow.
- **Wound progression**: Combat system reads `WoundList`, `CombatProfile`, `HomeostaticNeeds`, `DriveThresholds`, active actions. Writes updated `WoundList` via `WorldTxn`. Per-agent, local.
- **AI recovery boost**: Reads wounds from belief view (Principle 12 — not world state directly). Reads needs from beliefs. Per-agent, local.
- **No new information channels**: All data paths already exist. The spec adds logic that operates on existing per-agent state.

### 2. Positive-Feedback Analysis

**Identified amplifying loop**: Deprivation wound worsening:
hunger high → deprivation wound created/worsened → wound_load increases → if approaching incapacitation, agent cannot act → cannot eat → hunger stays high → further worsening

This loop existed before (with new wound creation per threshold hit), but worsening concentrates damage in a single wound rather than spreading it across duplicates. The quantitative dynamics are similar — total wound_load accumulates at the same rate.

### 3. Concrete Dampeners

- **Tolerance period**: `starvation_tolerance_ticks` / `dehydration_tolerance_ticks` in `MetabolismProfile` (`NonZeroU32`, minimum 1) ensures a minimum delay between worsening events. The exposure counter resets to 0 after each firing. Physical analogue: the body takes time to deteriorate further.
- **Recovery gate**: When an agent is fed, hydrated, and rested (needs below `high` threshold) and not in combat, wounds recover at `natural_recovery_rate` per tick. Physical process that counteracts the worsening.
- **Permille ceiling**: `saturating_add` caps severity at 1000. A single wound cannot exceed this. Physical limit: a wound cannot be "more than maximally severe."
- **Death**: `wound_load >= wound_capacity` terminates the agent. Ultimate dampener — the feedback loop ends.

### 4. Stored State vs. Derived Read-Model

**Stored (authoritative)**:
- `WoundList` component — `Vec<Wound>` per agent (id, body_part, cause, severity, inflicted_at, bleed_rate_per_tick)
- `DeprivationExposure` component — per-agent tick counters (hunger_critical_ticks, thirst_critical_ticks, bladder_critical_ticks)
- `CombatProfile` component — per-agent recovery/clot/capacity parameters
- `HomeostaticNeeds` component — per-agent need levels
- `DriveThresholds` component — per-agent threshold bands

**Derived (transient, never stored)**:
- `wound_load()` — sum of wound severities, computed on demand
- `is_incapacitated()` — wound_load vs profile threshold
- `is_wound_load_fatal()` — wound_load vs profile capacity
- `has_bleeding_wounds()` — iterates wounds
- `recovery_conditions_met()` — needs vs thresholds + combat state
- `has_clotted_wounds()` — iterates wounds (new, AI-side)
- Pain pressure, danger pressure — derived from wounds in belief view

No derived value is stored as authoritative state.

## Deliverables

### 1. Core API: WoundList Lookup Methods

Add `find_deprivation_wound()` and `find_deprivation_wound_mut()` to `WoundList` in `crates/worldwake-core/src/wounds.rs`.

**Tests** (in `wounds.rs`):
- `find_deprivation_wound_returns_match` — starvation + combat wound list, find starvation, miss dehydration
- `find_deprivation_wound_mut_updates_severity` — find and modify, assert list reflects change
- `find_deprivation_wound_returns_none_for_empty_list`

### 2. Investigation & Pruning Hardening Tests

Add focused tests in `crates/worldwake-systems/` that isolate each hypothesis:

- `zero_recovery_rate_wound_persists` — Clotted wound, pm(0) recovery, tick 50 times, assert unchanged
- `wound_bleed_clot_arithmetic_exact` — Known bleed/clot parameters, assert severity = initial + total bleed
- `pruning_only_at_severity_zero` — Mixed-severity list, assert only severity-0 wounds pruned
- `progress_wounds_returns_none_when_no_change` — Non-bleeding, pm(0) recovery, assert `None` return

Add `#[cfg(debug_assertions)]` pruning contract check in `progress_wounds()`.

### 3. Deprivation Wound Worsening

Replace `append_deprivation_wound` with `worsen_or_create_deprivation_wound` in `crates/worldwake-systems/src/needs.rs`.

**Tests**:
- `worsen_creates_new_when_no_existing` — empty list, assert 1 wound created
- `worsen_increases_existing_severity` — existing wound at pm(200), trigger with pm(500), assert pm(700), same WoundId
- `worsen_caps_at_permille_max` — existing at pm(800), worsen by pm(500), assert pm(1000)
- `different_kinds_create_separate_wounds` — starvation exists, trigger dehydration, assert 2 wounds
- `worsen_updates_inflicted_at` — existing at Tick(5), worsen at Tick(50), assert Tick(50)

### 4. Recovery-Aware AI Priority Boost

Modify `crates/worldwake-ai/src/ranking.rs`: add `has_clotted_wounds` to `RankingContext`, add `recovery_relevant` parameter to `drive_priority()`, update `relevant_self_consume_factors()` and `self_consume_priority()`.

**Tests** (in `ranking.rs` test module):
- `clotted_wound_boosts_hunger_high_to_critical`
- `bleeding_wound_no_boost`
- `clotted_wound_no_boost_below_high`
- `clotted_wound_boosts_sleep_high_to_critical`
- `clotted_wound_no_boost_relieve_or_wash`
- `no_wounds_no_boost`
- `critical_stays_critical`

### 5. Golden Test Verification

Run all golden tests. Deprivation worsening changes may shift deterministic hashes in scenarios where agents accumulate deprivation wounds. Recapture hashes for any affected golden tests.

## Risks

Low-to-moderate. The investigation and pruning hardening are risk-free (adding tests to existing code). Deprivation worsening changes wound accumulation behavior, which may shift golden test hashes. The AI priority boost is a small, well-bounded change to ranking logic.

No architectural changes — all modifications use existing types, traits, and patterns. The `WoundList` API additions are pure extensions. The ranking changes add a parameter to existing functions.

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - added `WoundList::find_deprivation_wound()` and `find_deprivation_wound_mut()` in `worldwake-core`
  - hardened wound progression/pruning coverage in `worldwake-systems::combat`, including zero-recovery persistence and pruning-at-zero-only checks
  - replaced duplicate deprivation wound creation with in-place worsening in `worldwake-systems::needs`
  - added recovery-aware clotted-wound promotion for hunger/thirst/fatigue ranking in `worldwake-ai::ranking`
  - verified the relevant golden suites and workspace checks without needing additional golden hash recapture
- Deviations from original plan:
  - the originally suspected zero-recovery "wound disappearance" anomaly was not reproduced as a live engine bug; the shipped work focused on contract hardening and adjacent architectural gaps instead of a speculative hotfix
  - the spec's planned golden recapture step closed out as verification-only because current goldens already passed after the delivered changes
- Verification results:
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace` ✅
