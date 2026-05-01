# S129CIREM-004: Patrol frontier exhaustion strands guard until self-care

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — patrol exhaustion retry policy in AI planning
**Deps**: archive/specs/S129-place-dirtiness-facility-wear.md, prior patrol spec (search archive for `patrol_route` originator)

## Problem

Original draft premise: `golden_survival_patrol::survival_patrol_proves_patrol_and_remote_pursuit_execution` and `golden_survival_patrol::survival_patrol_replay_is_deterministic` both failed at `crates/worldwake-ai/tests/golden_survival_patrol.rs:257` with `guard should commit patrol at Market Road`. Live reassessment on 2026-05-01 disproved that exact failure: both waypoint assertions now pass. The remaining live failure was later in the proof test:

```text
survival patrol should have no idle windows >= 40 ticks with needs > 300 permille:
[StuckIdleWindow { agent_name: "Guard Mira", start_tick: 1044, end_tick: 1084, max_need_at_start: 309 }]
```

Diagnostic trace showed `Guard Mira` completed the Watch Post patrol, traveled to Market Road, committed patrol there, returned to Watch Post, and then carried a `GoalKind::Patrol { place: Watch Post }` opportunity in `AgentDecisionRuntime.exhaustion_cache` as `ExhaustionRetryState::FrontierExhausted`. Because patrol's invalidation strategy was position-only, the agent stayed at Watch Post with the patrol opportunity permanently suppressed until a later self-care candidate crossed its emission threshold. The live bug was therefore not patrol motive arithmetic, route advancement, or Market Road self-care dominance; it was permanent frontier suppression for a recurring route duty.

## Assumption Reassessment (2026-05-01)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Pre-S129 baseline**: parent commit `fa0dd620` passes both patrol golden tests. Verified during S129 CI investigation. The waypoint failure is a regression introduced by the S129 changeset.
2. **Patrol motive does not pass through `apply_hygiene_motive_modifiers`** (`crates/worldwake-ai/src/ranking.rs:1644–1664` and the `_ => base` fallthrough). Patrol's motive is unchanged by S129 ranking. So patrol motive is the same number as pre-S129; what changed is the *competing* motive landscape on arrival at Market Road.
3. **Live `GoalKind::Patrol` substrate**: `crates/worldwake-ai/src/ranking.rs:1126` (`patrol_motive`), `crates/worldwake-ai/src/ranking.rs:1553–1591` for the arithmetic. Confirm patrol motive arithmetic has not been touched in S129 (it should not have been).
4. **Post-S129 self-care motive landscape at Market Road**: Market Road is `Latrine`-tagged. With S129's `emit_relieve_goal` change (`crates/worldwake-ai/src/candidate_generation.rs:3287–3326`), agents at a Latrine-tagged place emit a `Relieve` candidate anchored on the place with factor `1000` (sub-threshold) or `500` (over-threshold), in addition to the wilderness `None` anchor with factor `750`. Latrine + sub-threshold has factor `1000`, no penalty. Wash candidate at Market Road's basin is also available. So the self-care candidate landscape at Market Road is materially richer than pre-S129; patrol's modest `100` motive may now be losing the on-arrival ranking.
5. **First failure boundary**: there are two candidates. (a) Mira *travels* to Market Road but on arrival commits to wash/relieve/eat instead of patrol, and the patrol candidate stays available but never wins motive. (b) Mira does not even attempt the travel — patrol is dominated by self-care at Watch Post (which is also `[Village, Camp, Latrine]`) and Mira never leaves. Decision-trace dump on Mira will distinguish these.
6. **Live `patrol_action` start path**: search `crates/worldwake-systems/src/` and `crates/worldwake-ai/src/goal_model.rs` for the `patrol` action target/precondition surface. The test's success condition is "Mira commits patrol while `effective_place == market_road`" — so this is an action-trace assertion gated on place + action name. Verify the action's preconditions (e.g. `ActorAtPlace(market_road)` or similar) and confirm there is no precondition that newly fails post-S129.
7. **Patrol route advancement semantics**: confirm whether committing `patrol` at Watch Post advances `patrol_route.current_index` to point at Market Road, or whether the route advances on a different trigger (e.g. `dwell_ticks` elapsed). If patrol is committed at Watch Post but the route never advances, Mira will keep patrolling Watch Post indefinitely. Search `crates/worldwake-systems/src/` for `current_index`, `patrol_route`, `dwell_ticks` advancement.
8. **Coverage gap (precision-rules §3)**: no focused golden currently exercises "patrol motive vs. on-arrival self-care motive at a Latrine-tagged waypoint". The existing patrol golden tests the end-to-end contract but does not isolate the on-arrival ranking decision.
9. **Heuristic Removal Discipline (precision-rules §12)**: a tempting fix is to bump `patrol_motive_weight: 100` to `1000`. This substitutes a weight knob for the actually missing substrate (likely a "patrol takes precedence over routine self-care while a route segment is in progress" semantic, or a route-progression-driven priority class). Do not adopt the weight bump without naming the substrate.
10. **Cumulative arithmetic (precision-rules §15)**: `Guard Mira`'s drive thresholds and metabolism rates are not visible in the truncated read above; verify them in the full scenario. If Mira's needs accumulate fast enough that she is critical at Watch Post by the time the patrol cycle should fire, the test's narrative about "patrol commits at both waypoints" needs different framing. The reassessment must compute when Mira's first need crosses critical and compare against expected patrol cadence.
11. **Mismatch + correction**: the `survival_patrol` test was authored against pre-S129 ranking, where Latrine-tagged places had no `Relieve` Place-anchored candidate (only the wilderness `None` candidate). Adding the Latrine candidate per S129 §312 changed the motive landscape at every Latrine-tagged place. The patrol motive arithmetic was not adjusted to compensate. Either the patrol motive needs a route-aware boost when the agent is mid-route, or the spec's promise that patrol commits at both waypoints during a 1440-tick run needs to be rebuilt on a different substrate.
12. **Final live reassessment**: after rerunning `cargo test --release -p worldwake-ai --test golden_survival_patrol -- --ignored --test-threads=1`, the stale Market Road failure no longer reproduced. The proof failed instead on the survival-health idle-window assertion. A temporary diagnostic confirmed the patrol route advanced, both waypoint patrol commits occurred, and the post-return Watch Post patrol candidate was skipped because `FrontierExhausted` suppresses planning indefinitely until a concrete invalidation condition fires.

## Architecture Check

1. **Investigation precedes fix**: decision-trace dump on Mira distinguishes the two candidate failure modes. Without the trace, any patrol-motive bump risks regressing the survival contract assertions that already pass for Mira.
2. **No silent weight bump**: `patrol_motive_weight: 100` was set on a particular pre-S129 substrate. Bumping it without naming what changed is heuristic-tuning.
3. **Route-progression as concrete state**: per FND-3, the substrate for "Mira should make progress along the route" is the `patrol_route.current_index` and possibly a `last_visited_at` timestamp per waypoint. If patrol motive should rise as time-since-last-visit grows, that is a concrete-state input the ranking can read — *not* a hardcoded "patrol > self-care" rule.

## Verification Layers

1. **First-arrival decision at Market Road** -> decision-trace assertion at the tick when Mira's `effective_place` first equals Market Road (if she ever arrives). What does she choose? Patrol vs. competing self-care goals.
2. **Pre-arrival decision at Watch Post** -> decision-trace assertion at the tick after Mira's first `Watch Post` patrol commit. Does she queue travel to Market Road? If not, what does she do instead?
3. **Patrol route advancement** -> world-state assertion: after Mira commits patrol at Watch Post, does `patrol_route.current_index` advance to point at Market Road? Or does it stay at index 0?
4. **Action-trace for travel** -> confirm whether Mira commits a `travel` action toward Market Road at any point in the 1440-tick window.

## What to Change

Treat `GoalKind::Patrol` frontier exhaustion as a cooldown-backed retry rather than a permanent suppression. This keeps the concrete invalidation baseline and cooldown behavior, but prevents a recurring route duty from being stranded at a waypoint until position changes. Add a focused unit regression for `record_exhausted_goals` proving patrol frontier exhaustion becomes `BudgetRetryPending`.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs`
- No change to `crates/worldwake-ai/src/ranking.rs::patrol_motive`.
- No change to `crates/worldwake-systems/src/patrol_actions.rs`; `commit_patrol` already advances `current_index`.
- No new `PatrolRoute` state or save-format change.
- Verification cleanup also touched `crates/worldwake-core/src/belief.rs` and `crates/worldwake-ai/tests/golden_survival_tell.rs` with Clippy-suggested `map_or` / `map_or_else` rewrites required by the current `./scripts/verify.sh` lint gate.

## Out of Scope

- Drive-escalation wash recurrence — separate ticket S129CIREM-001.
- Late-game stuck idle in baseline / contested / scattered — completed in `archive/tickets/S129CIREM-002-late-game-stuck-idle.md`.
- Tell-session vs self-care — completed in `archive/tickets/S129CIREM-003-tell-session-vs-self-care.md`.
- Patrol motive arithmetic redesign for non-route-aware patrol — out of scope unless the trace shows it is needed.
- Pursuit / hostility behavior — already covered by the same scenario but tested via different assertions (`first_remote_pursuit_candidate_tick`, `attack_committed`); those are not failing.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_patrol::survival_patrol_proves_patrol_and_remote_pursuit_execution` — `first_market_road_patrol_tick` is `Some`.
2. `golden_survival_patrol::survival_patrol_replay_is_deterministic` — same outcome as the proof test under the same seed.
3. Focused retry-state regression for patrol frontier exhaustion.
4. Existing suite: `cargo test --workspace`, `./scripts/verify.sh`.

### Invariants

1. **Patrol routes make forward progress**: an agent assigned `patrol_route.assigned_places` with N >= 2 entries must commit `patrol` at every assigned waypoint within the survival contract's tick budget when the path is reachable and competing critical needs do not chronically override.
2. **Ranking substrate, not weight knob**: any change to patrol motive arithmetic introduces a concrete-state input (e.g. time-since-last-patrol-at-waypoint, route-progression state) rather than a static weight bump.
3. **No special-case rule**: patrol does not get a hardcoded "patrol > self-care" carve-out. The fix is named in concrete state + ranking.

## Test Plan

### New/Modified Tests

1. `agent_tick::planning::tests::record_exhausted_goals_records_patrol_frontier_exhaustion_as_budget_retry`.
2. Existing `golden_survival_patrol` ignored golden proves the full survival-patrol route and self-care contract.

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::record_exhausted_goals_records_patrol_frontier_exhaustion_as_budget_retry -- --exact`
2. `cargo test --release -p worldwake-ai --test golden_survival_patrol -- --ignored --test-threads=1`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-01.

- Reassessed the stale Market Road failure against the live branch. Both waypoint patrol assertions now pass; the remaining live failure was the later survival-health idle-window assertion.
- Changed `frontier_exhaustion_entry` so `GoalKind::Patrol` uses `BudgetRetryPending` instead of permanent `FrontierExhausted` suppression.
- Added focused unit coverage for the patrol retry-state contract.
- Fixed two existing current-toolchain Clippy `map_unwrap_or` violations exposed by `./scripts/verify.sh`; these were mechanical rewrites with no semantic change.
- Left patrol route advancement, patrol motive arithmetic, scenario authoring, and save format unchanged.

## Deviations

- The drafted route-aware motive boost was not the live fix. The concrete state seam was `AgentDecisionRuntime.exhaustion_cache`: recurring patrol duties need cooldown retry after frontier exhaustion.
- The drafted focused goldens were replaced with a lower-layer unit regression plus the existing scenario-backed golden, because live diagnostics showed route advancement and Market Road patrol already worked.
- A temporary diagnostic test was used during reassessment and removed before final verification.

## Verification Result

Passed:

1. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::record_exhausted_goals_records_patrol_frontier_exhaustion_as_budget_retry -- --exact`
1. `cargo test --release -p worldwake-ai --test golden_survival_patrol -- --ignored --test-threads=1`
1. `cargo test -p worldwake-ai`
1. `cargo test --workspace`
1. `./scripts/verify.sh`
