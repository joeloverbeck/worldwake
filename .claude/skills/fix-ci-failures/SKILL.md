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

   `--log-failed` returns the entire log of failing *steps*, which includes runner setup (toolchain install, cache restore, fetch) and may exceed 200KB for matrix workflows. Always pipe through `grep -E "FAILED|panicked|error\[|assertion|Diff in"` to extract signatures rather than reading whole output.
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
| **golden matrix scenario** | Job name `golden-<family> / <scenario>` failing; assertion line in log | `cargo test --release -p worldwake-ai --test golden_ai <scenario> -- --ignored --test-threads=1` (see note below) | Consult [docs/debugging-traces.md](../../../docs/debugging-traces.md). Treat goldens as authoritative; fix the production code |
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

### 5. Diagnose and decide

Once the failure reproduces:

1. Read the failed test, lint, or assertion in the codebase. Trace to root cause.
2. For golden test failures: consult [docs/debugging-traces.md](../../../docs/debugging-traces.md). Goldens are authoritative — the bug is in production code unless evidence proves the golden's contract is itself stale (in which case CLAUDE.md's `Authoritative-To-AI Impact Rule` applies). A third possibility exists and is easy to miss: a FOUNDATIONS-alignment change (e.g., closing a belief-leak so `effective_place`/`can_control` no longer read remote world state) can be **correct** and still break a golden by **unmasking a separate pre-existing defect** that the now-removed behavior was silently compensating for. Signals: the changed production behavior is provably correct (covered by its own passing unit tests) and the failing agent is otherwise healthy (e.g., a near-identical whole-run action-count profile). When this pattern holds, the resolution is neither reverting the change nor adapting the golden — it is isolating the unmasked defect (often in a different subsystem) and, if it is out of this fix's scope, filing a follow-up ticket per Step 12.
3. For any fix touching engine architecture, planner pipelines, action validation, or component registration: re-read the relevant sections of [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md).
4. When the failure is a regression and the branch has multiple commits since `main` (or since the last green CI run), consider bisecting via `git worktree add /tmp/<name>-bisect <commit-sha>` to isolate the introducing commit before deep code-reading. Run the narrowest local repro at each candidate commit. Worktrees are cheaper than `git checkout` and keep your main working tree clean. Clean up with `git worktree remove --force /tmp/<name>-bisect` for each worktree before committing — leftover worktrees pollute `git worktree list` and consume disk space, especially on space-constrained WSL2/VM disks. If the chosen remediation is a revert and `git revert <commit> --no-commit` conflicts on non-code files (e.g., an archived ticket that subsequent commits further modified), abort with `git revert --abort` and apply a targeted file-level revert via `git checkout <commit>^ -- <code-file1> <code-file2>` to restore only the source files. Verify scope with `git diff --stat HEAD` before staging. When the regression is in a golden/simulation, isolate the cause at the level of the changed code path — restore or revert the suspect function and confirm the whole run flips pass↔fail — **not** by comparing per-tick state between two versions. The simulation is deterministic but chaotic: two code versions diverge into different trajectories, so an agent's belief or world state at a fixed tick differs because its entire history diverged, making fixed-tick A/B state comparison misleading. Compare whole-run outcomes or action-count profiles instead.
5. If the failure reproduces locally but a prior CI run on an ancestor commit reported success for the same test, check 2-3 ancestor commits' CI runs (`gh run list --branch <branch> --workflow=<name>.yml --limit 10`) for that test before assuming the most recent commit introduced it. A single green CI run in the history may have been a flake; the actual regression may live further back. Bisecting from a "last green CI run" that was itself a flake will land on the wrong commit.
6. TDD discipline (per CLAUDE.md): never adapt tests to fix bugs. For real bugs, the failing test is already the regression proof.
7. When the fix changes AI, planner, or observer behavior — especially reverting a prior change — run `git log --oneline -- <touched-files>` and grep `archive/tickets/` plus `crates/worldwake-*/tests/fixtures/` for the names of OTHER tests or fixtures previously calibrated to the behavior being changed. Calibrations downstream of the change (e.g., a follow-up ticket that adjusted an observer-anomaly count or regenerated a diagnostics fixture under the now-being-reverted behavior) will likely fail on the next push and should be addressed in the same commit batch. Skipping this scoping move drives the whack-a-mole pattern across consecutive cycles. The same scoping move applies at the CI layer: when the change renames, consolidates, or removes test targets, binaries, benches, or examples, grep `.github/workflows/` for references to the old names — every per-family or matrix workflow naming a removed target will fail on the next push and must be fixed in the same commit batch.

Apply 1-3-1 (1 problem, 3 options, 1 recommendation) and stop for user direction when:

- A failure cannot be reproduced locally despite a clean checkout and matching toolchain. Options: (a) bisect against `main` to isolate the introducing commit, (b) re-run the CI job in case it's a flake, (c) write a follow-up ticket and defer the fix.
- Two or more plausible root causes exist for one failure.
- The minimal fix would require violating a FOUNDATIONS principle. Never proceed silently — surface the conflict.
- A `cargo audit` advisory has no fix version. Options: (a) pin to a non-vulnerable version range, (b) replace the dep, (c) document accepted risk and ignore via `cargo audit --ignore`.
- A toolchain/cache failure reproduces on re-run. This is infra, not a branch fix — surface to the user.
- A fix recovers most but not all of the original failures (partial coverage). Options: (a) commit the partial fix and open a follow-up ticket for the residual, (b) commit the partial fix and let the user file separately, (c) hold the fix and continue investigating before committing anything.
- Three or more targeted fix variants have been tried for one identified root cause and none satisfy all original failure classes. The fix-space is constrained; further trial-and-error rarely converges. Options: (a) revert the introducing change and file a follow-up ticket for the missing capability, (b) widen scope to a structural refactor of the offending mechanism, (c) accept partial coverage and document the residual.

When stopping for user direction, use the `AskUserQuestion` tool with one question whose options correspond to the three 1-3-1 alternatives. Tag the recommended option with `(Recommended)` in its label so the user can pick it without re-reading the rationale. When the options are competing fix *hypotheses* whose efficacy is unverified — common for golden/planner regressions, where trajectory divergence defeats per-tick reasoning — do not tag a hypothesis `(Recommended)` on intuition alone: either run a quick spike to validate the candidate before presenting it, or label it "unvalidated hypothesis" instead of `(Recommended)`. Recommending an unvalidated direction can lead the user to authorize an implementation that turns out inert, costing a full implement-and-revert cycle plus another round-trip.

**Deferral outcome.** If the decided 1-3-1 outcome is to *not* fix in this cycle (failure understood but deliberately deferred), skip the fix-push-verify spine (Steps 6-8 and 11-12's monitoring). Produce the follow-up ticket per Step 12's content requirements (cite the unmasking commit, the experiments already ruled out, and the suspected investigation surface), gate it via Step 10 approval, and commit/push the ticket alone — or leave it uncommitted if the user prefers. CI stays red by design; say so explicitly.

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
5. If the fix touches scenario `.ron` files or planner behaviour, expect derived artifacts to drift. Regenerate with `cargo run -p worldwake-cli --bin scenario-coverage -- --write` (for `docs/generated/scenario-coverage.md`) and `WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test --release -p worldwake-ai --test golden_ai scenario_diagnostics_fixture -- --ignored --test-threads=1` (for `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json`). Stage and commit alongside the source change.

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
2. Run `git diff` (or `git diff --stat` plus targeted `git diff <file>` for non-trivial hunks) to summarize changes.
3. Present to the user:
   - The failure(s) addressed (class + signature).
   - Per-file change rationale.
   - FOUNDATIONS principle citations for any architectural fix.
   - The local verification result (which commands ran, which passed).
   - The proposed commit message(s) for multi-commit sequences.
4. Wait for explicit user approval. Do not commit or push without it.

**If the changes were already committed or pushed externally** (e.g., the user committed during a long verification window): do not silently re-do or amend a *pushed* commit. Run `git status` and `git log` / `git branch -r --contains <sha>` to confirm whether the commit reached origin, verify the committed diff matches your intended fix, and surface any commit-message-convention deviation (imperative style + `Co-Authored-By` trailer) for the user to decide on — amending a pushed feature-branch commit requires a force-push with explicit authorization. Then skip to Step 12 monitoring.

### 11. Commit and push

On approval:

1. Stage files explicitly by name. Never use `git add -A` or `git add .`.
2. Commit with an imperative message matching recent repo style:
   - Patterns in this repo: `Fix clippy lints in worldwake-ai`, `Fix golden_survival_combat regression`, `fix(ci): ...`.
   - Include the trailer:
     ```
     Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
     ```
   - Use a HEREDOC to pass the message to ensure correct formatting.
3. For multi-commit sequences, create each commit separately, in class order.
4. Push without `--force`. Never force-push to `main` or `master`. For feature branches, force-push only with explicit user authorization in the same turn.

### 12. Wait for the CI re-run

After push:

1. Locate the new run for the latest commit:
   - `gh run list --branch <branch> --limit 1` to identify the run ID, or
   - `gh run watch <run-id>` to stream status until completion.
   - If `gh run watch` exits or stalls before the run reaches a terminal state (e.g., upstream API 5xx, exit-without-conclusion), fall back to polling: `until gh run view <run-id> --json status --jq '.status' | grep -q completed; do sleep 30; done`, then re-check the conclusion via `gh run view <run-id> --json conclusion --jq '.conclusion'`.
   - For CI runs that may exceed the prompt-cache TTL (Golden Survival typically takes 15+ minutes): if you started the poll as a harness-tracked background task (`run_in_background`), its completion notification is the primary signal — you do not need a `ScheduleWakeup`, since polling harness-tracked work with a wakeup is wasted. Add a `ScheduleWakeup` fallback (1200-1800s) only for *untracked* external waits, or if the background watcher itself could stall silently without firing a notification.
   - Exit codes from background `cargo test` and `gh run view` invocations can be misleading — a backgrounded `cargo test` whose tests panic has been observed to surface `exit code 0` in the harness notification even though `test result: FAILED` is in the captured output. Always grep the captured output for `test result: FAILED|panicked|conclusion.*failure` before trusting the notification's exit code.
2. Report the final status:
   - **Green ✓**: report success and stop.
   - **Red ✗**: summarize the new failures (which jobs, which signatures). Do not auto-loop into another fix-push cycle. If the residual failures need separate investigation, optionally create a follow-up ticket under `tickets/` using `tickets/_TEMPLATE.md`. Cite the bisect-identified introducing commit (if known), the experiments already ruled out, and the suspected investigation surface. Commit the ticket as a separate commit in the same push (combining the ticket with the fix is acceptable when the ticket's primary purpose is to document the residual failure created by the fix itself — e.g., a revert + follow-up-investigation ticket; in that case the commit message body must name the follow-up ticket file).

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
- Apply 1-3-1 (1 problem, 3 options, 1 recommendation) when reproduction fails, when multiple root causes are plausible, when a fix would violate FOUNDATIONS, when an audit advisory has no fix, when a toolchain failure reproduces on re-run, when a fix achieves only partial coverage of the original failures, or when 3+ targeted fix variants for one root cause have failed to satisfy all original failure classes.
- After the post-push CI re-run, do not auto-loop into another fix-push cycle. Report and stop.

## Example Usage

```
/fix-ci-failures
/fix-ci-failures 42
/fix-ci-failures 18234567890
```
