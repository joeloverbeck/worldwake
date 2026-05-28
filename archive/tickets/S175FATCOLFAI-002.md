# S175FATCOLFAI-002: Fatigue consequence path — exhaustion wound creation + death attribution

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` needs system (`apply_deprivation_consequences`, `determine_need_death_cause`)
**Deps**: S175FATCOLFAI-001

## Problem

`MetabolismProfile.exhaustion_collapse_ticks` (`crates/worldwake-core/src/needs.rs:160`) is named but has no live consumer: `apply_deprivation_consequences` (`crates/worldwake-systems/src/needs.rs:387`) creates wounds for `hunger_critical_ticks` (Starvation) and `thirst_critical_ticks` (Dehydration) but has no fatigue branch, and `DeprivationExposure.fatigue_critical_ticks` is incremented but never read. Fatigue is an unbounded loop with no terminal consequence. Separately, `determine_need_death_cause` (`needs.rs:248`) attributes deprivation deaths by comparing only `needs.hunger >= needs.thirst` — it never returns `Fatigue`, so an exhaustion-wound death would misattribute to `Hunger`, defeating the FND-29 traceability S175 exists to provide. This ticket wires both halves of the consequence path together (D2 + D4) plus the focused liveness test (D7 Scenario C).

## Assumption Reassessment (2026-05-28)

1. `apply_deprivation_consequences` (`needs.rs:387-445`) takes `(world, entity, tick, profile, &mut needs, &mut exposure)` and returns `(Option<WoundList>, Option<EntityId>)`. It has `if` branches for `hunger_critical_ticks` (`:398-408`, resets at `:406`) and `thirst_critical_ticks` (`:410-420`, resets at `:418`); a bladder accident branch (`:422-439`); and **no** fatigue branch. The local `wound_list: Option<WoundList>` and `wounds_changed: bool` are in scope; it returns `wounds_changed.then_some(...)` (`:441-444`). The spec's D2 pseudocode (`exposure.fatigue_critical_ticks >= profile.exhaustion_collapse_ticks.get()` → `worsen_or_create_deprivation_wound(&mut wound_list, world.get_component_wound_list(entity), DeprivationKind::Exhaustion, needs.fatigue, tick)` → reset + `wounds_changed = true`) matches the in-scope variable names and the helper signature exactly.
2. `worsen_or_create_deprivation_wound` (`needs.rs:447-470`) has signature `(&mut Option<WoundList>, Option<&WoundList>, DeprivationKind, Permille, Tick)`. `needs.fatigue` is a `Permille`. `profile.exhaustion_collapse_ticks` is `NonZeroU32`, so `.get()` is valid. `DeprivationExposure.fatigue_critical_ticks` exists (`needs.rs:117` region, `u32`).
3. `determine_need_death_cause(needs: HomeostaticNeeds)` (`needs.rs:248-255`) is the live attribution function (death is written by the needs system at `apply_pending_update`, `needs.rs:234`, gated by `is_wound_load_fatal` at `needs.rs:157`). It is **not** in `worldwake-core::combat` — the spec text was corrected for this during reassessment. `HomeostaticNeedId::Fatigue` already exists (`needs.rs:22`); `HomeostaticNeeds::value(need)` is the per-need accessor used by `update_exposure`/`critical_ticks`.
4. Existing inline tests in `needs.rs` `#[cfg(test)]` (boundary at `:484`) that exercise the changed functions: `needs_system_adds_starvation_wound_and_resets_hunger_exposure`:1260, `needs_system_adds_dehydration_wound_and_resets_thirst_exposure`:1313, `needs_system_kills_agent_from_deprivation_and_emits_death_event`:1363, `determine_need_death_cause_prefers_higher_pressure_and_breaks_ties_toward_hunger`:1512, `needs_system_requires_another_full_tolerance_period_before_second_wound`:1656, `needs_system_second_starvation_threshold_worsens_existing_wound`:1705. The `determine_need_death_cause_*` test must be **extended** (not adapted to a bug) to cover the fatigue case and the hunger/thirst/fatigue tie-break.
5. Adjacent-contradiction classification: extending `determine_need_death_cause` from a 2-need to a 3-need comparison is a **required consequence** of this ticket (without it, fatigue deaths misattribute). Two golden death-cause assertions must **not** regress: `crates/worldwake-ai/tests/scenarios/simulation_gaps.rs:777` and `crates/worldwake-ai/tests/scenarios/survival_self_care_interruption.rs:865`, both asserting `DeathCause::NeedDeprivation { need: Hunger }`. Adding `Fatigue` as a third comparand with the stable-max tie-break (`Hunger` first) preserves the result wherever fatigue is not the dominant pressure — both scenarios zero/limit fatigue relative to hunger, so neither flips.
6. Cumulative-arithmetic envelope: collapse fires only when `fatigue_critical_ticks` reaches `exhaustion_collapse_ticks` *without* dropping below the critical threshold (the existing `critical_ticks` helper at `needs.rs:472-482` resets the counter to 0 on any below-critical tick — D3, no code change). Each full interval creates/worsens one Exhaustion wound and resets the counter; repeated intervals raise wound load until `wound_load >= wound_capacity` (`is_wound_load_fatal`, `needs.rs:157`). The focused test uses a low `exhaustion_collapse_ticks` (60/120) to make the first wound reachable in a tractable horizon.

## Architecture Check

1. Merging D2 (wound creation) and D4 (death attribution) into one ticket avoids a transient FND-28-adjacent state where exhaustion wounds form but the resulting death misattributes to `Hunger` — both halves of the fatigue-collapse contract land atomically in the same file and test module. The fatigue branch mirrors the existing starvation/dehydration `if` blocks exactly, so the consequence pattern stays uniform (FND-3, FND-11: death is the terminal dampener).
2. No backwards-compatibility shim: `exhaustion_collapse_ticks` becomes live with no parallel "old fatigue path." Attribution stays a single mechanism (need-pressure comparison) extended to three needs rather than introducing a second wound-dominant attribution path beside it (FND-28).

## Verification Layers

1. Fatigue critical exposure ≥ threshold creates an `Exhaustion` wound and resets the counter -> focused unit test on `apply_deprivation_consequences` (authoritative `WoundList` + `DeprivationExposure` state).
2. Exhaustion-wound load ≥ capacity produces `DeathCause::NeedDeprivation { need: Fatigue }` -> focused unit test on `determine_need_death_cause` (authoritative `DeadAt` payload) + extended attribution unit test.
3. Existing hunger/thirst attribution unchanged -> the extended `determine_need_death_cause_prefers_higher_pressure_and_breaks_ties_toward_hunger` test (event-log/authoritative state) plus the two unchanged AI goldens named in Assumption Reassessment item 5.
4. Profile-field liveness (read each tick, not cached at spawn) -> focused unit test with two different `exhaustion_collapse_ticks` values (60 vs 120) producing wounds at the corresponding ticks (D7 Scenario C).

## What to Change

### 1. Fatigue branch in `apply_deprivation_consequences` (D2)

After the dehydration branch (`needs.rs:420`), add a fatigue branch mirroring the starvation/dehydration shape: when `exposure.fatigue_critical_ticks >= profile.exhaustion_collapse_ticks.get()`, call `worsen_or_create_deprivation_wound(&mut wound_list, world.get_component_wound_list(entity), DeprivationKind::Exhaustion, needs.fatigue, tick)`, then set `exposure.fatigue_critical_ticks = 0` and `wounds_changed = true`.

### 2. Extend `determine_need_death_cause` to attribute fatigue (D4)

Replace the `hunger >= thirst` two-need comparison with a stable max over the three wound-bearing needs `[Hunger, Thirst, Fatigue]` by `needs.value(*need)`, preserving the existing tie-break order (Hunger first). Bladder and dirtiness are excluded — they have no killing-wound path. Document inline that attribution is by need pressure (not dominant wound), per spec D4.

### 3. Extend the attribution unit test (D7 Scenario C, part)

Extend `determine_need_death_cause_prefers_higher_pressure_and_breaks_ties_toward_hunger` (`needs.rs:1512`) to assert: fatigue-dominant pressure → `Fatigue`; three-way tie → `Hunger` (stable-max order).

### 4. Focused liveness tests (D7 Scenario C)

Add focused needs-system tests proving: (a) with `exhaustion_collapse_ticks = nz(60)`, sustained critical fatigue creates an Exhaustion wound at tick 60 and resets `fatigue_critical_ticks`; (b) with `exhaustion_collapse_ticks = nz(120)`, the wound appears at tick 120; (c) continued exposure worsens/adds a wound and eventually `determine_need_death_cause` yields `Fatigue` once wound load is fatal. These mirror `needs_system_adds_starvation_wound_and_resets_hunger_exposure` and `needs_system_kills_agent_from_deprivation_and_emits_death_event`.

## Files to Touch

- `crates/worldwake-systems/src/needs.rs` (modify — fatigue branch, attribution function, inline `#[cfg(test)]` tests)

## Out of Scope

- The `DeprivationKind::Exhaustion` variant itself (S175FATCOLFAI-001, dependency).
- The `exhaustion_collapse_observed` forensic flag (S175FATCOLFAI-003).
- Full E2E golden scenarios A/B (S175FATCOLFAI-004) — this ticket carries only the focused unit-level liveness test (D7 Scenario C).
- D3 recovery reset: no code change — the existing `critical_ticks` helper (`needs.rs:472-482`) already zeroes `fatigue_critical_ticks` below critical. Verified by 004 Scenario B.
- Wound-dominant death attribution (spec Non-Goal / Open Question 2 — deferred to a future mixed-deprivation spec).

## Acceptance Criteria

### Tests That Must Pass

1. With `exhaustion_collapse_ticks = nz(60)`, an agent at sustained critical fatigue gains a `WoundCause::Deprivation(DeprivationKind::Exhaustion)` wound at tick 60, and `fatigue_critical_ticks` resets to 0.
2. With `exhaustion_collapse_ticks = nz(120)`, the same agent gains the wound at tick 120 (field read per tick, not cached).
3. `determine_need_death_cause` returns `Fatigue` when fatigue is the dominant pressure, `Hunger` on a three-way tie.
4. The two AI goldens (`simulation_gaps.rs`, `survival_self_care_interruption.rs`) still assert `Hunger` — no regression.
5. Existing suites: `cargo test -p worldwake-systems needs` and `cargo test -p worldwake-ai -- simulation_gaps survival_self_care_interruption`

### Invariants

1. The fatigue branch resets `fatigue_critical_ticks` to 0 on wound creation, identical to the starvation/dehydration reset semantics.
2. Attribution remains a single mechanism (need-pressure comparison) extended to three needs — no parallel wound-dominant attribution path coexists (FND-28).
3. Exhaustion wounds contribute to `wound_load` exactly like other deprivation wounds; the death trigger (`is_wound_load_fatal`) is unchanged.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs.rs` (`#[cfg(test)]`) — new focused tests: exhaustion-wound-at-threshold (60 and 120), reset-on-creation, second-wound/worsening, fatigue death attribution. Rationale: D2 + D4 + D7 Scenario C liveness.
2. `crates/worldwake-systems/src/needs.rs` — extend `determine_need_death_cause_prefers_higher_pressure_and_breaks_ties_toward_hunger` for the fatigue case and three-way tie. Rationale: D4 attribution contract.

### Commands

1. `cargo test -p worldwake-systems needs`
2. `cargo test -p worldwake-ai -- simulation_gaps survival_self_care_interruption` (no-regression guard for the Hunger goldens)
3. `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
4. `scripts/verify.sh`

## Outcome

**Completion date**: 2026-05-28

**What changed** (all in `crates/worldwake-systems/src/needs.rs`):
- Added the fatigue branch to `apply_deprivation_consequences`: when `exposure.fatigue_critical_ticks >= profile.exhaustion_collapse_ticks.get()`, it calls `worsen_or_create_deprivation_wound(..., DeprivationKind::Exhaustion, needs.fatigue, tick)`, resets `fatigue_critical_ticks = 0`, and sets `wounds_changed = true` — mirroring the starvation/dehydration branches exactly.
- Extended `determine_need_death_cause` from a two-need (`hunger >= thirst`) comparison to a stable max over the three wound-bearing needs (hunger, thirst, fatigue) by `needs.value(need)`.
- Extended the attribution unit test to cover fatigue-dominant, three-way tie, thirst/fatigue tie, and bladder/dirtiness-never-win cases.
- Added three focused needs-system tests: exhaustion wound at threshold 60 + counter reset; threshold-read-per-tick liveness (120-tick profile produces no wound at counter 60, wound at 120); and exhaustion-wound-load death attributed to `Fatigue`.

**Deviation from ticket pseudocode (corrected spec error)**: Spec D4 / ticket step 2 specified
`[Hunger, Thirst, Fatigue].into_iter().max_by_key(...)` with the comment "stable max keeps the first listed on ties". This is **factually wrong about Rust stdlib semantics**: `Iterator::max_by_key` returns the **last** maximal element on ties, so that listing would attribute a three-way tie to `Fatigue`, violating the ticket's own acceptance criterion (#3: "three-way tie → Hunger"). The implementation instead lists the needs in **reverse** tie-break priority — `[Fatigue, Thirst, Hunger]` — so that "last maximal wins" yields the intended hunger > thirst > fatigue priority. The behavioral contract (hunger > thirst > fatigue on ties) is exactly as the spec intended; only the mechanism is corrected. An inline comment documents this.

**Verification**:
- `cargo test -p worldwake-systems needs` — 76 passed (includes the 3 new fatigue tests + extended attribution test).
- `cargo test -p worldwake-ai --test golden_ai -- --ignored survival_self_care_interruption simulation_gaps` — 7 passed; both `Hunger`-death goldens unchanged (the two assertions at `simulation_gaps.rs:777` and `survival_self_care_interruption.rs:865` are in CI-only `#[ignore]` goldens; run explicitly to confirm no regression).
- `cargo clippy -p worldwake-systems --all-targets -- -D warnings` — clean.
