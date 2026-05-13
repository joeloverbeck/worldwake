# Ticket Classification

How to classify a ticket's shape before running the full workflow, and which fast paths apply (Step 0).

Before running the full workflow, classify the ticket:

## Canonical writer / allocator integrity quick path

When the ticket's core invariant is that one store, helper, allocator, or writer must become the canonical owner of IDs, counters, or another serialized write contract:

1. Sweep every cited write site before coding and classify each as `production writer`, `same-crate test helper`, `fixture/sample payload`, or `already inert`.
2. Correct the ticket immediately if its drafted production-scope claim collapses test helpers or fixtures into live runtime ownership.
3. Land the owner-side helper or allocator first in the canonical crate/module, including stale-state repair when the persisted counter or serialized write contract can already drift on the live branch.
4. Migrate the real production writer next, then migrate any helper-only sites that should use the same path for consistency or proof honesty.
5. Add focused proof for fresh allocation/write advancement, stale-state repair if applicable, and serialization/round-trip preservation when the counter or write contract persists across saves or component state.
6. Close out the ticket with the truthful ownership split: which sites were production, which were helper-only, and which helper migrations landed as consistency fallout rather than as newly discovered runtime ownership.

## Privacy-hardening / access-fence

When the ticket primarily tightens an existing visibility boundary, adds `compile_fail` fences, or otherwise hardens an existing API/ordering/access contract without changing runtime semantics, classify it as `privacy-hardening / access-fence` during intake.

For this shape, verify the live import/read surface first:

1. Sweep current readers across the workspace.
2. Classify each reader as same-module, same-crate, same-package test, or sibling-crate consumer.
3. Confirm the exact external symbol path the negative proof is asserting against.
4. Add the narrowest structural regression only when the ticket's invariant is about a singleton definition rather than ordinary privacy alone.

## Validation-suite / proof-gap

When the ticket is primarily a validation-suite / tests-only reassessment ticket for a feature family that already landed, classify it as `validation-suite / proof-gap`.

For this shape, sweep the drafted `New/Modified Tests`, `Files to Touch`, and `Acceptance Criteria` against the live branch before reading the proof plan literally:

1. Mark which drafted proofs are already landed.
2. Identify the strongest remaining honest proof seam.
3. Prefer tightening that existing seam over creating duplicate coverage.
4. When the truthful replay/save-load seam lives in a different file than the draft claimed, rewrite the ticket to the real owner before coding.

For validation-suite tickets whose remaining gap is golden or observer coverage:

1. Reassess the live substrate first: confirm the production behavior already landed or identify the exact lower-layer production gap before adding a golden.
2. Search existing `golden_*.rs` suites and generated golden docs for the same invariant, named scenario, or cross-system chain.
3. Reuse or tighten the strongest existing owning golden when it already covers the same layer; create a new golden file only when no existing suite owns the domain cleanly.
4. Add or adjust the executable proof, keeping scripted/hybrid harness legs explicit when a purely autonomous golden cannot hold the live boundary stable.
5. Regenerate golden inventory/docs when scenario metadata changes, then review the full generated footprint rather than only the new scenario-detail page.
6. Close out the active ticket/spec with the real proof seam, any drafted-vs-live deviations, the generated-doc surface, and the exact focused plus broad verification commands that passed.

## Schema-only / staged substrate

When the ticket primarily introduces a shared schema, payload, enum, save-format bump, or other substrate that sibling tickets will populate or render later, classify it as `schema-only / staged substrate` during intake.

For this shape:

1. Reassess the carrier contract, derive/trait surface, constructor fallout, save/replay/version boundaries, and focused round-trip coverage first.
   When the substrate introduces a net-new shared type name, first run an exact workspace name sweep across `crates/` and classify every hit as `same boundary`, `private unrelated helper`, or `blocking collision`; truth-sync the active ticket when a same-name private helper already exists but does not own the shared boundary.
2. Expect existing emitters/builders to keep populating the new surface with `None`/empty/default values until a later sibling ticket wires runtime use.
3. Record the staged state explicitly in closeout instead of implying the new schema is already live.
4. If the staged ticket also claims a real action, authored template, or downstream runtime exercise path, identify at least one truthful production consumer that carries the new substrate into live runtime state and prove that seam with focused verification.
5. If the new substrate is intentionally still test-only or helper-only, say so explicitly in closeout instead of marking the runtime path as landed.

Compact preflight for component-schema or shared-substrate tickets:

1. Identify the authoritative registration site and every generated or hand-maintained inventory it feeds: schema entries, `ComponentKind`, `ComponentValue`, sample builders, manifest-style tests, query helpers, and crate-root or table imports.
2. Classify macro-expansion fallout before editing. Some generated tables and delta/world helpers need bare type imports at the expansion site; other macros already use crate-qualified paths. Let the live macro body or compiler output decide no-change sites.
3. Check every default/bootstrap path that should make the substrate present: `World::create_agent`, scenario spawn/bootstrap, transaction helpers, and exact create-entity/create-agent delta assertions. Verify whether expected delta ordering follows schema/macro projection order rather than nearby insertion-call order.
4. Decide the persisted-shape boundary up front. If the substrate enters saveable world or simulation state, check `SAVE_FORMAT_VERSION`, plan the focused save/load proof, and confirm the bump exactly once after final focused proof is green.
5. For read-surface substrate, identify whether the named facade trait is blanket-forwarded from a narrower subtrait or runtime view. Patch the real owner, forwarding impl, and focused test seam rather than only the drafted facade type.
6. Before closeout, record the staged state and any macro-generated fallout as landed surface, including files the draft did not name but broad verification proved were owned.

## Shared enum/payload variant migration

When the ticket primarily adds a new variant to an existing shared enum or payload family, classify it as `shared enum/payload variant migration` during intake.

For this shape:

1. Reassess the owner enum/payload, any paired event-tag/manifest surface, crate-root re-exports, exhaustive `match` consumers, render/report observers, and focused serialization or round-trip proof before coding.
2. For shared payload widening, use a compact first pass before coding: identify the owner type, persisted save/load carrier and required `SAVE_FORMAT_VERSION` decision, render/report/CLI consumers, Debug-derived snapshot fixtures, inventory arrays/count assertions, crate-root re-export namespace collisions, every existing derive or size-sensitive lint bound on the widened enum or carrier plus nested payload types needed to satisfy those bounds, and any active sibling ticket or spec handoff text whose baseline becomes false after the new payload shape lands.
3. When a semantic type name already exists through multiple module paths or crate-root re-exports, verify the real owner module and prefer that owner-module import in implementation and tests instead of trusting the shortest re-export path.
4. When a shared payload or tag conversion crosses crate-owned types, verify the dependency direction before accepting the drafted conversion site. Put the conversion in the crate that can lawfully name both types, and truth-sync active tickets/specs when the draft names a dependency-inverting crate.
5. Expect downstream CLI/report/observer fallout even when the drafted file list only names the defining crate.
6. Perform one explicit early consumer sweep before implementation scope is final: search the workspace for `Type::Variant`, owner-type names in exhaustive `match` blocks, and known observer/report formatter entrypoints.
7. Classify each hit as owner, compile-only fallout, semantic renderer/observer fallout, or no-change cited file.
8. Use that sweep to seed the real `Files to Touch` / closeout boundary before the first broadened verification run.
9. If the changed type is embedded inside persisted runtime state or another enclosing saveable carrier, treat the ticket as a save-shape change and verify the enclosing persisted seam and version policy up front.
10. Inspect semantic consumers, not only exhaustive matches. Sorting, filtering, goal emission, report routing, discrepancy classification, and similar consumers may need explicit exclusion or routing updates.
    When several old variants collapse into one new public variant, identify every consumer that previously used the old variant name as branch provenance. If the new public payload shape cannot disambiguate all old branches, keep the event/report payload on the drafted public contract and add or preserve an internal producer/runtime discriminator for the lost branch semantics.
11. If the staged runtime path depends on place, entity, claim, or other bound semantics carried through an existing runtime step/report record, inspect whether the current carrier stores that fact explicitly or only infers it from another field.
12. If the inferred path would make the live binding dishonest, treat the carrier-field fix and constructor fallout as current-ticket scope before closeout.
13. If the ticket lands a new read-only wrapper or view type, check trait/lint expectations for the full surface early, including iterator companion impls.
14. If the ticket broadly flips consumer signatures from a raw collection to a wrapper/view type, sweep nearby and same-crate `#[cfg(test)]` modules for raw fixtures such as `vec![...]`, arrays, `&[]`, and `std::slice::from_ref`.
15. If the ticket introduces a mirrored enum or record in one crate to represent a type owned by another crate, identify the nearest lawful proof seam that can see both sides and prove parity there.
16. During closeout, rewrite any stale drafted `Files to Touch`, `Out of Scope`, or "no consumer/rendering change" prose when the consumer sweep or broadened verification proves a downstream observer, report, fixture, save/load, or renderer update was required.
17. Before the final broad verification gate, scan the cited active spec and same-family active tickets for old variant names, old save-version baselines, and present-tense claims that the pre-migration shape is still current. Truth-sync only the wording that becomes false once the shared migration lands.

## Shared field/type migration quick path

When the ticket migrates a shared field, enum payload, or other cross-crate type surface:

1. Confirm the dependency chain first, and rewrite stale `Deps` entries to archived live paths when the prerequisite already landed.
2. Compare the drafted file list against the live branch before coding: separate real constructors/consumers from stale mentions, wildcard matches, partial literals, and already-landed substrate.
3. Land the first honest type patch in the defining file plus the first required producer/consumer sites.
4. Run `cargo test --workspace --no-run` early to enumerate the real fallout set.
5. Resolve focused test IDs with `cargo test -- --list`, then run the narrowest exact/module-qualified selectors that actually prove the changed boundary.
6. Before closeout, trim the ticket back to the files and commands that were actually exercised, and note drafted files that compiled unchanged.

## Small/local tickets (fast path)

Applies to single-file additive CLI/tooling/reporting/action-registry change, narrow helper extraction, formatting update, or other owned-module additive change with no shared type/planner/golden/persistence/cross-crate fallout expected. Typical examples include a single-file transport/action registration, local handler addition, narrow helper extraction, or bin-local coverage for factored logic:

1. Resolve the exact live ticket/spec path, including typos or shorthand.
2. Confirm the dependency path and the exact owned symbol/file boundary. If a dependency is already archived, rewrite `Deps` to the archived path before coding or closeout instead of leaving a stale active-ticket reference.
3. Run a narrow constructor/usage sweep for the changed shape: confirm the named symbols and accessors exist, search local callers/render sites, check obvious constructor or test-helper fallout, and identify the narrowest real proof entry point. When the ticket is about an absent-profile, missing-field, or other "gracefully skip when unset" contract, explicitly verify that nearby test doubles do not synthesize defaults that would mask the missing-state path.
   If the ticket changes local storage semantics from append-only to deduplication, replacement, compaction, or another in-place retention rule, also sweep same-carrier diff/apply/serialization helpers before editing, even when the ticket otherwise stays single-file and local.
   If honest local report, observer, or formatter output requires adding or reshaping an internal diagnostic, trace, report, runtime, event, or payload carrier, reclassify immediately to the relevant carrier path in `SKILL.md` and `references/reassessment-checks.md`; run the workspace-wide constructor/consumer sweep before source edits instead of waiting for broad compile or lint fallout.
   If the ticket or cited spec includes both a formula/helper sketch and numeric examples or expected values, compute at least one representative sample during reassessment. Correct stale arithmetic in the ticket/spec before coding when the sketch and examples disagree.
   For formatter, report-row, or CLI output changes, also search for existing rendered-output fixtures or snapshots before coding, using the section title, formatter helper name, or distinctive summary tokens. Treat same-domain fixtures under `tests/fixtures/`, observer/report integration tests, or checked markdown snapshots as likely final render surfaces rather than waiting for the affected crate suite to discover them.
4. Implement the owned change with focused proof first.
5. Run the affected crate's tests as the normal broadened proof for the ticket. For Rust tickets, run `cargo fmt --all` before final broad verification; do not pass the active ticket Markdown or other non-Rust files to `rustfmt` unless the ticket explicitly owns a Markdown-formatting step. If the ticket's Test Plan or repo norms call for CI-matching clippy, run `cargo clippy --workspace --all-targets -- -D warnings` as part of normal broadened verification; use compile/lint fallout to catch remaining shared-shape literals/helpers and local cleanup. That broadened lint surface can also lawfully expose adjacent same-crate `tests/`, bin, or report-helper fallout that the ticket should absorb when it is still part of making the owned verification target pass.
   If the broadened crate test pass exposes existing same-domain integration/golden tests that already assert the exact payload, selection, or emitted-shape contract of the changed behavior, treat that as current-ticket verification fallout rather than assuming it belongs to a later golden-only ticket. Update the ticket's owned scope, touched files, and closeout notes to reflect that lawful verification surface.
   If the ticket's Acceptance Criteria or Test Plan names additional verification commands beyond the affected crate's tests and CI-matching clippy, those commands remain required even on the fast path.
   Immediately after formatting, run `git status --short` and inspect whether formatting touched unrelated files in the dirty worktree before continuing to verification. If formatter spillover occurred, either narrow the formatter invocation further or record the spillover explicitly in closeout.
6. For small/local UI, visualizer, or interactive tooling tickets that make a visible shell, modal, tab, control, or staged surface live, sweep crate-local README/manual-QA docs before closeout. Remove or update stale placeholder, "empty shell", or future-tense wording that now conflicts with the implemented surface, and record any doc refresh or no-change result in the active ticket closeout.
7. Close out the ticket with the actual verification set and tracked-vs-untracked note. This normally includes updating the ticket file itself with completion metadata such as `Status`, `Outcome`, `Deviations` when needed, and `Verification Result`, not just reporting those details in the conversation.

If a ticket appears to qualify for both the small/local fast path and the shared additive checklist below, the shared additive checklist wins for reassessment and early compile fallout.

For CLI/tooling-only tickets, if the owned logic can be factored into local helpers, prefer bin-local `#[cfg(test)]` coverage over command-only validation. When the ticket's user-visible contract is report, dump, or formatter output, also add at least one focused assertion against the final render/output surface rather than proving only the helper-level computation. For new or changed tests inside `src/bin/*.rs`, resolve the bin-local target before exact proof: run `cargo test -p <crate> --bin <bin> -- --list`, copy the full listed id, then run that id with `--exact`. For authored scenario/config tickets, confirm that an existing focused test actually loads or spawns the exact file/input under change; if not, absorb a narrow integration/load-path proof instead of relying only on broad workspace commands.

When a ticket stays local to one CLI/tooling module but the live computation surface and the final render/output surface are different sections of that same file, name both local boundaries during reassessment and prefer a shared local helper over duplicating logic between sections.
When a small/local same-file behavior ticket lands in a selector or ranking function that hides the intermediate discovered set or filtered population you actually need to prove, it is acceptable to extract a private same-file helper so focused tests can assert that intermediate contract directly without widening the ticket's scope.

Do not skip reassessment for small tickets, but scale it down: read the ticket, cited references, and owned symbol/file; confirm the dependency path is present; run a narrow existence/fallout sweep for prior implementation or obvious constructor/usage fallout. Do not force the full Step 2 matrix when the owned surface is genuinely small and local.

For small/local tickets, the required read set is the ticket, its cited spec/docs, and the owned symbol/file boundary (including nearby tests or render sites that prove the contract). Load additional reference docs only if reassessment exposes ambiguity, mismatch, or broader fallout. The normal fast path is that required read set, focused proof, the affected crate's tests, and any explicitly required CI-matching lint surface.
For Rust small/local tickets, prefer `cargo fmt --all` so formatting matches the repo's `verify.sh` gate, then inspect for formatter spillover before continuing. If the workspace formatter is blocked by unrelated broken files, use the narrowest repo-aware formatter that can honestly format the owned Rust surface and record the substitution.

For straightforward shared-type additive tickets (new field on an existing struct/component, derive-safe enum payload addition, or similar constructor fallout with no boundary dispute yet visible), start with the ticket, cited spec/docs, `references/reassessment-checks.md`, `references/verification.md`, and `references/closeout.md`; for authored scenario/schema structs, also load `references/shared-field-migration.md` before implementation scope is final. During the constructor fallout sweep, distinguish full manual struct literals from partial literals that already inherit new fields via `..Default::default()` or equivalent helpers before accepting the ticket's cited file list as real edit scope. When the ticket or spec shows sample literals for a shared carrier, verify the live field list on the actual type before copying those examples into focused tests or fixtures; drafted examples often lag the landed struct shape even when the contract is otherwise correct. `Option` additions often narrow the real patch set further: many partial literals remain lawful unchanged once verification confirms the inherited default matches the live contract, so keep the edit scope centered on full literals, serde/default proofs, and authored-input surfaces rather than patching every partial literal by default. If the new field lands on a serialized root, directly persisted carrier such as `World` or `SimulationState`, or a component payload included in the save/load shape, explicitly verify the save-shape/version seam during reassessment: check whether current policy requires a format-version bump, and plan a focused non-default roundtrip proof rather than assuming broad workspace tests will make the persistence contract obvious. Include test-module import fallout when the new type only appears in fixtures, assertions, or sample builders. For CLI/scenario schema structs in particular (for example `AgentDef` or `PlaceDef`), also expect same-crate fallout in handler/display test scenario builders, lints, destructuring/report helpers, and other manual authored-input literals that must stay exhaustive after the new field lands; for `AgentDef`/`PlaceDef`-style additions, those exhaustive same-crate test literals and report/generator readers are often the primary fallout set rather than incidental cleanup. When the additive shape is a universal agent or place component or other always-seeded bootstrap state, also check the canonical default-seeding path (for example `World::create_agent()` or the scenario place-spawn loop plus any exact create-entity/create-agent delta assertions) before finalizing scope, and confirm whether those exact delta assertions follow schema/macro projection order rather than nearby insertion-call order. After the first shared field or type-surface landing, prefer an early `cargo test --workspace --no-run` pass to enumerate all-target constructor fallout before the full test suite. Load `mismatch-handling.md`, `scope-extraction.md`, or `implementation-discipline.md` only if reassessment exposes a mismatch, ownership ambiguity, or non-mechanical implementation choice.
If a sibling ticket already landed additional fields on the same shared struct/component, treat those fields as live baseline during the constructor fallout sweep. Narrow the current ticket to the still-missing additions only, but preserve the sibling-landed fields everywhere the current ticket updates explicit literals, helpers, or fallback constructors.
For shared trait/read-surface additions, also verify whether the named trait is a blanket-forwarded facade. If the live default method owner is a narrower subtrait plus a blanket forwarder, classify the implementation boundary there before patching only the named facade trait.
When the migrated type is formatted for humans outside the primary owner module, also sweep display/report/render surfaces (for example CLI formatters, debug summaries, or golden helper renderers) even if the main behavior remains unchanged. Shared payload migrations often compile cleanly through wildcard matches while still leaving explicit formatter literals or imports stale.

## Shared additive fast checklist

1. For net-new shared type names, run an exact name sweep across `crates/` before trusting draft claims that the type does not exist. Classify hits as `same boundary`, `private unrelated helper`, or `blocking collision`, and update ticket/spec wording when a non-blocking private helper or sibling surface makes the draft premise false.
2. Confirm whether the new field/type is serde-deserialized from scenarios, saves, or other authored input; if so, decide the omitted-field/defaulting proof up front. If the draft claims old-save or cross-version loading from `#[serde(default)]`, inspect `SAVE_FORMAT_VERSION` and the live loader first; full old save files remain rejected unless explicit compatibility work is requested.
3. For authored scenario/schema fields, check generator/report/catalog readers such as `scenario_coverage`, golden inventory, or feature catalogs before coding; decide whether the new field is mapped now, intentionally unmapped, or follow-up owned.
   For universal or optional profile structs, also check whether `docs/profiles/all-profiles.md` is generator-owned. Before recording a profile-doc proof command, inspect the live generator help/docstring or source to confirm supported options; run the repo's profile-doc generator with its supported write/check mode, normally `python3 scripts/profile_docs.py --write` when no check mode exists.
   If `scripts/profile_docs.py --write` exits 0 but reports doc-comment gaps unrelated to the new field, record those warnings as pre-existing generator output in closeout and keep the generated diff only when it matches the landed profile surface.
4. Sweep manual literals and separate full `Type { ... }` literals from partial `..Default::default()` literals before accepting the drafted file list as real scope.
5. Edit constructor fallout with precise patches or syntax-aware tooling. Do not use broad regex/perl insertion across Rust struct literals; if any mechanical rewrite is still used, immediately inspect each changed hunk and re-scan touched files for accidental insertions before compiling.
6. If the additive field lands on a serialized root, persisted carrier, or component payload included in save/load state, decide the save-version policy and the focused non-default roundtrip proof up front rather than leaving it to late broad verification. Do not treat `#[serde(default)]` on a bincode-backed persisted payload as old-save compatibility by itself.
7. Name the honest proof surfaces early: default/serde behavior, authored-input parsing, bootstrap/default seeding, save/load proof when applicable, and the narrowest existing focused tests that already touch the shape.
8. Land the first shared field/type patch, then run `cargo test --workspace --no-run` early to let all-target compile fallout enumerate the remaining exhaustive constructors and helpers.
9. Treat compiler fallout as the source of truth for the remaining shared literal patch set; do not patch default-spread call sites unless the live contract actually requires it.
10. Before running focused Rust tests, resolve exact test IDs with `cargo test -- --list` and prefer module-qualified or `--exact` selectors over loose substrings.
11. Rerun the same broadened command that exposed fallout after each fix; do not rely on only the focused rerun when the broad proof has not yet gone green.
12. Update the active ticket's scope and verification sections to match the real shared-field fallout, especially when reassessment narrows the drafted constructor count or adds focused serde/authored-input/save-load proof.

For shared-field removals on existing structs/components, use the same early shared-surface discipline even when the ticket feels mechanically simple: start with a repo-wide accessor/literal/serde/authored-input sweep, then prefer an early `cargo test --workspace --no-run` pass to flush stale constructors, exhaustive fixtures, and test helpers before broad verification. Field removals often have narrower production ownership than additive tickets but broader compile fallout, so keep the patch set centered on real field readers, explicit literals, scenario/save inputs, and active docs rather than assuming only the defining type and first consumer need edits.

For shared-type validation or API-hardening tickets (for example: constructor validation, private-field migration, accessor introduction, serde validity enforcement, or other invariant-tightening on a type consumed across crates), do not treat the work as small/local just because the first edit lands in one file. Start with the ticket, cited spec/docs, `references/reassessment-checks.md`, `references/verification.md`, and `references/closeout.md`, then sweep construction paths, direct field readers, deserialization paths, and manual struct literals across the real downstream consumer boundary before finalizing scope.

## Planner-boundary exception

Single-file planner-root, snapshot-completeness, planner-traceability, or AI carriage-path tickets still use the full workflow when the contract under audit crosses the planner boundary even if the eventual edit surface stays narrow and local.
If a ticket appears to qualify for the small/local fast path by edit surface but also touches planner-root, snapshot-completeness, planner-traceability, or AI carriage-path contracts, this planner-boundary exception wins and the ticket uses the full workflow.
Planner-local runtime helper or metadata-dispatch tickets that do not alter root synthesis, search, snapshot carriage, traceability, candidate generation, or action start may stay on the small/local path when their owned symbol, focused proof, affected crate tests, and required lint/script gates cover the live boundary.
If full reassessment then proves the owned delta is validation-only or doc-and-proof only with no production behavior change, you may keep the planner-boundary reassessment but shrink the implementation and verification path back to the narrowest honest local proof and closeout surface.
Golden E2E tickets motivated by planner failures, observer reports, or scenario-specific regressions also use the full workflow even when the landed edit surface is one test file and the ticket remains test-only.
When a golden ticket asserts the presence of a new trace field or other non-default planner metadata, confirm not only that the candidate scenario exposes the relevant trace structure, but that the live scenario actually emits the asserted non-default value before treating it as a valid golden target.
When a golden or planner test-only ticket must create a new integration test file because the cited sibling substrate has not landed yet, the current ticket may absorb only the narrow local scaffolding needed to make its own proof surface real: file-local topology builders, focused setup helpers, and test-local fixture utilities. Do not silently absorb the sibling ticket's substantive scenarios or broader domain coverage. Record the deviation in reassessment and closeout, and keep the remaining sibling ownership explicit.
Within one golden/planner ticket, different scenarios may lawfully use different reconstruction strategies when the proof needs differ. A single file can mix static distilled fixtures with replayed scenario-to-tick reconstruction, as long as each case states why that path is necessary and the proof boundary remains the named planner/search contract rather than broad simulation behavior.

## All other tickets

Use the full workflow (Steps 1-8 in SKILL.md).

For full-workflow tickets, start by loading `references/reassessment-checks.md`, then `references/verification.md` and `references/closeout.md`; load `mismatch-handling.md`, `scope-extraction.md`, and `implementation-discipline.md` when reassessment or implementation reaches those steps or exposes the need. If the ticket lands a new module, type surface, helper, or staged function/method ahead of downstream integration, check during implementation whether that surface will be intentionally unused on landing and mark it deliberately before broad verification so staged scaffolding does not fail CI-matching lint passes. For Rust tickets in the full workflow, run `cargo fmt --all` before the final broadened verification pass so the last edit is not a formatting-only change that forces extra reruns before honest closeout. Do not route the active ticket Markdown or other non-Rust files through `rustfmt` unless the ticket explicitly owns that formatter surface. Inspect for formatter spillover immediately; if workspace formatting is blocked by unrelated broken files, use the narrowest repo-aware formatter that can honestly format the owned Rust surface and record the substitution.
Immediately after formatting in a dirty worktree, run `git status --short` so unrelated formatter spillover is visible before you proceed to verification or closeout.

When the ticket was authored by `/spec-to-tickets` in the current session from a freshly reassessed spec, scale reassessment to a targeted sweep: confirm the ticket's owned types still exist at stated paths, check for exhaustive matchers on modified enums, verify trait bounds on any types used in new test code, check for manual struct literals of modified types (constructors, test helpers, `from_*_for_test` patterns) that would need updating for new fields, and before adding new test-only accessors or helpers, check whether existing test infrastructure (e.g., `ActualWorldState::from_world`, test harness methods) already provides the needed capability.
