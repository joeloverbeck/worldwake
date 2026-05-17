# S151TESRELROU-011: Golden E2E coverage — testimony reliability + route preferences

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — new golden test scenarios only
**Deps**: archive/tickets/S151TESRELROU-001.md, archive/tickets/S151TESRELROU-002.md, archive/tickets/S151TESRELROU-003.md, archive/tickets/S151TESRELROU-004.md, archive/tickets/S151TESRELROU-005.md, archive/tickets/S151TESRELROU-006.md, archive/tickets/S151TESRELROU-007.md, archive/tickets/S151TESRELROU-008.md, archive/tickets/S151TESRELROU-009.md

## Problem

S151's D14 covers the end-to-end behavior of testimony reliability + route preferences through 7 golden scenarios (3 testimony + 4 route preference). These goldens are the canonical regression surface for Scenario G ("false rumor → wrongful accusation → correction" — see `docs/FOUNDATIONS.md:506-519`) substrate landing in S151 and for the route-preference learning loop. The mapping function unit tests live with the function in ticket 001 (not duplicated here).

## Assumption Reassessment (2026-05-17)

1. `docs/golden-e2e-testing.md` is the canonical golden authoring guide; `docs/generated/golden-e2e-inventory.md` and `docs/generated/golden-scenario-index.md` are the test-name and scenario inventories. Regenerate after this ticket lands via `python3 scripts/golden_inventory.py --write --check-docs`.
2. Live goal under test for the testimony goldens: `GoalKind::AskWitness { witness: EntityId, topic: TellTopic }` at `crates/worldwake-core/src/goal.rs:145-148`. Extractor: `extract_ask_witness_candidates` at `crates/worldwake-ai/src/candidate_generation.rs:2877-3045` (extended by ticket 007 with suppression). Ranking damping site: `apply_ask_witness_learned_damping` at `crates/worldwake-ai/src/ranking.rs:1494-1517` (extended by ticket 007).
3. Live cost function for route-preference goldens: `perceived_direct_travel_cost_from_memory` at `crates/worldwake-ai/src/route_threat.rs:187-212` (extended by ticket 008 with `RoutePreference` modifier).
4. Composition with S150 blockers (final route-preference scenario): `BlockerScope::RouteSegment` at `crates/worldwake-core/src/blocker_scope.rs` (S150 substrate) — blocker = hard suppression; preference = soft bias. The composition scenario asserts both behaviors fire simultaneously without one masking the other.
5. Per CLAUDE.md Authoritative-to-AI Impact Rule: ticket 007 modified candidate emission and ranking; ticket 008 modified cost computation. These goldens are the rule's checklist item #7 ("Golden tests pass") for the spec.
6. **Scenario isolation discipline (per `docs/precision-rules.md` Rule 8)**: each golden's setup must explicitly exclude unrelated lawful affordances that could mask the intended branch. For example, the "false accusation by repeated unreliable source" scenario must isolate the suppression-trigger path from competing AskWitness candidates whose ranking would otherwise dominate.

## Architecture Check

1. Per FND-31 (Validation and Falsification): each scenario asserts a specific behavioral invariant against a fixed seed; the assertion surface includes decision-trace + event-log + authoritative state per `docs/precision-rules.md` Rule 5.
2. Per FND-3: counters and derived views surface in golden assertions concretely (e.g., `direct_confirmations == 3` rather than "trust improved").
3. Per FND-26: golden scenarios exercise the end-to-end flow through state-mediated reads — observation hook (006) → runtime store update → ranking consumer (007) or cost consumer (008) → committed-goal or suppressed-goal outcome.
4. Composition test (RoutePreference + BlockerScope::RouteSegment) is the canonical FND-28 check that both surfaces coexist without entangling — blocker drives hard suppression; preference drives soft bias; neither replaces the other.

## Verification Layers

1. End-to-end behavioral outcome → golden test assertion against the event-log delta + final authoritative world state at fixed tick.
2. AI reasoning provenance → decision-trace assertions that the expected `CandidateDampingReason::TestimonySourceUnreliable` or `TestimonyOmissionReason::SourceUnreliable` appears (per Rule 5 layer mapping).
3. Per-tick determinism → byte-stable event log across two runs of the same scenario with the same seed.
4. Composition isolation (S150 blocker + S151 preference) → both surfaces fire independently; observer renders both.

## What to Change

### 1. Add `crates/worldwake-ai/tests/golden_testimony_reliability.rs` (new)

Three scenarios:

**Scenario A: Witness reports stale route hazard → refutation → damping**
- Setup: agent in town A; trusted-by-default witness reports "route A→B has bandits" (`PerceptionSource::Report { from: witness }`); agent travels A→B and observes no bandits.
- Expected: `TestimonyReliabilityEntry.direct_refutations == 1` for `(witness, RouteHazard)`; next `AskWitness` candidate from same source receives damping recorded as `CandidateDampingReason::TestimonySourceUnreliable`; decision trace assertion.

**Scenario B: Witness reports accurate threat → confirmation → preferred**
- Setup: agent and trusted witness; witness reports "threat at place P"; agent perceives the threat directly.
- Expected: `direct_confirmations == 1`; next `AskWitness` from same source ranks higher than baseline (no damping fires).

**Scenario C: False accusation by repeated unreliable source → suppression at threshold**
- Setup: agent receives N accusations (above `minimum_observations`) from the same source, all of which are subsequently refuted by direct observation.
- Expected: `TestimonyReliabilityEntry.direct_refutations >= minimum_observations`; trust falls below `trust_threshold * suppression_floor`; the next `AskWitness` candidate involving this source is suppressed at emission time, recorded as `TestimonyOmissionReason::SourceUnreliable` in `CandidateGenerationDiagnostics`; the corresponding `GoalSuppressedPayload.testimony_trust_context` is populated.

### 2. Add `crates/worldwake-ai/tests/golden_route_preferences.rs` (new)

Four scenarios:

**Scenario D: Safe traversal accumulation → preference rises → travel cost reduced**
- Setup: agent traverses route A→B safely 5 times.
- Expected: `RoutePreferenceEntry.safe_traversals == 5`; `preference > 500` (above neutral); `perceived_direct_travel_cost_from_memory` returns lower value than for an untracked route.

**Scenario E: Ambush during travel → preference falls → travel cost increased**
- Setup: agent traverses route A→B; an ambush event fires during the action's tick window (`EventTag::Combat` with agent as target).
- Expected: `dangerous_traversals == 1`; `last_dangerous_tick` and `last_traversal_event` populated; `perceived_direct_travel_cost_from_memory` returns higher value than for an untracked route.

**Scenario F: Decay after `days_to_decay_observations` → preference returns to neutral**
- Setup: agent traverses route A→B safely 5 times, then time advances past `days_to_decay_observations` without further traversals.
- Expected: `preference == 500` (neutral); travel cost matches the untracked-route baseline.

**Scenario G: `RoutePreference` + `BlockerScope::RouteSegment` (S150) compose**
- Setup: agent has both a positive preference for route A→B AND an active `BlockerScope::RouteSegment(A, B)` blocker (from S150).
- Expected: the blocker hard-suppresses any plan that would traverse A→B (no candidate emission for the blocked route), while the preference remains visible in inspection — the two surfaces don't entangle. Observer renders both. Decision trace shows the blocker as the suppression cause, not the preference.

### 3. Regenerate generated docs

After the goldens land, run `python3 scripts/golden_inventory.py --write --check-docs` to update `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and the per-scenario detail files under `docs/generated/golden-scenario-details/`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_testimony_reliability.rs` (new)
- `crates/worldwake-ai/tests/golden_route_preferences.rs` (new)
- `docs/generated/golden-e2e-inventory.md` (regenerate)
- `docs/generated/golden-scenario-index.md` (regenerate)
- `docs/generated/golden-scenario-details/` (regenerate per-file detail)

## Out of Scope

- Production code changes (all production support lands in tickets 001-009)
- The exhaustive `belief_topic_to_topic_scope` mapping unit tests (live with the function in ticket 001's `topic_scope.rs#[cfg(test)]`)
- `SAVE_FORMAT_VERSION`-related coverage (ticket 010)

## Acceptance Criteria

### Tests That Must Pass

1. All 7 golden scenarios above pass on the configured seeds.
2. Byte-stable event log across two consecutive runs of each scenario with the same seed.
3. Decision-trace assertions match the expected `TestimonyOmissionReason` / `CandidateDampingReason::TestimonySourceUnreliable` / route-cost-modifier reasoning at the documented decision points.
4. `python3 scripts/golden_inventory.py --check-docs` reports the new tests with no drift.
5. Existing suite: `cargo test --workspace`.

### Invariants

1. Per FND-31: every scenario has at least one decision-trace + event-log + final-state assertion (no scenario relies solely on a single layer).
2. Per Rule 8 (Scenario Isolation): each golden's setup explicitly excludes competing affordances that could mask the intended branch; the exclusions are documented inline in the scenario setup.
3. Byte-stable event-log replay across runs with fixed seed (Determinism).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_testimony_reliability.rs` — 3 scenarios (A, B, C).
2. `crates/worldwake-ai/tests/golden_route_preferences.rs` — 4 scenarios (D, E, F, G).

### Commands

1. `cargo test -p worldwake-ai --test golden_testimony_reliability`
2. `cargo test -p worldwake-ai --test golden_route_preferences`
3. `cargo test --workspace`
4. `python3 scripts/golden_inventory.py --write --check-docs`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `./scripts/verify.sh`
