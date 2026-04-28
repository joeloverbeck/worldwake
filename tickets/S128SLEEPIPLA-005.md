# S128SLEEPIPLA-005: Per-place sleep candidate emission and ranking

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — modifies `emit_sleep_goal` (`crates/worldwake-ai/src/candidate_generation.rs:3228`) to emit one `Sleep` candidate per believed sleep-eligible place; extends `motive_score` for `Sleep` in `crates/worldwake-ai/src/ranking.rs` to weigh `recovery_modifier` so high-quality places outrank low-quality ones.
**Deps**: archive/tickets/S128SLEEPIPLA-001.md, S128SLEEPIPLA-003

## Problem

Today, `emit_sleep_goal` (`crates/worldwake-ai/src/candidate_generation.rs:3228`) emits a single untargeted `Sleep` candidate based on fatigue + thresholds. Hillside Shelter "remained dormant" in the gameplay report (`reports/proposed-gameplay-mechanic-changes.md`) because there is no in-world reason for an agent to prefer it as a sleep site over Riverside Camp — sleep candidates do not carry place-quality information, and ranking does not weigh place. This ticket changes sleep emission to one candidate per believed sleep-eligible place and extends `motive_score` for `Sleep` to weigh `recovery_modifier`, so a well-authored shelter ranks above an open-air orchard at the same fatigue level.

## Assumption Reassessment (2026-04-27)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing focused/unit coverage at `crates/worldwake-ai/src/candidate_generation.rs::fatigue_and_bladder_emit_sleep_and_relieve` (line 9558) asserts that `emit_sleep_goal` emits one Sleep candidate when fatigue is high. After this ticket, the test asserts: one Sleep candidate per believed reachable place, each carrying the place's `SleepQualityProfile`. Per `docs/precision-rules.md` Rule 3, restate the intended invariant in the test rather than adapting it cosmetically.
2. `emit_sleep_goal` (`candidate_generation.rs:3228`) currently constructs one `GroundedGoal { kind: GoalKind::Sleep, ... }` per call without a place anchor. The spec D8 changes this to: enumerate believed sleep-eligible places (alive entities of `EntityKind::Place`, in graph, reachable per existing reachability logic the candidate emitter uses for other place-targeted goals), and emit one candidate per place, anchoring the place via the `OpportunityAnchor` site reference. The emitter reads each place's `SleepQualityProfile` via `GoalBeliefView::place_sleep_quality_profile` (added in S128SLEEPIPLA-003).
3. Shared boundary under audit: the candidate-emission contract for `Sleep` and the ranking contract for `Sleep`'s `motive_score`. Today, sleep is GoalKind-only with no per-anchor variation. After this ticket, sleep's `OpportunityAnchor` carries a place id; `motive_score` reads that place's belief-known `SleepQualityProfile.recovery_modifier` and incorporates it as a multiplier on the existing motive computation. The ranking change is additive — the existing fatigue-driven motive baseline is preserved; `recovery_modifier` is a tie-breaker / amplifier.
4. `GoalBeliefView::place_sleep_quality_profile(place: EntityId) -> SleepQualityProfile` (lands in S128SLEEPIPLA-003) is the canonical AI-layer accessor. The emitter uses this; the action handler in S128SLEEPIPLA-004 reads the authoritative profile at action start (which is FND-14-correct because action handlers execute against world state). No FND-14 violation here: the emitter reads belief; the handler reads world.
5. Ranking arithmetic check (Rule 4 / Rule 14): the current `motive_score` for `Sleep` is computed in `crates/worldwake-ai/src/ranking.rs` (locate via `match goal.kind { GoalKind::Sleep => ... }` grep). The extension multiplies the base motive by `recovery_modifier / 1000` (pure scalar, no float — implemented as `motive × recovery_modifier.value() / 1000`). At default `recovery_modifier: 1000`, motive is unchanged. At Hillside Shelter's `1300`, motive is amplified by 30%. At Fertile Fields' `900`, motive is reduced by 10%. The `motive_score` type's units (`Permille` or `u32`) constrains the arithmetic — confirm during reassessment and use saturating multiplication if needed.
6. Reachability: per other place-targeted candidate emitters (e.g., for harvest goals), the emitter iterates places the agent has belief of and filters by reachability. Reuse the same predicate. The agent's current place is always a valid candidate (recovery still happens at the current spot if no better site is known). If the agent has no belief of any other place, the only emitted candidate is the current place — same as today's effective behavior, just expressed as a per-place candidate.
7. Behavioral guarantee (Rule 14 timing vs semantics): the change is semantic, not timing. The same fatigue level still triggers sleep at the same tick. What changes is which place the planner adopts: when fatigue rises, the highest-`motive_score` candidate wins, and `recovery_modifier` is part of that score.
8. Adjacent contradiction check: `crates/worldwake-ai/src/feasibility.rs::test_sleep_always_likely` (line 689) — sleep remains always likely (fatigue-driven, not place-quality-gated). Per-place emission produces multiple candidates but each individual candidate is feasible. The test should remain green.

## Architecture Check

1. Per-place emission honors FND-7 (locality): only places the agent has belief of are eligible. Unknown places are filtered out implicitly because the belief-view accessor returns `SleepQualityProfile::default()` for unknown places (per S128SLEEPIPLA-003), but the emitter filters by belief presence before calling the accessor — no candidate emitted for never-observed places.
2. `motive_score` extension is a multiplier on the base motive, not a replacement — preserves the existing fatigue-driven baseline (drive escalation, urgency, etc.) and adds place-quality as a per-anchor differentiator. Per the candidate-scoring architecture pattern (`references/worldwake-validation-patterns.md`), gating logic stays in the emitter (every reachable place still emits a candidate); ranking decides relative priority. This split is preserved.
3. The change leaves D8's "make Hillside Shelter preferred" headline goal achievable without changing the action lifecycle (S128SLEEPIPLA-004 owns that). At equal fatigue, an agent with belief of Hillside Shelter (`1300`) and Riverside Camp (`1100`) will adopt Sleep anchored at Hillside Shelter — provided travel cost doesn't dominate (existing motive-vs-travel-cost tradeoffs apply per other place-targeted goals).

## Verification Layers

1. Per-place candidate count and identity → focused unit tests in `candidate_generation.rs` test module asserting that with N believed sleep-eligible places, N Sleep candidates are emitted, each with a distinct place anchor.
2. `motive_score` for `Sleep` weighs `recovery_modifier` → focused unit test in `ranking.rs` test module: two candidates with identical fatigue/agent state but different `recovery_modifier` produce ordered scores (higher `recovery_modifier` → higher score).
3. End-to-end site preference: an agent with belief of two reachable sleep sites adopts the higher-quality one → focused integration test in `agent_tick`-level test suite, OR deferred to S128SLEEPIPLA-007's golden test 5. Pick the focused-runtime route here to keep the proof close to the change; the golden test confirms the same property at the scenario level.
4. Decision-trace records the candidate's `recovery_modifier` (in evidence summary) → existing decision-trace infrastructure picks this up automatically when the candidate's evidence summary is populated; verify via decision-trace assertion rather than weakening to event-log indirection (per Rule 6).
5. Layer separation: candidate emission and ranking both live in `worldwake-ai`; reads place sleep quality through `GoalBeliefView` (sim-layer trait); no read of authoritative world state from AI for this concern.

## What to Change

### 1. Refactor `emit_sleep_goal` in `crates/worldwake-ai/src/candidate_generation.rs`

Replace the single-candidate emission with a per-place loop:

- Determine sleep-eligibility threshold (existing fatigue gate). If not eligible, no candidates emitted (existing behavior).
- Enumerate believed places: iterate the agent's belief store for `EntityKind::Place` entries (or use whatever existing helper sibling place-targeted emitters use — `enumerate_*_places` or similar; grep `crates/worldwake-ai/src/candidate_generation.rs` for the prior art).
- For each believed place that is reachable (reuse the existing reachability predicate), call `ctx.belief_view.place_sleep_quality_profile(place)` and emit a `Sleep` candidate with:
  - `kind: GoalKind::Sleep`
  - `OpportunityAnchor` site reference set to the place
  - Evidence summary including the place's `recovery_modifier` so ranking and trace can read it
- The emit-trace mechanism (`emit_candidate_with_trace` per the candidate-scoring architecture pattern) remains the entry point. `Sleep` retains its existing `OpportunityAnchor` shape — confirm during reassessment whether `Sleep`'s `OpportunityAnchor` already supports a place reference; if not, extend the relevant `OpportunityAnchor` variant (or add a new variant) as part of this ticket.

### 2. Extend `motive_score` for `Sleep` in `crates/worldwake-ai/src/ranking.rs`

Locate the `Sleep` arm in `motive_score` (grep `GoalKind::Sleep` in `ranking.rs`). Multiply the existing computation by `recovery_modifier.value() / 1000` (saturating arithmetic; `motive_score` likely returns `u32` or `Permille` — match the existing return type). At `recovery_modifier == 1000`, motive is unchanged; at `recovery_modifier == 1300`, motive is amplified ~30%; at `recovery_modifier == 900`, motive is reduced ~10%. Document the formula in code with one short comment naming the spec's intent (FND-22 diversity through place-quality).

### 3. Update `fatigue_and_bladder_emit_sleep_and_relieve` test

The existing test (`candidate_generation.rs:9558`) asserts one Sleep candidate is emitted at high fatigue. Reframe to assert: with N believed reachable places, N Sleep candidates are emitted, each carrying the corresponding place's `SleepQualityProfile`. Add a sibling test asserting that with no believed reachable places (only the current place known), exactly one Sleep candidate is emitted (anchored at the current place). Add a third test asserting that ranking orders two candidates by `recovery_modifier` when all other inputs are equal.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — refactor `emit_sleep_goal`, update `fatigue_and_bladder_emit_sleep_and_relieve`, add sibling tests)
- `crates/worldwake-ai/src/ranking.rs` (modify — extend `motive_score` Sleep arm)
- `Likely: crates/worldwake-ai/src/goal_model.rs` or `crates/worldwake-core/src/goal.rs` (modify — if `Sleep`'s `OpportunityAnchor` does not currently support a place reference, extend it; grep `GoalKind::Sleep` and the `OpportunityAnchor` enum to confirm during reassessment)

## Out of Scope

- Sleep action handler refactor (consuming the per-candidate place anchor) — handled by S128SLEEPIPLA-004
- `GoalBeliefView::place_sleep_quality_profile` accessor — handled by S128SLEEPIPLA-003
- Scenario authoring of per-place `SleepQualityProfile` — handled by S128SLEEPIPLA-006
- Golden tests for site preference — handled by S128SLEEPIPLA-007 (test 5)
- New `WakeCondition::PlaceNoLongerSafe` ranking penalty — out of scope per spec Non-Goals (S60 deferral)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai fatigue_and_bladder_emit_sleep_and_relieve` (reframed) — N believed reachable places → N Sleep candidates with distinct anchors.
2. `cargo test -p worldwake-ai sleep_candidate_emission_at_current_place_only` (new) — agent with belief of only the current place → exactly one Sleep candidate at that place.
3. `cargo test -p worldwake-ai sleep_motive_orders_by_recovery_modifier` (new) — two candidates with identical fatigue/state, different `recovery_modifier` → higher-modifier candidate has higher `motive_score`.
4. `cargo test -p worldwake-ai test_sleep_always_likely` (existing, `feasibility.rs:689`) — feasibility unchanged.
5. `cargo test -p worldwake-ai` — full AI suite.
6. Existing suite: `cargo test --workspace`.

### Invariants

1. Sleep candidates emitted = count of believed reachable places passing the eligibility gate (lower bound: 1, the current place; upper bound: total believed reachable places).
2. Each Sleep candidate's `OpportunityAnchor` references a distinct place; no duplicate place anchors.
3. `motive_score(Sleep, place)` is monotonic in that place's `recovery_modifier` (all else equal): higher `recovery_modifier` → higher score.
4. At `recovery_modifier == Permille::new_unchecked(1000)`, `motive_score(Sleep, place)` equals the pre-ticket baseline (no perturbation at default).
5. Reads of place sleep quality go through `GoalBeliefView::place_sleep_quality_profile`; no direct world-state read from the AI candidate emitter (FND-14).
6. Adopted candidate's anchored place is the place the action handler will write `SleepEpisode.place` to (handshake with S128SLEEPIPLA-004 — confirm via decision trace + action trace).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (modify — reframe `fatigue_and_bladder_emit_sleep_and_relieve`; add `sleep_candidate_emission_at_current_place_only`).
2. `crates/worldwake-ai/src/ranking.rs` (modify — add `sleep_motive_orders_by_recovery_modifier`).
3. `crates/worldwake-ai/src/agent_tick/` test module (consider adding) — focused integration test asserting decision-trace records the adopted candidate's place anchor and `recovery_modifier`. Defer to S128SLEEPIPLA-007 if the existing harness makes scenario-level coverage easier — note the choice in the implementation summary.

### Commands

1. `cargo test -p worldwake-ai candidate_generation ranking`
2. `cargo test -p worldwake-ai feasibility` (existing test must still pass)
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh`
