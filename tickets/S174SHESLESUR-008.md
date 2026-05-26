# S174SHESLESUR-008: Scenario B — survival-sleep-contention.ron (multi-slot contention + S44 queue promotion)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None (golden scenario + test file only)
**Deps**: `archive/tickets/S174SHESLESUR-001.md`, `archive/tickets/S174SHESLESUR-002.md`, `archive/tickets/S174SHESLESUR-003.md`, `archive/tickets/S174SHESLESUR-004.md`, `archive/tickets/S174SHESLESUR-005.md`, `archive/tickets/S174SHESLESUR-006.md`

## Problem

S174's Scenario B proves multi-slot rest-site contention and S44 queue promotion. With a single capacity-2 barracks and three tired agents, two should occupy and one should either queue (via S44's `PromotableContentionKind::RestSite` substrate) or rough-sleep. Queue grant promotion fires when one occupant releases. Without this scenario, the queue/grant promotion path for rest sites is not exercised end-to-end.

## Assumption Reassessment (2026-05-26)

1. Verified current code state: S44 contention substrate at `crates/worldwake-systems/src/facility_queue.rs` provides `ContentionQueue` + `ContentionPolicy` + grant/expiry semantics. `archive/tickets/S174SHESLESUR-004.md` added `PromotableContentionKind::RestSite` and the matching `contention_target_matches_kind` arm. `EventTag::ContentionResolved` and `EventTag::QueueGrantPromoted` exist (per S142 substrate) and fire on grant/promotion lifecycle events.
2. Spec assumption verified against S174 Scenario B. The scenario uses one place (`barracks` with `RestCapacity(2)` + roofed `SleepQualityProfile`) and three tired agents. Per spec, assertions include: two agents occupy, third either queues or rough-sleeps; queue grant promotion fires when one occupant releases; no stuck-idle window under elevated fatigue.
3. Shared abstraction boundary under audit: S44 queue substrate for rest sites (mirroring the Wash/Latrine queue paths from S173). The scenario exercises whether `RestSite` classification is properly recognized by the queue promotion logic.
4. Live `GoalKind` under test: `GoalKind::Sleep`. Operator surface: `DECL_SLEEP.relevant_ops = [Sleep, QueueForFacilityUse]` and `DECL_SLEEP.progress_barrier_ops = [Sleep]` (`archive/tickets/S174SHESLESUR-005.md` added the queue op to the relevant surface only). The scenario specifically exercises `QueueForFacilityUse` — the third agent's path through queue join → grant promotion → sleep start.
5. Ranking-sensitive: when two of three agents must occupy and one must queue, `archive/tickets/S174SHESLESUR-005.md`'s emission produces three KnownRestSite candidates (one per agent). The ranking layer (`ranking.rs::motive_score`) determines who wins the start race. Verify symmetry — all three agents have identical fatigue and identical sleep urgency, so ranking falls back to a deterministic tiebreaker (likely agent EntityId order). This is FOUNDATIONS-aligned only if the tiebreaker is explicit and inspectable.
6. Cumulative arithmetic: the scenario runs until all three agents complete a Sleep episode. Pick metabolisms such that occupants reach `target_recovery` within ~40-60 ticks (matching Scenario A's recovery cadence) so queue promotion has time to fire and the third agent completes within a reasonable horizon (e.g., 150 ticks).
7. Scenario isolation: the intended branch under test is multi-slot capacity + S44 queue promotion for rest sites. Lawful competing affordances excluded: other survival actions, multiple rest sites. The scenario must keep agents focused on the one barracks.

## Architecture Check

1. The scenario reuses S44's contention queue rather than introducing a parallel rest-queue mechanism. Per FOUNDATIONS FND-26 and S174's D2, the queue substrate is shared with Wash/Latrine — the only distinguishing characteristic is the `PromotableContentionKind::RestSite` discriminator (introduced in `archive/tickets/S174SHESLESUR-004.md`).
2. Capacity-2 (rather than capacity-3 with three agents all occupying) is the right scenario shape because capacity-3 would not exercise the queue path. The third agent's queue join is the headline behavior.
3. Deterministic tiebreaker for the start race is part of the scenario's contract — exposing the tiebreaker is a FOUNDATIONS-aligned debug surface, not a hidden authority.

## Verification Layers

1. All three agents emit KnownRestSite candidates targeting `barracks` -> decision trace
2. Two agents successfully start Sleep and write `RestOccupancy.occupants` -> event-log delta + authoritative world state (`RestOccupancy.occupants.len() == 2`)
3. Third agent's Sleep start fails the rest-site precondition -> action trace (Aborted event)
4. Third agent queues via S44's `PromotableContentionKind::RestSite` substrate -> `EventTag::ContentionResolved` with queue-position assignment (or rough-sleep emission if the agent chooses fallback over wait — depends on patience profile)
5. When one occupant commits and releases `RestOccupancy`, the queued agent receives a grant via `EventTag::QueueGrantPromoted` -> event-log assertion
6. The promoted agent's Sleep start succeeds and writes `RestOccupancy.occupants` -> event-log delta
7. No stuck idle: no agent has an elevated-fatigue window without a Sleep attempt or queue join within the scenario horizon -> `CriticalWindowReport.frames` length per agent assertion
8. Deterministic replay -> identical state hashes across two runs

## What to Change

### 1. Author the scenario RON file

Create `scenarios/survival-sleep-contention.ron` with:

- One place: `barracks` with `SleepQualityProfile { shelter: Roofed, ground_comfort: Soft, recovery_modifier: 1100 }` and `rest_capacity: Some(2)`
- Three agents (`Aster`, `Bram`, `Cleo`) co-located at `barracks`, all tired (high fatigue, near-sated other needs)
- Stable ChaCha8Rng seed
- `MetabolismProfile.rough_sleep_recovery_floor` default (300 permille)
- `ContentionPolicy` on `barracks` set so the queued agent will wait rather than immediately rough-sleeping (verify the policy fields at ticket-implementation time — `grant_hold_ticks` should be > 0)

### 2. Author the corresponding test file

Create `crates/worldwake-ai/tests/scenarios/survival_sleep_contention.rs` following Scenario A's structure. Assertions:

- 3 KnownRestSite emissions on tick 0 (one per agent)
- 2 occupancy writes (winning two agents)
- 1 precondition rejection for the third agent
- 1 queue join via `EventTag::ContentionResolved`
- 1 commit → 1 grant promotion → 1 promoted occupancy → 1 commit for the promoted agent
- Per-agent `CriticalWindowReport` has at most 1 frame with elevated fatigue, no stuck-idle windows

### 3. Hook the test

Add `mod survival_sleep_contention;` to `tests/scenarios/mod.rs`.

## Files to Touch

- `scenarios/survival-sleep-contention.ron` (new)
- `crates/worldwake-ai/tests/scenarios/survival_sleep_contention.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify — add `mod survival_sleep_contention;`)

## Out of Scope

- Single-slot rest-site contention (Scenario A / ticket 007)
- Hostile-proximity interruption (Scenario C / ticket 009)
- CLI player-POV (Scenario D / ticket 010)
- Failed-rest cascade for S175 (Scenario E / ticket 011)
- No production code changes

## Acceptance Criteria

### Tests That Must Pass

1. New scenario test `survival_sleep_contention::scenario_b_multi_slot_contention` passes all 8 verification-layer assertions
2. Deterministic replay test passes
3. Existing suite: `cargo test --workspace` passes

### Invariants

1. `RestOccupancy.occupants.len()` never exceeds 2 at `barracks` (the `RestCapacity` cap)
2. Queue grant promotion fires exactly once per occupant release
3. The third agent's Sleep episode completes — no agent permanently stuck at elevated fatigue under nominal scenario conditions

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_sleep_contention.rs` (new) — Scenario B E2E

### Commands

1. `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_sleep_contention`
2. `cargo test --workspace`
3. `./scripts/verify.sh`
