# S113BELENV-003: Envelope consumers — ranking, plan revalidation, feasibility probe

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI ranking formula, plan revalidation predicate, S112 feasibility probe rejection path
**Deps**: S113BELENV-001

## Problem

With envelope accessors landed (T001), three downstream consumers in `worldwake-ai` can now reason about belief confidence and status instead of treating every belief as crisp:

1. **Ranking** (`motive_score` in `ranking.rs:747`) — currently discounts nothing for stale beliefs because the signal doesn't exist. Agents rank a goal tied to a fresh observation the same as one tied to an eroded rumor, which violates FND-20 (resource-bounded practical reasoning — agents should weight act-vs-verify).
2. **Plan revalidation** (`revalidate_exact_target_step` in `plan_revalidation.rs:84`) — identity-bound steps (S108 `BindingStrictness::ExactIdentity`) currently do not short-circuit when the target-presence belief is `Contradicted`, so agents can waste a tick walking into a refuted belief.
3. **Feasibility probe** (`feasibility_probe.rs` + `FeasibilityVerdict` in `agent_tick/portfolio.rs:29-31`) — currently has no envelope-aware rejection for stale beliefs; S112's information-gathering slot cannot activate on the `Stale + ExactIdentity` case because the probe never emits `BeliefStale`.

All three integrations are small single-function modifications that share the same pattern — read envelope, branch on `status`/`confidence`. Bundling them into one ticket keeps the pattern-establishing review in one diff.

## Assumption Reassessment (2026-04-21)

1. `motive_score` at `crates/worldwake-ai/src/ranking.rs:747` has signature `fn motive_score(candidate: &GroundedGoal, context: &RankingContext<'_>) -> u32`. Goal-kind-specific subscore aggregation; no current multiplier for belief confidence. The `RankingContext<'_>` must expose (or already expose) `&dyn GoalBeliefView` for the new scaling to read the envelope. `revalidate_exact_target_step` at `crates/worldwake-ai/src/plan_revalidation.rs:84` takes the belief view via `view: &dyn RuntimeBeliefView`. Feasibility probe at `crates/worldwake-ai/src/feasibility_probe.rs:29-69` has access to `context.belief_view: &dyn RuntimeBeliefView` (line 18).
2. `FeasibilityVerdict` at `crates/worldwake-ai/src/agent_tick/portfolio.rs:29-31` carries two variants: `Plausible` and `RejectedBeforeSearch { reason: Discrepancy }`. `Discrepancy::BeliefStale` and `Discrepancy::BeliefContradicted` already exist in `crates/worldwake-core/src/discrepancy.rs:6-25` (lines 8 and 10). No new enum variants needed.
3. Shared abstraction boundary under audit: the envelope-read contract on the `GoalBeliefView` / `RuntimeBeliefView` surface. Ranking (on the priority path), revalidation (on the execution path), and probe (on the pre-search path) are three distinct phases but they share the same read contract — each branches on `status` and scales or rejects.
5. This is a planner-layer ticket. Live `GoalKind` surfaces under test: **identity-bound goals** (those whose active action has `BindingStrictness::ExactIdentity` per `crates/worldwake-sim/src/action_def.rs:12-18`) are the set where the revalidation and probe rejections fire. Ranking scaling applies to any goal whose `motive_score` consults belief-based data. Verify before implementation that `RankingContext` carries a belief view; if not, either thread one in (scope extension) or limit the scaling to goals whose context already includes belief signals (narrower but still useful).
6. Intended verification layer: focused unit tests for each of the three integrations (ranking arithmetic precision, revalidation short-circuit, probe rejection reason). No golden-test changes in-scope (golden is T005).
7. Ordering contract: none of the three integrations change lifecycle ordering. Ranking scaling is a motive-score transform — the ordering rule is still "higher score first"; the scale just changes what scores are produced. Revalidation and probe add *rejection* paths, not ordering changes. Compared branches (belief-driven vs non-belief-driven ranking) are symmetric in the current architecture — they use the same scoring substrate; envelope scaling adds a multiplicative post-factor.
8. No heuristic is being removed. The envelope-confidence multiplier is a new substrate that supplements existing motive scoring, not a replacement for an existing filter.
9. For revalidation, the first-failure-boundary classification is **authoritative start / post-start abort** — the predicate lives inside `revalidate_exact_target_step` which runs pre-commit; the `Contradicted` return escalates to the AI layer's plan-failure handler via `Discrepancy::BeliefContradicted`. Shared runtime request path checked: `plan_revalidation.rs::revalidate_next_step` (line 14) → `revalidate_exact_target_step` (line 84). Proof surface: focused runtime coverage in plan_revalidation.rs `#[cfg(test)]` at line 228.
13. Adjacent contradictions: none surfaced during reassessment. The `FeasibilityVerdict` enum already carries `Discrepancy` as the rejection reason, so adding a new short-circuit requires no variant widening (unlike a naive "add new variant" approach).

## Architecture Check

1. Each of the three integrations consumes the envelope through the existing `GoalBeliefView` / `RuntimeBeliefView` trait surface — no new cross-system authority path, no direct belief-store reads (P26). The consumers read state and decide; the belief-view trait mediates.
2. Ranking scaling preserves deterministic integer arithmetic — the formula `(motive as u64).saturating_mul(conf.value() as u64) / 1000` multiplies before dividing and lifts to `u64` to avoid overflow (CLAUDE.md Determinism invariant: no floats). Confirmed Permille is `u16` in [0, 1000], `motive` is `u32`.
3. Revalidation short-circuit uses the existing `Discrepancy::BeliefContradicted` — no new discrepancy variant, no new rejection code path, just a new trigger condition (P28).
4. Probe rejection uses the existing `RejectedBeforeSearch { reason: Discrepancy::BeliefStale }` — same hygiene.

## Verification Layers

1. Motive scaling arithmetic preserves precision at representative Permille values (500, 1000, 0) → focused unit test in `ranking.rs` `#[cfg(test)]`.
2. Revalidation of an identity-bound step with `status == Contradicted` returns failure with `Discrepancy::BeliefContradicted` → focused unit test in `plan_revalidation.rs` `#[cfg(test)]` (existing block starts at line 228; tests at 865, 890, ...).
3. Feasibility probe against an identity-bound target with `status == Stale` returns `FeasibilityVerdict::RejectedBeforeSearch { reason: Discrepancy::BeliefStale }` → focused unit test in `feasibility_probe.rs` or the probe's existing test module.
4. No event-log-delta or action-trace layer is required for this ticket — the three integrations are all reasoning-layer changes. Scenario-level proof is deferred to T005 (golden).

## What to Change

### 1. Ranking: envelope-aware motive scaling

In `crates/worldwake-ai/src/ranking.rs`, inside `motive_score` (line 747 onward) or a helper called from it:

- For each goal whose kind is belief-anchored (target presence or remote commodity stock), read the corresponding envelope from the context's belief view.
- Apply the scaling:
  ```rust
  let scaled = (motive as u64)
      .saturating_mul(confidence.value() as u64)
      / 1000;
  u32::try_from(scaled).unwrap_or(u32::MAX)
  ```
- Non-belief-anchored goals are unchanged.
- Document the formula inline with a short comment naming the overflow-avoidance rationale and the multiply-before-divide precision rule.

If `RankingContext` does not currently carry a `&dyn GoalBeliefView`, thread one in by extending `RankingContext` (this is a scope extension; if the existing context already has an equivalent surface, use it). In either case, the threading change is local to `ranking.rs`.

### 2. Plan revalidation: `Contradicted` short-circuit

In `crates/worldwake-ai/src/plan_revalidation.rs`, inside `revalidate_exact_target_step` (around lines 101-117 per prior reassessment):

- When the step is identity-bound (S108 `BindingStrictness::ExactIdentity` on the action def), read `view.believed_target_location(agent, target)`.
- If `envelope.status == BeliefStatus::Contradicted`, return a revalidation failure with `Discrepancy::BeliefContradicted`.
- Otherwise proceed with existing revalidation logic. The new check is a pre-filter, not a replacement.

### 3. Feasibility probe: `BeliefStale` rejection for identity-bound targets

In `crates/worldwake-ai/src/feasibility_probe.rs`:

- Before committing to full tactical search, read the target-presence envelope when the goal's active binding strictness is `ExactIdentity`.
- If `envelope.status == BeliefStatus::Stale`, return `FeasibilityVerdict::RejectedBeforeSearch { reason: Discrepancy::BeliefStale }`.
- `Certain` / `Probable` / `Disputed` proceed to `Plausible`. `Contradicted` is also a rejection, but that case is already covered by revalidation in §2; decide during implementation whether the probe should duplicate the contradiction check (belt-and-braces) or delegate to revalidation (cleaner). Default: let revalidation handle `Contradicted`; the probe only rejects `Stale`.

### 4. Unit tests

Add to each consumer's `#[cfg(test)]` block:

- **ranking.rs** — `motive_score` with `confidence = Permille(500)` halves a `motive = 500` score to 250; `confidence = Permille(1000)` preserves it at 500; `confidence = Permille(0)` zeroes it.
- **plan_revalidation.rs** — revalidate an identity-bound step whose target's `believed_target_location` returns `status: Contradicted` → failure with `Discrepancy::BeliefContradicted`. Companion test: same step with `status: Certain` → proceeds. Grep the existing `#[cfg(test)]` block at line 228 for related revalidation tests to avoid naming collisions.
- **feasibility_probe.rs** — probe against identity-bound target with `status: Stale` → `RejectedBeforeSearch { reason: Discrepancy::BeliefStale }`. Companion: same target with `status: Certain` → `Plausible`.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — new scaling in `motive_score`; extend `RankingContext` only if belief view isn't already accessible)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — new predicate in `revalidate_exact_target_step`)
- `crates/worldwake-ai/src/feasibility_probe.rs` (modify — new `BeliefStale` rejection path)

## Out of Scope

- Candidate-generation integration (T004 — distinct file and distinct set of emitter decisions).
- Populating `belief_snapshot` on the `BlockerRecordedPayload` / `PlanInvalidatedPayload` emitted from these paths — T002 adds the field; populating it for specific `Discrepancy` variants is a follow-up once T002 lands (can be done in a later small ticket or absorbed into this one during implementation if the diff stays small).
- Golden-test extension (T005).
- Changes to `FeasibilityVerdict` variant set (none needed — existing `Discrepancy` reason is sufficient).

## Acceptance Criteria

### Tests That Must Pass

1. Six new unit tests (two per integration) per §4 pass.
2. `cargo test -p worldwake-ai ranking` passes.
3. `cargo test -p worldwake-ai plan_revalidation` passes.
4. `cargo test -p worldwake-ai feasibility_probe` passes.
5. Full AI suite: `cargo test -p worldwake-ai` passes (catches regression in existing tests at `plan_revalidation.rs:865,890,...`).

### Invariants

1. Motive-score scaling is deterministic integer arithmetic; no floats introduced (CLAUDE.md Determinism).
2. Revalidation on a `status == Certain` identity-bound step produces the same outcome as pre-ticket (no behavior change for the fresh-belief case).
3. Probe returns `Plausible` for every envelope state other than `Stale` (and `Contradicted` if the implementer chose the belt-and-braces variant).
4. No changes to `Discrepancy` enum variants, `FeasibilityVerdict` enum variants, or `BindingStrictness` enum variants (P28 — reuse existing types).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` `#[cfg(test)]` — 3 new unit tests for motive-score scaling precision.
2. `crates/worldwake-ai/src/plan_revalidation.rs` `#[cfg(test)]` — 2 new unit tests for identity-bound revalidation with `Contradicted` vs `Certain`.
3. `crates/worldwake-ai/src/feasibility_probe.rs` `#[cfg(test)]` — 2 new unit tests for probe rejection with `Stale` vs `Certain`.

### Commands

1. `cargo test -p worldwake-ai motive_score -- --include-ignored=false` (targeted ranking).
2. `cargo test -p worldwake-ai plan_revalidation` (targeted revalidation).
3. `cargo test -p worldwake-ai feasibility_probe` (targeted probe).
4. `cargo test -p worldwake-ai` (full AI suite).
5. `cargo clippy --workspace --all-targets -- -D warnings`.
6. `./scripts/verify.sh` before PR.
