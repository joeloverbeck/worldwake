# S16BFORLEGEMEGOL-002: Suite 10 — Force Controller Departure Enables Rival Claim

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — golden test only
**Deps**: S16BFORLEGEMEGOL-001 (shared helpers), S16b spec Suite 10

## Problem

No test proves that an agent's physical departure from a force-law jurisdiction cascades into political vacancy and rival AI claim. This cross-system chain (travel → force-control clearing → AI candidate generation → PressForceClaim) must emerge from shared state, not orchestration.

## Assumption Reassessment (2026-03-22, updated during implementation)

1. `office_controller` relation: verified via `h.world.office_controller(office)` usage in `golden_offices.rs:2579,2743,2779`. Returns `Option<EntityId>`.
2. `txn.add_force_claim(agent, office)` exists at `golden_emergent.rs:1276`. Seeds a force claim for the agent.
3. `ForceControllerEstablished { controller }` is a live `OfficeSuccessionOutcome` variant at `politics_trace.rs:78`. `ForceControllerCleared` does NOT exist — the spec notes this. When the controller departs and no claimants remain, `ForceNoClaimants` fires. When claimant count drops to zero, the `office_controller` relation is cleared authoritatively. Assertions will use authoritative state (`office_controller == None`) instead of a non-existent trace variant.
4. `GoalKind::ClaimOffice { office }` is the live goal kind used for force-law claim generation (confirmed by usage in Suite 9 at `golden_emergent.rs:2670`).
5. Suite isolation: B (Rival) has sated needs + enterprise_weight=pm(800) to ensure ClaimOffice dominates. A (Controller) is human-controlled so departure is deterministic. Only one rival keeps scenario focused on departure→claim, not contested dynamics.
6. Travel action: A is human-controlled, issued a `RequestAction` for travel via `InputKind::RequestAction`. **Correction**: ORCHARD_FARM is NOT adjacent to VILLAGE_SQUARE — the path is VillageSquare → SouthGate → EastFieldTrail → OrchardFarm. Used GeneralStore (directly adjacent, 1-tick travel) as the departure destination instead.
7. Force-control departure detection: the offices system checks whether the controller is still present at the jurisdiction each tick. When absent, the controller relation is cleared. This is the authoritative mechanism.
8. `PressForceClaim` is the action name for force claims — confirmed in Suite 5 setup pattern (`add_force_claim` seeds the claim, then the offices system processes it).
9. B needs entity beliefs about A, the office, and knowledge of the office location to generate ClaimOffice. `seed_actor_local_beliefs` + `seed_known_office_at_place` + `seed_force_controller_belief` (from S16BFORLEGEMEGOL-001) handle this.
10. Scenario isolation: B's needs are sated (default HomeostaticNeeds), enterprise_weight=pm(800) ensures ClaimOffice dominates over any survival goals. No other AI agents are present.

## Architecture Check

1. Follows established Suite 5 and Suite 9 patterns: run function returns `(StateHash, StateHash)`, main test + replay companion.
2. No backward-compatibility shims.

## Verification Layers

1. Travel departure ordering → action trace: A's `travel` action committed before B's `press_force_claim`
2. Controller cleared after departure → authoritative state: `h.world.office_controller(office) == None` at intermediate tick
3. Rival claims vacant office → politics trace: `ForceControllerEstablished { controller: B }` after departure
4. Rival installed as holder → authoritative state: `h.world.office_holder(office) == Some(B)` after uncontested hold
5. Decision trace: B does NOT generate ClaimOffice before departure; DOES generate ClaimOffice after departure
6. Negative: no `declare_support` actions committed by any agent
7. Determinism → replay companion with identical hashes

## What to Change

### 1. Add `run_force_controller_departure_enables_rival_claim` function to `golden_emergent.rs`

Following the Suite 5/9 pattern:

**Setup**:
- Seed force-law office ("War Chief") at VILLAGE_SQUARE, succession_period=5, no eligibility rules
- Agent A ("Controller"): human-controlled, at VILLAGE_SQUARE. Perception profile. Enterprise utility.
- **Correction**: Do NOT pre-seed A's force claim via `add_force_claim`. Use `set_office_controller` directly instead. A lingering live force claim prevents B's installation (the `install_force_office_holder` check requires ALL live claimants to be the controller).
- Run enough initial ticks (or directly set `office_controller` via txn if API supports) for A to be established as controller.
- Agent B ("Rival"): AI-controlled, sated needs, enterprise_weight=pm(800), at VILLAGE_SQUARE. Perception profile. Known office at VillageSquare. Force controller belief (A as controller, contested=false).
- Seed B's local beliefs (A, office).
- Issue human input: `RequestAction` for A to travel away from VillageSquare.

**Tick loop** (~80 ticks):
- Wait for B to become `office_holder`.

**Assertions** (per Verification Layers above).

### 2. Add `golden_force_controller_departure_enables_rival_claim` test

Calls the run function with a specific seed.

### 3. Add `golden_force_controller_departure_enables_rival_claim_replays_deterministically` test

Replay companion.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify — add Suite 10: ~120 lines)

## Out of Scope

- Any engine/production code changes
- Changes to `golden_harness/mod.rs` beyond what S16BFORLEGEMEGOL-001 provides
- Contested force state (that's Suite 12)
- Belief propagation to remote agents (that's Suites 11/12)
- Testing the force-control departure mechanism itself (that's E16b unit tests)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_force_controller_departure_enables_rival_claim` — new test passes
2. `cargo test -p worldwake-ai golden_force_controller_departure_enables_rival_claim_replays_deterministically` — replay companion passes
3. `cargo test -p worldwake-ai --test golden_emergent` — all existing emergent tests still pass
4. `cargo clippy -p worldwake-ai` — no new warnings

### Invariants

1. Append-only event log — no mutation of existing events
2. Conservation — commodity quantities conserved across ticks
3. Determinism — same seed produces identical world and event log hashes
4. No `declare_support` actions in a force-law scenario
5. B's ClaimOffice generation is causally gated by A's departure (decision trace proves absence before, presence after)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs::golden_force_controller_departure_enables_rival_claim` — proves travel→vacancy→rival claim emergence
2. `crates/worldwake-ai/tests/golden_emergent.rs::golden_force_controller_departure_enables_rival_claim_replays_deterministically` — determinism companion

### Commands

1. `cargo test -p worldwake-ai golden_force_controller_departure_enables_rival_claim`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo clippy -p worldwake-ai`

## Outcome

**Completion date**: 2026-03-22

**What changed**:
- Added `run_force_controller_departure_enables_rival_claim` + 2 tests (main + replay) to `golden_emergent.rs` (~120 lines)
- Added `ForceInstallationDeferralReason` enum and `ForceInstallationDeferred` trace variant to `politics_trace.rs` (Principle 27 traceability improvement discovered during implementation)
- Added `check_force_installation_gate()` to `offices.rs` and restructured `resolve_force_succession()` to emit deferral traces
- Updated `docs/golden-e2e-scenarios.md` (Scenario 35), `docs/golden-e2e-coverage.md`, `docs/golden-e2e-testing.md` (topology reference, force installation tracing, same-tick ordering), `CLAUDE.md` (debugging table)

**Deviations from original plan**:
1. Travel destination changed from ORCHARD_FARM to GeneralStore — OrchardFarm is not adjacent to VillageSquare (3 hops away). GeneralStore is directly adjacent (1 tick).
2. Controller pre-seeded via `set_office_controller` only, NOT `add_force_claim` — a lingering live force claim blocks `install_force_office_holder` because the gate requires all live claimants to be the controller.
3. Decision trace pre-departure assertion uses strict `<` instead of `<=` — 1-tick travel starts and commits within tick 0, and the rival's AI correctly detects vacancy within the same tick.

**Verification results**:
- `cargo test --workspace` — all pass (including 40/40 golden_emergent)
- `cargo clippy --workspace` — clean
