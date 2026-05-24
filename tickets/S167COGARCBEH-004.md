# S167COGARCBEH-004: Dedicated CI workflow lane for archetype golden

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S167COGARCBEH-002, [`specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md)

## Problem

The behavioral-divergence golden (S167COGARCBEH-002) proves FND-31's "authored
causal reason" — that two archetype-distinguished agents choose differently for
the documented profile-weight reason. The proof is fragile by design: a future
profile retune that erases the divergence must fail loudly. Batching the
archetype golden into a shared CI lane risks the failure being hidden by
unrelated noise; isolating it into a dedicated `golden-cognitive-archetypes.yml`
keeps the proof visible on every PR.

The convention for per-family golden workflows is established
(`.github/workflows/golden-drive-escalation.yml` and six sibling
`golden-*.yml` lanes). This ticket adds the new lane.

## Assumption Reassessment (2026-05-24)

1. Existing CI workflow lanes: seven `golden-*.yml` files under
   `.github/workflows/` (verified during reassessment):
   `golden-drive-escalation.yml`, `golden-item-decay.yml`,
   `golden-observer-anomalies.yml`, `golden-planner-pathology.yml`,
   `golden-scenario-diagnostics.yml`, `golden-simulation-gaps.yml`,
   `golden-survival.yml`. The convention is matrix-per-family per the
   header comment in `golden-drive-escalation.yml`: "add scenarios to the
   matrix below; create a new golden-<family>.yml when a new scenario
   family lands."
2. The spec
   ([`specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md))
   D5 commits to a new `.github/workflows/golden-cognitive-archetypes.yml`
   modeled on `golden-drive-escalation.yml`. The matrix entry must
   reference S167COGARCBEH-002's new test module (`cognitive_archetypes_divergence`).
3. Shared boundary under audit: the CI workflow shape contract — same
   triggers (`push` to `main`/`master`, `pull_request`), same
   `concurrency` group pattern, same toolchain pin convention, same
   `cargo test --release -p worldwake-ai --test golden_ai --
   --ignored --test-threads=1 ${{ matrix.filter }}` invocation. The lane
   must match these patterns so future maintenance can apply
   per-pattern updates uniformly across all golden lanes.
4. The toolchain version pinned in sibling lanes is currently `1.93.0`
   (verified at `.github/workflows/golden-drive-escalation.yml:36`). Match
   whatever value the sibling lanes use at the time this ticket lands
   rather than hardcoding the version in this ticket text — the project
   may have advanced toolchain in the interval. The implementer must
   reread a sibling workflow at implementation time and copy the live
   value.

## Architecture Check

1. **Per-family matrix lane over shared lane** — adding the archetype
   golden to an existing batched lane would couple its visibility to
   unrelated tests' pass/fail noise. The per-family convention exists
   precisely because each family's proof contract is distinct and
   independent regression visibility matters. The convention header
   comment in `golden-drive-escalation.yml` documents this intent.
2. **Matrix shape from day one over single-test inline** — the matrix
   form (even with one entry) lets future archetype pairs land as
   additional matrix entries without restructuring the workflow. This
   matches the spec's Follow-ups section that anticipates `Bold vs
   Methodical`, `Sociable vs Skeptical`, and other pairs as future
   sibling matrix entries.
3. **Modeled on `golden-drive-escalation.yml` specifically** — that lane
   uses a single-scenario matrix today, which is the closest analog to
   this lane's initial state. Copying its shape minimizes
   convention-drift across the family-lane set.

## Verification Layers

1. CI lane invocation correctness -> the workflow lane runs on the next
   PR after landing and the matrix entry's `filter` value picks up the
   two tests authored in S167COGARCBEH-002. Verified by inspecting the
   workflow run output on the PR that lands this ticket.
2. Workflow file shape conformance -> manual diff against
   `golden-drive-escalation.yml` confirms parallel structure: triggers,
   concurrency, jobs, matrix, steps, toolchain pin, test invocation.
3. Single-layer ticket: this is CI infrastructure. Items 4–6 of the
   template's Verification Layers are not applicable — no decision
   trace, action trace, or event-log delta is involved.

## What to Change

### 1. Create `.github/workflows/golden-cognitive-archetypes.yml`

Copy `.github/workflows/golden-drive-escalation.yml` and adapt:

```yaml
# Family-per-matrix-workflow convention: add scenarios to the matrix below; create a
# new golden-<family>.yml when a new scenario family lands (combat, trade, exploration, …).
# See docs/plans/2026-04-17-per-family-golden-scenario-workflows-design.md.
name: Golden Cognitive Archetypes

on:
  push:
    branches:
      - main
      - master
  pull_request:

concurrency:
  group: golden-cognitive-archetypes-${{ github.ref }}
  cancel-in-progress: true

jobs:
  scenario:
    name: golden-cognitive-archetypes / ${{ matrix.scenario }}
    runs-on: ubuntu-latest
    timeout-minutes: 30
    strategy:
      fail-fast: false
      matrix:
        include:
          - scenario: cognitive_archetypes_divergence
            filter: "scenarios::cognitive_archetypes_divergence::"

    steps:
      - name: Checkout
        uses: actions/checkout@v6

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: <copy-from-sibling-lane-at-write-time>
          components: clippy,rustfmt

      - name: Cache cargo artifacts
        uses: Swatinem/rust-cache@v2
        with:
          key: golden-cognitive-archetypes-${{ matrix.scenario }}

      - name: Run golden_ai (${{ matrix.scenario }})
        run: cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 ${{ matrix.filter }}
```

Important: replace `<copy-from-sibling-lane-at-write-time>` with the
current toolchain value used by `golden-drive-escalation.yml` (currently
`1.93.0`, but verify at write time — the project may have advanced).

### 2. Verify the test filter resolves

After the workflow lands, the next PR run should execute exactly the two
tests authored in S167COGARCBEH-002
(`scenarios::cognitive_archetypes_divergence::forward` and
`scenarios::cognitive_archetypes_divergence::counterfactual_symmetry`).
Confirm by inspecting the workflow run logs on the PR. If the filter does
not match, narrow it to the exact module path observed in
`cargo test -p worldwake-ai --test golden_ai -- --list | grep
cognitive_archetypes_divergence`.

## Files to Touch

- `.github/workflows/golden-cognitive-archetypes.yml` (new)

## Out of Scope

- Authoring the scenario file — owned by S167COGARCBEH-001.
- Authoring the golden test — owned by S167COGARCBEH-002.
- Adding scenario-roadmap entry citing this workflow path — owned by
  S167COGARCBEH-003.
- Adding additional archetype pairs as matrix entries — reserved for
  future specs per the spec's Follow-ups section (`Bold vs Methodical`,
  `Sociable vs Skeptical`, etc.). Those land as additional matrix
  entries in this lane, not as additional workflow files.
- Modifying any sibling `golden-*.yml` lane.
- Changing the toolchain version used by sibling lanes — this ticket
  matches whatever value is current at write time.

## Acceptance Criteria

### Tests That Must Pass

1. The new workflow runs successfully on the PR that lands it AND on the
   PR that subsequently lands S167COGARCBEH-002's tests (verifying the
   matrix `filter` resolves to the intended test set).
2. The two cognitive-archetypes-divergence tests appear in the workflow's
   output on a passing run.
3. Existing suite: all sibling `golden-*.yml` lanes continue to run
   unchanged.

### Invariants

1. The workflow file shape (triggers, concurrency, jobs, matrix, steps,
   toolchain pin, test invocation) matches the sibling
   `golden-drive-escalation.yml` pattern. Deviation requires a documented
   reason in the workflow's header comments.
2. The matrix `filter` value picks up exactly the
   `scenarios::cognitive_archetypes_divergence::` module path — no
   unrelated tests run in this lane and no archetype tests are missed.
3. The toolchain version matches the value in sibling lanes at write
   time (not hardcoded to `1.93.0` if the project has advanced).

## Test Plan

### New/Modified Tests

1. `None — CI-infrastructure ticket; verification is the workflow's own
   pass/fail status on the PR that lands it and the PR that lands
   S167COGARCBEH-002's tests.`

### Commands

1. `diff .github/workflows/golden-cognitive-archetypes.yml
   .github/workflows/golden-drive-escalation.yml` — confirm the diff is
   limited to: workflow name, concurrency group name, matrix scenario
   name and filter, cache key, run-step display name. Anything else is
   convention drift and warrants review.
2. After landing: inspect the workflow run on the PR via GitHub Actions
   UI; confirm the matrix entry runs and the two
   `cognitive_archetypes_divergence` tests appear in the test output.
3. `scripts/verify.sh` (full pre-PR gate; does not exercise GitHub
   Actions itself but catches local regressions in the test that the
   workflow will run).
