# S41BANOFFEME-001: Reassess S41 Spec Against Current Codebase

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Potentially — see reassessment findings below
**Deps**: S41 spec (`specs/S41-bandit-offensive-emergence-goldens.md`)

## Problem

S41 defines three golden test suites covering offensive bandit dynamics (raid emergence, belief-based economic cascade, wound dampening). Before implementing any test code, every setup assumption must be validated against current engine behavior. This ticket performs that validation and corrects the spec where assumptions diverge.

## Assumption Reassessment (2026-03-30)

### Suite 1 (Scenario 47): Pressure-Driven Raid Emergence

1. **`GoalKind::RaidTarget { target }`** — confirmed live at `crates/worldwake-core/src/goal.rs:134`. Candidate generation emits it via `emit_raid_target_goals()` at `crates/worldwake-ai/src/candidate_generation.rs:1432–1471`.
2. **Raid candidate emission requires co-location + non-faction target** — confirmed. `local_raid_targets()` is called at line 1453; the function checks co-location and faction membership.
3. **Raid emission is suppressed when `derive_danger_pressure(ctx.view, ctx.agent) >= thresholds.danger.high()`** — confirmed at lines 1437–1444. This means bandits under active attack or with visible hostiles + wounds will not emit raid candidates.
4. **`GoalKind::LootCorpse { corpse }`** — confirmed live at `crates/worldwake-core/src/goal.rs:136`.
5. **`verify_live_lot_conservation()`** — confirmed available, used in multiple golden tests (e.g., `golden_production.rs`, `golden_emergent.rs`).
6. **Existing T22 helpers reusable**: `bandit_profile()`, `default_perception_profile()`, `connect()`, `build_custom_harness()`, `seed_agent_with_recipes()`, `stable_wound_list()` — all confirmed in `golden_t22_bandit_camp_destruction.rs` and `golden_harness/mod.rs`.
7. **RaidTarget ranking**: Priority class = `Medium` (ranking.rs:344), motive score = `enterprise_weight` (ranking.rs:518). The spec's `bandit_utility_profile()` sets `enterprise_weight: pm(0)`, which means RaidTarget motive score will be 0 even at Medium priority. **Correction needed**: Bandits for Suite 1 need non-zero `enterprise_weight` for raid goals to have meaningful motive scores, or hunger-driven self-consume must outcompete. The spec setup with `enterprise_weight: pm(0)` will produce RaidTarget at Medium/0 motive — it may still be selected if hunger goals are also Medium, but this is fragile. **Recommend**: Suite 1 bandits use a custom `UtilityProfile` with `enterprise_weight >= pm(300)`.

### Suite 2 (Scenario 48): Raid-Belief Economic Cascade

1. **`GoalKind::ShareBelief { listener, .. }`** — confirmed live at `crates/worldwake-core/src/goal.rs:63`.
2. **Tell action** — confirmed at `crates/worldwake-systems/src/tell_actions.rs`.
3. **`BelievedActivity`** — confirmed as a component on belief state at `crates/worldwake-core/src/belief.rs`.
4. **Merchant route adaptation**: The spec assumes merchant's planner selects routes based on danger beliefs. This depends on the planner's travel-cost computation weighting danger. **Needs verification**: Does the planner's travel-cost heuristic incorporate danger beliefs from `BelievedActivity`? If not, the merchant won't reroute regardless of beliefs. This is the critical engine dependency for Suite 2.
5. **Witness `ShareBelief` candidate generation**: Witnesses must generate `ShareBelief` candidates for observed combat events. **Needs verification**: Does `emit_share_belief_candidates()` in `candidate_generation.rs` emit sharing for `BelievedActivity` observations?
6. **4-place + RemoteFarm topology**: Not in T22's existing topology. Suite 2 needs its own topology builder function. The spec correctly identifies this.
7. **`MerchandiseProfile`, `DemandMemory`** — confirmed available in `worldwake-core`. Merchant agents need these components for enterprise goal generation.

### Suite 3 (Scenario 49): Wound-Dampened Raid Spiral

**CRITICAL DIVERGENCE**: The spec claims wound accumulation raises `ReduceDanger` pressure to suppress raids at the ranking layer. This is **incorrect** against current code:

1. **`ReduceDanger` priority** comes from `danger_class` (ranking.rs:367–369), which is derived from `derive_danger_pressure()` (pressure.rs:69–71).
2. **`derive_danger_pressure()`** returns 0 when `current_attackers.is_empty() && visible_hostiles.is_empty()` (pressure.rs:28–29). Wounds alone produce **zero** danger pressure without visible hostiles.
3. **Pain pressure** (`derive_pain_pressure()`, pressure.rs:61–67) sums wound severities, but only feeds `TreatWounds` priority (ranking.rs:370–377), not `ReduceDanger`.
4. **RaidTarget suppression rule** is `SuppressionRule::Never` (goal_policy.rs:151–156). Raids are never suppressed by stress/danger/self-care class.
5. **RaidTarget priority class** is always `Medium` (ranking.rs:344). No wound-based promotion or demotion exists.
6. **RaidTarget candidate emission** is only suppressed when `danger_pressure >= thresholds.danger.high()` (candidate_generation.rs:1437–1444), which requires visible hostiles or current attackers — NOT wound state alone.

**Consequence**: In the spec's setup (targets arrive, bandit fights them, targets die, new target arrives), after each combat the hostiles are gone (dead). Wound accumulation alone will NOT suppress raid candidates or outcompete them at ranking. The spec's claimed dampening mechanism does not exist in current code.

**Options**:
   - **Option A**: Modify the spec to test a mechanism that DOES exist — e.g., wounds + visible hostiles (keep a surviving hostile nearby so danger pressure stays elevated).
   - **Option B**: Introduce engine changes: make `emit_raid_target_goals()` also check pain pressure against a threshold (e.g., `flee_wound_threshold` from `BanditFactionPolicy`), adding wound-aware raid suppression at the candidate-generation layer.
   - **Option C**: Introduce ranking changes: make `RaidTarget` priority class wound-sensitive (e.g., demote to `Low` when pain pressure exceeds a threshold).

**Recommendation**: Option B — add wound-aware suppression to `emit_raid_target_goals()` using the faction's `flee_wound_threshold`. This aligns with FND-10 (physical dampener) and the `flee_wound_threshold` field that already exists on `BanditFactionPolicy` but is currently unused in candidate generation. This would be a small, targeted engine change that makes the spec's claimed behavior real.

### Cross-Suite

8. **Scenario IDs 47, 48, 49** — confirmed available. Existing golden tests use up to Scenario 46 (`golden_emergent.rs`).
9. **Test file location** — spec says extend `golden_t22_bandit_camp_destruction.rs`. Confirmed this file exists and has reusable infrastructure.
10. **`golden_inventory.py`** — confirmed at `scripts/golden_inventory.py`.

## Architecture Check

1. All three suites extend the existing T22 test file, reusing its harness helpers — no new test infrastructure needed.
2. No backwards-compatibility shims. Suite-specific topology builders and setup functions are additive.
3. Suite 3's engine change (Option B) is a 5-line addition to an existing guard clause in `emit_raid_target_goals()`, not a new system or architectural layer.

## Verification Layers

1. Spec accuracy → this reassessment document (manual review)
2. Suite 1 setup viability → existing focused tests for `RaidTarget` candidate generation in `candidate_generation.rs` tests (lines 5182+)
3. Suite 2 belief propagation → needs verification of `ShareBelief` candidate emission for combat observations
4. Suite 3 wound dampening mechanism → `emit_raid_target_goals()` guard clause + ranking arithmetic

## What to Change

### 1. Correct S41 spec

- Suite 1: Note that bandits need non-zero `enterprise_weight` for meaningful RaidTarget motive scores.
- Suite 3: Correct the dampening mechanism description. Replace "ranking places `ReduceDanger` above `RaidTarget`" with the actual mechanism (wound-aware suppression at candidate-generation layer via `flee_wound_threshold`).
- Suite 3, point 5: Remove the claim about `BlockedIntentMemory` absence being the proof — the proof surface is candidate-generation diagnostics showing RaidTarget omission due to wound load.

### 2. Flag engine dependency for Suite 3

Document that S41BANOFFEME-004 (Suite 3 implementation) is blocked on a small engine change to `emit_raid_target_goals()` adding wound-aware suppression.

## Files to Touch

- `specs/S41-bandit-offensive-emergence-goldens.md` (modify — correct divergences)

## Out of Scope

- Implementing any golden test code (covered by S41BANOFFEME-002 through S41BANOFFEME-004)
- Engine changes to `candidate_generation.rs` (covered by S41BANOFFEME-004 if Option B is approved)
- Any changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems`
- Modifying existing T22 tests

## Acceptance Criteria

### Tests That Must Pass

1. No test changes in this ticket — spec correction only.
2. Existing suite: `cargo test -p worldwake-ai` — no regressions from spec edits.

### Invariants

1. All spec corrections are traceable to specific code symbols cited in the reassessment above.
2. No claimed behavior in the corrected spec lacks a corresponding code path or flagged engine dependency.

## Test Plan

### New/Modified Tests

None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai` — confirm no regressions
2. `cargo clippy --workspace` — confirm no warnings
