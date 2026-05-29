# S175CIOWN-001: Make S175 exhaustion goldens CI-owned and correct the roadmap claim

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — CI workflow + docs only
**Deps**: None (S175 is implemented and archived; this only wires its CI ownership)

## Problem

Before this ticket, the two S175 exhaustion goldens were `#[ignore]` "CI-only"
tests whose ignore messages and the scenario roadmap both claimed they ran via
the golden-survival workflow, but the workflow's matrix did not include them, so
no workflow actually ran them. This was a proof-integrity defect: the project
documented CI-owned focused proof that did not exist. It was the P0 of the
2026-05-29 third-iteration Cluster 1 report and is independently confirmed below.

This ticket is a correctness/honesty fix and is **not** subject to the gameplay
hold that governs the S176–S178 specs in `specs/IMPLEMENTATION-ORDER.md`; it may be
implemented immediately.

## Assumption Reassessment (2026-05-29)

1. **The two goldens exist, are registered, and are `#[ignore]`.** CONFIRMED.
   `crates/worldwake-ai/tests/scenarios/survival_exhaustion_collapse.rs` and
   `…/survival_exhaustion_recovery.rs` are registered in
   `crates/worldwake-ai/tests/scenarios/mod.rs` (`pub mod survival_exhaustion_collapse;`
   / `pub mod survival_exhaustion_recovery;`). Each file holds two `#[ignore]`
   tests: `scenario_a_exhaustion_collapse` + `…_replays_deterministically`, and
   `scenario_b_exhaustion_recovery` + `…_replays_deterministically`. Ignore
   messages: `"CI-only: long-horizon exhaustion collapse; run via golden-survival workflow"`
   and `"CI-only: travel-and-recover survival horizon; run via golden-survival workflow"`.
2. **Before this ticket, `golden-survival.yml` did not run them, and no other
   workflow did.** CONFIRMED. The pre-change matrix in
   `.github/workflows/golden-survival.yml` had 17 entries; none was
   `survival_exhaustion_collapse` or `survival_exhaustion_recovery`. A pre-change
   grep for `exhaustion` across all `.github/workflows/*.yml` returned zero matches.
3. **Before this ticket, the roadmap repeated the false claim.** CONFIRMED.
   `docs/scenario-roadmap.md` §5.19 stated the two S175 goldens were `#[ignore]`
   CI-only "(run via the golden-survival workflow, per the long-horizon-collapse
   convention)". The parenthetical was false before this ticket landed.
4. **The run command runs ignored tests by filter.** CONFIRMED. The workflow step
   runs `cargo test --release -p worldwake-ai --test golden_ai -- --ignored
   --test-threads=1 ${{ matrix.filter }}`. `--ignored` runs only ignored tests, so
   a matrix entry with filter `scenarios::survival_exhaustion_collapse::` will run
   both Scenario-A tests; `scenarios::survival_exhaustion_recovery::` runs both
   Scenario-B tests. This mirrors the existing `items_decay` entry exactly.
5. **No engine or test-code change is needed.** The tests are correct and
   complete; only the workflow matrix and one roadmap line are wrong. The fix must
   not weaken or modify the goldens themselves (AGENTS.md: never adapt tests).

## Architecture Check

1. Adding two matrix entries to the existing per-family workflow is the project's
   own documented convention (`golden-survival.yml` header: "add scenarios to the
   matrix below"). No new workflow file is warranted — exhaustion is part of the
   survival family. This is cleaner than a dedicated `golden-exhaustion.yml`, which
   would fragment the survival family without benefit.
2. No backward-compatibility aliasing or shims; the roadmap edit replaces a false
   claim with a true one.

## Verified Layers

1. **The two goldens are CI-owned** -> `golden-survival.yml` has matrix entries for
   `golden-survival / exhaustion_collapse` and
   `golden-survival / exhaustion_recovery`.
2. **Local equivalence before push** -> the exact release `golden_ai --ignored`
   filters both passed locally, including the behavioral and
   `_replays_deterministically` tests for each scenario family.
3. **Roadmap claim is true** -> `docs/scenario-roadmap.md` §5.19 references the live
   `exhaustion_collapse` / `exhaustion_recovery` matrix entries; no remaining
   roadmap text asserts CI ownership that the matrix does not back.

## Landed Changes

### 1. Added two matrix entries to `golden-survival.yml`

Inserted two entries near `escort` / `final_integration`, mirroring the existing
pattern:

```yaml
          - scenario: exhaustion_collapse
            filter: "scenarios::survival_exhaustion_collapse::"
          - scenario: exhaustion_recovery
            filter: "scenarios::survival_exhaustion_recovery::"
```

### 2. Corrected `docs/scenario-roadmap.md` §5.19

The roadmap now says the S175 exhaustion goldens are CI-only through the
`exhaustion_collapse` and `exhaustion_recovery` matrix entries in
`golden-survival.yml`. The framing remains focused **auxiliary** proof, not
long-running collision proof.

### 3. Left ignore-message wording unchanged

The `#[ignore]` messages already say "run via golden-survival workflow," which
is now true. No golden logic changed.

## Landed Files

- `.github/workflows/golden-survival.yml` (modify)
- `docs/scenario-roadmap.md` (modify — §5.19)
- `specs/IMPLEMENTATION-ORDER.md` (status truth-sync for this ticket)

## Outcome

Completed on 2026-05-29.

- Added `exhaustion_collapse` and `exhaustion_recovery` to the existing
  golden-survival matrix.
- Corrected the roadmap's S175 CI-ownership wording to name the live matrix
  entries.
- Left the S175 golden test bodies and ignore messages unchanged because the
  workflow ownership claim is now backed by CI configuration.

## Verification Result

- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_exhaustion_collapse::` (2 passed).
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_exhaustion_recovery::` (2 passed).
