# Reassessment Checks

Detailed validation checklists for Step 2 of the ticket implementation workflow.

## Planner-specific reassessment

For planner-root, snapshot-completeness, or planner-traceability tickets, cite the relevant live contract from [docs/planner-contracts.md](../../../../docs/planner-contracts.md) during reassessment instead of reconstructing planner behavior from archived tickets, stale scenario prose, or local implementation fragments alone.

When the ticket is an audit-then-fix (e.g., "verify path X, fix if needed"), treat the audit as reassessment. Record findings in the reassessment section. If a gap is confirmed, auto-correct `Engine Changes`, `What to Change`, and `Files to Touch` before coding. If no gap exists, close with a reassessment-only Outcome documenting the audit trail.

If focused traces, regression tests, or lower-layer proofs falsify the current implementation hypothesis after coding has already started, stop and reassess immediately. Restate the live boundary, update the ticket sections that define owned scope, remove stale partial edits from the disproved approach, and only then continue.
If the falsified hypothesis was the ticket's core implementability claim rather than just one candidate fix, switch from implementation to rejection-or-successor triage immediately: revert the disproved code path, restate the live contradiction in the active ticket, decide whether the current ticket becomes a factual rejection record or narrows to a remaining valid slice, and create a successor ticket when real work remains.

## Reference and baseline validation

- Referenced files, types, functions, modules, commands, and tests exist.
- When the ticket's owned surface is partially landed in the worktree, treat the live state as baseline; limit edits to the missing slice.
- For CLI/scenario tickets, verify that authored bootstrap data populates the same live runtime registries, catalogs, and canonical bootstrap state the ticket expects. Do not treat per-entity wiring as sufficient until the scenario/bootstrap path and the runtime path agree on the same source of truth.
- Cross-check `Deps` against `What to Change` for additive tickets that assume earlier slices landed.
- When the ticket belongs to a numbered family or references a parent spec with split follow-up tickets, scan sibling tickets in that family before coding. Confirm whether adjacent missing substrate is already owned elsewhere so the current ticket neither over-claims sibling work nor narrows away an unowned gap.
- For staged decomposition tickets, verify whether any temporary carrier or intermediate shape named in the ticket still exists on the current branch. If an earlier slice already removed it, narrow the ticket to the remaining live debt.
- When roadmap summary, active spec, and live ticket disagree, compare all three and record which is authoritative.
- When the ticket extracts or reuses private helper logic, confirm exact crate/file ownership before finalizing the plan.
- Described architecture still matches live code.
- Stated coverage gaps are real and correctly classified.
- When adding, extending, or newly reading a universal agent profile or other always-present bootstrap component, verify both the schema registration/read surface and the canonical default-seeding path (for example `World::create_agent()` plus any transaction/bootstrap delta tests). Do not treat registration alone, or a plausible "component may be absent" assumption, as satisfying the live universal-profile contract.
- When a canonical registry or catalog gains entries (for example recipe, action, component, or manifest registries), sweep for hardcoded `RecipeId`, `ActionDefId`, ordinal, or registration-order assumptions in tests and helpers. Prefer resolving by name unless the stable ordinal is itself the owned contract.
- For scenario/world-authoring tickets, state whether the same runtime fact is currently authored through more than one lawful path, which path is canonical after the change, and whether any duplicate authoring path remains intentionally supported or is deferred to a named follow-up ticket.
- When reassessment changes the live root cause or owned surface, apply the section-update rule (see Section 3, "Affected section updates").
- When a ticket names campaign, harness, or telemetry metrics as proof obligations, verify the live output contract. Confirm the actual emitted keys, counters, and summary carrier instead of assuming the ticket's metric names are still current.
- When replacing inline code with a delegation to data populated by a prior ticket, verify line-by-line that the prior ticket's data captures every branch of the original code.
- When a ticket adds a pruning gate, prefilter, or early-return check in front of an existing helper that still decides the final lawful opportunities, compare the new gate predicate against the full live downstream helper contract before coding. Do not let the new front-door filter silently narrow branches the downstream helper would still lawfully admit (for example seller lots, corpse inventory, recipe-backed acquisition, or other non-obvious evidence families).

## Golden-specific reassessment

- Claimed missing scenarios are not already covered by current `golden_*` suites or generated golden inventory/docs.
- Identify the strongest existing owning `golden_*` suite before accepting the ticket's proposed file list; reuse existing ownership surfaces instead of creating new golden files.
- When existing goldens appear to cover the domain, verify whether they exercise the authored/runtime path under ticket ownership or bypass it through direct harness/world construction.
- When a failing golden motivates the ticket, restate the owned invariant before editing and decide whether the contradiction is most honestly proved at the golden layer or at a lower production layer. Prefer the strongest lower-layer proof for root cause.
- When the same scenario currently exists twice in one golden file as an active stale expectation plus an ignored optimistic "after fix" duplicate, treat that as a transitional ownership smell rather than two independent proof obligations. First determine the live contract on the current branch, then collapse the pair to one honest active regression surface for that scenario instead of carrying both copies forward.
- When same-domain verification fails, first check the referenced spec and any active sibling tickets for an explicit owner of that fallout before touching tests or broadening scope.
- When a shared concept has both upstream producers and downstream consumers, compare their semantics directly. If the consumer already supports a broader shape, correct the ticket to own that parity fix.
- If a claimed divergence is proved at lower layers but not stably isolatable as a golden without scenario-distorting scaffolding, correct the ticket to the strongest honest golden contract.
- For golden communication or information-path tickets, verify separately what actually degrades: provenance, confidence, communication class, eligibility, ranking, or another distinct mechanism.
- When a ticket proposes extending an existing trace/debug carrier, verify the exact live coverage of that carrier before coding.
- Scan the referenced spec for explicitly anticipated golden fallout, timing-sensitive scenarios, or downstream validation tickets. Use that mapping when triaging new failures.
- When the ticket adds candidate generation or goal model integration for a domain that already has golden coverage, run the existing golden suites for that domain as part of reassessment, before implementation begins. This catches cross-goal interference early.
- When a golden ticket proposes specific GoalKind pairs, verify that each goal's declared ops (in `goal_dispatch_decl.rs`) include the required PlannerOpKind. Correct the ticket's domain if not.
- When the ticket claims a specific scenario ID is free, verify by scanning all `golden_*.rs` files for that ID. Update the ticket if taken.
- For planner continuity or same-goal branch-stability bugs, triage in this order: is the committed branch absent from candidate generation, removed by a snapshot/read filter, reordered behind interleaved goals, or rejected later by search/start validation? Fix the earliest concrete layer.
- When canonical runtime registries change, sweep golden/conformance helper builders that mirror those registries. Keep helper registries aligned with runtime unless the divergence is explicitly owned and documented.

## Shared type, serialization, and persisted-shape sweep

When shared types, serialized carriers, or persisted components change, sweep these surfaces:
- Serialized fixtures, bundled scenarios, schema examples, and RON/JSON/YAML test inputs
- Helper factories, sample builders, and full struct literals across workspace crates
- Test-only mirror structs and manual bincode/seeded deserialize helpers
- Error, trace, request, and report carriers that store embedded enums by value
- Save/load version boundaries and `SAVE_FORMAT_VERSION` gates
- Crate-root re-exports and downstream imports for new shared types
- CLI handlers, diagnostic bins, renderers, and inspect/output code that read moved fields directly
- When a new shared type is defined under a submodule, verify the actual public import path before patching downstream crates.
- When a flat internal carrier becomes nested or decomposed into sub-structs, sweep both the type name and moved field names across the owning crate.

**Persisted-shape checks:**
- No legacy save support by default. When persisted shape changes, update the current save format; keep older versions rejected unless the user explicitly asks for compatibility.
- When removing or reclassifying persisted fields, search for stale tests, helpers, or docs that still assume older save versions load successfully.
- When adding persisted fields, make focused save tests populate those exact new fields with non-default values and assert after roundtrip.
- When introducing new persisted components alongside a temporary legacy carrier, keep the runtime boundary honest within the live current format.
- When a staged migration moves consumers off a legacy carrier but a later ticket owns removing it, classify remaining references by surface: production reads, test-only helpers, public re-exports, setup fixtures. Eliminate production reads within the current ticket's boundary.

## Helper, math, and default validation

- When behavior depends on helper math, scaling, or threshold arithmetic, inspect the exact live helper implementation and correct stale numeric prose.
- When the ticket proposes concrete default or profile values, compare against live fixtures, schema samples, and roundtrip examples.
- When migrating a shared field's type, verify whether that field carries more than one world meaning. If one scalar collapses distinct semantics, correct the ticket to split them.
- When a benchmark or profiling ticket introduces segmented telemetry, verify whether each configured segment is guaranteed to produce samples. Correct the ticket to allow an explicit empty-state result (e.g., `NA`) instead of forcing a fabricated numeric ratio.

## Action and behavior domain checks

- When a ticket mixes action admission rules with periodic maintenance behavior, identify which layer already owns each invariant.
- When a system or maintenance ticket claims a new transition-specific event/log surface, verify first whether ordinary `WorldTxn` component-delta events are already the canonical carrier.
- When attaching aftermath or evidence to an existing action family, verify whether the handler spans multiple custody, location, or target subcases. Narrow to the applicable subtype.
- When the ticket relies on passive perception of place-bound state, verify the place entity is observed through the same path as co-located entities.
- When the ticket changes contested access to a scarce affordance, decide explicitly whether the domain uses pure race resolution or lawful waiting via queue/grant/reservation. Surface contradictions with 1-3-1.
- When the ticket names S44 contention helpers, verify helper semantics match the live `ContentionPolicy` shape.
- When widening a shared callback or execution signature, search dependent crates for both production call paths and test-only handler registrations.
- When a shared execution or runtime context struct gains a field, search for manual struct literals across both production and test code.

## AI pipeline and affordance checks

**Affordance prerequisites:**
- When affordance generation depends on self-authoritative profile reads, verify those prerequisites in both production code and test harnesses.
- When proving real affordance enumeration against co-located agents/items/places, verify whether the affordance query also depends on the actor already believing those targets are present. Seed the corresponding belief/perception prerequisite in tests.
- When a ticket gates one agent's affordance on another agent's private belief carriers (e.g., `ExpectationStore`, `LastSeenMemory`), verify the read surface. In `PerAgentBeliefView`-style boundaries these may be self-only; cross-agent checks may need to stay actor-local at affordance time.
- When the ticket asks an existing query to distinguish new enum variants, verify the current read surface exposes enough information.
- When the ticket depends on UtilityProfile or disposition gating, verify the belief/read trait exposes that carrier.

**Goal and candidate pipeline:**
- When the ticket claims a goal family should become behaviorally selectable, check the full AI admission path: candidate generation, goal-policy suppression, ranking, selection.
- When a ticket audits threshold alignment between candidate emission and goal satisfaction, also inspect the matching hypothetical planner transition. Record whether one step or repeated steps clear the relevant band.
- When an existing operator becomes newly goal-satisfying for an additional goal family, compare operator legality across every live goal family that consumes that operator.
- When making a payload-override action live through the AI pipeline, compare planner-step revalidation against runtime request resolution.

**Typed queries and staged variants:**
- When adding a typed query alongside an existing boolean helper, verify boolean equivalence.
- When the ticket gates behavior on a typed right from a specific provenance source, verify whether right existence alone is lawful or the producing carrier is part of the contract.
- When a staged ticket introduces a shared enum before all variants are producible, distinguish "type surface lands now" from "variant becomes live now." Test reserved variants as absent.

**Belief and projection surfaces:**
- When the ticket keeps an action family unified while widening to new entity kinds, inspect `TargetSpec`, affordance enumeration, authoritative validation, planner semantics, and payload validators.
- When extending a projected belief or derived state, check for parallel snapshot builders, event carriers, or projection helpers.
- For cache/compression/performance tickets over derived belief or summary state, verify whether the derived surface depends only on stored membership or also on external inputs such as `current_tick`, activation, ordering, or other live context. Do not approve "changed set only" invalidation unless the ticket's contract models every input that can change the derived winner.
- When a new world artifact becomes perceivable and the spec says discovery affects behavior, verify at least one lawful downstream consumer exists.
- When the ticket says information should be "internalized," search for an existing belief lane or consumer before inventing a new belief substrate.
- When the ticket changes historical event content or view semantics, inspect renderers and detail views for reconstruction from live runtime state instead of stored event records.

**Planning state parity:**
- When making a new action handler's affordance enumeration live through the planner's search pipeline, verify that every `RuntimeBeliefView` method the handler calls is implemented on `PlanningState` (via `PlanningSnapshot`), not just on `PerAgentBeliefView`. The planning state's view defaults most trait methods to `None`.
- For trait-extraction tickets that move `RuntimeBeliefView` methods onto new sub-traits, audit `PlanningState` / `PlanningSnapshot` parity before broad mock fallout. When the snapshot doesn't carry the lawful backing state, widen the snapshot boundary deliberately rather than defaulting to `None`.

## Registry and schema checks

- For component-registration tickets, check hardcoded schema inventories, sample `ComponentValue` enumerations, and manifest-style tests.
- When registering a new authoritative component, search for hand-maintained `ComponentKind` inventories and sample builders outside the registration macro.
- When registering a new universal profile on `EntityKind::Agent`, or widening reads/tests around an existing one, verify the runtime bootstrap path that makes the profile universal, not just the schema entry. Sweep agent factory/default-seeding code and any delta/assertion tests that encode the bootstrap component set, and reject negative proofs that rely on agents lawfully lacking the seeded component.
- When a scenario ticket adds authoritative components to places, verify whether place entities are topology-owned and created before `World::new(topology)`. Land component assignment in the bootstrap `WorldTxn` phase if so.
- When renaming or replacing an authoritative identifier, search display strings, manifest inventories, serialized name surfaces, and identity-assertion tests.
- When adding or reordering a `SystemId`, verify separately whether runtime dispatch uses a dense ordinal (`SystemId::ALL`) vs. a distinct manifest (`SystemManifest::canonical()`). Update each independently.
- When introducing a new shared type alongside an existing model family, include crate-root re-exports and downstream imports in the sweep.

## Trait surface checks

- When extending a narrow trait or read surface, check for forwarding macros, blanket impls, paired runtime traits, or generated surfaces. Distinguish the canonical consumer boundary from implementation-detail mirrors.
- If a concrete type receives the target trait through a forwarding macro, treat the owned implementation boundary as potentially spanning the source trait, any paired runtime trait, and the macro site itself.
- When widening a shared trait, choose the narrowest ownership/borrowing form that preserves the canonical consumer path while minimizing snapshot and test-double fallout.

**Trait extraction sweep (for trait-split tickets):**
- Run a workspace-wide fallout sweep before editing: search for the old impl boundary and any forwarding macros or trait-forwarding sites across all crates, tests, and golden helpers.
- Prefer an all-target compile-only pass (`cargo test --workspace --no-run`) immediately after the first broad patch and before full test execution (see also Section 4, Type-change scope for all-targets guidance).
- Before rewriting impl blocks or UFCS calls, write down the exact moved method set and the exact methods remaining on the old trait. Use that partition as the source of truth.
- When splitting methods onto a new trait that provides non-panicking defaults, verify each production implementor still overrides every behaviorally required method. Add focused proof for any moved method whose default could silently preserve compilation while changing behavior.
- After moving methods, sweep for: stale UFCS calls on the old trait, method-call sites requiring the new trait import, dot-call fallout (`view.moved_method(...)`), helper methods and test-local impl internals (`self.moved_method(...)`).
- When blanket impls introduce a second lawful provider for an existing method name, sweep for ambiguity fallout requiring explicit trait qualification.
- When a trait split touches large mock/adapter/test-stub impl blocks, prefer replacing the whole impl partition in one pass over patching methods incrementally.
- Include shared golden harnesses and golden test infrastructure in trait-split fallout sweeps.
- Prioritize broken production implementors over broad mock cleanup when the first compile wave points there.

## Performance and allocation sweep

- When eliminating allocation on a hot path (e.g., replacing `format!` with a structured enum variant), verify all consumers of the changed return type: `.is_ok()`, `.unwrap_err()`, `.map_err()`, pattern matches, `Display`/`to_string()`. Include the exhaustive-match sweep from Section 5, Enum variant handling.
- When adding a boolean fast-path alongside an existing `Result`-returning function, verify both paths agree on the same inputs.
- When refactoring a function to accept pre-computed results by reference, enumerate all call sites and verify each passes the correct pre-computed data.
- When changing a trait method's return type from owned to borrowed (`T` -> `&T`), identify test mocks that construct the return value on-the-fly. Refactor those mocks to pre-populate owned storage and return references.

## Repo rules

- Ticket fidelity from [AGENTS.md](../../../../AGENTS.md)
- Foundational compliance from [docs/FOUNDATIONS.md](../../../../docs/FOUNDATIONS.md)
- Ticket structure from [tickets/_TEMPLATE.md](../../../../tickets/_TEMPLATE.md)
- When a documentation ticket edits repo policy surfaces, check sibling guidance files with overlapping authority (`AGENTS.md`, ticket-authoring docs).
