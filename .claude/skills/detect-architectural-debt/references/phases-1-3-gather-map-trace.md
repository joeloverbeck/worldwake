# Phases 1-3: Gather, Scenario Map, Trace

## Phase 1: GATHER — Build the Exercised Module Set

Starting from the test file(s), build a list of source modules that the tests exercise.

**Short-circuit for golden/integration tests**: If the test calls a top-level simulation step function (e.g., `step_once()`, `tick()`, or equivalent) in a loop, all source modules in the referenced crates are exercised. Skip per-symbol tracing and enumerate all `.rs` files in those crates' `src/` directories directly, excluding `lib.rs` barrel files and `mod.rs` files that only contain `mod` declarations.

**Otherwise, trace per-symbol**:

1. If the input is a directory, collect all `.rs` files in it (excluding `mod.rs` files that only contain `mod` declarations). If a single file, use that file.
2. Read the test file(s) and extract all `use` statements to identify which crates are referenced (e.g., `worldwake_core`, `worldwake_sim`, `worldwake_systems`, `worldwake_ai`).
3. Extract all type names, function names, struct names, and enum variant names actually used in the test code body (not just imported). Focus on symbols from the `worldwake_*` crates.
4. For each referenced crate, grep `crates/<crate-name>/src/` for the definitions of those symbols (`pub fn <name>`, `pub struct <name>`, `pub enum <name>`, `pub trait <name>`) to identify which source modules are exercised.
5. For each exercised source module, read its internal `use` and `mod` statements to add 1-2 levels of internal dependencies to the exercised set.
6. Produce a deduplicated list of all source modules exercised by the test suite.

**Important**: Rust crate `lib.rs` files re-export most public items. Do not count `lib.rs` as an exercised module — it is a barrel file. Trace through to the actual defining module.

**Git history analysis** (runs in parallel with symbol tracing): Run bounded git history: `git log --since="6 months ago" --name-only` on exercised files. Use recursive globs (e.g., `'crates/worldwake-*/src/**/*.rs'`). From the output, group files by commit. For each commit, enumerate all cross-crate file pairs that changed together. Count how many commits each pair co-appears in. Report the top 20 cross-crate pairs with 3+ co-changes, ordered by frequency. Also report the crate-to-crate coupling matrix (total co-changing commits per crate pair).

**Prior reports**: Read any `prior_reports` if provided. Also scan `reports/` for existing `architectural-debt-*` reports matching the same test context or related test contexts (same harness setup, same crate under test, same short-circuit scope). Two test files that both exercise the entire `worldwake-ai` crate via `step_once()` loops produce overlapping coverage — their reports are mutually relevant. If the exercised module set overlaps >80% with a prior report's scope, auto-activate `--differential` mode.

**Sub-agent delegation**: For large test suites (>20 direct `use` imports or barrel re-exports), delegate import tracing to 1-3 parallel Explore sub-agents. Also delegate git history analysis to a separate sub-agent if the file list exceeds 30 modules.

**Tool usage**: Read test files, Grep for `use worldwake_`, Grep for `pub (fn|struct|enum|trait)` definitions in crate source directories, Bash for `git log`.

## Phase 2: SCENARIO MAP — Cluster Tests into Behavioral Families

Treat tests as behavioral scenarios, not just import sources.

For each test or test family (a `mod tests` block, a `#[test]` function, or a golden test file), recover:

- **What behavior** is being exercised (e.g., "goal replanning after action failure")
- **Which setup path** it uses (e.g., scenario RON file, `TestHarness` builder, manual component registration)
- **Which assertions** define success/failure (e.g., `assert_eq!`, `assert!`, custom assertion helpers)
- **Which domain concepts** appear in names, helpers, and expected values (e.g., "goal", "belief", "action", "need", "trade")

For golden E2E tests specifically, also note:
- Which RON scenario file is loaded (if any)
- Which systems are exercised through the simulation loop
- What emergent behavior the test validates (cross-system interactions)

Then cluster tests into **scenario families** — named behavioral groups. Example shapes:

- "goal dispatch lifecycle"
- "belief propagation chain"
- "action validation pipeline"
- "need satisfaction cycle"
- "trade negotiation flow"
- "combat resolution sequence"
- "perception and observation"

Every later finding must be tied back to scenario families. A finding not grounded in test behavior is speculation.

**Soak/endurance tests**: When the test runs the simulation for many ticks and checks invariants, derive scenario families from the invariant categories and emergence assertions rather than from test function boundaries. Each per-tick invariant check (conservation, needs bounds, unique placement) and each emergence threshold check (death, trade, political events) becomes a scenario family.

**Resilience/chaos tests**: When the test injects disruptions (kills, deletions, workstation removal, teleportation) and validates invariants hold despite them, derive scenario families from the disruption categories and the invariant categories they stress. The disruption injection protocol itself is a scenario family if it exercises a distinct code path. Similarly, serialization roundtrip tests form their own scenario family around the serialization boundary.

**Determinism replay tests**: Tests that run the same scenario twice with the same seed and assert identical outcomes are determinism validators, not separate scenario families. Count them with their parent scenario but note determinism validation as a cross-cutting concern.

**Sub-agent delegation**: For large test directories (>30 test files), delegate scenario extraction to 2-3 parallel Explore sub-agents, each handling a subset. Merge and deduplicate scenario families.

## Phase 3: TRACE — Build Test-to-Code Traceability

Build test-to-code traceability using multiple strategies:

| Strategy | What it finds | Confidence |
|----------|--------------|------------|
| `use` statements | Direct dependencies | High |
| Static call graph (from `assert!`/`assert_eq!` back to production) | Functions actually exercised | High |
| Naming/lexical similarity (test helpers vs production functions) | Conceptual links | Medium |
| Temporal coupling from git history (files that co-change) | Hidden dependencies | Medium |

Each traceability link gets a confidence tag (high/medium/low) and a brief reason code.

The purpose of multi-strategy tracing is to catch hidden dependencies that `use` statements alone miss — trait dispatch, `SystemFn` registration, `register_action_handler` indirection, and temporal coupling are the most common sources of invisible links in this codebase.

**After short-circuit**: When Phase 1's short-circuit determined all modules are exercised, skip the `use` statement and call graph strategies. Focus on temporal coupling analysis and naming/lexical similarity for mapping modules to scenario families. The traceability table should focus on modules uniquely relevant to specific scenario families — modules that handle the test's distinctive code paths (e.g., save/load, disruption handling, invariant checking). A focused table of 10-15 key modules is more useful than an exhaustive listing of 200.
