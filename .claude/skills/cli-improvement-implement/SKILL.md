---
name: cli-improvement-implement
description: "Read latest CLI evaluation, implement top recommendations within crates/worldwake-cli/. Invoke after evaluate to fix highest-priority issues."
user-invocable: true
---

# CLI Implementation

Improve the CLI based on the latest evaluation's scores and recommendations.

## Invocation

```
/cli-improvement:implement
```

No arguments. Reads the latest evaluation from `reports/cli-evaluation.md`.

## Process

Follow these steps in order. Do not skip any step.

### Step 1: Read Latest Evaluation

Read `reports/cli-evaluation.md` — focus on the latest EVALUATION #N:

1. If the file is under 300 lines, read it in full. Otherwise, count total lines and read from `offset = totalLines - 200` to get the latest evaluation
2. Identify CRITICAL and HIGH recommendations
3. If no CRITICAL or HIGH exist, target the top 2-3 MEDIUM recommendations
4. Note which metrics scored lowest — these are the priority targets

### Step 2: Read Relevant Source Files

The CLI crate lives at `crates/worldwake-cli/src/`. Key files by area:

| Area | Files |
|------|-------|
| Action listing/execution | `handlers/actions.rs` |
| State display | `handlers/inspect.rs`, `handlers/tick.rs`, `handlers/world_overview.rs` |
| Event/trace display | `handlers/events.rs` |
| Character control | `handlers/control.rs` |
| Entity resolution | `display.rs` |
| REPL loop | `repl.rs` |
| Command definitions | `commands.rs` |
| Command dispatch | `handlers/mod.rs` |
| Save/load | `handlers/persistence.rs` |
| Scenario loading | `scenario/mod.rs`, `scenario/types.rs` |
| Integration tests | `tests/integration.rs` |

Read the files relevant to the top recommendations.

### Step 3: Classify Each Fix

For each recommendation, identify the *primary* classification and note any secondary files affected. This guides where to start, not where to stop — real fixes are often cross-cutting.

- **Display fix** (wrong presentation): Output is correct but shown poorly. Start in handlers or `display.rs`. Examples: debug format, cramped layout, missing labels. May require reading component type definitions in `worldwake-core/src/` to understand field names and types (read-only exploration, not a scope violation).
- **Validation fix** (wrong behavior): Actions are listed that shouldn't be, or errors lack context. Start in `handlers/actions.rs` or the relevant handler. Examples: missing profile check, unhelpful error message.
- **Flow fix** (wrong interaction): The command sequence is confusing or implicit. Start in `repl.rs` or `commands.rs`. Often also requires updating `handlers/mod.rs` (dispatch) and `tests/integration.rs`.
- **Upstream flag** (needs non-CLI changes): The fix requires changes to core/sim/systems/ai crates. Do NOT implement — flag it as a separate spec/ticket.
- **False positive** (not a CLI issue): The evaluation flagged something that works correctly, or the issue is in the evaluator's test methodology. Note the finding in the summary so the next evaluation can verify with a better test case.

If addressing 4+ recommendations or changes that cross-cut multiple handler files, consider using plan mode to align on approach before implementing.

### Step 4: Implement Changes

For the top recommendations (all CRITICAL and HIGH, plus MEDIUM recommendations that share the same files as the top fixes). Batch related changes to reduce churn, but don't expand scope beyond what can be verified in one pass.

1. Identify the specific file and function before writing code
2. If the fix approach is ambiguous, apply the 1-3-1 rule (1 problem, 3 options, 1 recommendation) and ask the user
3. Implement changes, highest-impact first
4. If command definitions change (`commands.rs`), also update `handlers/mod.rs` (dispatch) and tests in both `commands.rs` and `tests/integration.rs` to match the new types
5. After each fix, verify it compiles: `cargo build -p worldwake-cli`

### Step 5: Verify

Run the full verification suite:

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
```

If tests fail, fix the issues before proceeding.

For display fixes, also run 2-3 smoke-test commands against the evaluation scenario to verify the output looks correct:

```bash
cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec "<relevant command>" --state /tmp/cli-impl-verify.bin
```

Clean up the state file after: `rm /tmp/cli-impl-verify.bin`

### Step 6: Summary

Present what was changed:
- Which recommendations were addressed
- Which files were modified
- Any recommendations that were flagged as upstream (needing non-CLI changes)
- Suggest running `/cli-improvement:evaluate` to measure the impact. Ensure changes are on the branch that the evaluate skill will build from.

## Scope Constraints

- **CLI-only**: Only modify files in `crates/worldwake-cli/`. If a fix requires changes to `worldwake-core`, `worldwake-sim`, `worldwake-systems`, or `worldwake-ai`, flag it as a separate spec/ticket.
- **No backward-compatibility wrappers**: Replace, don't wrap. Per Principle 28.
- **No report updates**: Do NOT modify `reports/cli-evaluation.md`. The next evaluate cycle measures impact.
- **No scenario changes**: Do NOT modify `scenarios/cli-evaluation.ron`. That's the scenario skill's job.
- **Follow existing patterns**: Match the CLI crate's existing code style, error handling patterns, and display conventions.

## Key Files Reference

### Display Formatting Patterns

The CLI uses these display helpers in `display.rs`:
- `entity_display_name()` — name resolution for entities
- `resolve_entity()` — user input to EntityId conversion
- `format_needs_bar()` — progress bar for homeostatic needs
- `format_quantity()` — item count display
- `format_location()` — location description (includes in-transit status)
- `format_control_source()` — control label
- `format_state_delta()` — human-readable event delta formatting

### Handler Patterns

Handlers follow: `fn handle_X(sim, ...) -> CommandResult` where `CommandResult = Result<CommandOutcome, CommandError>`. Some also take `repl_state` (actions.rs) or `registries` (actions.rs, tick.rs, world_overview.rs).

### Command Definition Patterns

Commands use Clap derive macros in `commands.rs`. For multi-word arguments, use `#[arg(trailing_var_arg = true)]`. For required multi-word args, add `num_args = 1..`. The dispatch function in `handlers/mod.rs` joins `Vec<String>` args with spaces before passing to handlers.

### Error Patterns

Errors use `CommandError::new("message")` — a simple string wrapper. When improving error messages, keep them concise and actionable: what went wrong + what to do instead.
