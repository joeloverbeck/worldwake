# S115AGEMAN-007: golden_agenda_lifecycle scenario

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — golden scenario + observer rendering assertions only.
**Deps**: [S115AGEMAN-005](S115AGEMAN-005.md)

## Problem

Unit and integration tests (tickets 003, 004, 006) cover the agenda manager's behavior at the function and component level, but the full end-to-end contract — an agent adopts a purchase goal, the goal becomes pending when the merchant departs, revives when the merchant returns, commits, and completes; plus a cargo-delivery goal reaches destination and transitions to `Suspended` via the Satisfied pre-check — can only be proved with a golden E2E scenario. The observer-surface claim from spec D5 ("plateauing agent is visible as '5 pending goals, 2 suspended'") also needs a rendering assertion to anchor the FND-29 debuggability contract. This ticket writes that golden.

## Assumption Reassessment (2026-04-22)

1. Golden harness layout: `crates/worldwake-ai/tests/golden_*.rs` follows a standard pattern (scenario RON file in `scenarios/`, harness helper in `golden_harness/`, test assertions in the golden file). This ticket creates `crates/worldwake-ai/tests/golden_agenda_lifecycle.rs` plus `scenarios/agenda-lifecycle.ron` and reuses the existing harness infrastructure.
2. Observer rendering surface: `crates/worldwake-cli/src/bin/observer.rs` already renders per-agent decision state (see ~line 366 onward for `DecisionEventPayload` rendering). This ticket extends the observer to distinguish `pending` vs `suspended` agenda entries and render revival triggers for pending. The exact rendering extension is small (add a new section or extend an existing one).
3. The shared boundary under audit is `AgendaState` as observed from outside the ai crate: the observer reads it via the public runtime-map accessor (ticket 002 added `AgendaState` as a field on `AgentDecisionRuntime`; the existing runtime-map accessor path serves observer reads per spec D8). No new accessor trait is introduced.
4. `docs/generated/golden-e2e-inventory.md` is the canonical golden-test inventory. Regenerate with `python3 scripts/golden_inventory.py --write --check-docs` after adding the new goldens.
5. Golden scenario isolation (precision-rules §8): the purchase-revival scenario is intended to prove agenda lifecycle correctness through a merchant-departs-returns cycle. Lawful competing affordances at the test place (hunger-drive eating, sleep) are intentionally excluded from setup by giving the test agent no hunger/fatigue pressure during the relevant ticks. The cargo-delivery scenario similarly isolates the Satisfied pre-check by ensuring the agent has no competing goals.
6. Scenario content validation (codebase-validation §3.3B): `WorkstationTag`, `PlaceTag`, commodity kinds, and `HomeostaticNeedId` must match current enum variants. Recipe names use Title Case. `MerchandiseProfile` home_facility uses string-name in `*Def` wrapper.

## Architecture Check

1. Golden scenario is a declarative RON file + test harness — no new production code beyond observer rendering extension. The FND-29 debuggability contract is surfaced through the observer view, which is already the designated inspection surface.
2. Scenario isolates one lifecycle contract per test (purchase revival; cargo satisfaction) — avoids the mixed-branch trap where multiple agenda outcomes compete and any one passing masks a regression in the others.

## Verification Layers

1. Purchase revival — integration (golden) test: scenario RON boots, step 20 ticks, assert event-log contains `GoalOffered → GoalCommitted → GoalSuspended (merchant departed) → GoalCommitted (merchant returned) → PlanAdopted → ActionCommitted (purchase)` in order.
2. Cargo satisfaction — golden test: scenario boots with agent carrying commodity to destination; after reaching destination, assert `runtime.agenda_state.committed.as_ref().unwrap().phase == AgendaPhase::Suspended` AND observer output for that agent contains the suspended-entry rendering (text match on "Suspended" label + goal description).
3. Observer rendering — focused unit test in `observer.rs` asserting the new pending/suspended section formats correctly for a fixed input `AgendaState`.
4. Event-log ordering — action trace key `(tick, sequence_in_tick)`: the `GoalCommitted` event precedes `PlanAdopted` within the same tick (agenda commits before plan selection consumes the slot).

## What to Change

### 1. New scenario `scenarios/agenda-lifecycle.ron`

Design:
- Single test agent + single merchant NPC + one place (shared market) + one adjacent place (merchant's home).
- Merchant moves between the market and home on a scripted patrol (or deterministic schedule).
- Agent has `AcquireCommodity { commodity: Bread, purpose: SelfConsume }` goal driven by hunger or by scenario-seeded belief that bread is purchasable.
- Scenario ticks: 1-3 merchant at market, agent commits purchase goal; 4-8 merchant at home, agent's goal → pending (revival trigger: `CounterpartyAvailable { counterparty: merchant, place: market }`); 9-12 merchant returns, revival fires, commit again, complete purchase.

Validate per codebase-validation §3.3B:
- `WorkstationTag` values: check against `crates/worldwake-core/src/production.rs`.
- `PlaceTag` values: check against `crates/worldwake-core/src/topology.rs`.
- Commodity: `CommodityKind::Bread` exists at `crates/worldwake-core/src/items.rs`.
- Recipe names: if any, use Title Case.
- `AgendaProfile` values: authored with default or custom 10/4/2 to exercise small capacities.

### 2. New scenario `scenarios/agenda-cargo-suspended.ron` (OR reuse cargo_harness via in-test setup)

Cargo-delivery scenario reusing existing `cargo_harness` fixture: agent carries commodity to destination; after reaching, `classify_rejection`'s Satisfied pre-check fires; `AgendaState.committed.phase` transitions to `Suspended`; observer renders the suspended entry for one tick before `KillCondition::External` clears it on a subsequent tick (but since `External` never fires on its own, the entry persists until explicit clearing — adjust test to either assert the persistence OR seed a `KillCondition::TickExpiry` for cleanup).

### 3. New test file `crates/worldwake-ai/tests/golden_agenda_lifecycle.rs`

Two tests:
- `agent_purchase_goal_revives_when_merchant_returns`: drives `scenarios/agenda-lifecycle.ron`, asserts event sequence (purchase revival).
- `cargo_delivery_suspends_committed_goal_via_satisfied_classifier`: drives `scenarios/agenda-cargo-suspended.ron`, asserts `AgendaState.committed.phase == Suspended` after delivery and observer rendering.

### 4. Observer pending/suspended section

Extend `crates/worldwake-cli/src/bin/observer.rs` to add a per-agent rendering of agenda state:
- Count of pending + count of suspended
- For each pending: goal key short description + revival trigger summary
- For each suspended: goal key short description + reason label (Satisfied / Infeasible / other)

Keep the rendering concise — target 10-20 lines per agent.

### 5. Regenerate golden inventory

Run `python3 scripts/golden_inventory.py --write --check-docs` to refresh `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/`.

## Files to Touch

- `scenarios/agenda-lifecycle.ron` (new)
- `scenarios/agenda-cargo-suspended.ron` (new, or reuse existing cargo_harness setup)
- `crates/worldwake-ai/tests/golden_agenda_lifecycle.rs` (new)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — add agenda-state rendering section)
- `docs/generated/golden-e2e-inventory.md` (regenerate)
- `docs/generated/golden-scenario-index.md` (regenerate)
- `docs/generated/golden-scenario-details/` (regenerate)

## Out of Scope

- Changes to production agenda logic (tickets 003-005)
- Additional unit/integration tests (ticket 006)
- Observer UI beyond the agenda-state section (future ticket if needed)
- Performance guards — golden runtime should stay within existing test-runtime budgets; if it doesn't, a separate perf ticket is warranted

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_agenda_lifecycle -- agent_purchase_goal_revives_when_merchant_returns` passes.
2. `cargo test -p worldwake-ai --test golden_agenda_lifecycle -- cargo_delivery_suspends_committed_goal_via_satisfied_classifier` passes.
3. `cargo test -p worldwake-cli -- observer` — new rendering unit test passes.
4. `python3 scripts/golden_inventory.py --check-docs` passes (regenerated docs in sync).
5. Existing suite: `cargo test --workspace` passes.

### Invariants

1. Purchase-revival scenario exercises the full `Pending → Revived → Committed → Completed` lifecycle through real merchant arrival/departure (no omniscience — revival trigger fires only when the agent perceives the merchant).
2. Cargo-delivery scenario drives the Satisfied pre-check path, not any carve-out (verified by the path through `classify_rejection`).
3. Observer rendering distinguishes `Pending` vs `Suspended` and shows revival triggers — FND-29 debuggability contract.
4. Scenario determinism: repeated runs with the same seed produce identical event logs.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_agenda_lifecycle.rs` (new) — two golden tests as specified in Change 3.
2. `crates/worldwake-cli/src/bin/observer.rs` (modify inline `#[cfg(test)]`) — rendering format test.

### Commands

1. `cargo test -p worldwake-ai --test golden_agenda_lifecycle`
2. `cargo test -p worldwake-cli -- observer`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test --workspace`
5. `./scripts/verify.sh`
