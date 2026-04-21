# S113BELENV-005: Golden — stale-belief surfacing

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (test-only; exercises existing engine behavior from T001)
**Deps**: archive/tickets/S113BELENV-001.md, archive/tickets/S113BELENV-003.md

## Problem

The envelope lands as infrastructure (`S113BELENV-001`) and the stale/contradicted reasoning consumers land in downstream AI tickets, but the cross-system story — "an agent acquires a belief, time passes, and the envelope surfaces that belief as stale without any refresh" — is not yet covered by a golden E2E test. Spec S113 Validation #14 calls for an extension of an existing rumor-driven scenario (if present) or the `survival-scattered.ron`-based golden to assert the envelope surfaces stale beliefs after sufficient staleness decay (ticks elapsed such that `effective_claim_confidence` drops below the agent's `claim_confidence_threshold`) without any perception update.

This ticket adds one golden assertion on an existing scenario rather than authoring a new scenario, per the spec's own framing ("extend an existing ... scenario"). The intended invariant: a belief formed at tick `T` with fresh confidence decays through the `Certain → Probable → Stale` band boundaries on its natural schedule, and the scenario-backed golden proves the envelope surfacing at the `Stale` boundary through the live `PerAgentBeliefView` seam.

## Assumption Reassessment (2026-04-21)

1. `scenarios/survival-scattered.ron` exists per prior reassessment (spot-check confirmed during `/reassess-spec`). The corresponding golden lives in `crates/worldwake-ai/tests/golden_*.rs` — implementer greps `docs/generated/golden-e2e-inventory.md` to find the exact test name, per `tickets/README.md` guidance on the canonical golden inventory.
2. No rumor-driven scenario exists today with the exact shape S113 imagined; `survival-scattered` is the fallback the spec authorizes. Shared abstraction boundary under audit: the test harness's interaction between `scenarios/*.ron` and the golden runner. Intended invariant restatement: *the envelope surfaces a `Stale` status once effective confidence has decayed below `claim_confidence_threshold`, using the live `PerAgentBeliefView::believed_target_location(...)` seam over a scenario-backed belief fixture.*
3. The golden must not rely on wall-clock time, floats, or HashMap iteration order (CLAUDE.md Determinism). The decay math is deterministic: `effective = confidence - staleness_penalty_per_tick * ticks_elapsed`. With default `confidence_policy.staleness_penalty_per_tick = Permille(12)` and `claim_confidence_threshold = Permille(50)`, a fresh direct observation (Permille(950)) crosses into `Stale` at tick `ceil((950 - 50) / 12) = 75` — but this is only the default; the scenario's agent may override these.
4. Intended invariant: at the assertion tick, the agent's envelope read of a target location formed at tick `0` returns `status: Stale`, with confidence below the acting agent's `claim_confidence_threshold`, and no simulation step refreshes that belief before the read.
5. This is a golden / E2E ticket, but the strongest honest seam in `survival-scattered` is the envelope read itself. The current live ranking scale-down from `S113BELENV-003` only reaches specific goal families (`RaidTarget` target-location confidence), and `survival-scattered` does not exercise that seam lawfully.
6. AI regression type — intended layer is golden E2E coverage. Local needs-only harness is **not** sufficient here because the assertion depends on the full ranking substrate; full action registries are required. State harness boundary explicitly in the test.
7. No ordering-sensitive claim in the golden — the proof is on envelope state (`status`) and motive-score magnitude at a specific tick, not on cross-agent interaction order. If a comparative assertion is used (fresh vs stale agent), strict tick separation is not required; same-tick comparison is sufficient.
12. The scenario must be explicit about which lawful competing affordances were excluded from setup. If extending `survival-scattered`, document in the scenario's comment block that the envelope assertion is the intended branch and that any additional need-driven distractions are intentionally absent (or already handled by the scenario's existing isolation choices).
13. Resolved reassessment finding: `survival-scattered` does not exercise the already-landed `S113BELENV-003` `RaidTarget` scaling seam, so this ticket is narrowed to *envelope surfacing* only. It asserts `status: Stale` through the live belief-view surface and does not claim a motive-score comparison.

## Architecture Check

1. The golden extends an existing scenario rather than authoring a new one, minimizing engine-unrelated churn (test-only ticket).
2. The assertion reads `status` and confidence through the normal `PerAgentBeliefView` surface over the scenario-backed world + belief store — no bespoke helper that bypasses the belief-vs-world separation.
3. Decay math is deterministic per CLAUDE.md Determinism invariants — `ChaCha8Rng`-seeded setup, no floats, no wall-clock time, `BTreeMap`-based iteration.

## Verification Layers

1. Envelope surfacing — at the assertion tick, `believed_target_location` for the test agent's target returns `status: Stale` and confidence below `claim_confidence_threshold` → belief-view inspection via the test harness's live `PerAgentBeliefView` surface.
2. Fresh baseline — at the acquisition tick, the same seeded target-location belief still reads `status: Certain`, proving the test is observing decay rather than seeding a pre-stale fixture.
3. No event-log-delta assertion is required here — the golden is proving a reasoning-layer derived read, not authoritative world mutation.
4. This is a cross-system golden (scenario fixture + belief decay + envelope projection). Per validation-surface mapping: reasoning-layer invariant → strongest live belief-view seam. Decision trace is not required because no candidate-selection or action-lifecycle claim is under test.

## What to Change

### 1. Identify the target golden

Use `docs/generated/golden-e2e-inventory.md` to locate the golden that loads `scenarios/survival-scattered.ron` (or the closest extant rumor-driven scenario if one has been added since the spec was written). Record its path under `crates/worldwake-ai/tests/` for the Files to Touch list.

### 2. Extend the golden with a belief-fixture setup

If the existing scenario's initial conditions do not seed a belief that will naturally decay during the scenario's tick window, extend the golden's setup to include one. Options (implementer picks during writing):

- Seed a target-location belief at tick 0 via a scripted pre-run observation followed by a long gap.
- Use an existing scenario feature (rumor injection, report delivery) if available.
- Add a second agent whose belief freshness differs for comparative assertion.

Document the isolation choice in the golden's scenario comment block: this proof reads the same seeded scenario state at later ticks through `PerAgentBeliefView` and intentionally does not step the simulation, because any live step could lawfully refresh the target and would change the branch under test.

### 3. Add the stale-envelope assertion

Inside the golden's test body, at the chosen assertion tick:

- Read the envelope for the test belief through the test harness's belief-view surface.
- Assert `envelope.status == BeliefStatus::Stale`.
- Assert `envelope.confidence < claim_confidence_threshold`.
- Assert the same belief reads as fresh (`Certain`) at the acquisition tick to prove the test is observing decay rather than a pre-stale seed.

### 4. Regenerate golden inventory

After adding the test, run `python3 scripts/golden_inventory.py --write --check-docs` to regenerate `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and per-file details — these are the canonical golden inventories.

## Files to Touch

- `crates/worldwake-ai/tests/golden_*.rs` (modify — specific file named during implementation via the golden inventory lookup)
- `docs/generated/golden-e2e-inventory.md` + `docs/generated/golden-scenario-index.md` + `docs/generated/golden-scenario-details/` (regenerate)

## Out of Scope

- Authoring a net-new scenario — spec explicitly authorizes extending an existing one.
- Changes to engine code, belief storage, or envelope derivation — all upstream in `S113BELENV-001`.
- Generic motive-scaling proof for belief-weighted ranking — `survival-scattered` does not exercise that live seam honestly.
- Adding decision-trace fields or changing `BlockerRecordedPayload`/`PlanInvalidatedPayload` shapes — T002 owns payload changes.
- Assertion on the `belief_snapshot` field of a blocker/invalidation event — `S113BELENV-003` is where the relevant blocker snapshots get populated; this ticket could add a follow-on assertion later but it is not in-scope here.
- Candidate-gen emitter coverage for `emit_remote_*` ([archive/tickets/S113BELENV-004.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S113BELENV-004.md) owns that).

## Acceptance Criteria

### Tests That Must Pass

1. The extended/new golden test passes with the new stale-envelope assertion.
2. `cargo test -p worldwake-ai --test golden_<file>` (narrow to the specific extended golden).
3. Full AI suite: `cargo test -p worldwake-ai` — the envelope changes must not cause regressions in any other golden.
4. `python3 scripts/golden_inventory.py --check-docs` passes (doc regeneration is complete and consistent).

### Invariants

1. The golden is deterministic — `ChaCha8Rng` seed, `BTreeMap` iteration, no floats, no wall-clock reads (CLAUDE.md Determinism).
2. The assertion is on live envelope state through `PerAgentBeliefView`, not on ad-hoc debug output or direct world truth.
3. The scenario-isolation choice is documented in the test comments (Precision Rule 8).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_<file>.rs` — extended existing test (or new test within the same file) with the stale-envelope assertion.

### Commands

1. `cargo test -p worldwake-ai -- --list | grep golden_<file>` (confirm test name exists before writing assertions).
2. `cargo test -p worldwake-ai --test golden_<file> -- --ignored --exact <test_name>` (targeted, narrowed to the extended ignored golden after confirming the exact selector).
3. `cargo test -p worldwake-ai` (full AI suite).
4. `python3 scripts/golden_inventory.py --write --check-docs` (regenerate + verify docs).
5. `./scripts/verify.sh` before PR.

## Outcome

Completed 2026-04-21.

- Narrowed the ticket to the strongest honest live seam: `survival-scattered` does not exercise the landed `RaidTarget` motive-scaling path from `S113BELENV-003`, so the implementation proves stale-envelope surfacing only.
- Added ignored golden `seeded_target_location_belief_decays_to_stale_without_refresh` to `crates/worldwake-ai/tests/golden_survival_scattered.rs`.
- The regression seeds a scenario-backed target-location belief into the claim store, reads it through live `PerAgentBeliefView::believed_target_location(...)` at the acquisition tick and at the first stale tick, and intentionally does not step the simulation so no lawful perception refresh can change the branch under test.
- Regenerated the golden inventory docs after adding the new scenario block and golden entry.

## Verification Result

- `cargo test -p worldwake-ai --test golden_survival_scattered -- --list`
- `cargo fmt --all`
- `cargo test -p worldwake-ai --test golden_survival_scattered seeded_target_location_belief_decays_to_stale_without_refresh -- --ignored --exact`
- `python3 scripts/golden_inventory.py --write --check-docs`
- `cargo test -p worldwake-ai --test golden_survival_scattered`
- `cargo test -p worldwake-ai`

I did not run `cargo clippy --workspace --all-targets -- -D warnings` or `./scripts/verify.sh`.
