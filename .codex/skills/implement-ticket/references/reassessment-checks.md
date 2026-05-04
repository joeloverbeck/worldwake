# Reassessment Checks

General reassessment validation checklists for Step 2 of the ticket implementation workflow. See also `reassessment-planner-ai.md` for planner/AI-specific checks and `reassessment-golden.md` for golden E2E and observer-report checks.

## Reference and baseline validation

- Referenced files, types, functions, modules, commands, and tests exist.
- When the ticket's owned surface is partially landed in the worktree, treat the live state as baseline; limit edits to the missing slice.
- For CLI/scenario tickets, verify that authored bootstrap data populates the same live runtime registries, catalogs, and canonical bootstrap state the ticket expects. Do not treat per-entity wiring as sufficient until the scenario/bootstrap path and the runtime path agree on the same source of truth.
- Cross-check `Deps` against `What to Change` for additive tickets that assume earlier slices landed.
- When the ticket belongs to a numbered family or references a parent spec with split follow-up tickets, scan sibling tickets in that family before coding. Confirm whether adjacent missing substrate is already owned elsewhere so the current ticket neither over-claims sibling work nor narrows away an unowned gap.
- For staged decomposition tickets, verify whether any temporary carrier or intermediate shape named in the ticket still exists on the current branch. If an earlier slice already removed it, narrow the ticket to the remaining live debt.
- When the drafted API implies behavior that the drafted stored fields cannot yet support, stop and prove that mismatch explicitly during reassessment. Do not invent retention state, identifiers, or side-channel carriers that the ticket/spec has not actually defined; either narrow the semantics to an honest placeholder or update the ticket to add the missing substrate first.
- When roadmap summary, active spec, and live ticket disagree, compare all three and record which is authoritative.
- When the ticket extracts or reuses private helper logic, confirm exact crate/file ownership before finalizing the plan.
- When focused runtime proof in a downstream crate needs preloaded authoritative component or world state, verify the lawful test-seeding path during reassessment. Prefer `WorldTxn`, public component setters, or existing harness helpers over private direct world mutation, and correct the ticket's proof seam before implementation if the drafted setup assumes inaccessible mut accessors.
- Described architecture still matches live code.
- Stated coverage gaps are real and correctly classified.
- When adding, extending, or newly reading a universal agent profile or other always-present bootstrap component, verify both the schema registration/read surface and the canonical default-seeding path (for example `World::create_agent()` plus any transaction/bootstrap delta tests). Do not treat registration alone, or a plausible "component may be absent" assumption, as satisfying the live universal-profile contract.
- When a canonical registry or catalog gains entries (for example recipe, action, component, or manifest registries), sweep for hardcoded `RecipeId`, `ActionDefId`, ordinal, or registration-order assumptions in tests and helpers. Prefer resolving by name unless the stable ordinal is itself the owned contract.
- For scenario/world-authoring tickets, state whether the same runtime fact is currently authored through more than one lawful path, which path is canonical after the change, and whether any duplicate authoring path remains intentionally supported or is deferred to a named follow-up ticket.
- When reassessment changes the live root cause or owned surface, apply the section-update rule (see `mismatch-handling.md`, "Affected section updates").
- When a ticket names campaign, harness, or telemetry metrics as proof obligations, verify the live output contract. Confirm the actual emitted keys, counters, and summary carrier instead of assuming the ticket's metric names are still current.
- When replacing inline code with a delegation to data populated by a prior ticket, verify line-by-line that the prior ticket's data captures every branch of the original code.
- When a ticket adds a pruning gate, prefilter, or early-return check in front of an existing helper that still decides the final lawful opportunities, compare the new gate predicate against the full live downstream helper contract before coding. Do not let the new front-door filter silently narrow branches the downstream helper would still lawfully admit (for example seller lots, corpse inventory, recipe-backed acquisition, or other non-obvious evidence families).

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

**Shared rename checklist (types, modules, generated accessors):**
- Run a repo-wide symbol sweep for the old type/module/accessor names across all workspace crates before trusting the ticket's file list.
- Include crate-root re-exports, `pub mod` declarations, and generated accessor families (for example `get_component_*`, `set_component_*`, `query_*`, `entities_with_*`) in the rename inventory.
- Sweep downstream compile consumers, not just first-order production readers: CLI handlers, inspect/report formatters, golden harnesses, integration tests, and same-domain systems crates.
- Treat compile-only fallout from those downstream import/accessor consumers as current-ticket scope for the rename, even when the drafted ticket only named the defining crate or macro expansion sites.
- After the first broad rename patch lands, prefer an early all-target compile-only pass such as `cargo test --workspace --no-run` to enumerate missed rename fallout before focused proof.

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
- If the named facade trait is blanket-implemented from narrower subtraits, verify which subtrait actually owns the default method body or read contract. In that pattern, the real implementation boundary may be the owning subtrait plus the blanket forwarding impl rather than the facade trait alone.
- When widening a shared trait, choose the narrowest ownership/borrowing form that preserves the canonical consumer path while minimizing snapshot and test-double fallout.
- When a trait/read-surface method enforces belief locality or another actor-relative rule, verify the signature carries the actor/agent identity needed to enforce that rule. If the drafted signature only carries the subject or target while the contract says "known to the agent", correct the signature and dependent ticket/spec snippets before coding.

**Trait extraction sweep (for trait-split tickets):**
- Run a workspace-wide fallout sweep before editing: search for the old impl boundary and any forwarding macros or trait-forwarding sites across all crates, tests, and golden helpers.
- Prefer an all-target compile-only pass (`cargo test --workspace --no-run`) immediately after the first broad patch and before full test execution (see also `scope-extraction.md`, Type-change scope for all-targets guidance).
- Before rewriting impl blocks or UFCS calls, write down the exact moved method set and the exact methods remaining on the old trait. Use that partition as the source of truth.
- When splitting methods onto a new trait that provides non-panicking defaults, verify each production implementor still overrides every behaviorally required method. Add focused proof for any moved method whose default could silently preserve compilation while changing behavior.
- After moving methods, sweep for: stale UFCS calls on the old trait, method-call sites requiring the new trait import, dot-call fallout (`view.moved_method(...)`), helper methods and test-local impl internals (`self.moved_method(...)`).
- When blanket impls introduce a second lawful provider for an existing method name, sweep for ambiguity fallout requiring explicit trait qualification.
- When a trait split touches large mock/adapter/test-stub impl blocks, prefer replacing the whole impl partition in one pass over patching methods incrementally.
- Include shared golden harnesses and golden test infrastructure in trait-split fallout sweeps.
- Prioritize broken production implementors over broad mock cleanup when the first compile wave points there.

## Performance and allocation sweep

- When eliminating allocation on a hot path (e.g., replacing `format!` with a structured enum variant), verify all consumers of the changed return type: `.is_ok()`, `.unwrap_err()`, `.map_err()`, pattern matches, `Display`/`to_string()`. Include the exhaustive-match sweep from `implementation-discipline.md`, Enum variant handling.
- When adding a boolean fast-path alongside an existing `Result`-returning function, verify both paths agree on the same inputs.
- When refactoring a function to accept pre-computed results by reference, enumerate all call sites and verify each passes the correct pre-computed data.
- When changing a trait method's return type from owned to borrowed (`T` -> `&T`), identify test mocks that construct the return value on-the-fly. Refactor those mocks to pre-populate owned storage and return references.

## Repo rules

- Ticket fidelity from [AGENTS.md](../../../../AGENTS.md)
- Foundational compliance from [docs/FOUNDATIONS.md](../../../../docs/FOUNDATIONS.md)
- Ticket structure from [tickets/_TEMPLATE.md](../../../../tickets/_TEMPLATE.md)
- When a documentation ticket edits repo policy surfaces, check sibling guidance files with overlapping authority (`AGENTS.md`, ticket-authoring docs).

## Reassessment bookkeeping

When reassessment materially changes the ticket, keep a compact drafted-vs-live record in the ticket while you work. A minimal pattern is:
- `Already landed`: substrate or dependency state that exists on the branch already
- `Still live`: the remaining owned delta
- `New fallout`: files/symbols discovered during reassessment or compile fallout
- `No-change cited files`: drafted files checked and confirmed not to require edits

When the ticket replaces a semantic transport path, helper, or other shared boundary (for example canonicalizing facts onto one storage lane), inventory the repo-wide live call sites of the old path during reassessment and classify them immediately as current-ticket scope, sibling-owned fallout, or explicitly deferred follow-up. Do not wait for late broadened verification to discover whether remaining callers are still part of the migration boundary.

When the ticket's core question is whether an existing invariant is intentional or an oversight, inspect the introducing commit and archived spec/ticket closeout early in reassessment before broadening implementation scope.

If a dependency ticket has not landed and the missing piece is only narrow local substrate required to make the current ticket's owned proof surface executable, you may absorb that substrate without stopping: create the minimal local helper/file scaffolding, keep sibling substantive coverage out of scope, and update the current ticket to record the dependency mismatch and absorbed boundary. If the missing dependency would require adopting the sibling ticket's substantive contract, stop and use 1-3-1 instead.

When existing lower-layer proof in the same domain already establishes the production contract under audit, cite that proof during reassessment and keep the current ticket scoped to the remaining golden/test-only delta rather than reopening production ownership without evidence.

Before trusting the ticket as executable, cross-check its internal sections for contradictions. Reconcile conflicts between `Problem`, `Engine Changes`, `What to Change`, `Files to Touch`, `Acceptance Criteria`, `Verification Layers`, and any explicit in/out-of-scope notes before coding. Treat `Out of Scope` and any explicit "no new variants", "tests only", or "Engine Changes: None" claims as first-class contradiction surfaces during reassessment. If those sections disagree about ownership, proof surface, or whether production changes are required, update the ticket first instead of carrying the contradiction into implementation.
When a ticket includes a drafted derivation order, classification snippet, or helper sketch, compare it against the ticket's own worked examples, expected anomaly text, and focused test expectations before coding. If the helper sketch and the ticket's examples imply different semantics, treat that as a current-ticket contradiction and correct the ticket first.

When a ticket needs a side effect from a read-only generation or analysis pass, check first whether the live code already has a deferred `pending_*` result carrier, read-phase accumulator, or equivalent side-effect handoff before inventing a new mutation path. If such a carrier already exists, prefer extending that seam and correct the ticket to the live read/write boundary before coding.

When the owned change stays inside a private module or other single-file local boundary and the focused proof also lives in that file's `#[cfg(test)]` module, verify whether local test doubles, default trait methods, or helper builders are masking the new state path. If honest proof requires widening that local harness so tests inject real values instead of inheriting defaults, keep that harness fallout inside the current ticket rather than treating it as out-of-scope noise.

When a ticket adds trace or report fields, verify which file owns the canonical human-readable formatter versus which files only contain sample builders, tests, or helper constructors before accepting the ticket's claimed render/output edit surface.

When the ticket includes a proposed function signature, helper sketch, or API snippet, verify that the live helper contract actually supports that shape. If the current branch requires an additional dependency, carrier, or argument to use the cited helper lawfully, correct the signature sketch and matching `What to Change` snippets during reassessment before implementation.

## Action/behavior tick-aware invariants

When a ticket adds a component or field that mirrors a live relation-derived invariant rather than a single setter call, explicitly name the predicate during reassessment and enumerate every mutation path that can make it become true or false. Do not stop at the first named setter in the ticket if the live invariant also changes through containment teardown, possession changes, transit/state toggles, archive preparation, or other relation-driven paths.

When a tick-aware invariant is supposed to stay authoritative through mutation helpers, also sweep direct `World`/lifecycle/preparation paths that can bypass `WorldTxn` or the obvious public wrapper. If any of those paths can lawfully create or clear the same state, either absorb them into the current ticket or stop and correct the ticket boundary before implementation.

## Cross-crate accessor, trait-surface, and API-surface tickets

For cross-crate accessor, trait-surface, or API-surface tickets, verify the real downstream caller-facing boundary before coding, not just the immediate trait or type named in the ticket. If live callers consume the data through a broader wrapper, supertrait, blanket impl, or facade surface, correct the ticket to that owned boundary before editing code.
When a ticket names a trait as the change surface, also verify whether that trait is the concrete implementation boundary, a forwarding/facade layer, or only a marker supertrait. Scope the ticket to the concrete runtime implementor plus any required forwarding layer before coding rather than treating the named trait as the only owned edit site by default.

## Additive shared struct and field additions

When a ticket adds a field to a shared struct/component that is serde-deserialized from scenarios, saves, or other explicit inputs, verify omitted-field compatibility during reassessment instead of assuming the struct's `Default` impl is sufficient. Also prove the positive authored-input path: if the ticket makes a new field live for scenario/save/authored input, plan focused proof that an explicit input value for that field parses successfully, not just that omission still defaults lawfully. Decide whether the ticket must own a field-level serde default, explicit input migration, or fixture/scenario updates before implementation.
When a ticket removes a live scenario/config/save field, also sweep authored-file comments, schema-drift notes, and nearby maintenance annotations in the same active files when the intended invariant is that no live references remain. Do not stop at behavioral cleanup if those files still describe the removed field as current schema.
When the owning crate needs a focused omitted-field serde proof and does not already have a suitable text serializer in dev-dependencies, prefer adding a dev-only dependency in that crate over moving the proof to a broader integration crate.
When a shared struct/component gains fields, do not wait for compile errors alone to discover fallout. During reassessment, run an early repo-wide sweep for explicit struct literals, helper constructors, scenario builders, fixtures, and test harness code that instantiate the type manually across all workspace crates, and treat those updates as current-ticket scope when the new field is part of the live contract.
When a later sibling ticket has already landed additional fields on the same shared struct/component, treat those live sibling-added fields as baseline immediately. Narrow the current ticket to the still-missing fields only, but preserve the already-landed sibling fields at every explicit literal, helper constructor, and fallback path you touch during current-ticket constructor fallout.

## Internal diagnostic and trace carriers

When a ticket adds internal diagnostic, trace, or metadata carriage, preserve existing public/external call signatures unless the ticket explicitly owns that API change; prefer an internal helper, wrapper, or traced variant for the new carrier rather than widening public fallout by default.
When a ticket adds a field to an internal diagnostic, trace, or metadata struct, sweep the full carriage chain before accepting a single-file scope: producer, internal conversion/wrapper layers, renderers or report surfaces, manual struct literals, and all-target test/CLI consumers. Treat those as part of the owned reassessment boundary even when the original ticket only names the producer file.

## Event, trace, and payload carrier tickets

When a system ticket claims a new event-log, trace, or transition carrier, verify first whether the live canonical carrier is already ordinary `WorldTxn` event payload fields (`action_name`, tags, targets, visibility, witness data) before planning a new structured event path.

When a spec or ticket says a maintenance mutation, recovery step, or passive reconciliation is silent, distinguish "no domain-specific event tag" from "no causal record at all." Do not add domain-specific or generic event tags unless the live system contract already requires them; prefer ordinary hidden `WorldTxn` component deltas when that is the strongest honest append-only record.

When an event tag or payload should fire only for a runtime sub-branch, verify whether the action definition's static `causal_event_tags` are applied unconditionally by the scheduler before adding the tag there. Prefer handler-local `txn.add_tag(...)` / payload writes for conditional emissions, and add focused proof for both the emitting branch and the non-emitting branch so the event log does not claim a causal record that did not occur.

When a ticket drafts an event family, trace family, or other grouped forensic surface and reassessment shows only some variants have authoritative live provenance, do not treat that as all-or-nothing. Narrow the current ticket to the subset whose payload causes are already concrete at honest write seams, rewrite acceptance criteria and out-of-scope sections before coding, and create explicit follow-up tickets for the deferred variants instead of emitting inferred placeholders or quietly dropping them.

When the event or trace itself is still correct but only a subset of its payload enum, reason taxonomy, or subtype family is authoritatively provable on the live branch, do not discard the whole event as unsupported. Narrow the current ticket to that payload subset, rewrite acceptance criteria / invariants / spec references before coding, and create a follow-up ticket for the deferred payload variants instead of widening the live claim beyond what the runtime actually records.

When a ticket's event, trace, or report family is still conceptually correct but the current shared/core payload contract cannot represent the live authoritative causes without lossy mapping, treat widening that contract as current-ticket scope before transport or emission work. Rewrite the ticket's `Engine Changes`, `Files to Touch`, acceptance criteria, and proof plan first so the implementation lands as "widen the shared contract, then wire the live causes" rather than a silent schema expansion discovered mid-stream.

When a ticket adds a new authoritative event/trace/runtime carrier in a domain that already has an older memory, learning, ranking, or report substrate, audit those adjacent substrates separately before broadening scope. If the older substrate is narrower but still honest for its current consumers, record that intentional asymmetry in the ticket and leave it narrow rather than widening it automatically as incidental fallout from the new carrier.

## Formula and Permille math

When a ticket or cited spec includes both a formula snippet and numeric reference values, tables, or worked examples, verify at least one representative sample during reassessment before coding. For `Permille`-driven accumulation or decay formulas in particular, also sanity-check at least one live default-profile example numerically rather than only comparing the prose and code shape; drafted `/ 1000` normalizations can otherwise silently quantize the intended effect to zero. If the formula and documented values disagree, treat that as a ticket/spec mismatch: correct the active ticket's snippet and reassessment notes immediately when the intended direction is unambiguous, rather than carrying the stale math into focused proof.

When a formula, ranking rule, trace predicate, or early return passes through floors, caps, clamps, `.max(1)`, saturating arithmetic, or similar bounded operations, verify whether "output unchanged" really means "no signal." If a real failure, wait, capacity, or other diagnostic axis can be hidden by a floor/clamp, express the no-op predicate in terms of the underlying signal axes and update the ticket/spec before coding.

## Helper semantic quantity and reverse helpers

When a ticket relies on an existing helper or accessor, verify not only that the symbol exists on the expected boundary but that its live implementation returns the intended semantic quantity for the concrete subject type under test. Do not trust plausible naming alone when helpers can be overloaded, entity-type-specific, or historically repurposed; if the live helper computes a different concept than the ticket assumes, correct the ticket to the lawful contract before editing code.
When a ticket proposes a reverse helper into an existing parallel gate or branch family, verify whether the live mapping is one-to-one or one-to-many before finalizing the helper shape. If the gate has separate lawful branches for multiple needs, commodities, or motives, preserve every live branch that the source value can satisfy instead of collapsing to a single "primary" match by default.
