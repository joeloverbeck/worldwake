---
name: implement-ticket
description: "Implement or reassess a Worldwake ticket. Use when asked to work from a ticket in `tickets/`, `archive/tickets/`, or a worktree ticket path: read the ticket, validate its assumptions against the live codebase and repo rules, correct mismatches before coding when needed, then implement and verify the requested deliverables."
---

# Worldwake Ticket Implementation

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), the target ticket, directly relevant active specs/docs, and every spec/doc explicitly cited by the user before editing code. Confirm every dependency ticket path/status, but read archived dependency bodies only when the active ticket relies on their closeout text or the live code does not make the dependency contract evident. For planner-root, snapshot-completeness, or planner-traceability work, also read [docs/planner-contracts.md](../../../docs/planner-contracts.md) before finalizing the reassessment. If that doc does not cover the exact planner boundary under audit, record the gap and fall back to the landed implementation/spec/ticket chain instead of treating the contract as unknowable. Reassess first, then implement — do not treat a ticket as mechanically executable until its assumptions match the current codebase. Do not stop at intermediate reassessment or partial fallout; continue until the ticket is completed, fully verified, or blocked by a user decision that requires 1-3-1.

When several reassessment branches seem applicable at once, use this priority order before coding: prove the live branch first; if the drafted premise is false, rewrite the ticket/spec immediately; if the live branch still supports a narrower complete slice inside the ticket's stated domain, finish that slice and create/update a follow-up ticket for the disproved remainder; use 1-3-1 only when the remaining boundary change is architecturally ambiguous, materially expands scope, or needs a user decision. Follow the canonical mixed-outcome branch in `Mixed outcome: narrow fix landed, broader golden still false` below. `references/mismatch-handling.md` is supplemental guidance for that flow and does not override the explicit mixed-outcome and narrowing rules in this skill.

An implement-ticket invocation authorizes narrow truthing edits to the active ticket and directly cited active spec files when those edits are required to make the implemented seam, acceptance criteria, or verification record truthful. It also authorizes narrow status/dependency truth-sync edits to `specs/IMPLEMENTATION-ORDER.md` when Step 1 or closeout checks find text that is now false about the current ticket/spec boundary. It does not authorize broader repo-policy, process, or architectural-doc rewrites; for those, propose the doc update or use 1-3-1 when the decision is blocking.

## Top Rules

- Reassess against the live branch first. Do not implement the draft literally until the ticket/spec matches current code.
- Before presenting any 1-3-1, load `references/mismatch-handling.md`. If the choice involves duplicate lawful state paths, duplicate transport paths, cache-vs-truth ambiguity, or another `FOUNDATIONS`-level boundary, evaluate the options against `docs/FOUNDATIONS.md` before naming the recommendation.
- When the user approves a reassessment option or `FOUNDATIONS`-driven boundary reset, record the selected implementation boundary in the active ticket/spec before coding unless the user explicitly asked for analysis only.
- If a narrow production fix lands but the drafted broader golden/E2E story is still false, follow `Mixed outcome: narrow fix landed, broader golden still false` below.
- Prefer the strongest existing honest proof seam. Extend an existing focused unit/runtime/golden test instead of creating the drafted new file mechanically. If the ticket explicitly owns a new golden scenario/documented coverage surface and no existing suite owns that scenario page, a new `golden_*.rs` file is acceptable; record why the adjacent suite was not the owner.
- If the ticket adds or removes an internal diagnostic, trace, report, runtime, or other shared carrier field, run a constructor/consumer sweep before the first source edit and treat all CI-shaped constructor fallout as current-ticket scope.
- Keep Cargo sequential, confirm ambiguous or pre-existing exact selectors with `-- --list`, and record only truthful verification boundaries.
- Before the first Cargo command you will count as proof, load `references/verification.md` unless the ticket is already classified as a small/local fast path whose focused selector and proof boundary are unambiguous.
- Close out the ticket/spec with the real landed seam and deviations. Do not leave the correction only in conversation.
- For shared Rust struct-literal fallout, do not use broad regex/perl insertion across files. Use compiler-guided precise patches or syntax-aware tooling, and inspect each changed hunk immediately when any mechanical rewrite was used.
- During ticket/spec stale-claim scans, quote Markdown/code-span search patterns safely. Use single-quoted `rg` patterns for backticked symbols so the shell does not treat them as command substitution.
- Leave active ticket status non-completed until the last executable gate required for the final source diff has passed. Provisional verification notes may name remaining gates, but `Status: COMPLETED` must remain unset until those gates are observed complete.
- Do not archive from `implement-ticket` alone; archive only when the user explicitly asks for archival or another invoked workflow owns it.

Keep this top-level skill as routing plus hard stops. Load the referenced files for detailed case law only when the ticket shape, reassessment, implementation, verification, or closeout step needs that detail. When new guidance is not a global hard stop, put the detailed rule in the relevant `references/*.md` file and keep any top-level change to a one-sentence routing note.

## Common Fast Lanes

- Small/local CLI, observer, report, or formatter work: classify through `references/ticket-classification.md` small/local guidance; keep the proof beside the owned render/output surface; if the focused test lives in `src/bin/*.rs`, resolve the full bin-local test id with `cargo test -p <crate> --bin <bin> -- --list` before any `--exact` proof.
- Golden/observer fixture drift: use the Quick Routing golden/observer branch; prove the strongest existing lower-layer event/report contract before editing fixtures, then record fixture-only closeout when production stayed unchanged.
- Shared carrier, payload, field, or enum work: use the shared-surface and carrier-chain routes before source edits; sweep constructors, consumers, renderers, and save/version boundaries according to the matching reference.
- Closeout-only or post-proof truthing: load `references/closeout.md`; rewrite stale draft proof/spec/ticket wording first, then run scoped Markdown/diff hygiene and final dirty-path classification.

## Execution Hard Stops

- Never include a Cargo command in `multi_tool_use.parallel`. Run Cargo alone and wait for any in-flight Cargo command before starting another.
- If a resumed or compacted session has an unpollable proof-counted command, treat that prior run as untrusted and rerun it before recording verification.
- Do not use broad regex/perl insertion for shared Rust struct-literal fallout; use compiler-guided precise patches or syntax-aware tooling and inspect the changed hunks.
- Quote stale-scan `rg` patterns containing Markdown code spans with single quotes so backticked symbols are not treated as shell command substitution.
- Do not set `Status: COMPLETED` while any required proof gate is still running, blocked, unpollable, or unrun. Keep provisional closeout notes under `Verification Result` with `Blocked` rows until proof is observed complete.
- After any source, generated, scenario, test, or executable-behavior edit that follows a broad gate, rerun the narrowest affected proof and the required broad gate before completing the ticket. For final ticket/spec Markdown-only edits, run `git diff --check` plus targeted stale-claim scans.

## Mandatory Closeout Checklist

Before marking an active ticket `COMPLETED` or sending the final response, load `references/closeout.md` unless Step 0 classified the ticket as a clear small/local fast path.

Top-level closeout hard stops:
- Do not edit `Status: COMPLETED` or final verification claims while any proof-counted command is still running, blocked on an unavailable process handle, or not observed complete.
- Confirm the final source/test/generated diff is covered by completed executable verification; if source, generated, scenario, test, or executable behavior changes after a broad gate, rerun the narrowest affected proof and the required broad gate before changing `Status` to `COMPLETED`. Clippy-only or lint-only source cleanup still makes the earlier broad gate stale.
- Rewrite stale drafted acceptance/proof/spec wording before final status: remove unrun command lists, unproved `Verification Layers`, false implementation sketches, and disproved phrases from the active ticket and directly cited active specs.
- Add or update `## Verification Result`; every recorded command/proof item must start with `Passed`, `Waived`, or `Blocked`, and wrapper-equivalent proof may be claimed only when the live wrapper gates were run or individually covered.
- Run Markdown/diff hygiene after final ticket/spec edits, normally `git diff --check`, and use explicit untracked-file whitespace checks for owned untracked files.
- Run `python3 .codex/skills/implement-ticket/scripts/check_closeout.py <ticket-path>` before the final response when practical; treat warnings as a prompt to reread and truth the ticket, not as a substitute for the checklist.
- Finish with `git status --short` and classify new, pre-existing, generated, and unrelated dirty paths so the final summary attributes only the owned work.

Cargo hard stop: never use `multi_tool_use.parallel` for any Cargo invocation in this workflow. Cargo lock contention makes parallel `cargo test`, `cargo clippy`, and even `cargo test ... -- --list` probes unreliable here. Do not include a Cargo command inside any `multi_tool_use.parallel` bundle, even when the other bundled commands are read-only; run the Cargo command separately.

Before any Cargo command:
- Run it alone, outside `multi_tool_use.parallel`.
- Do not bundle it with `rg`, `sed`, `git status`, `git diff`, or any other read-only helper command.
- Poll or wait for any known in-flight Cargo session before starting another one.
- If `cargo test ... -- --list` returns several tests for a shared family/prefix, do not append `--exact` to that same prefix. Either run the family filter without `--exact` and record the nonzero test set, or run each concrete listed test name exactly.
- For tests inside `src/bin/*.rs`, use the binary target during selector discovery and exact proof. Prefer `cargo test -p <crate> --bin <bin> -- --list`, copy the full listed id such as `tests::name`, then run `cargo test -p <crate> --bin <bin> tests::name -- --exact`; a package-level bare function name can compile targets while executing zero tests.

For CLI smoke checks where the ticket drafts a Cargo command piped to a read-only filter, such as `cargo run ... -- --help | grep flag-name`, keep the Cargo execution itself unambiguous. Prefer one of these proof shapes:
- Run the Cargo command alone and inspect or relay the relevant output line.
- If you keep the drafted shell pipeline for fidelity, treat it as a CLI smoke/output check rather than a Cargo concurrency pattern: run it alone, never inside `multi_tool_use.parallel`, and do not start another Cargo command until it exits.

## Quick Routing

- Start by deciding whether the ticket is really implementation work, validation-suite work, golden/observer proof work, or shared-surface migration work.
- For shared enum/payload variant tickets, first classify the owner enum/payload, any sibling event/tag or re-export surface that must stay in lockstep, and the likely exhaustive consumer families (observer/CLI/report/render/test fixtures). Treat the migration as more than an owner-file edit even when the first patch lands in one crate. When a ticket pairs an `EventTag` with a decision/event payload enum, sweep each carrier independently: tag membership may be non-exhaustive while `DecisionEventPayload` or report payload renderers still have exhaustive observer/bin/report matches such as `src/bin/observer.rs`.
  For new `InstitutionalClaim` variants, load `references/reassessment-checks.md` and use its institutional-claim variant checklist before coding.
  For shared payload widening, event payloads, or cross-crate tag/payload conversions, use the compact preflight in `references/ticket-classification.md` and the event carrier-chain checks in `references/reassessment-checks.md` before accepting the drafted carrier or conversion site.
- For internal diagnostic, trace-carrier, report, runtime, event, or payload carrier changes, load `references/reassessment-checks.md` and use its carrier-chain, constructor/consumer, typed-reference, and save-boundary checks before coding.
- For planner/search filters that consume a per-tick derived read-model, use `references/reassessment-planner-ai.md` and trace read-phase creation, production handoff, test-wrapper defaults, and the final planner/search consumer before patching.
  When a planner-boundary ticket also adds or removes a trace/provenance carrier field, apply both planner-boundary reassessment and trace-carrier constructor/consumer sweep guidance.
- For shared trace/report/runtime carrier field removals, do the same enclosing-save sweep before accepting a draft's "trace-only" or "not persisted" claim. Check wrappers such as `AgendaEntry`, `AgentDecisionRuntime`, saved runtime mirrors, bincode round-trip fixtures, and `SAVE_FORMAT_VERSION`; removing fields from a nested serialized carrier is still a current-format save-shape change even when the carrier also appears in per-tick trace output. When removing fields from `Component` profile types such as `CognitiveProfile` or `PerceptionProfile`, treat the change as persisted current-format save shape unless the live code proves otherwise; check `SAVE_FORMAT_VERSION`, generated profile docs, scenario defaults, fixture inputs, and sibling ticket/spec handoffs that claim no migration or later bump.
- For shared trait/read-surface additions, classify the real consumer-facing boundary before editing: facade trait vs owning subtrait, blanket/default implementation, forwarding macro or blanket impl, runtime/per-agent backing type, sibling traits with the same method name, and test doubles that may inherit a default `None`/empty implementation. Prove the accessor through the real runtime/per-agent view when that is the honest consumer path, and truth-sync ticket/spec wording if the draft names a stale macro, constructor, or implementation layer.
- For schema-only / staged substrate tickets, run the compact preflight in `references/ticket-classification.md` before patching. Keep top-level routing to the owner/carrier/save/public-surface decision; use the reference for macro fallout, generated inventories, runtime operand classification, inert staged variants, and dormant-infrastructure closeout.
- For authored scenario schema renames, payload migrations, or additive authored fields, treat the authored definition as a shared scenario boundary even when it lives in `worldwake-cli`. Load `references/shared-field-migration.md`; sweep committed `scenarios/`, `scenario_coverage`, lints, handler/display helpers, same-crate tests, and cross-crate fixture constructors before proof; then record any generator or constructor fallout as current-ticket scope.
- For S134 or other `ActionDef.effect_schema` tickets, including effect-schema consumers, indexes, and read-models, load `references/effect-schema-migrations.md` before source edits. Classify each drafted step as genuinely generic or category-owned during reassessment, and do not flatten domain aftermath into a generic effect step just to satisfy stale spec sketches. For planner-switch or planner-parity tickets, also load `references/reassessment-planner-ai.md` and name the candidate-generation, search, transition, and action-start proof boundaries before broad verification.
- For new `Precondition` variants, compare each consumer's read boundary separately before coding: semantic declaration/inventory, authoritative validation (`World`/`WorldTxn`), affordance/runtime belief evaluation (`RuntimeBeliefView` and its concrete trait owner), target-index/implication helpers, and serialization or representative inventory tests. If one consumer cannot read the needed state through its current boundary, identify the narrow trait/helper owner and update the ticket before patching.
- For documentation-only roadmap/report tickets, first rank the authoritative live sources before writing prose: generated companions and live code/generator rules first, authored scenarios/tests next, draft design docs last when they conflict with live evidence. If the draft collapses structural activation, behavioral proof, and broader “landed” status into one claim, rewrite the ticket/doc boundary to those separate layers before editing.
- For roadmap-owned scenario landing tickets, treat the full landing contract as one owned closeout seam: authored scenario, backing golden, generated companions, workflow ownership, and roadmap/editorial status must agree before the row is marked landed. Do not stop at the scenario/golden diff if the live row or CI matrix still understates the landed seam.
- For ordinary implementation tickets that land or extend behavior for an existing `docs/scenario-roadmap.md` row, borrow the `scenario-roadmap-landing` truth-sync standard for docs/generated/roadmap consistency without switching to the full roadmap-landing workflow. Scenario, golden, generated companions, and roadmap prose must not tell conflicting stories about the landed behavior.
- For tooling/report/generator tickets, first confirm the canonical read-only input boundary and whether every claimed output row or classification is actually derivable from that boundary. If the draft asks the tool to infer runtime-only or later-stage facts from a narrower authored schema, rewrite the ticket to the honest schema-owned seam before coding.
- For UI, visualizer, or interactive tooling tickets, treat crate-local README/manual-QA text as part of the live handoff surface even on the small/local fast path. If the ticket makes a visible shell, control, modal, tab, or staged surface live, sweep nearby user-facing docs before closeout and remove or update stale placeholder/future-tense/manual-QA wording.
- For golden/observer proof tickets, first decide whether the mismatch is renderer/fixture drift or a real upstream event/report contract regression. Prove the upstream contract at the strongest existing lower-layer owning seam before editing any fixture, and if that contract is still honest, narrow the ticket to fixture truthing plus closeout.
  For golden metadata, authored-scenario, generated-doc, ignored-test, and closeout details, load `references/verification.md` and `references/reassessment-golden.md`; keep the top-level decision here to mismatch classification plus strongest honest proof seam.
- For validation-suite / tests-only tickets, diff the drafted `Files to Touch`, `New/Modified Tests`, and `Acceptance Criteria` against the live branch immediately; record `already landed`, `still live`, and `no-change cited files` before planning new tests.
- For existing per-tick maintenance-pass extensions, keep the added pass in the current SystemFn when that is the live owner. Before coding, name the pass ordering, collect-before-commit or apply-time reread shape, transaction tags/targets, missing-profile or missing-component skip behavior, idempotence/no-op contract, and whether the change affects persisted shape. When the added side effect lives beside an existing staged write map, identify whether it has an independent dirty/changed carrier and update early-return, transaction-open, and commit gates so the new mutation is not skipped when the older staged map is unchanged.
- When the live system routes mutations through a local `commit_*`, `apply_*`, or similar helper, treat that helper as the mutation boundary. Thread narrow effect data into it and write inside its existing `WorldTxn` rather than adding a second after-the-fact `World` mutation path.

## Always First

- Resolve the exact live ticket path and any cited spec/reference path before relying on draft wording.
- If the ticket family is already partially landed on the live branch, identify the remaining live delta before adopting the drafted `What to Change`, `Files to Touch`, or proof plan sections.
- If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all operations.

Apply the Cargo hard stop above throughout the workflow. After a resume or context compaction, check for and poll any known in-flight proof-counted command before rerunning the same command. This includes Cargo plus generated-doc scripts, report generators, CI wrappers, and other executable checks whose exit status will be recorded in closeout. If the saved process handle is unavailable, cannot be polled, or no longer proves a completed exit status, treat the prior run as untrusted: rerun the command and record only the observed completed run as verification.
When the ticket's proof plan names a specific script, bin target, workflow entry, or test file, verify the exact live entrypoint name before treating the drafted command as authoritative. If the ticket spells the target differently from the live repo, correct the proof lane to the honest current entrypoint before closeout instead of preserving the stale command.
When the proof lane depends on a repo script such as `scripts/verify.sh` or `./scripts/verify.sh`, treat the script's live contents and observed output as authoritative over surrounding prose summaries. If the ticket's Test Plan names the script, inspect it during context loading or reassessment before coding so repo-specific gates can shape naming, cleanup, and implementation constraints. Inspect or summarize the actual gate set before closeout, especially when the script has gained extra checks beyond the documented high-level command list. If the script exits successfully but its streamed output has no explicit final success marker, read the script or run a cheap `sed`/`rg` over it before recording the exact gate list. If the last wrapper command is quiet and the process handle closes without a clear completion line, rerun that final command directly or explicitly record why the observed wrapper exit status proves it.
If the final/reassessed Test Plan still lists `scripts/verify.sh` or `./scripts/verify.sh` but the session is not preparing a PR, inspect the script and either run the wrapper or run all live gates it currently wraps directly. Record any substitution explicitly in ticket closeout, including which wrapper gate set was covered and that the wrapper itself was not run. If reassessment removes a drafted wrapper as stale, unowned, or outside the truthful acceptance boundary, record the reason, substituted proof boundary, and strongest final commands instead; do not claim wrapper-equivalent proof unless the wrapper gate set was covered.
When the live gate set includes repo-specific cleanup, removal, or drift-detection scripts, inspect their searched symbols or invariants early enough that new code does not reintroduce banned names or stale surfaces. If a script exists to prove an old concept is gone, treat its patterns as naming constraints during implementation, not only as final verification fallout.
For Worldwake Rust tickets, default to `cargo fmt --all` after significant Rust edits so local formatting matches the repo's `verify.sh` gate. In dirty worktrees, immediately inspect `git status --short` after formatting and classify any unrelated formatter spillover before proceeding.
When a patch changes a serde/bincode-persisted enum, struct, or carrier, check `SAVE_FORMAT_VERSION` even if the ticket was not classified as serialization or save/load work. Save/load is a representation boundary; record any required version bump or intentional no-bump in closeout.

For long diagnostic implementations likely to hit interruption or context compaction, keep a compact checkpoint after major pivots: current hypothesis, temporary probes still present, commands already passed, and remaining verification gates. Use the active ticket when the notes are durable closeout facts; otherwise use a disposable local note or the conversation, and fold only final truths into ticket closeout.

## Mixed outcome: narrow fix landed, broader golden still false

When focused live proof confirms a real narrow production fix inside the ticket's domain but the drafted higher-level golden/E2E ending still does not hold afterward, load `references/mixed-outcome.md` and follow that playbook. In short: land the narrow owning seam, stop broad scenario churn, rewrite the active ticket/spec to the proved boundary, record the still-false premise, create or update the follow-up owner, clean up temporary probes, and close out with the split explicitly documented. For multi-scenario golden suites where only some scenarios can move to a source-backed seam, use the reference's scenario-by-scenario proof-seam split.

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

After classification, decide and record whether `references/verification.md` must be loaded before the first proof-counted command. Load it before proof for all non-small/local tickets, and for any small/local ticket whose focused selector, generated artifact check, wrapper gate, or proof boundary is ambiguous.

For tickets that clearly fit the small/local fast path, the fast-path instructions in `references/ticket-classification.md` are sufficient for the Step 2, Step 6, and Step 7 reference-loading requirements unless reassessment exposes ambiguity, mismatch, broader fallout, or verification uncertainty. In that case, load the specific deeper reference named by the workflow section that now applies. Do not force the full reassessment, verification, and closeout reference set for genuinely local work when the ticket, cited docs/specs, owned symbols, focused proof, affected crate tests, and required lint/script gates already cover the live boundary.

### 1. Load the ticket context

1. Read the target ticket file.
2. Read every directly relevant reference (specs, docs, code symbols, test files), including any specs/docs explicitly named by the user even when the ticket does not link them. Treat Markdown links, raw repo paths, and path mentions inside ticket comments as cited references when they point to specs, docs, or process rules that affect reassessment or verification.
3. When the user supplies a glob, shorthand, or obvious near-match typo, confirm the exact live file path before reading or relying on it.
   If the invocation includes extra positional tokens or path-like hints after the ticket path, resolve the first live ticket path first, then treat the remaining tokens as optional reference hints unless they clearly resolve to another ticket or a worktree root.
   If the only live match changes the numbered ticket/spec family, announce the resolved path before coding; if more than one plausible numbered-family match exists, use 1-3-1 before proceeding.
4. When the ticket name implies a numbered family or the user cites a parent spec, search for sibling tickets in the same family and confirm whether adjacent missing substrate is already owned elsewhere before broadening or narrowing the current ticket.
   Use filesystem discovery such as `rg --files tickets | rg '<family-id>'` or an equivalent exact family scan; do not rely only on `git status` or ordinary diffs, because untracked sibling tickets are valid active state.
   When the parent spec is broader than the active ticket and sibling tickets already own adjacent deliverables, treat that sibling decomposition as the live implementation boundary unless reassessment proves the split is obsolete.
   When that family already exists as dirty or untracked draft tickets/specs in the worktree and the user asked for implementation-only work, keep edits scoped to the active ticket unless broader family ticket/spec updates are required to keep the repo truthful.
   When live API reassessment corrects a code-like helper, function signature, event-emission shape, or payload-construction snippet in the active ticket, immediately search active sibling tickets in the same family for the same stale snippet and update only the code-like guidance whose current wording is now false.
   When the current ticket absorbs a comment, doc note, cleanup item, visibility change, or other handoff previously assigned to a sibling, also truth-sync sibling narrative ownership claims so they no longer say the later ticket still owns already-landed work.
   When the user cites an active `specs/SNN*` family or the ticket belongs to an active spec family, scan `specs/IMPLEMENTATION-ORDER.md` for the spec id and any deliverable names that reassessment changes, defers, or completes. Update only text that is now false about the live ticket/spec boundary.
5. If the ticket's Test Plan names `scripts/verify.sh` or `./scripts/verify.sh`, inspect the live script during this context-loading pass so its current gates shape implementation constraints, naming choices, and closeout verification.
6. Check whether the active ticket file is tracked or untracked in the current worktree. Untracked ticket drafts are valid active state, but they will not appear in ordinary `git diff` output. Keep this in mind during diffs, close-out evidence, and follow-up ticket reporting throughout the workflow. Apply the same caution to any new implementation files or directories you create: use `git status --short` or another explicit untracked-file listing so newly added source trees are not omitted from the landed-surface summary.
7. Snapshot the current worktree with `git status --short` before coding. Classify unrelated dirty paths immediately as pre-existing user or autogenerated state, and keep them out of ticket fallout unless the ticket truly touches them. Use this early classification later during closeout so unrelated paths are not misreported as ticket work.
8. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all operations.

### 2. Reassess assumptions before coding

Verify the ticket against the current codebase, not stale architectural memory. Check `Deps` — confirm each dependency is present on the current branch. If a dependency ticket has already been completed and archived, rewrite `Deps` to the live archived path instead of leaving a stale active-ticket reference. For archived prerequisite tickets, prefer live-code verification of the dependency contract when the symbols, tests, or closeout surface are evident; read the archived ticket body when the current ticket depends on its narrative conclusion, accepted deviation, or ambiguous handoff. If the requested ticket depends on an active pending sibling ticket, stop before implementation and use 1-3-1 to confirm whether to implement the prerequisite first, narrow/reject the requested ticket, or pause. When the user approves the dependency-first path, treat both the prerequisite and requested ticket as active closeout surfaces: implement the prerequisite first, then the requested ticket, and update both ticket/spec boundaries truthfully before final verification. For mixed-layer, planner, golden, or authoritative-validation work, name the exact symbols and boundaries under audit.

When a ticket adds a new crate, binary, GUI shell, or other dependency-bearing tooling surface, treat drafted third-party dependency snippets as provisional until compiled on the live branch. Run an early focused compile/check after adding the manifest, and if crate feature constraints, build scripts, or current dependency APIs disprove the draft, update the ticket/spec dependency text before continuing instead of leaving the correction only in Cargo fallout.

When a ticket/spec touches `Permille` or another bounded numeric wrapper, validate cited examples, profile values, and sibling-ticket snippets against the live type bounds during reassessment. If active spec-family examples are out of range, correct the active spec/ticket family before coding instead of letting focused tests discover the invalid values later.
When a shared value-type ticket makes trait, identity, or keying claims, verify those claims against the live derives and key carrier before coding or closeout. Check both the new type and the containing/future consumer type for `Copy`, `Hash`, `Eq`, `Ord`, serialization, and save-shape wording, then truth-sync active sibling tickets/specs whose handoff text still describes a stale identity mechanism.
When a ticket or cited active spec sketches a code-like API shape such as a helper signature, method receiver, enum path, or constructor form for the owned surface, compare that sketch against the live or landed shape during reassessment. If the implementation intentionally lands a different method/free-function/export shape, truth-sync the active ticket/spec before final broad proof when possible, not only during final stale-claim cleanup.

Load `references/reassessment-checks.md` unless Step 0 classified the ticket as a clear small/local fast path whose cited docs/specs, owned symbols, and live proof boundary are already covered by `references/ticket-classification.md`. For planner-root, snapshot-completeness, planner-traceability, or AI pipeline work, also load `references/reassessment-planner-ai.md`. For golden E2E or observer-motivated tickets, also load `references/reassessment-golden.md` and `references/reassessment-golden-observer-report.md` unless the same small/local fast-path exception applies.

Detailed domain-specific reassessment case law lives in `references/reassessment-checks.md` and the other references named by the ticket shape. Keep this Step 2 section to universal reassessment obligations; load the relevant reference instead of expanding the top-level workflow.

### 3. Handle mismatches explicitly

Load `references/mismatch-handling.md` when reassessment exposes a contradiction, risky ticket/code divergence, or a user decision that requires 1-3-1.

Before recommending one of the 1-3-1 options, run a `FOUNDATIONS` check over the choices. If the contradiction involves two possible canonical owners for the same fact, duplicate mutation/diff/transport paths, or a cache that could become truth, default the recommendation toward one canonical owner unless the spec proves the separate carrier is a real world artifact with its own lifecycle, observers, invalidators, and legal effects.

When the user interrupts or redirects implementation with a request to reassess options against `docs/FOUNDATIONS.md`, pause source edits and treat the request as an architectural boundary check. State one concrete contradiction, offer three viable options with one recommendation, wait for approval when the choice materially changes scope, then patch the active ticket/spec with the selected boundary before coding.

When reassessment corrects a code-like sketch in the active ticket itself — helper signature, match arm, payload construction, event emission, or trace-routing snippet — update that local code block in the same pass as the surrounding prose. Do not wait until final closeout to remove a snippet that now describes an impossible implementation shape.

When the user chooses an option from a 1-3-1 or other reassessment menu, immediately record the selected option's implementation boundary in the active ticket/spec before coding, unless the user explicitly asked for analysis only. Keep the note narrow: name the chosen boundary, the rejected stale premise, and any verification or sibling-sync consequence that is already known.

If reassessment changes a shared API, type contract, schema shape, or cross-ticket assumption, update any still-active dependent tickets/spec references in the same family before implementing or closing out the current ticket. If the current ticket completes but remains active and a sibling's dependency reference is still truthful, no sibling edit is required; update siblings when their wording now falsely says the blocker is unresolved, when the dependency path changed through archival, or when the landed contract differs from the sibling assumption.
When reassessment changes a family-wide shared API, payload, or save-version premise, run a targeted active-family stale-claim scan before the final broad verification gate, then do a lightweight final scan after closeout prose. This keeps the broad gate tied to the final source/test/generated diff while still catching late prose drift.
When a ticket family has multiple planned save-shape changes, keep the sibling version chain truthful as each ticket lands. If the current ticket takes a `SAVE_FORMAT_VERSION` bump earlier than the draft expected, update later active siblings and active specs to treat the new version as their baseline and to own the next bump, instead of leaving stale `old -> new` wording in place. Include concrete stale-phrase scans such as `#[serde(default)]`, `old saves`, `pre-SNN`, `custom Deserialize`, `SAVE_FORMAT_VERSION`, `old -> new`, and `compatibility` when the save-shape premise changes.
If that reassessment also disproves an archived sibling's forward-looking handoff, dependency note, or prior closeout claim, amend the archived record truthfully. Keep the edit narrow and factual: record that the later live reassessment corrected the handoff, without rewriting the archived ticket into the current ticket's full closeout.
When a second-pass ticket/spec rewrite materially narrows or rebinds the live architectural seam after earlier family sync already happened, re-check active sibling tickets and active specs again before closeout. Do not assume the first sync pass is still sufficient once the owned boundary changes a second time.
If a golden-driven ticket proves that the scenario contract itself is underspecified — for example, the scenario can pass or fail without proving the authored causal branch — escalate from local ticket reassessment to the owning golden-policy / scenario-roadmap docs and any live roadmap tickets instead of recording the gap only in ticket closeout.
When a golden reassessment renames a scenario test or changes its authored contract, sweep any tracked `docs/generated/golden-*` inventory, index, or detail artifacts that mirror that scenario and update both test-name references and scenario prose before closeout.
When those generated artifacts carry source-line references or other layout-sensitive metadata, rerun the generator after final formatting or comment cleanup as well, not only after semantic test edits, so the generated companion still points at the final live file layout.
After regeneration, inspect any new or changed `docs/generated/golden-scenario-details/*.md` `Setup` and `Proves` blocks for disproved terms from reassessment; generated detail prose mirrors scenario metadata comments and can preserve stale semantics even when the tests assert the corrected contract.
For generated golden companions, prefer `python3 scripts/golden_inventory.py --write --check-docs` as the canonical regenerate-and-check command. Use standalone `python3 scripts/golden_inventory.py --check-docs` only as a deliberate read-only drift check, and expect it may print the generated inventory rather than acting as a quiet closeout confirmation.

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

For derived forensic/report/read-model tickets, including registry-derived indexes cached on runtime drivers, use the compact checklist in `references/scope-extraction.md`.

For small CLI/tooling tickets that touch a single binary or local helper surface, explicitly check whether the honest focused proof belongs beside the owned function/module (for example in `src/bin/*.rs` tests) rather than in a new integration-test binary. If the ticket sketches a new `tests/*.rs` file but the live seam is a local formatter/helper inside one binary, narrow the test placement to that seam and record the deviation in the ticket closeout.
If the live emitter, runtime mutation seam, and focused verification harness already live together in one existing file or module, prefer landing the change there over creating a new sibling file from the drafted sketch. Record that live seam explicitly in the ticket closeout whenever it wins over the drafted file layout.
When the live proof seam or owned implementation boundary differs from the ticket's drafted sketch, record the exact landed seam in the ticket closeout instead of preserving the drafted shape in prose. Small/local tickets often land through a private extracted helper or a narrower same-file formatter/test seam even when the draft described a more direct in-place edit.
Before implementation scope is final, compare `Verification Layers`, `Acceptance Criteria`, `Test Plan`, and drafted `New/Modified Tests` for one-to-one proof coverage. If those sections name different required commands or proof seams, either plan to run the union of required proof commands or rewrite the ticket before closeout to explain the truthful substitution. If the ticket names an invariant that requires its own focused proof but the drafted tests do not include it, correct the ticket/test plan up front instead of discovering the missing proof only during closeout.
When the ticket names exact fixture cardinalities, row counts, scenario counts, or similar numeric acceptance examples, mirror those values in the focused proof unless live reassessment shows they are impossible or misleading. If the proof intentionally uses different numbers, record that deviation in the ticket before closeout.
When a small/local ticket creates a user-facing UI or visualizer shell, keep the staged shell operational and avoid visible in-app text that explains implementation staging, future tickets, or debugging internals. Put staging boundaries and deferred controls in ticket/spec closeout instead; the app surface should show only user-meaningful state and controls for the slice that actually landed.
When a small/local UI, visualizer, or tooling ticket makes a previously staged surface live, sweep the crate-local `README.md`, manual QA checklist, and nearby user-facing docs before closeout. Remove stale “empty shell”, “placeholder”, or future-tense wording that now conflicts with the implemented tool, and record any README/manual-QA refresh in the active ticket's landed file surface.

For doc-only or compile-time regression tickets, use the compact checklist in `references/scope-extraction.md`.

### 5. Implement with Worldwake discipline

Load `references/implementation-discipline.md` when reassessment shows a non-mechanical, shared-surface, or otherwise higher-risk change that needs extra guardrails. For straightforward small/local tickets whose reassessment already proved the edit path is mechanical, you may proceed without loading that reference.

Use `references/implementation-discipline.md` for detailed editing patterns: exact constructor/callsite rewrites, helper/context shaping, trait-boundary reuse, shared aftermath extraction, maintenance-pass batch staleness, and CI-shaped lint fallout from shared payload widening.

### 6. Verify at the right boundary

Run the narrowest correct verification first, then broaden.

Load `references/verification.md` unless Step 0 classified the ticket as a clear small/local fast path whose focused selector, generated artifact check, wrapper gate, and proof boundary are unambiguous and already covered by `references/ticket-classification.md`.

#### Cargo execution discipline

Use `references/verification.md` for focused selector rules, exact-test pitfalls, multi-test proof shapes, and invalid drafted Cargo syntax. Keep only the hard stop here: a selector that runs zero intended tests, or only the wrong test set, is non-proof and must not be recorded as verification.

If repo guidance or the active verification contract requires `cargo fmt --all`, run it, then immediately inspect `git status --short` and classify any formatter spillover in already-dirty files as unrelated or adjacent fallout unless the current ticket truly owns those paths. Record that spillover explicitly in closeout rather than silently attributing the formatted files to the ticket.

When a tooling or CI-script ticket needs a deliberate negative probe and the owned contract is that a whole script fails at a new late-added gate, let the full script run and record the exact failing step/message instead of short-circuiting through a narrower surrogate. Pair that probe with cleanup-safe restoration so the worktree returns to its pre-probe state before closeout.

#### Focused vs. broadened scope isolation

If focused proof passes and broadened verification fails in an unrelated existing test/fixture family, isolate that failure immediately. Create or update the owning ticket, record the blocker in the current ticket closeout, and keep the current ticket scoped to the invariant that was actually proved instead of absorbing the unrelated fallout by default.
When a required broad or ignored golden fails but the focused proof, source diff, and other broad gates suggest the current ticket is not causal, prove pre-existence before labeling it unrelated. Prefer a clean baseline worktree at `HEAD` or another explicit clean baseline run of the same command; record the compared commit, command, and matching failure signature in the active ticket closeout and create/update the owning follow-up ticket.

When broadened verification fails on a mechanical, semantics-preserving lint in code outside the ticket's behavioral owner, first confirm it is not a hidden contract change. If it is trivial verification hygiene such as a Clippy-suggested `map_or` rewrite, the current ticket may absorb the narrow cleanup so the CI-matching gate can pass; record the touched files and no-semantic-change nature in closeout, then rerun the full wrapper or equivalent live gate set after the lint cleanup. If the lint requires judgment, changes public API shape, or affects behavior, hand it off as unrelated fallout instead of absorbing it silently.

When broadened verification exposes intentionally staged unused private surface (for example new enum variants, helper entry points, or report fields that sibling tickets will start using later), the current ticket may absorb the narrowest local lint-safe annotation or cleanup needed to keep CI-matching verification green. Record that staged-state deviation explicitly in the ticket closeout instead of silently treating it as unrelated noise.

When a ticket or spec explicitly fixes a public or shared API shape that conflicts with CI-matching pedantic lint expectations (for example a small `Copy` type passed by reference), do not silently flip the signature just to satisfy clippy. Either correct the ticket/spec during reassessment if the contract is not actually important, or preserve the requested shape and apply the narrowest lint-safe allowance/adapter needed, then record that deviation explicitly in closeout.

When a golden/test-only ticket is green at its owned proof seam and existing lower-layer coverage still proves the production contract, a broader same-crate rerun may still expose an unrelated golden in another family. Isolate that failure, hand it off explicitly, and keep the current ticket closed as test-only rather than reopening production ownership by default.

#### Golden, observer, and report scope verification

Use `references/verification.md` for detailed golden/observer/report proof rebinding: exact integration-test binaries, ignored scenario cases, event-payload scope, multi-producer output surfaces, and temporal-contract classification.

When a need/drive golden or authored scenario requires retuning pressure weights, rates, thresholds, or escalation parameters, enumerate all lawful relief paths for the target pressure before editing profile numbers. Check whether another candidate already relieves the same need as a side effect, such as food relieving thirst, rest affecting multiple pressures, or a wash/relief action resetting exposure. Bind the scenario edit to the intended causal branch first, then retune authored state so the proof exercises that branch rather than hiding it behind a competing relief path.

#### AI runtime, planner, and golden verification

For detailed AI runtime, planner-boundary, hybrid golden, scenario-metadata, fixture-refresh, and generated-doc fallout guidance, use `references/verification.md` and the relevant reassessment references instead of expanding the case logic here. Keep the top-level decision simple:
- if the proof crosses planner or AI runtime boundaries, load `references/reassessment-planner-ai.md` and prove the earliest honest boundary before broad verification;
- if the proof is a golden, observer, or generated-report seam, load `references/reassessment-golden.md` and `references/reassessment-golden-observer-report.md`;
- if the live golden seam changes, update scenario metadata and regenerated companions as part of the owned verification/closeout surface.

#### Manual probes and temporary scaffolding

Use `references/verification.md` for cleanup-safe manual-probe details. Before broadened verification, remove temporary diagnostic tests, trace prints, probe assertions, and marker strings unless the ticket explicitly owns keeping that instrumentation; when marker names were used, run a targeted `rg` cleanup check and then rerun the narrowest affected proof.
Temporary diagnostic worktrees are acceptable for clean-baseline reproduction when a failure may be pre-existing. Put them outside the repository checkout, use them only for isolated proof, then remove them before closeout or explicitly report any remaining path and why it was left behind.

#### Save-format and serialized-surface verification

When a shared serialized-surface ticket mentions both a save-format bump and `#[serde(default)]` or other defaulted new fields, decide explicitly whether older saves are meant to load at all before you finalize reassessment. For bincode-backed save payloads, treat additive fields as positional shape changes; `#[serde(default)]` is not an old-save compatibility plan by itself. If the repo's no-backwards-compat rule means the real contract is version rejection, rewrite the ticket/spec wording immediately instead of letting default-field language imply an unsupported migration path.

When a shared serialized-surface ticket widens a schema, payload, enum, or saveable record, perform one final save-format check after focused proof is green: reopen the live `SAVE_FORMAT_VERSION` constant and confirm the landed diff bumped it exactly once relative to the current worktree state, not merely relative to the earlier reassessment snapshot or ticket text.

### 7. Close out the ticket honestly

Load `references/closeout.md` unless Step 0 classified the ticket as a clear small/local fast path and `references/ticket-classification.md` already covers the final ticket truthing, stale-scan, hygiene, and status requirements for the landed diff.

If final edits happen after broad verification, classify the freshness boundary before reporting completion: code, generated artifact, scenario, or executable-proof edits require rerunning the relevant broad or targeted executable checks; non-generated ticket/spec Markdown edits normally require `git diff --check` plus any targeted stale-claim scans. Record which broad gates were pre-final-edit and which post-edit checks prove the final diff.
After final ticket/spec Markdown edits made after executable verification, run `git diff --check` or explicitly record why it was not run before reporting completion.

Do not set an active ticket's `Status` to `COMPLETED` until the required executable verification gates for that ticket have passed. If a proof-counted command is still running, blocked on a process handle, or unpollable after resume, leave the status non-completed until the command is observed complete or rerun. If closeout text must be drafted before the final gate, keep `Status` non-completed and pair the draft with provisional `Blocked` verification rows naming the remaining gates; do not report completion until the final ticket text reflects the observed completed verification.

For documentation-only roadmap/report tickets, use this compact closeout checklist before marking completion:
- restate the authoritative-source order that won during reassessment (for example generated companion, live code/generator rule, authored scenario/test, then draft design doc)
- distinguish generated evidence from editorial intent in the landed doc
- classify any live warnings as intentional exclusions, auxiliary evidence, or explicit follow-up work rather than leaving them as unexplained noise
- verify section structure, top-level cross-links, and any claimed lockstep table/catalog against the live source they mirror
- when a scenario row or status changes, sweep same-document summary tables, status matrices, and section-anchor references so older `planned` / `in progress` prose does not survive above or below the rewritten owning section
- when an implementation ticket changes `scenarios/*.ron`, `golden_*.rs`, or generated golden docs in a way that adds, removes, renames, or materially re-scopes behavioral proof for a roadmap scenario, inspect `docs/scenario-roadmap.md` before closeout and update any feature catalog, status summary, ordered-row entry, row-section prose, and final-integration support note that now conflicts with the generated docs or live assertions
- record drafted-vs-live boundary corrections in the ticket `Outcome`/`Deviations`, especially when a draft “landed” claim narrowed into `auxiliary` or `planned`

Use `references/closeout.md` for detailed closeout edge cases: optional rendering and disabled sections, final ticket/spec rereads, prose-only post-gate hygiene, staged-default/dormant-infrastructure scans, stale sketches, final status comparison, family-wide truth scans, fixture-only closeout, and generated-warning records.

When a ticket adds new authored scenario/schema fields, explicitly check whether `scenario_coverage` or another live feature catalog should map them before final closeout. If the field is intentionally still unmapped, record that generated warning as expected live output rather than silently leaving the new schema surface unexplained.
For audit-then-fix tickets that classify multiple rows or arms, include a compact row-level proof disposition in closeout whenever rows were added, removed, or repaired during reassessment. Each changed row should name the focused proof, shared lower-layer proof rationale, or explicit out-of-scope owner that makes the final status honest.
If broadened verification proves the landed contract differs from the drafted invariant or top-level ticket summary, update the ticket's header fields and any now-false acceptance or invariant wording as part of closeout instead of recording the difference only under `Deviations`.
When a behavior-preservation ticket originally demanded a separate pre/post event-log, state-hash, or baseline artifact and the final proof used focused tests, goldens, wrapper gates, or another honest substitute instead, rewrite the acceptance text to the proof actually run. Record explicitly that no separate baseline artifact was captured rather than leaving the old proof claim implied.
Before final reporting, scan the active ticket/spec text for stale draft markers and disproved phrases introduced by the session: `TBD`, `likely`, `confirm placement`, obsolete proof commands, old ownership claims, impossible sketches, and any key terms that reassessment replaced. Update or explicitly supersede only the text that is now false.
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
