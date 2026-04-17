# Per-Family Golden-Scenario CI Workflows — Design

## Brainstorm Context

- **Original request**: After deleting 4 orphaned workflows (long-scenarios, replay-consistency, soak, stress-disruptions) whose underlying test files were removed in S104-001, design replacement workflows scoped per scenario, running only in CI (not locally), since they take much longer than most golden e2e suites.
- **Reference file**: none (inline context only).
- **Key interview insights**:
  - Local-exclusion mechanism: `#[ignore]` attribute — idiomatic Rust, no file moves or feature gating needed.
  - Workflow granularity: family-per-matrix-workflow — scales to many future scenarios without linear YAML growth.
  - Starting point: `golden-survival.yml` covering the 3 existing survival scenarios; new family workflows (`golden-combat.yml`, etc.) land as families appear.
- **Final confidence**: 93–95%. Remaining ambiguity is empirical (timeout budget, whether every PR runs the matrix) — resolved after first CI run, not by further design.
- **Classification**: implementation-adjacent (CI + test organization; no simulation behavior change).

## 1. Overview

Gate the 3 survival golden tests with `#[ignore = "..."]` so local `cargo test --workspace` skips them silently. Run them in CI via a new `.github/workflows/golden-survival.yml` — a single workflow with a 3-entry matrix (one job per scenario). Establish *one matrix workflow per scenario family* as the convention for future growth.

## 2. Test annotation change

In each of the following files, add `#[ignore = "CI-only: long-running 1440-tick scenario; run via golden-survival workflow"]` to every `#[test]` fn:

- `crates/worldwake-ai/tests/golden_survival_baseline.rs`
- `crates/worldwake-ai/tests/golden_survival_contested.rs`
- `crates/worldwake-ai/tests/golden_survival_scattered.rs`

No file relocation, no feature flags, no env vars, no helper macros.

**Consequences:**
- `cargo test --workspace` (local, via `scripts/verify.sh`) — skipped silently.
- `cargo test` on the individual file — skipped silently.
- Manual local run: `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`.
- `ci.yml` `verify` job unchanged — already uses `cargo test --workspace`, which skips ignored tests by default.

## 3. Workflow: `.github/workflows/golden-survival.yml`

```yaml
name: Golden Survival
on:
  push:
    branches: [main, master]
  pull_request:
concurrency:
  group: golden-survival-${{ github.ref }}
  cancel-in-progress: true
jobs:
  scenario:
    name: golden-survival / ${{ matrix.scenario }}
    runs-on: ubuntu-latest
    timeout-minutes: 30
    strategy:
      fail-fast: false
      matrix:
        scenario: [baseline, contested, scattered]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { toolchain: 1.93.0 }
      - uses: Swatinem/rust-cache@v2
        with: { key: golden-survival-${{ matrix.scenario }} }
      - name: Run golden_survival_${{ matrix.scenario }}
        run: cargo test --release -p worldwake-ai --test golden_survival_${{ matrix.scenario }} -- --ignored --test-threads=1
```

### Key decisions

| Choice | Rationale |
|---|---|
| `--release` | 1440-tick runs are meaningfully slower in debug; matches prior long-test workflow practice. |
| `--test-threads=1` | Deterministic; avoids CPU contention across the ~5 `#[test]` fns per file. |
| `fail-fast: false` | One scenario failing doesn't mask failures in the others. |
| `concurrency` + `cancel-in-progress` | Long jobs; cancelling stale PR pushes saves runner minutes. |
| `timeout-minutes: 30` | Generous ceiling; tighten empirically after first CI runs. |
| Per-matrix cache key | Avoids cross-scenario invalidation; isolated `target/` reuse per job. |
| Matching toolchain (1.93.0) to `ci.yml` | Single source of truth avoids drift. |

## 4. Scaling convention

Document in the design doc and as a comment at the top of `golden-survival.yml`:

> *Family-per-matrix-workflow. Add scenarios to an existing family's matrix; create a new `golden-<family>.yml` when a new family lands (combat, trade, exploration, social, …).*

**Growth model:** YAML files proportional to *family count*, not scenario count. Each matrix entry is ~2 lines; each new family is one new file copied from this one.

**Deferred decision:** Once total matrix runtime strains PR feedback loops (empirically, once aggregate minutes cross some threshold), migrate from `pull_request` trigger to `main`-only, scheduled, or label-gated runs. Not addressed now — out of scope until signal shows it matters.

## 5. Edge cases

- **Adding a scenario mid-PR**: append one line to `matrix.scenario`. Naming contract: `scenarios/survival-<name>.ron` ↔ `tests/golden_survival_<name>.rs` ↔ matrix entry `<name>`. Enforced by convention — CI fails loudly on violation.
- **Shared test helpers**: already live in `tests/golden_harness/`; no workflow changes when helpers evolve.
- **Branch protection drift**: require the *workflow* (`Golden Survival`) in branch protection, not individual job rows — so adding matrix entries doesn't require branch-protection updates.
- **Dev forgets `--ignored`**: harmless — tests skip silently, no false positive. CI catches real regressions on push.
- **Scenario file renamed**: update the matrix entry and the test filename together; the tight 1:1:1 naming contract makes drift visible.

## 6. Testing strategy

Two commands verify the design before push:

1. `cargo test --workspace` — all 3 survival tests report as `ignored` in output; nothing else regresses. Confirms local exclusion.
2. `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored` — actually runs the baseline suite. Confirms the CI command works.

Third verification: observe the matrix execute on this PR after push.

## 7. FOUNDATIONS alignment

Implementation-adjacent — no simulation behavior change, no formal alignment table required. Relevant nod: **FND-31 (Validation First-Class)** — moving long tests to CI-only preserves local DX without weakening validation, since CI still runs them on every PR.

## 8. Files touched

| File | Change |
|---|---|
| `crates/worldwake-ai/tests/golden_survival_baseline.rs` | `#[ignore = "..."]` on each `#[test]` fn |
| `crates/worldwake-ai/tests/golden_survival_contested.rs` | `#[ignore = "..."]` on each `#[test]` fn |
| `crates/worldwake-ai/tests/golden_survival_scattered.rs` | `#[ignore = "..."]` on each `#[test]` fn |
| `.github/workflows/golden-survival.yml` | New file (see §3) |

No changes to: `scripts/verify.sh`, `.github/workflows/ci.yml`, `Cargo.toml`, or any scenario `.ron`.
