# S168PARPLASKE-004: Validation goldens (reuse + fallback)

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None — golden tests + replay equivalence checks. New scenario RON files; new test files in `crates/worldwake-ai/tests/`.
**Deps**: `archive/tickets/S168PARPLASKE-001.md` (`revalidate_skeleton_step`); `S168PARPLASKE-002` (populated `remaining_skeleton`); `S168PARPLASKE-003` (`search_plan_seeded` + `PartialPlanResumeTrace`); `specs/S168-partial-plan-skeleton-reuse.md` (Validation and Falsification section).

## Problem

S168's Validation and Falsification section (FND-31) declares four proof obligations the focused unit tests in tickets 001-003 do not satisfy:

1. **Golden (reuse)**: information-barrier suspend → lawful carrier satisfies barrier → skeleton revalidates → same pursuit resumes via seeded search; trace shows reuse.
2. **Golden (fallback)**: assumption goes stale before resume → reuse rejected → full replan; trace shows the invalidation reason.
3. **Replay/save-load equivalence**: both goldens replay identically; the budget-exhaustion save round-trips with the now-populated skeleton.
4. **No-regression**: survival/integration goldens unaffected (resume behavior is equivalent or strictly better-bounded).

The focused tests in 001-003 prove the function-level contracts: revalidation correctness, population correctness, resume routing, trace emit, seeded-search internal fallback. They do not prove the end-to-end causal chain — "an agent that suspended at an information barrier and saw the barrier satisfied resumes its remembered pursuit via seeded search and the world reaches the same lawful state as full replan would." That requires golden E2E coverage.

This ticket delivers both goldens, the replay/save-load equivalence checks, and a no-regression survival sweep.

## Assumption Reassessment (2026-05-24)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Codebase shape (golden harness)**. Verified:
   - Golden E2E tests live in `crates/worldwake-ai/tests/golden_ai/` (per CLAUDE.md's "post-S154 golden form" reference). Each scenario module path is filterable by substring.
   - Decision-trace assertions in goldens use the same `PartialPlanResumeTrace` emit point ticket 003 wires; verify the assertion API supports asserting on a trace variant + payload.
   - Existing info-barrier goldens (if any) — locate via `rg "InformationBarrier" crates/worldwake-ai/tests/` during ticket reassessment. The new goldens may parallel an existing info-barrier scenario shape.
2. **Spec/doc references**. S168 Validation and Falsification (`specs/S168-partial-plan-skeleton-reuse.md:279-291`). The reuse golden's narrative: "information-barrier suspend → lawful carrier satisfies barrier → skeleton revalidates → same pursuit resumes via seeded search with the trace showing revalidation." The fallback golden's narrative: "assumption goes stale before resume → reuse rejected → full replan, trace shows the invalidation reason."
3. **Cross-system boundary under audit (precision rule 2)**. The reuse golden spans: information-barrier suspension (agenda_manager), companion goal that brings the lawful carrier (existing S114/S149 machinery), barrier satisfaction (perception/belief layer), resume gate (`try_resume_partial_plan`), revalidation (ticket 001), seeded search (ticket 003), action execution. Each layer's contribution must be verifiable from the trace, not inferred from the final world state.
4. **Live `GoalKind` / operator surface under test (precision rule 13)**. The reuse golden needs a `GoalKind` whose pursuit can lawfully suspend at an information barrier and whose skeleton steps are *preservable* (i.e., not combat / target-identity-bound). Likely candidates: commodity-acquisition where the seller's location is unknown until a witness reports it. Verify the live operator surface during ticket reassessment — pick the `GoalKind` based on what current planner emission supports today, not what the spec narrates abstractly.
5. **Scenario isolation (precision rule 8)**. The reuse golden must isolate the resume-via-seeded-search branch from competing lawful affordances. Document at ticket-write time: (a) the intended branch (resume via seeded search), (b) lawful competing affordances the scenario excludes (e.g., alternative acquisition routes that would compete with the resumed pursuit), (c) why those competitors are intentionally absent from setup.
6. **Replay/save-load equivalence (precision rule 14, FND-12)**. The save round-trip case the spec calls out: "the budget-exhaustion save round-trips with the now-populated skeleton." After ticket 002, the budget-exhausted segment carries `Some(_)` skeleton; the save/load surface must preserve that across a round-trip. Coverage path: serialize a `SimulationState` whose agenda contains a populated partial plan segment, deserialize, confirm the skeleton survives losslessly. May leverage existing `partial_plan_segment_roundtrips_through_bincode_with_all_barrier_facts:378` patterns extended at the `SimulationState` level.
7. **Negative cases (precision rule 12)**. Spec lists three:
   - **no skeleton-as-rail**: a populated skeleton step whose precondition no longer holds → reuse rejected, lawful replan; the trace records the invalidation reason. Provable via the fallback golden's trace assertion (the invalidation reason is named, not just "fallback fired").
   - **no world-truth read**: revalidation reads only belief view. Provable via ticket 001's focused unit test (mock that panics on world accessor); the golden itself doesn't need a separate assertion if the focused-test coverage proves the architectural property.
   - **no skeleton for combat/target-identity steps**: provable via ticket 002's filter tests; the golden doesn't need a separate assertion.
8. **No-regression scope**. The spec says "survival/integration goldens unaffected (resume behavior is equivalent or strictly better-bounded)." This is satisfied by running the existing survival/integration suite and confirming no failures, plus a focused check on any existing test that exercises `try_resume_partial_plan` (enumerated in ticket 003's Assumption Reassessment).
9. **Existing test coverage to extend**:
   - The negative-result of "no reuse" path before ticket 002 should be re-validated to confirm fallback equivalence: the same agent in the same scenario without skeleton population (or with the skeleton manually nulled in a test variant) reaches the same final state. Confirms the optimization is lawful (FND-12 causal-equivalence contract).
10. **Adjacent contradictions**. None known. The goldens are downstream of the foundation work; if 001-003 land correctly, the golden chain is observable.

## Architecture Check

1. **Goldens prove the cross-system causal chain that focused tests cannot.** Per FND-31, "passing a local golden end state is not evidence by itself" — but local focused tests are also insufficient when the contract spans information-barrier suspension → companion spawn → barrier satisfaction → resume → seeded search → action execution. The trace-assertion discipline (precision rule 6: prefer decision-trace assertions over weaker indirect evidence) anchors the proof at the strongest available causal layer.
2. **Replay/save-load equivalence is a hard FND-12 obligation.** A populated skeleton that survives save/load proves the change to the field's content respects the save-format contract (no version bump needed because the field already serialized; but the *content* round-trip must be verified).
3. **No-regression sweep guards against unintended ranking shifts.** The seeded path is a strict optimization in the spec's framing, but any change to the search-control bias could subtly affect plan ordering for adjacent scenarios. Running the existing survival/integration goldens catches that class of regression at the cross-system layer.

## Verification Layers

1. **Reuse causal chain** → decision-trace assertion: the reuse golden asserts `PartialPlanResumeTrace { decision: ReusedSeededSearch, per_step_verdicts: [Reusable, …], seeded_ops: Some(…) }` is present at the resume tick, AND the agent's subsequent action(s) match the skeleton's high-level intent.
2. **Reuse action lifecycle** → action trace: the resumed pursuit's actions appear in the action trace in the expected order (matching the skeleton's op sequence at the tactical level).
3. **Reuse final world state** → authoritative world state: the goal condition is satisfied (e.g., the agent ends up holding the acquired commodity), matching what full replan from the same belief state would achieve.
4. **Fallback causal chain** → decision-trace assertion: the fallback golden asserts `PartialPlanResumeTrace { decision: FallbackToReplanInvalid(reason), per_step_verdicts: [..., Invalid(reason)], seeded_ops: None }` AND the agent re-enters the `Pending` agenda phase with `resume_attempt_count` incremented.
5. **Fallback action lifecycle** → action trace: the fallback golden's resumed pursuit goes through full candidate generation + ranking + search (visible as the standard agent-tick trace events) rather than seeded search.
6. **Save/load equivalence** → focused integration test: serialize a state with populated partial plan segment, deserialize, confirm bit-identical (or behaviorally identical) on next tick. Distinct proof surface from the focused bincode round-trip ticket 002 added (which proves the segment alone round-trips; this proves the *enclosing* state does too).
7. **No-regression** → existing survival/integration golden suite passes unchanged.

Per precision rule 5, each invariant maps to a single proof surface. The reuse golden uses three (decision trace + action trace + authoritative state) because cross-system causal chains require layer separation (precision rule 2: do not collapse distinct layers into one vague claim).

## What to Change

### 1. Scenario authoring for reuse golden

Create a new RON scenario (likely `crates/worldwake-ai/tests/scenarios/<descriptive_name>.ron` — confirm exact path during ticket reassessment by inspecting current golden scenario layout) that:

- Spawns an agent with a commodity-acquisition goal whose seller's location is unknown.
- Includes a lawful witness who can be questioned (companion goal target).
- Includes an information-barrier-triggering condition such that the agent's planning suspends with a populated `remaining_skeleton` (validated by ticket 002).
- After N ticks, the witness arrives (or the agent reaches the witness) and the barrier is satisfied.
- The resume should then trigger; the skeleton revalidates `Reusable`; seeded search proceeds.

Document the scenario isolation choices per Assumption Reassessment #5.

### 2. Reuse golden test

Create `crates/worldwake-ai/tests/golden_ai/<scenario_name>.rs` (or extend the existing golden scaffolding) that:

- Loads the scenario from §1.
- Runs the simulation to completion.
- Asserts on the decision trace: at the resume tick, `PartialPlanResumeTrace { decision: ReusedSeededSearch, … }` is present and names the seeded ops.
- Asserts on the action trace: the resumed pursuit's actions match the skeleton's op order at the tactical level.
- Asserts on the final world state: the goal condition is satisfied.

### 3. Scenario authoring for fallback golden

Create a sibling RON scenario where:

- Same setup as §1, but the world state changes between suspension and resume such that a load-bearing belief in the skeleton becomes stale or contradicted before the barrier is satisfied.
- Example: the seller's believed stock is depleted by another agent before the witness arrives.
- The resume condition fires (barrier satisfied), but revalidation returns `Invalid(BeliefContradicted)` or similar.

### 4. Fallback golden test

Create `crates/worldwake-ai/tests/golden_ai/<scenario_name>_fallback.rs` that:

- Asserts the decision trace at the resume tick records `FallbackToReplanInvalid(reason)` with the named reason.
- Asserts the agent re-enters `AgendaPhase::Pending` (verifiable via the agenda-state inspection or via the subsequent agent-tick trace events).
- Asserts the agent eventually replans and either achieves the goal through a different path or abandons it (whichever is lawful given the new belief state).

### 5. Replay / save-load equivalence

Add a focused integration test (likely `crates/worldwake-ai/tests/<save_load_equivalence>.rs` or extend an existing save/load test file) that:

- Drives the reuse scenario from §1 to the point of suspension (with populated skeleton).
- Serializes the `SimulationState` to bytes.
- Deserializes; runs one more tick.
- Compares the post-tick state against running the original state for the same tick — they must be behaviorally identical (deterministic seed; same decision trace; same actions committed).
- Repeat for the fallback scenario from §3.

### 6. No-regression sweep

- Run `cargo test -p worldwake-ai --test golden_ai` (or whatever the post-S154 form is — confirm via `docs/generated/golden-e2e-inventory.md` at ticket-write time).
- Confirm all survival/integration goldens pass unchanged.
- If any test fails, the fix is *not* to weaken the test — it's a regression in the seeded-search bias affecting plan selection in adjacent scenarios. Surface it as a finding before proceeding.

### 7. Documentation regeneration

After the new goldens land, regenerate `docs/generated/golden-e2e-inventory.md` and `docs/generated/golden-scenario-index.md` per `tickets/README.md` line 8: `python3 scripts/golden_inventory.py --write --check-docs`.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/<reuse_scenario>.ron` (new — exact filename per scenario-authoring conventions)
- `crates/worldwake-ai/tests/scenarios/<fallback_scenario>.ron` (new)
- `crates/worldwake-ai/tests/golden_ai/<reuse_scenario>.rs` (new — exact location confirmed via `rg "InformationBarrier" crates/worldwake-ai/tests/` at ticket reassessment)
- `crates/worldwake-ai/tests/golden_ai/<fallback_scenario>.rs` (new)
- Likely: `crates/worldwake-ai/tests/<save_load_equivalence>.rs` (new — confirm extension vs new file via grep of existing save_load tests)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerated)
- `docs/generated/golden-scenario-index.md` (modify — regenerated)
- `docs/generated/golden-scenario-details/` (modify — regenerated entries for the new scenarios)

## Out of Scope

- Skeleton population — ticket 002.
- Revalidation function — ticket 001.
- Seeded search + resume integration + trace struct — ticket 003.
- Resource/jurisdiction/coordination barrier goldens — spec Non-Goals.
- Combat plan / target-identity-bound skeleton goldens — spec Non-Goals.
- Performance regression guards (the spec is an optimization but explicitly the *lowest-benefit* one; profiling is a stretch goal mentioned only in Risks). Not required for this ticket per S168 Risks section.

## Acceptance Criteria

### Tests That Must Pass

1. The reuse golden passes and asserts the three causal layers (decision trace + action trace + final world state) per Verification Layers #1-3.
2. The fallback golden passes and asserts the two causal layers (decision trace + action trace) per Verification Layers #4-5.
3. The save/load equivalence test passes for both reuse and fallback scenarios per Verification Layer #6.
4. The full survival/integration golden suite passes unchanged per Verification Layer #7.
5. `python3 scripts/golden_inventory.py --check-docs` reports no doc drift after regeneration.
6. Existing suite: `cargo test -p worldwake-ai` passes.

### Invariants

1. The reuse golden's final world state matches what full replan from the same belief state would produce (FND-12 causal-equivalence contract). Provable by Verification Layer #3 (final state) compared against a baseline run with skeleton population disabled (or with the skeleton manually nulled in a test variant).
2. The fallback golden's reason field in `PartialPlanResumeTrace::FallbackToReplanInvalid(reason)` matches the actual invalidation cause documented in the scenario setup (e.g., setup says "stock depleted by other agent" → trace records `BeliefContradicted` or equivalent). Provable by Verification Layer #4.
3. The replay/save-load round-trip preserves the populated skeleton bit-identically across serialize/deserialize cycles. Provable by Verification Layer #6.
4. No-regression: every existing survival/integration golden that passed before this ticket lands still passes after it. Provable by Verification Layer #7.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai/<reuse_scenario>.rs` — reuse golden (§2).
2. `crates/worldwake-ai/tests/golden_ai/<fallback_scenario>.rs` — fallback golden (§4).
3. `crates/worldwake-ai/tests/<save_load_equivalence>.rs` — save/load round-trip with populated skeleton (§5).
4. `crates/worldwake-ai/tests/scenarios/<reuse_scenario>.ron` — scenario RON for §1.
5. `crates/worldwake-ai/tests/scenarios/<fallback_scenario>.ron` — scenario RON for §3.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai <reuse_scenario_substring>` — targeted reuse golden.
2. `cargo test -p worldwake-ai --test golden_ai <fallback_scenario_substring>` — targeted fallback golden.
3. `cargo test -p worldwake-ai --test golden_ai` — full golden suite (no-regression check).
4. `cargo test -p worldwake-ai` — full ai-crate suite.
5. `python3 scripts/golden_inventory.py --write --check-docs` — regenerate generated docs.
6. `scripts/verify.sh` — pre-PR full gate (fmt, clippy --all-targets -D warnings, test --workspace).
