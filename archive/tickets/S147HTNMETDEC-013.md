# S147HTNMETDEC-013: Autonomous HTN method trace propagation and full D10 goldens

**Status**: COMPLETED
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

## Verified Layers

1. Autonomous production method trace -> agent-tick decision trace (`PlanAttemptTrace.method_trace.method_id == Some(MethodSchemaId(5))`) for a generated `ProduceCommodity` candidate.
2. Evidence propagation -> focused lower-layer test over generated `GoalOffer.evidence_places` / `evidence_entities` for the method-required source.
3. Flat fallback remains lawful -> existing `golden_htn_methods.rs` disabled-method scenario still passes.
4. Full D10 narratives -> golden tests only after the production bridge proves the autonomous method path.
5. Typed method failure -> event-log or discrepancy-memory assertion only if the live method execution path actually reaches a method failure boundary; otherwise split to a narrower follow-up.

## Landed Changes

### 1. Autonomous evidence bridge audited and fixed

The generated `ProduceCommodity` offer already carried the resource-source place/entity evidence at the candidate-generation boundary. The missing bridge was snapshot-backed method precondition evaluation: `PlanningState` carried known recipe IDs but not recipe definitions, so `RecipeInput` method preconditions could not resolve inside strategic planning. `select_method_with_recipes(...)` now lets strategic planning pass the live `RecipeRegistry` into method precondition evaluation while keeping resource-source checks belief/evidence-backed.

### 2. `golden_htn_methods.rs` extended

Added autonomous generated-candidate coverage proving `ProduceCommodity` records `MethodSchemaId(5)` in `MethodPlanAttemptTrace`, plus snapshot-selector and generated-offer evidence witnesses. The existing disabled-method flat fallback remains covered.

### 3. S147 handoff truthed

The full non-production D10 narrative set remains active and is split to `tickets/S147HTNMETDEC-014.md`: `FulfillBountyDirect`, `FulfillBountyInvestigation`, escort/failure, and typed `Discrepancy::MethodFailure(MethodFailureContext)` goldens.

## Landed Files

- `crates/worldwake-ai/src/candidate_generation.rs` (focused generated-offer evidence test)
- `crates/worldwake-ai/src/htn/mod.rs` (export `select_method_with_recipes`)
- `crates/worldwake-ai/src/htn/selector.rs` (recipe-registry precondition bridge and evidence-backed resource-source lookup)
- `crates/worldwake-ai/src/search/strategic.rs` (method selector call passes the strategic recipe registry)
- `crates/worldwake-ai/tests/golden_htn_methods.rs` (autonomous production evidence, selector, trace, and replay coverage)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-scenario-details/htn-methods.md` (regenerated)
- `docs/generated/golden-coverage-matrix.md` (regenerated)
- `specs/S147-htn-method-decomposition.md` (truth-sync)
- `specs/IMPLEMENTATION-ORDER.md` (truth-sync)

## Out of Scope

- Per-method observer formatting already completed in `archive/tickets/S147HTNMETDEC-010.md`.
- Adding new `PlannerOpKind` variants.
- Introducing story-beat methods or method-only goals.
- Performance regression gates.

## Acceptance Result

### Passed Gates

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

## Verification Result

1. Passed: `cargo test -p worldwake-ai --lib remote_recipe_produce_goal_carries_input_source_place_evidence`
2. Passed: `cargo test -p worldwake-ai --lib htn::selector -- --nocapture`
3. Passed: `cargo test -p worldwake-ai --test golden_htn_methods`
4. Passed: `python3 scripts/golden_inventory.py --write --check-docs`
5. Passed: `cargo test -p worldwake-ai`

## Outcome

Completed: 2026-05-17

- Added snapshot-backed recipe-registry support to HTN method selection through `select_method_with_recipes(...)`, and wired strategic planning to pass its live `RecipeRegistry`.
- Kept method resource-source preconditions belief/evidence-backed by checking generated `GoalOffer` evidence entities and their believed places instead of adding global world reads.
- Added focused candidate-generation proof that remote recipe input source evidence is present on generated `ProduceCommodity` offers.
- Added autonomous `golden_htn_methods.rs` production scenarios for generated-offer evidence, snapshot-backed selection, method-trace recording, and deterministic replay.
- Regenerated golden inventory/docs for the new HTN scenario block.
- Truth-synced `specs/S147-htn-method-decomposition.md` and `specs/IMPLEMENTATION-ORDER.md`; remaining non-production D10 narratives are owned by `tickets/S147HTNMETDEC-014.md`.

Deviations:

- The ticket did not force the remaining bounty/investigation/escort/failure narratives into this implementation. Those require separate stable fixtures over their own action/legal/failure substrates and are split to `tickets/S147HTNMETDEC-014.md`.
