# S130SURRECFRO-003: ExploreLocation hypothesis field + need-to-hypothesis mapping

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `GoalKind::ExploreLocation` payload widening across `worldwake-core`, `worldwake-ai`, and any consumers (test fixtures included)
**Deps**: `archive/tickets/S130SURRECFRO-002.md`, spec `specs/S130-survey-records-frontier-disconfirmation.md` D2

## Problem

S130's surveying behavior requires the agent's exploration intent to carry a hypothesis — what they expected to find. The existing `GoalKind::ExploreLocation { target_place, motivating_need }` shape carries the *why* but not the *what*. This ticket widens the variant to include `hypothesis: HypothesisKind`, defines the canonical need-to-hypothesis mapping at candidate emission, and updates every destructure/construction site across the workspace per FND-28 (no shim).

## Assumption Reassessment (2026-05-02)

1. `GoalKind::ExploreLocation { target_place: EntityId, motivating_need: ExplorationMotivation }` is defined at `crates/worldwake-core/src/goal.rs:149-152`; spec confirms 65 destructure/construction sites workspace-wide. All 65 use explicit field listing (no `..Default::default()` spread) and `HypothesisKind` has no `Default` impl, so every site needs explicit `hypothesis:` population.
2. `ExplorationMotivation` at `crates/worldwake-core/src/goal.rs:18-22` has variants `NeedDriven(HomeostaticNeedId)` and `Proactive`; `HomeostaticNeedId` at `crates/worldwake-core/src/needs.rs:19-25` has variants `Hunger, Thirst, Fatigue, Bladder, Dirtiness`. The need→hypothesis mapping in spec D2 covers all five.
3. `GoalKindPlannerExt` impl arms for `ExploreLocation` use `..` or `target_place, ..` patterns (verified: `goal_model.rs:597, 646, 1448, 1624, 1785`), so they tolerate the new field without per-arm updates. The `is_satisfied` arm at `goal_model.rs:1448` and `matches_binding` at `goal_model.rs:1785` continue to bind on `target_place` only — adding `hypothesis` does not break Travel binding.
4. Need-driven emission site is `crates/worldwake-ai/src/candidate_generation.rs:2810`; proactive emission site is `crates/worldwake-ai/src/candidate_generation.rs:3030`. Both currently populate `target_place` and `motivating_need` only.
5. `GoalKey` derivation at `goal.rs:200-277` stores the full `GoalKind` payload and derives equality/ordering over it — adding `hypothesis` produces distinct `GoalKey`s for `ExploreLocation` goals with the same place but different hypothesis (e.g., Hunger vs Thirst). Commitment, blocker memory, and discrepancy memory naturally separate these as distinct goals; no collision handling needed.
6. Ranking-arm destructure at `crates/worldwake-ai/src/ranking.rs:1129` (`} => exploration_motive(context, motivating_need)`) needs `..` or explicit field handling to compile — ticket 006 will add hypothesis-aware damping; this ticket adds `..` to keep the workspace building until 006 lands.
7. Exhaustive match sites use `..` extensively, so the variant payload widening is mostly a mechanical pass: existing match arms with `..` need no change; arms that destructure all named fields need `hypothesis` added; construction sites need `hypothesis: …` populated.
8. Existing tests touching `GoalKind::ExploreLocation` construction: ranking tests (`explore_location_ranking_is_not_biased_by_place_dirtiness:8677`, `explore_location_need_driven_priority_tracks_underlying_need_band:9139`, `explore_location_motive_uses_need_utility_scaled_by_curiosity:9180`, `explore_location_proactive_motive_uses_curiosity_buildup_and_need_slack:9272`), candidate-generation tests, goal_model tests at lines 8664/8761/8797/8813. Each test fixture's construction site receives an explicit `hypothesis:` value matching the test's intent (need-driven → `need_hypothesis(need)`; proactive → `HypothesisKind::Proactive`).

## Architecture Check

1. Adding `hypothesis` to the variant rather than introducing a sibling goal kind preserves `ExploreLocation`'s identity — the agent's intent is still "explore this place under this motivation," now with the additional question "what do I expect to find?". Distinct GoalKeys per (place, hypothesis) combination flow naturally from the existing `GoalKind` payload equality/ordering without bespoke key derivation.
2. The need-to-hypothesis mapping `need_hypothesis(need: HomeostaticNeedId) -> HypothesisKind` is a `const fn` in `candidate_generation.rs`, the file that already owns need-driven emission. Per-agent dietary preference for `MayContainCommodity` is non-goal (uniform mapping in this spec); a follow-on can introduce per-agent food preference state when consumption diversity matters.
3. No backward-compatibility shim — FND-28 mandate. The existing 2-field shape is retired; the new 3-field shape is the only authoritative form.
4. Travel binding continues to bind on `target_place` only (`matches_binding` at `goal_model.rs:1785`) — Travel ops are hypothesis-agnostic, which is correct: the agent's path doesn't change based on what they hope to find.

## Verification Layers

1. Need→hypothesis mapping correctness → focused unit test on `need_hypothesis` covering all five `HomeostaticNeedId` variants.
2. Goal-key distinction across hypotheses → focused unit test asserting `GoalKey::from(ExploreLocation { same_place, same_need, hypothesis: A })` ≠ `GoalKey::from(ExploreLocation { same_place, same_need, hypothesis: B })`.
3. Travel binding ignores hypothesis → focused unit test asserting `matches_binding(ExploreLocation { target_place, hyp: A }, &[target_place])` and the same for hypothesis B both return true (binding remains place-only).
4. `is_satisfied` continues to fire on arrival regardless of hypothesis → focused unit test asserting agent at target_place satisfies `ExploreLocation` with any hypothesis.
5. Workspace builds — Cargo build is the integration proof that all 65 sites are updated; no compile-time stragglers.

## What to Change

### 1. Variant payload widening

In `crates/worldwake-core/src/goal.rs`, change `GoalKind::ExploreLocation` to:

```rust
ExploreLocation {
    target_place: EntityId,
    motivating_need: ExplorationMotivation,
    hypothesis: HypothesisKind,
},
```

### 2. Need-to-hypothesis mapping

Add to `crates/worldwake-ai/src/candidate_generation.rs` (alongside existing `emit_*_goal` for needs):

```rust
const fn need_hypothesis(need: HomeostaticNeedId) -> HypothesisKind {
    match need {
        HomeostaticNeedId::Hunger => HypothesisKind::MayContainCommodity {
            commodity: CommodityKind::Apple,
        },
        HomeostaticNeedId::Thirst => HypothesisKind::MayContainCommodity {
            commodity: CommodityKind::Water,
        },
        HomeostaticNeedId::Bladder => HypothesisKind::MayContainLatrine,
        HomeostaticNeedId::Dirtiness => HypothesisKind::MayContainWashBasin,
        HomeostaticNeedId::Fatigue => HypothesisKind::MayContainSleepSite,
    }
}
```

### 3. Update need-driven and proactive emission sites

At `candidate_generation.rs:2810` (need-driven) populate `hypothesis: need_hypothesis(need_id)`. At `candidate_generation.rs:3030` (proactive) populate `hypothesis: HypothesisKind::Proactive`.

### 4. Update all destructure/construction sites

Sweep workspace for `GoalKind::ExploreLocation` (65 sites per `grep -rn "GoalKind::ExploreLocation" crates/`). For each:

- Destructure with `..` already present → no change.
- Destructure with all fields named → add `, hypothesis` (or `, hypothesis: _` when not used).
- Construction → populate `hypothesis: …` with the intent of the surrounding code (need-driven test fixtures use `need_hypothesis(need)`; proactive test fixtures use `HypothesisKind::Proactive`; arbitrary fixtures pick the shape that matches the test scenario).

Specifically, at `ranking.rs:1129`, change the match-arm destructure to `} => exploration_motive(context, motivating_need)` → `} => exploration_motive(context, motivating_need)` with `..` capturing `hypothesis` until ticket 006 wires the damping.

### 5. Existing test-fixture updates

Update the four ranking tests (`explore_location_ranking_is_not_biased_by_place_dirtiness:8677`, `explore_location_need_driven_priority_tracks_underlying_need_band:9139`, `explore_location_motive_uses_need_utility_scaled_by_curiosity:9180`, `explore_location_proactive_motive_uses_curiosity_buildup_and_need_slack:9272`) plus the goal_model construction sites at `goal_model.rs:8664/8761/8797/8813` and any other test fixtures that build `ExploreLocation` literals.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify — variant payload widening)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — `need_hypothesis` + emit sites)
- `crates/worldwake-ai/src/ranking.rs` (modify — match-arm destructure pattern)
- `crates/worldwake-ai/src/goal_model.rs` (modify — match arms and test fixtures)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — any `ExploreLocation` test fixtures)
- `crates/worldwake-ai/src/agent_tick/mod.rs` and submodules (modify — any destructure sites)
- Likely: `crates/worldwake-ai/tests/golden_*.rs` and `crates/worldwake-systems/src/**/*.rs` (modify — discovery via `grep -rn "GoalKind::ExploreLocation" crates/`; update each match by mechanical rule above)

## Out of Scope

- Hypothesis-aware ranking damping (ticket 006 — uses the new `hypothesis` field to consult survey memory)
- Perception-time hypothesis evaluation against perceived entities (ticket 007)
- Per-agent dietary preference for `MayContainCommodity` — explicitly non-goal per spec D2; uniform mapping for this spec
- New goal-kind sibling (e.g., `SurveyLocation`) — explicitly rejected by spec; hypothesis is part of the existing goal, not a new kind

## Acceptance Criteria

### Tests That Must Pass

1. New: `need_hypothesis_maps_each_homeostatic_need_to_expected_hypothesis` — covers all 5 variants.
2. New: `goal_key_distinguishes_explore_location_by_hypothesis` — same place + same need + different hypothesis → distinct `GoalKey`.
3. New: `matches_binding_for_explore_location_ignores_hypothesis` — Travel binding remains place-only.
4. Existing: `explore_location_ranking_is_not_biased_by_place_dirtiness` — adjusted fixtures; assertion semantics unchanged.
5. Existing: `explore_location_need_driven_priority_tracks_underlying_need_band` — adjusted fixtures; assertion semantics unchanged.
6. Existing: `explore_location_motive_uses_need_utility_scaled_by_curiosity` — adjusted fixtures.
7. Existing: `explore_location_proactive_motive_uses_curiosity_buildup_and_need_slack` — adjusted fixtures.
8. Existing suite: `cargo test --workspace`.
9. `cargo clippy --workspace --all-targets -- -D warnings` (compile-time gate that all 65 sites are populated).

### Invariants

1. No `..` shim hiding an unwritten field — every construction site explicitly populates `hypothesis` (FND-28 + ticket fidelity rule).
2. `GoalKey` for two `ExploreLocation` goals with same `(target_place, motivating_need)` but different `hypothesis` are distinct (commitment, blocker memory, discrepancy memory naturally separate them).
3. Travel binding (`matches_binding`) continues to bind on `target_place` only — hypothesis does not affect Travel ops.
4. `is_satisfied(ExploreLocation { target_place, .. })` semantics unchanged — agent at target_place satisfies the goal regardless of hypothesis.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (`#[cfg(test)]` block) — 1 new unit test: `need_hypothesis_maps_each_homeostatic_need_to_expected_hypothesis`.
2. `crates/worldwake-core/src/goal.rs` (`#[cfg(test)]` block) — 1 new unit test: `goal_key_distinguishes_explore_location_by_hypothesis`.
3. `crates/worldwake-ai/src/goal_model.rs` (`#[cfg(test)]` block) — 1 new unit test: `matches_binding_for_explore_location_ignores_hypothesis`.
4. Existing tests adjusted: 4 explore_location ranking tests at ranking.rs:8677/9139/9180/9272 (fixture updates only — no assertion changes).
5. All other test fixtures across the workspace that build `ExploreLocation` literals — mechanical fixture updates (no assertion changes).

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::need_hypothesis`
2. `cargo test -p worldwake-core goal::tests::goal_key_distinguishes`
3. `cargo test -p worldwake-ai goal_model::tests::matches_binding`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `./scripts/verify.sh`
