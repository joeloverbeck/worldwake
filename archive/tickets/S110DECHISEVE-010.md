# S110DECHISEVE-010: Richer repair-kind provenance for decision events

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — add authoritative repair-acceptance provenance transport from failed plan capture through plan adoption and completion-time emission
**Deps**: archive/tickets/S110DECHISEVE-009.md

## Problem

`S110DECHISEVE-009` lands only the currently provable `RepairKind::AlternateTarget` slice. The broader S110 repair taxonomy (`AlternateRoute`, `AlternateMerchant`, `AlternateRecipe`) is still not represented by an authoritative runtime carrier, so those variants cannot yet be emitted honestly.

## Assumption Reassessment (2026-04-20)

1. The live durable repair substrate is still `RepairKey { goal_key, alternate_target }` in `crates/worldwake-core/src/repair_memory.rs`, which proves only alternate-target success for ranking/memory purposes.
2. The authoritative event-log problem is broader than repair memory: the runtime needs an explicit accepted-repair carrier from failed plan capture through replacement-plan adoption to successful completion.
3. `crates/worldwake-ai/src/agent_tick/active_action.rs::handle_current_step_failure` is the honest failed-plan capture seam, `crates/worldwake-ai/src/agent_tick/planning.rs::adopt_selected_plan` is the honest repair-acceptance seam, and `crates/worldwake-ai/src/agent_tick/mod.rs` remains the honest completion/event-emission seam.
4. Route, merchant, and recipe repairs must be classified from concrete failed-plan vs accepted-plan structure at adoption time, not inferred later from repair memory, anchors alone, or decision traces.
5. `RepairMemory` may remain alternate-target-specific for now if ranking still only consumes that substrate; this ticket owns truthful decision-event provenance, not a broader learning-policy rewrite.

## Architecture Check

1. Adding explicit successful repair provenance is cleaner than inferring repair class from later memory or opportunity anchors.
2. The end state should preserve one authoritative carrier from repair acceptance to event emission, rather than parallel guessed classifications in ranking, memory, and the event log.
3. The classifier must prefer concrete semantic replacements over generic fallbacks when multiple plan differences are present. For example, a changed seller should emit `AlternateMerchant` rather than collapsing to `AlternateTarget`, and a changed recipe should emit `AlternateRecipe` rather than `AlternateTarget`.

## Verification Layers

1. Failed-plan capture -> focused unit/runtime coverage at the failure seam that records pending repair context.
2. Repair acceptance transport -> focused unit/runtime coverage at the plan-adoption seam that classifies accepted repairs.
3. Event emission -> focused `agent_tick` runtime tests proving each newly supported `RepairKind`.
4. If one drafted repair kind still lacks truthful proof after reassessment, narrow again instead of emitting a guessed variant.

## What to Change

### 1. Add runtime repair provenance carrier

Add an AI-local runtime carrier that:

- captures the failed plan and failed step at the failure seam
- classifies the accepted replacement at plan adoption time
- persists the accepted repair provenance until successful completion or replacement

### 2. Classify accepted repair kinds from concrete plan structure

Implement a classifier that compares the failed plan to the accepted replacement plan and emits only concrete kinds:

- `AlternateTarget` when the replacement uses a different grounded target/opportunity anchor for the same repaired intent
- `AlternateMerchant` when the replacement trade step switches to a different concrete counterparty
- `AlternateRecipe` when a `ProduceCommodity` repair switches to a different concrete recipe that still serves the same produced commodity
- `AlternateRoute` when the repaired plan keeps the same repaired intent/binding but changes the concrete travel sequence

### 3. Emit `RepairApplied` from accepted provenance

Replace the remaining completion-time alternate-target inference with the accepted repair-provenance carrier. Keep alternate-target repair-memory recording aligned with the same accepted provenance, and do not widen `RepairMemory` unless the implementation proves it is required for this ticket.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs`
- `crates/worldwake-ai/src/agent_tick/active_action.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/agent_tick/mod.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`
- `specs/S110-decision-history-events.md` (if emission notes or payload commentary need factual update)

## Out of Scope

- Replay coverage
- Observer rendering
- Widening `RepairMemory` or ranking heuristics beyond the alternate-target substrate unless the code change proves that dependency is necessary

## Acceptance Criteria

### Tests That Must Pass

1. Focused tests prove failed-plan capture, accepted repair classification, and completion-time emission for each newly supported repair class.
2. `cargo test -p worldwake-ai`

### Invariants

1. No `RepairApplied` variant is emitted from inferred post-hoc context alone.
2. Every emitted repair class is backed by an explicit accepted-repair carrier in live runtime state.
3. Completion-time emission clears or consumes the accepted repair carrier so stale repairs do not leak across later plans.

## Test Plan

### New/Modified Tests

1. Focused failure/adoption/emission tests for `AlternateTarget`, `AlternateMerchant`, `AlternateRecipe`, and `AlternateRoute`.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-20.

- Added AI-local repair provenance transport to `AgentDecisionRuntime` so failed plans are captured at the failure seam and accepted repairs are classified at plan adoption.
- Replaced the old completion-time alternate-target heuristic with accepted-repair-driven `RepairApplied` emission for `AlternateTarget`, `AlternateMerchant`, `AlternateRecipe`, and `AlternateRoute`.
- Kept `RepairMemory` alternate-target-specific; only accepted `AlternateTarget` repairs still record durable repair memory because that is the only live ranking substrate that currently consumes repair memory.
- Updated the active S110 spec notes so `RepairApplied` now documents the live failure-capture -> adoption-classification -> completion-emission seam.

## Verification Result

Passed on 2026-04-20.

1. `cargo test -p worldwake-ai classify_accepted_repair_prefers_alternate_merchant_over_anchor_change -- --exact`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
