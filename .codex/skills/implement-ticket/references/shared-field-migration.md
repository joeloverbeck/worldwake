# Shared Field Migration

Use this reference when a ticket adds a field to a shared struct, component,
record, payload, schema type, authored scenario type, or serialized surface.

## Pre-Sweep

Before the first focused proof, sweep every live construction shape that can
break or silently inherit the new field:

- exact explicit literals such as `Type {`
- public constructors such as `Type::new(`
- helper builders or fixture factories that wrap the type
- destructuring patterns such as `Type { field_a, field_b }`
- downstream `src/bin/*`, integration tests, generated-doc tooling, and sibling
  crates when the type is library-visible

Classify each match before editing:

- full explicit literal: add the field unless the ticket changes the canonical
  construction path instead
- spread literal using `..Type::default()` or another struct update: leave it
  alone unless the ticket changes default semantics
- constructor/helper call: update it only if the signature changes or the helper
  owns the new field
- destructuring pattern: add the field, use an existing rest pattern style, or
  rewrite the destructure only if the test/proof does not own full enumeration

If the explicit-literal sweep is empty, or the remaining matches are clearly
non-owning helpers/fixtures, inspect the canonical constructor/helper/builder
path next before broadening the fallout claim. When the live branch centralizes
creation through `Type::new(...)`, builder methods, or another narrow seam,
rewrite the ticket to that truthful constructor-owned boundary instead of
preserving a drafted repo-wide literal-migration story.

## Authored Scenario And Generator Surfaces

When the shared type is an authored scenario/test-facing definition such as
`AgentDef` or `PlaceDef`, pre-sweep same-crate helpers, handlers, lints,
destructuring/report helpers, and `#[cfg(test)]` modules that build it
explicitly before trusting a narrow focused test.

When that authored scenario/test-facing type is also used across crate
boundaries or downstream generators, extend that pre-sweep before the first
broadened proof run. Search for explicit literals, destructuring patterns, and
helper constructors in those downstream surfaces up front instead of letting
generator or integration-test fallout appear only at the end.

Apply constructor fallout with precise patches or syntax-aware edits. Do not use
broad text rewrites that can match type definitions, snippets, or unrelated
same-shaped blocks; after each mechanical pass, re-scan the touched files for
accidental insertions before compiling.

When adding a new authored scenario/schema field, also search live
coverage/catalog/report generators for the new field name before finalizing
scope, such as `scenario_coverage`, golden inventory, or feature catalogs.
Decide whether the new field is `mapped now`, `intentionally unmapped until
authored`, or `follow-up required`, and record that decision in ticket closeout
when the generator boundary is relevant. If the decision is `mapped now`, prove
that mapping with a focused generator/report test or a cleanup-safe synthetic
authored fixture that sets the new field; a broad `--check` over existing
committed scenarios is not sufficient when no current scenario authors that
field.

## Derives, Views, And Wrappers

Before finalizing a supposedly runtime-only or local-only field addition on a
shared type, inspect the enclosing type's live derives, trait bounds, and any
round-trip or serialization tests already attached to it. If the host type
already derives `Ord`, `PartialOrd`, `Serialize`, `Deserialize`, or similar broad
bounds, treat satisfying that existing derive surface as current-ticket scope
for the new field types before the first compile rather than discovering it only
from compile fallout.

When a ticket adds a runtime report, forensic surface, or other derived
read-model type, verify the requested trait/derive surface up front on the live
branch rather than trusting the ticket sketch. Check whether every nested field
already satisfies the promised bounds (`Clone`, `Eq`, `Serialize`,
`Deserialize`, etc.), and treat missing derives or stale field shapes as
current-ticket scope before finalizing the file list.

When a ticket adds a field to a shared record, payload, or schema type, sweep any
live trait/view/builder/wrapper surfaces that expose or mirror that record, such
as `*View` traits, report rows, pending-record wrappers, or renderer inputs. If
leaving those surfaces unchanged would make the new field unreachable or
dishonest through the current abstraction boundary, extend them in the same
ticket instead of treating the field addition as self-contained.

When that new field's type is referenced from nested Rust `#[cfg(test)]` modules
or other nested scopes, verify whether the added constructor fallout also needs
a local import or a fully qualified path there. File-scope imports often do not
reach the actual literal site that now mentions the new shared type.

When a staged/shared-surface ticket or spec includes code snippets, sample
literals, or focused-test examples, verify every drafted enum variant, constant,
helper, and symbol against the live branch before copying the snippet into
implementation or proof. Treat stale snippet names as reassessment drift and
correct them before coding rather than discovering them only from compile
fallout.

When a staged ticket adds a new trace/report/schema field to a shared struct,
verify the visibility boundary separately for the outer field and the nested
payload types. Existing external helpers may need the field to remain
constructible at the wider visibility tier even while the inner payload taxonomy
stays crate-local or otherwise narrower; do not assume the new nested types must
inherit the outer field's visibility automatically.

When a ticket adds historical/provenance state to an authoritative store,
explicitly separate the consumers that should retain that history from the
derived summary/read-models that must stop treating it as current truth. Name
both sides before editing, then prove both: the historical/reporting surface
still exposes the new state, and the ordinary current-state summary ignores it
where required.
