# S153GOLDGAPSCALE-001: False-rumor justice golden + testimony-reliability helper

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None (substrate is archived: S151 testimony reliability, S139 AskWitness, S109 discrepancy, S136 decision payload)

## Problem

S153 D2 calls for a golden that proves S151's cross-source testimony contradiction with prior reliability state: an unreliable witness's accusation must be damped by concrete prior experience with that witness and contradicted by a reliable witness — without any authored "ignore W" rule and without omniscient truth. Existing testimony goldens covered single-source updates but not the cross-source contradiction increment against pre-seeded accusation unreliability. This ticket adds that regression inside the existing testimony-reliability golden owner, plus the shared `expect_testimony_reliability_update` harness helper (D5 slice), a deterministic replay companion (D6 slice), and its falsification comment (D7 slice).

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Substrate confirmed against current code: `TestimonyReliability` is keyed by `TestimonyReliabilityKey { source, topic }`; the per-key `TestimonyReliabilityEntry` carries `direct_refutations` and `contradicted_claims` (`crates/worldwake-core/src/testimony_reliability.rs`). Update methods `record_refutation` / `record_confirmation` / `record_contradiction` are called from `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs`. The trust threshold is `TestimonyTrustProfile.minimum_observations` plus `trust_threshold` profile math, not an entry counter.
2. Spec reference: `specs/S153-golden-gaps-ai-architecture-scaling.md` D2 originally named a new `false_rumor_justice.rs` module, but live golden ownership already exists at `crates/worldwake-ai/tests/scenarios/testimony_reliability.rs`. Reassessment narrowed this ticket to extend that existing owner rather than duplicate the S151 testimony suite.
3. Shared boundary under audit: `TestimonyReliability` authoritative core state and the `DecisionEventPayload::GoalSuppressed` testimony context. Live `ranking.rs` applies S151 testimony damping to `GoalKind::AskWitness`; it does not currently provide a separate `Accuse` ranking-damping surface. This ticket therefore proves the false-rumor justice substrate at the strongest existing lower layer: accusation-credibility reliability state, contradiction update, low-trust summary, and suppressed testimony decision payload context.
4. Live `GoalKind`s under test: `GoalKind::AskWitness` and `GoalKind::Accuse` as metadata coverage for the false-rumor justice claim family. The executable assertion is not a full autonomous magistrate accusation workflow; office-vacancy and scaled-contention remain sibling tickets.
5. AI-regression layer: golden helper-level coverage under the consolidated `golden_ai` harness, not full `agent_tick` with action registries. This is the honest current proof seam because the existing S151 testimony reliability goldens are direct payload/state regressions and already own the neighboring single-source cases.
6. Adjacent-contradiction classification: the drafted "Accuse candidate damped in ranking" wording was stale against live code. The completed scope records the narrowed proof and leaves production candidate/ranking changes out of scope.

## Architecture Check

1. Extending `testimony_reliability.rs` keeps the regression beside the existing S151 reliability owner and avoids a duplicate module for the same state/payload contract. The `expect_testimony_reliability_update` helper composes over authoritative `TestimonyReliabilityEntry` state — a thin test wrapper over runtime types, per the golden_harness convention.
2. No backward-compatibility shims: this is net-new test coverage; no production path is aliased, and the helper is new (no existing-helper override).

## Verified Layers

1. W has pre-seeded accusation unreliability (`direct_refutations == 2`) -> authoritative `TestimonyReliabilityEntry` state.
2. V contradicts W without inheriting W's negative history -> distinct `TestimonyReliabilityKey { source, topic }` entries.
3. W's `TestimonyReliabilityEntry.contradicted_claims` increments for the `(W, AccusationCredibility)` key -> authoritative core component state, asserted via `expect_testimony_reliability_update`.
4. W remains below the trust threshold after contradiction -> `TestimonyTrustSummary` derived from the entry and `TestimonyTrustProfile`.
5. The decision payload records the suppressed unreliable testimony context -> event-log payload (`DecisionEventPayload::GoalSuppressed` with `SuppressedByUnreliableTestimony`).
6. Determinism (D6): two identical helper-level runs produce equal before/after reliability entries, trust summary, payload, and corroborating-source absence result.

## Landed Changes

### 1. Extend `testimony_reliability.rs`

Scenario 443 in `crates/worldwake-ai/tests/scenarios/testimony_reliability.rs` now proves: Witness W has two prior accusation-credibility refutations, Witness V has no negative entry for the same topic, V's contradiction advances only W's `contradicted_claims`, W remains below the trust threshold, and the suppressed-goal decision payload carries W's low-trust summary.

### 2. New helper `expect_testimony_reliability_update`

`expect_testimony_reliability_update(source, topic, before, after, observation_event)` now lives in the golden harness and asserts a single reliability transition for one `(source, topic)` key plus retained provenance for the observation event that drove it.

### 3. Add the falsification comment

Scenario 443 carries a `// Falsification:` comment block describing the failed state: W remains above threshold or the contradiction counter does not advance after V contradicts the claim.

### 4. Determinism rerun (D6)

`golden_false_rumor_justice_contradiction_deterministic_replay` runs the helper-level chain twice with identical inputs and asserts equality of the before/after reliability entries, trust summary, suppressed payload, and corroborating-entry absence result.

### 5. Regenerate golden-inventory docs

`python3 scripts/golden_inventory.py --write --check-docs` regenerated the inventory, scenario index, scenario detail page, and coverage matrix.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/testimony_reliability.rs` (modify — add Scenario 443 tests)
- `crates/worldwake-ai/tests/golden_harness/testimony_assertions.rs` (new — `expect_testimony_reliability_update`)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — register/re-export the new helper module)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerate)
- `docs/generated/golden-scenario-index.md` (modify — regenerate)
- `docs/generated/golden-scenario-details/testimony-reliability.md` (modify — regenerate)
- `docs/generated/golden-coverage-matrix.md` (modify — regenerate)

## Out of Scope

- No production code changes (no engine, no action preconditions, no validation, no candidate-emission/ranking edits) — test + harness only.
- No committed RON scenario file.
- No new `false_rumor_justice.rs` module; the existing S151 testimony reliability suite is the owner.
- No full autonomous magistrate accusation workflow or production ranking changes.
- The office-vacancy (D3 → ticket 002) and scaled-contention (D4 → ticket 003) goldens.
- `expect_route_blocker_lifecycle` helper (ticket 003's D5 slice).

## Acceptance Criteria

### Tests Passed

1. `golden_false_rumor_justice_*` passes, asserting: pre-seeded `direct_refutations >= 2`; the corroborating witness does not inherit W's negative history; W's contradiction increments `contradicted_claims` via `expect_testimony_reliability_update`; W remains below trust threshold; `DecisionEventPayload::GoalSuppressed` carries the low-trust summary.
2. Determinism: two identical helper-level runs produce equal reliability entries, trust summary, suppressed payload, and corroborating-source absence result.
3. Existing suite passed: `cargo test -p worldwake-ai --test golden_ai`
4. Golden-inventory consistency passed: `python3 scripts/golden_inventory.py --write --check-docs`

### Invariants

1. False-rumor suppression remains grounded in source/topic testimony reliability state, never omniscient truth (FND-14, FND-15).
2. Testimony reliability changes are concrete per-`(source, topic)` state updates with an inspectable acquisition path (FND-22A).
3. Determinism: the helper-level chain is stable across identical inputs and ordered authoritative state.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/testimony_reliability.rs` — Scenario 443 proving cross-source testimony contradiction against pre-seeded reliability state, plus deterministic replay.
2. `crates/worldwake-ai/tests/golden_harness/testimony_assertions.rs` — new `expect_testimony_reliability_update` helper exercised by the golden.

### Commands Run

1. `cargo test -p worldwake-ai --test golden_ai golden_false_rumor_justice_contradiction`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-20.

- Added S153 false-rumor justice coverage as Scenario 443 in the existing `testimony_reliability.rs` golden owner.
- Added `golden_false_rumor_justice_contradiction_updates_unreliable_source` and `golden_false_rumor_justice_contradiction_deterministic_replay`.
- Added the shared golden helper `expect_testimony_reliability_update`.
- Regenerated golden inventory, scenario index, testimony-reliability detail page, and coverage matrix.
- Updated `specs/S153-golden-gaps-ai-architecture-scaling.md` so D2 describes the existing-module helper-level proof instead of a duplicate `false_rumor_justice.rs` module.
- Updated `specs/IMPLEMENTATION-ORDER.md` and then-pending siblings `archive/tickets/S153GOLDGAPSCALE-002.md` / `tickets/S153GOLDGAPSCALE-003.md` so active handoff prose no longer overstates false-rumor justice as end-to-end or cites stale `CLAUDE.md` guidance. `S153GOLDGAPSCALE-002` was later rejected and archived after live reassessment proved the office-vacancy golden needs a new substrate owner.

## Deviations

- The drafted new `false_rumor_justice.rs` module did not land. Live reassessment found `crates/worldwake-ai/tests/scenarios/testimony_reliability.rs` is the correct S151 owner for testimony reliability state and payload regressions.
- The drafted full autonomous magistrate accusation workflow did not land. Live ranking code applies testimony reliability damping to `AskWitness`, not a separate `Accuse` ranking path, so this ticket proved the strongest existing lower layer: source/topic reliability update, low-trust summary, and suppressed unreliable testimony payload context.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai golden_false_rumor_justice_contradiction`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai --test golden_ai`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
- Passed `./scripts/verify.sh`
- Passed `git diff --check` after final Markdown truth-sync edits
