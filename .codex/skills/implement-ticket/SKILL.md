---
name: implement-ticket
description: "Implement or reassess a Worldwake ticket. Use when asked to work from a ticket in `tickets/`, `archive/tickets/`, or a worktree ticket path: read the ticket, validate its assumptions against the live codebase and repo rules, correct mismatches before coding when needed, then implement and verify the requested deliverables."
---

# Worldwake Ticket Implementation

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), the target ticket, and any ticket-linked specs or docs before editing code. For planner-root, snapshot-completeness, or planner-traceability work, also read [docs/planner-contracts.md](../../../docs/planner-contracts.md) before finalizing the reassessment. Reassess first, then implement — do not treat a ticket as mechanically executable until its assumptions match the current codebase.

## Workflow

### 1. Load the ticket context

1. Read the target ticket file.
2. Read every directly relevant reference (specs, docs, code symbols, test files).
3. When the user supplies a glob or shorthand, confirm the exact live file path before reading or relying on it.
4. Check whether the active ticket file is tracked or untracked in the current worktree before planning reassessment notes or close-out edits. Untracked ticket drafts are valid active state, but remember they will not appear in ordinary `git diff` output.
5. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all operations.

### 2. Reassess assumptions before coding

Verify the ticket against the current codebase, not stale architectural memory. Check `Deps` — confirm each dependency is present on the current branch. For mixed-layer, planner, golden, or authoritative-validation work, name the exact symbols and boundaries under audit.

For planner-root, snapshot-completeness, or planner-traceability tickets, cite the relevant live contract from [docs/planner-contracts.md](../../../docs/planner-contracts.md) during reassessment instead of reconstructing planner behavior from archived tickets, stale scenario prose, or local implementation fragments alone.

For trivial single-file additive tickets, scale the reassessment down deliberately: read the ticket, cited references, and owned symbol/file; confirm the dependency path is present; and run a narrow existence/fallout sweep for prior implementation or obvious constructor/usage fallout. Do not skip reassessment, but do not force the full matrix when the owned surface is genuinely small and local.

When the ticket was authored by `/spec-to-tickets` in the current session from a freshly reassessed spec, scale reassessment to a targeted sweep: confirm the ticket's owned types still exist at stated paths, check for exhaustive matchers on modified enums, verify trait bounds on any types used in new test code, check for manual struct literals of modified types (constructors, test helpers, `from_*_for_test` patterns) that would need updating for new fields, and before adding new test-only accessors or helpers, check whether existing test infrastructure (e.g., `ActualWorldState::from_world`, test harness methods) already provides the needed capability. Do not repeat the full reference validation already performed during spec reassessment.

When the ticket is an audit-then-fix (e.g., "verify path X, fix if needed"), treat the audit as reassessment. Record findings in the reassessment section. If a gap is confirmed, auto-correct `Engine Changes`, `What to Change`, and `Files to Touch` before coding. If no gap exists, close with a reassessment-only Outcome documenting the audit trail.

If focused traces, regression tests, or lower-layer proofs falsify the current implementation hypothesis after coding has already started, stop and reassess immediately. Restate the live boundary, update the ticket sections that define owned scope, remove stale partial edits from the disproved approach, and only then continue implementation. Do not carry a stale hypothesis forward just because code already exists.

#### Reference and baseline validation

- Referenced files, types, functions, modules, commands, and tests exist.
- When the ticket's owned surface is partially landed in the worktree, treat the live state as baseline; limit edits to the missing slice.
- Keep tracked-vs-untracked state in mind when reading diffs and close-out evidence: untracked ticket drafts and newly created files will not appear in ordinary `git diff` output.
- Cross-check `Deps` against `What to Change` for additive tickets that assume earlier slices landed.
- When roadmap summary, active spec, and live ticket disagree, compare all three and record which is authoritative.
- When the ticket extracts or reuses private helper logic, confirm exact crate/file ownership before finalizing the plan.
- Described architecture still matches live code.
- Stated coverage gaps are real and correctly classified.
- When reassessment changes the live root cause or owned surface, update all ticket sections that encode scope and proof, not just problem prose: `Engine Changes`, `Architecture Check`, `What to Change`, `Files to Touch`, `Acceptance Criteria`, and `Test Plan`.
- When replacing inline code with a delegation to data populated by a prior ticket, verify line-by-line that the prior ticket's data captures every branch of the original code. Staged migrations risk silent behavior loss if the data ticket was incomplete.

#### Golden-specific reassessment

- Claimed missing scenarios are not already covered by current `golden_*` suites or generated golden inventory/docs.
- Identify the strongest existing owning `golden_*` suite before accepting the ticket's proposed file list; reuse existing ownership surfaces instead of creating new golden files.
- When same-domain verification fails, first check the referenced spec and any active sibling tickets for an explicit owner of that fallout before touching tests or broadening scope. If the failure is already named as downstream-owned work, document it and keep the current ticket boundary honest unless the user expands scope.
- When a shared concept has both upstream producers and downstream consumers, compare their semantics directly. If the consumer already supports a broader shape, correct the ticket to own that parity fix.
- If a claimed divergence is proved at lower layers but not stably isolatable as a golden without scenario-distorting scaffolding, correct the ticket to the strongest honest golden contract and record which lower-layer proof remains authoritative.
- For golden communication or information-path tickets, verify separately what actually degrades: provenance, confidence, communication class, eligibility, ranking, or another distinct mechanism.
- When a ticket proposes extending an existing trace/debug carrier, verify the exact live coverage of that carrier before coding. If the current trace only covers one subpath, correct the ticket to either stay within that subpath or explicitly widen the trace surface as owned scope.
- Scan the referenced spec for explicitly anticipated golden fallout, timing-sensitive scenarios, or downstream validation tickets. Use that mapping when triaging new failures so spec-owned fallout is handed off deliberately instead of being rediscovered through ad hoc test edits.
- When the ticket adds candidate generation or goal model integration for a domain that already has golden coverage (e.g., Care, Combat, Expectation), run the existing golden suites for that domain as part of reassessment, before implementation begins. This catches cross-goal interference early — a new candidate emitter can cause goal-switching collisions with existing goal families for the same target entity.
- When a golden ticket proposes specific GoalKind pairs to exercise a contention, planning, or action-lifecycle invariant, verify that each goal's declared ops (in `goal_dispatch_decl.rs`) include the required PlannerOpKind. If the goal family lacks the required op, correct the ticket's domain before coding.
- When the ticket claims a specific scenario ID is free, verify by scanning all `golden_*.rs` files for that ID before accepting it. Update the ticket if the ID is already taken.
- For planner continuity or same-goal branch-stability bugs, triage in this order before choosing a fix shape: is the committed branch absent from candidate generation, removed by a snapshot/read filter, reordered behind interleaved goals, or rejected later by search/start validation? Fix the earliest concrete layer that loses the branch; do not jump straight to continuity memory or downstream replanning glue.

#### Shared type, serialization, and persisted-shape sweep

When shared types, serialized carriers, or persisted components change, sweep these surfaces:
- Serialized fixtures, bundled scenarios, schema examples, and RON/JSON/YAML test inputs
- Helper factories, sample builders, and full struct literals across workspace crates
- Test-only mirror structs and manual bincode/seeded deserialize helpers
- Error, trace, request, and report carriers that store embedded enums by value
- Save/load version boundaries and `SAVE_FORMAT_VERSION` gates
- Crate-root re-exports and downstream imports for new shared types
- When a new shared type is defined under a submodule, verify the actual public import path before patching downstream crates. Do not assume a crate-root re-export exists or will be added unless the live code already provides it.

Specific persisted-shape checks:
- Worldwake does not support legacy saves by default. When persisted shape changes, update the current save format and keep older versions rejected unless the user explicitly asks for compatibility work.
- When removing or reclassifying persisted fields, search for stale tests, helpers, or docs that still assume older save versions load successfully. Rewrite them to reflect current-format-only support.
- When adding persisted fields to an existing serialized component, make focused save tests populate those exact new fields with non-default values and assert them after roundtrip. Do not rely on broad equality proofs that can pass with the new fields left empty.
- When introducing new persisted components alongside a temporary legacy carrier, keep the runtime boundary honest within the live current format, but do not add older-save migration paths unless the user explicitly requests them.
- When a staged migration moves consumers off a legacy carrier but a later ticket owns removing it, classify remaining references by surface: production reads, test-only helpers, public re-exports, setup fixtures. Eliminate production reads within the current ticket's boundary without introducing legacy-save compatibility.

#### Helper, math, and default validation

- When behavior depends on helper math, scaling, or threshold arithmetic, inspect the exact live helper implementation and correct stale numeric prose.
- When the ticket proposes concrete default or profile values, compare against live fixtures, schema samples, and roundtrip examples.
- When migrating a shared field's type, verify whether that field carries more than one world meaning. If one scalar collapses distinct semantics, correct the ticket to split them.
- When a benchmark or profiling ticket introduces segmented telemetry, verify whether each configured segment is guaranteed to produce samples on the claimed proof surface. If a lawful run can yield zero samples for a segment, correct the ticket and output contract to allow an explicit empty-state result (for example `NA`) instead of forcing a fabricated numeric ratio or threshold.

#### Action and behavior domain checks

- When a ticket mixes action admission rules with periodic maintenance behavior, identify which layer already owns each invariant.
- When a system or maintenance ticket claims a new transition-specific event/log surface, verify first whether ordinary `WorldTxn` component-delta events are already the canonical carrier. Do not widen scope to a bespoke event payload unless the live codebase actually needs a new carrier.
- When attaching aftermath or evidence to an existing action family, verify whether the handler spans multiple custody, location, or target subcases. Narrow to the applicable subtype.
- When the ticket relies on passive perception of place-bound state, verify the place entity is observed through the same path as co-located entities. If not, correct the ticket to own the explicit current-place projection path.
- When the ticket changes contested access to a scarce affordance, decide explicitly whether the domain uses pure race resolution or lawful waiting via queue/grant/reservation. Surface contradictions with 1-3-1.
- When the ticket names S44 contention helpers, verify helper semantics match the live `ContentionPolicy` shape for that domain.
- When widening a shared callback or execution signature, search dependent crates for both production call paths and test-only handler registrations.
- When a shared execution or runtime context struct gains a field, search for manual struct literals across both production and test code, not only typed function signatures or handler registrations.

#### AI pipeline and affordance checks

- When affordance generation depends on self-authoritative profile reads, verify those prerequisites in both production code and test harnesses.
- When proving real affordance enumeration against co-located agents, items, or places, verify whether the affordance query also depends on the actor already believing those targets are present. If so, seed the corresponding belief/perception prerequisite in tests instead of assuming authoritative co-location alone will expose the affordance.
- When a ticket tries to gate one agent's affordance on another agent's private belief carriers (for example `ExpectationStore`, `LastSeenMemory`, or `ViolationMemory`), verify the read surface first. In `PerAgentBeliefView`-style boundaries these carriers may be self-only, so cross-agent checks may need to stay actor-local at affordance time and move the other-agent requirement to authoritative start/commit validation instead of pretending the actor can lawfully inspect it.
- When the ticket asks an existing query to distinguish new enum variants, verify the current read surface exposes enough information. If not, correct the ticket to include read-surface widening.
- When the ticket depends on UtilityProfile or disposition gating, verify the belief/read trait exposes that carrier. If the gate exists only on authoritative components, correct the ticket to include read-surface widening.
- When the ticket claims a goal family should become behaviorally selectable, check the full AI admission path: candidate generation, goal-policy suppression, ranking, selection. A variant emitted only under conditions a suppression rule blocks requires ticket correction.
- When a ticket audits threshold alignment between candidate emission and goal satisfaction, also inspect the matching hypothetical planner transition for that goal family. Record whether one step or repeated steps are supposed to clear the relevant band, and correct the ticket if `apply_planner_step` still models a different contract than runtime execution.
- When making a payload-override action live through the AI pipeline, compare planner-step revalidation against runtime request resolution. Correct admission-path mismatches before treating failures as golden-only.
- When adding a typed query alongside an existing boolean helper, verify boolean equivalence. If typed results can be present while the boolean stays false, correct invariants and proofs.
- When the ticket gates behavior on a typed right from a specific provenance source, verify whether right existence alone is lawful or the producing carrier is part of the contract.
- When a staged ticket introduces a shared enum before all variants are producible, distinguish "type surface lands now" from "variant becomes live now." Test reserved variants as absent.
- When adding a new shared enum variant ahead of integration tickets, sweep dependent exhaustive tables. Prefer bounded compile-safe inert branches over reusing older variant behavior.
- When adding a new shared enum variant, also check bounded non-owner exhaustive consumers such as ranking/policy code, failure handling, observation/runtime helpers, relay-selection or ordering helpers, renderers, and detail-formatting surfaces. If those consumers only need compile-safe inert handling and do not widen behavioral scope, absorb them into the ticket rather than treating them as a separate architecture change.
- When the ticket keeps an action family unified while widening to new entity kinds, inspect `TargetSpec`, affordance enumeration, authoritative validation, planner semantics, and payload validators.
- When extending a projected belief or derived state, check for parallel snapshot builders, event carriers, or projection helpers that also reconstruct that model.
- When a new world artifact becomes perceivable and the spec says discovery affects behavior, verify at least one lawful downstream consumer exists. Do not land decorative but causally inert snapshot fields.
- When the ticket says information should be "internalized," search for an existing belief lane or consumer before inventing a new belief substrate.
- When the ticket changes historical event content or view semantics, inspect renderers and detail views for reconstruction from live runtime state instead of stored event records.
- When making a new action handler's affordance enumeration live through the planner's search pipeline, verify that every `RuntimeBeliefView` method the handler calls is implemented on `PlanningState` (via `PlanningSnapshot`), not just on `PerAgentBeliefView`. The planning state's view defaults most trait methods to `None`; affordance enumeration that depends on actor-local carriers (`expectation_store`, `ask_witness_memory`, `last_seen_memory`) silently produces zero payloads if the snapshot doesn't include the carrier.

#### Registry and schema checks

- For component-registration tickets, check hardcoded schema inventories, sample `ComponentValue` enumerations, and manifest-style tests.
- When registering a new authoritative component, search for hand-maintained `ComponentKind` inventories and sample builders outside the registration macro.
- When a scenario ticket adds authoritative components to places, verify whether place entities are topology-owned and created before `World::new(topology)`. If so, land component assignment in the bootstrap `WorldTxn` phase rather than the topology builder.
- When renaming or replacing an authoritative identifier, search display strings, manifest inventories, serialized name surfaces, and identity-assertion tests.
- When adding or reordering a `SystemId`, verify separately whether runtime dispatch uses a dense ordinal (`SystemId::ALL`) vs. a distinct manifest (`SystemManifest::canonical()`). Update each independently.
- When introducing a new shared type alongside an existing model family, include crate-root re-exports and downstream imports in the sweep.

#### Trait surface checks

- When extending a narrow trait or read surface, check for forwarding macros, blanket impls, paired runtime traits, or generated surfaces. Distinguish the canonical consumer boundary from implementation-detail mirrors.
- If a concrete type receives the target trait through a forwarding macro, treat the owned implementation boundary as potentially spanning the source trait, any paired runtime trait, and the macro site itself rather than assuming the downstream consumer crate named in the ticket is the only edit surface.
- When widening a shared trait, choose the narrowest ownership/borrowing form that preserves the canonical consumer path while minimizing snapshot and test-double fallout.

#### Performance and allocation sweep

- When a ticket eliminates allocation on a hot path (e.g., replacing `format!` with a structured enum variant), verify all consumers of the changed return type: `.is_ok()`, `.unwrap_err()`, `.map_err()`, pattern matches, `Display`/`to_string()` formatting. Confirm no caller depends on the old allocation shape (e.g., string content matching).
- When adding a new enum variant to replace an allocating variant, include the exhaustive-match and downstream-mapping sweep from the "new enum variant" guidance in Section 5.9.
- When a performance ticket adds a boolean fast-path alongside an existing `Result`-returning function, verify both paths agree on the same inputs.
- When refactoring a function to accept pre-computed results by reference instead of re-computing internally, enumerate all call sites and verify each passes the correct pre-computed data. Confirm no call site depended on the function internally re-computing fresh state mid-sequence.
- When changing a trait method's return type from owned to borrowed (`T` → `&T`), identify test mocks that construct the return value on-the-fly — they cannot return references to temporaries. Refactor those mocks to pre-populate owned storage and return references into it, rather than reverting to an owned return type or using `Cow`.

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

Record each auto-correction: ticket says / live code has / correction applied / why safe.
Place these notes directly under the ticket's `Assumption Reassessment` section as numbered entries so later review can see what changed and why.

If the active ticket predates the current template and does not already have an `Assumption Reassessment` section, add one before recording reassessment notes. Do not force the full template onto a short active ticket unless the missing sections are needed to keep the live scope honest.

When a correction changes the real fallout surface, update every affected ticket section (`What to Change`, `Files to Touch`, `Verification Layers`, `Test Plan`), not just one list.

When reassessment narrows ownership or proves that a listed command belongs to downstream-owned fallout, reconcile the ticket's success criteria in the same pass. Update `Acceptance Criteria`, `Tests That Must Pass`, `Verification Layers`, and command expectations so the close-out contract matches the live owned surface instead of leaving stale must-pass commands behind.

When reassessment converts a ticket into a reassessment-only, doc-only, or no-production-change completion, remove leftover placeholder scaffolding from acceptance criteria, verification, test-plan, and command sections so the finished ticket does not mix resolved scope with pre-reassessment TODO text.

Treat a stale acceptance criterion, scenario assertion surface, or proof target as a low-risk auto-correction only when the live symbols and behavior make the narrower honest contract directionally unambiguous. In that case, record: ticket says / live contract has / correction applied / why safe, and update the acceptance text plus any affected proof-surface sections in the same pass.

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

When the user confirms a direction that changes architecture boundary, affected files, or proof surface, update `What to Change`, `Files to Touch`, `Verification Layers`, and `Test Plan` before coding.

### 4. Extract the implementation scope

Turn the ticket into a concrete task list from `What to Change`, `Acceptance Criteria`, and reassessment findings.

Separate:
- required in-scope work
- blocked work needing user direction
- explicit out-of-scope work

When the ticket inherits broader spec language, distinguish the end-state architecture claim from the narrower contract this ticket owns after reassessment. Do not keep proof surface broad just because the parent spec states the larger destination.

When the parent spec describes an eventual causal story but the current ticket only owns substrate or maintenance scaffolding, keep those separate explicitly. If the spec's end-state locality or behavior prose is broader than the live mechanism this ticket can honestly land, narrow the ticket to the current owned mechanism and name the deferred downstream behavior instead of over-claiming the present slice.

When a ticket bundles multiple deliverables and reassessment narrows the ticket to only one lawful slice, verify explicitly that every removed deliverable is still owned by an existing active ticket. If any removed deliverable no longer has a live owner, create the follow-up ticket before coding instead of leaving that work implicit.

If the ticket's requested invariant exposes a production contradiction, correct the scope first.

#### Golden scope narrowing

- Remove duplicate proof unless the new scenario proves a materially different contract.
- If a proposed invariant is real at lower layers but not stably exposable as a golden, narrow to the durable golden slice and preserve the lower-layer proof as authoritative.
- When a golden ticket mixes valid negative coverage gaps with an over-claimed positive proof, preserve the honest golden slice and correct the ticket to include any bounded production fix.
- When one golden ticket contains multiple scenarios, allow different proof depths per scenario (decision trace, action trace, authoritative state) rather than flattening to uniform assertion style.

#### Type-change scope

When shared types change, include the sweep surfaces from Section 2 ("Shared type, serialization, and migration sweep") in the task list. Additional scope guidance:
- Before editing, run a concrete constructor/shape sweep for the changed type across workspace crates (for example `rg -n 'BlockedIntent \{' crates`), then rerun the same sweep before final verification to confirm no live literal or helper was missed.
- For broad shared-struct shape changes, it is acceptable to land the shared type first and then use sequential `cargo build` / `cargo test` compile failures to enumerate the remaining fallout. Prefer this over guessing when the compiler can authoritatively surface every missing literal or helper site.
- Do not treat `cargo build --workspace` alone as exhaustive fallout enumeration for shared-shape changes. Test-only constructors, helper factories, and same-crate test modules can stay hidden until `--all-targets` compilation (for example CI-matching `cargo clippy --workspace --all-targets -- -D warnings`). If the shape change touches a widely reused struct or enum, include an all-targets verification pass in the fallout sweep before closing the ticket.
- When behavior moves between carriers, rewrite setup paths onto the new authoritative carrier rather than only deleting the stale field.
- When a constructor begins seeding defaults it previously omitted, reassess tests proving "missing component" behavior — prefer rewriting to the new constructor contract.
- When new components participate in persisted world state, expand save/load fixture builders so persistence tests actually serialize and deserialize the new components.

#### Component registration scope

Distinguish:
- the authoritative schema declaration
- live macro-expansion sites or generated API surfaces
- runtime code-generation sites requiring the bare type in scope
- test-only helper or manifest sites mirroring the component set

Do not assume every schema macro reference needs a new import — verify actual local type use.

#### Trait surface scope

- Verify whether the live architecture uses forwarding macros, blanket impls, or paired runtime traits. Correct the ticket if the implementation boundary is broader than the prose.
- When reassessment exposes multiple ownership shapes for a new API, decide the shape before broad implementation.
- When a widely used helper or wrapper appears to need a signature change, verify whether it is actually the live production boundary or mainly a test/unfiltered convenience surface. If only a narrower production entry point needs the new behavior, prefer widening that path and preserving the broader helper unchanged rather than mechanically churning all callers.

#### Staged work

- Temporary duplicated logic is acceptable only if a named follow-up ticket owns the caller-rewire or old-path removal. State this boundary explicitly.
- When a ticket describes itself as "pure additions," verify whether an internal helper refactor is needed. If so, correct `Engine Changes`, `Architecture Check`, and `Files to Touch`.

### 5. Implement with Worldwake discipline

1. Keep edits minimal and targeted.
2. Prefer existing abstraction boundaries over duplicating logic.
3. TDD for bug fixes: add test capturing the bug → confirm it fails → fix behavior.
4. Never adapt tests to preserve a bug.
5. No backward-compatibility shims, aliases, or dual paths.
6. Preserve critical invariants from [AGENTS.md](../../../AGENTS.md): belief-only planning, information locality, append-only event log, determinism, conservation, unique location.
7. When authoritative validation or affordance-surface behavior changes, verify the full AI pipeline per `Authoritative-To-AI Impact Rule` in [AGENTS.md](../../../AGENTS.md). If the change removes candidates earlier, update stale downstream expectations.
8. When widening an action into a new custody or state regime, audit related stored state carriers for stale markers.
9. When adding a new enum variant, search for exhaustive matches and state validators in dependent crates before broad verification. Also search for hardcoded array/vec inventories (`const ALL`, test-only `ALL_KEYS` arrays) and count assertions (`assert_eq!(keys.len(), N)`) that mirror the enum's variant set. These are not pattern matches and won't produce compiler errors — they silently become incomplete or fail at runtime. Also check runtime-reachable wildcard catch-all matches that panic on unexpected variants, such as the `(strategy, goal_kind) => unreachable!(...)` arm in `feasibility.rs`. When adding a new variant to a shared error enum, also sweep: `Display` impl, cross-crate error-mapping functions (e.g., `map_reservation_error` in `start_gate.rs`), variant-inventory tests (e.g., `each_variant_displays_non_empty`), and crate-root re-exports.
10. When a new variant is not supposed to be live yet, land explicit inert dispatch/policy/ranking branches. Do not prematurely wire real runtime behavior.
11. When adding, removing, or replacing an `EntityKind`, include kind-classification and lifecycle-routing helpers in the sweep.
12. When adding a field to a shared model, search for hand-written constructors and test literals across sibling modules, including same-crate test modules. When adding a field whose value differs per dispatch variant but the current code shares a single constant across multiple variants (or-patterned match arms), split the shared constant into per-variant constants. Update the match arms and any test inventories that reference the old shared constant name.
13. When turning a single-shot action into a staged lifecycle, prove each phase separately: start admission, intermediate evolution, commit conditions, abort aftermath.
14. When an action uses a profile-driven or expression-driven duration, make test helpers derive or tolerate the real completion window. Do not copy a nearby fixed-duration helper and assume the same tick cadence.
15. When splitting uniform behavior into variant-specific rules, rewrite existing compressed tests into per-case proofs.
16. When making a new planner-visible operator lawful, sweep the full planner contract: goal dispatch, relevant-op declarations, progress barriers, goal-model expectations, heuristic/guidance surfaces (`goal_relevant_places`, evidence-place fallback, travel-pruning inputs when relevant), search tests. Verify the `may_appear_mid_plan` / `is_progress_barrier` combination: with `may_appear_mid_plan=false`, the operator can ONLY appear as a terminal step (requires `terminal_kind` to return `Some` — typically via `is_progress_barrier` or goal satisfaction). With `may_appear_mid_plan=true`, it can appear anywhere in the plan. If a ticket says "mid-plan step" but the operator has `may_appear_mid_plan=false`, the ticket needs architectural correction.
17. When a planner goal must synthesize a runtime payload, verify the activation chain end to end: the goal carries enough identity to build the payload, root/current-place guidance makes the operator reachable, and terminal-step semantics treat the action as goal-satisfying rather than leaf-only.
18. When the first planner fix only makes an operator partially live, immediately re-check the rest of the same operator chain before declaring success: candidate shape, root synthesis, payload construction, terminal semantics, and the focused planner proof.
19. When one goal family spans multiple target subtypes, verify operator availability per subtype. Check for stale operators leaking across subtypes.
20. When a goal family ends in a place-sensitive terminal action, add focused coverage for both target satisfaction and return-to-terminal-place legality.
21. When a colocated leaf action becomes live, verify the colocated terminal case separately from travel-plus-leaf planning.
22. When adding a new candidate emitter for a domain that already has active goal families (e.g., Care domain has both TreatWounds and EscortToSafety), verify that the new goal does not cause goal-switching collisions with existing goals for the same target entity. Run the existing golden suites for that domain before writing the new golden test. A new candidate can silently win ranking and trigger a goal switch that starts a second action while the first is still running, producing `DuplicateActor` or similar runtime errors.
23. When a goal generates as a candidate with nonzero motive but is never selected, diagnose in this order: (a) verify `compute_motive` returns > 0, (b) verify `synthesized_root_candidate_targets` provides a root candidate for the terminal op, (c) verify `is_progress_barrier` identifies the terminal op, (d) verify `build_payload_override` succeeds for the goal's ops, (e) verify `estimate_duration` returns `Some` for the action's `DurationExpr`. Each of these is a silent skip in the search pipeline.
24. When adding a new `GoalKind` variant, use the compiler to surface exhaustive-match sites, but also sweep these runtime-reachable surfaces that use wildcards or arrays: `GoalDispatchKey` enum + `ALL` array + `from_goal_kind`, `goal_kind_discriminant` in ranking.rs, `feasibility.rs` strategy-goal match, `format_goal_kind` in display.rs, and any shared signal/motive helpers in ranking.rs. Consider using `cargo build --workspace` first to catch compiler errors, then grep for the closest existing sibling (e.g., `ReportMissing` for `ReportFound`) to find runtime-only sites.

### 6. Verify at the right boundary

Run the narrowest correct verification first, then broaden.

Typical order:
1. Focused test covering the changed behavior
2. Crate-level tests for the affected crate
3. Broader workspace validation if the change crosses boundaries

#### Verification mechanics

- When the change touches multiple proof surfaces in one crate, run each focused selector needed.
- If a canonical interface is realized through a forwarding layer, prove both the consumer-facing call and the forwarding path.
- Check that focused selectors actually match new/changed test names — thematic filters can miss sibling tests with different prefixes.
- Prefer separate `cargo test` invocations per selector over combining exact test names in one command.
- Run multiple `cargo test` or `cargo clippy` commands sequentially when they share the same build profile — lock contention makes parallel same-profile runs unreliable. Commands with different profiles (e.g., `cargo test` vs `cargo clippy`) can safely run in parallel.
- Treat focused selectors the same way: even narrow same-profile `cargo test` invocations should run one at a time, never through parallel tool wrappers.
- When a broad verification run dies by `SIGKILL` or another likely environment/resource kill after focused suites are already green, rerun the named interrupted/failing suite in isolation before deciding whether to repeat the full broad run. Record that distinction in the ticket outcome instead of treating the killed broad run as a semantic failure by default.
- When a broader or longer-running verification command is intentionally waived after user direction, do not silently omit it or imply it passed. Record the exact completed command set plus the waived command in the ticket `Outcome` / `Verification Result`, and state that the narrower proof surface was accepted by user choice.
- After changing code post-verification, rerun narrowest affected tests and any stale broader commands.
- When CI/clippy forces a signature reshape, sweep all call sites before the next verification pass.
- When CI/compile fallout follows a shared context-field change, sweep manual struct literals as well as direct function call sites before the next verification pass.
- When a migration reshapes a common API surface, expect lint fallout as well as compile fallout. Satisfy trait expectations like `Default` instead of suppressing lints.
- When long-running verification commands are in flight, reuse those sessions rather than spawning duplicates.
- When new registered actions or systems cause broad failures, triage for catalog-order drift, completeness assertions, and registry-expansion fallout before assuming the feature's runtime logic is broken.
- If a focused failing proof exposes a real production contradiction in a ticket currently marked test-only or `Engine Changes: None`, update the ticket sections that define scope (`Engine Changes`, `Architecture Check`, `What to Change`, `Files to Touch`, `Out of Scope`) before continuing. Do not leave the ticket describing "tests only" work once live code changes are required.
- When a ticket fixes a repeated pattern across multiple call sites, run a post-implementation pattern sweep (e.g., grep for the unfixed pattern) to confirm no sites were missed. Record the sweep result in the ticket Outcome.
- When a workspace-wide verification command fails on files outside the ticket's owned surface (e.g., untracked binaries, pre-existing lint failures), verify the failure is unrelated by running the same command scoped to the ticket's owned crates. Record the pre-existing failure and the scoped-pass result in the ticket Outcome. Do not fix unrelated failures as part of the ticket.
- When broader verification fails on a golden in the same domain, action family, or planner path as the ticket's owned behavior, do one contract-level triage pass before labeling it unrelated: check whether the failure is stale setup only, a lower-layer production contradiction, or a real AI-pipeline mismatch. Only treat it as unrelated after that bounded triage.

Use the repo-approved commands from [AGENTS.md](../../../AGENTS.md):

```bash
cargo test -p worldwake-core <test_name>
cargo test -p worldwake-core
cargo test -p worldwake-ai
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

#### Golden test verification

- If stronger behavior now reaches completion earlier than a test assumed, recalibrate test inputs instead of weakening the implementation.
- When broader verification surfaces a timing-sensitive golden whose contract still holds, recalibrate the fixture's timing budget or hold window before broadening production scope.
- When a golden assumes agents observe co-located facts at tick 0, verify the setup explicitly seeds those beliefs or perception prerequisites.
- When a golden uses external action requests for scripted setup, set that actor's `ControlSource` to `Human` or `None` — do not leave it on `Ai` and treat autonomous interference as evidence.
- When a golden proves durable learned-state aftermath, assert the semantic contract unless exact tick identity is the owned invariant.
- If focused implementation shows the corrected ticket still over-claims, narrow the ticket before final verification.
- When a valid architecture change makes a golden stale, update it to prove the new lawful contract rather than preserving outdated reasons.
- When a golden transport/delivery/claim chain is about durable aftermath, avoid over-specifying intermediate substeps unless that substep is the owned contract.
- When a golden still fails after lawful setup, reassess whether it exposed a missing lower-layer contract. Fix the production boundary first.
- If the architecture change invalidates the old invariant, rewrite the scenario and update its header/comments.
- When adding `// Scenario ...` metadata, keep `Setup`, `Proves`, and `Cross-system chain` entries in the generator-friendly live format used by current golden files. After regenerating docs, inspect the generated scenario-map prose for truncation or malformed wrapped fields before closing the ticket.
- When adding or renumbering `// Scenario N:` blocks, treat identifiers as repo-global. Pre-scan nearby or highest live IDs and resolve collisions.
- After scenario metadata changes, refresh the generated golden inventory/docs as part of the verification surface.
- When a golden test must isolate one goal from a competing goal family that shares the same observable input (e.g., EscortToSafety vs TreatWounds for wounded entities), use belief-source manipulation: seed beliefs via `PerceptionSource::Report` instead of `DirectObservation` to prevent candidate emission paths that gate on direct observation. This is a scenario isolation technique, not a production constraint.
- When a golden scenario depends on motive arithmetic driven by metabolism rates or profile values, estimate the crossover tick (when the competing need overtakes the initial driver) from the rate differential. Start with conservative values that produce the crossover well within the tick budget. If the first run misses the milestone, adjust rates rather than expanding the tick budget.
- When a golden scenario depends on a specific target subtype from a shared target surface such as `EntityAtActorPlaceAnyOf`, verify the full live selection path before relying on setup ordering: affordance enumeration, belief prerequisites, planner snapshot inclusion/filtering, any planner-side reordering, and authoritative validation. Treat `EntityId` order only as a lower-layer tie-breaker when the live path actually preserves that ordering end to end.

#### Migration verification checklist

For migrations moving config/profile state from driver-global to per-entity components:
1. Remove the driver/global field and constructor arguments
2. Move test/golden setup onto authoritative component writes for relevant entities
3. Update runtime/save-load mirrors and serialization helpers
4. Add harness helpers for per-entity profile injection when repeated setup would sprawl
5. Rerun both tests and CI-matching clippy after the API reshape

### 7. Close out the ticket honestly

After the owned implementation is fully verified:

1. Update the ticket's `Status` when repo policy and the current task imply the ticket is now complete. Do not mark it complete before the required verification surface has passed.
2. If reassessment, implementation, or broad verification exposed an adjacent but out-of-scope contradiction, cleanup, or architectural compromise, create or update a follow-up ticket immediately instead of leaving the dependency implicit.
3. Give each follow-up explicit `Deps` links to the implemented ticket and any still-pending sibling tickets or active specs it depends on.
4. Distinguish clearly between:
   - bugs fixed inside the current ticket
   - compromises accepted to finish the current ticket safely
   - remaining work that needs its own ticket
5. Do not silently broaden the current ticket during close-out just because the next fix is obvious. If the remaining work has its own architectural boundary, capture it as a follow-up ticket instead.
- Keep scenario prose aligned with updated assertions so the documented contract stays traceable.
- When the implemented ticket intentionally changes a contract still described in an active spec, update that active spec text in the same pass unless a named follow-up ticket explicitly owns the spec drift.

#### Planner and AI proof

- Prove behavior at the strongest available layer, not a weaker downstream proxy.
- When adding start-failure aftermath before action instantiation, check whether the surrounding path normally abandons empty transactions. Preserve that contract.
- When a ticket claims cross-layer valuation agreement, check whether the shared scorer computes marginal value over the actor's current accessible stock.
- When a ticket changes action availability, include at least one proof through real affordance enumeration, not just direct action construction.
- For exact-bound planner-root candidates, do not treat target binding as the whole contract when operator legality depends on intermediate goal state.
- When making a goal family live, verify its ranking entry in `compute_motive` returns a nonzero motive for the expected scenario inputs. A stub `=> 0` ranking silently prevents goal selection even when candidate generation and planner search are fully wired. When the new goal shares a signal or motive helper with existing goals (e.g., `expectation_response_signal`, `score_product`), verify the shared helper's filtering criteria match the new goal's expected state — a helper that checks `Overdue` will silently yield zero for a goal that fires on `Resolved`.

#### Staged scaffolding

When a ticket lands pure scaffolding ahead of downstream integration, wire immediate call sites or mark the temporary unused surface deliberately. Do not let staged work fail later CI clippy passes by accident.

### 8. Close the loop on the ticket

If the user asked for full ticket completion, archive per [docs/archival-workflow.md](../../../docs/archival-workflow.md):
- Mark completion status accurately
- Add an `Outcome` section (what changed, how verified)
- Note approved partial completion; create follow-up tickets when required

If the user asked only for implementation or analysis, do not archive. Keep factual completion details current for a later archival pass.
- For implementation-only completion, set `Status: COMPLETED` on the active ticket once the required verification surface has passed.
- Append factual close-out notes to the active ticket using the same minimal structure expected later at archival: `## Outcome`, verification results, and any explicit deviations from the original ticket wording or scope that were accepted during reassessment.
- If the active ticket is short-form or pre-template, add only the minimum missing sections needed to make reassessment and close-out traceable. The usual minimum is `## Assumption Reassessment`, `## Outcome`, optional `## Deviations`, and `## Verification Result`.

Default assumption: unless the user explicitly asks to archive, says "full ticket completion," or otherwise requests the archival workflow, treat the task as implementation-only and do not archive in this turn.

Before finishing:
- Re-check `What to Change`, `Files to Touch`, `Verification Layers`, and `Test Plan` against the actual landed diff. Remove reassessment-only fallout that did not become real edits.
- If reassessment or verification changed the semantic contract the ticket describes, also re-check `Problem`, `Architecture Check`, and `Acceptance Criteria` so the ticket's narrative matches the landed behavior rather than an earlier draft.
- Re-check inline code snippets, example signatures, or API sketches against the final landed shape.
- If formatting was required in a dirty worktree, check immediately for formatter spillover into already-modified files outside the ticket's owned surface and call that out explicitly in close-out repo-state notes.
- After golden scenario metadata changes, refresh the generated golden inventory/docs.

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
- Architectural contradictions: solve or escalate with 1-3-1 (see Section 3 decision tree). Do not patch around them.

## Example Usage

```
/implement-ticket tickets/LEGACTTOO-009*
/implement-ticket tickets/FITLSEC7RULGAP-001*
/implement-ticket .claude/worktrees/my-feature/tickets/FOO-003*
```
