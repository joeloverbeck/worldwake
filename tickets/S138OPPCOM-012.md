# S138OPPCOM-012: Align opportunity compiler load accounting with result cap

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `crates/worldwake-ai/src/opportunity_compiler/compile.rs` load accounting and matching golden regression assertion.
**Deps**: `archive/tickets/S138OPPCOM-010.md`, `archive/specs/S138-opportunity-compiler.md`

## Problem

`cargo test -p worldwake-ai --test golden_opportunity_compiler survival_baseline_replay_is_deterministic_and_compiler_load_is_bounded -- --exact` currently fails with:

```text
compiled opportunities per tick should stay within default compile_opportunity_cap; max=23
```

The failing assertion in `crates/worldwake-ai/tests/golden_opportunity_compiler.rs::survival_baseline_replay_is_deterministic_and_compiler_load_is_bounded` compares `OpportunityCompilerLoad.compiled_count` to the default `CognitiveProfile.compile_opportunity_cap` of `16`. The live compiler increments `compiled_count` before truncating the `opportunities` vector to the cap in `crates/worldwake-ai/src/opportunity_compiler/compile.rs`, so the load counter can exceed the emitted result length even when the returned opportunity vector is correctly capped.

This leaves the S138 performance guard ambiguous: either `compiled_count` is intended to mean pre-cap candidates considered, in which case the golden should assert result length or `cap_truncated` instead, or it is intended to mean post-cap emitted opportunities, in which case the compiler must move the counter after truncation.

## Assumption Reassessment (2026-05-18)

1. Active ticket scan found no current owner in `tickets/` for `golden_opportunity_compiler`, `OpportunityCompilerLoad.compiled_count`, or `compile_opportunity_cap`; the relevant owners are archived S138 tickets.
2. `archive/specs/S138-opportunity-compiler.md` specifies that `CognitiveProfile.compile_opportunity_cap` caps the result length of `compile_opportunities` per tick per agent, and that `OpportunityCompilerLoad` records per-tick load for inspection.
3. `archive/tickets/S138OPPCOM-010.md` landed the deterministic replay/performance guard and describes it as a compiled-count ceiling under the default replay.
4. The live failing assertion is in `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` and uses `load.compiled_count`; the live compiler increments `load.compiled_count` before `opportunities.truncate(cap)`.
5. Shared abstraction boundary under audit: `compile_opportunities(...) -> (Vec<Opportunity>, OpportunityCompilerLoad)`, specifically the meaning of `OpportunityCompilerLoad.compiled_count` relative to the returned capped `Vec<Opportunity>` and `cap_truncated`.
6. The intended invariant is deterministic bounded opportunity compiler work under the default survival-baseline replay, without wall-clock timing or hidden global state reads.
7. This is not owned by `archive/tickets/S148PORMOTBAC-FOLLOWUP-001.md`: that ticket repaired feasibility/search admission for stale pursuit and self-care acquisition and did not touch opportunity compiler accounting.

## Architecture Check

1. The fix should make the counter contract explicit instead of only raising the golden threshold. A higher magic bound would hide whether the cap applies to returned opportunities, pre-cap candidates, or both.
2. The result must preserve FND-12: performance compression and instrumentation may change how work is reported, never what opportunity facts the agent can lawfully observe or act on.
3. No backwards-compatibility aliases or duplicate counters should be added unless the review proves both pre-cap and post-cap counts are independently necessary diagnostics.

## Verification Layers

1. Result cap contract -> focused unit coverage in `crates/worldwake-ai/src/opportunity_compiler/compile.rs` proving returned `Vec<Opportunity>` length stays at `compile_opportunity_cap` and `cap_truncated` records overflow.
2. Load counter semantics -> focused unit or golden assertion proving `compiled_count` means the chosen contract, either post-cap emitted opportunities or pre-cap candidates considered.
3. Deterministic replay guard -> `cargo test -p worldwake-ai --test golden_opportunity_compiler survival_baseline_replay_is_deterministic_and_compiler_load_is_bounded -- --exact`.
4. Full affected surface -> `cargo test -p worldwake-ai --test golden_opportunity_compiler`.

## What to Change

### 1. Decide and encode `compiled_count` semantics

Review `OpportunityCompilerLoad` consumers in `decision_trace.rs`, `scenario_diagnostics`, and `golden_opportunity_compiler.rs`. Choose one explicit meaning:

- post-cap emitted opportunity count, with pre-cap overflow represented by `cap_truncated`; or
- pre-cap candidate count, with the golden bound checking the returned opportunity count or `compiled_count - cap_truncated`.

Update code and assertions to match that single meaning.

### 2. Strengthen the cap regression

Add or adjust focused coverage so a fixture with more than `compile_opportunity_cap` viable opportunities proves the returned vector is capped and the load counters remain internally consistent.

## Files to Touch

- `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (modify)
- `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` (modify)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (modify only if the counter meaning changes for diagnostics)

## Out of Scope

- Changing default `CognitiveProfile.compile_opportunity_cap`.
- Changing opportunity salience, risk, legality, or learned-memory damping behavior.
- Changing S148 portfolio, feasibility, or planner-search admission.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_opportunity_compiler survival_baseline_replay_is_deterministic_and_compiler_load_is_bounded -- --exact`
2. `cargo test -p worldwake-ai --test golden_opportunity_compiler`
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `compile_opportunities` never returns more than `CognitiveProfile.compile_opportunity_cap` opportunities for an agent tick.
2. `OpportunityCompilerLoad` counters are internally consistent and documented by executable assertions.
3. The deterministic replay performance guard remains counter-based, not wall-clock-based.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/opportunity_compiler/compile.rs` — focused cap-overflow fixture proving result length, `compiled_count`, and `cap_truncated` semantics.
2. `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` — deterministic survival-baseline replay guard aligned with the finalized counter contract.

### Commands

1. `cargo test -p worldwake-ai --test golden_opportunity_compiler survival_baseline_replay_is_deterministic_and_compiler_load_is_bounded -- --exact`
2. `cargo test -p worldwake-ai --test golden_opportunity_compiler`
3. `cargo test -p worldwake-ai`
