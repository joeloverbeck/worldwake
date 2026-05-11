# S142CONEVEINS-007: End-to-end golden coverage for contention inspectability

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test infrastructure only; no production code change
**Deps**: archive/tickets/S142CONEVEINS-001.md, archive/tickets/S142CONEVEINS-002.md, archive/tickets/S142CONEVEINS-003.md, archive/tickets/S142CONEVEINS-004.md, archive/tickets/S142CONEVEINS-005.md, archive/tickets/S142CONEVEINS-006.md

## Problem

Tickets 001 through 006 land the type substrate, payload widening, two emission substrates, AI lookup, and observer rendering for S142. Without end-to-end golden coverage, the spec's headline contracts — "every resolution emits", "deterministic emission order", "end-to-end attribution from `BlockingFact::ReservationConflict` to `ContentionResolved`" — have no canonical regression proof. This ticket adds the four scenario goldens listed in the spec's Validation and Falsification section, plus the replay-parity assertion on `survival-contested.ron`, in a new `golden_contention_inspectability.rs` file under `crates/worldwake-ai/tests/`.

## Assumption Reassessment (2026-05-10)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The existing golden test infrastructure in `crates/worldwake-ai/tests/` includes 20+ `golden_*.rs` files (verified by `ls`). Per `docs/golden-e2e-testing.md` (canonical guide for golden tests, per `tickets/README.md` line 9), each golden file owns its own scenario set, harness setup, and assertion style. The new `golden_contention_inspectability.rs` follows the existing pattern (e.g., `golden_quantity_aware_acquisition.rs` — verified to exist).
2. `scenarios/survival-contested.ron` exists (verified). The 1440-tick replay parity test loads this scenario, runs to completion, captures the `events_by_tag(EventTag::ContentionResolved)` sequence with payloads, then runs again from the same seed and asserts the captured sequence is byte-identical to the second run. Per CLAUDE.md's Determinism invariant (`ChaCha8Rng`-seeded, BTreeMap iteration), the assertion holds.
3. The 4 scenarios in the spec's Validation section are: (1) three-agent single-slot orchard via the resource-extraction path → ticket 004's emission proves coverage; (2) survival-contested.ron-style slot grants → ticket 004's emission + ticket 003's facility-queue emission; (3) wash-basin facility queue admission → ticket 003's emission; (4) end-to-end `BlockingFact::ReservationConflict` attribution with non-`None` `contention_event` → ticket 005's lookup. Each golden exercises a different code path and asserts at the strongest available layer.
4. Per `docs/precision-rules.md` Rule 5 (verification surface mapping), each golden's invariant maps to a distinct surface: scenarios 1/2/3 assert event-log delta (events present with correct claimants); scenario 4 asserts the populated `contention_event` field on the recorded blocker, plus the corresponding event presence (composite assertion). Replay parity asserts byte-equality of the event-log slice.
5. Per Rule 8 (scenario isolation): scenarios 1 and 2 use minimal authored scenarios designed to isolate the resource-extraction path from facility-queue contention; scenario 3 isolates the facility-queue path. Each scenario's setup explicitly excludes lawful competing affordances that would emit unrelated `ContentionResolved` events. Document the isolation choice in each test's body.

## Architecture Check

1. End-to-end goldens are the strongest causal-chain proof for the spec. They exercise the full chain from grant decision through event emission, AI lookup, and observer rendering.
2. Per FND-31 (validation and falsification are first-class), this ticket adds the canonical regression scenarios so future changes that break the contention-resolution contract surface as test failures, not silent behavioral drift.
3. Per FND-28, the goldens assert against the canonical post-S142 contract; no comparison with a "pre-S142 fallback" form is included. Goldens that fail to land alongside the implementation tickets indicate the implementation is incomplete, not that the goldens need a fallback path.
4. Replay parity confirms determinism: per CLAUDE.md's Determinism invariant, the same seed must produce the same event sequence. Adding `ContentionResolved` events to the canonical event log expands the deterministic surface; the replay-parity test guards against accidental non-determinism in emission order or claimant ordering.

## Verification Layers

1. Three-agent orchard (extraction-path) emission — event-log delta + payload inspection
2. Survival-contested.ron multi-substrate emission — event-log delta + payload inspection
3. Wash-basin facility-queue emission — event-log delta + payload inspection
4. End-to-end `BlockingFact::ReservationConflict.contention_event` attribution — composite assertion: blocker memory snapshot inspection + event-log lookup verification
5. Replay parity on 1440-tick `survival-contested.ron` — event-log byte-equality assertion across two runs from the same seed

## What to Change

### 1. New file `crates/worldwake-ai/tests/golden_contention_inspectability.rs`

File structure follows existing golden-test conventions per `docs/golden-e2e-testing.md`. Contains 5 test functions:

- `golden_three_agents_single_slot_orchard_emit_per_grant`: minimal scenario with one orchard `ResourceSource` + `ResourceExtractionQueues` configured for 1 slot, 3 agents converging at offset arrival ticks. Run to completion. Assert: exactly one `ContentionResolved` event per slot grant; claimants in arrival order; `winner` matches first arrival; `Granted`/`QueuedAhead`/`QueuedBehind` outcomes correctly classified; `resolution_rule == ContentionResolutionRule::ArrivalTime`.

- `golden_survival_contested_multi_substrate_emission`: load `scenarios/survival-contested.ron`, run for the scenario's defined window (e.g., 400 ticks per the contested authored bound). Assert: `ContentionResolved` events emit at both facility-queue grants (well, latrine, etc.) and resource-extraction grants (orchard, well-water-source, etc.); each event carries the correct `(facility, action)` `AffordanceKey`; deterministic claimant ordering (BTreeMap ordinals).

- `golden_wash_basin_facility_queue_admission`: minimal scenario with a wash basin facility configured for `auto_promote = true`, 2+ agents queuing for the basin. Assert: facility-queue path emits `ContentionResolved` at each `promote_ready_head` grant; queued-ahead/queued-behind classification matches arrival order; `winner` matches the head waiter.

- `golden_blocker_memory_attribution_e2e`: scenario where Agent A and Agent B both attempt to acquire a single-slot resource. Agent B fails with `ReservationConflict`. Assert: B's `BlockerMemory` carries `BlockingFact::ReservationConflict { affordance, contention_event: Some(_) }`; the populated `EventId` resolves to a `ContentionResolved` event in the log with `at_tick == B's failure tick` and `winner == Some(A's actor)`.

- `golden_survival_contested_replay_parity`: load `scenarios/survival-contested.ron`, run for 1440 ticks twice from the same seed; capture `events_by_tag(EventTag::ContentionResolved)` from each run; assert byte-identical event sequences and payloads.

### 2. Test fixtures

For scenarios 1, 3, 4: author minimal RON-free fixtures using the existing `ScenarioBuilder` / `World` test helpers (consistent with sibling golden-test conventions). For scenarios 2 and 5: load `scenarios/survival-contested.ron` directly per the spec.

## Files to Touch

- `crates/worldwake-ai/tests/golden_contention_inspectability.rs` (new)

## Out of Scope

- Implementation code (tickets 001 through 006 own the production changes)
- Soak harness extension — replay parity is bounded to the 1440-tick `survival-contested.ron` window per the spec; longer-window soak is a future concern
- Generated golden inventory regeneration — per `tickets/README.md`, run `python3 scripts/golden_inventory.py --write --check-docs` as part of this ticket's verification commands but the regenerated docs land alongside the ticket, not as a separate ticket

## Acceptance Criteria

### Tests That Must Pass

1. `golden_three_agents_single_slot_orchard_emit_per_grant`
2. `golden_survival_contested_multi_substrate_emission`
3. `golden_wash_basin_facility_queue_admission`
4. `golden_blocker_memory_attribution_e2e`
5. `golden_survival_contested_replay_parity`
6. Existing golden suite remains green: `cargo test -p worldwake-ai --tests`
7. Generated golden inventory updated and matches code: `python3 scripts/golden_inventory.py --write --check-docs`

### Invariants

1. Per FND-31, the goldens are canonical regressions: future commits that break any of the asserted contracts must fail this file's tests.
2. Per CLAUDE.md Determinism, the replay-parity test asserts byte-identical event sequences across two same-seed runs.
3. Per Rule 8 (scenario isolation), each minimal scenario's setup excludes unrelated lawful competing affordances; the test bodies document the isolation choice.
4. Per FND-26, goldens assert at the event-log delta layer (the canonical authoritative surface) rather than at observer rendering or AI internal trace.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_contention_inspectability.rs` (new) — 5 golden tests covering the spec's 4 validation scenarios + replay parity.

### Commands

1. `cargo test -p worldwake-ai --test golden_contention_inspectability`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `python3 scripts/golden_inventory.py --write --check-docs`
5. `./scripts/verify.sh`
