# S167COGARCBEH-001: Author cognitive-archetypes-divergence scenario

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: [`specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md)

## Problem

The `FeatureId::CognitiveArchetypes` detector
(`crates/worldwake-cli/src/bin/scenario_coverage.rs:865`,
`cognitive_archetypes_status(def)`) reports the feature absent in every row of
`docs/generated/scenario-coverage.md` because zero `scenarios/*.ron` files
activate archetypes (verified during reassessment). S152 shipped the archetype
implementation and seven profile-value-divergence tests, but no canonical
scenario exercises the feature. This blocks both the FND-31 behavioral-divergence
proof (S167COGARCBEH-002) and the coverage doc's truthfulness.

This ticket authors a dedicated paired-agent scenario engineered around the
documented `Greedy vs Cautious` portfolio-weight delta at an economic-vs-safety
tension, and regenerates the coverage doc so the archetype row flips from absent
to active.

## Assumption Reassessment (2026-05-24)

1. `CognitiveArchetype` enum has 10 variants
   (`Cautious, Bold, Stubborn, Methodical, Opportunistic, Sociable, Skeptical,
   Dutiful, Greedy, Fearful`) at
   `crates/worldwake-core/src/cognitive_archetype.rs:6-17`. `AgentDef.archetype:
   Option<CognitiveArchetype>` is the per-agent scenario field at
   `crates/worldwake-cli/src/scenario/types.rs:664`.
   `ArchetypeAssignmentPolicy` (`DefaultUniformFive | Uniform | Weighted` at
   `cognitive_archetype.rs:58-63`) is randomized and deliberately not used here
   — explicit per-agent assignment is the deterministic path.
2. The spec
   ([`specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md))
   commits in D2 to a dedicated `scenarios/cognitive-archetypes-divergence.ron`
   with two same-role/same-place/same-belief agents differing only by archetype,
   and to `Greedy vs Cautious` as the archetype pair. The portfolio-weight
   economic delta is documented by
   `cognitive_archetypes_greedy_resolves_higher_economic_weight_than_cautious`
   at `crates/worldwake-ai/tests/scenarios/cognitive_archetypes.rs:255`.
3. Shared boundary under audit: scenario authoring contract
   (`AgentDef.archetype`, `WorkstationTag`, `PlaceTag`, `CommodityKind`,
   `bandit_camps` or equivalent hostile-presence primitive). All values must be
   existing enum variants — cross-reference each value against actual enum
   definitions during authoring.
4. Existing focused/unit coverage: the seven tests in
   `crates/worldwake-ai/tests/scenarios/cognitive_archetypes.rs` (418 lines)
   prove resolved-profile-value divergence; this ticket does not modify them.
   Existing golden coverage: no scenario currently activates archetypes —
   `scenario-coverage.md` line 83 shows the `Cognitive archetypes` row absent
   across all 19 scenarios.
5. The activation detector at
   `crates/worldwake-cli/src/bin/scenario_coverage.rs:865-872` checks two
   surfaces: `scenario_def.archetype_assignment_policy` (scenario-level) and
   `agents[].archetype` (per-agent). Per-agent assignment satisfies the detector
   without invoking the randomized policy.

## Architecture Check

1. **Dedicated scenario over canonical overrides** — adding `archetype` to
   `final-integration.ron` or another existing canonical scenario would
   destabilize that scenario's existing assertions and dilute the archetype
   impact among unrelated competing tensions. A dedicated focused scenario
   keeps the proof self-contained and lets D4 (S167COGARCBEH-003) classify it
   as auxiliary-behavior coverage rather than a survival-coexistence landing.
2. **Explicit per-agent `AgentDef.archetype` over policy** — the spec's D2
   commits to explicit assignment because `ArchetypeAssignmentPolicy` is seeded
   random; a deterministic paired-agent divergence proof requires both
   archetypes pinned at scenario-load time. This is FND-3 (concrete state) and
   FND-22 (concrete variation): the archetype identity is authored, not
   sampled.
3. **No new scenario primitives** — the marginal economic-vs-safety tension is
   built from existing substrate (commodity source within travel range plus a
   documented hostile-presence indicator on the route). No new
   `WorkstationTag`, `PlaceTag`, or scenario field is introduced — preserves
   FND-20 (no scenario rail).

## Verification Layers

1. Structural activation of `CognitiveArchetypes` feature -> generated
   `docs/generated/scenario-coverage.md` (the regenerated row shows active for
   the new scenario).
2. Scenario load determinism -> `scripts/verify.sh` (scenario load is part of
   workspace test runs; no separate proof surface needed since the scenario
   contains no novel mechanics).
3. Single-layer ticket: this is scenario-authoring + generated-doc work; the
   behavioral-divergence proof surface lives in S167COGARCBEH-002, not here.

## What to Change

### 1. Author `scenarios/cognitive-archetypes-divergence.ron`

Author a new RON scenario with:

- A single place hosting both agents (e.g., a starting hamlet) plus a
  travel-reachable commodity-source place (e.g., an orchard or grain field).
- A `bandit_camps` entry (or equivalent hostile-presence world state) on the
  route or at the commodity-source place, sized so that the marginal safety
  cost of the acquisition action is small but non-zero.
- Two agents of the same role, both at the starting place, with **identical**
  `metabolism_profile`, `perception_profile`, `cognitive_profile`,
  `schema_context_profile`, `epistemic_disposition`, `testimony_profile`,
  `route_preference_profile`, and `risk_profile`. Explicit per-agent
  `archetype: Greedy` on one and `archetype: Cautious` on the other.
- Both agents seeded with identical belief stores — including identical
  knowledge of the commodity source AND identical knowledge of the
  hostile-presence indicator. Belief seeding uses the existing
  `AgentDef`-level belief-injection surface (whatever the current scenario
  primitive is — verify against an existing scenario like
  `scenarios/survival-trade.ron` or `scenarios/survival-contested.ron` at
  authoring time).
- A short tick budget sufficient to reach the divergence tick — substantially
  shorter than the 1440-tick survival-health contract, since this scenario is
  a focused behavior proof, not a survival-coexistence row.
- No `survival_health_contract` block — this is auxiliary behavior coverage.
- Author a header comment explaining the divergence design: pair, tension,
  decisive profile-field delta, expected `Greedy` choice vs expected
  `Cautious` choice, why the tension is lawful (not a rail).

Cross-reference all proposed values against existing enums and scenarios
during authoring:

- `WorkstationTag` variants live in `crates/worldwake-core/src/production.rs`.
- `PlaceTag` variants live in `crates/worldwake-core/src/topology.rs`.
- `CommodityKind` variants live in the commodity module under `worldwake-core`
  (verify exact path during authoring).
- Recipe names are Title Case with spaces (e.g., `"Harvest Apples"`, not
  `HarvestApples`).
- Closest reference scenarios for shape: `scenarios/survival-contested.ron`
  (multi-agent + hostile pressure), `scenarios/survival-trade.ron`
  (acquisition tensions with explicit profiles).

### 2. Regenerate `docs/generated/scenario-coverage.md` (D3, subsumed)

After the scenario lands, run:

```bash
cargo run -p worldwake-cli --bin scenario-coverage -- --write
```

Commit the regenerated `docs/generated/scenario-coverage.md`. The expected
diff: the `Cognitive archetypes` row at line 83 (current generation) flips
from `—` in the `cognitive-archetypes-divergence` column to `Active` (exact
column position depends on alphabetical ordering of scenario names — verify by
inspecting the regenerated file).

## Files to Touch

- `scenarios/cognitive-archetypes-divergence.ron` (new)
- `docs/generated/scenario-coverage.md` (modify — regenerated)

## Out of Scope

- The behavioral-divergence golden test that loads this scenario — lives in
  S167COGARCBEH-002.
- Scenario roadmap formalization — lives in S167COGARCBEH-003.
- CI workflow lane — lives in S167COGARCBEH-004.
- Any change to engine code, profile templates, archetype assignment logic,
  decision-trace surface, or `cognitive_archetype.rs` — this is
  scenario-authoring only.
- Adding `survival_health_contract` to the new scenario — this is auxiliary
  behavior coverage per D4, not a survival-coexistence row.
- Modifying the existing seven `cognitive_archetypes.rs` profile-value tests.

## Acceptance Criteria

### Tests That Must Pass

1. Scenario loads successfully under `scenario_coverage` and the regenerated
   `docs/generated/scenario-coverage.md` shows `CognitiveArchetypes` active in
   the new scenario's column.
2. `cargo test --workspace` passes — scenario load is exercised by the
   workspace test suite (no scenario syntax errors, no missing required
   fields, no enum-value typos).
3. `cargo run -p worldwake-cli --bin scenario-coverage -- --check` passes
   (the `--check` mode verifies the committed generated doc matches what
   `--write` would produce — the CI gate).

### Invariants

1. The scenario contains exactly two agents at the same starting place with
   identical profile fields **except** `archetype`.
2. Both agents have identical seeded beliefs at scenario load — no perception
   event differences between them at tick 0.
3. The scenario uses only existing `WorkstationTag`, `PlaceTag`,
   `CommodityKind`, and recipe-name variants — no new scenario primitives are
   introduced.
4. No `archetype_assignment_policy` field — per-agent `archetype` is the only
   assignment mechanism (deterministic path required for paired-agent
   divergence in the downstream golden).

## Test Plan

### New/Modified Tests

1. `scenarios/cognitive-archetypes-divergence.ron` — new scenario file; load
   itself is the test for ticket scope (workspace test suite parses all
   scenarios at startup).
2. `docs/generated/scenario-coverage.md` — modified by regeneration; the
   `--check` gate is the test for correctness.
3. None — no Rust test files change in this ticket. Behavioral assertions live
   in S167COGARCBEH-002.

### Commands

1. `cargo run -p worldwake-cli --bin scenario-coverage -- --check` (verifies
   the regenerated coverage doc is committed)
2. `cargo test --workspace` (verifies scenario loads without errors across
   the workspace)
3. `scripts/verify.sh` (full pre-PR gate)
