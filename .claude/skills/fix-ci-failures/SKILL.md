---
name: fix-ci-failures
description: "Diagnose and fix GitHub Actions workflow failures on the current branch. Use when CI reports a failure after a push. Triages via gh CLI, classifies by failure taxonomy, reproduces locally, applies a FOUNDATIONS-aligned fix, gates the diff for user approval, then commits, pushes, and waits for the CI re-run to confirm green."
user-invocable: true
arguments:
  - name: target
    description: "Optional PR number (e.g. 42) or workflow run ID. Defaults to the current branch's most recent failing run(s)."
    required: false
---

# Fix CI Failures

Diagnose and fix failing GitHub Actions workflows on the current branch.

Read [CLAUDE.md](../../../CLAUDE.md) and [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md) before diagnosing or implementing any fix. Reproduce every failure locally before fixing — never rely on log inspection alone. Gate the diff for user approval before any commit or push.

## Workflow

### 1. Locate failing runs

1. If the user supplied a `target` argument, resolve it:
   - Numeric PR (e.g., `42`): `gh pr checks 42` to list failing checks; `gh pr view 42 --json statusCheckRollup` for details.
   - Workflow run ID: `gh run view <id>` to confirm the failing run.
2. Otherwise, detect the current branch and enumerate failing runs:
   - `git rev-parse --abbrev-ref HEAD` to get the branch name.
   - `gh run list --branch <branch> --status failure --limit 5` to list recent failures.
3. If the branch lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all file operations.
4. If multiple distinct runs failed (e.g., `CI` and `Golden Survival`), record all of them — each may need separate diagnosis.

### 2. Inspect failure logs

For each failing run:

1. `gh run view <run-id> --log-failed` to fetch only the failed-step logs.
2. Extract failure signatures — the actual error lines, not surrounding context. Look for:
   - `error[E####]:` (compiler errors)
   - `error: ... -D <lint>` (clippy)
   - `test result: FAILED. ... <name> ... FAILED` (test runner)
   - `Diff in <file> at line N` (rustfmt)
   - `Crate: <name>, Vulnerability: ...` (cargo audit)

   `--log-failed` returns the entire log of failing *steps*, which includes runner setup (toolchain install, cache restore, fetch) and may exceed 200KB for matrix workflows. Always pipe through `grep -E "FAILED|panicked|error\[|assertion|Diff in"` to extract signatures rather than reading whole output. Note that `--log-failed` prefixes every line with the job name (e.g., `golden-survival / trade`), the step, and a timestamp — so a grep pattern that includes the workflow or family name matches *every* line and returns only runner setup noise. (The step field may render as the literal `UNKNOWN STEP` for some runners — do not rely on it; grep by failure signature, not step name.) Identify the failed jobs first with `gh run view <id> --json jobs --jq '.jobs[] | select(.conclusion=="failure") | .name'`, then grep for failure signatures. worldwake test names frequently embed `fail`/`error`/`assertion`/`drift`/`mismatched` (e.g. `..._rejects_mismatched_...`, `classify_rejection_method_failure...`), so a bare signature grep over-matches *passing* (`... ok`) tests — append `| grep -v '\.\.\. ok'` and prefer line-anchored signatures (`test result: FAILED`, `^error`, `panicked at`, `error\[E`) over bare `assertion`/`error` tokens. For an aggregated single-step job (the `CI`/`verify` job runs fmt → test → clippy sequentially inside one `Verification` step), the failing gate is usually at the **tail** of `--log-failed` — run `tail -60` after the grep, since `--json jobs` narrows to the step, not the gate.
3. For matrix workflows (`golden-*.yml`), record which scenarios failed by their job names (e.g., `golden-survival / combat`).
4. If a single workflow has multiple failed jobs, capture each independently — they may have unrelated root causes.
5. If grepped output still exceeds the tool-result budget (~32KB), narrow further with `head -<N>`, target a specific job with `gh run view <id> --job <job-id> --log`, or filter by step name with `--log` and `awk '/##\[group\].*Run /{step=$0} step && /FAILED|panicked/{print step; print}'`.

### 3. Classify each failure

Use the taxonomy below to assign every failure to a class. Each class prescribes the exact local reproduction command and the standard remediation pattern.

| Class | Telltale log signature | Local repro command | Standard remediation |
|-------|------------------------|---------------------|----------------------|
| **rustfmt** | `cargo fmt --all -- --check` step fails; diff hunks in log | `cargo fmt --all -- --check` | `cargo fmt --all` |
| **build/compile** | `error[E####]:` in log | `cargo build --workspace` | Fix import / stale literal / type mismatch surfaced by compiler |
| **unit/integration test** | `test result: FAILED. ... <name> ... FAILED` | `cargo test -p <crate> <test_name>` | Diagnose: real bug vs. test drift. Never adapt test to bug |
| **clippy** | `error: ... -D <lint>` from `cargo clippy --workspace --all-targets -- -D warnings` | `cargo clippy --workspace --all-targets -- -D warnings` | Fix the lint. Never `#[allow]` to silence |
| **golden matrix scenario** | Job name `golden-<family> / <scenario>` failing; assertion line in log | `cargo test --release -p worldwake-ai --test golden_ai <scenario> -- --ignored --test-threads=1` (see note below) | Consult [docs/debugging-traces.md](../../../docs/debugging-traces.md). Treat goldens as authoritative; fix the production code. If the failing scenario depends on authored `.ron` data (e.g., last-seen `observed_kind`, belief fields) that a correct upstream change now requires, the fix may be completing that scenario data per the FND-1 truth-adjustment row (Step 7), not production code |
| **cargo audit** | `audit-check` job; `Crate: <name>, Vulnerability: ...` | `cargo audit` (after `cargo install cargo-audit` if missing) | Bump the dep in `Cargo.toml`. If no fix version exists, escalate per Step 5 |
| **toolchain/cache/checkout** | Failure in `Checkout`, `Install Rust toolchain`, or `Cache cargo artifacts` step | (no local equivalent) | Re-run via `gh run rerun <run-id>` once. If reproducible, escalate as infra issue, not as a code fix |
| **fixture drift** | `"scenario diagnostics fixture drifted; regenerate expected-scenario-diagnostics.json intentionally"` or a fixture-stable golden panics with no production code change | `cargo test --release -p worldwake-ai --test golden_ai scenario_diagnostics_fixture -- --ignored --test-threads=1` | Regenerate via `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1`; commit message must cite which intentional behavior change caused the drift (see Step 7's FND-1 row) |
| **workflow-config drift** | `error: no test target named X in <pkg>` (or a workflow step invoking a renamed/removed target, bin, bench, or example) | Run the workflow's exact command and confirm the target-resolution error | Update the `.github/workflows/*.yml` invocation to the current target/filter, verified against the crate's actual test targets via `cargo test -p <crate> --test <t> -- --list` or the `tests/` layout. This is a branch fix, unlike transient toolchain/cache infra |

The `--release --ignored --test-threads=1` flags on the golden matrix repro command are required because the per-family workflows (`.github/workflows/golden-<family>.yml`) gate these tests with `#[ignore = "CI-only: ..."]`. When in doubt, copy the exact `Run` step from the workflow YAML — it is the source of truth, **except** when the failure signature is `error: no test target named X` (or the invocation otherwise references a renamed/removed target): then the YAML is the artifact under repair, not the source of truth. Cross-check the invocation against the crate's actual targets via `cargo test -p <crate> --test <t> -- --list` or the `tests/` layout before trusting it (see the **workflow-config drift** taxonomy row).

When multiple specific tests in the same `--test` target fail, invoke them as separate runs — `cargo test` treats the second positional argument as something other than a test filter and rejects it with "unexpected argument". For broader filtering, use a shared substring as a single positional: `cargo test ... -- --ignored some_substring` matches anywhere in the test name. Because the match is a substring of the full test path (`scenarios::<module>::<fn>`), a bare scenario name can over-match function names in sibling modules — e.g. `survival_baseline` also matches the `scenario_diagnostics` tests whose function names embed `survival_baseline`. Anchor the filter as `scenarios::<scenario>::` to bind it to the module. Before trusting a green result, confirm the filter selects the intended tests with `cargo test ... --test golden_ai -- --ignored --list <filter>`: a filter that matches zero tests makes `cargo test` exit 0 ("0 filtered out") — a silently passing job that runs nothing, strictly worse than a red failure.

### 4. Reproduce locally

Hard requirement: every failure must reproduce locally before fixing.

1. Run the exact command prescribed by the class.
2. Confirm the failure signature matches what CI reported.
3. If the failure does not reproduce locally despite a clean checkout and matching toolchain (`rustup show` should match `1.93.0`), escalate per Step 5. Do not proceed with a hypothesis-driven fix.

`./scripts/verify.sh` runs the CI workflow gates *except the gated golden-matrix workflows* (`golden-*.yml`): it runs `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check` in order. Because `cargo test --workspace` does not run `#[ignore]` tests, verify.sh never exercises the per-family gated goldens — re-run the relevant gated family separately (see the golden-matrix taxonomy row). Use it as the canonical local gate, but run the narrowest class-specific repro first to avoid rebuilds.

Local-repro commands often produce huge stdout when goldens panic — failing-test backtraces and per-tick traces from observation harnesses commonly exceed the inline tool-result budget (~32KB), forcing the tool to dump to a temp file and surface only a small preview. To avoid the friction of re-reading temp files, run repros with `run_in_background: true` and grep the captured output with `grep -E 'FAILED|panicked|test result'`, or inline-pipe through `tail -N` / the same grep before the tool truncates.

Note: `command 2>&1 | tail -N` buffers output until the source command exits — the captured file appears empty mid-run, so it cannot be used for live progress checks. For long-running gates (`verify.sh`, full gated-golden families) where you want live progress visibility, redirect without piping (`command > log 2>&1`) and `tail -f log` (or `cat log`) directly, or rely on the harness completion notification rather than polling the captured file.

### 5. Diagnose and decide

Once the failure reproduces:

1. Read the failed test, lint, or assertion in the codebase. Trace to root cause.
2. For golden test failures: consult [docs/debugging-traces.md](../../../docs/debugging-traces.md). Goldens are authoritative — the bug is in production code unless evidence proves the golden's contract is itself stale (in which case CLAUDE.md's `Authoritative-To-AI Impact Rule` applies). **Obtaining a faithful trace**: the `observer` binary may not reproduce a golden's bespoke harness setup (e.g., Tick-0 belief seeding via `seed_actor_beliefs`), so an observer run can silently diverge from the golden's trajectory; when you need a faithful per-tick trace, instrument the golden harness directly with temporary `eprintln`, then revert it before staging — confirm via `git status` that no test or source instrumentation remains. The `observer` dump itself can reach tens of MB / hundreds of thousands of lines — never read it whole; `grep` targeted sections instead (per-agent action-timeline bins, the anomaly list, the Failed-Plan-Frequency breakdown, and the action-trace summary). A third possibility exists and is easy to miss: a FOUNDATIONS-alignment change (e.g., closing a belief-leak so `effective_place`/`can_control` no longer read remote world state) can be **correct** and still break a golden by **unmasking a separate pre-existing defect** that the now-removed behavior was silently compensating for. Signals: the changed production behavior is provably correct (covered by its own passing unit tests) and the failing agent is otherwise healthy (e.g., a near-identical whole-run action-count profile). When this pattern holds, the resolution is neither reverting the change nor adapting the golden — it is isolating the unmasked defect (which may or may not be in a different subsystem). **Before choosing between fixing it now and deferring, localize the unmasked defect to a specific file and mechanism, then write a one-paragraph tractability assessment.** A defect that sounds multi-subsystem at first ("the planner mis-ranks the branch") often reduces to a concrete, in-scope fix once localized (e.g., a missing recipe-feasibility prune in candidate generation that a per-tick trace surfaces). Only treat deferral-with-a-follow-up-ticket (Step 12) as the *recommended* path when that localization shows the fix genuinely spans multiple subsystems, or a quick spike to fix it has already failed; otherwise attempt the fix in this cycle. One alignment change may unmask a *heterogeneous* failure set rather than a single defect — some failures are tractable in-cycle, others are open-ended divergence regressions, and a derived artifact (e.g. a diagnostics fixture) may simply need intentional regeneration. Triage each failure independently (fix-now vs. regenerate vs. defer); do not assume one root implies one remediation. When you defer a subset, the follow-up ticket (Step 12) should enumerate which sibling failures share the root and which were fixed or regenerated in-cycle.
3. For any fix touching engine architecture, planner pipelines, action validation, or component registration: re-read the relevant sections of [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md).
4. When the failure is a regression and the branch has multiple commits since `main` (or since the last green CI run), consider bisecting via `git worktree add /tmp/<name>-bisect <commit-sha>` to isolate the introducing commit before deep code-reading. **Compute the candidate-commit set against `origin/main` (the actual PR base — `gh pr view --json baseRefName` / `git rev-parse origin/main`), not local `main`.** Local `main` may be ahead of or behind origin; `git log main..HEAD` then hides real candidate commits (a bundled, un-CI'd feature committed to local `main` looks like it is "already on main" when it is not — exactly the case that lets a large unvalidated change reach a PR diff unnoticed) or invents phantom ones. Confirm with `git rev-parse main origin/main` and prefer `origin/main..HEAD` for the candidate set. Before building an ancestor worktree, confirm the failing test or scenario file existed at that commit (`git cat-file -e <sha>:<path>` or `git log -1 <sha> -- <path>`) — a golden added mid-branch will not exist at an earlier bisect point, wasting a multi-minute release build. Run the narrowest local repro at each candidate commit. Worktrees are cheaper than `git checkout` and keep your main working tree clean. When only one or two files changed in the suspect commit, `git checkout <ancestor> -- <file>` in-place (then restore with `git checkout HEAD -- <file>`) reuses the existing `target/` for a fast incremental rebuild — cheaper than a fresh worktree build; reserve worktrees for whole-commit bisects or when many files changed. Clean up with `git worktree remove --force /tmp/<name>-bisect` for each worktree before committing — leftover worktrees pollute `git worktree list` and consume disk space, especially on space-constrained WSL2/VM disks. If the chosen remediation is a revert and `git revert <commit> --no-commit` conflicts on non-code files (e.g., an archived ticket that subsequent commits further modified), abort with `git revert --abort` and apply a targeted file-level revert via `git checkout <commit>^ -- <code-file1> <code-file2>` to restore only the source files. Verify scope with `git diff --stat HEAD` before staging. When the regression is in a golden/simulation, isolate the cause at the level of the changed code path — restore or revert the suspect function and confirm the whole run flips pass↔fail — **not** by comparing per-tick state between two versions. The simulation is deterministic but chaotic: two code versions diverge into different trajectories, so an agent's belief or world state at a fixed tick differs because its entire history diverged, making fixed-tick A/B state comparison misleading. Compare whole-run outcomes or action-count profiles instead.
5. If the failure reproduces locally but a prior CI run on an ancestor commit reported success for the same test, check 2-3 ancestor commits' CI runs (`gh run list --branch <branch> --workflow=<name>.yml --limit 10`) for that test before assuming the most recent commit introduced it. A single green CI run in the history may have been a flake; the actual regression may live further back. Bisecting from a "last green CI run" that was itself a flake will land on the wrong commit. For `#[ignore]`d gated-golden families (`golden-*.yml`), the branch's first PR run is often their first-ever execution — there is no branch-local ancestor history to check, because `cargo test --workspace` and verify.sh skip them during development. Confirm regression-vs-preexisting by checking the **merge-base / `main`** gated-golden history instead (`gh run list --branch main --workflow=golden-<family>.yml --limit 10`), not the branch's. A family that is green on `main` but red on the branch's first PR run is a branch-introduced regression even though the branch has no prior run of it.
6. TDD discipline (per CLAUDE.md): never adapt tests to fix bugs. For real bugs, the failing test is already the regression proof.
7. When the fix changes AI, planner, or observer behavior — especially reverting a prior change — run `git log --oneline -- <touched-files>` and grep `archive/tickets/` plus `crates/worldwake-*/tests/fixtures/` for the names of OTHER tests or fixtures previously calibrated to the behavior being changed. Calibrations downstream of the change (e.g., a follow-up ticket that adjusted an observer-anomaly count or regenerated a diagnostics fixture under the now-being-reverted behavior) will likely fail on the next push and should be addressed in the same commit batch. Skipping this scoping move drives the whack-a-mole pattern across consecutive cycles. The same scoping move applies at the CI layer: when the change renames, consolidates, or removes test targets, binaries, benches, or examples, grep `.github/workflows/` for references to the old names — every per-family or matrix workflow naming a removed target will fail on the next push and must be fixed in the same commit batch.
8. **Recently-shipped-feature regression pattern.** When the regression was introduced by a feature shipped on the *current* branch (not inherited from `main`), the pattern has three distinct sub-paths with fundamentally different remediations. When different failures in the same PR fit different sub-paths, apply each independently per point 5's heterogeneous-failure-set rule — for example, regenerate one fixture (Sub-path B) and ticket a sibling failure (Sub-path C) in the same fix cycle. Discriminate up front by asking: are the failing tests *tests of the new feature itself* (Sub-path A — its own goldens are red), are they *downstream test/fixture calibrations in unrelated scenarios whose authored contracts remain sensible* (Sub-path B — feature's own goldens stay green and the calibrations just need new values), or are they *downstream scenarios whose authored contracts now fail because the feature changed mechanics they implicitly relied on* (Sub-path C — feature's own goldens stay green but a previously-valid scenario contract is now violated)? Pick the sub-path(s) before evaluating any remediation.

   - **Sub-path A — feature itself broken or end-to-end unverified.** Evaluate whether the cleanest fix is to stub/disable the feature pending a separate follow-up cycle — especially when the feature touches engine/planner code whose end-to-end behavior is unverified by goldens. None of the standard 1-3-1 triggers (FND violation, 3+ failed fix variants) need fire; the relevant signal is "this feature shipped recently, broke things downstream, and proving the full fix requires more end-to-end coverage than this cycle can budget." Read the feature's archived spec under `archive/specs/` and its implementing tickets for self-assessed priority/risk — a spec that calls itself an "optimization" or "lowest-benefit" addition is a strong signal that disabling-with-follow-up is preferable to in-cycle redesign. The fix path then becomes: stub the producer/symbol, delete or update tests pinning the now-disabled behavior, write a follow-up ticket capturing the failure modes and the verification layers required before re-enable (see Step 12's feature-reactivation ticket shape).
   - **Sub-path B — feature correct and verified, downstream calibration stale.** When the feature's own goldens are green but unrelated downstream tests/fixtures (observer-anomaly assertions, scenario-diagnostics fixtures, indirect convergence-scenario calibrations, narrative inventories) red because they were calibrated before the feature shipped, the fix is Step 7's FND-1 truth-adjustment row — update the test assertion or regenerate the fixture in-cycle, never stub the feature. The commit message must cite the architectural change that motivated the new calibration plus the feature's own golden(s) that prove the underlying behavior is correct. Do NOT apply Sub-path A's stub-and-defer in this case: the feature is functioning as designed and reverting it would itself be a regression. This sub-path is also the typical signature when gated `golden-*.yml` workflows red on a branch's first PR run while `verify.sh` stayed green during development — the feature's verification matched the developer's gated-test scope, but downstream calibrations only surfaced on PR.
   - **Sub-path C — collateral real-behavior regression, feature's own goldens green.** When a previously-green scenario whose authored contract is reasonable now fails because the feature changed mechanics the scenario implicitly relied on (e.g., a survival-contract assertion the feature did not target but whose preconditions the feature shifted), the remediation is: (a) attempt in-cycle localization per the localization checkpoint earlier in this step — Sub-path C is *not* an automatic deferral; (b) if the fix is genuinely out of this cycle's scope, file a follow-up ticket naming the localization surface and the invariants the fix must respect (do not stub the feature, do not relax the scenario contract); (c) ship any sibling Sub-path B calibration fixes in the same push so CI recovers the workflows that *can* recover. The originating workflow stays red by design until the ticket lands; say so explicitly in the post-push report. Use `tickets/_TEMPLATE.md` (the regular template — the feature-reactivation shape in Step 12 is specific to Sub-path A stubbing). **Discriminating B from C when the failing assertion is a scenario contract**: changing a scenario's assertion is a legitimate FND-1 truth-adjustment (Sub-path B) *only* when the new behavior is desirable emergence produced by a dampener that still engages; when the failing state is an undampened-loop or stuck-equilibrium failure (the contract encodes a real outcome the feature broke), the contract is correct and must not be relaxed — that is Sub-path C, fix or defer the mechanism.
9. **Spec-intent reading before the 1-3-1.** Before presenting any 1-3-1 that disables or reverts a feature, read the originating spec (under `specs/` or `archive/specs/`) and the implementing ticket(s). The 1-3-1 options must be intent-aware: distinguish "preserve the dormant code (was the feature ever genuinely activated end-to-end?)" from "fix the activation gap in-cycle" from "defer with follow-up." Without this grounding, the 1-3-1 may miss the option the user would actually pick — the FOUNDATIONS read alone (Step 5 point 3) covers principle compliance but not design-intent fidelity.

Apply 1-3-1 (1 problem, 3 options, 1 recommendation) and stop for user direction when:

- A failure cannot be reproduced locally despite a clean checkout and matching toolchain. Options: (a) bisect against `main` to isolate the introducing commit, (b) re-run the CI job in case it's a flake, (c) write a follow-up ticket and defer the fix.
- Two or more plausible root causes exist for one failure.
- The minimal fix would require violating a FOUNDATIONS principle. Never proceed silently — surface the conflict.
- A `cargo audit` advisory has no fix version. Options: (a) pin to a non-vulnerable version range, (b) replace the dep, (c) document accepted risk and ignore via `cargo audit --ignore`.
- A toolchain/cache failure reproduces on re-run. This is infra, not a branch fix — surface to the user.
- A fix recovers most but not all of the original failures (partial coverage). Options: (a) commit the partial fix and open a follow-up ticket for the residual, (b) commit the partial fix and let the user file separately, (c) hold the fix and continue investigating before committing anything.
- Deeper diagnosis materially changes the nature or scope of the remaining work versus what the user authorized in a prior 1-3-1 (e.g. failures you framed as test-harness seeding turn out to need production-code changes or open-ended divergence isolation). Re-surface before proceeding — do not silently expand into work the user approved under a different understanding. Options: (a) ship the parts already validated and ticket the rest, (b) keep going on the deeper work now, (c) hold everything and reopen the originating spec/ticket.
- Three or more targeted fix variants have been tried for one identified root cause and none satisfy all original failure classes. The fix-space is constrained; further trial-and-error rarely converges. Options: (a) revert the introducing change and file a follow-up ticket for the missing capability, (b) widen scope to a structural refactor of the offending mechanism, (c) accept partial coverage and document the residual.

When stopping for user direction, use the `AskUserQuestion` tool with one question whose options correspond to the 1-3-1 alternatives — three by default, up to four where the decision space genuinely warrants it (the tool supports four). A layered decision may also need a brief follow-up confirming question for a *sub-decision* (e.g., how to handle the interim while a deferral is chosen) — that is fine, and does not re-open the full 1-3-1. Tag the recommended option with `(Recommended)` in its label so the user can pick it without re-reading the rationale. When the options are competing fix *hypotheses* whose efficacy is unverified — common for golden/planner regressions, where trajectory divergence defeats per-tick reasoning — do not tag a hypothesis `(Recommended)` on intuition alone: either run a quick spike to validate the candidate before presenting it, or label it "unvalidated hypothesis" instead of `(Recommended)`. Recommending an unvalidated direction can lead the user to authorize an implementation that turns out inert, costing a full implement-and-revert cycle plus another round-trip.

**Deferral outcome.** Only present deferral as the recommended option once the unmasked defect is localized to a concrete file and mechanism (per Step 5's checkpoint) and that localization shows the fix is genuinely out of this cycle's scope — do not default to deferral while the root cause is still characterized only as "multi-subsystem" or "unvalidated." When the spike narrows the surface to a small set of single-subsystem candidates (typically 2-3) but doesn't pick between them, run one more localization step — a per-tick decision trace, an action-start trace, or a focused unit test that toggles one candidate at a time — to converge to a single mechanism before tagging deferral as `(Recommended)`. Presenting deferral as *one option* in a 1-3-1 alongside "localize further now" remains acceptable; the rule constrains which option is tagged `(Recommended)`, not whether deferral is offered at all. If the decided 1-3-1 outcome is to *not* fix in this cycle (failure understood but deliberately deferred), skip the fix-push-verify spine (Steps 6-8 and 11-12's monitoring). Produce the follow-up ticket per Step 12's content requirements (cite the unmasking commit, the experiments already ruled out, and the suspected investigation surface), gate it via Step 10 approval, and commit/push the ticket alone — or leave it uncommitted if the user prefers. Before accepting the deferral, determine whether the failing commit is already on `main` (e.g., `git rev-list --count main..HEAD` or `git branch --contains <sha>`); if it is, deferral leaves `main` itself red, not merely a PR check — a materially worse hygiene state. CI stays red by design; say so explicitly — and when the commit is on `main`, state in the report that `main` (not just a PR) is the red surface.

### 6. Implement the fix

1. Keep edits minimal and targeted. Do not bundle unrelated cleanup.
2. Respect FOUNDATIONS-alignment per Step 7 below.
3. Forbidden shortcuts:
   - No backward-compatibility shims, deprecated wrappers, or dual code paths (CLAUDE.md).
   - No `#[allow(<lint>)]` to silence clippy. Fix the lint.
   - No `--no-verify`, `--no-gpg-sign`, or any hook bypass.
   - No `HashMap` / `HashSet` introduced in authoritative state (determinism rule).
   - No floats or wall-clock time introduced in simulation code.
4. For diagnosed bugs not yet covered by a failing test, add the failing test first, confirm it fails, then fix the code.
5. If the fix touches scenario `.ron` files or planner behaviour, derived artifacts may drift. Check for drift first: run `cargo run -p worldwake-cli --bin scenario-coverage -- --check` and the `scenario_diagnostics_fixture` stability golden (`cargo test --release -p worldwake-ai --test golden_ai scenario_diagnostics_fixture -- --ignored --test-threads=1`). If either reports drift, regenerate via `cargo run -p worldwake-cli --bin scenario-coverage -- --write` (for `docs/generated/scenario-coverage.md`) and `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test --release -p worldwake-ai --test golden_ai scenario_diagnostics_fixture -- --ignored --test-threads=1` (for `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json`), then stage and commit alongside the source change.

### 7. FOUNDATIONS-alignment contract

The fix must respect:

| Principle | Application |
|-----------|-------------|
| FND-14 (belief-only planning) | A golden-test fix in the AI pipeline must preserve belief-only contract; never plan from world state |
| FND-26 (system decoupling) | A fix in `worldwake-systems/<system>` must not import from a sibling system crate |
| No backward-compatibility layers (CLAUDE.md) | Deprecated wrappers, dual code paths, or migration shims are never the right fix |
| Determinism (CLAUDE.md) | Never introduce `HashMap`/`HashSet` in authoritative state, floats, or wall-clock time as a fix shortcut |
| Conservation (CLAUDE.md) | Item-handling fixes must not bypass `verify_conservation` |
| TDD bugfixing (CLAUDE.md) | For diagnosed bugs, add a failing test that captures the bug, confirm it fails, then fix |
| FND-1 truth-adjustment | A test assertion update or fixture regeneration is a legitimate fix when the underlying production behavior change is intentional and verified by a separate golden — never to mask a bug. The commit message must cite the architectural change that motivated the new value, plus the golden(s) that prove the underlying behavior is correct |

If a fix would violate any of these, stop and apply 1-3-1.

### 8. Verify locally

Run the narrowest correct verification first, then broaden.

1. The class-specific repro command from Step 4 — confirm it now passes.
2. Crate-level tests for the affected crate (e.g., `cargo test -p worldwake-ai`).
3. `./scripts/verify.sh` — the canonical pre-PR gate. It matches the CI workflow gates *except the gated golden-matrix workflows* (`golden-*.yml`), which run `#[ignore]` tests verify.sh skips; re-run the relevant gated family separately per the golden-matrix taxonomy row.

When the fix touches engine or framework code (tick pipeline, scheduler, action validation, planner, component registration), re-run the *entire* gated golden family for the failing workflow — e.g., all `golden-survival.yml` scenarios via `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1` — not just the single failing scenario. Framework changes affect sibling scenarios that the single-scenario repro and verify.sh cannot catch (CLAUDE.md's Authoritative-to-AI Impact Rule: ALL golden tests must pass).

**Revert-to-known-green shortcut.** When the fix is a file-level revert to a specific ancestor commit (e.g., `git checkout <sha> -- <files>`) and that ancestor has documented green CI history on the affected workflow family (verified via `gh run list --branch main --workflow=golden-<family>.yml --limit 10`), substitute the full-matrix local run with: (a) verify.sh, (b) the originally-failing scenario's module re-run, and (c) the ancestor's green-run citation in the approval summary. This only applies to pure file-level reverts — not to partial reverts that combine the ancestor's code path with new changes.

When the fix mutates authoritative or belief state (e.g. seeding a belief inside an action handler, changing what a commit writes), explicitly run the affected scenario's determinism/replay golden (e.g. `<scenario>_replays_deterministically`) in addition to the failing assertion. A state-mutating fix can satisfy the failing assertion yet break deterministic replay, and verify.sh's `cargo test --workspace` does not cover `#[ignore]`d replay goldens.

When the fix touches only test code, fixture data, or other files outside `crates/worldwake-{core,sim,systems,ai,cli}/src/`, the verification scope shrinks to (a) the originally-failing tests and (b) sibling tests in the same `--test` target or fixture module. Production-code goldens (e.g., baseline, scattered, preferences, trade) need not be re-run unless their own scenario `.ron` or fixture file also changed in the commit. State explicitly in the approval summary that "no production code changed → goldens X/Y/Z not re-run" so the reviewer can confirm the scope shrink.

If a broader run fails outside the original failure class, classify the new failure and loop back to Step 3 within the same skill invocation. Do not push partial fixes.

### 9. Multi-failure handling

When the run produced multiple failures:

- **Group by class.** Fix all rustfmt issues, then all clippy issues, then all test failures, etc.
- **Each class = one commit.** Cleaner history; bisectable. Stage files explicitly per commit (`git add <path>`, never `git add -A` or `git add .`).
- **Cascading failures get one commit.** If a single root-cause fix resolves several failing tests, that's one commit.
- **Independent classes get separate commits**, even when pushed together.
- **Multiple independent root causes within one class:** prefer one commit per root cause if the diffs are cleanly separable; combine into one commit only when the root causes share architectural origin (e.g., both lost in the same migration). Bisectability and revertability are the deciding criteria.
- **Single push at the end** of the multi-commit sequence. The diff-approval gate in Step 10 covers all commits at once.

### 10. Present the diff and request approval

Before any `git add`, `git commit`, or `git push`:

1. Run `git status` to confirm what will be committed.
2. Run `git log @{u}..HEAD --oneline` to enumerate any unpushed local commits. The push at Step 11 will publish every commit in that list, not only the fix commit you are about to create. If any pre-date this session or are otherwise unrelated to the fix, list them in the approval summary so the user can confirm what `git push` will publish alongside the fix.
3. Run `git diff` (or `git diff --stat` plus targeted `git diff <file>` for non-trivial hunks) to summarize changes.
4. Present to the user:
   - The failure(s) addressed (class + signature).
   - Per-file change rationale.
   - FOUNDATIONS principle citations for any architectural fix.
   - The local verification result (which commands ran, which passed).
   - The proposed commit message(s) for multi-commit sequences.
   - Any unrelated unpushed commits from item 2 that the push will publish.
5. Wait for explicit user approval. Do not commit or push without it.

**If the changes were already committed or pushed externally** (e.g., the user committed during a long verification window): do not silently re-do or amend a *pushed* commit. Run `git status` and `git log` / `git branch -r --contains <sha>` to confirm whether the commit reached origin, verify the committed diff matches your intended fix, and surface any commit-message-convention deviation (imperative style + `Co-Authored-By` trailer) for the user to decide on — amending a pushed feature-branch commit requires a force-push with explicit authorization. Then skip to Step 12 monitoring.

### 11. Commit and push

On approval:

1. Stage files explicitly by name. Never use `git add -A` or `git add .`.
2. Commit with an imperative message matching recent repo style:
   - Patterns in this repo: `Fix clippy lints in worldwake-ai`, `Fix golden_survival_combat regression`, `fix(ci): ...`.
   - Include the `Co-Authored-By:` trailer per CLAUDE.md's Commit Conventions, using the current session model (do not hardcode a stale version) — e.g.:
     ```
     Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
     ```
   - Use a HEREDOC to pass the message to ensure correct formatting.
3. For multi-commit sequences, create each commit separately, in class order.
4. Push without `--force`. Never force-push to `main` or `master`. For feature branches, force-push only with explicit user authorization in the same turn.

**Code fix + follow-up ticket: one or two commits.** When the fix produces both a code change AND a follow-up ticket, either grouping is acceptable. Two common shapes:

- **Feature-revert paired with a re-enable-conditions ticket (Sub-path A pattern):** single commit when the ticket's primary purpose is to document the specific deferral created by the code change (the commit message body should name the ticket file), or two commits when the ticket also stands on its own as future work.
- **Sub-path B calibration fix paired with a Sub-path C ticket for a sibling collateral regression from the same upstream:** prefer two commits — they're cleanly separable, the code change is revertable, and the ticket lands independently if review redirects the code fix. Single commit is acceptable when the commit message body names the ticket file and explains that both addresses are downstream of the same upstream change.

Splitting is the safer default in both shapes — it keeps the code change cleanly revertable and lets the ticket land independently if review redirects the code fix. Step 12's "Red ✗" branch repeats this for residual-failure tickets specifically; the same rule applies to any deferred-work ticket created during a successful fix cycle.

### 12. Wait for the CI re-run

After push:

1. Locate the new run for the latest commit:
   - `gh run list --branch <branch> --limit 1` to identify the run ID, or
   - `gh run watch <run-id>` to stream status until completion.
   - If `gh run watch` exits or stalls before the run reaches a terminal state (e.g., upstream API 5xx, exit-without-conclusion), fall back to polling: `until gh run view <run-id> --json status --jq '.status' | grep -q completed; do sleep 30; done`, then re-check the conclusion via `gh run view <run-id> --json conclusion --jq '.conclusion'`.
   - **Polling multiple workflows on one commit.** Matrix CI commonly triggers many concurrent workflow runs per push. To wait for *all* runs of a commit to complete, adapt the polling to a `--commit <sha>` filter — but use the **full** 40-character SHA from `git rev-parse HEAD`, not a short prefix (`gh`'s `--commit` filter does not match prefixes reliably and silently returns an empty result on mismatch). The until-loop must also gate on "at least one run matches" — a bare `[ unfinished -eq 0 ]` check is trivially satisfied before runs register, exiting the loop with zero work done. Use a compound exit: `until [ $(gh run list --branch <branch> --limit 15 --json status,headSha --jq '[.[] | select(.headSha == "<FULL_SHA>" and .status != "completed")] | length') -eq 0 ] && [ $(gh run list --branch <branch> --limit 15 --json headSha --jq '[.[] | select(.headSha == "<FULL_SHA>")] | length') -gt 0 ]; do sleep 60; done`.
   - For CI runs that may exceed the prompt-cache TTL (Golden Survival typically takes 15+ minutes): if you started the poll as a harness-tracked background task (`run_in_background`), its completion notification is the primary signal — you do not need a `ScheduleWakeup`, since polling harness-tracked work with a wakeup is wasted. Schedule a fallback wakeup (1200-1800s) only when (a) the background poll is `gh run watch` (known to drop on API 5xx and exit without conclusion), or (b) the task's output file is still empty 60+ seconds after the expected completion time. Do not schedule fallback wakeups for harness-tracked `Bash` background tasks executing locally-deterministic commands like `cargo test`, `verify.sh`, or a polling `until`-loop — these reliably fire their completion notifications.
   - Exit codes from background `cargo test` and `gh run view` invocations can be misleading — a backgrounded `cargo test` whose tests panic has been observed to surface `exit code 0` in the harness notification even though `test result: FAILED` is in the captured output. Always grep the captured output for `test result: FAILED|panicked|conclusion.*failure` before trusting the notification's exit code.
2. Report the final status:
   - **Green ✓**: report success and stop.
   - **Red ✗**: summarize the new failures (which jobs, which signatures). Do not auto-loop into another fix-push cycle. If the residual failures need separate investigation, optionally create a follow-up ticket under `tickets/` using `tickets/_TEMPLATE.md`. Cite the bisect-identified introducing commit (if known), the experiments already ruled out, and the suspected investigation surface. Commit the ticket as a separate commit in the same push (combining the ticket with the fix is acceptable when the ticket's primary purpose is to document the residual failure created by the fix itself — e.g., a revert + follow-up-investigation ticket; in that case the commit message body must name the follow-up ticket file).

**Feature-reactivation follow-up ticket shape.** When the fix path disables a shipped feature pending separate work (Step 5 point 8 Sub-path A), the follow-up ticket has a stereotyped shape distinct from the generic `_TEMPLATE.md`. Include:

1. **Failure Modes to Resolve Before Re-enable** — enumerate each specific failure the producer activation surfaced (e.g., "witness unavailable at suspension time → trap"; "multi-agent scenarios where the consumer chain still fails downstream"; "no safety net for stuck suspensions"). One numbered item per failure mode, each citing the originally-regressed golden(s) that reproduced it.
2. **Architecture Check** — name the symmetry/safety-net conditions the re-enabled feature must satisfy (e.g., producer must check the same precondition the consumer does; entries must have a safety-net kill condition; the end-to-end chain must be proven by a golden).
3. **Verification Layers** — list the focused unit tests, end-to-end goldens, and re-runs of the originally-regressed goldens that must all be green before the producer is re-enabled. Tie each to a specific failure mode where applicable.
4. **What to Change** — name the disabled symbol explicitly (file + function), its current stubbed return value, and the call sites that currently no-op as a result. The next implementer must be able to find the disable site without re-reading this skill's session history.

This shape is more specific than `_TEMPLATE.md`'s generic Verification/What-to-Change sections because the deferred work is *gated* — there's a measurable bar (all originally-regressed goldens green AND new coverage for the deferred failure modes) before the feature can ship again.

If the re-run is still red, the next iteration is a fresh skill invocation by the user. The skill's contract is one diagnose-fix-push cycle plus one re-run confirmation.

## Guardrails

- Never adapt tests to fix bugs — fix the code (CLAUDE.md TDD bugfixing).
- Never use `--no-verify`, `--no-gpg-sign`, or any hook bypass.
- Never `#[allow(...)]` a clippy lint to silence CI.
- Never add backward-compat shims, deprecated wrappers, or dual paths.
- Never `git push --force` to `main` or `master`. Force-push to feature branches only with explicit user authorization.
- Stage files explicitly by name; never `git add -A` or `git add .`.
- Worktree discipline: if the branch lives under `.claude/worktrees/<name>/`, all file operations use the worktree root as the base path.
- Reproduce every failure locally before fixing. Hypothesis-driven fixes without local repro are not acceptable.
- Apply 1-3-1 (1 problem, 3 options, 1 recommendation) when reproduction fails, when multiple root causes are plausible, when a fix would violate FOUNDATIONS, when an audit advisory has no fix, when a toolchain failure reproduces on re-run, when a fix achieves only partial coverage of the original failures, when deeper diagnosis materially changes the scope versus what the user authorized in a prior 1-3-1, or when 3+ targeted fix variants for one root cause have failed to satisfy all original failure classes.
- After the post-push CI re-run, do not auto-loop into another fix-push cycle. Report and stop.

## Example Usage

```
/fix-ci-failures
/fix-ci-failures 42
/fix-ci-failures 18234567890
```
