---
name: implement-ticket
description: "Implement or reassess a Worldwake ticket. Use when asked to work from a ticket in `tickets/`, `archive/tickets/`, or a worktree ticket path: read the ticket, validate its assumptions against the live codebase and repo rules, correct mismatches before coding when needed, then implement and verify the requested deliverables."
---

# Worldwake Ticket Implementation

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), the target ticket, and any ticket-linked specs or docs before editing code. For planner-root, snapshot-completeness, or planner-traceability work, also read [docs/planner-contracts.md](../../../docs/planner-contracts.md) before finalizing the reassessment. If that doc does not cover the exact planner boundary under audit, record the gap and fall back to the landed implementation/spec/ticket chain instead of treating the contract as unknowable. Reassess first, then implement — do not treat a ticket as mechanically executable until its assumptions match the current codebase. Do not stop at intermediate reassessment or partial fallout; continue until the ticket is completed, fully verified, or blocked by a user decision that requires 1-3-1.

When several reassessment branches seem applicable at once, use this priority order before coding: prove the live branch first; if the drafted premise is false, rewrite the ticket/spec immediately; if the live branch still supports a narrower complete slice inside the ticket's stated domain, finish that slice and create/update a follow-up ticket for the disproved remainder; use 1-3-1 only when the remaining boundary change is architecturally ambiguous, materially expands scope, or needs a user decision. Follow the canonical mixed-outcome branch in `Mixed outcome: narrow fix landed, broader golden still false` below. `references/mismatch-handling.md` is supplemental guidance for that flow and does not override the explicit mixed-outcome and narrowing rules in this skill.

An implement-ticket invocation authorizes narrow truthing edits to the active ticket and directly cited active spec files when those edits are required to make the implemented seam, acceptance criteria, or verification record truthful. It does not authorize broader repo-policy, process, or architectural-doc rewrites; for those, propose the doc update or use 1-3-1 when the decision is blocking.

## Top Rules

- Reassess against the live branch first. Do not implement the draft literally until the ticket/spec matches current code.
- If a narrow production fix lands but the drafted broader golden/E2E story is still false, follow `Mixed outcome: narrow fix landed, broader golden still false` below.
- Prefer the strongest existing honest proof seam. Extend an existing focused unit/runtime/golden test instead of creating the drafted new file mechanically.
- Keep Cargo sequential, confirm ambiguous or pre-existing exact selectors with `-- --list`, and record only truthful verification boundaries.
- Close out the ticket/spec with the real landed seam and deviations. Do not leave the correction only in conversation.
- Do not archive from `implement-ticket` alone; archive only when the user explicitly asks for archival or another invoked workflow owns it.

Keep this top-level skill as routing plus hard stops. Load the referenced files for detailed case law only when the ticket shape, reassessment, implementation, verification, or closeout step needs that detail.

Cargo hard stop: never use `multi_tool_use.parallel` for any Cargo invocation in this workflow. Cargo lock contention makes parallel `cargo test`, `cargo clippy`, and even `cargo test ... -- --list` probes unreliable here. Do not include a Cargo command inside any `multi_tool_use.parallel` bundle, even when the other bundled commands are read-only; run the Cargo command separately.

## Quick Routing

- Start by deciding whether the ticket is really implementation work, validation-suite work, golden/observer proof work, or shared-surface migration work.
- For shared enum/payload variant tickets, first classify the owner enum/payload, any sibling event/tag or re-export surface that must stay in lockstep, and the likely exhaustive consumer families (observer/CLI/report/render/test fixtures). Treat the migration as more than an owner-file edit even when the first patch lands in one crate.
- For new `Precondition` variants, compare each consumer's read boundary separately before coding: semantic declaration/inventory, authoritative validation (`World`/`WorldTxn`), affordance/runtime belief evaluation (`RuntimeBeliefView` and its concrete trait owner), target-index/implication helpers, and serialization or representative inventory tests. If one consumer cannot read the needed state through its current boundary, identify the narrow trait/helper owner and update the ticket before patching.
- For documentation-only roadmap/report tickets, first rank the authoritative live sources before writing prose: generated companions and live code/generator rules first, authored scenarios/tests next, draft design docs last when they conflict with live evidence. If the draft collapses structural activation, behavioral proof, and broader “landed” status into one claim, rewrite the ticket/doc boundary to those separate layers before editing.
- For roadmap-owned scenario landing tickets, treat the full landing contract as one owned closeout seam: authored scenario, backing golden, generated companions, workflow ownership, and roadmap/editorial status must agree before the row is marked landed. Do not stop at the scenario/golden diff if the live row or CI matrix still understates the landed seam.
- For ordinary implementation tickets that land or extend behavior for an existing `docs/scenario-roadmap.md` row, borrow the `scenario-roadmap-landing` truth-sync standard for docs/generated/roadmap consistency without switching to the full roadmap-landing workflow. Scenario, golden, generated companions, and roadmap prose must not tell conflicting stories about the landed behavior.
- For tooling/report/generator tickets, first confirm the canonical read-only input boundary and whether every claimed output row or classification is actually derivable from that boundary. If the draft asks the tool to infer runtime-only or later-stage facts from a narrower authored schema, rewrite the ticket to the honest schema-owned seam before coding.
- For UI, visualizer, or interactive tooling tickets, treat crate-local README/manual-QA text as part of the live handoff surface even on the small/local fast path. If the ticket makes a visible shell, control, modal, tab, or staged surface live, sweep nearby user-facing docs before closeout and remove or update stale placeholder/future-tense/manual-QA wording.
- For golden/observer proof tickets, first decide whether the mismatch is renderer/fixture drift or a real upstream event/report contract regression. Prove the upstream contract at the strongest existing lower-layer owning seam before editing any fixture, and if that contract is still honest, narrow the ticket to fixture truthing plus closeout.
- For validation-suite / tests-only tickets, diff the drafted `Files to Touch`, `New/Modified Tests`, and `Acceptance Criteria` against the live branch immediately; record `already landed`, `still live`, and `no-change cited files` before planning new tests.

## Always First

- Resolve the exact live ticket path and any cited spec/reference path before relying on draft wording.
- If the ticket family is already partially landed on the live branch, identify the remaining live delta before adopting the drafted `What to Change`, `Files to Touch`, or proof plan sections.
- If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all operations.

Apply the Cargo hard stop above throughout the workflow. After a resume or context compaction, check for and poll any known in-flight Cargo session before rerunning the same command. If the saved process handle is unavailable, cannot be polled, or no longer proves a completed exit status, treat the prior run as untrusted: rerun the command and record only the observed completed run as verification.
When the ticket's proof plan names a specific script, bin target, workflow entry, or test file, verify the exact live entrypoint name before treating the drafted command as authoritative. If the ticket spells the target differently from the live repo, correct the proof lane to the honest current entrypoint before closeout instead of preserving the stale command.
When the proof lane depends on a repo script such as `./scripts/verify.sh`, treat the script's live contents and observed output as authoritative over surrounding prose summaries. If the ticket's Test Plan names the script, inspect it during context loading or reassessment before coding so repo-specific gates can shape naming, cleanup, and implementation constraints. Inspect or summarize the actual gate set before closeout, especially when the script has gained extra checks beyond the documented high-level command list. If the script exits successfully but its streamed output has no explicit final success marker, read the script or run a cheap `sed`/`rg` over it before recording the exact gate list.
If a ticket lists `./scripts/verify.sh` but the session is not preparing a PR, inspect the script and either run the wrapper or run all live gates it currently wraps directly. Record any substitution explicitly in ticket closeout, including which wrapper gate set was covered and that the wrapper itself was not run.
When the live gate set includes repo-specific cleanup, removal, or drift-detection scripts, inspect their searched symbols or invariants early enough that new code does not reintroduce banned names or stale surfaces. If a script exists to prove an old concept is gone, treat its patterns as naming constraints during implementation, not only as final verification fallout.
For Worldwake Rust tickets, default to `cargo fmt --all` after significant Rust edits so local formatting matches the repo's `verify.sh` gate. In dirty worktrees, immediately inspect `git status --short` after formatting and classify any unrelated formatter spillover before proceeding.
When a patch changes a serde/bincode-persisted enum, struct, or carrier, check `SAVE_FORMAT_VERSION` even if the ticket was not classified as serialization or save/load work. Save/load is a representation boundary; record any required version bump or intentional no-bump in closeout.

For long diagnostic implementations likely to hit interruption or context compaction, keep a compact checkpoint after major pivots: current hypothesis, temporary probes still present, commands already passed, and remaining verification gates. Use the active ticket when the notes are durable closeout facts; otherwise use a disposable local note or the conversation, and fold only final truths into ticket closeout.

## Mixed outcome: narrow fix landed, broader golden still false

When focused live proof confirms a real narrow production fix inside the ticket's domain but the drafted higher-level golden/E2E ending still does not hold afterward, load `references/mixed-outcome.md` and follow that playbook. In short: land the narrow owning seam, stop broad scenario churn, rewrite the active ticket/spec to the proved boundary, record the still-false premise, create or update the follow-up owner, clean up temporary probes, and close out with the split explicitly documented.

Do not use this branch when broader verification fails only because an existing golden assertion still encodes a stronger subclaim than the current ticket now truthfully owns. If the same scenario still proves the ticket's real integration seam, narrow that assertion in-scope, rerun the golden, and close out without manufacturing a follow-up ticket.

## Workflow

### 0. Classify ticket shape and pick the right path

Load `references/ticket-classification.md`.

Pick the closest ticket shape before Step 1, then follow the matching quick path or full workflow. At minimum classify whether the ticket is:
- small/local
- validation-suite / proof-gap
- privacy-hardening / access-fence
- schema-only / staged substrate
- shared field/type migration
- shared enum/payload variant migration
- planner-boundary / golden E2E
- canonical writer / allocator integrity
- all other full-workflow work

For tickets that clearly fit the small/local fast path, the fast-path instructions in `references/ticket-classification.md` are sufficient for the Step 2, Step 6, and Step 7 reference-loading requirements unless reassessment exposes ambiguity, mismatch, broader fallout, or verification uncertainty. In that case, load the specific deeper reference named by the workflow section that now applies. Do not force the full reassessment, verification, and closeout reference set for genuinely local work when the ticket, cited docs/specs, owned symbols, focused proof, affected crate tests, and required lint/script gates already cover the live boundary.

### 1. Load the ticket context

1. Read the target ticket file.
2. Read every directly relevant reference (specs, docs, code symbols, test files).
3. When the user supplies a glob, shorthand, or obvious near-match typo, confirm the exact live file path before reading or relying on it.
   If the invocation includes extra positional tokens or path-like hints after the ticket path, resolve the first live ticket path first, then treat the remaining tokens as optional reference hints unless they clearly resolve to another ticket or a worktree root.
4. When the ticket name implies a numbered family or the user cites a parent spec, search for sibling tickets in the same family and confirm whether adjacent missing substrate is already owned elsewhere before broadening or narrowing the current ticket.
   When the parent spec is broader than the active ticket and sibling tickets already own adjacent deliverables, treat that sibling decomposition as the live implementation boundary unless reassessment proves the split is obsolete.
   When that family already exists as dirty or untracked draft tickets/specs in the worktree and the user asked for implementation-only work, keep edits scoped to the active ticket unless broader family ticket/spec updates are required to keep the repo truthful.
   When live API reassessment corrects a code-like helper, function signature, event-emission shape, or payload-construction snippet in the active ticket, immediately search active sibling tickets in the same family for the same stale snippet and update only the code-like guidance whose current wording is now false.
   When the user cites an active `specs/SNN*` family or the ticket belongs to an active spec family, scan `specs/IMPLEMENTATION-ORDER.md` for the spec id and any deliverable names that reassessment changes, defers, or completes. Update only text that is now false about the live ticket/spec boundary.
5. Check whether the active ticket file is tracked or untracked in the current worktree. Untracked ticket drafts are valid active state, but they will not appear in ordinary `git diff` output. Keep this in mind during diffs, close-out evidence, and follow-up ticket reporting throughout the workflow. Apply the same caution to any new implementation files or directories you create: use `git status --short` or another explicit untracked-file listing so newly added source trees are not omitted from the landed-surface summary.
6. Snapshot the current worktree with `git status --short` before coding. Classify unrelated dirty paths immediately as pre-existing user or autogenerated state, and keep them out of ticket fallout unless the ticket truly touches them. Use this early classification later during closeout so unrelated paths are not misreported as ticket work.
7. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all operations.

### 2. Reassess assumptions before coding

Verify the ticket against the current codebase, not stale architectural memory. Check `Deps` — confirm each dependency is present on the current branch. If a dependency ticket has already been completed and archived, rewrite `Deps` to the live archived path instead of leaving a stale active-ticket reference. If the requested ticket depends on an active pending sibling ticket, stop before implementation and use 1-3-1 to confirm whether to implement the prerequisite first, narrow/reject the requested ticket, or pause. When the user approves the dependency-first path, treat both the prerequisite and requested ticket as active closeout surfaces: implement the prerequisite first, then the requested ticket, and update both ticket/spec boundaries truthfully before final verification. For mixed-layer, planner, golden, or authoritative-validation work, name the exact symbols and boundaries under audit.

When a ticket adds a new crate, binary, GUI shell, or other dependency-bearing tooling surface, treat drafted third-party dependency snippets as provisional until compiled on the live branch. Run an early focused compile/check after adding the manifest, and if crate feature constraints, build scripts, or current dependency APIs disprove the draft, update the ticket/spec dependency text before continuing instead of leaving the correction only in Cargo fallout.

When a ticket/spec touches `Permille` or another bounded numeric wrapper, validate cited examples, profile values, and sibling-ticket snippets against the live type bounds during reassessment. If active spec-family examples are out of range, correct the active spec/ticket family before coding instead of letting focused tests discover the invalid values later.

Load `references/reassessment-checks.md`. For planner-root, snapshot-completeness, planner-traceability, or AI pipeline work, also load `references/reassessment-planner-ai.md`. For golden E2E or observer-motivated tickets, also load `references/reassessment-golden.md` and `references/reassessment-golden-observer-report.md`.

For AI candidate-generation tickets whose acceptance criteria mention diagnostics, name the full visibility chain before coding: the local diagnostic carrier, any exported decision-trace carrier, the omission/reason taxonomy, and the focused test that proves the externally visible trace path. Do not treat a local diagnostic record as sufficient evidence when the ticket's proof surface is a public trace, report, or debugging view.

When a trace, report, or UI ticket names multiple carriers, columns, panes, or externally visible surfaces, prove each named surface independently before closeout. Avoid aggregate assertions such as "decision OR action exists" or membership-only checks when the acceptance criteria require distinct carriers to be populated or rendered.

When a ticket or spec claims that a value will appear in an evidence summary, trace record, diagnostic payload, or that existing infrastructure will "pick it up automatically", verify the exact live carrier type before coding. Confirm whether the carrier can represent the claimed field without lossy mapping; if not, rewrite the ticket/spec to the strongest honest seam, such as an anchor-driven or trait-read contract, before implementing the behavior.

#### Golden, observer, and report tickets

Load `references/reassessment-golden-observer-report.md` when a ticket's proof surface is a golden, observer fixture, generated report, roadmap summary, or other read-only companion. Keep the top-level workflow focused on routing and ownership; use that reference for the detailed scenario-isolation, fixture-truthing, and generated-report representability checks.

When the claimed proof target is an observer/CLI formatter or renderer that is private to a `src/bin/*` binary, do not replace renderer proof with a handwritten formatting surrogate while still claiming renderer coverage. Either drive the compiled binary, add a same-binary unit test if the formatter is test-reachable there, or rewrite the ticket/spec/golden wording to the strongest honest seam, such as proving the decision payload fields that the private renderer consumes rather than rendered output itself.

#### Mixed AI/runtime synthesis and trade arithmetic

When a mixed AI/runtime ticket can synthesize the same payload family or opening offer through both planner fallback and authoritative affordance/runtime code, compare those constructors up front and treat parity across those generation paths as current-ticket scope rather than fixing only one side by default.

When a trade or bundle ticket is motivated by one side of the payload arithmetic (for example a corrected opening offer or higher seller price), inspect the full live trade shape before writing new proofs or closeout language: offered quantity, requested quantity, and the actual committed transfer amount may narrow the truthful contract from “full lot” to “unit purchase” or another smaller lawful seam.

#### Canonical write ownership and authoritative sync

When the ticket is about canonical write ownership, ID allocation, counter integrity, or another “one lawful writer” contract, classify every cited write site before editing: `production writer`, `same-crate test helper`, `fixture/sample payload`, or `already inert`. Do not preserve a drafted production-scope claim if live reassessment shows some cited sites are only test setup or fixtures; correct the ticket first, then decide whether those helper sites still need migration as consistency fallout.

When a live system already prunes or clears an authoritative derived surface when it becomes invalid, check that same sync seam first for the lawful restoration path when the surface becomes valid again. Prefer extending that existing authoritative reconciliation boundary before moving ownership into AI actions, planner restaging, or golden-only scaffolding.

#### Shared struct / field additions

For additive shared struct/component field tickets, load `references/shared-field-migration.md`. Before the first focused proof, run one repo-wide pre-sweep for exact explicit literals (`Type {`), public constructors (`Type::new(`), helper builders, and destructuring patterns on the changed type. Classify those sites by ownership and construction style up front instead of letting compile fallout discover constructor or destructuring updates piecemeal.

When a ticket adds a new authoritative component through `ComponentTables` or the component schema macro, treat it as persisted shape unless live code proves otherwise. Check the save/load carrier and `SAVE_FORMAT_VERSION`, add non-default round-trip proof at the persisted boundary when the component is saved, and keep generated `ComponentKind` / inventory expectations in the same order as the live schema registration site.
For component-schema macro fallout, classify each expansion site before adding imports or hand edits. Some generated tables and delta/world helpers need bare component type imports at the expansion site, while transaction-selection macros may already use crate-qualified component paths. Let the live macro body and compiler output decide the no-change sites, and record any cited no-change macro site in closeout instead of adding noisy imports.

#### Event, trace, and payload carrier tickets

When a system ticket claims a new event-log, trace, or transition carrier, verify first whether the live canonical carrier is already ordinary `WorldTxn` event payload fields (`action_name`, tags, targets, visibility, witness data) before planning a new structured event path.

When a spec or ticket says a maintenance mutation, recovery step, or passive reconciliation is silent, distinguish "no domain-specific event tag" from "no causal record at all." Do not add domain-specific or generic event tags unless the live system contract already requires them; prefer ordinary hidden `WorldTxn` component deltas when that is the strongest honest append-only record.

When an event tag or payload should fire only for a runtime sub-branch, verify whether the action definition's static `causal_event_tags` are applied unconditionally by the scheduler before adding the tag there. Prefer handler-local `txn.add_tag(...)` / payload writes for conditional emissions, and add focused proof for both the emitting branch and the non-emitting branch so the event log does not claim a causal record that did not occur.

When a ticket drafts an event family, trace family, or other grouped forensic surface and reassessment shows only some variants have authoritative live provenance, do not treat that as all-or-nothing. Narrow the current ticket to the subset whose payload causes are already concrete at honest write seams, rewrite acceptance criteria and out-of-scope sections before coding, and create explicit follow-up tickets for the deferred variants instead of emitting inferred placeholders or quietly dropping them.

When the event or trace itself is still correct but only a subset of its payload enum, reason taxonomy, or subtype family is authoritatively provable on the live branch, do not discard the whole event as unsupported. Narrow the current ticket to that payload subset, rewrite acceptance criteria / invariants / spec references before coding, and create a follow-up ticket for the deferred payload variants instead of widening the live claim beyond what the runtime actually records.

When a ticket's event, trace, or report family is still conceptually correct but the current shared/core payload contract cannot represent the live authoritative causes without lossy mapping, treat widening that contract as current-ticket scope before transport or emission work. Rewrite the ticket's `Engine Changes`, `Files to Touch`, acceptance criteria, and proof plan first so the implementation lands as “widen the shared contract, then wire the live causes” rather than a silent schema expansion discovered mid-stream.

When a ticket adds a new authoritative event/trace/runtime carrier in a domain that already has an older memory, learning, ranking, or report substrate, audit those adjacent substrates separately before broadening scope. If the older substrate is narrower but still honest for its current consumers, record that intentional asymmetry in the ticket and leave it narrow rather than widening it automatically as incidental fallout from the new carrier.

#### Action definition metadata changes

When a ticket changes `ActionDef` metadata rather than only handler internals, classify the changed metadata as a shared contract before coding. For `DurationExpr`, `Interruptibility`, binding strictness, payload defaults, preconditions, targets, reservation requirements, visibility, or causal tags, sweep the corresponding planner duration contracts, belief-view duration resolution, affordance/candidate assumptions, conformance tests, action registry inventory tests, and event/tag assertions before closeout. Treat downstream AI or harness fallout as current-ticket scope when it follows directly from the changed action-definition contract.

When a ticket claims a wire-format, serialization, or round-trip invariant for a named inner record type, verify whether the canonical persisted seam on the live branch is actually an enclosing save/load carrier or state object. If the outer persisted seam is the real contract, prove the invariant through that harness and rewrite the ticket's implementation/proof wording before coding instead of forcing a lower-level test path that bypasses the actual persisted boundary.

#### Belief / planner snapshot tickets

For belief-barrier or snapshot-admission tickets, explicitly classify each planner-visible carrier under audit as `authoritative local`, `belief-backed remote`, `explicit evidence`, or `out of scope` before changing code, so remote omniscience can be removed without accidentally stripping lawful local visibility.

#### Rewrite rules when reassessment disproves the draft

For golden E2E tickets, explicitly decide whether reassessment disproved the ticket's invariant or only disproved the drafted proof seam. If the invariant still holds but live replanning, control flow, or timing removes the authored autonomous observation window, rewrite the ticket/spec to the strongest honest proof seam before coding instead of treating the whole contract as false.

If reassessment first shows the drafted carrier is wrong and then shows the scenario never reaches that failure boundary at all, do a second-pass rewrite from “wrong carrier” to “wrong branch”: rename the ticket/test to the first actually reachable live contract and update acceptance wording before implementing.

When reassessment or the first focused proof shows a mixed outcome — a narrow production bug inside the ticket's stated domain is real and fixable, but the drafted higher-level golden/E2E premise still does not hold afterward — keep those two conclusions separate and follow `Mixed outcome: narrow fix landed, broader golden still false` above.

When a first-pass ticket/spec rewrite itself later proves too strong under focused live evidence, do not treat that rewritten boundary as sacred. If the live branch still supports a narrower complete slice that honestly resolves the real contradiction the ticket owns, rewrite the active ticket/spec again to that narrower boundary and finish it rather than forcing the stronger rewrite through or leaving the ticket artificially incomplete.

#### Bind abstract domain language to live carriers

When a ticket or spec uses generic domain language such as “affordance presence”, “local support”, or “relevant local state”, bind that phrase to the exact live carrier before coding. If the branch uses concrete place tags, workstation markers, item lots, resource sources, or another existing convention rather than a dedicated helper/type named in the prose, record that narrowing and implement against the live carrier instead of inventing a new abstraction by default.

When a theft, custody, or facility-stock ticket is motivated by “missing” or “unavailable” local goods, classify the contradiction before coding as one of: absent from place, absent from lawful control path, or absent from authoritative stock path. If the lot is still co-located and locally observable but no longer lawfully controllable by the owner, do not force the ticket back into a pure place-absence story; reassess the owner boundary across violation detection, investigation, accusation, and scenario authoring first.

When a ticket or spec includes a function sketch that reads through `GoalBeliefView` or another actor-facing belief trait, verify whether any required methods are actor-scoped before accepting an actor-free signature. If the live trait requires an explicit actor or another carrier the sketch omitted, correct the ticket/spec and helper signature before implementation instead of smuggling the missing context in later.

When adding actor-scoped belief-view accessors for physical state, separate stable authored profiles from dynamic current state. Stable authored profiles may follow a broader known-entity precedent only when live code proves that contract. Dynamic state such as dirtiness, fullness, stock, occupancy, or current capacity must default for remote or merely known targets unless the live belief store has an explicit propagated carrier; a known remote entity is not an authoritative current-state read.

When adding a method to a trait or read surface, search sibling traits in the same consumer bound set for the same method name, even when the arity differs. If another lawful provider exists, sweep dot-call sites and use explicit trait qualification at sites that must continue to call the older provider.

When a ticket is primarily about categorization, slotting, family membership, or another classifier boundary, compare the drafted category members and exclusions against the live grouping surfaces already used by ranking, policy, dispatch, suppression, or other same-domain classifiers. If the ticket/spec omits a currently live member, includes a stale one, or splits a family differently from those surfaces, correct the ticket before coding instead of hard-coding a new local taxonomy by default.

When a ticket maps a parameterized aspect, tag, enum payload, or claim family to a need, motive, actionability class, or opportunity class, validate the parameter against the live semantic owner before coding. For example, `ResourceAvailable(commodity)` should be checked against the commodity's concrete profile/effect helpers before treating it as relevant to hunger, thirst, medicine, fuel, or another need. Add or update a negative focused proof for a value that carries the same broad aspect but does not satisfy the current need or class, so the new predicate does not silently become "any member of this aspect family".

When a ticket introduces a local ordering, ranking, slot-selection, or weighted-score rule, explicitly check whether a stronger live global priority contract already gates that branch (for example `GoalPriorityClass`, priority bands, interrupt policy, or another higher-tier carrier). If the drafted local ordering would bypass or weaken that existing contract, rewrite the ticket/spec before coding instead of letting the new score path silently outrank the real global priority boundary.

#### Shared renames, visibility, and grep-count claims

For shared renames of types, modules, or schema-generated accessor families, do not trust the drafted file list until you have run a repo-wide sweep for the old symbol/module/accessor names across all workspace crates. Treat downstream import sites, CLI/report consumers, golden harnesses, and other compile consumers as current-ticket fallout even when the ticket only named the defining crate plus first-order readers.

When a golden or observer ticket targets a CLI/report pipeline that currently exists only under `src/bin/*` with no callable public helper, treat the compiled binary itself as a potentially honest E2E seam during reassessment. In that case, prefer an integration test that drives `env!("CARGO_BIN_EXE_<name>")` with temp inputs/outputs over inventing a new public helper just to satisfy the draft.

When a report or observer ticket inserts a new section into output that already uses numbered headings or section-boundary extractors, check immediately whether the change is truly output-additive or whether it also renumbers later sections and shifts downstream snapshot boundaries. Treat that heading/boundary fallout as current-ticket scope, update existing fixtures/tests accordingly, and correct any stale “strictly additive” ticket wording before closeout.

When a ticket wires validation into `spawn_scenario`, `load_scenario_file`, or another shared authored-input/load boundary, sweep not only `scenarios/` but also any test fixtures, observer scenarios, or golden `.ron` files that pass through the same boundary. If broadened verification then fails on one of those same-domain fixture inputs, treat the minimal fixture fix or justified override as current-ticket fallout when the ticket's required test target depends on it.

When a golden belief/read-model ticket proves behavior through a live envelope, projection, or derived belief-view surface, verify that the chosen golden harness seeding helper populates the same substrate the production read actually uses. If the live read derives from claim/provenance storage but the helper only seeds summary snapshots or another narrower cache, patch the fixture to seed the lawful substrate directly before trusting the regression.

For Rust `compile_fail`, privacy, or accessor-fence tickets, verify the exact external symbol path and failure mode before trusting the drafted snippet. Confirm whether the doctest is compiled as an external crate, whether a referenced symbol is actually re-exported from the crate root or only reachable through a module path, and whether the snippet would fail open independently for each intended visibility leak rather than only when multiple symbols change together. If private-type semantics make a drafted field-access regression impossible to prove in the stated form, rewrite the ticket/spec to the strongest honest boundary before coding.

When a ticket demotes visibility on an existing field, function, helper, or type, do not trust a drafted “zero blast radius” claim without a live sweep. Re-check current readers/importers across the workspace, classify them as same-crate production, same-package tests, or sibling-crate consumers, and rewrite the ticket's reassessment or acceptance text immediately if the reader count or boundary claim has drifted.

When a ticket cites grep-based counts or “zero matches” claims in `Problem`, `Assumption Reassessment`, or `Verification Layers`, rerun the exact live grep before implementation and again during closeout if the claim is central to the boundary being changed. If the count has drifted, update the ticket immediately instead of preserving the stale fact and only correcting it in conversation.

When searching Markdown prose, ticket text, or generated docs for literal code spans, quote the shell pattern safely. Prefer single-quoted `rg` patterns, or escape Markdown backticks, so the shell does not treat the search text as command substitution.

#### Proof seam rebinding when the draft is impossible

When a ticket or spec names a focused proof seam, test, or registration-count check that does not actually exist on the live branch, stop and rebind the proof plan to the nearest honest live seam before coding or closeout. Prefer the current manifest/registration site, constructor or delta inventory, macro-expansion-backed compile surface, or existing round-trip/runtime harness that really owns the invariant, then rewrite the ticket's verification wording to match instead of preserving the stale proof name.

When a ticket requires a method or API behavior whose semantics are not actually representable from the drafted stored fields yet (for example retention/expiry behavior without expiry metadata, or a derived classifier without the needed source inputs), treat that as a reassessment mismatch. Update the ticket first, then land the narrowest honest placeholder semantics the live shape can support instead of inventing unsupported state just to satisfy the drafted method list.

When a ticket's requested function signature, helper parameters, or owned input surface cannot represent a distinction the drafted invariant relies on (for example self vs. other, local vs. remote, or per-anchor classification without the needed carrier), treat that as a reassessment mismatch immediately. Rewrite the ticket/spec to the strongest honest boundary before coding instead of smuggling in extra context, guessing hidden state, or silently broadening the implementation contract.

#### Helper reuse and new-file discipline

When a small staged helper/module ticket probes, classifies, or prefilters behavior that is already modeled elsewhere in the same domain, sweep the existing helper surfaces first (for example ranking, dispatch, blocker matching, affordance enumeration, or synthesized-target helpers). Prefer reusing the live helper contract or narrowing the ticket to that seam instead of re-deriving a parallel local interpretation inside the new file by default.

When a ticket drafts a new file or module and the live branch already has a same-name or clearly same-domain module elsewhere in the crate, stop before creating the file and classify the canonical ownership seam first. Do not follow the drafted file sketch mechanically when the live branch already centralizes that domain under a different module boundary.

When a ticket makes an existing module start consuming a trait method that already has a meaningful default implementation, sweep the local test doubles/stubs/mocks for that trait before trusting focused regressions. If the new proof depends on that method, verify the doubles override it honestly rather than silently inheriting the default `None`/empty/stale placeholder path and failing at the harness seam instead of the production seam.

When a ticket mostly adds a new file plus a small declaration/edit in a large existing file (for example `pub mod ...;`, enum registration, or a one-line export), treat the existing file as a fragile edit surface: confirm the expected declaration layout before patching, then immediately re-read the touched header/section after editing to confirm the file skeleton is still intact before moving on to verification.

When a small/local helper-extraction ticket asks to move existing inline logic into a helper, enumerate the exact live branches currently inlined before editing. If the ticket names an extra semantic branch that is not present in the inline code, rewrite the ticket before coding and mark that branch as future-owned instead of adding new behavior during a preservation refactor.

When a small/local UI, visualizer, or read-model test asserts rows or state derived from a scenario, verify whether the asserted state exists immediately after load or only after deterministic simulation advancement. If the fixture must advance first, encode that advancement in the focused test and record the temporal proof boundary in ticket closeout instead of treating a tick-0 absence as a missing implementation.

When a visualizer or read-only UI snapshot derives values through AI/belief helpers, enumerate the required lawful read context before coding: agent belief store, current tick, scheduler active actions, action definitions, recipe registry when the helper needs recipes, and any runtime trace or profile carrier that affects the derived value. If the live UI cannot supply a required context, either thread that read-only context into the snapshot path or rebind the ticket/proof to the strongest honest value; do not silently compute a plausible-looking substitute from unrelated fields.

#### Second-pass correction

If implementation or focused test setup later disproves a remaining ticket/spec subclaim that survived the initial reassessment, stop and do a second-pass correction before final verification. Update the active ticket/spec to the strongest honest live seam immediately instead of leaving that mismatch implicit until closeout.

### 3. Handle mismatches explicitly

Load `references/mismatch-handling.md` when reassessment exposes a contradiction, risky ticket/code divergence, or a user decision that requires 1-3-1.

If reassessment changes a shared API, type contract, schema shape, or cross-ticket assumption, update any still-active dependent tickets/spec references in the same family before implementing or closing out the current ticket. If the current ticket completes but remains active and a sibling's dependency reference is still truthful, no sibling edit is required; update siblings when their wording now falsely says the blocker is unresolved, when the dependency path changed through archival, or when the landed contract differs from the sibling assumption.
If that reassessment also disproves an archived sibling's forward-looking handoff, dependency note, or prior closeout claim, amend the archived record truthfully. Keep the edit narrow and factual: record that the later live reassessment corrected the handoff, without rewriting the archived ticket into the current ticket's full closeout.
When a second-pass ticket/spec rewrite materially narrows or rebinds the live architectural seam after earlier family sync already happened, re-check active sibling tickets and active specs again before closeout. Do not assume the first sync pass is still sufficient once the owned boundary changes a second time.
If a golden-driven ticket proves that the scenario contract itself is underspecified — for example, the scenario can pass or fail without proving the authored causal branch — escalate from local ticket reassessment to the owning golden-policy / scenario-roadmap docs and any live roadmap tickets instead of recording the gap only in ticket closeout.
When a golden reassessment renames a scenario test or changes its authored contract, sweep any tracked `docs/generated/golden-*` inventory, index, or detail artifacts that mirror that scenario and update both test-name references and scenario prose before closeout.
When those generated artifacts carry source-line references or other layout-sensitive metadata, rerun the generator after final formatting or comment cleanup as well, not only after semantic test edits, so the generated companion still points at the final live file layout.

If reassessment exposes a separate architectural concern that must be tracked but is not honestly owned by the current ticket, create or update a dedicated follow-up ticket before proceeding, and rewrite the active ticket so that concern is referenced explicitly as an external dependency or out-of-scope blocker rather than left implicit.
If a narrow production fix lands and a broader drafted golden/E2E ending is still false afterward, do not keep extending the current ticket in search of a synthetic passing end state; follow `Mixed outcome: narrow fix landed, broader golden still false` above.
If an investigation/disposition ticket concludes that the live contradiction is already owned by an existing sibling ticket, close the current ticket by recording that conclusion and updating the sibling ticket's scope/deps factually instead of creating a duplicate follow-up.
When the current ticket resolves a blocker that had previously been split out from an active sibling, immediately update that sibling ticket's `Deps`, verification contract, and blocker wording so it no longer reads as still blocked on the now-completed concern.
When that follow-up path requires creating a new ticket, read `tickets/README.md` and `tickets/_TEMPLATE.md` first and write the new ticket in full repo-ready form instead of treating it as an informal reassessment note.
When reassessment shows the blocker is a missing substrate already captured by an active draft spec, create or update a bounded implementation ticket from that spec immediately and rewrite the current ticket to depend on that implementation ticket instead of leaving the spec as an implicit blocker.
When repeated follow-up tickets in the same numbered family keep exposing the same missing contract, proof surface, or traceability substrate, stop and assess whether the remaining concern now belongs in a new spec or roadmap update rather than another local ticket.
When the current ticket lands truthfully and focused proof is green, but the remaining concern is broader architectural duplication, provenance drift, or another `FOUNDATIONS`-level contract gap rather than unfinished ticket fallout, do not reopen the current ticket artificially. Close the ticket to its real landed seam, record the architectural residue explicitly, and hand it off to the right owner: a follow-up ticket for bounded implementation work, a new `specs/*` draft for contract-level redesign, or `post-ticket-review` when the next step is architectural assessment rather than immediate implementation.
If a golden/observer ticket exposes a concrete contradiction in the exact read/report/proof surface it is asserting against, the current ticket may absorb the narrowest production fix needed to make that surface honest. When that happens, update the ticket's `Engine Changes`, `Files to Touch`, and closeout `Deviations` instead of preserving a stale “tests only” scope.
When a new golden probe or validator exposes a real production bug and the current ticket absorbs that narrowest fix, also add the strongest focused lower-layer regression you can at the exact creation/update seam so future regressions do not need the long-run golden to rediscover them.
If a ticket's manual regression step, `compile_fail` sketch, or claimed proof seam is impossible as written because of language/tooling semantics, rewrite the active ticket/spec to the strongest honest proof seam before final verification instead of preserving the stale step and only noting the mismatch in conversation.

### 4. Extract the implementation scope

Load `references/scope-extraction.md` when the owned edit surface, dependency boundary, or honest verification scope is not already clear from reassessment and ticket classification.

For derived forensic/report/read-model tickets, use the compact checklist in `references/scope-extraction.md`.

For small CLI/tooling tickets that touch a single binary or local helper surface, explicitly check whether the honest focused proof belongs beside the owned function/module (for example in `src/bin/*.rs` tests) rather than in a new integration-test binary. If the ticket sketches a new `tests/*.rs` file but the live seam is a local formatter/helper inside one binary, narrow the test placement to that seam and record the deviation in the ticket closeout.
If the live emitter, runtime mutation seam, and focused verification harness already live together in one existing file or module, prefer landing the change there over creating a new sibling file from the drafted sketch. Record that live seam explicitly in the ticket closeout whenever it wins over the drafted file layout.
When the live proof seam or owned implementation boundary differs from the ticket's drafted sketch, record the exact landed seam in the ticket closeout instead of preserving the drafted shape in prose. Small/local tickets often land through a private extracted helper or a narrower same-file formatter/test seam even when the draft described a more direct in-place edit.
Before implementation scope is final, compare `Verification Layers`, `Acceptance Criteria`, and drafted `New/Modified Tests` for one-to-one proof coverage. If the ticket names an invariant that requires its own focused proof but the drafted tests do not include it, correct the ticket/test plan up front instead of discovering the missing proof only during closeout.
When a small/local ticket creates a user-facing UI or visualizer shell, keep the staged shell operational and avoid visible in-app text that explains implementation staging, future tickets, or debugging internals. Put staging boundaries and deferred controls in ticket/spec closeout instead; the app surface should show only user-meaningful state and controls for the slice that actually landed.
When a small/local UI, visualizer, or tooling ticket makes a previously staged surface live, sweep the crate-local `README.md`, manual QA checklist, and nearby user-facing docs before closeout. Remove stale “empty shell”, “placeholder”, or future-tense wording that now conflicts with the implemented tool, and record any README/manual-QA refresh in the active ticket's landed file surface.

For doc-only or compile-time regression tickets, use the compact checklist in `references/scope-extraction.md`.

### 5. Implement with Worldwake discipline

Load `references/implementation-discipline.md` when reassessment shows a non-mechanical, shared-surface, or otherwise higher-risk change that needs extra guardrails. For straightforward small/local tickets whose reassessment already proved the edit path is mechanical, you may proceed without loading that reference.

For repo-wide struct-literal or constructor fallout caused by a shared field addition, target exact constructor matches first (for example `EventPayload {` rather than a looser nearby-field pattern), then immediately re-scan touched files for accidental edits in same-shaped blocks before moving on to verification. Treat this cleanup pass as part of the implementation step, not optional polish after tests fail.
When extending an existing helper with several tightly related new inputs, check the repo's CI-shaped clippy surface before letting the signature sprawl. Prefer a small context struct or similarly local bundling at the call boundary over growing the parameter list until `too_many_arguments` forces a late cleanup or lint allowance.
When a ticket needs extra local runtime or call-site context inside one subsystem, do not widen an established public helper or crate-visible API by reflex. If the broader surface is not part of the owned contract, prefer a narrower internal helper or local wrapper that threads the added context only through the truthful live seam, and record that narrowed shape in ticket closeout when the draft implied a wider API edit.
Before reusing an existing helper across AI/runtime belief surfaces, verify that the helper already lives on the same trait/view boundary as the owned seam. If the live helper is bound to `RuntimeBeliefView`, another runtime-only trait, or a different trait-object family than the planner-facing code under audit, prefer a small local adapter or a new seam-local helper over widening the runtime helper just to make the types line up.
When a new path needs the same classification plus side-effectful aftermath as an existing handler (for example discrepancy recording, blocker/event writes, or memory updates), prefer extracting a shared helper from the live path over copying the write logic or only widening a classifier's visibility. Reuse the authoritative aftermath seam directly so same-domain paths stay behaviorally aligned.
When a maintenance pass transfers conserved quantity from one source to multiple possible consumers, audit batch staleness before choosing the implementation shape. If precomputed `(consumer, source, next_state)` updates could overdraw a shared source after an earlier consumer commits, prefer apply-time rereads, an explicit per-source accumulator, or another deterministic source-allocation structure; then record the landed source/sink contract in ticket closeout.

### 6. Verify at the right boundary

Run the narrowest correct verification first, then broaden.

Load `references/verification.md`.

#### Cargo execution discipline

Resolve focused Cargo test selectors before using `--exact`, normally with `cargo test -p <crate> --lib -- --list` for crate unit tests. If a module or prefix matches multiple tests, either run it without `--exact` and confirm the intended nonzero test count, or run each concrete listed test separately with `--exact`. See `references/verification.md` for detailed selector patterns and examples.

When adding several new tests whose names do not share a prefix, do not use one filtered `-- --list` query as proof that the whole set exists. Use an unfiltered list plus a focused text search, or run separate list queries for every intended exact selector before relying on exact test runs or writing closeout verification.

If a drafted Cargo command names multiple positional test filters before `--` (for example `cargo test -p crate foo:: bar::`), treat the command as invalid draft syntax. Rebind it to valid focused proof by splitting it into separate Cargo invocations or replacing it with one exact/module-qualified selector, then update the ticket's command list and closeout instead of preserving the impossible command.

If repo guidance or the active verification contract requires `cargo fmt --all`, run it, then immediately inspect `git status --short` and classify any formatter spillover in already-dirty files as unrelated or adjacent fallout unless the current ticket truly owns those paths. Record that spillover explicitly in closeout rather than silently attributing the formatted files to the ticket.

When a tooling or CI-script ticket needs a deliberate negative probe and the owned contract is that a whole script fails at a new late-added gate, let the full script run and record the exact failing step/message instead of short-circuiting through a narrower surrogate. Pair that probe with cleanup-safe restoration so the worktree returns to its pre-probe state before closeout.

#### Focused vs. broadened scope isolation

If focused proof passes and broadened verification fails in an unrelated existing test/fixture family, isolate that failure immediately. Create or update the owning ticket, record the blocker in the current ticket closeout, and keep the current ticket scoped to the invariant that was actually proved instead of absorbing the unrelated fallout by default.

When broadened verification fails on a mechanical, semantics-preserving lint in code outside the ticket's behavioral owner, first confirm it is not a hidden contract change. If it is trivial verification hygiene such as a Clippy-suggested `map_or` rewrite, the current ticket may absorb the narrow cleanup so the CI-matching gate can pass; record the touched files and no-semantic-change nature in closeout, then rerun the full wrapper or equivalent live gate set after the lint cleanup. If the lint requires judgment, changes public API shape, or affects behavior, hand it off as unrelated fallout instead of absorbing it silently.

When broadened verification exposes intentionally staged unused private surface (for example new enum variants, helper entry points, or report fields that sibling tickets will start using later), the current ticket may absorb the narrowest local lint-safe annotation or cleanup needed to keep CI-matching verification green. Record that staged-state deviation explicitly in the ticket closeout instead of silently treating it as unrelated noise.

When a ticket or spec explicitly fixes a public or shared API shape that conflicts with CI-matching pedantic lint expectations (for example a small `Copy` type passed by reference), do not silently flip the signature just to satisfy clippy. Either correct the ticket/spec during reassessment if the contract is not actually important, or preserve the requested shape and apply the narrowest lint-safe allowance/adapter needed, then record that deviation explicitly in closeout.

When a golden/test-only ticket is green at its owned proof seam and existing lower-layer coverage still proves the production contract, a broader same-crate rerun may still expose an unrelated golden in another family. Isolate that failure, hand it off explicitly, and keep the current ticket closed as test-only rather than reopening production ownership by default.

#### Golden, observer, and report scope verification

When a golden or observer ticket relies on named integration-test binaries, explicitly check whether the cited command executes the intended authored scenario cases or only compiles the binary plus non-ignored helper tests. If the motivating long-run scenario cases are still `#[ignore]`, record that distinction honestly in the ticket closeout instead of implying those scenarios ran.

When a ticket drafts a shorthand golden-family command such as `golden_survival` or another family label rather than exact test binaries, resolve the real integration-test binary names on the live branch before finalizing verification. Check whether the scenario-asserting cases inside those binaries are `#[ignore]`; if so, replace the shorthand with the exact per-binary commands and include `-- --ignored` wherever that is the only truthful way to prove the owned scenario contract.

When an event-log or decision-history ticket owns one specific emitted payload rather than same-tick exclusivity, prove the presence and exact contents of that causal branch directly instead of asserting the entire event family count is `1` unless exclusivity is itself part of the authored contract. Same-tick sibling events are often lawful once the live harness seeds the full belief or observation preconditions.

When a migrated output surface is fed by multiple lawful producers (for example observation, candidate generation, and planning paths writing into one combined vector), confirm which producers the existing focused regression actually exercises before broadening assertions to the whole combined output. If the harness only seeds one producer, keep the proof scoped to that exercised producer or extend the harness explicitly before claiming the combined contract.

When a broadened golden starts looking truthful only over a longer window than the draft implied, explicitly classify the contract before closeout: immediate post-return behavior, action-lifecycle ordering, or eventual completion over a cooldown/retention horizon. Rewrite the ticket and golden assertions to that exact temporal boundary instead of leaving “completes” ambiguous.

When a need/drive golden or authored scenario requires retuning pressure weights, rates, thresholds, or escalation parameters, enumerate all lawful relief paths for the target pressure before editing profile numbers. Check whether another candidate already relieves the same need as a side effect, such as food relieving thirst, rest affecting multiple pressures, or a wash/relief action resetting exposure. Bind the scenario edit to the intended causal branch first, then retune authored state so the proof exercises that branch rather than hiding it behind a competing relief path.

#### AI runtime, planner, and golden verification

For detailed AI runtime, planner-boundary, hybrid golden, scenario-metadata, fixture-refresh, and generated-doc fallout guidance, use `references/verification.md` and the relevant reassessment references instead of expanding the case logic here. Keep the top-level decision simple:
- if the proof crosses planner or AI runtime boundaries, load `references/reassessment-planner-ai.md` and prove the earliest honest boundary before broad verification;
- if the proof is a golden, observer, or generated-report seam, load `references/reassessment-golden.md` and `references/reassessment-golden-observer-report.md`;
- if the live golden seam changes, update scenario metadata and regenerated companions as part of the owned verification/closeout surface.

#### Manual probes and temporary scaffolding

Use `references/verification.md` for cleanup-safe manual-probe details. Before broadened verification, remove temporary diagnostic tests, trace prints, probe assertions, and marker strings unless the ticket explicitly owns keeping that instrumentation; when marker names were used, run a targeted `rg` cleanup check and then rerun the narrowest affected proof.

#### Save-format and serialized-surface verification

When a shared serialized-surface ticket mentions both a save-format bump and `#[serde(default)]` or other defaulted new fields, decide explicitly whether older saves are meant to load at all before you finalize reassessment. If the repo's no-backwards-compat rule means the real contract is version rejection, rewrite the ticket/spec wording immediately instead of letting default-field language imply an unsupported migration path.

When a shared serialized-surface ticket widens a schema, payload, enum, or saveable record, perform one final save-format check after focused proof is green: reopen the live `SAVE_FORMAT_VERSION` constant and confirm the landed diff bumped it exactly once relative to the current worktree state, not merely relative to the earlier reassessment snapshot or ticket text.

### 7. Close out the ticket honestly

Load `references/closeout.md`.

If final edits happen after broad verification, classify the freshness boundary before reporting completion: code, generated artifact, scenario, or executable-proof edits require rerunning the relevant broad or targeted executable checks; non-generated ticket/spec Markdown edits normally require `git diff --check` plus any targeted stale-claim scans. Record which broad gates were pre-final-edit and which post-edit checks prove the final diff.
After final ticket/spec Markdown edits made after executable verification, run `git diff --check` or explicitly record why it was not run before reporting completion.

Do not set an active ticket's `Status` to `COMPLETED` until the required executable verification gates for that ticket have passed. If a status field must be touched earlier during a closeout rewrite, pair it immediately with a provisional verification note that names the remaining gates, and do not report completion until the final ticket text reflects the observed completed verification.

For documentation-only roadmap/report tickets, use this compact closeout checklist before marking completion:
- restate the authoritative-source order that won during reassessment (for example generated companion, live code/generator rule, authored scenario/test, then draft design doc)
- distinguish generated evidence from editorial intent in the landed doc
- classify any live warnings as intentional exclusions, auxiliary evidence, or explicit follow-up work rather than leaving them as unexplained noise
- verify section structure, top-level cross-links, and any claimed lockstep table/catalog against the live source they mirror
- when a scenario row or status changes, sweep same-document summary tables, status matrices, and section-anchor references so older `planned` / `in progress` prose does not survive above or below the rewritten owning section
- when an implementation ticket changes `scenarios/*.ron`, `golden_*.rs`, or generated golden docs in a way that adds, removes, renames, or materially re-scopes behavioral proof for a roadmap scenario, inspect `docs/scenario-roadmap.md` before closeout and update any feature catalog, status summary, ordered-row entry, row-section prose, and final-integration support note that now conflicts with the generated docs or live assertions
- record drafted-vs-live boundary corrections in the ticket `Outcome`/`Deviations`, especially when a draft “landed” claim narrowed into `auxiliary` or `planned`

Before closing out, re-read any ticket claims about optional rendering, disabled flags, or suppressed sections and confirm the landed behavior matches those claims exactly. If the ticket distinguishes between “render empty-state” and “omit entirely when disabled,” make sure at least one focused assertion proves that exact disabled-path contract before marking the ticket complete.
After the final green verification run, re-open the active ticket and compare its `Problem`, `What to Change`, `Acceptance Criteria`, `Invariants`, and `Test Plan` against the landed diff. If late implementation fallout changed the truthful seam, compatibility shape, downstream consumer, or proof boundary, correct the ticket before reporting completion rather than leaving the earlier rewritten wording to overclaim the result.
If those final closeout corrections are prose-only ticket/spec/archive edits after the broad gate is already green, run at least diff hygiene and targeted stale-claim scans over the touched Markdown before final reporting. Rerun the full code verification only when code, generated docs, scenarios, test fixtures, or executable proof surfaces changed after the broad gate.
If a late semantic precision check changes source code after a broad gate has already passed, treat the broad gate as stale: rerun the narrowest focused proof for the changed behavior, rerun the applicable broad gate or wrapper, then update the ticket verification record so the final source edit is covered by the final observed executable run.
Also compare final `git status --short` against the initial worktree snapshot. Classify any newly appearing unrelated paths as concurrent or unowned before final reporting, and do not attribute them to the ticket unless the landed diff or verification fallout actually touched that ownership boundary. If the implementation created untracked source files or directories, include them from `git status --short` in the landed-surface summary instead of relying on `git diff --stat`, which omits untracked files.
When you intentionally edit a ticket or active spec file that was already dirty before the session, describe only the exact semantic closeout or truthing edits you made. Do not imply ownership of the whole file diff, and keep unrelated pre-existing prose churn out of the landed-surface summary.
When a ticket/spec correction changes a family-wide assumption, run a targeted truth scan before closeout. Search the active ticket, active spec, sibling tickets, and relevant archived handoff records for the disproved phrase, invariant, version number, or proof claim, then update only the records whose current text is now false.
When a focused repro disproves the drafted golden premise, include a compact factual record in the ticket closeout: the exact focused command, the observed selected path or candidate absence, the corrected conclusion about why the premise failed on the live branch, and the follow-up owner if a successor ticket now owns the real gap.
If reassessment or verification narrowed the owned seam, introduced a private helper, or required narrow same-ticket lint cleanup to support staged infrastructure, say so explicitly in the ticket's `Deviations`/`Outcome` notes rather than leaving the drafted implementation sketch as the only recorded shape.
When a trade or bundle ticket lands after a broadened golden changed from false to true, confirm that the closeout names the truthful transfer contract precisely: whether the commit is immediate or eventual, and whether the trade completes as a unit purchase, partial-lot purchase, or full-lot transfer.
When a golden/observer ticket closes as fixture-only after upstream contract verification, record that shape explicitly in closeout: name the lower-layer proof that kept production ownership honest, list any suspected production/emitter files as `no-change cited files`, and state that the landed diff was fixture truthing rather than a code-path fix.
When a generated report, inventory, or companion artifact intentionally emits warning rows about live schema/catalog gaps and the generator's proof surface is otherwise green, record those warnings explicitly in closeout as truthful live output rather than treating them as implementation failure. Name the warning set, say why the generator is still correct, and leave follow-up ownership explicit if the warnings expose unmapped but in-scope authored fields.
When a ticket adds new authored scenario/schema fields, explicitly check whether `scenario_coverage` or another live feature catalog should map them before final closeout. If the field is intentionally still unmapped, record that generated warning as expected live output rather than silently leaving the new schema surface unexplained.
If broadened verification proves the landed contract differs from the drafted invariant or top-level ticket summary, update the ticket's header fields and any now-false acceptance or invariant wording as part of closeout instead of recording the difference only under `Deviations`.
If implementation added a focused proof that the drafted `Test Plan` omitted but the ticket's own invariants or verification layers required, update the ticket's `Acceptance Criteria`, `New/Modified Tests`, and command wording together during closeout so the final handoff reflects the real proof surface coherently.
When the ticket lands a truthful narrow fix but a drafted broader golden/E2E premise remains disproved, make that split explicit in closeout by following `Mixed outcome: narrow fix landed, broader golden still false` above.
When the ticket is complete but the session uncovered a broader architectural gap outside the ticket's honest owned seam, record the handoff target explicitly during closeout: use a follow-up ticket for bounded implementation residue, a new `specs/*` draft when the remaining gap is contract-level or repeated enough to need redesign, and `post-ticket-review` when the next truthful action is architectural review rather than immediate implementation. Do not leave that architectural residue only in conversation.

### 8. Close the loop on the ticket

Covered in `references/closeout.md` (Step 8 section).

## Guardrails

- Name exact files, symbols, layers, and invariants for non-trivial claims.
- Treat tests, traces, event logs, and authoritative state as different proof surfaces.
- Architectural contradictions: solve or escalate with 1-3-1 (see `references/mismatch-handling.md`, Escalation decision tree). Do not patch around them.
- For focused test commands, verify that the selector actually proves the owned surface. Never record a zero-test selector as verification; use `references/verification.md` for the detailed exact-selector rules.

## Example Usage

```
/implement-ticket tickets/LEGACTTOO-009*
/implement-ticket tickets/FITLSEC7RULGAP-001*
/implement-ticket .claude/worktrees/my-feature/tickets/FOO-003*
```
