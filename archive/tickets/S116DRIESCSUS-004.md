# S116DRIESCSUS-004: Motive-scoring integration in ranking.rs with unit coverage

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `BeliefView` / `PerAgentBeliefView` accessors, `RankingContext`, `drive_score`, `relevant_self_consume_factors`, `RankedDriveMotiveInput` field in `goal_model.rs`, decision-trace fixture fallout
**Deps**: archive/tickets/S116DRIESCSUS-002.md, archive/tickets/S116DRIESCSUS-003.md

## Problem

Spec S116 requires motive-score multipliers applied at read time in `ranking.rs`, sourced from the authoritative `DeprivationExposure` counter and `DriveEscalationProfile`. With ticket 003 now landed locally, dirtiness and the other four homeostatic counters are live authoritative inputs rather than a future fallback path. Decision traces must surface the per-need multiplier alongside `pressure` and `weight`. `score_product(weight, pressure)` must remain pure so decision traces keep weight × pressure legible; the multiplier is applied one layer up.

## Assumption Reassessment (2026-04-17)

1. Ranking entry points (grepped 2026-04-17): `motive_score` at `crates/worldwake-ai/src/ranking.rs:585`; `drive_score` at ranking.rs:1167; `score_product` at ranking.rs:1288; `relevant_self_consume_factors` at ranking.rs:1234.
2. `RankingContext<'a>` at ranking.rs:342 is populated in `RankingContext::new` at ranking.rs:362 from `view: &'a dyn GoalBeliefView`. Existing fields read from the view include `needs`, `thresholds`, `exploration_profile`, `diversification_profile`, `last_proactive_exploration_tick`, `satiation_profile`, `obligation_tracker`. Two new fields follow the same pattern.
3. `BeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:194-195` declares `homeostatic_needs` and `drive_thresholds`. Extended trait `PerAgentBeliefView` at `per_agent_belief_view.rs:459-465` implements both. Test doubles live at `belief_view.rs:1871-1876` and `goal_model.rs:3533-3537` (AI-side mock). Two new accessors `deprivation_exposure` and `drive_escalation_profile` mirror this shape.
4. Decision-trace carrier: `RankedDriveMotiveInput { drive, pressure, weight, score, relief_per_unit, recovery_relevant }` lives in `crates/worldwake-ai/src/goal_model.rs` and is constructed in `relevant_self_consume_factors` (ranking.rs:1188-1198) and via `drive_provenance_from_inputs`. Adding `escalation_multiplier: MultiplierPermille` there is the extension point; snapshot/test fallout may also touch `decision_trace.rs`.
5. `RankedDriveKind` → `HomeostaticNeedId` mapping: `RankedDriveKind::Hunger → HomeostaticNeedId::Hunger` etc. Both enums cover the same 5 domain concepts; a `From` impl or match helper is the translation site.
6. Intended verification layer (precision rule 2): AI / belief-view / planning-layer logic. No authoritative-system change.
7. Pre-S116 behavior must be exactly reproduced when exposure = None, profile = Default, and counter = 0 (multiplier = 1000 permille = identity). This is the principal regression guard — verified by the D9 neutrality unit test.
8. `score_product(weight, pressure) -> u32` stays pure per precision rule 6 and spec D4 — the multiplier is applied at the callers so decision trace shows raw `weight × pressure` alongside the multiplier as separate provenance inputs.
9. Shared abstraction boundary under audit: the `RankedDriveMotiveInput` struct in `goal_model.rs` — it is the per-drive motive-scoring trace carrier, read by downstream decision-trace consumers including `decision_trace.rs` snapshot emitters.
10. Live dependency check: this implementation still only needs the profile API from ticket 002 to compile the ranking-side read path, but now that archived ticket 003 is landed locally it also provides the truthful dirtiness exposure behavior used by focused escalation tests. Closeout must not describe dirtiness escalation as a hypothetical pre-003 fallback path.

## Architecture Check

1. `score_product` stays `u32 = weight * pressure` — the multiplier is applied at each caller site (`drive_score`, `relevant_self_consume_factors`) after the base score is computed. Decision trace therefore carries `weight`, `pressure`, `score = weight * pressure`, `escalation_multiplier`, and the effective motive score is `score * escalation_multiplier / 1000` — all four values inspectable separately.
2. Accessor-driven reads preserve FND-14 agent-owned locality: ranking reads only the planning agent's own `DeprivationExposure` and `DriveEscalationProfile` components.
3. `RankingContext::new` populates new fields from the same `view` already in scope — no new context construction path or argument.
4. Backwards compatibility via multiplier neutrality (1000 permille) when counter = 0 — not a shim, because `escalation_multiplier` is a single canonical computation that happens to return identity in the counter-never-escalated case. No dual-path code.

## Verification Layers

1. Multiplier neutrality when `ticks <= start_after_ticks` → focused unit test: `drive_score` returns pre-S116 value (regression guard).
2. Linear growth past `start_after_ticks` → focused unit test: multiplier = 2000 permille doubles the `drive_score` output.
3. Saturation at `max_multiplier` → focused unit test: multiplier-scale value plateaus at cap regardless of further counter growth.
4. Self-consume factor path covered → focused unit test: `relevant_self_consume_factors` emits `DriveFactor` with matching `escalation_multiplier` for hunger.
5. Decision trace surfaces the new field → focused test that constructs a `RankedDriveMotiveInput` under active escalation and asserts `escalation_multiplier > MultiplierPermille::IDENTITY`.
6. Ranking is AI-layer logic (precision rule 2) — authoritative-layer verification is the job of tickets 003 (counter maintenance) and 006 (goldens), not this ticket.

## What to Change

### 1. Trait accessors

In `crates/worldwake-sim/src/belief_view.rs`, extend the `BeliefView` trait (around line 194):

```rust
fn deprivation_exposure(&self, agent: EntityId) -> Option<DeprivationExposure>;
fn drive_escalation_profile(&self, agent: EntityId) -> Option<DriveEscalationProfile>;
```

Provide default impls only if the existing `homeostatic_needs`/`drive_thresholds` pair has defaults; otherwise require implementations (consistent with how those two are declared today).

Also extend the sibling `PerAgentBeliefView` trait declarations around line 460-467.

### 2. Accessor implementations

Implement `deprivation_exposure` and `drive_escalation_profile` on:

- The primary `World`-backed `BeliefView` implementation in `belief_view.rs` (search for `impl BeliefView for ...` around line 1259).
- `PerAgentBeliefView` implementation in `per_agent_belief_view.rs:459+`.
- Test double(s) at `belief_view.rs:1871+`.
- AI-side mock in `crates/worldwake-ai/src/goal_model.rs:3533-3537` (extend the existing `impl` block).

Each impl reads the agent's `DeprivationExposure` / `DriveEscalationProfile` component via `world.get_component_*` or the appropriate per-agent cache path.

### 3. RankingContext extension

At `ranking.rs:342`, add two fields:

```rust
struct RankingContext<'a> {
    // ... existing fields ...
    exposure: Option<DeprivationExposure>,
    escalation_profile: Option<DriveEscalationProfile>,
}
```

In `RankingContext::new` at ranking.rs:379-397, populate them:

```rust
exposure: view.deprivation_exposure(agent),
escalation_profile: view.drive_escalation_profile(agent),
```

### 4. Multiplier application in `drive_score`

Modify `drive_score` (ranking.rs:1167) to also receive the `HomeostaticNeedId` discriminant (or compute it by inspecting the pressure/weight closure call context — whichever keeps the call-site churn minimal). After computing `base = score_product(weight, pressure)`:

```rust
let ticks = context.exposure.map(|e| e.ticks_at_critical(need)).unwrap_or(0);
let params = context.escalation_profile
    .as_ref()
    .map(|p| p.params_for(need))
    .unwrap_or_default();
let multiplier = escalation_multiplier(ticks, params);
base.saturating_mul(u32::from(multiplier.value())) / 1000
```

Call sites at ranking.rs:607-621 (Sleep → Fatigue, Relieve → Bladder, Wash → Dirtiness) pass the matching `HomeostaticNeedId`.

### 5. Multiplier application in `relevant_self_consume_factors`

Modify `relevant_self_consume_factors` at ranking.rs:1234-1271 to compute the multiplier per `DriveFactor` using the factor's `drive → HomeostaticNeedId` mapping, and attach it to the factor (add an `escalation_multiplier: MultiplierPermille` field on the private `DriveFactor` struct at ranking.rs:402-409).

Downstream at ranking.rs:1194 where `RankedDriveMotiveInput` is constructed from a `DriveFactor`, copy `escalation_multiplier` through. Apply the multiplier to the final `score` returned to the caller the same way `drive_score` does.

### 6. Decision-trace field

Add `pub escalation_multiplier: MultiplierPermille` to `RankedDriveMotiveInput` in `crates/worldwake-ai/src/goal_model.rs`. Populate in all construction sites — the direct-drive provenance path, the `relevant_self_consume_factors` path, and any snapshot/test literals. For non-drive goal kinds that do not read a homeostatic counter, the multiplier remains `MultiplierPermille::IDENTITY` (neutral).

### 7. `RankedDriveKind → HomeostaticNeedId` helper

Add a small match helper (in `ranking.rs` or alongside `RankedDriveKind`):

```rust
fn homeostatic_need_id_for_drive(kind: RankedDriveKind) -> Option<HomeostaticNeedId> {
    match kind {
        RankedDriveKind::Hunger => Some(HomeostaticNeedId::Hunger),
        RankedDriveKind::Thirst => Some(HomeostaticNeedId::Thirst),
        RankedDriveKind::Fatigue => Some(HomeostaticNeedId::Fatigue),
        RankedDriveKind::Bladder => Some(HomeostaticNeedId::Bladder),
        RankedDriveKind::Dirtiness => Some(HomeostaticNeedId::Dirtiness),
        RankedDriveKind::Pain | RankedDriveKind::Danger => None, // out of scope per spec non-goals
    }
}
```

Exact variant list should be verified against the live `RankedDriveKind` enum; adjust per that enum's actual members.

### 8. Unit tests (D9)

Add four new unit tests in `ranking.rs`'s `#[cfg(test)] mod tests` block:

- `drive_score_preserves_pre_s116_motive_when_counter_below_start_after` — constructs a `RankingContext` with exposure = default (counter = 0) and asserts `drive_score` output matches a hand-computed `score_product(weight, pressure)`.
- `drive_score_doubles_when_multiplier_is_2000_permille` — constructs a counter value past `start_after_ticks` sized to produce multiplier ≈ 2000 permille, asserts output is approximately `2 * score_product(weight, pressure)`.
- `drive_score_saturates_at_max_multiplier` — runs counter well past the cap-hitting value, asserts output ≈ `score_product(weight, pressure) * max_multiplier.value() / 1000`.
- `relevant_self_consume_factors_attaches_escalation_multiplier_to_hunger_factor` — constructs a hunger-dominant context with active escalation, asserts the emitted `DriveFactor` (or `RankedDriveMotiveInput`) carries `escalation_multiplier > MultiplierPermille::IDENTITY`.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait declarations + one impl + test double)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — trait + impl)
- `crates/worldwake-ai/src/goal_model.rs` (modify — test mock around line 3533)
- `crates/worldwake-ai/src/ranking.rs` (modify — `RankingContext`, `DriveFactor`, `drive_score`, `relevant_self_consume_factors`, `homeostatic_need_id_for_drive` helper, 4 new unit tests)
- `crates/worldwake-ai/src/decision_trace.rs` (modify only if fixture / snapshot literals need the extra field)

## Out of Scope

- `needs_system` counter maintenance — ticket 003.
- Scenario RON integration — ticket 005.
- Goldens — ticket 006.
- Escalation for non-homeostatic drives (pain, danger) — spec non-goal.

## Acceptance Criteria

### Tests That Must Pass

1. All 4 new unit tests listed above pass.
2. Pre-existing ranking tests in `crates/worldwake-ai/src/ranking.rs` all pass. Any decision-trace snapshot tests that print `RankedDriveMotiveInput` may need one additional field in the expected output.
3. Existing suite: `cargo test -p worldwake-ai`, `cargo test -p worldwake-sim`.

### Invariants

1. `score_product(weight, pressure)` remains `u32 = weight * pressure` — unchanged signature, unchanged arithmetic.
2. When `context.exposure` is `None` or `ticks_at_critical(need) <= params.start_after_ticks`, motive scores are bit-identical to pre-S116.
3. Decision-trace `RankedDriveMotiveInput` carries `escalation_multiplier` for every drive-driven goal motive input. Non-drive goals carry `MultiplierPermille::IDENTITY` (neutral).
4. No authoritative-world-state mutation in this ticket.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — 4 new unit tests (D9) covering neutrality, 2× growth, saturation, and self-consume factor attachment.
2. `crates/worldwake-ai/src/ranking.rs` / `decision_trace.rs` — any snapshot tests printing `RankedDriveMotiveInput` extended with the new field.

### Commands

1. `cargo test -p worldwake-ai ranking`
2. `cargo test -p worldwake-ai`
3. `cargo test -p worldwake-sim`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-17.

- Extended the AI read surface so `GoalBeliefView` / `ProfileBeliefView` now expose `deprivation_exposure(...)` and `drive_escalation_profile(...)`, with the live per-agent implementation in `crates/worldwake-sim/src/per_agent_belief_view.rs` and matching stub/mock support in `belief_view.rs`, `goal_model.rs`, and ranking tests.
- Extended `crates/worldwake-ai/src/ranking.rs` so `RankingContext` reads exposure/profile once, `drive_score(...)` applies the per-need escalation multiplier for direct drive goals, and self-consume motive scoring applies the same multiplier through `DriveFactor` without changing `score_product(...)`.
- Added `escalation_multiplier: MultiplierPermille` to `RankedDriveMotiveInput` in `crates/worldwake-ai/src/goal_model.rs` and updated construction / fixture sites so decision-trace provenance now surfaces the multiplier separately from raw `weight * pressure`.
- Added the four focused D9 unit tests for neutrality, 2x growth, saturation, and self-consume-factor attachment.

## Deviations

- Reassessment confirmed the live read boundary is `GoalBeliefView` forwarding into `ProfileBeliefView`, not a standalone world-backed `BeliefView` impl to edit directly. The implementation changed the shared profile/read surface once and relied on the existing forwarding layer.
- `RankedDriveMotiveInput` already lives in `crates/worldwake-ai/src/goal_model.rs`; `crates/worldwake-ai/src/decision_trace.rs` only needed fixture fallout updates, not ownership changes.
- With archived ticket `S116DRIESCSUS-003` now landed locally, the focused tests exercised truthful live escalation input rather than a hypothetical pre-003 identity-only dirtiness path. Closeout therefore records `003` as part of the live dependency stack for this run.

## Verification Result

- Passed `cargo test -p worldwake-ai ranking`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
