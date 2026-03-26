# AITRACE-002: Planner/runtime legality provenance in traces

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — decision/action trace payloads, planner candidate provenance, runtime start-failure provenance
**Deps**: [archive/tickets/E17CRITHEJUS-013.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E17CRITHEJUS-013.md), [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md), [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md), [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md)

## Problem

Mixed-layer AI failures are still harder to explain than they should be. The current trace surfaces are strong at showing:

1. whether a goal was generated or selected
2. whether an action started, committed, or aborted

But they are weaker at showing the concrete shared fact that made the planner believe a branch was lawful when runtime later rejected or aborted it. That gap violates the debug surface expected by `FOUNDATIONS.md` Principle 27 and weakens the separation between belief-state reasoning and authoritative legality from Principles 12, 24, and 25.

The E17 justice chain exposed the exact missing surface: the planner selected `Fine` from a quantity read that looked lawful in traces, but only lower-layer code inspection revealed that the belief helper was overstating locally collectible commodity. The product should make that contradiction inspectable from trace data.

## Assumption Reassessment (2026-03-27)

1. Current trace surfaces already cover outcome-level reasoning well:
   - decision traces in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
   - action traces in [`crates/worldwake-sim/src/action_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs)
   - search root diagnostics in [`crates/worldwake-ai/src/search/candidates.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs)
2. The missing surface is not “was the branch present?” but “which concrete belief/runtime read made it appear lawful?” That is a different traceability boundary than candidate omission or start/abort lifecycle.
3. Shared abstraction boundary under audit: planner candidate legality and runtime start legality for the same exact branch, especially where both rely on belief-view helpers or snapshot-backed quantitative reads.
4. Intended invariant: if planner and runtime disagree about whether a branch is lawful, the causal reason for the disagreement must be reconstructable from trace/state inspection without ad hoc debug output.
5. Live goal/operator surfaces most likely to need this are exact-bound or legality-sensitive branches such as:
   - `GoalKind::PunishAccused` via `candidate_generation` and punishment start validators
   - `GoalKind::Accuse` via exact-target affordance reproduction and start validation
   - resource/cargo/trade branches that depend on quantity and control reads
6. This is a mixed-layer AI ticket. The intended layer is neither candidate generation alone nor golden E2E alone; it spans candidate generation, request/start, and trace diagnostics. Full action registries and at least one golden-style regression are required.
7. The current docs already acknowledge a weaker version of this gap:
   - [`docs/golden-e2e-testing.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) says to drop to lower-layer tests when traces do not expose enough provenance
   - [`docs/planner-contracts.md`](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) documents root omission and duration-skip diagnostics, but not planner/runtime legality mismatches after a branch is surfaced
8. This ticket is not asking for debug-only prints or compatibility shims. Under Principle 26, any new provenance surface must become part of the live clean trace model, not an auxiliary workaround.
9. Adjacent contradiction classification:
   - required consequence in scope: missing trace data for planner/runtime legality disagreements
   - out of scope: changing substantive planner legality rules or runtime action validation semantics unless a missing trace payload cannot be emitted without a small supporting refactor
10. The relevant arithmetic/data contract is concrete and local, not abstract. This ticket should expose exact quantities, target identities, helper origin, or authoritative mismatch facts, not fuzzy “confidence” summaries. That is required by Principles 3 and 25.

## Architecture Check

1. The clean architecture is to improve the shared trace data model where planner legality and runtime legality meet, rather than adding local logs in individual systems. That keeps debugability a first-class product surface under Principle 27.
2. This is cleaner than sprinkling `eprintln!` diagnostics through tests or action handlers because the provenance remains structured, deterministic, and queryable.
3. This is cleaner than adding broad “debug mode” branches because the resulting trace artifacts remain ordinary state-adjacent observability, not a parallel development-only path.
4. No backward-compatibility aliasing or duplicate trace pipelines should be introduced. Existing trace sinks should gain the missing lawful data in place.

## Verification Layers

1. Planner candidate legality records the concrete read used for a selected branch -> focused decision-trace/unit coverage in `worldwake-ai`
2. Runtime start/abort records the authoritative fact that contradicted the planner-side legality assumption -> focused action/request-resolution trace coverage in `worldwake-sim` and/or `worldwake-systems`
3. Cross-layer contradiction is inspectable without ad hoc logging -> focused mixed-layer regression plus one golden/debugging-oriented proof surface
4. The ticket does not use durable downstream world-state as a proxy for legality disagreement; the contract is the earlier planning/start boundary itself
5. If traces still cannot explain a selected branch after the new payloads land, the strongest lower-layer proof surface remains the relevant belief-view or validator unit test; if that remains common after this ticket, it should spawn another traceability follow-up

## What to Change

### 1. Extend planner-side trace payloads for legality-sensitive candidate formation

Update decision-trace or candidate-trace structures so legality-sensitive branches can record the concrete planner-visible facts they used when those facts are decisive for branch choice.

Examples:
- exact observed quantity used for `Fine`
- target/register binding used for exact-bound branches
- helper/read surface category used to derive the fact

The payload must stay concrete and bounded. Do not add arbitrary free-form strings.

### 2. Extend runtime start/abort trace payloads for legality contradiction facts

When a request reaches authoritative start and fails or aborts for a legality reason that could contradict planner belief, emit structured contradiction data naming:

- the action/goal branch
- the specific authoritative fact that failed
- the concrete entity/place/quantity involved when relevant

This should be queryable from existing trace sinks rather than requiring test-local logging.

### 3. Add focused cross-layer regression coverage

Add focused tests that prove:

- a legality-sensitive branch records the planner-side fact it used
- the same branch records the runtime-side contradiction when start fails or aborts
- the two traces together explain the mismatch without source inspection

Prefer a justice-style or trade/resource-style branch where quantity/control legality is already known to be subtle.

### 4. Add one golden-oriented traceability proof

Add or update one golden or golden-harness regression that demonstrates the improved trace surface is sufficient to debug a mixed-layer disagreement without custom output.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/search/candidates.rs` (modify if root/selection provenance needs extension)
- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-sim/src/request_resolution_trace.rs` (modify if the contradiction boundary belongs there)
- `crates/worldwake-sim/src/tick_step.rs` (modify if trace wiring is needed)
- `crates/worldwake-ai/tests/` (modify/add focused + golden-facing coverage)
- `crates/worldwake-sim/src/` tests (modify/add focused trace coverage)

## Out of Scope

- Changing justice, trade, or transport legality rules themselves unless required to expose trace data cleanly
- Adding generic stringly debug logs
- Rewriting existing trace sinks wholesale
- Broad UI work for trace presentation

## Acceptance Criteria

### Tests That Must Pass

1. Focused test proving decision trace records concrete legality provenance for a selected candidate
2. Focused test proving runtime trace records the authoritative contradiction fact for the same branch
3. Focused mixed-layer regression proving the mismatch is explainable from trace data alone
4. Existing relevant AI and sim trace suites remain green
5. Existing suite: `cargo test --workspace`

### Invariants

1. Traceability remains concrete-state-based: entities, places, quantities, and named lawful facts, not abstract “scores” or human-only prose
2. The new provenance surface does not create a parallel authority path; it only explains existing lawful planner/runtime behavior

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` or related focused tests — prove planner-side legality provenance is recorded for a selected branch
2. `crates/worldwake-sim/src/action_trace.rs` or request/start trace tests — prove runtime contradiction provenance is recorded structurally
3. `crates/worldwake-ai/tests/golden_*.rs` — one regression proving the new traces make a mixed-layer disagreement inspectable without ad hoc output

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
