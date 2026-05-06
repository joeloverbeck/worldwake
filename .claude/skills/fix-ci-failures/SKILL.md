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
3. For matrix workflows (`golden-*.yml`), record which scenarios failed by their job names (e.g., `golden-survival / combat`).
4. If a single workflow has multiple failed jobs, capture each independently — they may have unrelated root causes.

### 3. Classify each failure

Use the taxonomy below to assign every failure to a class. Each class prescribes the exact local reproduction command and the standard remediation pattern.

| Class | Telltale log signature | Local repro command | Standard remediation |
|-------|------------------------|---------------------|----------------------|
| **rustfmt** | `cargo fmt --all -- --check` step fails; diff hunks in log | `cargo fmt --all -- --check` | `cargo fmt --all` |
| **build/compile** | `error[E####]:` in log | `cargo build --workspace` | Fix import / stale literal / type mismatch surfaced by compiler |
| **unit/integration test** | `test result: FAILED. ... <name> ... FAILED` | `cargo test -p <crate> <test_name>` | Diagnose: real bug vs. test drift. Never adapt test to bug |
| **clippy** | `error: ... -D <lint>` from `cargo clippy --workspace --all-targets -- -D warnings` | `cargo clippy --workspace --all-targets -- -D warnings` | Fix the lint. Never `#[allow]` to silence |
| **golden matrix scenario** | Job name `golden-<family> / <scenario>` failing; assertion line in log | `cargo test -p worldwake-ai --test golden_<family>_<scenario>` | Consult [docs/debugging-traces.md](../../../docs/debugging-traces.md). Treat goldens as authoritative; fix the production code |
| **cargo audit** | `audit-check` job; `Crate: <name>, Vulnerability: ...` | `cargo audit` (after `cargo install cargo-audit` if missing) | Bump the dep in `Cargo.toml`. If no fix version exists, escalate per Step 5 |
| **toolchain/cache/checkout** | Failure in `Checkout`, `Install Rust toolchain`, or `Cache cargo artifacts` step | (no local equivalent) | Re-run via `gh run rerun <run-id>` once. If reproducible, escalate as infra issue, not as a code fix |

### 4. Reproduce locally

Hard requirement: every failure must reproduce locally before fixing.

1. Run the exact command prescribed by the class.
2. Confirm the failure signature matches what CI reported.
3. If the failure does not reproduce locally despite a clean checkout and matching toolchain (`rustup show` should match `1.93.0`), escalate per Step 5. Do not proceed with a hypothesis-driven fix.

`./scripts/verify.sh` matches CI exactly (it runs `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check` in order). Use it as the canonical local gate, but run the narrowest class-specific repro first to avoid rebuilds.

### 5. Diagnose and decide

Once the failure reproduces:

1. Read the failed test, lint, or assertion in the codebase. Trace to root cause.
2. For golden test failures: consult [docs/debugging-traces.md](../../../docs/debugging-traces.md). Goldens are authoritative — the bug is in production code unless evidence proves the golden's contract is itself stale (in which case CLAUDE.md's `Authoritative-To-AI Impact Rule` applies).
3. For any fix touching engine architecture, planner pipelines, action validation, or component registration: re-read the relevant sections of [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md).
4. When the failure is a regression and the branch has multiple commits since `main` (or since the last green CI run), consider bisecting via `git worktree add /tmp/<name>-bisect <commit-sha>` to isolate the introducing commit before deep code-reading. Run the narrowest local repro at each candidate commit. Worktrees are cheaper than `git checkout` and keep your main working tree clean.
5. TDD discipline (per CLAUDE.md): never adapt tests to fix bugs. For real bugs, the failing test is already the regression proof.

Apply 1-3-1 (1 problem, 3 options, 1 recommendation) and stop for user direction when:

- A failure cannot be reproduced locally despite a clean checkout and matching toolchain. Options: (a) bisect against `main` to isolate the introducing commit, (b) re-run the CI job in case it's a flake, (c) write a follow-up ticket and defer the fix.
- Two or more plausible root causes exist for one failure.
- The minimal fix would require violating a FOUNDATIONS principle. Never proceed silently — surface the conflict.
- A `cargo audit` advisory has no fix version. Options: (a) pin to a non-vulnerable version range, (b) replace the dep, (c) document accepted risk and ignore via `cargo audit --ignore`.
- A toolchain/cache failure reproduces on re-run. This is infra, not a branch fix — surface to the user.

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

If a fix would violate any of these, stop and apply 1-3-1.

### 8. Verify locally

Run the narrowest correct verification first, then broaden.

1. The class-specific repro command from Step 4 — confirm it now passes.
2. Crate-level tests for the affected crate (e.g., `cargo test -p worldwake-ai`).
3. `./scripts/verify.sh` — the canonical pre-PR gate; matches CI exactly.

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
2. Report the final status:
   - **Green ✓**: report success and stop.
   - **Red ✗**: summarize the new failures (which jobs, which signatures). Do not auto-loop into another fix-push cycle.

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
- Apply 1-3-1 (1 problem, 3 options, 1 recommendation) when reproduction fails, when multiple root causes are plausible, when a fix would violate FOUNDATIONS, when an audit advisory has no fix, or when a toolchain failure reproduces on re-run.
- After the post-push CI re-run, do not auto-loop into another fix-push cycle. Report and stop.

## Example Usage

```
/fix-ci-failures
/fix-ci-failures 42
/fix-ci-failures 18234567890
```
