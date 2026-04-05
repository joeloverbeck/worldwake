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
   - when the ticket's owned surface is already partially landed in the current dirty worktree, treat the live worktree state as the implementation baseline; verify which deliverables are already satisfied and limit edits to the missing slice instead of assuming the ticket still starts from an untouched branch state
   - when a ticket presents itself as a later additive step on top of a file, module, or surface created by an earlier ticket, cross-check `Deps` against its own `What to Change`; if the text already assumes that earlier slice landed, correct stale `Deps` before coding
   - when roadmap summary text, an active spec, and the live ticket disagree about whether a slice is still active, compare all three explicitly and record which surface is authoritative for the current turn before coding
   - when a ticket extracts or reuses private helper logic, confirm the exact current crate/file ownership of that helper before finalizing the implementation plan or `Files to Touch`
   - described architecture still matches the live code
   - stated coverage gaps are real and classified correctly
   - for golden-driven tickets, claimed missing scenarios are not already covered by the current `golden_*` suites or the generated golden inventory/docs
   - for golden-only or golden-closeout tickets, identify the strongest existing owning `golden_*` suite before accepting the ticket's proposed file list; if the scenario belongs in an existing suite, correct the ticket to reuse that ownership surface instead of creating a new golden file by habit
   - when a ticket depends on a named shared concept that already has both upstream producers and downstream consumers in live code, compare those semantics directly before accepting the ticket's narrative. If the downstream consumer already supports a broader lawful shape than the current producer emits, correct the ticket first to own that producer/consumer parity fix instead of treating the gap as golden-only fallout
   - for golden-driven tickets, if a claimed divergence is already proved at lower layers but does not remain stably isolatable in the current golden harness without scenario-distorting scaffolding, correct the ticket to the strongest honest golden contract and record which lower-layer proof remains authoritative for the rejected slice
   - for golden communication or information-path tickets that rely on "degradation," verify separately what actually degrades in live code: provenance/source, confidence, communication class, eligibility, ranking, or another distinct mechanism. Do not let the ticket collapse different degradation paths into one stale narrative
   - when shared types change, serialized fixtures, bundled scenarios, schema examples, and other non-Rust deserialization inputs still match the live struct shape
   - when a ticket changes a shared serialized event/log carrier such as `EventPayload`, `EventRecord`, or another append-only causal model, sweep all workspace crates for hand-written literals, fixture builders, and harness constructors instead of limiting the fallout search to the owning crate
   - when a ticket widens a commonly embedded shared enum such as `ActionPayload`, inspect error, trace, request, and report carriers that store that enum by value, not just direct payload consumers. Size-driven CI fallout such as `result_large_err` can surface far from the owning payload file once the enum grows
   - when an authoritative persisted component or profile changes serialized shape, inspect the live save/load version boundary and correct version gates such as `SAVE_FORMAT_VERSION` when the format contract changed
   - when a ticket adds an explicit migration path for an older save version, search for stale save-version rejection tests and other version-assumption assertions before coding. Distinguish versions that should now load through migration from truly unsupported intermediate or unknown versions instead of leaving old "previous version must fail" tests intact by habit
   - when a ticket introduces new persisted components or profiles alongside a still-live legacy carrier for the same behavior, explicitly check coexistence coherence before coding. If setup, save, or scenario paths can still lawfully write only the legacy carrier, decide whether the new carriers must be derived from it during the migration window or whether every such setup path is in scope to migrate immediately; do not leave the staged world able to carry behaviorally divergent copies of the same agent state
   - when a staged migration ticket moves live consumers off a legacy component or profile but a later ticket still owns removing that legacy carrier, classify every remaining reference by surface before coding: production reads, test-only compatibility helpers, public re-exports, and setup fixtures. Eliminate production reads within the current ticket's boundary, but do not treat every surviving test-only or temporary compatibility reference as a failure if a named follow-up ticket still owns final carrier removal
   - when a ticket's intended behavior depends on helper math, scaling, saturation, or threshold arithmetic, inspect the exact live helper implementation and correct stale numeric prose before coding
   - when a ticket proposes concrete default or profile values, compare them against live representative fixtures, schema samples, and world-roundtrip examples before coding; if the ticket's values are placeholders or stale, correct them to the strongest live baseline first
   - when a ticket migrates the type of a shared field, verify whether that field currently carries more than one world meaning. If one scalar is acting as both coverage and canonical locality, or otherwise collapsing distinct semantics into one slot, correct the ticket to split those meanings before doing a mechanical type migration
   - when save/runtime structs or other persisted shapes gain or lose fields, search for test-only mirror structs and manual `bincode`/seeded deserialize helpers, not just production fixtures
   - when a ticket depends on shared static data such as recipe definitions, schemas, or other registry-backed content, confirm the live service bundle, execution context, or callback boundary that would need to carry that data; if the current runtime boundary does not expose it, correct the ticket before coding to name the real substrate change
   - when a ticket mixes action admission rules with periodic system-maintenance behavior, identify which layer already owns each invariant. If admission-time validation or enqueue logic already enforces a contract, do not restate it as new maintenance-system work unless the live code truly needs a second repair path
   - when a ticket attaches new aftermath, evidence, or other causal residue to an existing action family, verify whether that handler spans multiple lawful custody, location, or target subcases. If the spec's new aftermath only lawfully applies to one subtype, correct the ticket to that narrower subtype before coding instead of emitting the residue for every live branch
   - when a ticket relies on passive perception of place-bound state, verify whether the current place entity is actually observed through the same path as co-located entities. If the place entity is not part of the generic co-location iteration, correct the ticket first to own the explicit current-place projection path instead of assuming place components will appear automatically
   - when one ticket correction reveals a second contradiction in the same owned surface, rerun the boundary check before coding instead of assuming the first correction settled the plan. Helper-level semantics, policy interpretation, or live invariant ownership can invalidate an otherwise-corrected ticket path
   - when a ticket changes contested access to a scarce or exclusive affordance, decide explicitly whether that domain is meant to be pure race resolution or lawful waiting via queue, grant, reservation, or similar world state. If waiting is part of the intended world model, do not implement a rejection-only path as if it were sufficient; surface the contradiction with 1-3-1 and correct the ticket boundary first
   - when a ticket names specific S44 contention helpers, verify that the helper semantics actually match the live `ContentionPolicy` shape for that domain. In particular, for race-mode `max_waiters: Some(0)`, confirm whether the lawful path is queue admission or direct grant acquisition before accepting the ticket's proposed implementation plan
   - when a ticket widens a shared callback or execution signature, search dependent crates for both production call paths and test-only direct handler registrations or manual `on_commit` / `on_abort` invocations so stale harnesses do not survive the initial implementation pass
   - when affordance generation depends on self-authoritative profile reads, those profile prerequisites are present in both production code and representative AI/planner test harnesses
   - for component-registration tickets, hardcoded schema inventories, sample `ComponentValue` enumerations, and manifest-style tests that mirror the authoritative component set still match the live schema after the new entry lands
   - when a ticket registers a new authoritative component, explicitly search for hand-maintained `ComponentKind` inventories, `ComponentValue` sample builders, or other enum/sample manifests that mirror the schema outside the registration macro itself; do not rely on later verification fallout to discover those registry mirrors one by one
   - when a ticket renames or replaces an authoritative system, component, action, or other stable identifier, search not only direct symbol references but also display strings, manifest-style inventories, serialized name surfaces, and tests that assert those stable identities
   - when a ticket adds or reorders an authoritative `SystemId`, verify separately whether live runtime dispatch is keyed by a dense ordinal surface such as `SystemId::ALL` and whether authoritative execution order is carried by a distinct manifest such as `SystemManifest::canonical()`. If both exist, update each independently instead of assuming handler-array order and scheduler order are the same contract
   - when a ticket introduces a new shared type alongside an existing crate-root-exported model family, include crate-root re-export surfaces and downstream imports in the sweep rather than treating the change as field-only fallout
   - when a ticket extends a narrow trait or read surface, check for forwarding macros, blanket impls, paired runtime traits, or other generated surfaces that materialize that API indirectly; if they exist, distinguish the canonical consumer boundary from any implementation-detail mirror the live architecture requires
   - when a ticket widens a shared trait or read surface, explicitly choose the narrowest ownership/borrowing form that preserves the canonical consumer path while minimizing snapshot, harness, and test-double fallout; do not default to borrowed or owned shapes without checking the real cross-crate construction cost
   - when a ticket asks an existing query or derived-state path to distinguish new enum variants or status outcomes, verify that the current read surface exposes enough information to separate every requested case. If the live trait or view cannot lawfully distinguish the promised variants, correct the ticket first to name the required read-surface widening instead of forcing the implementation through incomplete state
   - when a ticket depends on self-authoritative gating from `UtilityProfile`, disposition profiles, or another per-agent authoritative component, verify that the active belief/read trait already exposes that carrier. If the promised gate exists only on authoritative world components and not on the current AI-facing read surface, correct the ticket before coding to include the necessary read-surface widening rather than treating the gate as AI-local implementation only
   - when a ticket claims an already-emitted goal family should now become behaviorally selectable, check the full AI admission path rather than only candidate emission or ranking in isolation: candidate generation, goal-policy suppression, ranking, and the immediate selection path. If a live variant is emitted only under conditions that an existing suppression rule still blocks, correct the ticket before coding instead of treating the work as ranking-only or selection-only
   - when a ticket makes a payload-override action or similarly non-enumerated affordance live through the AI pipeline, compare planner-step revalidation against runtime request resolution. If best-effort request resolution would lawfully accept the step but revalidation still rejects it because no reproduced affordance variant exists, correct that admission-path mismatch before treating the failure as golden-only or selection-only fallout
   - when a ticket adds a typed query alongside an existing boolean helper over the same world relation, verify whether the new typed result is actually boolean-equivalent to the old helper or whether some live variants lawfully exceed that boolean contract. If a typed result such as jurisdiction, advisory access, or another non-control right can be present while the boolean helper stays false, correct the ticket's invariants and focused proofs before coding instead of preserving stale parity assumptions
   - when a ticket gates behavior on a typed right that can be carried by a specific office, faction, institution, container, or other provenance-bearing source, verify whether right existence alone is lawful or whether the producing carrier is part of the contract. If provenance matters, correct the ticket and focused proofs to require the matching `via` source instead of asserting only the right kind
   - when a staged ticket chain introduces a shared enum or model family before all of its variants are lawfully producible, distinguish explicitly between "the type surface lands now" and "this variant becomes a live result now." If the current ticket only owns the type surface, correct the ticket and proof surface so reserved variants are tested as absent rather than silently omitted or prematurely wired
   - when a ticket adds a new shared enum or model variant ahead of later integration tickets, sweep dependent exhaustive tables during reassessment and decide whether each non-owning surface needs explicit inert handling now. Prefer bounded compile-safe, non-live branches over silently reusing an older variant's behavior or stealing later-ticket behavior into the current slice
   - when a ticket keeps an existing action family unified while widening it to new entity kinds, custody states, or target regimes, inspect the shared action-target surface before treating the work as handler-local. Check `TargetSpec`, affordance enumeration, authoritative validation, planner semantics, and payload validators; if the live schema can only bind the old shape, correct the ticket to name that shared action-surface widening first
   - when a ticket extends a projected belief or other derived state model, check for parallel snapshot builders, event carriers, or projection helpers that also reconstruct that model. If more than one path rebuilds the same state, correct the ticket to the shared projection boundary instead of patching only one observation path
   - when a ticket makes a new world artifact, record, or other first-class social object perceivable and the active spec says that discovery should affect behavior, verify that the perceived fact has at least one lawful downstream consumer in scope. Do not land a decorative snapshot field that leaves the new artifact visible but causally inert; broaden or split the ticket instead
   - when a ticket says new information should be "internalized," first search for an already-live belief lane, read-model, or planner/runtime consumer that can lawfully carry the effect. Prefer routing the fact through an existing canonical consumer before inventing a new stored belief substrate unless the ticket explicitly owns that broader model
   - when a ticket changes historical event content or event-view semantics, inspect renderers, traces, and detail views for any reconstruction from live scheduler or world state. If a historical surface is deriving facts from current runtime state instead of the stored event record, correct the ticket boundary to use append-only event data directly
4. Reassess against Worldwake's repo rules:
   - ticket fidelity from [AGENTS.md](../../../AGENTS.md)
   - foundational compliance from [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md)
   - ticket structure from [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md)
   - when a documentation ticket edits repo policy or rule surfaces, check sibling guidance files with overlapping authority such as `AGENTS.md`, `CLAUDE.md`, and ticket-authoring docs; if the same contract should remain mirrored, either update those surfaces in-scope or correct the ticket to say why the mirror is intentionally out of scope
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

If an initial correction exposes a second mismatch in the same owned surface, rerun the boundary check before coding instead of treating the first correction as final. When that later reassessment shows a still-claimed subdomain can no longer be landed in the current ticket and no remaining ticket honestly owns it, create or update the follow-up ticket chain immediately so the active roadmap does not keep implying missing behavior is already covered.

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

When one golden ticket contains multiple scenarios, do not force them to share one proof depth. If one scenario is best proved at decision trace while another is best proved at action trace or authoritative world state, correct each scenario to its own strongest honest boundary rather than flattening the ticket into one uniform assertion style.

When a shared type changes, treat helper factories, sample fixtures, serialized scenario/config inputs, and schema examples as part of the construction-site sweep, not just direct Rust struct literals.

When a widely used serialized component or profile gains a field, proactively search sibling crates for full struct literals embedded in RON/JSON/YAML tests, bundled scenarios, and schema-shape deserialization fixtures. Do not assume the owning crate's Rust constructors are the only fallout surface.

When a save/runtime struct changes shape, also search for test-side mirror structs and hand-written runtime seed/deserialize helpers that may still encode the old field set even after production save/load code compiles.

When behavior moves from one authoritative profile or component carrier to another, search tests, harness helpers, and scenario setup for places that were expressing that behavior through the old carrier. Rewrite those setup paths onto the new authoritative carrier rather than only deleting the stale field from literals.

When a constructor or authoritative factory begins seeding defaults that it previously omitted, reassess tests that were proving "missing component" behavior on freshly created entities. Prefer rewriting those tests to the new constructor contract unless the ticket explicitly owns a lawful post-construction teardown path for that missing-state proof.

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

When a ticket describes itself as "pure additions" on top of an existing boolean/query/helper API, verify whether the honest implementation still requires a bounded internal helper refactor to keep one canonical logic path. If it does, correct sections such as `Engine Changes`, `Architecture Check`, and `Files to Touch` rather than leaving the ticket to imply a duplicate-path implementation.

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
10. When a new shared variant is not supposed to be live yet, make that non-liveness explicit in downstream handlers. Land inert dispatch, policy, ranking, explanation, or display branches as needed to keep the workspace compiling and the architecture truthful, but do not prematurely wire real runtime or planner behavior that belongs to a later ticket.
11. When adding, removing, or replacing an `EntityKind`, include kind-classification helpers in the sweep, not just enum matches and schema registration. Check lifecycle-routing helpers that govern placement, archival, control, or similar world semantics so the new kind participates in the same authoritative contracts as its peers.
12. When adding a field to a shared model, trace, scenario/config type, or other cross-module state carrier, proactively search for hand-written constructors and test literals in sibling modules that build that struct directly, including same-crate test modules outside the owning file. Do not rely only on later compile fallout to discover stale fixture sites.
13. When a ticket turns an action from single-shot validation into a staged lifecycle, prove each phase separately: start admission, intermediate local-state evolution, commit conditions, and abort-side aftermath. Do not assume start-time validation and post-abort consequences share the same proof boundary.
14. When a ticket splits previously uniform behavior into class-, variant-, or profile-specific rules, search for existing focused tests that currently compress those cases into one expectation and rewrite them into explicit per-case proofs instead of only adding new tests alongside stale broad assertions.
15. When a ticket makes a new planner-visible operator lawful for an existing goal family, sweep the planner contract end to end: goal dispatch or relevant-op declarations, progress-barrier rules, goal-model expectations, and search/planner-root tests. Do not stop at affordance or candidate-generation changes if the goal family still advertises the old operator set.
16. When one existing goal family spans multiple target subtypes, fulfillment modes, or domain variants, verify operator availability per subtype instead of only at the family-wide declaration level. Check whether stale operators from one subtype still leak into candidate generation, root synthesis, or goal-model admission for another subtype once the family broadens.
17. When a goal family ends in a place-sensitive terminal action such as `claim_bounty`, do not let focused tests cover only the degenerate same-place shape unless the ticket explicitly owns only that case. If the spec or ticket depends on a distinct terminal place, add focused planner/root coverage for both target satisfaction and return-to-terminal-place legality so root availability does not silently assume co-location.
18. When a newly live planner operator is a colocated `ActorPlace` or similarly same-place leaf action, do not assume travel-plus-leaf coverage proves the leaf contract completely. Verify the colocated terminal case separately: once the actor is already at the goal place, root synthesis or direct leaf admission may still be missing even if remote `Travel -> leaf` planning works. Add a focused planner/root proof for that colocated leaf path when the operator is meant to be directly available at the terminal place.

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

When using exact Rust test-name selectors with `cargo test`, prefer separate invocations per selector unless you are intentionally relying on one shared substring filter. Do not assume multiple exact test names can be passed in a single `cargo test` command.

When verification uses multiple `cargo test` or `cargo clippy` commands against the same workspace, prefer running them sequentially rather than in parallel. Cargo lock contention and in-flight stale compiles can make parallel runs noisier and less trustworthy after recent edits.

If you change code after a broader verification pass, rerun the narrowest affected tests and any broader command whose earlier result is now stale. This includes post-clippy cleanup or other late mechanical edits in files that already passed earlier tests.

When CI or clippy forces a bounded reshape of a shared function signature or parameter bundle during implementation, proactively sweep all production call sites, same-file test invocations, and public re-exports before the next verification pass. Do not rely on staggered compile fallout to discover the remaining invocation sites one by one.

When a migration changes a public constructor into a zero-arg `new()` or otherwise reshapes a common API surface, expect lint fallout as well as compile fallout. Re-run CI-matching clippy after the constructor sweep and satisfy trait expectations such as `Default` when that is the clean architectural fit rather than suppressing the lint.

When a required verification tool or script invokes broader repo checks and exposes an adjacent blocker outside the ticket's main architecture change, distinguish:
- ticket-owned fallout that the current ticket should absorb
- toolchain or verification-gate fallout that only surfaced because the required check reached farther

If the blocker is small, local, and necessary to complete the required verification path, repair it and keep the ticket scope honest about why that extra edit happened. If it is broader or would materially expand the ticket beyond its corrected boundary, stop and use 1-3-1 instead of silently absorbing unrelated work.

If stronger lawful behavior now reaches completion earlier than an older focused test assumed, recalibrate the test inputs to preserve the intended proof surface instead of weakening the implementation to preserve stale timing or valuation assumptions.

When broader required verification surfaces a timing-sensitive golden whose semantic contract still holds, prefer recalibrating the fixture's explicit timing budget, hold window, or other scenario timing inputs to the current lawful scheduler or queue ordering before broadening production scope.

When a golden's narrative assumes an agent already locally observes co-located entities, risks, or other same-place facts at tick 0, verify that the setup explicitly seeds those local beliefs or the exact perception prerequisites needed to produce them. Do not rely on an implicit first-tick perception warmup if the scenario prose claims the observation is already present.

When a golden uses external action requests or other scripted setup to drive part of the world state and that actor's autonomous reasoning is not itself part of the owned proof surface, explicitly set that actor's `ControlSource` to `Human` or `None` before relying on the scripted path. Do not leave a setup actor on `Ai` by default and then treat autonomous interference as evidence about the ticket's intended invariant.

When a golden proves durable learned-state aftermath such as memory records, counters, or timestamps, assert the semantic contract unless exact tick identity is itself the owned invariant. If only ordering or recency matters, prove that boundary directly instead of pinning the record to `current_tick`.

If focused implementation or verification shows that the corrected ticket still over-claims what the live harness can stably prove, narrow the ticket again before final verification so the archive-ready proof surface matches the strongest honest demonstrated boundary.

If an attempted acceptance proof reveals a deeper shared-layer contradiction outside the ticket's corrected scope, do not silently pull that broader fix into the current ticket. Update the ticket's proof surface back to the owned boundary unless the broader change is already in scope or the user explicitly confirms the expansion.

When long-running verification commands are already in flight, prefer polling or reusing those sessions rather than spawning duplicate cargo or tool invocations. This keeps verification readable and avoids unnecessary process or session churn in constrained environments.

When a ticket adds new registered actions or systems, triage broad integration or golden failures for catalog-order drift, completeness assertions, planner-surface assumptions, or other registry-expansion fallout before assuming the new feature's direct runtime logic is broken. Distinguish those secondary verification failures from the ticket's primary owned behavior.

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

For exact-bound planner-root candidates, do not treat target binding as the whole contract when operator legality also depends on intermediate goal state. Verify whether synthesized or affordance-backed root candidates need an additional stateful availability gate so exact-bound actions do not surface before their prerequisite progress has actually been achieved.

When a valid architecture change makes an existing golden scenario stale, update the golden to prove the new lawful contract rather than preserving outdated failure reasons, plan shapes, or scenario narratives.

When a golden transport, delivery, or claim chain is ultimately about durable world-state aftermath, avoid over-specifying intermediate substeps such as a particular `put_down` or travel-start trace unless that substep is itself the owned contract. Prefer authoritative destination state, claim-place state, conservation, and the terminal action-order boundary, then tighten intermediate trace assertions only when the scenario really exists to prove that lower layer.

When a new or corrected golden still fails after the scenario setup is made lawful, reassess whether the golden has exposed a missing lower-layer contract rather than mere fixture fallout. If it has, fix the production boundary, add focused proof at that lower layer, and only then finalize the golden closeout instead of treating the failure as golden-only churn.

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
