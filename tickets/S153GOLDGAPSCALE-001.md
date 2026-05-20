# S153GOLDGAPSCALE-001: False-rumor justice golden + testimony-reliability helper

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: None (substrate is archived: S151 testimony reliability, S139 AskWitness, S109 discrepancy, S136 decision payload)

## Problem

S153 D2 calls for a golden that proves S151's cross-source testimony contradiction with prior reliability state: an unreliable witness's accusation must be damped by the magistrate's concrete prior experience with that witness and contradicted by a reliable witness — without any authored "ignore W" rule and without omniscient truth. No golden currently exercises *cross-source contradiction against pre-seeded reliability state*; existing testimony goldens cover single-source updates. This ticket adds that regression plus the shared `expect_testimony_reliability_update` harness helper (D5 slice), the scenario's determinism rerun (D6 slice), and its falsification comment (D7 slice).

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Substrate confirmed against current code: `TestimonyReliability` is keyed by `TestimonyReliabilityKey { source, topic }`; the per-key `TestimonyReliabilityEntry` carries `direct_refutations` and `contradicted_claims` (`crates/worldwake-core/src/testimony_reliability.rs`). Update methods `record_refutation` / `record_confirmation` / `record_contradiction` (`testimony_reliability.rs:84-116`) are called from `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs:103-125`. The trust threshold is `TestimonyTrustProfile.minimum_observations` (a profile field, **not** an entry counter). `GoalKind::AskWitness { witness, topic }` (`crates/worldwake-core/src/goal.rs:145`) and `GoalKind::Accuse { crime_register, accused, violation_id }` (`goal.rs:172`) both exist as GoalKind + action. `Discrepancy::BeliefContradicted` is a unit variant (`crates/worldwake-core/src/discrepancy.rs`). The decision payload is `DecisionEventPayload` (S136). No existing `golden_false_rumor_justice*` test exists (checked `crates/worldwake-ai/tests/scenarios/`).
2. Spec reference: `specs/S153-golden-gaps-ai-architecture-scaling.md` D2 (post-reassessment form — target module `crates/worldwake-ai/tests/scenarios/false_rumor_justice.rs`, registered in `tests/scenarios/mod.rs`, run via the `golden_ai` harness).
3. Shared boundary under audit: pre-seeded `TestimonyReliability` *authoritative core state* is read by AI-layer ranking (`crates/worldwake-ai/src/ranking.rs`) to damp the `Accuse` candidate and by candidate generation to emit `AskWitness`. The golden audits the AI ranking/candidate layer reacting to authoritative reliability state — it does not modify either layer.
4. Live `GoalKind`s under test: `GoalKind::AskWitness` and `GoalKind::Accuse`. The spec flags the `AskWitness` → `SlotKind::SocialMotive` slot mapping as to-be-confirmed: during implementation, confirm the emission slot in `crates/worldwake-ai/src/agent_tick/portfolio.rs` and the `Accuse` ranking-damping path in `ranking.rs` before asserting on them. If the live emission slot or damping surface differs from the narrative, correct ticket scope here first (precision rule 13).
5. AI-regression layer: golden E2E exercising full `agent_tick` with full action registries (the chain spans testimony perception → candidate generation → ranking → decision payload), not a needs-only harness.
6. Adjacent-contradiction classification: if reassessment finds the `Accuse` candidate is suppressed at *generation* (gate) rather than damped at *ranking* (ordering), that is a phase distinction (precision rule 1), not a defect — assert at the actual phase; do not weaken the contract to "no Accuse appears."

## Architecture Check

1. Inline-fixture construction (the landed `belief_wall_trap.rs` precedent) keeps the regression self-contained and avoids introducing a non-existent `scenarios/golden-*.ron` convention; RON backing is optional and out of scope. The `expect_testimony_reliability_update` helper composes over authoritative `TestimonyReliabilityEntry` state — a thin test wrapper over runtime types, per the golden_harness convention.
2. No backward-compatibility shims: this is net-new test coverage; no production path is aliased, and the helper is new (no existing-helper override).

## Verification Layers

1. M emits no committed `Accuse` despite receiving W's claim -> decision trace (`Accuse` candidate present but damped / not selected) AND action trace (no `Accuse` action committed) AND authoritative world state (no accusation artifact created).
2. M emits and selects an `AskWitness` candidate (corroboration-seeking) -> decision trace.
3. V's contradicting testimony surfaces a belief contradiction -> `Discrepancy::BeliefContradicted` in the decision/belief layer.
4. W's `TestimonyReliabilityEntry.contradicted_claims` increments for the `(W, topic)` key -> authoritative core component state, asserted via the new `expect_testimony_reliability_update` helper.
5. The decision payload records the candidate comparison -> event-log delta (`DecisionEventPayload`, S136).
6. Determinism (D6): two runs at the same seed produce a byte-identical event log AND an equal `ScenarioDiagnosticsReport` -> event-log byte comparison + report `Eq` comparison.

## What to Change

### 1. New golden module `false_rumor_justice.rs`

Build an inline fixture with three agents: Witness W (unreliable — pre-seed `TestimonyReliabilityEntry` for the `(W, topic)` key with `direct_refutations >= 2`), Witness V (reliable; co-located with the alleged event and saw nothing), Magistrate M. W tells M that agent A stole from a stash; A is innocent. Run ticks and assert the seven-step chain (D2 assertions 1–7): pre-seeded refutations; W's claim enters M's store at low confidence (computed trust below `TestimonyTrustProfile.minimum_observations`); `Accuse(A)` candidate damped; `AskWitness(V)` emitted/selected; V's contradiction → `Discrepancy::BeliefContradicted`; no `Accuse` committed; W's `contradicted_claims` increments.

### 2. New helper `expect_testimony_reliability_update`

Add `expect_testimony_reliability_update(source, topic, before, after, observation_event)` to the golden harness — asserts a single reliability transition for one `(source, topic)` key (before/after counter values) and that the observation event that drove it is present in the log.

### 3. Register the module and add the falsification comment

Add `pub mod false_rumor_justice;` to `tests/scenarios/mod.rs`. Add a `// Falsification:` comment block (D7): e.g., "If M commits `Accuse(A)` despite W's low reliability and V's contradicting testimony, the S151 reliability damping failed."

### 4. Determinism rerun (D6)

Run the scenario twice at the same seed; assert byte-identical event log and equal `ScenarioDiagnosticsReport` (model after `expect_deterministic_reports` in `golden_harness/survival_forensics_assertions.rs`, adapted to `ScenarioDiagnosticsReport`).

### 5. Regenerate golden-inventory docs

After the module compiles and is registered, run `python3 scripts/golden_inventory.py --write --check-docs` and commit the regenerated inventory.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/false_rumor_justice.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify — add `pub mod false_rumor_justice;`)
- `crates/worldwake-ai/tests/golden_harness/testimony_assertions.rs` (new — `expect_testimony_reliability_update`)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — register/re-export the new helper module)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerate)
- `docs/generated/golden-scenario-index.md` (modify — regenerate)
- `To be confirmed:` `docs/generated/golden-scenario-details/<false_rumor_justice>.md` (regenerate output path created by `scripts/golden_inventory.py`; confirm exact filename after running the generator)

## Out of Scope

- No production code changes (no engine, no action preconditions, no validation, no candidate-emission/ranking edits) — test + harness only.
- No committed RON scenario file (inline fixture per the `belief_wall_trap.rs` precedent); RON backing is optional and not required.
- The office-vacancy (D3 → ticket 002) and scaled-contention (D4 → ticket 003) goldens.
- `expect_route_blocker_lifecycle` helper (ticket 003's D5 slice).

## Acceptance Criteria

### Tests That Must Pass

1. `golden_false_rumor_justice_*` passes, asserting: pre-seeded `direct_refutations >= 2`; W's claim enters M's store at low confidence; `Accuse(A)` damped in ranking; `AskWitness(V)` emitted and selected; V's contradiction → `Discrepancy::BeliefContradicted`; **no** `Accuse` committed; W's `contradicted_claims` increments (via `expect_testimony_reliability_update`).
2. Determinism: two same-seed runs produce a byte-identical event log and an equal `ScenarioDiagnosticsReport`.
3. Existing suite: `cargo test -p worldwake-ai --test golden_ai`
4. Golden-inventory consistency: `python3 scripts/golden_inventory.py --check-docs`

### Invariants

1. M never commits `Accuse` without sufficient corroboration — decisions follow belief + concrete reliability state, never omniscient truth (FND-14, FND-15).
2. Testimony reliability changes are concrete per-`(source, topic)` state updates with an inspectable acquisition path (FND-22A).
3. Determinism: byte-stable replay under `ChaCha8Rng` + `BTreeMap`-ordered authoritative state (CLAUDE.md Critical Invariants).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/false_rumor_justice.rs` — new golden proving cross-source testimony contradiction against pre-seeded reliability state.
2. `crates/worldwake-ai/tests/golden_harness/testimony_assertions.rs` — new `expect_testimony_reliability_update` helper exercised by the golden.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai false_rumor_justice`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `scripts/verify.sh`
