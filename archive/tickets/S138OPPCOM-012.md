# S138OPPCOM-012: Align opportunity compiler load accounting with result cap

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `crates/worldwake-ai/src/opportunity_compiler/compile.rs` load accounting and matching golden regression assertion.
**Deps**: `archive/tickets/S138OPPCOM-010.md`, `archive/specs/S138-opportunity-compiler.md`

## Problem

Before this ticket, `cargo test -p worldwake-ai --test golden_opportunity_compiler survival_baseline_replay_is_deterministic_and_compiler_load_is_bounded -- --exact` failed with:

```text
compiled opportunities per tick exceeded default compile_opportunity_cap; max=23
```

The failing assertion in `crates/worldwake-ai/tests/golden_opportunity_compiler.rs::survival_baseline_replay_is_deterministic_and_compiler_load_is_bounded` compares `OpportunityCompilerLoad.compiled_count` to the default `CognitiveProfile.compile_opportunity_cap` of `16`. The live compiler increments `compiled_count` before truncating the `opportunities` vector to the cap in `crates/worldwake-ai/src/opportunity_compiler/compile.rs`, so the load counter can exceed the emitted result length even when the returned opportunity vector is correctly capped.

That left the S138 performance guard ambiguous: either `compiled_count` meant pre-cap candidates considered, making result length or `cap_truncated` the correct golden target instead, or it meant post-cap emitted opportunities, making post-truncation accounting the required compiler behavior.

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

## Verified Layers

1. Result cap contract -> focused unit coverage in `crates/worldwake-ai/src/opportunity_compiler/compile.rs` proves returned `Vec<Opportunity>` length stays at `compile_opportunity_cap`, `compiled_count` records post-cap emitted opportunities, and `cap_truncated` records overflow.
2. Load counter semantics -> `OpportunityCompilerLoad` field docs and the focused cap-overflow assertion make `compiled_count` the emitted post-cap opportunity count.
3. Deterministic replay guard -> `cargo test -p worldwake-ai --test golden_opportunity_compiler survival_baseline_replay_is_deterministic_and_compiler_load_is_bounded -- --exact` passed after the counter alignment.
4. Full affected surface -> `cargo test -p worldwake-ai --test golden_opportunity_compiler` and `cargo test -p worldwake-ai` passed.

## Landed Changes

### 1. Encoded `compiled_count` semantics

`OpportunityCompilerLoad.compiled_count` now means post-cap emitted opportunity count. `cap_truncated` remains the overflow diagnostic for viable opportunities dropped by the per-agent cap.

`crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` did not require a code change because it reads the field generically into the `opportunity_compiled_count` percentile bucket; the producer contract changed at the source and remains paired with `opportunity_cap_truncated`.

### 2. Strengthened the cap regression

The existing cap-overflow fixture in `crates/worldwake-ai/src/opportunity_compiler/compile.rs` now asserts `load.compiled_count == opportunities.len()` for a capped result and still asserts `cap_truncated` for overflow.

## Landed Files

- `crates/worldwake-ai/src/opportunity_compiler/compile.rs`
- `crates/worldwake-ai/src/decision_trace.rs`
- `archive/tickets/S138OPPCOM-012.md`

No change was needed in `crates/worldwake-ai/tests/golden_opportunity_compiler.rs`; the existing failing golden assertion became truthful once the producer counter matched the result cap. No change was needed in `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`; it consumes the corrected counter and the existing truncation bucket.

## Out of Scope

- Changing default `CognitiveProfile.compile_opportunity_cap`.
- Changing opportunity salience, risk, legality, or learned-memory damping behavior.
- Changing S148 portfolio, feasibility, or planner-search admission.

## Acceptance Result

### Tests Passed

1. `cargo test -p worldwake-ai --test golden_opportunity_compiler survival_baseline_replay_is_deterministic_and_compiler_load_is_bounded -- --exact`
2. `cargo test -p worldwake-ai --test golden_opportunity_compiler`
3. `cargo test -p worldwake-ai`

### Invariants Verified

1. `compile_opportunities` never returns more than `CognitiveProfile.compile_opportunity_cap` opportunities for an agent tick.
2. `OpportunityCompilerLoad` counters are internally consistent: `compiled_count` is emitted post-cap count, and `cap_truncated` is the count of viable opportunities dropped by cap truncation.
3. The deterministic replay performance guard remains counter-based, not wall-clock-based.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-ai/src/opportunity_compiler/compile.rs` — focused cap-overflow fixture proves result length, `compiled_count`, and `cap_truncated` semantics.
2. `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` — unchanged; the deterministic survival-baseline replay guard now passes against the corrected producer semantics.

## Outcome

Completed on 2026-05-18.

- Aligned `OpportunityCompilerLoad.compiled_count` with the capped returned `Vec<Opportunity>` by setting it after `opportunities.truncate(cap)`.
- Kept pre-cap overflow represented by `OpportunityCompilerLoad.cap_truncated` rather than adding a duplicate diagnostic counter or raising the golden threshold.
- Documented the load-counter fields on `OpportunityCompilerLoad`.

## Deviations

- `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` did not need an assertion edit because the existing replay guard already asserted the intended emitted-count cap once producer accounting was corrected.
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` did not need an edit because it consumes the corrected `compiled_count` and existing `cap_truncated` fields generically.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib opportunity_compiler::compile::tests::compile_opportunities_applies_floor_damping_and_cap -- --exact`.
- Passed `cargo test -p worldwake-ai --test golden_opportunity_compiler survival_baseline_replay_is_deterministic_and_compiler_load_is_bounded -- --exact`.
- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-ai --test golden_opportunity_compiler`.
- Passed `cargo test -p worldwake-ai`.
