# Scope Extraction

How to turn the ticket into a concrete task list (Step 4).

## Core process

Turn the ticket into a concrete task list from `What to Change`, `Acceptance Criteria`, and reassessment findings.

Separate:
- required in-scope work
- blocked work needing user direction
- explicit out-of-scope work

When the ticket inherits broader spec language, distinguish the end-state architecture claim from the narrower contract this ticket owns after reassessment.

When the parent spec describes an eventual causal story but the current ticket only owns substrate or maintenance scaffolding, keep those separate explicitly. Narrow the ticket to the current owned mechanism and name the deferred downstream behavior.

When a ticket bundles multiple deliverables and reassessment narrows the ticket to only one lawful slice, verify every removed deliverable is still owned by an existing active ticket. Create the follow-up ticket before coding if any removed deliverable has no live owner (see `mismatch-handling.md`, Escalation decision tree, for follow-up guidance).

If the ticket's requested invariant exposes a production contradiction, correct the scope first.

## Golden scope narrowing

- Remove duplicate proof unless the new scenario proves a materially different contract.
- If a proposed invariant is real at lower layers but not stably exposable as a golden, narrow to the durable golden slice and preserve the lower-layer proof as authoritative.
- When a golden ticket mixes valid negative coverage gaps with an over-claimed positive proof, preserve the honest golden slice and correct the ticket.
- Allow different proof depths per scenario (decision trace, action trace, authoritative state) rather than flattening to uniform assertion style.

## Derived forensic/report/read-model tickets

Use this compact checklist before editing:

- name the authoritative inputs and trace inputs the model is allowed to read
- verify nested field trait support for the requested public type shape
- confirm the deterministic ordering/storage rule (`BTree*`, stable `Vec` order, no float math)
- separate bounded-capture/filtering policy from raw candidate collection
- identify any same-crate type fallout needed to keep the requested API honest
- if the ticket asks you to prove a derived trace/report field is copied or transformed from authoritative input, target the focused proof at the actual constructor/builder boundary even when the type is declared in a different module
- when canonical names, classifications, or display rows depend on an existing registry/catalog, verify whether the live render/helper signature must accept that input explicitly instead of assuming the change is purely local `writeln!` fallout
- for UI/visualizer snapshot fields derived through AI or belief helpers, list the runtime context the helper actually reads (belief store, current tick, scheduler active actions, action definitions, recipe registry if relevant) and make that context part of the owned scope when the final rendered value would otherwise be misleading

## Doc-only or compile-time regression tickets

Use this compact checklist before editing:

- verify the exact external path for every symbol referenced in docs/tests (`crate_root::Type` vs `module::Type`)
- confirm whether each negative proof fails independently for the intended regression, or only when multiple symbols change together
- pair negative `compile_fail` coverage with a positive compile/runnable proof when that guards against always-pass regressions
- if language privacy or type-checking semantics block the drafted proof shape, rewrite the ticket/spec to the strongest honest seam before implementation and closeout

## Type-change scope

When shared types change, include the sweep surfaces from the reassessment checks ("Shared type, serialization, and persisted-shape sweep") in the task list.

- Before editing, run a concrete constructor/shape sweep for the changed type across workspace crates (e.g., `rg -n 'BlockedIntent \{' crates`), then rerun after implementation. Treat raw grep hits as candidate sites only: confirm whether each hit is a full manual literal, a partial `..Default::default()` literal, or just a type/impl definition before broadening the patch list, and use compiler fallout to validate the remaining real edit surface.
- For broad shared-struct shape changes, landing the shared type first and using sequential `cargo build` / `cargo test` compile failures to enumerate remaining fallout is acceptable.
- After the first compile wave identifies pure missing-field fallout, a bounded mechanical patch across remaining literals is acceptable before rerunning compile verification.
- Do not treat `cargo build --workspace` alone as exhaustive fallout enumeration for shared-shape changes. Test-only constructors, helper factories, and same-crate test modules can stay hidden until `--all-targets` compilation (e.g., `cargo clippy --workspace --all-targets -- -D warnings`). Include an all-targets verification pass before closing the ticket.
- When behavior moves between carriers, rewrite setup paths onto the new authoritative carrier rather than only deleting the stale field.
- When a constructor begins seeding defaults it previously omitted, reassess tests proving "missing component" behavior.
- When new components participate in persisted world state, expand save/load fixture builders so persistence tests actually serialize and deserialize the new components.

## Component registration scope

Distinguish:
- the authoritative schema declaration
- live macro-expansion sites or generated API surfaces
- runtime code-generation sites requiring the bare type in scope
- test-only helper or manifest sites mirroring the component set
- universal/bootstrap seeding paths when the new component is supposed to exist by default on newly created entities

Verify actual local type use before adding imports.

## Trait surface scope

See reassessment checks, Trait surface checks for detailed checks. Additional scope decisions:
- When the named trait is already a stable consumer-facing facade, reassess whether the lawful cleanup is to preserve that facade and decompose only the implementation path beneath it.
- When reassessment exposes multiple ownership shapes for a new API, decide the shape before broad implementation.
- When a widely used helper or wrapper appears to need a signature change, verify whether it is actually the live production boundary or mainly a test/unfiltered convenience surface. Prefer widening only the narrower production entry point when possible.

## Staged work

- Temporary duplicated logic is acceptable only if a named follow-up ticket owns the caller-rewire or old-path removal. State this boundary explicitly.
- When a ticket describes itself as "pure additions," verify whether an internal helper refactor is needed. If so, correct `Engine Changes`, `Architecture Check`, and `Files to Touch`.
