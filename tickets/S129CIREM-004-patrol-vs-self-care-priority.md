# S129CIREM-004: Guard never patrols Market Road waypoint under post-S129 ranking

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Likely — patrol motive arithmetic vs. self-care goals at waypoint places
**Deps**: archive/specs/S129-place-dirtiness-facility-wear.md, prior patrol spec (search archive for `patrol_route` originator)

## Problem

`golden_survival_patrol::survival_patrol_proves_patrol_and_remote_pursuit_execution` and `golden_survival_patrol::survival_patrol_replay_is_deterministic` both fail at `crates/worldwake-ai/tests/golden_survival_patrol.rs:257` with `guard should commit patrol at Market Road`. The companion expectation `first_watch_post_patrol_tick.expect("guard should commit patrol at Watch Post")` (line 254) succeeds, so the guard *does* commit patrol at the starting waypoint — but never reaches the second waypoint and never commits patrol there.

`Guard Mira`'s patrol route is `assigned_places: ["Watch Post", "Market Road"]` (`scenarios/survival-patrol.ron:197–199`). Market Road is `[Field, Farm, Latrine]`-tagged with co-located water (`Water` resource source), apples, and a `WashBasin` (`scenarios/survival-patrol.ron:23, 209–216`). The 3-tick travel from Watch Post to Market Road is bidirectional (`scenarios/survival-patrol.ron:27`).

`Guard Mira`'s `patrol_profile.patrol_motive_weight: 100` is modest (`scenarios/survival-patrol.ron:195`). Per `crates/worldwake-ai/src/ranking.rs::patrol_motive` (~line 1553), patrol motive multiplies the base weight by `1 + unresolved_thefts + believed_vacancies + believed_contests`. With no unresolved thefts and no office vacancies in the scenario, patrol motive stays at the base `100` — modest enough that any moderate-pressure self-care goal at Market Road would outrank it on arrival.

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

### 1. Investigation: dump Mira's decision and action traces

Add a one-shot diagnostic test that runs `survival_patrol` and prints, for Mira:

- Per-tick decision trace from her first patrol commit at Watch Post through tick 200 (well past the expected first-Market-Road-arrival tick under base_dwell + travel).
- Per-tick action trace for the same range (look for `travel`, `patrol`, self-care commits).
- The `patrol_route` component snapshot at each commit point — confirm `current_index` advancement.

### 2. Fix (driven by trace findings)

Choose exactly one based on the trace:

- **If the patrol route never advances after the first commit**: route-advancement bug. Fix: ensure `commit_patrol` (or whichever handler advances the route) writes `current_index = (current_index + 1) % assigned_places.len()` consistently.
- **If the route advances but Mira does not start travel toward Market Road**: candidate generation or planning bug. The `Patrol` goal at the next-index waypoint should generate a travel-to-waypoint plan; verify the planner is composing it.
- **If Mira reaches Market Road but never commits `patrol` there**: on-arrival ranking dominated by self-care. Fix: introduce route-aware patrol priority. When the agent is mid-route and the next waypoint is `current place`, patrol motive should rise — concrete state input is `time_since_last_patrol_at_current_index` (if such state exists; introduce it if not). Do not just bump the static `patrol_motive_weight`.
- **If Mira's hunger/thirst/fatigue/bladder dominates self-care for the entire 1440-tick window**: the scenario tuning may be too tight and the ticket scope expands to include scenario rebalance. Verify against Mira's metabolism rates.

### 3. Targeted golden coverage

After the fix, add focused goldens:

- `patrol_route_advances_after_dwell_at_waypoint` — single-agent patrol route, assert `current_index` rotates correctly across waypoints.
- `patrol_takes_priority_over_routine_self_care_at_waypoint` — single-agent fixture, agent at next waypoint with moderate (sub-critical) need, assert patrol commits before agent diverts to self-care.

## Files to Touch

- `crates/worldwake-ai/tests/` (new diagnostic + new focused golden)
- `crates/worldwake-ai/src/ranking.rs::patrol_motive` (only if route-aware boost is the fix)
- `crates/worldwake-systems/src/` (only if route-advancement is the fix; search for the patrol commit handler)
- `crates/worldwake-core/src/` (only if a new `last_patrolled_at` per-waypoint state is needed; this would be a new component)

## Out of Scope

- Drive-escalation wash recurrence — separate ticket S129CIREM-001.
- Late-game stuck idle in baseline / contested / scattered — completed in `archive/tickets/S129CIREM-002-late-game-stuck-idle.md`.
- Tell-session vs self-care — separate ticket S129CIREM-003.
- Patrol motive arithmetic redesign for non-route-aware patrol — out of scope unless the trace shows it is needed.
- Pursuit / hostility behavior — already covered by the same scenario but tested via different assertions (`first_remote_pursuit_candidate_tick`, `attack_committed`); those are not failing.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_patrol::survival_patrol_proves_patrol_and_remote_pursuit_execution` — `first_market_road_patrol_tick` is `Some`.
2. `golden_survival_patrol::survival_patrol_replay_is_deterministic` — same outcome as the proof test under the same seed.
3. New focused goldens (one per sub-shape identified by the trace).
4. Existing suite: `cargo test --workspace`, `./scripts/verify.sh`.

### Invariants

1. **Patrol routes make forward progress**: an agent assigned `patrol_route.assigned_places` with N >= 2 entries must commit `patrol` at every assigned waypoint within the survival contract's tick budget when the path is reachable and competing critical needs do not chronically override.
2. **Ranking substrate, not weight knob**: any change to patrol motive arithmetic introduces a concrete-state input (e.g. time-since-last-patrol-at-waypoint, route-progression state) rather than a static weight bump.
3. **No special-case rule**: patrol does not get a hardcoded "patrol > self-care" carve-out. The fix is named in concrete state + ranking.

## Test Plan

### New/Modified Tests

1. New diagnostic test (one-shot) — dumps Mira's decision/action/patrol-route traces. Disposable.
2. `patrol_route_advances_after_dwell_at_waypoint` (focused).
3. `patrol_takes_priority_over_routine_self_care_at_waypoint` (focused) — only needed if the trace shows on-arrival ranking is the bug.

### Commands

1. `cargo test --release -p worldwake-ai --test golden_survival_patrol -- --ignored --test-threads=1`
2. `cargo test --workspace`
3. `./scripts/verify.sh`
