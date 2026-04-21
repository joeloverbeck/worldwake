# S113BELENV-005: Golden — stale-belief surfacing and motive scaling

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (test-only; exercises existing engine behavior from T001 + T003)
**Deps**: S113BELENV-001, S113BELENV-003

## Problem

The envelope lands as infrastructure (T001) and as reasoning-layer consumers (T003), but the cross-system story — "an agent acquires a belief, time passes, the belief decays, and the planner visibly re-weights its motive" — is not yet covered by a golden E2E test. Spec S113 Validation #14 calls for an extension of an existing rumor-driven scenario (if present) or the `survival-scattered.ron`-based golden to assert the envelope surfaces stale beliefs after sufficient staleness decay (ticks elapsed such that `effective_claim_confidence` drops below the agent's `claim_confidence_threshold`) without any perception update.

This ticket adds one golden assertion on an existing scenario rather than authoring a new scenario, per the spec's own framing ("extend an existing ... scenario"). The intended invariant: a belief formed at tick T with fresh confidence decays through the `Certain → Probable → Stale` band boundaries on its natural schedule, and the ranking consequence (motive scaling by envelope confidence per T003) is visible in the agent's trajectory.

## Assumption Reassessment (2026-04-21)

1. `scenarios/survival-scattered.ron` exists per prior reassessment (spot-check confirmed during `/reassess-spec`). The corresponding golden lives in `crates/worldwake-ai/tests/golden_*.rs` — implementer greps `docs/generated/golden-e2e-inventory.md` to find the exact test name, per `tickets/README.md` guidance on the canonical golden inventory.
2. No rumor-driven scenario exists today with the exact shape S113 imagined; `survival-scattered` is the fallback the spec authorizes. Shared abstraction boundary under audit: the test harness's interaction between `scenarios/*.ron` and the golden runner. Intended invariant restatement: *the envelope surfaces a `Stale` status once effective confidence has decayed below `claim_confidence_threshold`, and the ranking scale-down (T003) makes the stale-belief goal measurably less preferred than a fresh-belief goal.*
3. The golden must not rely on wall-clock time, floats, or HashMap iteration order (CLAUDE.md Determinism). The decay math is deterministic: `effective = confidence - staleness_penalty_per_tick * ticks_elapsed`. With default `confidence_policy.staleness_penalty_per_tick = Permille(12)` and `claim_confidence_threshold = Permille(50)`, a fresh direct observation (Permille(950)) crosses into `Stale` at tick `ceil((950 - 50) / 12) = 75` — but this is only the default; the scenario's agent may override these.
4. Intended invariant: at the assertion tick, the agent's envelope read of a target location formed at tick 0 returns `status: Stale`, and the `motive_score` for any goal anchored on that target is below the same goal ranked with a `Certain`-status target at the same tick (comparison requires either a second agent or a controlled fixture; implementer picks whichever is cleaner within the scenario).
5. This is a golden / E2E ticket. Live `GoalKind` surface: whichever GoalKind the extended scenario uses for its acquisition/engagement arc — implementer names this during implementation (likely `ConsumeOwnedCommodity`, `EngageHostile`, or similar depending on scenario anchor). The specific `GoalKind` affects whether T003's ranking scaling actually reaches the motive arithmetic — scaling only applies to goals whose `motive_score` formula reads belief-based signals.
6. AI regression type — intended layer is golden E2E coverage. Local needs-only harness is **not** sufficient here because the assertion depends on the full ranking substrate; full action registries are required. State harness boundary explicitly in the test.
7. No ordering-sensitive claim in the golden — the proof is on envelope state (`status`) and motive-score magnitude at a specific tick, not on cross-agent interaction order. If a comparative assertion is used (fresh vs stale agent), strict tick separation is not required; same-tick comparison is sufficient.
12. The scenario must be explicit about which lawful competing affordances were excluded from setup. If extending `survival-scattered`, document in the scenario's comment block that the envelope assertion is the intended branch and that any additional need-driven distractions are intentionally absent (or already handled by the scenario's existing isolation choices).
13. Adjacent contradiction: if T003 did not end up wiring ranking scaling for the specific `GoalKind` the scenario uses, this golden cannot prove the motive-scaling half of the story. In that case, the golden narrows to *envelope surfacing* only — asserting `status: Stale` without the motive-score comparison. Implementer surfaces this as a scope-narrowing finding during implementation if it applies.

## Architecture Check

1. The golden extends an existing scenario rather than authoring a new one, minimizing engine-unrelated churn (test-only ticket).
2. The assertion reads `status` and/or motive-score magnitudes through normal trace/observer surfaces — no bespoke harness-only access to belief state that would cheat the belief-vs-world separation.
3. Decay math is deterministic per CLAUDE.md Determinism invariants — `ChaCha8Rng`-seeded setup, no floats, no wall-clock time, `BTreeMap`-based iteration.

## Verification Layers

1. Envelope surfacing — at the assertion tick, `believed_target_location` for the test agent's target returns `status: Stale` → decision-trace assertion or belief-view inspection via the test harness's belief-view surface.
2. Motive scaling consequence — the agent's candidate motive for the stale-target goal is measurably below a fresh-target baseline → decision-trace motive-score assertion (preferred) or candidate-ordering assertion.
3. No event-log-delta assertion is required here — the golden is proving reasoning-layer behavior, not authoritative world mutation.
4. This is a cross-system golden (belief decay + envelope + ranking). Per validation-surface mapping: reasoning-layer invariants → decision trace; not action trace (no action lifecycle under test) and not event-log delta (no authoritative mutation under test).

## What to Change

### 1. Identify the target golden

Use `docs/generated/golden-e2e-inventory.md` to locate the golden that loads `scenarios/survival-scattered.ron` (or the closest extant rumor-driven scenario if one has been added since the spec was written). Record its path under `crates/worldwake-ai/tests/` for the Files to Touch list.

### 2. Extend the scenario with a belief-fixture setup (if needed)

If the existing scenario's initial conditions do not seed a belief that will naturally decay during the scenario's tick window, extend the scenario's setup to include one. Options (implementer picks during writing):

- Seed a target-location belief at tick 0 via a scripted pre-run observation followed by a long gap.
- Use an existing scenario feature (rumor injection, report delivery) if available.
- Add a second agent whose belief freshness differs for comparative assertion.

Document the isolation choice in the scenario comments per Precision Rule 8.

### 3. Add the envelope-surfacing assertion

Inside the golden's test body, at the chosen assertion tick:

- Read the envelope for the test belief through the test harness's belief-view surface.
- Assert `envelope.status == BeliefStatus::Stale`.
- If a comparative setup is used, assert the motive score differential between the fresh-target candidate and the stale-target candidate is non-zero and in the expected direction.

### 4. Regenerate golden inventory

After adding the test, run `python3 scripts/golden_inventory.py --write --check-docs` to regenerate `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and per-file details — these are the canonical golden inventories.

## Files to Touch

- `crates/worldwake-ai/tests/golden_*.rs` (modify — specific file named during implementation via the golden inventory lookup)
- `scenarios/survival-scattered.ron` *or* a copy/variant (modify — only if the existing scenario needs additional setup for the belief fixture)
- `docs/generated/golden-e2e-inventory.md` + `docs/generated/golden-scenario-index.md` + `docs/generated/golden-scenario-details/` (regenerate)

## Out of Scope

- Authoring a net-new scenario — spec explicitly authorizes extending an existing one.
- Changes to engine code, belief storage, envelope derivation, or ranking arithmetic — all upstream in T001/T003.
- Adding decision-trace fields or changing `BlockerRecordedPayload`/`PlanInvalidatedPayload` shapes — T002 owns payload changes.
- Assertion on the `belief_snapshot` field of a blocker/invalidation event — T003 is where snapshots get populated; this ticket could add a follow-on assertion later but it is not in-scope here.
- Candidate-gen emitter coverage for `emit_remote_*` (T004 owns that).

## Acceptance Criteria

### Tests That Must Pass

1. The extended/new golden test passes with the new envelope-surfacing assertion.
2. `cargo test -p worldwake-ai --test golden_<file>` (narrow to the specific extended golden).
3. Full AI suite: `cargo test -p worldwake-ai` — the envelope changes must not cause regressions in any other golden.
4. `python3 scripts/golden_inventory.py --check-docs` passes (doc regeneration is complete and consistent).

### Invariants

1. The golden is deterministic — `ChaCha8Rng` seed, `BTreeMap` iteration, no floats, no wall-clock reads (CLAUDE.md Determinism).
2. The assertion is on envelope state / motive-score arithmetic, not on ad-hoc debug output (Precision Rule 15 — decision-trace preference).
3. The scenario-isolation choice is documented in the scenario or test comments (Precision Rule 8).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_<file>.rs` — extended existing test (or new test within the same file) with the envelope-surfacing assertion.
2. `scenarios/survival-scattered.ron` — minor extension if needed for belief-fixture setup.

### Commands

1. `cargo test -p worldwake-ai -- --list | grep golden_<file>` (confirm test name exists before writing assertions).
2. `cargo test -p worldwake-ai --test golden_<file>` (targeted, narrowed to the extended golden).
3. `cargo test -p worldwake-ai` (full AI suite).
4. `python3 scripts/golden_inventory.py --write --check-docs` (regenerate + verify docs).
5. `./scripts/verify.sh` before PR.
