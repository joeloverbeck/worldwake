# CI Dependency Automation & Advisory Scanning

## Brainstorm Context

- **Original request**: Evaluate whether a Dependabot workflow is needed for this Rust repo, and identify other realistic CI automations worth adding.
- **Reference file**: none.
- **Key interview insights**:
  - Repo is public (`github.com/joeloverbeck/worldwake`), solo author.
  - Project is determinism-critical (golden replay hashes, `blake3` canonical hashing, seeded `ChaCha8Rng`, no floats, `forbid unsafe_code`). Dep updates can silently perturb replay.
  - Authored dep surface is tiny (`serde`, `bincode`, `rand_chacha`, `blake3`); 106 transitive crates.
  - Existing workflows: `ci.yml`, `golden-survival.yml`, `golden-drive-escalation.yml`. No `dependabot.yml`, no advisory scan, rustfmt installed but unenforced.
  - Workspace is currently `cargo fmt --all -- --check` clean — enabling the check introduces zero churn.
  - User chose **Conservative** posture: no routine Cargo-update PRs; rely on deliberate manual `cargo update` to protect determinism. Confirmed preference for Approach A (minimal scope).
- **Final confidence**: 92% at approach selection, 100% at design approval. Assumptions: `rustsec/audit-check@v2` remains the canonical action; if it's deprecated, substitute `cargo install cargo-audit` + direct invocation.

## Classification

Implementation-adjacent. No simulation behavior changes. No FOUNDATIONS.md principles touched.

## Deliverables

Three independent, reversible additions:

1. **`.github/dependabot.yml`** — weekly `github-actions` ecosystem updates. Cargo ecosystem deliberately omitted.
2. **New `audit` job** in `.github/workflows/ci.yml` — `rustsec/audit-check@v2`, fails on RustSec advisories.
3. **`cargo fmt --all -- --check`** added as first step in `scripts/verify.sh`.

## Concrete Shapes

### `.github/dependabot.yml`

```yaml
version: 2
updates:
  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 5
```

### `ci.yml` addition — new `audit` job (parallel to `verify`)

```yaml
  audit:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: Run cargo audit
        uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

No toolchain install, no cache — `audit-check` reads `Cargo.lock` directly.

### `scripts/verify.sh` addition

Insert as the first verification step (cheapest, no compile required):

```bash
echo "[verify] cargo fmt --all -- --check"
cargo fmt --all -- --check
```

## Key Decisions

| Decision | Rationale |
|---|---|
| Exclude `cargo` from `dependabot.yml` | Each patch update risks perturbing `blake3` / `bincode` / `rand_chacha` output and burning full CI (including two golden matrices) on low-info PRs. Security alerts still surface via GitHub's always-on advisory UI. |
| Include only `github-actions` ecosystem | Supply-chain risk on pinned actions is the asymmetric win (~1 PR/month, nearly always clean). |
| `audit` as a separate CI job, not inside `verify` | Orthogonal concern; advisory failures diagnosed independently of build/test failures. No shared state needed. |
| `cargo fmt` at the top of `verify.sh` | Fails fast, no compile cost, closes the unenforced-rustfmt gap. Pinned toolchain (1.93.0) means no version drift. |
| No `cargo-deny`, no Cargo batch updates | Deferred. Reversible additions we can layer on if/when pain emerges — avoid paying speculative cost. |

## Edge Cases

- **Advisory for unfixable transitive dep**: add `.cargo/audit.toml` with `[advisories] ignore = ["RUSTSEC-YYYY-NNNN"]` and a comment, or pass via action input. Accepts noise to unblock; does not hide the advisory.
- **Dependabot bumps `dtolnay/rust-toolchain` or `Swatinem/rust-cache`**: `verify` + two golden workflows run on the PR, so regressions (including determinism breaks) surface before merge.
- **Action schema change breaks the `with:` block**: `verify` job fails on the PR. Easy revert.
- **rustfmt version drift across contributors**: pinned via `rust-toolchain.toml` channel `1.93.0`.

## Verification Plan

1. `bash scripts/verify.sh` locally → fmt + test + clippy all pass.
2. Push to a throwaway branch; confirm green: `verify`, `audit`, `golden-survival`, `golden-drive-escalation`.
3. After merge, verify on GitHub under **Insights → Dependency graph → Dependabot** that `github-actions` shows a recent "Last checked" timestamp.

## Non-Goals

- Auto-merge configuration for Dependabot PRs (solo review tax is already low at ~1/month).
- License policy enforcement, duplicate-crate detection (deferred; add `cargo-deny` if future pain justifies it).
- Coverage reporting, CodeQL (low-ROI for a research prototype).
- Release/publishing workflows (premature — no published crates).
