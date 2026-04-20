# S122FRAASSCOM-002: Populate `CommodityAvailableAt` in `populate_assumptions`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `populate_assumptions` signature widened from `(domain: &IntentionDomain, agent, view)` to `(frame: &IntentionFrame, agent, view)`; the `Travel | Errand` arm gains a conditional `CommodityAvailableAt` push driven by `frame.expected_commodity()`.
**Deps**: archive/tickets/S122FRAASSCOM-001.md

## Problem

The `FrameAssumption::CommodityAvailableAt` variant exists in the type system but `populate_assumptions` never adds it to any frame. Without population, the per-tick assumption refresh has nothing to evaluate. With S122FRAASSCOM-001's `expected_commodity` helper available, this ticket extends the `Travel | Errand` arm to push the variant when the committed goal is `AcquireCommodity`. The new assumption is harmless until S122FRAASSCOM-003 wires the real evaluator (the existing always-true stub returns `AllPass`).

## Assumption Reassessment (2026-04-20)

1. Existing focused/unit coverage in `crates/worldwake-ai/src/agent_tick/frame.rs#[cfg(test)]`: 5 `populate_*` tests at lines 768 (Travel), 788 (Care), 813 (Escort), 844 (Errand), 864 (Generic). Each currently passes `&IntentionDomain::...` to `populate_assumptions`. After widening, all 5 call sites need to pass `&IntentionFrame`. The `make_frame` helper at line 735 already constructs an `IntentionFrame` with `goal: GoalKey::new(GoalKind::Sleep)` — Travel/Errand domains with the Sleep goal produce only `RouteExists` (no `CommodityAvailableAt`), so the existing assertions remain valid after migration. One new test added: Travel + `AcquireCommodity` goal asserts both `RouteExists` AND `CommodityAvailableAt`. The stub-pinning test `commodity_available_at_stubbed_as_pass` at line 949 continues to pass because the always-true stub arm is unchanged in this ticket (deletion lands in S122FRAASSCOM-003).
2. Spec deliverable D2 defined in `specs/S122-frame-assumption-commodity-availability.md` (lines 116–141). Per-tick population call site at `crates/worldwake-ai/src/agent_tick/mod.rs:501` confirmed by reassessment — `frame.assumptions = populate_assumptions(&frame.domain, agent, &view);`.
3. Shared abstraction boundary under audit: `populate_assumptions` signature in `agent_tick/frame.rs` and its single production call site in `agent_tick/mod.rs:501`. The boundary is the function signature.
6. Intended layer: AI / planning-layer logic. Local needs-only harness is sufficient — population is a pure-read computation against a mock view.
13. Adjacent contradiction noted: D7 (stub removal) cannot land in this ticket without also landing D3, because removing the always-true match arm makes `evaluate_assumptions`'s match non-exhaustive (the variant still exists in `FrameAssumption`). Re-classified D7 into S122FRAASSCOM-003 to keep the workspace compiling per the workspace-builds-after-each-ticket constraint. The spec's Decomposition Hint bundled D2 + D7 under T2; this ticket implements D2 only and S122FRAASSCOM-003 picks up D7 alongside the replacement evaluator arm.

## Architecture Check

1. Widening to `&IntentionFrame` is the minimum signature change that exposes `frame.goal.kind` (needed by `expected_commodity`) without leaking goal details into `IntentionDomain`. The single production call site at `mod.rs:501` already holds `frame: &mut IntentionFrame` in scope, so the change is local.
2. No backwards-compatibility shim — the old `(domain, agent, view)` signature is replaced, not aliased. Care / Escort / Generic arms are unchanged; only Travel | Errand gains one conditional push.

## Verification Layers

1. Population produces both `RouteExists` and `CommodityAvailableAt` for Travel + AcquireCommodity -> focused unit test asserts both variants in the returned vec.
2. Population skips `CommodityAvailableAt` for non-acquisition Travel goals -> focused unit test (existing `populate_travel_produces_route_exists` continues to assert only `RouteExists` because `make_frame` defaults to `GoalKind::Sleep`).
3. Population skips `CommodityAvailableAt` for non-Travel/Errand domains -> focused unit tests (existing Care/Escort/Generic tests unchanged).
6. Single-layer ticket — assumption population is observable through unit assertions on the returned `Vec<FrameAssumption>`. No action lifecycle or event-log delta involved; per-tick re-population is idempotent for stable goals.

## What to Change

### 1. Widen `populate_assumptions` signature

- File: `crates/worldwake-ai/src/agent_tick/frame.rs`
- Change `pub(super) fn populate_assumptions(domain: &IntentionDomain, agent: EntityId, view: &dyn RuntimeBeliefView) -> Vec<FrameAssumption>` to `pub(super) fn populate_assumptions(frame: &IntentionFrame, agent: EntityId, view: &dyn RuntimeBeliefView) -> Vec<FrameAssumption>`.
- Inside the function body, derive `let domain = &frame.domain;` and proceed with the existing `match *domain { ... }`.

### 2. Push `CommodityAvailableAt` in the `Travel | Errand` arm

- File: `crates/worldwake-ai/src/agent_tick/frame.rs`
- After the existing `RouteExists` push in the `Travel { destination } | Errand { destination }` arm, add:

  ```rust
  if let Some((commodity, place)) = frame.expected_commodity() {
      assumptions.push(FrameAssumption::CommodityAvailableAt { commodity, place });
  }
  ```

### 3. Update the production call site

- File: `crates/worldwake-ai/src/agent_tick/mod.rs:501`
- Change `frame.assumptions = populate_assumptions(&frame.domain, agent, &view);` to `frame.assumptions = populate_assumptions(frame, agent, &view);`.

### 4. Migrate the 5 existing unit-test call sites

- File: `crates/worldwake-ai/src/agent_tick/frame.rs#[cfg(test)]`
- Each `populate_*` test (lines 768/788/813/844/864) currently constructs the domain inline and calls `populate_assumptions(&IntentionDomain::..., agent, &view)`. Update each to construct `let frame = make_frame(domain, FrameState::Active);` (or equivalent) and call `populate_assumptions(&frame, agent, &view)`.
- Existing assertions remain valid: `make_frame` defaults `goal: GoalKey::new(GoalKind::Sleep)`, so `expected_commodity` returns `None` and no `CommodityAvailableAt` is added.

### 5. Add new unit test

- File: `crates/worldwake-ai/src/agent_tick/frame.rs#[cfg(test)]`
- New test `populate_travel_with_acquire_commodity_produces_route_and_commodity`:

  ```rust
  let agent = make_entity(0);
  let place_a = make_entity(10);
  let dest = make_entity(20);
  let mut view = MockBeliefView::new();
  view.alive.insert(agent);
  view.places.insert(agent, place_a);
  let frame = IntentionFrame {
      goal: GoalKey::from(GoalKind::AcquireCommodity {
          commodity: CommodityKind::Apple,
          purpose: CommodityPurpose::SelfConsume,
      }),
      domain: IntentionDomain::Travel { destination: dest },
      assumptions: Vec::new(),
      state: FrameState::Active,
      established_at: Tick(0),
      last_progress_tick: None,
      stalled_ticks: 0,
      patience_limit: 30,
  };
  let assumptions = populate_assumptions(&frame, agent, &view);
  assert!(assumptions.contains(&FrameAssumption::RouteExists { from: place_a, to: dest }));
  assert!(assumptions.contains(&FrameAssumption::CommodityAvailableAt {
      commodity: CommodityKind::Apple,
      place: dest,
  }));
  ```

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — signature widening + new push + 5 test migrations + 1 new test)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — call site update at line 501)

## Out of Scope

- Stub removal (D7) — moved to S122FRAASSCOM-003 due to exhaustiveness constraint.
- Wiring the `evaluate_assumptions` arm — S122FRAASSCOM-003.
- Confidence gating, container-bound goods, seller-listed commodities — spec Non-Goals.
- Trace surface payload — S122FRAASSCOM-004.

## Acceptance Criteria

### Tests That Must Pass

1. New: `populate_travel_with_acquire_commodity_produces_route_and_commodity` — Travel + AcquireCommodity → both `RouteExists` and `CommodityAvailableAt` present.
2. Migrated: `populate_travel_produces_route_exists` — Travel + Sleep goal (default) → only `RouteExists`.
3. Migrated: `populate_care_produces_target_alive_and_route` — Care domain → no `CommodityAvailableAt`.
4. Migrated: `populate_escort_produces_target_alive_and_route` — Escort domain → no `CommodityAvailableAt`.
5. Migrated: `populate_errand_produces_route_exists` — Errand + Sleep goal → only `RouteExists`.
6. Migrated: `populate_generic_produces_no_critical_threat` — Generic domain → no `CommodityAvailableAt`.
7. Existing `commodity_available_at_stubbed_as_pass` (line 949) continues to pass — stub arm unchanged in this ticket.
8. Existing suite: `cargo test -p worldwake-ai --lib agent_tick` passes.

### Invariants

1. `populate_assumptions` reads only from the frame and the view; no global state. (FND-7.)
2. `CommodityAvailableAt` is added if and only if the frame's domain is Travel or Errand AND the committed goal is `AcquireCommodity`. (FND-21 — assumption reflects intention.)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/frame.rs#[cfg(test)]` — 1 new test (`populate_travel_with_acquire_commodity_produces_route_and_commodity`) and 5 migrated tests (call signature update only).

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::frame`
2. `cargo test -p worldwake-ai --lib agent_tick`
3. `cargo clippy --workspace --all-targets -- -D warnings`
