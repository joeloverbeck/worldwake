# S147HTNMETDEC-013: Autonomous HTN method trace propagation and full D10 goldens

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — likely updates candidate evidence propagation or strategic method integration, plus golden coverage.
**Deps**: `archive/tickets/S147HTNMETDEC-011.md` (first stable HTN golden seam), `archive/tickets/S147HTNMETDEC-008.md` (planner integration), `archive/tickets/S147HTNMETDEC-009.md` (trace + diagnostics), `archive/tickets/S147HTNMETDEC-012.md` (recipe-input method preconditions)

## Problem

`S147HTNMETDEC-011` added the first `golden_htn_methods.rs` owner for the stable live seam: selector-level `ProduceWithGather` proof and agent-tick flat fallback when produce methods are disabled. During that implementation, a focused autonomous production probe showed that a generated `ProduceCommodity` planning attempt still records `method_trace == None` even when the actor knows a remote resource source; the selector only chooses `ProduceWithGather` when the `GoalOffer` evidence places explicitly include the resource-source place.

The remaining S147 D10 contract is therefore not just "write more goldens." The live candidate/evidence-to-method boundary must first be audited so autonomous generated candidates can lawfully carry the evidence needed for method selection and trace recording. After that boundary is truthful, the full D10 golden set can cover autonomous method trace propagation and the bounty/investigation/escort/failure narratives without fixture distortion.

## Assumption Reassessment (2026-05-17)

1. `crates/worldwake-ai/tests/golden_htn_methods.rs` currently proves two scenario blocks: selector chooses `MethodSchemaId(5)` from a belief-backed `GoalOffer`, and disabled produce methods fall back to flat strategic search with `method_trace == None`.
2. The live contradiction is at the shared boundary between generated `GoalOffer` evidence (`candidate_generation.rs`) and `htn::selector` preconditions consumed by `search/strategic.rs`. Reassessment must name exactly which generated goal families omit method-required evidence places or entities before changing code.
3. The first failure boundary observed by ticket 011 is autonomous `ProduceCommodity`: the direct selector path succeeds, while the agent-tick generated planning attempt falls back flat.
4. Full bounty, investigation, escort, and typed method-failure goldens should not be authored until the candidate/evidence bridge is proven for at least the production method family. Otherwise the scenarios risk proving hand-constructed `GoalOffer` fixtures rather than autonomous planning.
5. Adjacent contradictions should be classified separately: missing candidate evidence propagation is in scope; missing lower-level action materialization, absent legal setup for bounty claims, or unstable witness/escort action lifecycle should become separate follow-ups if they are not required to prove the S147 method trace bridge.

## Architecture Check

1. The clean target is one canonical evidence path: generated candidates must carry the same lawful belief/evidence places that `MethodSelector` needs, rather than giving method tests a parallel hand-authored `GoalOffer` substrate.
2. Method selection must remain belief-only and actor-relative. Do not add omniscient world reads to make autonomous methods pass.
3. No backwards-compatibility shim: if generated candidates are missing required evidence, update the candidate/evidence contract or method precondition bridge directly.

## Verification Layers

1. Autonomous production method trace -> agent-tick decision trace (`PlanAttemptTrace.method_trace.method_id == Some(MethodSchemaId(5))`) for a generated `ProduceCommodity` candidate.
2. Evidence propagation -> focused lower-layer test over generated `GoalOffer.evidence_places` / `evidence_entities` for the method-required source.
3. Flat fallback remains lawful -> existing `golden_htn_methods.rs` disabled-method scenario still passes.
4. Full D10 narratives -> golden tests only after the production bridge proves the autonomous method path.
5. Typed method failure -> event-log or discrepancy-memory assertion only if the live method execution path actually reaches a method failure boundary; otherwise split to a narrower follow-up.

## What to Change

### 1. Audit and fix the autonomous evidence bridge

Trace the `ProduceCommodity` candidate emitted in the ticket 011 fixture from candidate generation through `search/strategic.rs::plan_with_budget_trace`. Ensure the method selector sees the same resource-source evidence that direct selector tests see, without bypassing belief locality.

### 2. Extend `golden_htn_methods.rs`

Add an autonomous method-trace scenario once the bridge is fixed. Then add as much of S147 D10's original bounty/investigation/escort/failure coverage as the live action substrate can prove honestly.

### 3. Truth-sync S147 spec and implementation order if needed

If reassessment shows the full six-narrative D10 contract needs further splitting, update `specs/S147-htn-method-decomposition.md` and `specs/IMPLEMENTATION-ORDER.md` so they describe the staged validation path truthfully.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (likely — evidence propagation audit/fix)
- `crates/worldwake-ai/src/search/strategic.rs` (possible — method integration audit)
- `crates/worldwake-ai/tests/golden_htn_methods.rs` (modify)
- `docs/generated/golden-e2e-inventory.md` (regenerated if golden metadata changes)
- `docs/generated/golden-scenario-index.md` (regenerated if golden metadata changes)
- `docs/generated/golden-scenario-details/` (regenerated if golden metadata changes)
- `docs/generated/golden-coverage-matrix.md` (regenerated if golden metadata changes)
- `specs/S147-htn-method-decomposition.md` (possible truth-sync)
- `specs/IMPLEMENTATION-ORDER.md` (possible truth-sync)

## Out of Scope

- Per-method observer formatting already completed in `archive/tickets/S147HTNMETDEC-010.md`.
- Adding new `PlannerOpKind` variants.
- Introducing story-beat methods or method-only goals.
- Performance regression gates.

## Acceptance Criteria

### Tests That Must Pass

1. Autonomous `ProduceCommodity` method trace records `MethodSchemaId(5)` from generated candidate evidence.
2. Disabled-method flat fallback remains covered by `golden_htn_methods.rs`.
3. `python3 scripts/golden_inventory.py --write --check-docs` passes after golden metadata changes.
4. `cargo test -p worldwake-ai --test golden_htn_methods` passes.
5. `cargo test -p worldwake-ai` passes.

### Invariants

1. Method selection remains belief-only and actor-relative.
2. Generated candidates carry lawful evidence; methods do not query global world state to compensate.
3. Flat GOAP fallback remains available when methods are disabled or no method preconditions match.
4. Any incomplete original D10 narrative is assigned to a named follow-up rather than silently dropped.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_htn_methods.rs` — extend with autonomous method-trace coverage and any stable full-D10 scenarios.
2. Focused lower-layer test near the evidence producer if candidate evidence propagation changes.

### Commands

1. `cargo test -p worldwake-ai --test golden_htn_methods`
2. `python3 scripts/golden_inventory.py --write --check-docs`
3. `cargo test -p worldwake-ai`
