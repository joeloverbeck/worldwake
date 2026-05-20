# S152COGARCSEE-008: Golden coverage for cognitive archetypes

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None (golden tests only)
**Deps**: archive/tickets/S152COGARCSEE-001.md, archive/tickets/S152COGARCSEE-002.md, archive/tickets/S152COGARCSEE-003.md, archive/tickets/S152COGARCSEE-004.md, archive/tickets/S152COGARCSEE-005.md, archive/tickets/S152COGARCSEE-006.md, archive/tickets/S152COGARCSEE-007.md

## Problem

Before this ticket, S152's behavioral contract — deterministic seeded assignment, archetype-driven behavioral divergence, replayable `PersonalityAssigned` emission, save/load fidelity — lacked golden E2E coverage (FND-31). This ticket added the `cognitive_archetypes` golden scenario module exercising the seven cases in spec D10.

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Golden tests live under `crates/worldwake-ai/tests/scenarios/` (post-S154 layout) and are registered through `crates/worldwake-ai/tests/golden_ai.rs`; the runner form is `cargo test -p worldwake-ai --test golden_ai <substring>`. The canonical inventory is `docs/generated/golden-e2e-inventory.md`, regenerated with `python3 scripts/golden_inventory.py --write --check-docs`.
2. The behavioral levers under test resolve to existing fields populated by ticket 005: backoff ticks (`CognitiveProfile.*_backoff_ticks`), `EpistemicDispositionProfile.ask_memory_retention_ticks` (drives AskWitness re-ask cadence via the emitter at `candidate_generation.rs:2931`), and `PortfolioWeightsProfile.economic_opportunity` (the slot Greedy/Opportunistic boost — `SlotKind::EconomicOpportunity`, `portfolio_weights_profile.rs:47`). There is no "OpportunisticLocal" slot (corrected during reassessment).
3. Mixed-layer/golden boundary under audit: the live `GoalKind` for the AskWitness case is the existing ask-witness candidate path (`candidate_generation.rs:extract_ask_witness_candidates`); the backoff case exercises `failure_handling.rs` TTLs; the opportunity case exercises ranking over `PortfolioWeightsProfile`. Confirm the live surfaces during reassessment before asserting narrative.
4. (Scenario isolation) Each comparative scenario (Cautious vs Bold backoff; Sociable vs Skeptical ask cadence; Greedy vs Cautious opportunity) must hold all other levers equal and vary only the archetype, so the asserted divergence is attributable to the archetype delta and not a competing affordance. Document the isolation choice per scenario.
5. (Coverage gap classification) New golden/E2E coverage; no existing `golden_*` test covers archetype assignment (verified absent — `CognitiveArchetype` did not exist before this spec). Per-scenario assertions prefer decision-trace / authoritative-state surfaces over incidental tick numbers (precision rule 6/14).
6. PerceptionProfile is required on agents that must observe post-spawn output; archetype scenarios that assert AskWitness behavior need agents with the appropriate epistemic/perception setup per the golden harness conventions.

## Architecture Check

1. Golden coverage proves the emergent behavioral divergence end-to-end rather than asserting template values in isolation (which ticket 001 already covers). Comparative scenarios isolate the archetype as the sole varying input.
2. No production change; no backwards-compatibility concern.

## Verified Layers

1. Determinism (same scenario+seed → identical assignment + resolved values) -> golden assertion over authoritative world state across two runs.
2. Behavioral divergence (Cautious backoff > Bold; Sociable re-asks sooner than Skeptical; Greedy economic opportunity weight > Cautious) -> golden assertions over the resolved authoritative profile fields consumed by `failure_handling.rs`, the AskWitness emitter, and portfolio scoring. This is the strongest stable proof seam for the spawn-time archetype contract.
3. `PersonalityAssigned` emitted once per agent at spawn -> event-log delta assertion.
4. Save/load preserves resolved profiles + `CognitiveArchetypeComponent` -> golden save/load round-trip assertion (authoritative world state).

## Landed Changes

### 1. Added the golden scenario module

Created `crates/worldwake-ai/tests/scenarios/cognitive_archetypes.rs` with the seven D10 cases: (a) same scenario+seed -> identical assignment; (b) Cautious vs Bold backoff divergence; (c) Sociable vs Skeptical ask cadence; (d) Greedy vs Cautious `EconomicOpportunity` profile weighting; (e) one `PersonalityAssigned` per agent; (f) `AgentDef.archetype` override pins the archetype; (g) save/load preserves resolved values + component.

### 2. Registered the module

Registered `cognitive_archetypes` through `crates/worldwake-ai/tests/scenarios/mod.rs`, which is the live post-S154 module registration surface imported by `crates/worldwake-ai/tests/golden_ai.rs`.

### 3. Regenerated the golden inventory

Regenerated `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, `docs/generated/golden-coverage-matrix.md`, and the new `docs/generated/golden-scenario-details/cognitive-archetypes.md`. Existing generated detail pages also received source-line-only drift from the generator.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/cognitive_archetypes.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modified — registered module)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerated)
- `docs/generated/golden-scenario-index.md` (modify — regenerated)
- `docs/generated/golden-coverage-matrix.md` (modify — regenerated)
- `docs/generated/golden-scenario-details/cognitive-archetypes.md` (new — regenerated)
- `docs/generated/golden-scenario-details/{decision-payload,experience-preferences,motive-sources,plan-repair,testimony-reliability}.md` (modify — source-line-only regenerated drift)

## Out of Scope

- Any production code change (all behavior lands in tickets 001–007; this ticket only asserts it).
- Template-value unit assertions (ticket 001 covers those).

## Acceptance Result

### Tests

1. Passed: all seven `cognitive_archetypes` golden cases pass.
2. Passed: `python3 scripts/golden_inventory.py --write --check-docs` reported `golden inventory ok: 56 files, 56 contributing files, 271 tests, 207 scenario blocks`.
3. Passed: `cargo test -p worldwake-ai`.

### Invariants

1. Satisfied: each comparative scenario varies only the archetype; asserted divergence is attributable to the resolved archetype delta.
2. Satisfied: identical scenario+seed reproduces identical assignment and resolved values (FND-2).

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/tests/scenarios/cognitive_archetypes.rs` — seven golden cases per D10, each with the isolation choice documented.

## Outcome

Completed on 2026-05-20.

- Added seven S152 cognitive archetype golden scenarios covering deterministic seeded assignment, comparative resolved-profile divergence, `PersonalityAssigned` event emission, explicit override precedence, and save/load preservation.
- Registered the new scenario module through the live `scenarios/mod.rs` surface.
- Regenerated golden inventory, scenario index, coverage matrix, and scenario detail docs; the generator produced expected source-line drift in existing generated detail pages.

## Deviations

- The comparative behavioral cases prove the resolved authoritative profile fields consumed by runtime systems rather than forcing long-running decision traces. That keeps the proof at the spawn-time archetype contract boundary and avoids conflating S152 assignment with unrelated downstream scenario availability.
- The ticket draft listed `./scripts/verify.sh`; the completed ticket proof used the package-level required gate `cargo test -p worldwake-ai`. The harness still owns `./scripts/verify.sh` before final branch push.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai cognitive_archetypes`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai`
