# FND02-004: Feedback Dampening Audit Across Phase 2 Systems

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Possible — code fixes only if the audit finds a real undamped loop
**Deps**: Phase 2 complete

## Problem

No systematic audit has been performed on Phase 2 systems for amplifying feedback loops (Principle 10: Every Positive Feedback Loop Needs a Physical Dampener). Each system was implemented independently; cross-system amplification patterns may exist undocumented and undamped. Numerical clamps (`min`, `max`, `clamp`) are not acceptable dampeners — only physical world mechanisms qualify.

## Assumption Reassessment (2026-03-14)

1. `docs/dampening-audit-phase2.md` does not exist — confirmed, must be created.
2. The core Phase 2 domains are present, but the AI-side enterprise dampening logic is not isolated to `enterprise.rs`. The effective enterprise/goal-spiral surface spans `enterprise.rs`, `candidate_generation.rs`, `ranking.rs`, `budget.rs`, and `failure_handling.rs`.
3. All systems use `Permille` and integer types (no floats) — confirmed.
4. `DemandMemory` has aging mechanism (`TradeDispositionProfile.demand_memory_retention_ticks`) — confirmed, this is a potential dampener for trade loops.
5. `BlockedIntentMemory` has expiration — confirmed, this dampens goal spirals.
6. `PlanningBudget` limits planning depth/width — confirmed via `budget.rs`.
7. The ticket’s original examples overstate some current Phase 2 loops:
   - Trade is still buyer-driven plus merchant restock preparation; seller-side `SellCommodity` compounding is deferred to S04.
   - Combat currently has incapacitation/death, natural clotting/recovery, and treatment gating, but not general weapon depletion as a dampener.
8. Not every AI stability mechanism is a Principle 10 dampener. `PlanningBudget`, beam width, and blocked-intent TTL are planner guardrails. They should be documented separately from physical world dampeners rather than misclassified as world-state mechanisms.

## Architecture Check

1. Analysis-first approach — audit all loops before any code changes. Code fixes only if undamped loops are discovered.
2. No backwards-compatibility shims — any fixes add new dampening mechanisms, not wrappers.
3. Distinguish two categories explicitly:
   - Physical/world dampeners required by Principle 10 for simulation feedback loops.
   - Planner guardrails that keep search and replan churn bounded but are not substitutes for world dampeners.

## What to Change

### 1. Audit Needs/Metabolism system

**Files to read**: `crates/worldwake-systems/src/needs.rs`, `crates/worldwake-systems/src/needs_actions.rs`

Investigate:
- Does need satisfaction create conditions that accelerate need growth? (eating -> energy -> activity -> faster hunger)
- Document dampeners: resource depletion (food consumed is gone), action duration (eating takes time), capacity limits, deprivation wound consequences.

### 2. Audit Production system

**Files to read**: `crates/worldwake-systems/src/production.rs`, `crates/worldwake-systems/src/production_actions.rs`

Investigate:
- Does production create conditions accelerating further production? (crafting tools -> faster crafting -> more tools)
- Document dampeners: raw material depletion, workstation occupancy, action duration, storage/load limits (`LoadUnits`, container capacity).

### 3. Audit Trade system

**Files to read**: `crates/worldwake-systems/src/trade.rs`, `crates/worldwake-systems/src/trade_actions.rs`

Investigate:
- Current Phase 2 loop shape, not the deferred S04 seller loop: remembered unmet demand -> restock/move cargo behavior -> stock placed at destination -> restock gap closes.
- Document dampeners: demand-memory aging (`demand_memory_retention_ticks`), stock-at-destination closing the gap, inventory/load limits, travel time between places, and commodity/coin conservation.

### 4. Audit Combat system

**Files to read**: `crates/worldwake-systems/src/combat.rs`

Investigate:
- Does combat create conditions for more combat? (wounds -> vulnerability -> more attacks -> more wounds)
- Document dampeners actually present in code: wound incapacitation, death, natural clotting, natural recovery when basic needs are below high thresholds, same-place/duration constraints, and medicine-gated healing. Do not assume generic weapon depletion if the code does not model it.

### 5. Audit AI enterprise and planning stability

**Files to read**: `crates/worldwake-ai/src/enterprise.rs`, `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/ranking.rs`, `crates/worldwake-ai/src/budget.rs`, `crates/worldwake-ai/src/failure_handling.rs`

Investigate:
- Which enterprise loops are real world-state feedback loops, and which are planner churn risks?
- Document physical dampeners for the world-facing enterprise loop: demand memory aging, destination-local stock closing restock gaps, carry/load limits, and travel time.
- Document planner guardrails separately: `PlanningBudget`, beam width, switch margin, and `BlockedIntentMemory` expiration.

### 6. Document findings

Create `docs/dampening-audit-phase2.md` with:
- Per-system section listing all identified amplifying loops.
- For each simulation loop: the physical dampener mechanism (not numerical clamps).
- A short planner-guardrails section where applicable, clearly marked as non-physical safeguards.
- Cross-system interactions that could amplify.
- Any undamped loops requiring code fixes.

### 7. Fix undamped loops (if any)

If the audit reveals loops with no physical dampener (only numerical clamps), add concrete dampening mechanisms through physical world processes.

## Files to Touch

- `docs/dampening-audit-phase2.md` (new — audit document)
- `tickets/FND02-004-dampening-audit-phase2.md` (update assumptions/scope before implementation)
- `crates/worldwake-systems/src/needs.rs` (read for audit; modify only if undamped loop found)
- `crates/worldwake-systems/src/needs_actions.rs` (read for audit)
- `crates/worldwake-systems/src/production.rs` (read for audit; modify only if undamped loop found)
- `crates/worldwake-systems/src/production_actions.rs` (read for audit)
- `crates/worldwake-systems/src/trade.rs` (read for audit; modify only if undamped loop found)
- `crates/worldwake-systems/src/trade_actions.rs` (read for audit)
- `crates/worldwake-systems/src/combat.rs` (read for audit; modify only if undamped loop found)
- `crates/worldwake-ai/src/enterprise.rs` (read for audit; modify only if undamped loop found)
- `crates/worldwake-ai/src/candidate_generation.rs` (read for audit)
- `crates/worldwake-ai/src/ranking.rs` (read for audit)
- `crates/worldwake-ai/src/budget.rs` (read for audit)
- `crates/worldwake-ai/src/failure_handling.rs` (read for audit)

## Out of Scope

- Do NOT restructure any Phase 2 systems.
- Do NOT refactor code for style or performance — only add dampening mechanisms if missing.
- Do NOT audit Phase 1 systems (E01-E08) — they are stable and not in scope.
- Do NOT add new systems or components — only document existing behavior and fix gaps.
- Do NOT retrofit planner-only guardrails into fake world-state mechanics just to satisfy the audit.

## Acceptance Criteria

### Tests That Must Pass

1. `docs/dampening-audit-phase2.md` exists and covers all five Phase 2 system domains.
2. Each identified amplifying loop has a documented physical dampener (not a numerical clamp).
3. No undamped loops remain after any code fixes.
4. Relevant system and golden test suites covering needs, production, trade, combat, and AI decision logic are run and pass.
5. `cargo test --workspace` passes.
6. `cargo clippy --workspace` passes.

### Invariants

1. No new numerical-only clamps (`min`, `max`, `clamp`) introduced as dampeners — all dampeners must be physical world mechanisms.
2. Planner guardrails may be documented, but they do not count as substitutes for physical dampeners.
3. Existing system behavior preserved unless explicitly adding a dampener.
4. Determinism maintained — no `HashMap`, `HashSet`, `f32`, `f64` introduced.
5. Conservation invariants remain intact.

## Test Plan

### New/Modified Tests

1. If undamped loops are found and fixed: add regression tests proving the dampener limits amplification.
2. If the audit only changes documentation/ticket scope, no new tests are required; verification comes from existing relevant suites plus workspace-wide test/lint passes.

### Commands

1. `cargo test -p worldwake-systems` — targeted verification for needs/production/trade/combat
2. `cargo test -p worldwake-ai` — targeted verification for candidate generation, ranking, and golden AI scenarios
3. `cargo test --workspace` — final regression pass
4. `cargo clippy --workspace` — lint check

## Outcome

- Completed: 2026-03-14
- What actually changed:
  - Reassessed and corrected the ticket’s assumptions before implementation.
  - Added [docs/dampening-audit-phase2.md](/home/joeloverbeck/projects/worldwake/docs/dampening-audit-phase2.md) documenting the implemented Phase 2 feedback loops and their dampeners.
  - Expanded the AI enterprise audit scope to include `candidate_generation.rs`, `ranking.rs`, `budget.rs`, and `failure_handling.rs`, because the implemented dampening/guardrail architecture is distributed across those modules.
- Deviations from original plan:
  - No engine code changes were made, because the audit did not find an undamped implemented simulation loop that justified architectural churn.
  - The ticket’s original trade example was corrected to match current Phase 2 reality: buyer-driven trade plus merchant restock support, not seller-side `SellCommodity` compounding.
  - Planner guardrails were documented separately from physical dampeners instead of being treated as substitutes for Principle 10 world mechanisms.
- Verification results:
  - `cargo test -p worldwake-systems` passed.
  - `cargo test -p worldwake-ai` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
