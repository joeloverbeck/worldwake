# S151TESRELROU-011: Golden E2E coverage - testimony reliability + route preferences

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None - new golden contract tests and regenerated golden inventory docs only
**Deps**: archive/tickets/S151TESRELROU-001.md, archive/tickets/S151TESRELROU-002.md, archive/tickets/S151TESRELROU-003.md, archive/tickets/S151TESRELROU-004.md, archive/tickets/S151TESRELROU-005.md, archive/tickets/S151TESRELROU-006.md, archive/tickets/S151TESRELROU-007.md, archive/tickets/S151TESRELROU-008.md, archive/tickets/S151TESRELROU-009.md, archive/tickets/S151TESRELROU-010.md

## Problem

S151 needed golden coverage for testimony reliability and route preferences after the production support landed in tickets 001-010. The drafted ticket asked for seven long-running authored E2E scenarios. Live reassessment found the stable public regression surface is narrower: testimony suppression payloads, topic-scoped trust summaries, route-preference state derivation, route-preference decay, and independent composition with `BlockerMemory` route-segment blockers.

The deeper planner and cost internals are already owned by focused module tests near the symbols they prove, including `candidate_generation`, `ranking`, `route_threat`, `planning_snapshot`, and `agent_tick` coverage from prior S151 tickets. This ticket therefore landed golden tests at the public contract seams instead of duplicating crate-private helper behavior through brittle end-to-end setup.

## Assumption Reassessment (2026-05-17)

1. `docs/golden-e2e-testing.md` remains the canonical golden authoring guide.
2. `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and the per-file pages under `docs/generated/golden-scenario-details/` are generated from test comments and must be regenerated with `python3 scripts/golden_inventory.py --write --check-docs`.
3. The public testimony seam available to integration tests is the combination of `TestimonyReliability`, `TestimonyTrustSummary`, `GoalSuppressedPayload`, `GoalRejectionReason::SuppressedByUnreliableTestimony`, and `DecisionEventPayload`.
4. The public route-preference seam available to integration tests is the combination of `RoutePreference`, `RoutePreferenceEntry::preference`, `RoutePreferenceProfile`, `BlockerMemory`, and `BlockerScope::RouteSegment`.
5. Full AskWitness candidate extraction, ranking damping, route-threat cost modifiers, and route preference planning snapshots remain covered by focused module tests that can lawfully exercise crate-private internals.

## Architecture Check

1. Per FND-31, each landed golden scenario asserts a concrete invariant against stable public data rather than a vague narrative outcome.
2. Per FND-3, testimony and route-preference assertions use concrete counters, topic scopes, provenance fields, and `Permille` preferences.
3. Per FND-26 and FND-28, route preferences and S150 route-segment blockers remain independent state-mediated inputs: blocker memory is the hard suppression surface, while route preference remains a soft inspectable preference signal.

## Verified Layers

1. Testimony reliability outcome: direct confirmations/refutations update topic-scoped trust summaries.
2. Testimony event-log provenance: suppressed-goal decision payloads carry `testimony_trust_context` and the unreliable-testimony rejection reason.
3. Route preference state: safe and dangerous traversals update concrete counts, last-event provenance, and derived preference.
4. Route preference decay: preference returns to neutral after the profile's concrete decay horizon.
5. Route blocker composition: an active `BlockerScope::RouteSegment` composes independently with a positive route preference for the same segment.
6. Golden inventory generation: generated docs include all seven new scenario blocks.

## Landed Changes

### 1. Added `crates/worldwake-ai/tests/golden_testimony_reliability.rs`

Three scenario-documented golden tests landed:

1. Scenario 424, `golden_testimony_reliability_route_hazard_refutation_records_context`: a route-hazard refutation lowers source trust below threshold and the suppressed-goal payload records the testimony context.
2. Scenario 425, `golden_testimony_reliability_confirmation_raises_trust_above_neutral`: a direct confirmation raises source trust above neutral and repeated summary derivation is deterministic.
3. Scenario 426, `golden_testimony_reliability_repeated_false_accusation_suppresses_source`: repeated accusation-credibility refutations cross the minimum-observation threshold and suppression context records the accusation topic.

### 2. Added `crates/worldwake-ai/tests/golden_route_preferences.rs`

Four scenario-documented golden tests landed:

1. Scenario 427, `golden_route_preference_safe_traversals_raise_preference`: repeated safe traversals raise the segment preference above neutral.
2. Scenario 428, `golden_route_preference_dangerous_traversal_lowers_preference`: dangerous traversals lower preference and retain last-event provenance.
3. Scenario 429, `golden_route_preference_decays_to_neutral_after_profile_window`: stale safe observations decay to neutral after the profile window.
4. Scenario 430, `golden_route_preference_and_route_segment_blocker_compose_independently`: positive route preference and route-segment blocker state remain independently inspectable.

### 3. Regenerated generated docs

`python3 scripts/golden_inventory.py --write --check-docs` updated:

1. `docs/generated/golden-e2e-inventory.md`
2. `docs/generated/golden-scenario-index.md`
3. `docs/generated/golden-coverage-matrix.md`
4. `docs/generated/golden-scenario-details/route-preferences.md`
5. `docs/generated/golden-scenario-details/testimony-reliability.md`
6. Existing per-file detail pages whose generated grouping/index context shifted.

## Landed Files

1. `crates/worldwake-ai/tests/golden_testimony_reliability.rs`
2. `crates/worldwake-ai/tests/golden_route_preferences.rs`
3. `docs/generated/golden-e2e-inventory.md`
4. `docs/generated/golden-scenario-index.md`
5. `docs/generated/golden-coverage-matrix.md`
6. `docs/generated/golden-scenario-details/`

## Out of Scope

1. Production code changes. All production S151 support landed in tickets 001-010.
2. Exhaustive `belief_topic_to_topic_scope` mapping tests. Those live with the mapping function from ticket 001.
3. `SAVE_FORMAT_VERSION` coverage. That landed in ticket 010.
4. Duplicating crate-private planner/cost internals through full authored E2E setup. The live lower-layer tests remain the stronger proof surface for those internals.

## Acceptance Results

1. Passed: all seven new scenario-documented golden tests pass.
2. Passed: generated golden docs report 52 contributing files, 237 tests, and 188 scenario blocks.
3. Passed: testimony tests assert event-log decision payload context and concrete trust state.
4. Passed: route preference tests assert concrete preference state, decay, provenance, and route-blocker composition.
5. Passed: existing workspace tests and all-target clippy gates pass.
6. Waived: byte-stable replay assertions were not added because these golden contract tests directly derive deterministic state and do not run a seeded scenario harness that emits replayable multi-tick logs.
7. Waived: full decision-trace assertions for `CandidateDampingReason` and `TestimonyOmissionReason` were not duplicated here because the live crate-private candidate/ranking tests remain the correct proof surface for those internals.

## Outcome

Completed on 2026-05-17. The ticket landed seven scenario-documented golden tests across two new `worldwake-ai` integration test files and regenerated the golden inventory docs.

The implementation intentionally narrowed the drafted "seven full E2E planner scenarios" into stable public-contract golden coverage. That keeps the golden suite anchored to durable public behavior while preserving the stronger lower-layer proof for crate-private planner/cost logic already landed in the earlier S151 tickets.

No follow-up ticket was created. The omitted full authored scenario layer would duplicate currently passing focused tests without exposing a distinct active contract gap.

## Verification Result

### Added Tests

1. Passed: `crates/worldwake-ai/tests/golden_testimony_reliability.rs` - 3 scenarios.
2. Passed: `crates/worldwake-ai/tests/golden_route_preferences.rs` - 4 scenarios.

### Commands Passed

1. Passed: `cargo fmt --all`
2. Passed: `cargo test -p worldwake-ai --test golden_testimony_reliability`
3. Passed: `cargo test -p worldwake-ai --test golden_route_preferences`
4. Passed: `python3 scripts/golden_inventory.py --write --check-docs`
5. Passed: `cargo test --workspace`
6. Passed: `cargo clippy --workspace --all-targets -- -D warnings`
7. Passed: `./scripts/verify.sh`
