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
- Prefer separate `cargo test` invocations per selector over combining exact test names in one command.
- Run multiple `cargo test` or `cargo clippy` commands sequentially when they share the same build profile -- lock contention makes parallel same-profile runs unreliable. Different profiles (e.g., `cargo test` vs `cargo clippy`) are logically safe to overlap, but the shared target directory can still serialize them; prefer parallel runs only when the extra waiting is acceptable.
- When a broad verification run dies by `SIGKILL` or another likely environment/resource kill after focused suites are green, rerun the named interrupted/failing suite in isolation before repeating the full broad run.
- When a broader verification command is intentionally waived after user direction, record the exact completed command set plus the waived command in the ticket `Outcome`.
- Remove temporary debug or trace scaffolding before final verification unless the ticket explicitly owns keeping that instrumentation. After cleanup, rerun the narrowest affected proof.
- After changing code post-verification, rerun narrowest affected tests and any stale broader commands.
- When CI/clippy forces a signature reshape, sweep all call sites before the next verification pass.
- When CI/compile fallout follows a shared context-field change, sweep manual struct literals as well as direct function call sites.
- When a migration reshapes a common API surface, expect lint fallout as well as compile fallout. Satisfy trait expectations like `Default` instead of suppressing lints.
- Prefer formatting only the owned files. If you must run a broader formatter in a dirty worktree, inspect formatter spillover immediately and restore unrelated files.
- When long-running verification commands are in flight, reuse those sessions rather than spawning duplicates.
- When new registered actions or systems cause broad failures, triage for catalog-order drift, completeness assertions, and registry-expansion fallout before assuming the feature's runtime logic is broken.
- If a focused failing proof exposes a real production contradiction in a ticket marked test-only, update the ticket sections that define scope before continuing.
- When a ticket fixes a repeated pattern across multiple call sites, run a post-implementation pattern sweep (e.g., grep for the unfixed pattern) to confirm no sites were missed.
- When workspace-wide verification fails on files outside the ticket's owned surface (e.g., untracked binaries, pre-existing lint failures), verify the failure is unrelated by running scoped to the ticket's owned crates. Record the pre-existing failure and the scoped-pass result in the ticket Outcome.
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
