# S167COGARCBEH-003: Formalize archetype proof row in scenario-roadmap.md

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: [`archive/tickets/S167COGARCBEH-001.md`](S167COGARCBEH-001.md), [`archive/tickets/S167COGARCBEH-002.md`](S167COGARCBEH-002.md), [`specs/S167-cognitive-archetype-behavioral-proof.md`](../../specs/S167-cognitive-archetype-behavioral-proof.md)

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
   ([`specs/S167-cognitive-archetype-behavioral-proof.md`](../../specs/S167-cognitive-archetype-behavioral-proof.md))
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
5. Live reassessment at the time of this ticket found that
   `.github/workflows/golden-cognitive-archetypes.yml` did not exist yet because
   S167COGARCBEH-004 owned that CI lane. Outcome amended 2026-05-24:
   S167COGARCBEH-004 later landed the workflow at that path.

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

## Verified Layers

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

## Landed Changes

### 1. Add catalog row in §2 Gameplay Feature Catalog

Added a new row to the table in `docs/scenario-roadmap.md`:

```markdown
| Cognitive archetypes | Per-agent `AgentDef.archetype` set OR `archetype_assignment_policy` authored | [`cognitive_archetype.rs`](../../crates/worldwake-core/src/cognitive_archetype.rs), [`scenario/mod.rs`](../../crates/worldwake-cli/src/scenario/mod.rs) spawn-time delta application | Landed in [§5.18](#518-landed-auxiliary-cognitive-archetypes-divergence) |
```

(Section number `5.18` is illustrative — verify the next available
sub-section number in §5 at write time.)

The activation signal text must match `cognitive_archetypes_status(def)` at
`crates/worldwake-cli/src/bin/scenario_coverage.rs:865-872` — it checks both
`scenario_def.archetype_assignment_policy` and per-agent `archetype`.

### 2. Added ordering table row in §4.2 Ordered Roadmap

Added a new row at the end of the table after row 17 `final-integration`:

```markdown
| 18 | `cognitive-archetypes-divergence` | Cognitive archetypes (auxiliary behavior coverage) | Landed | Auxiliary behavior-proof row; not subject to 1440-tick survival-health contract. Owns the FND-31 archetype-decision causal proof. |
```

### 3. Added landed entry in §5 Landed Scenarios

Added `### 5.18 Landed Auxiliary: cognitive-archetypes-divergence` using the
§4.1 Entry Contract Template:

```markdown
### 5.18 Landed Auxiliary: cognitive-archetypes-divergence

**Status**: Landed
**Source scenario**: [`scenarios/cognitive-archetypes-divergence.ron`](../../scenarios/cognitive-archetypes-divergence.ron)
**Backing goldens**: [`cognitive_archetypes_divergence.rs`](../../crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs)
**Depends on**: archived [S152 archetype implementation](../specs/S152-cognitive-archetypes-seeded-diversity.md)

This row is **auxiliary behavior coverage**, not a survival-coexistence
landing. It owns the FND-31 archetype-decision causal proof: two same-role
same-belief agents differing only by `CognitiveArchetype` choose different
actions under identical local facts, with the decision trace and a test-side
profile-delta attribution naming the cause (Greedy's lower
`RoutePreferenceProfile.dangerous_traversal_penalty` making the mixed-history
Risky Orchard route cheaper than the neutral Sheltered Cut route). A
counterfactual archetype-swap replay asserts the divergence reverses
correspondingly.

The row has no `survival_health_contract` and runs on a short tick budget;
the proof is decision divergence + counterfactual symmetry + knowledge
legality, not 1440-tick coexistence. The scenario uses no new mechanics:
existing substrate primitives author the route-choice tension, and the
archetype delta flows through ordinary route-aware ranking/search.

The dedicated CI lane is landed at
[`.github/workflows/golden-cognitive-archetypes.yml`](../../.github/workflows/golden-cognitive-archetypes.yml).
```

### 4. Updated Status Summary in §3

Added the new auxiliary row to the §3 status table.

## Landed Files

- `docs/scenario-roadmap.md` (modify — 4 update points across §2, §3, §4.2,
  §5)

## Out of Scope

- Authoring the scenario file — completed in
  [`archive/tickets/S167COGARCBEH-001.md`](S167COGARCBEH-001.md).
- Authoring the golden test — owned by S167COGARCBEH-002.
- CI workflow file — landed later by S167COGARCBEH-004 at
  [`.github/workflows/golden-cognitive-archetypes.yml`](../../.github/workflows/golden-cognitive-archetypes.yml).
- Coverage doc regeneration — completed in
  [`archive/tickets/S167COGARCBEH-001.md`](S167COGARCBEH-001.md).
- Any change to `scenario_coverage.rs`'s `FEATURES` table or the
  `CognitiveArchetypes` feature row registration — already present and
  cited by the new catalog row.
- Adding feature catalog rows for any other coverage gap (portfolio
  weights, intention disposition, expectation store, last-seen memory,
  social observations) — explicit Non-Goal in the spec, reserved for
  follow-up specs.

## Acceptance Result

### Acceptance Proof

1. Passed: the catalog row's `Feature` column text matches `scenario_coverage.rs`'s
   `CognitiveArchetypes` feature name byte-for-byte (verified by grep).
2. Passed: all cited landed-entry source/proof paths resolve to existing files
   at review time (the source scenario, backing golden, and the active
   S167COGARCBEH-004 CI-lane owner ticket).
3. Waived `scripts/verify.sh` for this per-ticket closeout because the landed
   diff is non-generated Markdown only; the implement-spec-tickets harness final
   branch phase still owns the full pre-PR gate before push.

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

## Test Plan Result

### Focused Tests

1. `None — documentation-only ticket; verification is command-based and
   existing runtime coverage is named in Assumption Reassessment.`

### Commands Result

1. Passed `grep -n "Cognitive archetypes\|CognitiveArchetypes"
   docs/scenario-roadmap.md
   crates/worldwake-cli/src/bin/scenario_coverage.rs` — verify the
   feature name appears in both files and the text matches.
2. Passed `test -f scenarios/cognitive-archetypes-divergence.ron && test -f
   crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs &&
   test -f tickets/S167COGARCBEH-004.md` — verified cited S167 source/proof/owner
   while S167COGARCBEH-004 was still active
   paths resolve.
3. Passed `git diff --check -- docs/scenario-roadmap.md
   archive/tickets/S167COGARCBEH-003.md` — scoped Markdown whitespace hygiene.
4. Waived `scripts/verify.sh` for this per-ticket closeout because no source,
   generated, scenario, test, or executable behavior changed after the prior
   S167 proof tickets; the harness final branch phase still owns the full
   pre-PR gate before push.

## Outcome

Completed on 2026-05-24.

- Added the `Cognitive archetypes` gameplay-feature catalog row to
  `docs/scenario-roadmap.md`, aligned with the live
  `FeatureDef.name = "Cognitive archetypes"` and the
  `cognitive_archetypes_status(def)` activation logic.
- Added the §3 status-summary row, §4.2 ordered-roadmap row, and §5.18 landed
  auxiliary entry for `cognitive-archetypes-divergence`.
- Truth-synced this ticket's draft wording: the landed causal reason is the
  Greedy vs. Cautious route-preference penalty delta, not a portfolio economic
  weight trade, and the dedicated CI lane was left to S167COGARCBEH-004.

Outcome amended: 2026-05-24.

- S167COGARCBEH-004 landed the dedicated CI lane at
  [`.github/workflows/golden-cognitive-archetypes.yml`](../../.github/workflows/golden-cognitive-archetypes.yml).

## Deviations

- The roadmap entry originally cited active `tickets/S167COGARCBEH-004.md` as the
  CI-lane owner because the workflow was intentionally not created until
  S167COGARCBEH-004. Outcome amended 2026-05-24: the roadmap now links the landed
  workflow file.

## Verification Result

- Passed `grep -n "Cognitive archetypes\|CognitiveArchetypes" docs/scenario-roadmap.md crates/worldwake-cli/src/bin/scenario_coverage.rs`.
- Passed `test -f scenarios/cognitive-archetypes-divergence.ron && test -f crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs && test -f tickets/S167COGARCBEH-004.md` before S167COGARCBEH-004 was archived.
- Passed post-S167COGARCBEH-004 stale-reference repair for the landed workflow path.
- Passed `git diff --check -- docs/scenario-roadmap.md archive/tickets/S167COGARCBEH-003.md`.
- Waived `scripts/verify.sh` for this per-ticket closeout because the landed diff is non-generated Markdown only; the implement-spec-tickets final branch phase still owns the full pre-PR gate before push.
