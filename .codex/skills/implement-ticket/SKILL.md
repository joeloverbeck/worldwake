---
name: implement-ticket
description: "Implement or reassess a Worldwake ticket. Use when asked to work from a ticket in `tickets/`, `archive/tickets/`, or a worktree ticket path: read the ticket, validate its assumptions against the live codebase and repo rules, correct mismatches before coding when needed, then implement and verify the requested deliverables."
---

# Worldwake Ticket Implementation

Read [AGENTS.md](../../../AGENTS.md), [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md), the target ticket, and any ticket-linked specs or docs before editing code. For planner-root, snapshot-completeness, or planner-traceability work, also read [docs/planner-contracts.md](../../../docs/planner-contracts.md) before finalizing the reassessment. If that doc does not cover the exact planner boundary under audit, record the gap and fall back to the landed implementation/spec/ticket chain instead of treating the contract as unknowable. Reassess first, then implement — do not treat a ticket as mechanically executable until its assumptions match the current codebase. Do not stop at intermediate reassessment or partial fallout; continue until the ticket is completed, fully verified, or blocked by a user decision that requires 1-3-1.

## Workflow

### 0. Classify ticket shape and pick the right path

Load `references/ticket-classification.md`.

### 1. Load the ticket context

1. Read the target ticket file.
2. Read every directly relevant reference (specs, docs, code symbols, test files).
3. When the user supplies a glob, shorthand, or obvious near-match typo, confirm the exact live file path before reading or relying on it.
4. When the ticket name implies a numbered family or the user cites a parent spec, search for sibling tickets in the same family and confirm whether adjacent missing substrate is already owned elsewhere before broadening or narrowing the current ticket.
5. Check whether the active ticket file is tracked or untracked in the current worktree. Untracked ticket drafts are valid active state, but they will not appear in ordinary `git diff` output. Keep this in mind during diffs, close-out evidence, and follow-up ticket reporting throughout the workflow.
6. If the ticket lives under `.claude/worktrees/<name>/`, treat that worktree root as the repository root for all operations.

### 2. Reassess assumptions before coding

Verify the ticket against the current codebase, not stale architectural memory. Check `Deps` — confirm each dependency is present on the current branch. If a dependency ticket has already been completed and archived, rewrite `Deps` to the live archived path instead of leaving a stale active-ticket reference. For mixed-layer, planner, golden, or authoritative-validation work, name the exact symbols and boundaries under audit.

When a ticket adds a runtime report, forensic surface, or other derived read-model type, verify the requested trait/derive surface up front on the live branch rather than trusting the ticket sketch. Check whether every nested field already satisfies the promised bounds (`Clone`, `Eq`, `Serialize`, `Deserialize`, etc.), and treat missing derives or stale field shapes as current-ticket scope before finalizing the file list.

When a system ticket claims a new event-log, trace, or transition carrier, verify first whether the live canonical carrier is already ordinary `WorldTxn` event payload fields (`action_name`, tags, targets, visibility, witness data) before planning a new structured event path.

Load `references/reassessment-checks.md`. For planner-root, snapshot-completeness, planner-traceability, or AI pipeline work, also load `references/reassessment-planner-ai.md`. For golden E2E or observer-motivated tickets, also load `references/reassessment-golden.md`.

For belief-barrier or snapshot-admission tickets, explicitly classify each planner-visible carrier under audit as `authoritative local`, `belief-backed remote`, `explicit evidence`, or `out of scope` before changing code, so remote omniscience can be removed without accidentally stripping lawful local visibility.
For golden E2E tickets, explicitly decide whether reassessment disproved the ticket's invariant or only disproved the drafted proof seam. If the invariant still holds but live replanning, control flow, or timing removes the authored autonomous observation window, rewrite the ticket/spec to the strongest honest proof seam before coding instead of treating the whole contract as false.

When a ticket or spec uses generic domain language such as “affordance presence”, “local support”, or “relevant local state”, bind that phrase to the exact live carrier before coding. If the branch uses concrete place tags, workstation markers, item lots, resource sources, or another existing convention rather than a dedicated helper/type named in the prose, record that narrowing and implement against the live carrier instead of inventing a new abstraction by default.
When a golden or observer ticket targets a CLI/report pipeline that currently exists only under `src/bin/*` with no callable public helper, treat the compiled binary itself as a potentially honest E2E seam during reassessment. In that case, prefer an integration test that drives `env!("CARGO_BIN_EXE_<name>")` with temp inputs/outputs over inventing a new public helper just to satisfy the draft.
For Rust `compile_fail`, privacy, or accessor-fence tickets, verify the exact external symbol path and failure mode before trusting the drafted snippet. Confirm whether the doctest is compiled as an external crate, whether a referenced symbol is actually re-exported from the crate root or only reachable through a module path, and whether the snippet would fail open independently for each intended visibility leak rather than only when multiple symbols change together. If private-type semantics make a drafted field-access regression impossible to prove in the stated form, rewrite the ticket/spec to the strongest honest boundary before coding.

### 3. Handle mismatches explicitly

Load `references/mismatch-handling.md` when reassessment exposes a contradiction, risky ticket/code divergence, or a user decision that requires 1-3-1.

If reassessment changes a shared API, type contract, schema shape, or cross-ticket assumption, update any still-active dependent tickets/spec references in the same family before implementing or closing out the current ticket.

If reassessment exposes a separate architectural concern that must be tracked but is not honestly owned by the current ticket, create or update a dedicated follow-up ticket before proceeding, and rewrite the active ticket so that concern is referenced explicitly as an external dependency or out-of-scope blocker rather than left implicit.
If an investigation/disposition ticket concludes that the live contradiction is already owned by an existing sibling ticket, close the current ticket by recording that conclusion and updating the sibling ticket's scope/deps factually instead of creating a duplicate follow-up.
When the current ticket resolves a blocker that had previously been split out from an active sibling, immediately update that sibling ticket's `Deps`, verification contract, and blocker wording so it no longer reads as still blocked on the now-completed concern.
When that follow-up path requires creating a new ticket, read `tickets/README.md` and `tickets/_TEMPLATE.md` first and write the new ticket in full repo-ready form instead of treating it as an informal reassessment note.
When reassessment shows the blocker is a missing substrate already captured by an active draft spec, create or update a bounded implementation ticket from that spec immediately and rewrite the current ticket to depend on that implementation ticket instead of leaving the spec as an implicit blocker.
When repeated follow-up tickets in the same numbered family keep exposing the same missing contract, proof surface, or traceability substrate, stop and assess whether the remaining concern now belongs in a new spec or roadmap update rather than another local ticket.
If a golden/observer ticket exposes a concrete contradiction in the exact read/report/proof surface it is asserting against, the current ticket may absorb the narrowest production fix needed to make that surface honest. When that happens, update the ticket's `Engine Changes`, `Files to Touch`, and closeout `Deviations` instead of preserving a stale “tests only” scope.
If a ticket's manual regression step, `compile_fail` sketch, or claimed proof seam is impossible as written because of language/tooling semantics, rewrite the active ticket/spec to the strongest honest proof seam before final verification instead of preserving the stale step and only noting the mismatch in conversation.

### 4. Extract the implementation scope

Load `references/scope-extraction.md` when the owned edit surface, dependency boundary, or honest verification scope is not already clear from reassessment and ticket classification.

For derived forensic/report/read-model tickets, use this compact scope checklist before editing:
- name the authoritative inputs and trace inputs the model is allowed to read
- verify nested field trait support for the requested public type shape
- confirm the deterministic ordering/storage rule (`BTree*`, stable `Vec` order, no float math)
- separate bounded-capture/filtering policy from raw candidate collection
- identify any same-crate type fallout needed to keep the requested API honest
- if the ticket asks you to prove a derived trace/report field is copied or transformed from authoritative input, target the focused proof at the actual constructor/builder boundary even when the type is declared in a different module
- when canonical names, classifications, or display rows depend on an existing registry/catalog, verify whether the live render/helper signature must accept that input explicitly instead of assuming the change is purely local `writeln!` fallout

For small CLI/tooling tickets that touch a single binary or local helper surface, explicitly check whether the honest focused proof belongs beside the owned function/module (for example in `src/bin/*.rs` tests) rather than in a new integration-test binary. If the ticket sketches a new `tests/*.rs` file but the live seam is a local formatter/helper inside one binary, narrow the test placement to that seam and record the deviation in the ticket closeout.
When the live proof seam or owned implementation boundary differs from the ticket's drafted sketch, record the exact landed seam in the ticket closeout instead of preserving the drafted shape in prose. Small/local tickets often land through a private extracted helper or a narrower same-file formatter/test seam even when the draft described a more direct in-place edit.

For doc-only or compile-time regression tickets, use this compact scope checklist before editing:
- verify the exact external path for every symbol referenced in docs/tests (`crate_root::Type` vs `module::Type`)
- confirm whether each negative proof fails independently for the intended regression, or only when multiple symbols change together
- pair negative `compile_fail` coverage with a positive compile/runnable proof when that guards against always-pass regressions
- if language privacy or type-checking semantics block the drafted proof shape, rewrite the ticket/spec to the strongest honest seam before implementation and closeout

### 5. Implement with Worldwake discipline

Load `references/implementation-discipline.md` when reassessment shows a non-mechanical, shared-surface, or otherwise higher-risk change that needs extra guardrails. For straightforward small/local tickets whose reassessment already proved the edit path is mechanical, you may proceed without loading that reference.

### 6. Verify at the right boundary

Run the narrowest correct verification first, then broaden.

Prefer sequential `cargo` verification runs unless there is a concrete reason to parallelize them; this keeps cargo-lock contention, attribution, and close-out evidence truthful.
Do not launch multiple `cargo` commands in parallel for focused proofs or broadened verification. Run one Cargo command to completion before starting the next.

Load `references/verification.md`.

When a golden or observer ticket relies on named integration-test binaries, explicitly check whether the cited command executes the intended authored scenario cases or only compiles the binary plus non-ignored helper tests. If the motivating long-run scenario cases are still `#[ignore]`, record that distinction honestly in the ticket closeout instead of implying those scenarios ran.

When request-resolution, `tick_step`, or other authoritative failure-boundary changes move where a rejection or legality failure surfaces, expect mixed-layer fallout in AI blocker classification, planner recovery, or golden behavior. Treat at least one targeted `worldwake-ai` or golden regression check as normal before relying on full-workspace verification alone.
When a golden owns an AI-selected binding contract but a purely autonomous run no longer exposes a stable stale-request window, expect a hybrid proof seam: snapshot the AI-selected binding from decision trace, carry it through the narrowest lawful external request or harness action, set control source explicitly, and then assert the authoritative/runtime outcome. Record that narrowed proof seam in the ticket/spec instead of continuing to chase a fully autonomous scenario that the live branch cannot hold still.
When a planner-boundary fix removes an unlawful omniscient carrier, expect dependent tests to fail until remote fixtures are rewritten to seed the needed belief or evidence state explicitly. Treat that as normal fallout to audit, not automatic proof that the production fix is wrong.
When broadened verification exposes intentionally staged unused private surface (for example new enum variants, helper entry points, or report fields that sibling tickets will start using later), the current ticket may absorb the narrowest local lint-safe annotation or cleanup needed to keep CI-matching verification green. Record that staged-state deviation explicitly in the ticket closeout instead of silently treating it as unrelated noise.

### 7. Close out the ticket honestly

Load `references/closeout.md`.

Before closing out, re-read any ticket claims about optional rendering, disabled flags, or suppressed sections and confirm the landed behavior matches those claims exactly. If the ticket distinguishes between “render empty-state” and “omit entirely when disabled,” make sure at least one focused assertion proves that exact disabled-path contract before marking the ticket complete.
If reassessment or verification narrowed the owned seam, introduced a private helper, or required narrow same-ticket lint cleanup to support staged infrastructure, say so explicitly in the ticket's `Deviations`/`Outcome` notes rather than leaving the drafted implementation sketch as the only recorded shape.
If broadened verification proves the landed contract differs from the drafted invariant or top-level ticket summary, update the ticket's header fields and any now-false acceptance or invariant wording as part of closeout instead of recording the difference only under `Deviations`.

### 8. Close the loop on the ticket

Covered in `references/closeout.md` (Step 8 section).

## Guardrails

- Name exact files, symbols, layers, and invariants for non-trivial claims.
- Treat tests, traces, event logs, and authoritative state as different proof surfaces.
- Architectural contradictions: solve or escalate with 1-3-1 (see `references/mismatch-handling.md`, Escalation decision tree). Do not patch around them.
- For focused test commands, verify that the selector actually proves the owned surface. Substring filters can run extra tests or, for integration-test binaries, compile the target while executing zero tests. When exactness matters, prefer the narrowest truthful selector such as an exact unit-test name or `cargo test -p <crate> --test <file_stem>` for integration-test binaries instead of a loose name filter. `--exact` may require the fully qualified module path rather than the bare function name, including ordinary lib tests and `src/bin/*.rs` unit tests (for example `agent_tick::planning::tests::my_case` or `tests::my_case`); check `cargo test ... -- --list` before recording the command in ticket closeout.

## Example Usage

```
/implement-ticket tickets/LEGACTTOO-009*
/implement-ticket tickets/FITLSEC7RULGAP-001*
/implement-ticket .claude/worktrees/my-feature/tickets/FOO-003*
```
