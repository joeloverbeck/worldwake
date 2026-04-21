# S113BELENV-003: Envelope consumers — ranking, plan revalidation, feasibility probe

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI ranking formula, exact-target revalidation predicate, failure classification on invalid revalidation, S112 feasibility probe rejection path, belief-driven decision-payload snapshot population on the affected AI emission paths
**Deps**: archive/tickets/S113BELENV-001.md, archive/tickets/S113BELENV-002.md, archive/tickets/S113BELENV-006.md

## Problem

With envelope accessors landed (T001), four downstream consumers in `worldwake-ai` can now reason about belief confidence and status instead of treating every belief as crisp:

1. **Ranking** (`motive_score` in `ranking.rs:747`) — the live ranking seam already has `RankingContext.view: &dyn GoalBeliefView`, but the current raid-target motive path still treats target-presence confidence like crisp knowledge. Agents rank a raid backed by a fresh location belief the same as one backed by eroded rumor, which violates FND-20 (resource-bounded practical reasoning — agents should weight act-vs-verify).
2. **Plan revalidation** (`revalidate_exact_target_step` in `plan_revalidation.rs:84`) — once claim-level refutation carriage lands in `S113BELENV-006`, identity-bound steps (S108 `BindingStrictness::ExactIdentity`) should short-circuit when the target-presence belief is `Contradicted`, so agents do not waste a tick walking into a refuted belief. The live seam here is still boolean-only; contradiction/staleness classification must happen downstream.
3. **Failure classification for invalid revalidation** (`failure_handling.rs`) — invalid exact-target revalidation currently flows through `handle_current_step_failure(..., execution_failure: None)` and falls back to generic discrepancy classification. Once revalidation becomes envelope-aware, this lane must classify `BeliefContradicted` / `BeliefStale` from the same live envelope instead of collapsing to `ImproperPlanningState`.
4. **Feasibility probe** (`feasibility_probe.rs` + `FeasibilityVerdict` in `agent_tick/portfolio.rs:29-31`) — currently has no envelope-aware rejection for stale beliefs; S112's information-gathering slot cannot activate on the `Stale + ExactIdentity` case because the probe never emits `BeliefStale`.
5. **Decision payload population** (`agent_tick/execution.rs` blocker-memory persistence sites) — `S113BELENV-002` landed the optional `belief_snapshot` field and save-format bump, but the live runtime emitters still lawfully write `None`. On the affected discrepancy/blocker persistence paths, those same branches should capture the live envelope into `BlockerRecordedPayload` so decision history stays reconstructible.

The three reasoning integrations are small single-function modifications that share the same pattern — read envelope, branch on `status`/`confidence` — and the paired decision-payload population rides the same branch family. Bundling them into one ticket keeps the pattern-establishing review in one diff.

## Assumption Reassessment (2026-04-21)

1. `motive_score` at `crates/worldwake-ai/src/ranking.rs:747` has signature `fn motive_score(candidate: &GroundedGoal, context: &RankingContext<'_>) -> u32`. Goal-kind-specific subscore aggregation; no current multiplier for belief confidence. The reassessment confirms `RankingContext<'_>` already exposes `view: &dyn GoalBeliefView`, so no context-threading change is needed. The most honest live belief-anchored ranking seam is `GoalKind::RaidTarget` via `raid_target_motive(...)`: the loot calculation stays crisp, but the target-presence envelope can still scale the final motive by location confidence.
2. `FeasibilityVerdict` at `crates/worldwake-ai/src/agent_tick/portfolio.rs:29-31` carries two variants: `Plausible` and `RejectedBeforeSearch { reason: Discrepancy }`. `Discrepancy::BeliefStale` and `Discrepancy::BeliefContradicted` already exist in `crates/worldwake-core/src/discrepancy.rs:6-25` (lines 8 and 10). No new enum variants needed.
3. Shared abstraction boundary under audit: the envelope-read contract on the `GoalBeliefView` / `RuntimeBeliefView` surface. Ranking (priority path), exact-target revalidation (execution predicate), failure classification (post-invalid-revalidation discrepancy lane), and probe (pre-search path) are distinct phases but they share the same read contract — each branches on `status` and scales or rejects.
4. `revalidate_exact_target_step` at `crates/worldwake-ai/src/plan_revalidation.rs:84` takes `view: &dyn RuntimeBeliefView` and returns `bool`, not a richer discrepancy result. Invalid exact-target revalidation currently flows through `agent_tick/execution.rs::enqueue_valid_step_or_handle_failure(...)` into `handle_current_step_failure(..., execution_failure: None)`, so contradiction/staleness ownership belongs in `failure_handling.rs` rather than in a changed `plan_revalidation` return type.
5. `failure_handling.rs::classify_discrepancy(...)` already classifies `Discrepancy::BeliefContradicted` for local commodity availability and already has clearing semantics for both `BeliefStale` and `BeliefContradicted`. No new discrepancy variants or clearing taxonomy are needed; this ticket extends the live triggers.
6. `feasibility_probe.rs:29-69` already has access to `context.belief_view: &dyn RuntimeBeliefView`. `known_target_failure(...)` currently decides target known-ness and route viability from `entity_kind`, `effective_place`, `is_alive`, and `is_dead`, with no use of `believed_target_location(...)`. This is the live seam for stale pre-search rejection.
7. This is a planner-layer ticket. Live `GoalKind` surfaces under test: **identity-bound goals** (those whose active action has `BindingStrictness::ExactIdentity` per `crates/worldwake-sim/src/action_def.rs:12-18`) are the set where the revalidation, failure-classification, and probe rejections fire. Ranking scaling is narrowed to the live belief-anchored goal family whose motive already depends on a remote target: `GoalKind::RaidTarget`, scaled by target-location confidence.
6. Intended verification layer: focused unit tests for each of the three integrations (ranking arithmetic precision, revalidation short-circuit, probe rejection reason). No golden-test changes in-scope (golden is T005).
7. Ordering contract: none of the three integrations change lifecycle ordering. Ranking scaling is a motive-score transform — the ordering rule is still "higher score first"; the scale just changes what scores are produced. Revalidation and probe add *rejection* paths, not ordering changes. Compared branches (belief-driven vs non-belief-driven ranking) are symmetric in the current architecture — they use the same scoring substrate; envelope scaling adds a multiplicative post-factor.
8. No heuristic is being removed. The envelope-confidence multiplier is a new substrate that supplements existing motive scoring, not a replacement for an existing filter.
9. For revalidation, the first-failure-boundary classification is **authoritative start / post-start abort** — the predicate lives inside `revalidate_exact_target_step` which runs pre-commit; the `Contradicted` return escalates to the AI layer's plan-failure handler via `Discrepancy::BeliefContradicted`. Shared runtime request path checked: `plan_revalidation.rs::revalidate_next_step` (line 14) → `revalidate_exact_target_step` (line 84). Proof surface: focused runtime coverage in plan_revalidation.rs `#[cfg(test)]` at line 228.
13. Adjacent contradictions: `S113BELENV-001` lands `BeliefStatus::Contradicted` as staged taxonomy only; live contradiction derivation is deferred to `S113BELENV-006` because the current claim store has no explicit refutation marker. This ticket therefore depends on `S113BELENV-006` for the contradiction-driven branches and can still land stale-belief handling independently if reassessed that way later.

## Architecture Check

1. Each integration consumes the envelope through the existing `GoalBeliefView` / `RuntimeBeliefView` trait surface — no new cross-system authority path, no direct belief-store reads (P26). The consumers read state and decide; the belief-view trait mediates.
2. Ranking scaling preserves deterministic integer arithmetic. `ranking.rs` already has `effective_motive_score(base_score, multiplier)` for permille scaling, so this ticket should reuse that helper instead of open-coding a second multiply/divide path.
3. Exact-target revalidation remains a boolean predicate. This ticket adds a pre-filter for contradicted location beliefs there, but the discrepancy classification stays downstream in `failure_handling.rs`, which is the live place that turns invalid revalidation into `Discrepancy::*`.
4. Probe rejection uses the existing `RejectedBeforeSearch { reason: Discrepancy::BeliefStale }` — same hygiene.
5. Populating `belief_snapshot` on the already-extended payload types is cleaner than leaving the decision-history contract staged indefinitely once these exact branches become envelope-aware. The live producer seam for this ticket is `BlockerRecordedPayload` on blocker/discrepancy persistence in `agent_tick/execution.rs`; `PlanInvalidatedPayload` is not currently the affected branch family.

## Verification Layers

1. Raid-target motive scaling preserves precision at representative Permille values (500, 1000, 0) → focused unit test in `ranking.rs` `#[cfg(test)]`.
2. Revalidation of an identity-bound step with `status == Contradicted` returns `false` pre-commit → focused unit test in `plan_revalidation.rs` `#[cfg(test)]`.
3. Invalid exact-target revalidation with `status == Contradicted` or `status == Stale` classifies to the matching `Discrepancy::*` instead of generic `ImproperPlanningState` → focused unit test in `failure_handling.rs` `#[cfg(test)]`.
4. Feasibility probe against an identity-bound target with `status == Stale` returns `FeasibilityVerdict::RejectedBeforeSearch { reason: Discrepancy::BeliefStale }` → focused unit test in `feasibility_probe.rs`.
5. Belief-driven `BlockerRecordedPayload` emitted from the newly wired branches carries `belief_snapshot: Some(...)` with the live envelope metadata → focused runtime/decision-payload tests on the affected `agent_tick` persistence seams.
6. No action-trace layer is required for this ticket — the integrations are reasoning-layer plus decision-history enrichment. Scenario-level proof is deferred to T005 (golden).

## What to Change

### 1. Ranking: envelope-aware raid-target motive scaling

In `crates/worldwake-ai/src/ranking.rs`, inside `raid_target_motive(...)` or a helper it calls:

- Read `context.view.believed_target_location(agent, target)` for the raid target.
- Scale the final raid motive by the target-location envelope confidence after computing the existing loot-driven base score.
- Non-raid goal families are unchanged in this ticket. If reassessment discovers another already-live belief-envelope motive seam in the same file, it may be included, but the minimum honest scope is raid-target ranking.

### 2. Plan revalidation: `Contradicted` short-circuit at the boolean seam

In `crates/worldwake-ai/src/plan_revalidation.rs`, inside `revalidate_exact_target_step` (around lines 101-117 per prior reassessment):

- When the step is identity-bound (S108 `BindingStrictness::ExactIdentity` on the action def), read `view.believed_target_location(agent, target)`.
- If `envelope.status == BeliefStatus::Contradicted`, return `false`.
- Otherwise proceed with existing revalidation logic. The new check is a pre-filter, not a replacement.

### 3. Failure classification: map invalid exact-target envelopes to discrepancy reasons

In `crates/worldwake-ai/src/failure_handling.rs`:

- On the invalid exact-target revalidation lane that currently falls through to generic discrepancy handling, re-read `view.believed_target_location(agent, target)`.
- If `envelope.status == BeliefStatus::Contradicted`, classify `Discrepancy::BeliefContradicted`.
- If `envelope.status == BeliefStatus::Stale`, classify `Discrepancy::BeliefStale`.
- Otherwise preserve the existing fallback behavior. Do not add new discrepancy variants or clearing rules.

### 4. Feasibility probe: `BeliefStale` rejection for identity-bound targets

In `crates/worldwake-ai/src/feasibility_probe.rs`:

- Before committing to full tactical search, read the target-presence envelope when the goal's active binding strictness is `ExactIdentity`.
- If `envelope.status == BeliefStatus::Stale`, return `FeasibilityVerdict::RejectedBeforeSearch { reason: Discrepancy::BeliefStale }`.
- `Certain` / `Probable` / `Disputed` proceed to `Plausible`. `Contradicted` is also a lawful pre-search rejection if encountered here; prefer to reject it as `Discrepancy::BeliefContradicted` rather than knowingly proceeding into a refuted target belief.

### 5. Decision-event payload population on the newly wired branches

On the blocker/discrepancy persistence paths that now branch on the envelope:

- When a blocker-memory persistence site emits `BlockerRecordedPayload` for `Discrepancy::BeliefStale` or `Discrepancy::BeliefContradicted` on the newly wired paths, populate `belief_snapshot` from the same envelope read that triggered the branch instead of leaving `None`.
- Reuse the existing `BeliefSnapshot` / `BeliefStatusTag` schema from `S113BELENV-002`; no payload-shape changes are needed here. Use the existing shared converter path if available rather than inventing a duplicate helper.

### 6. Unit tests

Add to each consumer's `#[cfg(test)]` block:

- **ranking.rs** — raid-target motive scaling with target-location `confidence = Permille(500)` halves the final motive; `confidence = Permille(1000)` preserves it; `confidence = Permille(0)` zeroes it.
- **plan_revalidation.rs** — revalidate an identity-bound step whose target's `believed_target_location` returns `status: Contradicted` → `false`. Companion test: same step with `status: Certain` → proceeds.
- **failure_handling.rs** — invalid exact-target revalidation with `status: Contradicted` / `status: Stale` classifies to the matching discrepancy instead of `ImproperPlanningState`.
- **feasibility_probe.rs** — probe against identity-bound target with `status: Stale` → `RejectedBeforeSearch { reason: Discrepancy::BeliefStale }`; companion `Contradicted` test if the live probe seam also rejects that state.
- **agent_tick emission tests** — the affected blocker/discrepancy payloads carry `belief_snapshot: Some(...)` with the expected `confidence`, `status`, and `acquired_tick`.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — raid-target motive scaling through target-location confidence)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — contradicted exact-target predicate in `revalidate_exact_target_step`)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — classify stale/contradicted exact-target invalidation on the failure lane)
- `crates/worldwake-ai/src/feasibility_probe.rs` (modify — stale pre-search rejection for identity-bound targets, with contradicted rejection if the live seam supports it cleanly)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — populate `belief_snapshot` on the newly belief-driven blocker/discrepancy emission paths)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — payload assertions for populated snapshots)

## Out of Scope

- Candidate-generation integration (T004 — distinct file and distinct set of emitter decisions).
- Golden-test extension (T005).
- Changes to `FeasibilityVerdict` variant set (none needed — existing `Discrepancy` reason is sufficient).

## Acceptance Criteria

### Tests That Must Pass

1. Focused unit tests for ranking, revalidation, failure classification, feasibility probe, and the newly populated decision-payload seams per §6 pass.
2. `cargo test -p worldwake-ai ranking` passes.
3. `cargo test -p worldwake-ai plan_revalidation` passes.
4. `cargo test -p worldwake-ai failure_handling` passes.
5. `cargo test -p worldwake-ai feasibility_probe` passes.
6. Full AI suite: `cargo test -p worldwake-ai` passes (catches regression in existing tests at `plan_revalidation.rs:880,907,...`).

### Invariants

1. Motive-score scaling is deterministic integer arithmetic; no floats introduced (CLAUDE.md Determinism).
2. Revalidation on a `status == Certain` identity-bound step produces the same outcome as pre-ticket (no behavior change for the fresh-belief case).
3. Newly belief-driven `BlockerRecordedPayload` emissions from this ticket's branches populate `belief_snapshot` instead of leaving `None`.
4. Probe returns `Plausible` for every envelope state other than the explicitly rejected stale/contradicted target states on the identity-bound seam.
5. No changes to `Discrepancy` enum variants, `FeasibilityVerdict` enum variants, or `BindingStrictness` enum variants (P28 — reuse existing types).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` `#[cfg(test)]` — focused raid-target motive scaling tests.
2. `crates/worldwake-ai/src/plan_revalidation.rs` `#[cfg(test)]` — identity-bound revalidation tests for `Contradicted` vs `Certain`.
3. `crates/worldwake-ai/src/failure_handling.rs` `#[cfg(test)]` — exact-target invalidation classification tests for `Stale` / `Contradicted`.
4. `crates/worldwake-ai/src/feasibility_probe.rs` `#[cfg(test)]` — probe rejection tests for the stale identity-bound target seam.
5. `crates/worldwake-ai/src/agent_tick/tests.rs` — focused payload assertions proving populated `belief_snapshot` on the newly wired blocker/discrepancy branches.

### Commands

1. `cargo test -p worldwake-ai ranking` (targeted ranking).
2. `cargo test -p worldwake-ai plan_revalidation` (targeted revalidation).
3. `cargo test -p worldwake-ai failure_handling` (targeted failure classification).
4. `cargo test -p worldwake-ai feasibility_probe` (targeted probe).
5. `cargo test -p worldwake-ai` (full AI suite).
6. `cargo clippy --workspace --all-targets -- -D warnings`.
7. `./scripts/verify.sh` before PR.

## Outcome

Implemented on 2026-04-21.

- `ranking.rs` now scales `RaidTarget` motive by the target-location envelope confidence instead of treating remote target presence as crisp.
- `plan_revalidation.rs` now rejects exact-identity steps up front when any specific target's location envelope is `Contradicted`.
- `failure_handling.rs` now classifies invalid exact-target revalidation as `Discrepancy::BeliefStale` / `Discrepancy::BeliefContradicted` on the explicit revalidation-failure lane instead of falling through to `ImproperPlanningState`.
- `feasibility_probe.rs` now rejects stale and contradicted exact-target beliefs before search on the identity-bound seam.
- `agent_tick/execution.rs` now populates `BlockerRecordedPayload.belief_snapshot` for target-belief discrepancy entries by re-reading the live target-location envelope at emit time.
- Deviation from earlier draft scope: ranking integration landed at the strongest honest live seam, `RaidTarget` target-location confidence scaling, rather than a broader generic stock/location motive rollout; live decision-payload population likewise landed on the `BlockerRecordedPayload` discrepancy branches touched by this ticket, while `PlanInvalidatedPayload` producer wiring remains deferred.

## Verification Result

Passed on 2026-04-21:

1. `cargo test -p worldwake-ai ranking`
2. `cargo test -p worldwake-ai plan_revalidation`
3. `cargo test -p worldwake-ai feasibility_probe`
4. `cargo test -p worldwake-ai failure_handling`
5. `cargo test -p worldwake-ai persist_discrepancy_memory_captures_belief_snapshot_for_target_belief_discrepancy`
6. `cargo fmt --all`
7. `cargo test -p worldwake-ai`

Not run:

1. `cargo clippy --workspace --all-targets -- -D warnings`
2. `./scripts/verify.sh`
