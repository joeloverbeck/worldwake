# S130SURRECFRO-006: AI ranking damping with survey memory

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `survey_damping_factor` helper, ranking-arm wrapping for `ExploreLocation`, decision-trace damping entries
**Deps**: `archive/tickets/S130SURRECFRO-001.md`, `archive/tickets/S130SURRECFRO-003.md`, 004, 005, spec `specs/S130-survey-records-frontier-disconfirmation.md` D7

## Problem

When an agent has a fresh negative survey for `(target_place, hypothesis)` — i.e., they recently visited the place looking for the same thing and found nothing — the ranking layer should suppress re-exploration of that place under that hypothesis. The ranking score for `ExploreLocation` is currently `exploration_motive(context, motivating_need)` (a `u32` returned from `ranking.rs:1146`); this ticket wraps that with a `Permille`-multiplied damping factor derived from the freshest matching survey record. Damping fades with record age via the `negative_survey_damping_window` (per `ExplorationProfile`).

## Assumption Reassessment (2026-05-02)

1. The `ExploreLocation` ranking arm at `crates/worldwake-ai/src/ranking.rs:1127-1129` calls `exploration_motive(context, motivating_need)` and returns its `u32` result; `exploration_motive` is defined at `ranking.rs:1146`. After ticket 003, the destructure at line 1129 includes `..` to capture the new `hypothesis` field — this ticket destructures `hypothesis` explicitly to feed it into damping.
2. `Permille`-multiply-into-`u32` arithmetic pattern is established in `ranking.rs` at lines 398, 431, 1162, 1510, 1710, 1726, 1752 (e.g., `score.saturating_mul(u32::from(factor.value())) / 1000`). Survey damping uses the same pattern — no new helpers required.
3. `GoalBeliefView::survey_memory(agent)` (added in ticket 004) returns `Option<&SurveyMemory>`; reading the freshest matching record uses `SurveyMemory::find(place, hypothesis)` (added in `archive/tickets/S130SURRECFRO-002.md`).
4. `ExplorationProfile.negative_survey_damping_window: u32` and `ExplorationProfile.negative_survey_damping_strength: Permille` are added in ticket 001 and accessible through `context.exploration_profile` (existing accessor at `ranking.rs:1152` analog).
5. Existing tests exercising the `exploration_motive` path: `explore_location_ranking_is_not_biased_by_place_dirtiness:8677`, `explore_location_need_driven_priority_tracks_underlying_need_band:9139`, `explore_location_motive_uses_need_utility_scaled_by_curiosity:9180`, `explore_location_proactive_motive_uses_curiosity_buildup_and_need_slack:9272`. After ticket 003's fixture updates, each test fixture has a hypothesis but no survey memory record — so damping factor is 1000/1000 (full score), and existing assertions hold unchanged. This ticket adds new tests for the damping-active path.
6. `CandidateDampingEntry` and `CandidateDampingReason::SurveyMemoryNegative` (added in ticket 005) are the trace-payload shape; the ranking arm pushes `CandidateDampingEntry { goal_key, reason: SurveyMemoryNegative { … } }` onto the active `CandidateTrace.damped` vec when damping is applied (factor < 1000).
7. **AI regression layer** (Assumption Reassessment item 6): intended layer is candidate ranking — runtime `agent_tick` decision-trace coverage is the proof surface; the local needs-only harness is sufficient because survey damping involves only `ExplorationProfile` reads and `SurveyMemory` reads, not non-needs affordances or political/system actions.

## Architecture Check

1. Damping is a multiplicative `Permille` factor applied to the existing motive score — the original ranking shape (priority class + motive score) is preserved; damping only modulates the motive score within its existing class. This avoids re-architecting the ranking ladder.
2. The damping factor is *derived* per-tick from `SurveyMemory` and the current tick (FND-27 — derived view, never authoritative state). It is recomputed on every ranking pass, never stored.
3. `survey_damping_factor` is a pure function — `(survey: Option<&SurveyRecord>, current_tick: Tick, profile: &ExplorationProfile) -> Permille`. Easy to unit-test in isolation; integrated via call site at `ranking.rs:1127`.
4. Survey damping respects FND-7 (locality) and FND-26 (state-mediated cross-system): perception writes `SurveyMemory`; ranking reads `SurveyMemory` via `GoalBeliefView::survey_memory()` — no imperative cross-system call.
5. No backward-compat shim — the existing `exploration_motive` call is replaced by a damping-wrapped call; the inner function is unchanged.

## Verification Layers

1. `survey_damping_factor` returns `Permille::new(1000)` when the survey record is `None` → focused unit test.
2. `survey_damping_factor` returns `Permille::new(1000)` when the record is positive (`found = true`) → focused unit test.
3. `survey_damping_factor` returns `Permille::new(1000)` when the record age exceeds `negative_survey_damping_window` → focused unit test.
4. `survey_damping_factor` returns the attenuated value `1000 - (confidence * strength / 1000)` when fresh-negative → focused unit tests at multiple confidence/strength values.
5. Damping is recorded in the decision trace when applied → focused unit test asserting the active `CandidateTrace.damped` vec contains the expected entry.
6. Single-cross-system-tier ticket (AI ranking + belief-view read + decision-trace write) — golden coverage of the end-to-end "damping suppresses re-exploration" scenario lives in ticket 009.

## What to Change

### 1. `survey_damping_factor` helper

Add to `crates/worldwake-ai/src/ranking.rs`:

```rust
fn survey_damping_factor(
    survey: Option<&SurveyRecord>,
    current_tick: Tick,
    profile: &ExplorationProfile,
) -> Permille {
    let Some(record) = survey else {
        return Permille::new_unchecked(1000);
    };
    if record.found {
        return Permille::new_unchecked(1000);
    }
    let age = current_tick.0.saturating_sub(record.recorded_tick.0);
    if age >= u64::from(profile.negative_survey_damping_window) {
        return Permille::new_unchecked(1000);
    }
    let confidence = u32::from(record.confidence.value());
    let strength = u32::from(profile.negative_survey_damping_strength.value());
    let attenuation = confidence.saturating_mul(strength) / 1000;
    Permille::new_unchecked(1000u32.saturating_sub(attenuation).min(1000) as _)
}
```

### 2. Wrap the `ExploreLocation` ranking arm

At `crates/worldwake-ai/src/ranking.rs:1127-1129`, change the match arm to destructure `hypothesis` and wrap the result:

```rust
GoalKind::ExploreLocation {
    target_place,
    motivating_need,
    hypothesis,
} => {
    let raw = exploration_motive(context, motivating_need);
    let survey = context
        .belief_view
        .survey_memory(agent)
        .and_then(|m| m.find(target_place, hypothesis));
    let factor = survey_damping_factor(survey, context.tick, &context.exploration_profile);
    if factor.value() < 1000 {
        if let Some(record) = survey {
            context.trace.candidates.damped.push(CandidateDampingEntry {
                goal_key: candidate_goal_key,
                reason: CandidateDampingReason::SurveyMemoryNegative {
                    place: target_place,
                    hypothesis,
                    recorded_tick: record.recorded_tick,
                    confidence: record.confidence,
                },
            });
        }
    }
    raw.saturating_mul(u32::from(factor.value())) / 1000
}
```

(Variable names — `agent`, `candidate_goal_key`, `context.belief_view`, `context.trace.candidates`, `context.exploration_profile`, `context.tick` — bind to the ranking-context surface available at the call site; reassessment confirmed these are accessible at line 1127's scope. The exact names may differ slightly from the surrounding ranking arms — match the existing local style.)

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — `survey_damping_factor`, `ExploreLocation` arm wrap, new tests)

## Out of Scope

- Perception-time hypothesis evaluation that creates the `SurveyRecord` (ticket 007)
- Per-tick decay of `SurveyMemory` (ticket 008)
- Golden coverage of end-to-end "damping suppresses re-exploration" (ticket 009)
- Damping for goal kinds other than `ExploreLocation` — out of scope; the spec's damping is scoped to `ExploreLocation`

## Acceptance Criteria

### Tests That Must Pass

1. New: `survey_damping_factor_returns_unity_when_record_is_none`.
2. New: `survey_damping_factor_returns_unity_when_record_is_positive`.
3. New: `survey_damping_factor_returns_unity_when_record_is_stale_past_window`.
4. New: `survey_damping_factor_attenuates_with_fresh_negative_record` — multiple confidence/strength values per spec formula.
5. New: `ranking_pushes_damping_entry_when_explore_location_is_damped` — focused unit test on the ranking arm.
6. Existing: `explore_location_ranking_is_not_biased_by_place_dirtiness` — passes unchanged (no survey record in fixture → factor = 1000).
7. Existing: `explore_location_need_driven_priority_tracks_underlying_need_band` — passes unchanged.
8. Existing: `explore_location_motive_uses_need_utility_scaled_by_curiosity` — passes unchanged.
9. Existing: `explore_location_proactive_motive_uses_curiosity_buildup_and_need_slack` — passes unchanged.
10. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. `survey_damping_factor` is a pure function — no side effects, deterministic given inputs. Decoded as: same `(survey, tick, profile)` inputs always produce the same `Permille` output.
2. Damping is recorded in `CandidateTrace.damped` only when the factor is strictly less than 1000 (no entry for "damping considered but not applied").
3. The original `exploration_motive` function remains unchanged — damping wraps it without modifying its body.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (`#[cfg(test)]` block) — 5 new focused unit tests covering `survey_damping_factor` and the ranking-arm trace-push behavior (per Acceptance Criteria 1–5).

### Commands

1. `cargo test -p worldwake-ai ranking::tests::survey_damping`
2. `cargo test -p worldwake-ai ranking::tests::ranking_pushes_damping`
3. `cargo test -p worldwake-ai explore_location`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`
