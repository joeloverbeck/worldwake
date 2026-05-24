# S167COGARCBEH-003: Formalize archetype proof row in scenario-roadmap.md

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: [`archive/tickets/S167COGARCBEH-001.md`](../archive/tickets/S167COGARCBEH-001.md), S167COGARCBEH-002, [`specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md)

## Problem

`docs/scenario-roadmap.md` is the hand-authored editorial companion to the
generated coverage doc. It records which features have **landed** under the
three-part contract (structural activation + authored-behavior proof +
authored-causal-reason proof). The new
`scenarios/cognitive-archetypes-divergence.ron` (S167COGARCBEH-001) plus its
backing golden (S167COGARCBEH-002) need a roadmap row so future archetype work
(e.g., runtime adaptation under FND-22A) inherits the same landing rules and
the row is auditable alongside the existing landed scenarios.

This ticket adds four coordinated updates to `docs/scenario-roadmap.md` per the
existing entry contract template.

## Assumption Reassessment (2026-05-24)

1. `docs/scenario-roadmap.md` exists and follows a §1 preamble + §2 catalog +
   §3 status summary + §4 priority roadmap + §5 landed scenarios + §6
   maintenance workflow structure (verified during reassessment). The §4.1
   Entry Contract Template at lines 115-151 defines the canonical shape for
   landed entries.
2. The spec
   ([`specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md))
   D4 commits to four update points: catalog row in §2, ordering table row in
   §4.2, landed entry in §5 (auxiliary-coverage classification), Status Summary
   update in §3. The auxiliary classification mirrors the §5.17 entries for
   `cli-evaluation.ron`, `survival-need-projection.ron`, and `simulation_gaps.rs`.
3. Shared boundary under audit: the doc-coverage contract — every landed
   scenario in the roadmap must point to (a) a real `scenarios/*.ron`, (b) a
   real backing golden, (c) a real feature row in the §2 catalog with an
   activation signal that matches `scenario_coverage.rs`'s detector logic
   byte-for-byte. The catalog feature names in §2 must remain aligned with the
   live `FEATURES` table at
   `crates/worldwake-cli/src/bin/scenario_coverage.rs` (a note at roadmap
   line 30 emphasizes byte-for-byte alignment).
4. Existing roadmap rows that classify as auxiliary (not survival-coexistence)
   live in §5.17: `cli-evaluation.ron` (CLI/schema coverage only),
   `survival-need-projection.ron` (spec-S126 chain-isolation coverage only),
   `simulation_gaps.rs` (auxiliary simulation-gap coverage only). The new
   archetype row follows this pattern — it owns the FND-31 archetype-decision
   causal proof, not a survival-coexistence landing.

## Architecture Check

1. **Auxiliary classification over survival-row classification** — the new
   scenario has no `survival_health_contract` and is not a 1440-tick
   survival-coexistence scenario. Classifying it as auxiliary preserves the
   roadmap's two-axis structure: §4.2 survival-coexistence rows vs. §5.17
   auxiliary behavior coverage. Forcing it into a survival-row slot would
   require either inflating the scenario to a survival-coexistence proof
   (out of scope per the spec) or weakening the survival-row contract.
2. **Four coordinated updates over a single section append** — the roadmap's
   structure requires updates in §2 (catalog), §3 (status summary), §4.2
   (ordering table), and §5 (landed entry). Splitting the doc update across
   these sections is the convention; consolidating into a single new section
   would diverge from the existing readability pattern future readers
   navigate.

## Verification Layers

1. Documentation truthfulness (single-layer ticket — proof surface is human
   review + the doc's own coverage references resolving) -> manual
   verification that the cited paths
   (`scenarios/cognitive-archetypes-divergence.ron`,
   `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs`)
   resolve. No automated proof surface; the roadmap is editorial.
2. Catalog alignment with `scenario_coverage.rs` -> grep
   `scenario_coverage.rs` for `CognitiveArchetypes` feature name and confirm
   the catalog row's `Feature` column matches the live feature row text.
3. Single-layer ticket: this is editorial documentation. Items 4–6 of the
   template's Verification Layers are not applicable — no decision trace,
   action trace, or event-log delta is involved.

## What to Change

### 1. Add catalog row in §2 Gameplay Feature Catalog

Add a new row to the table at `docs/scenario-roadmap.md` lines 32+:

```markdown
| Cognitive archetypes | Per-agent `AgentDef.archetype` set OR `archetype_assignment_policy` authored | [`cognitive_archetype.rs`](../crates/worldwake-core/src/cognitive_archetype.rs), [`scenario/mod.rs`](../crates/worldwake-cli/src/scenario/mod.rs) spawn-time delta application | Landed in [§5.18](#518-landed-auxiliary-cognitive-archetypes-divergence) |
```

(Section number `5.18` is illustrative — verify the next available
sub-section number in §5 at write time.)

The activation signal text must match `cognitive_archetypes_status(def)` at
`crates/worldwake-cli/src/bin/scenario_coverage.rs:865-872` — it checks both
`scenario_def.archetype_assignment_policy` and per-agent `archetype`.

### 2. Add ordering table row in §4.2 Ordered Roadmap

Add a new row at the end of the table at lines 158-175, after row 17
`final-integration`:

```markdown
| 18 | `cognitive-archetypes-divergence` | Cognitive archetypes (auxiliary behavior coverage) | Landed | Auxiliary behavior-proof row; not subject to 1440-tick survival-health contract. Owns the FND-31 archetype-decision causal proof. |
```

### 3. Add landed entry in §5 Landed Scenarios

Add a new sub-section (e.g., `### 5.18 Landed Auxiliary:
cognitive-archetypes-divergence`) using the §4.1 Entry Contract Template:

```markdown
### 5.18 Landed Auxiliary: cognitive-archetypes-divergence

**Status**: Landed
**Source scenario**: [`scenarios/cognitive-archetypes-divergence.ron`](../scenarios/cognitive-archetypes-divergence.ron)
**Backing goldens**: [`cognitive_archetypes_divergence.rs`](../crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs)
**Depends on**: archived [S152 archetype implementation](../archive/specs/S152-cognitive-archetypes-seeded-diversity.md)

This row is **auxiliary behavior coverage**, not a survival-coexistence
landing. It owns the FND-31 archetype-decision causal proof: two same-role
same-belief agents differing only by `CognitiveArchetype` choose different
actions under identical local facts, with the decision trace and a test-side
profile-delta attribution naming the cause (Greedy's higher
`portfolio_weights.economic_weight` tipping a marginal economic-vs-safety
trade). A counterfactual archetype-swap replay asserts the divergence
reverses correspondingly.

The row has no `survival_health_contract` and runs on a short tick budget;
the proof is decision divergence + counterfactual symmetry + knowledge
legality, not 1440-tick coexistence. The scenario uses no new mechanics:
existing substrate primitives author the marginal economic-vs-safety tension,
and the archetype delta flows through ordinary ranking/search.

The scenario runs in its own dedicated CI workflow lane,
[`golden-cognitive-archetypes.yml`](../.github/workflows/golden-cognitive-archetypes.yml),
so a future profile retune that erases the divergence fails loudly in
isolation rather than silently in a batched lane.
```

### 4. Update Status Summary in §3

Read the current §3 contents and add the new auxiliary row to whichever
counter or list tracks landed scenarios. The exact update depends on §3's
current structure — inspect at write time and apply the minimal coherent
update. If §3 maintains a per-category count (survival-row count + auxiliary
count), increment the auxiliary count. If §3 lists landed rows by name, add
the new row's name.

## Files to Touch

- `docs/scenario-roadmap.md` (modify — 4 update points across §2, §3, §4.2,
  §5)

## Out of Scope

- Authoring the scenario file — completed in
  [`archive/tickets/S167COGARCBEH-001.md`](../archive/tickets/S167COGARCBEH-001.md).
- Authoring the golden test — owned by S167COGARCBEH-002.
- CI workflow file — owned by S167COGARCBEH-004 (the roadmap entry cites
  the workflow path, but the workflow file itself lands in 004).
- Coverage doc regeneration — completed in
  [`archive/tickets/S167COGARCBEH-001.md`](../archive/tickets/S167COGARCBEH-001.md).
- Any change to `scenario_coverage.rs`'s `FEATURES` table or the
  `CognitiveArchetypes` feature row registration — already present and
  cited by the new catalog row.
- Adding feature catalog rows for any other coverage gap (portfolio
  weights, intention disposition, expectation store, last-seen memory,
  social observations) — explicit Non-Goal in the spec, reserved for
  follow-up specs.

## Acceptance Criteria

### Tests That Must Pass

1. The catalog row's `Feature` column text matches `scenario_coverage.rs`'s
   `CognitiveArchetypes` feature name byte-for-byte (verified by grep).
2. All cited paths in the new landed entry resolve to existing files at
   review time (the source scenario, backing golden, and CI workflow all
   exist because S167COGARCBEH-001/002/004 are dependencies).
3. Existing suite: `scripts/verify.sh` passes. (Roadmap docs do not have a
   dedicated automated check beyond markdown rendering; the test surface is
   editorial review.)

### Invariants

1. The new entry follows the §4.1 Entry Contract Template — Status, Source
   scenario, Backing goldens, Depends on, narrative explaining the proof
   contract, deliberate inactivity (if any).
2. The catalog feature name and activation signal match the live
   `scenario_coverage.rs` registration byte-for-byte.
3. The auxiliary classification is preserved — the entry explicitly
   distinguishes itself from survival-coexistence rows, and the §4.2
   ordering table notes the auxiliary status.
4. All four update points (§2 catalog, §3 status summary, §4.2 ordering,
   §5 landed entry) land in the same diff — partial updates would leave
   the roadmap internally inconsistent.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and
   existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `grep -n "Cognitive archetypes\|CognitiveArchetypes"
   docs/scenario-roadmap.md
   crates/worldwake-cli/src/bin/scenario_coverage.rs` — verify the
   feature name appears in both files and the text matches.
2. `scripts/verify.sh` — full pre-PR gate (catches markdown parse errors,
   broken intra-doc references if any).
