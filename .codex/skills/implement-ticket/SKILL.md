---
name: implement-ticket
description: "Implement or reassess a Worldwake ticket. Use when asked to work from a ticket in `tickets/`, `archive/tickets/`, or a worktree ticket path: read the ticket, validate its assumptions against the live codebase and repo rules, correct mismatches before coding when needed, then implement and verify the requested deliverables."
---

# Worldwake Ticket Implementation

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), the target ticket, and any ticket-linked specs or docs before editing code. For planner-root, snapshot-completeness, or planner-traceability work, also read [docs/planner-contracts.md](../../../docs/planner-contracts.md) before finalizing the reassessment. Reassess first, then implement — do not treat a ticket as mechanically executable until its assumptions match the current codebase. Do not stop at intermediate reassessment or partial fallout; continue until the ticket is completed, fully verified, or blocked by a user decision that requires 1-3-1.

## Workflow

### 0. Classify ticket shape and pick the right path

Before running the full workflow, classify the ticket:

**Small/local tickets** (fast path) — single-file additive CLI/tooling/reporting change, narrow helper extraction, formatting update, no shared type/planner/golden/persistence/cross-crate fallout expected:
1. Resolve the exact live ticket/spec path, including typos or shorthand.
2. Confirm the dependency path and the exact owned symbol/file boundary.
3. Run a constructor/usage sweep for the changed shape (see Section 4, Type-change scope).
4. Implement the owned change with focused proof first.
5. Use all-target compile fallout to catch remaining shared-shape literals/helpers.
6. Close out the ticket with the actual verification set and tracked-vs-untracked note.

For CLI/tooling-only tickets, if the owned logic can be factored into local helpers, prefer bin-local `#[cfg(test)]` coverage over command-only validation.

Do not skip reassessment for small tickets, but scale it down: read the ticket, cited references, and owned symbol/file; confirm the dependency path is present; run a narrow existence/fallout sweep for prior implementation or obvious constructor/usage fallout. Do not force the full Section 2 matrix when the owned surface is genuinely small and local.

**All other tickets** — use the full workflow below (Sections 1-8).

When the ticket was authored by `/spec-to-tickets` in the current session from a freshly reassessed spec, scale reassessment to a targeted sweep: confirm the ticket's owned types still exist at stated paths, check for exhaustive matchers on modified enums, verify trait bounds on any types used in new test code, check for manual struct literals of modified types (constructors, test helpers, `from_*_for_test` patterns) that would need updating for new fields, and before adding new test-only accessors or helpers, check whether existing test infrastructure (e.g., `ActualWorldState::from_world`, test harness methods) already provides the needed capability.

### 1. Load the ticket context

1. Read the target ticket file.
2. Read every directly relevant reference (specs, docs, code symbols, test files).
3. When the user supplies a glob, shorthand, or obvious near-match typo, confirm the exact live file path before reading or relying on it.
4. Check whether the active ticket file is tracked or untracked in the current worktree. Untracked ticket drafts are valid active state, but they will not appear in ordinary `git diff` output. Keep this in mind during diffs, close-out evidence, and follow-up ticket reporting throughout the workflow.
5. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all operations.

### 2. Reassess assumptions before coding

Verify the ticket against the current codebase, not stale architectural memory. Check `Deps` — confirm each dependency is present on the current branch. For mixed-layer, planner, golden, or authoritative-validation work, name the exact symbols and boundaries under audit.

For planner-root, snapshot-completeness, or planner-traceability tickets, cite the relevant live contract from [docs/planner-contracts.md](../../../docs/planner-contracts.md) during reassessment instead of reconstructing planner behavior from archived tickets, stale scenario prose, or local implementation fragments alone.

When the ticket is an audit-then-fix (e.g., "verify path X, fix if needed"), treat the audit as reassessment. Record findings in the reassessment section. If a gap is confirmed, auto-correct `Engine Changes`, `What to Change`, and `Files to Touch` before coding. If no gap exists, close with a reassessment-only Outcome documenting the audit trail.

If focused traces, regression tests, or lower-layer proofs falsify the current implementation hypothesis after coding has already started, stop and reassess immediately. Restate the live boundary, update the ticket sections that define owned scope, remove stale partial edits from the disproved approach, and only then continue.

#### Reference and baseline validation

- Referenced files, types, functions, modules, commands, and tests exist.
- When the ticket's owned surface is partially landed in the worktree, treat the live state as baseline; limit edits to the missing slice.
- For CLI/scenario tickets, verify that authored bootstrap data populates the same live runtime registries, catalogs, and canonical bootstrap state the ticket expects. Do not treat per-entity wiring as sufficient until the scenario/bootstrap path and the runtime path agree on the same source of truth.
- Cross-check `Deps` against `What to Change` for additive tickets that assume earlier slices landed.
- For staged decomposition tickets, verify whether any temporary carrier or intermediate shape named in the ticket still exists on the current branch. If an earlier slice already removed it, narrow the ticket to the remaining live debt.
- When roadmap summary, active spec, and live ticket disagree, compare all three and record which is authoritative.
- When the ticket extracts or reuses private helper logic, confirm exact crate/file ownership before finalizing the plan.
- Described architecture still matches live code.
- Stated coverage gaps are real and correctly classified.
- For scenario/world-authoring tickets, state whether the same runtime fact is currently authored through more than one lawful path, which path is canonical after the change, and whether any duplicate authoring path remains intentionally supported or is deferred to a named follow-up ticket.
- When reassessment changes the live root cause or owned surface, apply the section-update rule (see Section 3, "Affected section updates").
- When a ticket names campaign, harness, or telemetry metrics as proof obligations, verify the live output contract. Confirm the actual emitted keys, counters, and summary carrier instead of assuming the ticket's metric names are still current.
- When replacing inline code with a delegation to data populated by a prior ticket, verify line-by-line that the prior ticket's data captures every branch of the original code.

#### Golden-specific reassessment

- Claimed missing scenarios are not already covered by current `golden_*` suites or generated golden inventory/docs.
- Identify the strongest existing owning `golden_*` suite before accepting the ticket's proposed file list; reuse existing ownership surfaces instead of creating new golden files.
- When existing goldens appear to cover the domain, verify whether they exercise the authored/runtime path under ticket ownership or bypass it through direct harness/world construction.
- When a failing golden motivates the ticket, restate the owned invariant before editing and decide whether the contradiction is most honestly proved at the golden layer or at a lower production layer. Prefer the strongest lower-layer proof for root cause.
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

#### Shared type, serialization, and persisted-shape sweep

When shared types, serialized carriers, or persisted components change, sweep these surfaces:
- Serialized fixtures, bundled scenarios, schema examples, and RON/JSON/YAML test inputs
- Helper factories, sample builders, and full struct literals across workspace crates
- Test-only mirror structs and manual bincode/seeded deserialize helpers
- Error, trace, request, and report carriers that store embedded enums by value
- Save/load version boundaries and `SAVE_FORMAT_VERSION` gates
- Crate-root re-exports and downstream imports for new shared types
- When a new shared type is defined under a submodule, verify the actual public import path before patching downstream crates.
- When a flat internal carrier becomes nested or decomposed into sub-structs, sweep both the type name and moved field names across the owning crate.

**Persisted-shape checks:**
- No legacy save support by default. When persisted shape changes, update the current save format; keep older versions rejected unless the user explicitly asks for compatibility.
- When removing or reclassifying persisted fields, search for stale tests, helpers, or docs that still assume older save versions load successfully.
- When adding persisted fields, make focused save tests populate those exact new fields with non-default values and assert after roundtrip.
- When introducing new persisted components alongside a temporary legacy carrier, keep the runtime boundary honest within the live current format.
- When a staged migration moves consumers off a legacy carrier but a later ticket owns removing it, classify remaining references by surface: production reads, test-only helpers, public re-exports, setup fixtures. Eliminate production reads within the current ticket's boundary.

#### Helper, math, and default validation

- When behavior depends on helper math, scaling, or threshold arithmetic, inspect the exact live helper implementation and correct stale numeric prose.
- When the ticket proposes concrete default or profile values, compare against live fixtures, schema samples, and roundtrip examples.
- When migrating a shared field's type, verify whether that field carries more than one world meaning. If one scalar collapses distinct semantics, correct the ticket to split them.
- When a benchmark or profiling ticket introduces segmented telemetry, verify whether each configured segment is guaranteed to produce samples. Correct the ticket to allow an explicit empty-state result (e.g., `NA`) instead of forcing a fabricated numeric ratio.

#### Action and behavior domain checks

- When a ticket mixes action admission rules with periodic maintenance behavior, identify which layer already owns each invariant.
- When a system or maintenance ticket claims a new transition-specific event/log surface, verify first whether ordinary `WorldTxn` component-delta events are already the canonical carrier.
- When attaching aftermath or evidence to an existing action family, verify whether the handler spans multiple custody, location, or target subcases. Narrow to the applicable subtype.
- When the ticket relies on passive perception of place-bound state, verify the place entity is observed through the same path as co-located entities.
- When the ticket changes contested access to a scarce affordance, decide explicitly whether the domain uses pure race resolution or lawful waiting via queue/grant/reservation. Surface contradictions with 1-3-1.
- When the ticket names S44 contention helpers, verify helper semantics match the live `ContentionPolicy` shape.
- When widening a shared callback or execution signature, search dependent crates for both production call paths and test-only handler registrations.
- When a shared execution or runtime context struct gains a field, search for manual struct literals across both production and test code.

#### AI pipeline and affordance checks

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
- When a new world artifact becomes perceivable and the spec says discovery affects behavior, verify at least one lawful downstream consumer exists.
- When the ticket says information should be "internalized," search for an existing belief lane or consumer before inventing a new belief substrate.
- When the ticket changes historical event content or view semantics, inspect renderers and detail views for reconstruction from live runtime state instead of stored event records.

**Planning state parity:**
- When making a new action handler's affordance enumeration live through the planner's search pipeline, verify that every `RuntimeBeliefView` method the handler calls is implemented on `PlanningState` (via `PlanningSnapshot`), not just on `PerAgentBeliefView`. The planning state's view defaults most trait methods to `None`.
- For trait-extraction tickets that move `RuntimeBeliefView` methods onto new sub-traits, audit `PlanningState` / `PlanningSnapshot` parity before broad mock fallout. When the snapshot doesn't carry the lawful backing state, widen the snapshot boundary deliberately rather than defaulting to `None`.

#### Registry and schema checks

- For component-registration tickets, check hardcoded schema inventories, sample `ComponentValue` enumerations, and manifest-style tests.
- When registering a new authoritative component, search for hand-maintained `ComponentKind` inventories and sample builders outside the registration macro.
- When a scenario ticket adds authoritative components to places, verify whether place entities are topology-owned and created before `World::new(topology)`. Land component assignment in the bootstrap `WorldTxn` phase if so.
- When renaming or replacing an authoritative identifier, search display strings, manifest inventories, serialized name surfaces, and identity-assertion tests.
- When adding or reordering a `SystemId`, verify separately whether runtime dispatch uses a dense ordinal (`SystemId::ALL`) vs. a distinct manifest (`SystemManifest::canonical()`). Update each independently.
- When introducing a new shared type alongside an existing model family, include crate-root re-exports and downstream imports in the sweep.

#### Trait surface checks

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

#### Performance and allocation sweep

- When eliminating allocation on a hot path (e.g., replacing `format!` with a structured enum variant), verify all consumers of the changed return type: `.is_ok()`, `.unwrap_err()`, `.map_err()`, pattern matches, `Display`/`to_string()`. Include the exhaustive-match sweep from Section 5, Enum variant handling.
- When adding a boolean fast-path alongside an existing `Result`-returning function, verify both paths agree on the same inputs.
- When refactoring a function to accept pre-computed results by reference, enumerate all call sites and verify each passes the correct pre-computed data.
- When changing a trait method's return type from owned to borrowed (`T` -> `&T`), identify test mocks that construct the return value on-the-fly. Refactor those mocks to pre-populate owned storage and return references.

#### Repo rules

- Ticket fidelity from [AGENTS.md](../../../AGENTS.md)
- Foundational compliance from [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md)
- Ticket structure from [tickets/_TEMPLATE.md](../../../tickets/_TEMPLATE.md)
- When a documentation ticket edits repo policy surfaces, check sibling guidance files with overlapping authority (`AGENTS.md`, ticket-authoring docs).

### 3. Handle mismatches explicitly

If the ticket and live code disagree, stop and surface the discrepancy before implementation.

For each mismatch, state:
- what the ticket says
- what the codebase currently has
- whether the ticket should be corrected, the implementation should adapt, or the issue is blocked

#### Low-risk auto-corrections

Update the ticket immediately (without stopping) when the correction is mechanical and directionally unambiguous:
- Exact live spec path from a user-supplied glob
- Stale file/symbol/test references
- `Files to Touch`, `Verification Layers`, or command lists that need to match current codebase
- Component-registration fallout from live macro expansion or schema discovery
- Stale acceptance criteria, scenario assertion surfaces, or proof targets where the live symbols and behavior make the narrower honest contract directionally unambiguous

Record each auto-correction: ticket says / live code has / correction applied / why safe.
Place notes under the ticket's `Assumption Reassessment` section as numbered entries. If the section is missing, add one.

##### Affected section updates

When any correction changes the real fallout surface, update **all** affected ticket sections in the same pass: `What to Change`, `Files to Touch`, `Verification Layers`, `Test Plan`, and when scope narrows, also `Acceptance Criteria`, `Tests That Must Pass`, and command expectations.

When reassessment converts a ticket into reassessment-only, doc-only, or no-production-change completion, remove leftover placeholder scaffolding from acceptance criteria, verification, test-plan, and command sections.

If all owned proof surfaces already pass and the live outcome is only factual validation plus adjacent-fallout triage, convert the ticket immediately to a validation-only close-out. Remove stale implementation scaffolding, keep the proof surface honest, and create a follow-up ticket for any remaining work.

#### Escalation decision tree

| Situation | Action |
|-----------|--------|
| Low-risk factual mismatch (stale reference, path, command) | Auto-correct; record note |
| One correction reveals a second contradiction in the same surface | Rerun boundary check before coding; do not treat first correction as final |
| Later reassessment shows a subdomain can no longer land and no ticket owns it | Create or update follow-up ticket chain immediately |
| Architectural, ambiguous, or changes owned boundary | Stop; use 1-3-1 (1 problem, 3 options, 1 recommendation) |
| Adjacent blocker exposed by verification — small, local, needed for verification | Absorb; note why in ticket |
| Adjacent blocker — broad or would expand ticket materially | Stop; use 1-3-1 |
| Deeper shared-layer contradiction outside ticket scope | Do not pull into ticket; use 1-3-1 |

When using 1-3-1, evaluate each option against the relevant FOUNDATIONS principles. Name the principle numbers and state whether each option aligns or violates. A FOUNDATIONS violation disqualifies an option regardless of implementation simplicity.

Do not silently skip deliverables. Do not weaken the ticket without user confirmation.

When the user confirms a direction that changes architecture boundary, affected files, or proof surface, apply the affected section updates rule above before coding.

### 4. Extract the implementation scope

Turn the ticket into a concrete task list from `What to Change`, `Acceptance Criteria`, and reassessment findings.

Separate:
- required in-scope work
- blocked work needing user direction
- explicit out-of-scope work

When the ticket inherits broader spec language, distinguish the end-state architecture claim from the narrower contract this ticket owns after reassessment.

When the parent spec describes an eventual causal story but the current ticket only owns substrate or maintenance scaffolding, keep those separate explicitly. Narrow the ticket to the current owned mechanism and name the deferred downstream behavior.

When a ticket bundles multiple deliverables and reassessment narrows the ticket to only one lawful slice, verify every removed deliverable is still owned by an existing active ticket. Create the follow-up ticket before coding if any removed deliverable has no live owner (see Section 3, Escalation decision tree for follow-up guidance).

If the ticket's requested invariant exposes a production contradiction, correct the scope first.

#### Golden scope narrowing

- Remove duplicate proof unless the new scenario proves a materially different contract.
- If a proposed invariant is real at lower layers but not stably exposable as a golden, narrow to the durable golden slice and preserve the lower-layer proof as authoritative.
- When a golden ticket mixes valid negative coverage gaps with an over-claimed positive proof, preserve the honest golden slice and correct the ticket.
- Allow different proof depths per scenario (decision trace, action trace, authoritative state) rather than flattening to uniform assertion style.

#### Type-change scope

When shared types change, include the sweep surfaces from Section 2 ("Shared type, serialization, and persisted-shape sweep") in the task list.

- Before editing, run a concrete constructor/shape sweep for the changed type across workspace crates (e.g., `rg -n 'BlockedIntent \{' crates`), then rerun after implementation.
- For broad shared-struct shape changes, landing the shared type first and using sequential `cargo build` / `cargo test` compile failures to enumerate remaining fallout is acceptable.
- After the first compile wave identifies pure missing-field fallout, a bounded mechanical patch across remaining literals is acceptable before rerunning compile verification.
- Do not treat `cargo build --workspace` alone as exhaustive fallout enumeration for shared-shape changes. Test-only constructors, helper factories, and same-crate test modules can stay hidden until `--all-targets` compilation (e.g., `cargo clippy --workspace --all-targets -- -D warnings`). Include an all-targets verification pass before closing the ticket.
- When behavior moves between carriers, rewrite setup paths onto the new authoritative carrier rather than only deleting the stale field.
- When a constructor begins seeding defaults it previously omitted, reassess tests proving "missing component" behavior.
- When new components participate in persisted world state, expand save/load fixture builders so persistence tests actually serialize and deserialize the new components.

#### Component registration scope

Distinguish:
- the authoritative schema declaration
- live macro-expansion sites or generated API surfaces
- runtime code-generation sites requiring the bare type in scope
- test-only helper or manifest sites mirroring the component set

Verify actual local type use before adding imports.

#### Trait surface scope

See Section 2, Trait surface checks for detailed checks. Additional scope decisions:
- When the named trait is already a stable consumer-facing facade, reassess whether the lawful cleanup is to preserve that facade and decompose only the implementation path beneath it.
- When reassessment exposes multiple ownership shapes for a new API, decide the shape before broad implementation.
- When a widely used helper or wrapper appears to need a signature change, verify whether it is actually the live production boundary or mainly a test/unfiltered convenience surface. Prefer widening only the narrower production entry point when possible.

#### Staged work

- Temporary duplicated logic is acceptable only if a named follow-up ticket owns the caller-rewire or old-path removal. State this boundary explicitly.
- When a ticket describes itself as "pure additions," verify whether an internal helper refactor is needed. If so, correct `Engine Changes`, `Architecture Check`, and `Files to Touch`.

### 5. Implement with Worldwake discipline

#### General discipline

1. Keep edits minimal and targeted.
2. Prefer existing abstraction boundaries over duplicating logic.
3. TDD for bug fixes: add test capturing the bug, confirm it fails, fix behavior.
4. Never adapt tests to preserve a bug.
5. No backward-compatibility shims, aliases, or dual paths.
6. Preserve critical invariants from [AGENTS.md](../../../AGENTS.md): belief-only planning, information locality, append-only event log, determinism, conservation, unique location.
7. When authoritative validation or affordance-surface behavior changes, verify the full AI pipeline per `Authoritative-To-AI Impact Rule` in [AGENTS.md](../../../AGENTS.md).

#### Action lifecycle

8. When widening an action into a new custody or state regime, audit related stored state carriers for stale markers.
9. When turning a single-shot action into a staged lifecycle, prove each phase separately: start admission, intermediate evolution, commit conditions, abort aftermath.
10. When an action uses a profile-driven or expression-driven duration, make test helpers derive or tolerate the real completion window. Do not copy a nearby fixed-duration helper.
11. When splitting uniform behavior into variant-specific rules, rewrite existing compressed tests into per-case proofs.

#### Enum variant handling

12. When adding a new enum variant, search for exhaustive matches and state validators in dependent crates. Also search for:
    - Hardcoded array/vec inventories (`const ALL`, test-only `ALL_KEYS`) and count assertions (`assert_eq!(keys.len(), N)`)
    - Runtime-reachable wildcard catch-all matches that panic (e.g., `(strategy, goal_kind) => unreachable!(...)` in `feasibility.rs`)
    - `Display` impl, cross-crate error-mapping functions (e.g., `map_reservation_error` in `start_gate.rs`), variant-inventory tests, and crate-root re-exports
13. When a new variant is not supposed to be live yet, land explicit inert dispatch/policy/ranking branches.
14. When adding or replacing an `EntityKind`, include kind-classification and lifecycle-routing helpers in the sweep.
15. When adding a field to a shared model, search for hand-written constructors and test literals across sibling modules. When a field's value differs per dispatch variant but current code shares a single constant across multiple variants, split into per-variant constants.
16. When adding a new shared enum variant ahead of integration tickets, sweep dependent exhaustive tables. Prefer bounded compile-safe inert branches over reusing older variant behavior. Also check bounded non-owner exhaustive consumers: ranking/policy code, failure handling, observation/runtime helpers, relay-selection or ordering helpers, renderers, and detail-formatting surfaces. Absorb compile-safe inert handling rather than treating it as separate architecture change.

#### Planner and goal family wiring

17. When making a new planner-visible operator lawful, sweep the full planner contract: goal dispatch, relevant-op declarations, progress barriers, goal-model expectations, heuristic/guidance surfaces (`goal_relevant_places`, evidence-place fallback, travel-pruning inputs when relevant), search tests. Verify the `may_appear_mid_plan` / `is_progress_barrier` combination: with `may_appear_mid_plan=false`, the operator can ONLY appear as a terminal step. With `may_appear_mid_plan=true`, it can appear anywhere.
18. When a planner goal must synthesize a runtime payload, verify the activation chain end to end: the goal carries enough identity, root/current-place guidance makes the operator reachable, and terminal-step semantics treat the action as goal-satisfying.
19. When the first planner fix only makes an operator partially live, immediately re-check the rest of the same operator chain: candidate shape, root synthesis, payload construction, terminal semantics, and the focused planner proof.
20. When one goal family spans multiple target subtypes, verify operator availability per subtype. Check for stale operators leaking across subtypes.
21. When a goal family ends in a place-sensitive terminal action, add focused coverage for both target satisfaction and return-to-terminal-place legality.
22. When a colocated leaf action becomes live, verify the colocated terminal case separately from travel-plus-leaf planning.
23. When adding a new candidate emitter for a domain that already has active goal families, verify the new goal does not cause goal-switching collisions with existing goals for the same target entity. Run existing golden suites for that domain first.
24. When a goal generates as a candidate with nonzero motive but is never selected, diagnose in order: (a) `compute_motive` returns > 0, (b) `synthesized_root_candidate_targets` provides a root candidate, (c) `is_progress_barrier` identifies the terminal op, (d) `build_payload_override` succeeds, (e) `estimate_duration` returns `Some`.
25. When adding a new `GoalKind` variant, use the compiler to surface exhaustive-match sites, but also sweep runtime-reachable surfaces: `GoalDispatchKey` enum + `ALL` array + `from_goal_kind`, `goal_kind_discriminant` in ranking.rs, `feasibility.rs` strategy-goal match, `format_goal_kind` in display.rs, and shared signal/motive helpers. Consider `cargo build --workspace` first for compiler errors, then grep for the closest existing sibling to find runtime-only sites.

### 6. Verify at the right boundary

Run the narrowest correct verification first, then broaden.

Typical order:
1. Focused test covering the changed behavior
2. Crate-level tests for the affected crate
3. Broader workspace validation if the change crosses boundaries

#### Verification mechanics

- When the change touches multiple proof surfaces in one crate, run each focused selector needed.
- If a canonical interface is realized through a forwarding layer, prove both the consumer-facing call and the forwarding path.
- Check that focused selectors actually match new/changed test names.
- Prefer separate `cargo test` invocations per selector over combining exact test names in one command.
- Run multiple `cargo test` or `cargo clippy` commands sequentially when they share the same build profile — lock contention makes parallel same-profile runs unreliable. Different profiles (e.g., `cargo test` vs `cargo clippy`) can safely run in parallel.
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

#### Golden test verification

- If stronger behavior now reaches completion earlier than a test assumed, recalibrate test inputs instead of weakening the implementation.
- When broader verification surfaces a timing-sensitive golden whose contract still holds, recalibrate the fixture's timing budget or hold window.
- When a golden assumes agents observe co-located facts at tick 0, verify the setup explicitly seeds those beliefs or perception prerequisites.
- When a golden uses external action requests for scripted setup, set that actor's `ControlSource` to `Human` or `None`.
- When a golden proves durable learned-state aftermath, assert the semantic contract unless exact tick identity is the owned invariant.
- If focused implementation shows the corrected ticket still over-claims, narrow the ticket before final verification.
- When a valid architecture change makes a golden stale, update it to prove the new lawful contract.
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

#### Migration verification checklist

For migrations moving config/profile state from driver-global to per-entity components:
1. Remove the driver/global field and constructor arguments
2. Move test/golden setup onto authoritative component writes for relevant entities
3. Update runtime/save-load mirrors and serialization helpers
4. Add harness helpers for per-entity profile injection when repeated setup would sprawl
5. Rerun both tests and CI-matching clippy after the API reshape

### 7. Close out the ticket honestly

After the owned implementation is fully verified:

1. Update the ticket's `Status` when the required verification surface has passed.
2. If reassessment, implementation, or broad verification exposed an adjacent but out-of-scope contradiction, create or update a follow-up ticket immediately (see Section 3, Escalation decision tree).
3. If the owned invariant is proved and a broader rerun exposes a different unrelated blocker, close the current ticket honestly, record the broader blocker, and create the follow-up immediately.
4. Give each follow-up explicit `Deps` links to the implemented ticket and any still-pending sibling tickets or active specs.
5. Distinguish clearly between:
   - bugs fixed inside the current ticket
   - compromises accepted to finish the current ticket safely
   - remaining work that needs its own ticket
6. Do not silently broaden the current ticket during close-out. If the remaining work has its own architectural boundary, capture it as a follow-up.
7. Keep scenario prose aligned with updated assertions so the documented contract stays traceable.
8. When the implemented ticket intentionally changes a contract still described in an active spec, update that active spec text in the same pass unless a named follow-up ticket explicitly owns the spec drift.

#### Planner and AI proof

- Prove behavior at the strongest available layer, not a weaker downstream proxy.
- When adding start-failure aftermath before action instantiation, check whether the surrounding path normally abandons empty transactions. Preserve that contract.
- When a ticket claims cross-layer valuation agreement, check whether the shared scorer computes marginal value over the actor's current accessible stock.
- When a ticket changes action availability, include at least one proof through real affordance enumeration, not just direct action construction.
- For exact-bound planner-root candidates, do not treat target binding as the whole contract when operator legality depends on intermediate goal state.
- When making a goal family live, verify its ranking entry in `compute_motive` returns a nonzero motive. A stub `=> 0` ranking silently prevents goal selection. When the new goal shares a signal or motive helper with existing goals, verify the shared helper's filtering criteria match the new goal's expected state.
- When the operator can be contention-managed (`Harvest`, `Craft`, `Loot`, `Heal`, or similar), verify direct affordance admission and queue-action expansion. Check the affordance-filter layer explicitly so a newly live direct operator path cannot bypass queue/grant contention.

#### Staged scaffolding

When a ticket lands pure scaffolding ahead of downstream integration, wire immediate call sites or mark the temporary unused surface deliberately. Do not let staged work fail later CI clippy passes.

### 8. Close the loop on the ticket

If the user asked for full ticket completion, archive per [docs/archival-workflow.md](../../../docs/archival-workflow.md):
- Mark completion status accurately
- Add an `Outcome` section (what changed, how verified)
- Note approved partial completion; create follow-up tickets when required

If the user asked only for implementation or analysis, do not archive. Default assumption: unless the user explicitly asks to archive, treat the task as implementation-only.

For implementation-only completion:
- Set `Status: COMPLETED` on the active ticket once the required verification surface has passed.
- Append factual close-out notes: `## Outcome`, `## Verification Result`, and any explicit deviations.
- If the active ticket is short-form or pre-template, add only the minimum missing sections: `## Assumption Reassessment`, `## Outcome`, optional `## Deviations`, and `## Verification Result`.

Before finishing:
- Re-check `What to Change`, `Files to Touch`, `Verification Layers`, and `Test Plan` against the actual landed diff. Remove reassessment-only fallout that did not become real edits.
- If reassessment or verification changed the semantic contract, also re-check `Problem`, `Architecture Check`, and `Acceptance Criteria` so the ticket's narrative matches the landed behavior.
- Re-check inline code snippets, example signatures, or API sketches against the final landed shape.
- Re-check `Status`, `## Outcome`, and verification/command notes — they should reflect commands that actually passed, not the pre-reassessment plan.
- If formatting was required in a dirty worktree, check for formatter spillover and call it out explicitly.
- Report tracked-vs-untracked status for the active ticket and any follow-up tickets created during the session (see Section 1 for tracking awareness).
- After golden scenario metadata changes, refresh the generated golden inventory/docs (see Section 6, Golden test verification). Inspect the generated diff footprint and call out whether broader generated-file churn is expected inventory/index fallout or unexpected.

Minimal active-ticket close-out shape:

```markdown
## Outcome

Completed on YYYY-MM-DD.

- What changed
- Any bounded deviation from the original ticket wording

## Deviations

- Optional: semantic or scope correction accepted during reassessment/verification

## Verification Result

- Passed `<command 1>`
- Passed `<command 2>`
```

## Guardrails

- Name exact files, symbols, layers, and invariants for non-trivial claims.
- Treat tests, traces, event logs, and authoritative state as different proof surfaces.
- Architectural contradictions: solve or escalate with 1-3-1 (see Section 3, Escalation decision tree). Do not patch around them.

## Example Usage

```
/implement-ticket tickets/LEGACTTOO-009*
/implement-ticket tickets/FITLSEC7RULGAP-001*
/implement-ticket .claude/worktrees/my-feature/tickets/FOO-003*
```
