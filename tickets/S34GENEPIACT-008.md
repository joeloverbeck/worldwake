# S34GENEPIACT-008: Golden E2E tests — Scenario D variant, ask-witness chain, stale-belief refresh

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S34GENEPIACT-001 through S34GENEPIACT-007 (all engine changes complete)

## Problem

No golden E2E coverage exists for deliberate epistemic actions. Without golden tests, the emergent multi-system chains (rumor -> travel -> verify -> violation -> replan, and ask-witness -> report-belief -> plan) are not proven to work end-to-end through the full AI pipeline.

## Assumption Reassessment (2026-03-28)

1. The spec defines 3 golden tests plus deterministic replay companions (4 goldens in the spec's `### Golden tests` section):
   - Scenario D variant: rumor about commodity -> travel -> verify_belief (SupplyAvailability) -> SupplyDepleted violation -> replan
   - Ask-witness chain: ask co-located witness about entity location -> receive report-sourced belief -> plan travel
   - Stale-belief refresh: stale entity location belief -> VerifyBelief candidate -> travel -> entity found -> belief refreshed -> proceed
   - Deterministic replay companions for each
2. Golden tests live in `crates/worldwake-ai/tests/` (e.g., `golden_emergent.rs`, `golden_ai_decisions.rs`). Recent goldens use the full action registries via `build_full_action_registries()` and the `GoldenTestHarness` pattern.
3. All golden tests require `PerceptionProfile` on agents that need to observe post-action results (per CLAUDE.md: "Golden production tests require `PerceptionProfile` on agents that need to observe newly created entities").
4. All golden tests require `VerificationDispositionProfile` on the agent under test for candidate generation.
5. Decision traces (`h.driver.enable_tracing()`) should be used for diagnosing golden test behavior per CLAUDE.md debugging guidance.
6. Action traces (`h.enable_action_tracing()`) should be used for verifying action lifecycle per CLAUDE.md.
7. Deterministic replay companions use `replay_and_verify()` to prove replay consistency.
8. This is a golden E2E ticket. Full action registries required. Not a needs-only harness scenario.
9. Scenario isolation: each golden should isolate the intended epistemic chain from competing affordances by controlling the world setup (limited commodity sources, specific agent placement, targeted belief seeding).

## Architecture Check

1. Golden tests prove the emergent chain works end-to-end. They do not test individual components (those are covered in tickets 001-007). The golden tests verify that candidate generation, ranking, plan search, action execution, belief update, and replan all compose correctly.
2. No backward-compatibility shims. New test files only.

## Verification Layers

1. Scenario D variant (rumor -> travel -> verify -> violation -> replan) -> golden E2E: decision trace shows VerifyBelief candidate, plan search finds Travel->VerifyBelief, action trace shows verify_belief commit, belief store shows SupplyDepleted violation, subsequent tick shows replan to alternative
2. Ask-witness chain -> golden E2E: decision trace shows VerifyBelief candidate, plan search finds AskWitness, action trace shows ask_witness commit with Report provenance, subsequent planning uses the received belief
3. Stale-belief refresh -> golden E2E: decision trace shows VerifyBelief candidate for stale entity-location belief, travel + verify_belief confirms entity present, belief refreshed with DirectObservation, original goal proceeds
4. Deterministic replay -> replay_and_verify() for each golden
5. All verification via golden E2E layer; lower-layer coverage is in tickets 001-007.

## What to Change

### 1. Scenario D variant golden

Create a golden test (likely in `crates/worldwake-ai/tests/golden_emergent.rs` or a new `golden_epistemic.rs`):

- **Setup**: Agent at Place A with stale rumor-sourced belief about commodity availability at distant Place B (resource source). The source at Place B is actually depleted. Agent has `VerificationDispositionProfile` with belief_verification_threshold that the stale belief falls below. Agent has a need that requires the commodity.
- **Expected chain**: Agent emits need-based AcquireCommodity candidate -> evidence entity (source) has low-confidence belief -> emit_verify_belief_goals emits VerifyBelief(SupplyAvailability) -> planner finds Travel(PlaceB) -> VerifyBelief -> agent travels -> verifies -> finds depleted -> SupplyDepleted violation recorded -> replans to alternative source or different goal.
- **Assertions**: Decision trace shows VerifyBelief candidate emitted, plan search succeeds, action trace shows verify_belief commit, violation memory contains SupplyDepleted, agent replans (no longer pursuing depleted source).

### 2. Ask-witness chain golden

- **Setup**: Agent A at Place X with stale belief about entity E's location. Agent B at Place X with fresh direct-observation belief about entity E at Place Y. Agent A has `VerificationDispositionProfile`.
- **Expected chain**: Agent A emits VerifyBelief candidate for entity E -> planner finds AskWitness (Agent B is co-located) -> ask_witness commits -> Agent A receives Report-sourced belief about E at Place Y -> Agent A uses this belief in subsequent planning.
- **Assertions**: Action trace shows ask_witness commit, Agent A's belief store contains entity E at Place Y with Report provenance, conversation memory entries exist for both agents.

### 3. Stale-belief refresh golden

- **Setup**: Agent at Place A with stale belief about entity E being at Place B. Entity E IS at Place B. Agent has a goal that depends on interacting with entity E. Agent has `VerificationDispositionProfile`.
- **Expected chain**: Agent emits goal depending on E -> evidence entity E has low-confidence belief -> VerifyBelief candidate emitted -> planner finds Travel(PlaceB) -> VerifyBelief -> agent travels -> verifies -> entity found -> belief refreshed with DirectObservation (observed_tick = current_tick) -> VerifyBelief goal satisfied -> proceeds with original goal.
- **Assertions**: Decision trace shows VerifyBelief candidate, verify_belief action commits with entity present, belief refreshed, VerifyBelief goal satisfied, original goal resumes.

### 4. Deterministic replay companions

For each golden, add a `replay_and_verify()` companion test proving deterministic replay produces identical state hashes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_epistemic.rs` (new — or added to existing golden_emergent.rs)
- `crates/worldwake-ai/tests/` module wiring if new file

## Out of Scope

- Engine changes (all done in tickets 001-007)
- Focused unit/integration tests (done in tickets 001-007)
- Changes to existing golden tests
- Golden tests for `ask_witness` abort paths (covered by focused tests in ticket 004)
- Golden tests for `verify_belief` abort paths (covered by focused tests in ticket 003)
- Performance benchmarking

## Acceptance Criteria

### Tests That Must Pass

1. Golden: Scenario D variant — agent with stale rumor travels, verifies depleted source, records SupplyDepleted violation, replans to alternative
2. Golden: Ask-witness chain — agent asks co-located witness about entity location, receives report-sourced belief, uses it for subsequent planning
3. Golden: Stale-belief refresh — agent verifies stale entity-location belief, entity found, belief refreshed, proceeds with original goal
4. Deterministic replay companion for each golden (3 replay tests)
5. Existing suite: `cargo test -p worldwake-ai` (all existing goldens unaffected)

### Invariants

1. All golden tests use full action registries (not needs-only harness)
2. All agents under test have `PerceptionProfile` for post-action observation
3. All agents under test have `VerificationDispositionProfile` for candidate generation
4. Deterministic replay produces identical state hashes (P11)
5. No golden test depends on HashMap/HashSet ordering or floats
6. Canonical Scenario D can emerge through deliberate verification, not only passive perception (spec acceptance criterion 11)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_epistemic.rs` — 3 golden scenarios + 3 replay companions
2. Scenario annotations follow `// Scenario N:` pattern for golden inventory alignment

### Commands

1. `cargo test -p worldwake-ai golden_epistemic` (or matching test name pattern)
2. `cargo test -p worldwake-ai` (full AI suite to verify no regressions)
3. `cargo build --workspace`
