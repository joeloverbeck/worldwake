---
name: implement-ticket
description: "Implement or reassess a Worldwake ticket. Use when asked to work from a ticket in `tickets/`, `archive/tickets/`, or a worktree ticket path: read the ticket, validate its assumptions against the live codebase and repo rules, correct mismatches before coding when needed, then implement and verify the requested deliverables."
---

# Worldwake Ticket Implementation

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), the target ticket, and any ticket-linked specs or docs before editing code.

Keep the workflow compact and deterministic. Reassess first, then implement. Do not treat a ticket as mechanically executable until its assumptions match the current codebase.

## Workflow

### 1. Load the ticket context

1. Read the target ticket file.
2. Read every directly relevant reference named by the ticket:
   - specs
   - docs
   - code symbols
   - test files
3. When the user supplies a glob or shorthand spec reference, confirm the exact live matching file path before reading, citing, or relying on it.
4. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all reads, writes, searches, moves, and archival actions.

### 2. Reassess assumptions before coding

1. Verify the ticket against the current codebase, not against stale architectural memory.
2. Check the `Deps` field. Confirm each listed dependency is actually present on the current branch, whether as active planning material or as an archived completed prerequisite.
3. Validate the ticket's concrete claims:
   - referenced files exist
   - referenced types, functions, modules, commands, and tests exist
   - when a ticket extracts or reuses private helper logic, confirm the exact current crate/file ownership of that helper before finalizing the implementation plan or `Files to Touch`
   - described architecture still matches the live code
   - stated coverage gaps are real and classified correctly
   - for golden-driven tickets, claimed missing scenarios are not already covered by the current `golden_*` suites or the generated golden inventory/docs
   - for golden-driven tickets, if a claimed divergence is already proved at lower layers but does not remain stably isolatable in the current golden harness without scenario-distorting scaffolding, correct the ticket to the strongest honest golden contract and record which lower-layer proof remains authoritative for the rejected slice
   - when shared types change, serialized fixtures, bundled scenarios, schema examples, and other non-Rust deserialization inputs still match the live struct shape
   - when an authoritative persisted component or profile changes serialized shape, inspect the live save/load version boundary and correct version gates such as `SAVE_FORMAT_VERSION` when the format contract changed
   - when a ticket's intended behavior depends on helper math, scaling, saturation, or threshold arithmetic, inspect the exact live helper implementation and correct stale numeric prose before coding
   - when save/runtime structs or other persisted shapes gain or lose fields, search for test-only mirror structs and manual `bincode`/seeded deserialize helpers, not just production fixtures
   - when a ticket depends on shared static data such as recipe definitions, schemas, or other registry-backed content, confirm the live service bundle, execution context, or callback boundary that would need to carry that data; if the current runtime boundary does not expose it, correct the ticket before coding to name the real substrate change
   - when a ticket widens a shared callback or execution signature, search dependent crates for both production call paths and test-only direct handler registrations or manual `on_commit` / `on_abort` invocations so stale harnesses do not survive the initial implementation pass
   - when affordance generation depends on self-authoritative profile reads, those profile prerequisites are present in both production code and representative AI/planner test harnesses
   - for component-registration tickets, hardcoded schema inventories, sample `ComponentValue` enumerations, and manifest-style tests that mirror the authoritative component set still match the live schema after the new entry lands
   - when a ticket extends a narrow trait or read surface, check for forwarding macros, blanket impls, paired runtime traits, or other generated surfaces that materialize that API indirectly; if they exist, distinguish the canonical consumer boundary from any implementation-detail mirror the live architecture requires
   - when a ticket widens a shared trait or read surface, explicitly choose the narrowest ownership/borrowing form that preserves the canonical consumer path while minimizing snapshot, harness, and test-double fallout; do not default to borrowed or owned shapes without checking the real cross-crate construction cost
4. Reassess against Worldwake's repo rules:
   - ticket fidelity from [AGENTS.md](../../../AGENTS.md)
   - foundational compliance from [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md)
   - ticket structure from [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md)
5. For mixed-layer, planner, golden, or authoritative-validation work, name the exact symbols and boundaries under audit instead of using broad summaries.

### 3. Handle mismatches explicitly

If the ticket and live code disagree, stop and surface the discrepancy before implementation.

For each mismatch, state:
- what the ticket says
- what the codebase currently has
- whether the ticket should be corrected, the implementation should adapt, or the issue is blocked

For low-risk factual ticket corrections, you may update the ticket immediately before coding instead of stopping for confirmation. This applies only when the correction is mechanical and directionally unambiguous, such as:
- exact live spec path resolution from a user-supplied glob or shorthand
- stale file/symbol/test references
- `Files to Touch`, `Verification Layers`, or command lists that need to match the current codebase after reassessment
- component-registration fallout that is factual from live macro expansion or schema inventory discovery

When you make one of these direct corrections, record it in a compact reassessment note using:
- ticket says
- live code has
- correction applied
- why safe

When a low-risk factual correction changes the real fallout surface, update every affected ticket section that restates that surface, not just one inventory list. This commonly includes `What to Change`, `Files to Touch`, `Verification Layers`, and `Test Plan`.

Apply the 1-3-1 rule from [AGENTS.md](../../../AGENTS.md) when the correct direction is unclear or risky:
- 1 concrete problem
- 3 viable options
- 1 recommendation

Do not silently skip deliverables. Do not "fix" the problem by weakening the ticket without user confirmation.

When the user confirms a direction that changes the ticket's exact architecture boundary, affected files, or proof surface, update the relevant ticket sections before coding so the implementation and eventual archive remain faithful to the chosen plan. This commonly includes `Files to Touch`, `Verification Layers`, and `Test Plan`, not just the prose summary.

If the mismatch is architectural, ambiguous, or would change the owned boundary rather than merely correcting stale references, do not auto-correct it. Surface it first and use 1-3-1 when needed.

### 4. Extract the implementation scope

Turn the ticket into a concrete task list derived from:
- `What to Change`
- `Acceptance Criteria`
- required consequences discovered during reassessment

Separate:
- required in-scope work
- blocked work that needs user direction
- explicit out-of-scope work

When a ticket inherits broader spec language than this specific slice can honestly prove, distinguish:
- the end-state architecture claim the spec is aiming for
- the narrower contract this ticket actually owns after reassessment

Do not keep a later ticket's proof surface artificially broad just because the parent spec states the larger destination. Correct the ticket so its acceptance criteria match the boundary this slice can really deliver.

If the ticket's requested invariant exposes a production contradiction, correct the scope first instead of pretending it is a tests-only change.

For golden tickets, remove duplicate proof from scope unless the new scenario proves a materially different contract from the existing coverage.

If a proposed golden invariant turns out to be real at lower layers but not stably exposable as an E2E golden without changing the scenario's nature, narrow the ticket to the durable golden slice and explicitly preserve the lower-layer boundary as the authoritative proof surface for the rejected piece.

When a golden-driven ticket mixes still-valid negative coverage gaps with an over-claimed positive agreement proof, preserve the honest golden slice and correct the ticket to include any bounded production fix needed for the exposed contradiction instead of dropping the remaining golden work entirely.

When a shared type changes, treat helper factories, sample fixtures, serialized scenario/config inputs, and schema examples as part of the construction-site sweep, not just direct Rust struct literals.

When a widely used serialized component or profile gains a field, proactively search sibling crates for full struct literals embedded in RON/JSON/YAML tests, bundled scenarios, and schema-shape deserialization fixtures. Do not assume the owning crate's Rust constructors are the only fallout surface.

When a save/runtime struct changes shape, also search for test-side mirror structs and hand-written runtime seed/deserialize helpers that may still encode the old field set even after production save/load code compiles.

When behavior moves from one authoritative profile or component carrier to another, search tests, harness helpers, and scenario setup for places that were expressing that behavior through the old carrier. Rewrite those setup paths onto the new authoritative carrier rather than only deleting the stale field from literals.

For component-registration work, distinguish:
- the authoritative schema declaration itself
- all live macro-expansion sites or generated API surfaces that materialize the component set
- runtime code-generation or macro-expansion sites that truly require the bare type in scope
- test-only helper or manifest sites that mirror the component set

Do not assume every file that references the schema macro needs a new top-level import; verify actual local type use first.

When new components participate in persisted world state, check existing save/load or snapshot roundtrip fixture builders as part of the registration sweep. Expand those builders when needed so broad persistence tests actually serialize and deserialize the new components instead of only proving the schema/version boundary changed.

For trait-surface tickets, do not assume the named trait is implemented directly at each consumer. Verify whether the live architecture uses forwarding macros, blanket impls, or paired runtime traits, and correct the ticket if the implementation boundary is broader than the original prose.

When trait-surface reassessment exposes more than one plausible ownership shape for the new API, decide that shape before broad implementation. Prefer the form that keeps the canonical path honest while avoiding unnecessary fixture and test-double churn across dependent crates.

When a ticket is an explicit staged extraction step, temporary duplicated logic is acceptable only if the caller-rewire or old-path removal step is already owned by a named follow-up ticket. Correct the current ticket to state that boundary explicitly instead of leaving the duplication looking accidental.

### 5. Implement with Worldwake discipline

1. Keep edits minimal and targeted.
2. Prefer the existing abstraction boundary instead of duplicating logic.
3. Use TDD for bug fixes:
   - add or update a test that captures the bug
   - confirm it fails for the right reason
   - fix the behavior
4. Never adapt tests to preserve a bug.
5. Do not add backward-compatibility shims, aliases, or dual paths.
6. Preserve critical invariants from [AGENTS.md](../../../AGENTS.md), especially:
   - belief-only planning
   - information locality
   - append-only event log
   - determinism
   - conservation
   - unique location
7. If authoritative validation, control checks, action preconditions, target specs, or other affordance-surface behavior changes, verify the full AI pipeline called out in `Authoritative-To-AI Impact Rule` in [AGENTS.md](../../../AGENTS.md). If the change removes candidates earlier in that pipeline, update stale downstream planner/search harness expectations to the new admission contract instead of weakening the implementation to preserve obsolete traces.
8. When widening an existing action into a new custody or state regime, audit all related stored state carriers so the moved entity does not keep stale assignment, listing, queue, or other regime-specific markers after the transition.
9. When adding a new enum variant, search for exhaustive matches, pattern arms, and state validators in dependent crates and update the non-owning handlers explicitly before broad verification.
10. When adding a field to a shared model, trace, or other cross-module state carrier, proactively search for hand-written constructors and test literals in sibling modules that build that struct directly. Do not rely only on later compile fallout to discover stale fixture sites.
11. When a ticket turns an action from single-shot validation into a staged lifecycle, prove each phase separately: start admission, intermediate local-state evolution, commit conditions, and abort-side aftermath. Do not assume start-time validation and post-abort consequences share the same proof boundary.
12. When a ticket splits previously uniform behavior into class-, variant-, or profile-specific rules, search for existing focused tests that currently compress those cases into one expectation and rewrite them into explicit per-case proofs instead of only adding new tests alongside stale broad assertions.

### 6. Verify at the right boundary

Run the narrowest correct verification first, then broaden when warranted.

Typical order:
1. focused test covering the changed behavior
2. crate-level tests for the affected crate
3. broader workspace validation if the change crosses boundaries or the ticket requires it

When the change touches more than one focused proof surface inside the same crate, run each focused selector needed to cover those boundaries rather than assuming one name filter is sufficient.

If a canonical interface is realized through a forwarding layer, prove both:
- the canonical consumer-facing call
- the forwarding or runtime path that actually materializes it

Check that each focused selector actually matches the new or changed test names. A thematic filter can miss sibling tests in the same implementation slice when their names do not share the expected prefix.

If you change code after a broader verification pass, rerun the narrowest affected tests and any broader command whose earlier result is now stale. This includes post-clippy cleanup or other late mechanical edits in files that already passed earlier tests.

When CI or clippy forces a bounded reshape of a shared function signature or parameter bundle during implementation, proactively sweep all production call sites, same-file test invocations, and public re-exports before the next verification pass. Do not rely on staggered compile fallout to discover the remaining invocation sites one by one.

When a migration changes a public constructor into a zero-arg `new()` or otherwise reshapes a common API surface, expect lint fallout as well as compile fallout. Re-run CI-matching clippy after the constructor sweep and satisfy trait expectations such as `Default` when that is the clean architectural fit rather than suppressing the lint.

When a required verification tool or script invokes broader repo checks and exposes an adjacent blocker outside the ticket's main architecture change, distinguish:
- ticket-owned fallout that the current ticket should absorb
- toolchain or verification-gate fallout that only surfaced because the required check reached farther

If the blocker is small, local, and necessary to complete the required verification path, repair it and keep the ticket scope honest about why that extra edit happened. If it is broader or would materially expand the ticket beyond its corrected boundary, stop and use 1-3-1 instead of silently absorbing unrelated work.

If stronger lawful behavior now reaches completion earlier than an older focused test assumed, recalibrate the test inputs to preserve the intended proof surface instead of weakening the implementation to preserve stale timing or valuation assumptions.

When a golden proves durable learned-state aftermath such as memory records, counters, or timestamps, assert the semantic contract unless exact tick identity is itself the owned invariant. If only ordering or recency matters, prove that boundary directly instead of pinning the record to `current_tick`.

If an attempted acceptance proof reveals a deeper shared-layer contradiction outside the ticket's corrected scope, do not silently pull that broader fix into the current ticket. Update the ticket's proof surface back to the owned boundary unless the broader change is already in scope or the user explicitly confirms the expansion.

Use the repo-approved commands from [AGENTS.md](../../../AGENTS.md) when relevant, especially:

```bash
cargo test -p worldwake-core <test_name>
cargo test -p worldwake-core
cargo test -p worldwake-ai
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

For AI, planner, golden, or start-failure work, prove behavior at the strongest available layer instead of relying on a weaker downstream proxy.

When adding start-failure aftermath recording before action instantiation, check whether the surrounding start path normally abandons empty transactions. Preserve that contract so a new failure hook does not emit orphaned events, fake commits, or other empty-transaction side effects when it records nothing.

When a ticket claims cross-layer valuation agreement, explicitly check whether the shared scorer is computing marginal value over the actor's current accessible stock, especially when the proof depends on current-vs-receipt or retain-vs-transfer snapshots.

When a ticket changes whether an action should be available at all, include at least one focused proof that goes through real affordance enumeration rather than only constructing action instances directly.

When a valid architecture change makes an existing golden scenario stale, update the golden to prove the new lawful contract rather than preserving outdated failure reasons, plan shapes, or scenario narratives.

If the architecture change invalidates the old scenario invariant itself rather than just a timing detail or output shape, rewrite the scenario to prove the new contract and update the scenario header/comments to match.

When adding or renumbering a `// Scenario N:` block in a golden file, treat scenario identifiers as repo-global. Pre-scan nearby or highest live scenario IDs when practical, and be prepared to resolve collisions before the golden inventory refresh can pass.

When a ticket adds, removes, or renumbers a `// Scenario N:` block, refresh the generated golden inventory/docs and treat that refresh as part of the owned verification surface unless the ticket explicitly says otherwise.

When a ticket intentionally lands pure scaffolding ahead of downstream integration, either wire the immediate call sites if they are in scope or mark the temporary unused surface deliberately and record why. Do not let staged helper work fail later CI clippy passes by accident.

For migrations that move configuration or profile state from a driver-global field into authoritative per-entity components, use this checklist during reassessment and closeout:
- remove the driver/global field and constructor arguments
- move custom test or golden setup onto authoritative component writes for the relevant entities
- update runtime/save-load mirrors and manual serialization helpers
- add or update harness helpers for per-entity profile injection when repeated setup would otherwise sprawl
- rerun both tests and CI-matching clippy after the API reshape

### 7. Close the loop on the ticket

If the user asked for full ticket completion, update and archive the ticket per [docs/archival-workflow.md](../../../docs/archival-workflow.md).

When archiving:
- mark completion status accurately
- add an `Outcome` section with what changed and how it was verified
- note any approved partial completion and create a follow-up ticket when required

If the user asked only for implementation or analysis, do not archive automatically, but keep the factual completion details current enough that a later archival pass can record outcome and verification without reconstructing the session from scratch.

When golden coverage changes, keep the scenario prose and proof claim aligned with the updated assertions so the documented contract stays traceable.

When golden scenario metadata changes, re-check the generated golden inventory/docs before finishing so the scenario map, coverage matrix, and inventory stay consistent with the landed scenario IDs and prose.

Do not archive automatically if the user only asked for implementation or for analysis.

Before finishing, re-check ticket sections such as `What to Change`, `Files to Touch`, `Verification Layers`, and `Test Plan` against the actual landed diff and verification commands. Remove reassessment-only fallout from those sections if it did not become a real edit or proof surface.

If the ticket includes inline code snippets, example signatures, or mini before/after API sketches, re-check those examples against the final landed shape as part of closeout. Do not leave a ticket accurate in prose but stale in its embedded code examples.

## Guardrails

- Correct stale ticket assumptions before coding against them.
- Name exact files, symbols, layers, and invariants for non-trivial claims.
- Treat tests, traces, event logs, and authoritative state as different proof surfaces.
- If a golden or mixed-layer scenario narrative diverges from live code, correct the narrative first.
- If you hit a real architectural contradiction, solve the contradiction or escalate with 1-3-1. Do not patch around it.
