# S104SURBASREC-007: Remove survival-baseline ProduceCommodity budget exhaustion

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — survival planning / candidate-generation surface in `worldwake-ai`
**Deps**: archive/tickets/S104SURBASREC-004.md

## Problem

`scenarios/survival-baseline.ron` now proves the intended survival outcomes at 1440 ticks, but the observer still captures four unique `BudgetExhausted` snapshots on `ProduceCommodity { recipe_id: RecipeId(2) }` during that same run. The scenario substrate is already minimal apples+water survival with no authored seeded beliefs and no authored social weighting, so the remaining exhaustion is an AI/planner issue rather than a scenario-authoring defect. Layer 0 (`S104SURBASREC-005`) should not pin the baseline while these survival-path planner exhaustions remain live noise.

## Assumption Reassessment (2026-04-15)

1. `scenarios/survival-baseline.ron` exists locally and survives a 1440-tick observer run with zero deaths, all five needs below the sustained-critical threshold, all five survival action families observed, and Agent B reaching `Fertile Fields`.
2. The remaining observer failures are bounded and reproducible from the scenario itself: `reports/survival-baseline-validation.md` Section 8 records 4 unique `BudgetExhausted` signatures, all on `ProduceCommodity { recipe_id: RecipeId(2) }`.
3. Those snapshots occur both before and after discovery of `Fertile Fields`, so the issue is not only "no food source known yet". Current snapshots include:
   - Agent B at tick 2 from unknown location with only `Riverside Camp` belief
   - Agent B / Agent C / Agent A later at `Fertile Fields`
4. The authored scenario was already narrowed during `S104SURBASREC-004` implementation to remove extra substrate that could have explained the blowup:
   - no authored seeded place knowledge
   - no authored grain recipe
   - no `FieldPlot` or grain source
   - no authored social utility pressure
   - `tell_profile` explicitly zeroed on all three agents
5. The live observer anomaly flags also still show heuristic `STUCK_AGENT` entries (`max_consecutive_idle = 36/41/27`), but those stay below the scenario ticket's actual `> 50` idle-stretch contract and do not by themselves prove an engine bug. The clearer engine issue here is the repeated survival-path budget exhaustion.
6. Intended layer is `worldwake-ai` planning/candidate generation, not scenario schema. The likely audit boundary is the survival food-acquisition goal family leading to `ProduceCommodity`, including candidate emission, operator choice, and search branching once an orchard-backed apple path is already available.

## Architecture Check

1. Fixing the planner/search surface directly is cleaner than burying the issue inside more scenario distortion. The scenario now already proves lawful survival; further authored narrowing would hide the engine behavior instead of resolving it.
2. No backwards-compatibility shims are needed. The correct outcome is a cleaner live survival planning path, not a special-case exemption for this scenario.

## Verification Layers

1. Survival baseline still succeeds at the authored scenario level -> observer report authoritative world-state summary (`zero deaths`, `ticks above 750‰`, action counts, visited locations)
2. `ProduceCommodity` survival-path exhaustion is gone -> observer report Section 8 budget-exhaustion snapshots
3. If planner changes are required -> focused `worldwake-ai` proof at the candidate/planning boundary plus `cargo test -p worldwake-ai`
4. Single concern ticket — scenario file should remain unchanged unless reassessment proves a smaller truthful setup correction still exists

## What to Change

### 1. Audit the live survival food-acquisition path

Trace why the survival baseline still emits `ProduceCommodity { recipe_id: RecipeId(2) }` budget-exhaustion snapshots even after the scenario was reduced to apples+water survival. Name the exact live `GoalKind`, candidate source, and search/operator surface responsible for the blowup.

### 2. Remove the exhaustion without weakening the survival baseline

Implement the narrowest honest `worldwake-ai` fix so the survival baseline no longer records survival-path `ProduceCommodity` budget exhaustion. This may land in candidate generation, planning, or search pruning, but it must remain general engine behavior rather than scenario-specific patching.

### 3. Re-verify against the real baseline scenario

Use the existing `scenarios/survival-baseline.ron` observer run as the final proof surface. Do not replace it with a sterile fixture-only success claim.

## Files to Touch

- `crates/worldwake-ai/src/` (modify — exact file to be determined during reassessment)
- `reports/survival-baseline-validation.md` (regenerated verification artifact, untracked is acceptable)
- `tickets/S104SURBASREC-007.md` (update during closeout)

## Out of Scope

- Re-authoring `scenarios/survival-baseline.ron` again unless reassessment finds a concrete remaining substrate defect
- Layer 0 golden tests (`S104SURBASREC-005`)
- Broader social / political / enterprise affordance cleanup not required to remove the survival-path exhaustion
- Observer heuristic noise such as redundant-perception counts unless it becomes the proven cause of the exhaustion

## Acceptance Criteria

### Tests That Must Pass

1. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output reports/survival-baseline-validation.md` — no Section 8 `BudgetExhausted` snapshots on survival-path `ProduceCommodity`
2. `cargo test -p worldwake-ai` — AI crate remains green after the engine fix
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean

### Invariants

1. `scenarios/survival-baseline.ron` continues to prove zero deaths and managed needs at 1440 ticks
2. The fix removes the live engine-side survival planning blowup rather than hiding it behind scenario-specific authored shortcuts

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/` focused test(s) near the changed planning/candidate surface — prove the narrowed engine contract that removes the exhaustion source
2. `None` for scenario file authoring — final proof remains the observer run on `scenarios/survival-baseline.ron`

### Commands

1. `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 1440 --output reports/survival-baseline-validation.md`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
