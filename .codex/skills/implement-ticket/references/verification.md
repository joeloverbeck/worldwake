# Verification

How to verify implementation at the right boundary (Step 6).

## Typical verification order

1. Focused test covering the changed behavior
2. Crate-level tests for the affected crate
3. Broader workspace validation if the change crosses boundaries

## Verification mechanics

- When the change touches multiple proof surfaces in one crate, run each focused selector needed.
- If a canonical interface is realized through a forwarding layer, prove both the consumer-facing call and the forwarding path.
- Check that focused selectors actually match new/changed test names.
- Check that focused selectors isolate the owned proof surface closely enough for the ticket. Substring filters may lawfully run extra tests; when exactness matters, prefer the narrowest truthful selector rather than treating any nonzero match as precise proof.
- Prefer separate `cargo test` invocations per selector over combining exact test names in one command.
- For Rust tickets, format the owned files before final broad verification. Prefer formatting only the owned files; if a broader formatter is necessary in a dirty worktree, inspect formatter spillover immediately and restore unrelated files.
- Run multiple `cargo test` or `cargo clippy` commands sequentially when they share the same build profile -- lock contention makes parallel same-profile runs unreliable. Different profiles (e.g., `cargo test` vs `cargo clippy`) are logically safe to overlap, but the shared target directory can still serialize them; prefer parallel runs only when the extra waiting is acceptable.
- When a broad verification run dies by `SIGKILL` or another likely environment/resource kill after focused suites are green, rerun the named interrupted/failing suite in isolation before repeating the full broad run.
- When a broader verification command is intentionally waived after user direction, record the exact completed command set plus the waived command in the ticket `Outcome`.
- Remove temporary debug or trace scaffolding before final verification unless the ticket explicitly owns keeping that instrumentation. After cleanup, rerun the narrowest affected proof.
- When a newly added ignored traceability or golden reproducer exists only to expose the pre-fix contradiction, remove or rewrite it before closeout if the shipped fix changes the live trace shape and the reproducer is no longer a stable contract test.
- After changing code post-verification, rerun narrowest affected tests and any stale broader commands.
- When CI/clippy forces a signature reshape, sweep all call sites before the next verification pass.
- When CI/compile fallout follows a shared context-field change, sweep manual struct literals as well as direct function call sites.
- When a migration reshapes a common API surface, expect lint fallout as well as compile fallout. Satisfy trait expectations like `Default` instead of suppressing lints.
- When long-running verification commands are in flight, reuse those sessions rather than spawning duplicates.
- When new registered actions or systems cause broad failures, triage for catalog-order drift, completeness assertions, and registry-expansion fallout before assuming the feature's runtime logic is broken.
- If a focused failing proof exposes a real production contradiction in a ticket marked test-only, update the ticket sections that define scope before continuing.
- When a ticket fixes a repeated pattern across multiple call sites, run a post-implementation pattern sweep (e.g., grep for the unfixed pattern) to confirm no sites were missed.
- When workspace-wide verification fails on files outside the ticket's owned surface (e.g., untracked binaries, pre-existing lint failures), verify the failure is unrelated by running scoped to the ticket's owned crates. Record the pre-existing failure and the scoped-pass result in the ticket Outcome.
- When broader verification is blocked by a pre-existing unrelated dirty or untracked file, non-semantic lint/format cleanup needed to complete CI-style verification is acceptable, but still record that file as unrelated pre-existing fallout and do not imply the unrelated feature work was completed.
- When broader verification fails on a golden in the same domain or planner path as the ticket's owned behavior, do one contract-level triage pass before labeling it unrelated.

```bash
cargo test -p <affected-crate> <test_name>
cargo test -p <affected-crate>
cargo test -p worldwake-core <test_name>
cargo test -p worldwake-core
cargo test -p worldwake-ai
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Golden test verification

- If stronger behavior now reaches completion earlier than a test assumed, recalibrate test inputs instead of weakening the implementation.
- When broader verification surfaces a timing-sensitive golden whose contract still holds, recalibrate the fixture's timing budget or hold window.
- When a golden assumes agents observe co-located facts at tick 0, verify the setup explicitly seeds those beliefs or perception prerequisites.
- When a golden uses external action requests for scripted setup, set that actor's `ControlSource` to `Human` or `None`.
- When a golden proves durable learned-state aftermath, assert the semantic contract unless exact tick identity is the owned invariant.
- If focused implementation shows the corrected ticket still over-claims, narrow the ticket before final verification.
- When a valid architecture change makes a golden stale, update it to prove the new lawful contract.
- When `python3 scripts/golden_inventory.py --write --check-docs` is part of the broadened proof, expect generated fallout across multiple `docs/generated/golden-*` artifacts rather than only the scenario inventory/detail pair. Typical expected churn includes `golden-e2e-inventory.md`, `golden-scenario-index.md`, scenario detail pages, and `golden-coverage-matrix.md`. Review the full generated diff and keep it when it matches the landed scenario metadata and inventory semantics.
- When a golden transport/delivery/claim chain is about durable aftermath, avoid over-specifying intermediate substeps.
- When a golden still fails after lawful setup, reassess whether it exposed a missing lower-layer contract. Fix the production boundary first.
- If the architecture change invalidates the old invariant, rewrite the scenario and update its header/comments.
- When adding `// Scenario ...` metadata, keep `Setup`, `Proves`, and `Cross-system chain` entries in the generator-friendly live format. After regenerating docs, inspect for truncation or malformed wrapped fields.
- When adding or renumbering `// Scenario N:` blocks, treat identifiers as repo-global. Pre-scan nearby or highest live IDs and resolve collisions.
- After scenario metadata changes, refresh the generated golden inventory/docs as part of the verification surface.
- When a golden test must isolate one goal from a competing goal family that shares the same observable input, use belief-source manipulation: seed beliefs via `PerceptionSource::Report` instead of `DirectObservation`.
- When a golden scenario depends on motive arithmetic driven by metabolism rates or profile values, estimate the crossover tick from the rate differential. Start with conservative values. If the first run misses the milestone, adjust rates rather than expanding the tick budget.
- When a golden scenario depends on a specific target subtype from a shared target surface such as `EntityAtActorPlaceAnyOf`, verify the full live selection path: affordance enumeration, belief prerequisites, planner snapshot inclusion/filtering, any planner-side reordering, and authoritative validation.
- When a ticket mixes correctness validation with campaign or soak performance claims, bifurcate: if correctness/golden proof passes but performance still regresses, close the behavior-validation slice honestly and create a follow-up for the remaining optimization.

## Migration verification checklist

For migrations moving config/profile state from driver-global to per-entity components:
1. Remove the driver/global field and constructor arguments
2. Move test/golden setup onto authoritative component writes for relevant entities
3. Update runtime/save-load mirrors and serialization helpers
4. Add harness helpers for per-entity profile injection when repeated setup would sprawl
5. Rerun both tests and CI-matching clippy after the API reshape

## Resolving focused test IDs

Before running focused Rust tests, resolve the exact live test IDs first (for example via `cargo test -- --list`) so the first selector is already module-qualified or exact when needed, rather than a loose substring that may compile a target while executing zero tests. A focused command that builds successfully but runs zero intended tests does not count as proof and must be corrected before continuing.

Before running a ticket-named focused command, verify that the selector actually proves the owned surface. If a substring filter would compile a target while running zero tests, or would run a broader unrelated surface than the ticket claims, correct the command immediately and update the ticket's command list during reassessment/closeout.
For Rust unit-test modules, module-name selectors like `cargo test -p <crate> <module_name>` can still fan out across unrelated bins, integration targets, or zero-test harnesses. After a `-- --list` check, prefer the narrowest truthful exact or module-qualified selector the current test binary layout supports. In multi-target Rust crates, when the proof lives in the library unit-test binary, prefer `cargo test -p <crate> --lib <module_path> -- --exact` over trying the crate-root selector form first.

## Cargo lock contention

For Rust verification commands that share Cargo build/artifact directories (for example `cargo test -p <crate>`, focused `cargo test` selectors, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings`), prefer running them sequentially rather than in parallel. Parallelize cheap reads/searches and non-Cargo focused checks freely, but avoid turning verification into cargo lock contention.

## Omitted-field serde proof fixtures

For omitted-field serde proofs on complex structs, prefer a format-agnostic fixture when hand-written text would be brittle: serialize a full value, remove only the target field from the serialized text, then deserialize and assert the defaulted field value.

## Golden inventory refresh fallout

When the ticket adds, renames, or materially re-scopes a `golden_*.rs` file or scenario block, run the repository's golden inventory/doc refresh as part of broadened verification and treat the generated docs as expected fallout to review and keep aligned with the landed scenario metadata. That generated fallout can extend beyond the scenario-details page to global inventory totals, coverage tables, and index entries; record the wider generated surface honestly in the ticket when it lands. In a dirty worktree, generated inventory docs can also absorb pre-existing local scenario metadata outside the current ticket, so quickly inspect whether the widened generated diff is current-ticket fallout, pre-existing local state, or a mix, and phrase closeout accordingly. Also expect those generators to compile feature-gated or list-only test targets outside the normal workspace proof surface; if the generator is part of the ticket's owned verification contract, helper or fixture failures exposed only by that refresh still count as lawful current-ticket fallout and should be fixed before closeout.
When `python3 scripts/golden_inventory.py --write --check-docs` is part of the owned verification contract, also expect validation fallout outside the generated markdown files themselves: orphaned `// Scenario ...` blocks that no longer own any `#[test]` function, and stale manual ``golden_*`` references in checked docs such as `docs/golden-e2e-testing.md`, both count as current-ticket fallout and should be fixed before closeout.

## Repo-wide live-contract fallout

If reassessment revealed that additive substrate from an earlier ticket already landed, include repository-wide live-contract fallout in the broadened verification sweep, not just the ticket's newly edited file set. Typical fallout includes stale `ALL` lists, exhaustiveness fixtures, representative-goal inventories, explicit length assertions that still reflect the pre-addition shape, and adjacent registry/declaration surfaces such as feasibility or invalidation strategies, provenance-family mappings, and other dispatch-table contracts that must now treat the additive shape as live behavior rather than inert scaffolding.
For component-registration tickets specifically, expect broadened fallout to include hardcoded component or manifest inventories, `ComponentValue`/sample-builder coverage, and exact bootstrap delta assertions in addition to compile-time macro expansion sites.

For additive planner-root tickets, also sweep helpers keyed by shared planner transitions or op-family semantics rather than only declaration tables and enum matches. Typical fallout includes planner-only synthetic candidate builders, search helpers that expand candidates from shared `PlannerTransitionKind` behavior, and exhaustive `PlannerOpKind` matches in non-obvious support modules such as observation/runtime reconciliation, blocker classification, or related-place/related-entity helpers.

## Behavior-expanding and scenario-isolation fallout

For behavior-expanding tickets, expect broadened golden fallout to include stale scenario isolation, not just compile or enum-shape fallout. If an existing golden now reaches a newly lawful branch, tighten the scenario so it still proves its intended invariant using explicit local belief seeding, profile/perception overrides, or other lawful setup constraints rather than silently preserving the old behavior. If the scenario's motivating invariant is still proved at an earlier or more stable boundary than the stale end-to-end narrative, narrow the golden back to that stable contract instead of overfitting the test to preserve the old execution story.
When a golden or replay fixture is meant to put an item on the ground, release carried stock, or otherwise change local control semantics, re-check authoritative owner, possessor, container, and ground-location state together before treating a changed golden outcome as a production regression. Old passing behavior can be masking a faulty fixture when only one of those coupled relations was updated.

## Belief-retention and epistemic-scope fallout

For belief-retention, perception-horizon, and other epistemic-scope tickets, expect broadened fallout to include tests that assert disappearance timing, stale-belief pruning, or planner candidate loss under the old window. When those expectations change because the ticket lawfully preserves beliefs longer, restate the live epistemic contract in the test and keep the proof at the strongest honest boundary rather than treating every AI-side expectation shift as a production regression. Also recheck cross-agent transfer and relay assertions (`tell`, `ask`, witness/report propagation, replay or integration harnesses) for presentation-tick semantics: a lawful migration may refresh the receiver's presentation history while preserving provenance and content, so stale "original observation tick survives transfer" expectations should be updated as verification fallout rather than misdiagnosed as regressions.
For belief-retention, arrival-reinforcement, or similar tickets that mutate an existing belief record, prove during reassessment whether a first-visit or no-record case is lawful on the live path. If it is, decide explicitly whether the ticket must seed the belief before applying the mutation rather than assuming an existing record is always present.

## Planner/search fallback and retained-successor fixtures

When a new fallback contract becomes lawful, re-check nearby planner/search tests and traces that previously asserted failure, suppression, or exhaustion. The honest post-change contract may now be `Found(ProgressBarrier)` or another bounded fallback plan rather than `not found`, and those expectation shifts should be treated as intentional verification fallout, not as automatic regressions.
When a planner/search post-pass helper mutates successor state after construction, and the live retained-successor path lawfully filters or terminalizes the specific branch shape you need to inspect, it is acceptable to prove that helper with a directly constructed lawful successor from the same production builder instead of overfitting the fixture to keep that branch retained.

## Broadened verification loop and reruns

When broadened verification fails, treat each failure as current-ticket fallout and continue the fix-and-rerun loop until the broadened target passes or you hit a real 1-3-1 blocker. Do not stop after the first full-suite failure if the next step is a straightforward fallout fix within the ticket's live scope.
When a planner/candidate-generation fix changes self-care or other high-priority AI behavior, same-domain fallout may surface one layer later as active-action interrupt/replan churn rather than another candidate-emission bug; treat that execution-layer loop as current-ticket fallout when it is still part of restoring the owned AI contract.
After each fallout fix, rerun the same broadened verification target that exposed the failure before treating the branch as green. Do not rely on focused follow-up checks alone when the broader package or suite has not yet been rerun clean.
If a broadened verification command was started but its final exit status can no longer be recovered from the active tooling session, treat that proof as incomplete and rerun the same broadened command before closeout rather than assuming it passed from partial streamed output.
When an all-target compile-only pass or broadened verification reports the same underlying file through multiple targets (for example a shared golden harness compiled both as a test and via a bin test target), treat that as expected target duplication rather than separate ownership. Fix the shared source once, then rerun the same broad command.
If the final post-verification edit is isolated to a bin, integration test, or other target surfaced only by broadened lint/CI fallout, rerun the narrowest honest proof for that exact target (for example `cargo test -p <crate> --bin <name> --no-run`) before closeout so the last change is not left unproved.
If diagnosis required temporary tracing, debug prints, probe assertions, or similar instrumentation, remove them before broadened verification and before updating the ticket's final outcome/verification text.
