# S167COGARCBEH-004: Dedicated CI workflow lane for archetype golden

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: [`archive/tickets/S167COGARCBEH-002.md`](S167COGARCBEH-002.md), [`archive/specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md)

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
   ([`archive/specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md))
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

## Verified Layers

1. CI lane invocation correctness -> locally verified with the exact workflow
   command after adding the required ignored-test metadata:
   `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::cognitive_archetypes_divergence::`
   executed exactly the two S167 tests.
2. Workflow file shape conformance -> manual diff against
   `golden-drive-escalation.yml` confirms parallel structure: triggers,
   concurrency, jobs, matrix, steps, toolchain pin, test invocation.
3. Single-layer ticket: this is CI infrastructure. Items 4–6 of the
   template's Verification Layers are not applicable — no decision
   trace, action trace, or event-log delta is involved.

## Landed Changes

### 1. Created `.github/workflows/golden-cognitive-archetypes.yml`

Copied `.github/workflows/golden-drive-escalation.yml` and adapted:

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
          toolchain: 1.93.0
          components: clippy,rustfmt

      - name: Cache cargo artifacts
        uses: Swatinem/rust-cache@v2
        with:
          key: golden-cognitive-archetypes-${{ matrix.scenario }}

      - name: Run golden_ai (${{ matrix.scenario }})
        run: cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 ${{ matrix.filter }}
```

The landed toolchain value is `1.93.0`, copied from
`golden-drive-escalation.yml` at implementation time.

### 2. Verified the test filter resolves

The exact workflow command executes the two tests authored in S167COGARCBEH-002
(`scenarios::cognitive_archetypes_divergence::forward` and
`scenarios::cognitive_archetypes_divergence::counterfactual_symmetry`).
The first attempted workflow-shaped run executed zero tests because those
tests were not marked ignored yet; this ticket corrected that S167 golden
metadata so the lane is non-empty.

## Landed Files

- `.github/workflows/golden-cognitive-archetypes.yml` (new)
- `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs`
  (updated only the two `#[ignore]` metadata annotations so the
  workflow's `--ignored` invocation executes the S167 golden tests)

## Out of Scope

- Authoring the scenario file — completed in
  [`archive/tickets/S167COGARCBEH-001.md`](S167COGARCBEH-001.md).
- Authoring the golden test — owned by S167COGARCBEH-002.
- Adding scenario-roadmap entry for this auxiliary proof row — completed in
  [`archive/tickets/S167COGARCBEH-003.md`](S167COGARCBEH-003.md).
- Adding additional archetype pairs as matrix entries — reserved for
  future specs per the spec's Follow-ups section (`Bold vs Methodical`,
  `Sociable vs Skeptical`, etc.). Those land as additional matrix
  entries in this lane, not as additional workflow files.
- Modifying any sibling `golden-*.yml` lane.
- Changing the toolchain version used by sibling lanes — this ticket
  matches whatever value is current at write time.

## Acceptance Result

### Tests That Passed Or Remain PR-Owned

1. Passed locally with the exact workflow command: the matrix `filter`
   resolves to the intended test set and executes the two S167 tests.
2. Passed locally: both `cognitive_archetypes_divergence` tests appear in
   the exact workflow command output.
3. Waived PR-only workflow-run inspection until the branch is pushed and
   GitHub Actions runs the new lane; the local command covers the same
   Cargo invocation and filter.
4. Passed by diff inspection: sibling `golden-*.yml` lanes were not
   modified.

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

## Test Plan Result

### Added/Modified Test Surfaces

1. No new tests were added. The existing S167 golden tests gained
   `#[ignore]` metadata so the dedicated CI lane's `--ignored` command
   actually runs them.

### Commands Run

1. Passed `cargo test -p worldwake-ai --test golden_ai -- --list cognitive_archetypes_divergence`
   (listed `scenarios::cognitive_archetypes_divergence::forward` and
   `scenarios::cognitive_archetypes_divergence::counterfactual_symmetry`).
2. The first `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::cognitive_archetypes_divergence::`
   run executed zero tests, exposing the missing ignored-test metadata.
3. Passed `cargo fmt --all`.
4. Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::cognitive_archetypes_divergence::`
   (2 passed).
5. Passed `diff -u .github/workflows/golden-drive-escalation.yml .github/workflows/golden-cognitive-archetypes.yml`
   by inspection; the diff is limited to workflow name, concurrency group,
   matrix scenario/filter, and cache key.
6. Waived `scripts/verify.sh` for this per-ticket closeout because the
   `implement-spec-tickets` harness owns the full pre-PR gate before the
   final branch push.

## Outcome

Completed on 2026-05-24.

- Added `.github/workflows/golden-cognitive-archetypes.yml` as a
  per-family golden workflow lane matching the live
  `golden-drive-escalation.yml` structure and Rust toolchain pin.
- Set the lane matrix to
  `scenarios::cognitive_archetypes_divergence::`.
- Added ignored-test metadata to the two S167 cognitive archetype golden
  tests so the workflow's `--ignored` command executes the intended tests
  instead of passing with zero tests.

## Deviations

- The ticket originally listed only the workflow file as touched. Live
  verification showed the drafted workflow command executed zero tests
  until the two S167 golden tests were marked ignored, matching the
  repository's existing per-family golden lane convention.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_ai -- --list cognitive_archetypes_divergence`.
- Passed `cargo fmt --all`.
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::cognitive_archetypes_divergence::`.
- Passed workflow diff inspection against `.github/workflows/golden-drive-escalation.yml`.
- Waived `scripts/verify.sh` for this ticket closeout because the
  `implement-spec-tickets` harness owns the final pre-push verification
  gate for the S167 family.
