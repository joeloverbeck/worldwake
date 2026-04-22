# S115AGEMAN-007: observer agenda-state rendering

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — observer report rendering + ai runtime read accessor for tooling/tests.
**Deps**: [archive/tickets/S115AGEMAN-005](./S115AGEMAN-005.md), [archive/tickets/S115AGEMAN-008](./S115AGEMAN-008.md)

## Problem

`AgendaState` landed in the AI runtime, but the observer dump still had no direct summary of committed/pending/suspended agenda state. Spec S115 D5's debuggability claim therefore remained unproved at the observer surface. The drafted purchase-revival golden slice was reassessed separately and moved into follow-up ticket `S115AGEMAN-008` after live evidence showed the current branch keeps the goal committed instead of parking it in `pending` when the merchant departs from the locally bound trade seam.

## Assumption Reassessment (2026-04-22)

1. The shared boundary under audit is `worldwake_ai::AgentDecisionRuntime.agenda_state` as read by `crates/worldwake-cli/src/bin/observer.rs`. The live observer had decision-history rendering but no agenda-state section.
2. The observer can lawfully read AI runtime state directly through `AgentTickDriver`; no new sim-layer accessor or trait is needed. The landed change adds a small read-only runtime accessor on `AgentTickDriver`.
3. The cargo `Suspended` lifecycle seam is already covered by focused runtime proof in `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying`; the live missing slice here was observer rendering, not a second cargo golden.
4. The drafted new golden/scenario plan drifted. Existing `crates/worldwake-ai/tests/golden_merchant_selling.rs::remote_branch_selection_reaches_local_trade_binding_before_merchant_departure` already owns the earlier merchant-purchase golden substrate.
5. Focused live repro disproved the drafted purchase-revival golden premise on the current branch. Running the buyer to the real local trade-binding seam, then moving the merchant away, left `runtime.agenda_state.committed` populated and never parked the goal into `pending` across ticks 10-49. That contradiction is now recorded in archived ticket `S115AGEMAN-008`.
6. The observer surface can truthfully render pending counts, suspended counts, pending revival triggers, and suspended kill conditions from the live `AgendaState` shape. It cannot truthfully render a distinct suspended "reason label" because that data is not stored on `AgendaEntry`.
7. Correction applied: ticket says "new golden_agenda_lifecycle scenario"; live code has an already-covered cargo runtime seam plus a disproved purchase-revival golden premise; correction is to narrow this ticket to observer agenda-state rendering and move the production/golden remainder to `S115AGEMAN-008`.

## Architecture Check

1. Reading `AgendaState` through `AgentTickDriver` keeps the observer on the existing AI-runtime boundary instead of inventing a second carrier or a CLI-local cache.
2. The rendering uses the real stored agenda data (`committed`, `pending`, `suspended`, `RevivalTrigger`, `KillCondition`) and does not infer fields the runtime does not carry.

## Verification Layers

1. Observer dump includes committed/pending/suspended agenda summary -> focused `observer.rs` unit test through `format_report`.
2. Pending entries render revival-trigger details from live `RevivalTrigger` variants -> focused `observer.rs` unit test.
3. Suspended entries render stored kill-condition details from live `KillCondition` variants -> focused `observer.rs` unit test.
4. Existing merchant-purchase golden substrate still reaches the local trade-binding seam -> existing `golden_merchant_selling` focused golden.
5. Drafted purchase-revival golden premise is false on the live branch -> focused golden repro recorded in follow-up `S115AGEMAN-008`, not re-expressed as a passing test here.

## What to Change

### 1. Observer agenda-state section

Extend `crates/worldwake-cli/src/bin/observer.rs` so Section 8 renders:
- committed goal summary
- pending count and pending goal lines with revival-trigger text
- suspended count and suspended goal lines with kill-condition text

### 2. AI runtime read accessor

Expose a narrow read-only runtime accessor on `AgentTickDriver` so observer/report tooling and unit tests can read per-agent `AgentDecisionRuntime` without reconstructing internal state.

### 3. Focused observer proof

Add a unit test in `observer.rs` that seeds `AgendaState` through `AgentTickDriver` and asserts the rendered report includes committed/pending/suspended agenda details.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — read/runtime seeding helpers for tooling/tests)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — render agenda-state summary + focused test)
- `tickets/S115AGEMAN-007.md` (modify — truthful closeout)
- `archive/tickets/S115AGEMAN-008.md` (follow-up ticket that resolved the disproved golden / production seam)

## Out of Scope

- New purchase-revival golden coverage on the current branch
- Production agenda-manager behavior changes needed to demote the locally bound purchase goal into `pending`
- Golden inventory regeneration (no new golden scenario landed)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli --bin observer tests::format_report_renders_agenda_state_summary -- --exact`
2. `cargo test -p worldwake-cli --bin observer`
3. Existing seam check: `cargo test -p worldwake-ai --test golden_merchant_selling remote_branch_selection_reaches_local_trade_binding_before_merchant_departure -- --exact`

### Invariants

1. The observer dump exposes live `AgendaState` for each agent without inventing new state carriers.
2. Pending entries show revival-trigger text sourced from `RevivalTrigger`.
3. Suspended entries show kill-condition text sourced from `KillCondition`.
4. The ticket does not claim a passing purchase-revival golden while that premise remains false on the current branch.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` — adds `format_report_renders_agenda_state_summary` to prove committed/pending/suspended rendering.
2. `None` — purchase-revival golden work moved to `S115AGEMAN-008` after focused repro showed a live production contradiction.

## Outcome

Completed on 2026-04-22.

- Added `AgentTickDriver::runtime()` so the observer can read live per-agent agenda state.
- Added observer Section 8 agenda rendering for committed/pending/suspended entries, including pending revival-trigger text and suspended kill-condition text.
- Added focused observer coverage for the new rendering.
- Reassessed and removed the drafted golden/scenario plan from this ticket after focused live evidence showed the purchase goal stays committed instead of parking to `pending` when the merchant departs from the locally bound trade seam.

## Deviations

- The drafted "reason label" rendering for suspended entries was narrowed to kill-condition rendering because `AgendaEntry` does not store a distinct suspended reason label on the live branch.
- The purchase-revival golden slice did not land here; it is isolated as follow-up ticket `S115AGEMAN-008`.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer tests::format_report_renders_agenda_state_summary -- --exact`
- Passed `cargo test -p worldwake-cli --bin observer`
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling remote_branch_selection_reaches_local_trade_binding_before_merchant_departure -- --exact`
