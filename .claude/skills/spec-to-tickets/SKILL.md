---
name: spec-to-tickets
description: Break a spec into actionable, detailed tickets aligned with FOUNDATIONS.md. Use when asked to decompose a spec into tickets.
---

# Spec to Tickets

Break a numbered spec into a series of small, actionable implementation tickets.

## Invocation

```
/spec-to-tickets <spec-path> <NAMESPACE>
```

**Arguments** (both required, positional):
- `<spec-path>` — path to the spec file (e.g., `specs/S05-merchant-stock-storage-and-stalls.md`)
- `<NAMESPACE>` — ticket namespace prefix (e.g., `S05MERSTOSTALL`)

If `<NAMESPACE>` is omitted, propose one derived from the spec number and abbreviated title (e.g., `S102-frontier-aware-exploration.md` → `S102FROAWAEXP`). Ask the user to confirm or override before proceeding. If `<spec-path>` is missing, ask the user to provide it before proceeding.

## Process

Follow these steps in order. Do not skip any step.

### Step 1: Mandatory Reads

Read ALL of these files before any analysis:

1. **The spec file** (from argument 1) — read the entire file
2. **`tickets/_TEMPLATE.md`** — the canonical ticket structure; every ticket you produce must follow this template exactly
3. **`tickets/README.md`** — the ticket authoring contract; understand the required sections and checks
4. **`docs/FOUNDATIONS.md`** — architectural commandments; every ticket must align with these principles. Skip if read earlier in this session and not modified since. If the file exceeds the Read tool's token limit, read the preamble and principle listing (first ~200 lines) using offset/limit, which covers the non-negotiable principles. The summary in `CLAUDE.md` (available via system context) supplements the direct read.
5. **`docs/precision-rules.md`** — precision rules for technical claims; governs Assumption Reassessment and Verification Layers sections. Skip if read earlier in this session and not modified since.

### Step 2: Codebase Validation

Before decomposing, validate the spec's assumptions against the actual codebase:

- **Grep/Glob** for file paths mentioned in the spec — confirm they exist
- **Grep** for types, functions, and modules the spec references — confirm they are real and current
- **Flag** any stale assumptions, missing files, or renamed entities
- For specs adding **new universal agent components**, grep `create_agent` in `crates/worldwake-core/src/world.rs` to check whether the bootstrap path seeds default values for existing universal profiles. If it does, the new component must also be seeded there, and the `world_txn.rs` `create_agent()` delta assertion must be updated. Include both in the ticket's Files to Touch and What to Change.
- If you find discrepancies, present them to the user before proceeding
- If `/reassess-spec` was run on this spec in the current session and all findings were resolved, Step 2 validation may be abbreviated to a spot-check of key references rather than a full re-validation. 3–5 targeted greps is sufficient. An abbreviated spot-check should verify at least:

  - **(a) Primary references**: the spec's primary type/function references still exist at the stated paths.
  - **(b) Serialization**: if the spec modifies serialized state or save/load paths, `SAVE_FORMAT_VERSION` hasn't changed since reassessment. Skip this check for specs that don't touch serialization.
  - **(c) Sibling specs**: no new files in `specs/` reference the same types.
  - **(d) Struct construction sites**: for specs that add fields to existing structs, grep for struct literal construction sites (e.g., `StructName {`) to verify all construction sites are accounted for in the tickets' Files to Touch. If the construction site count is high (>20), note the count in the Step 4 summary table's Notes column justification and consider whether the field-addition ticket should be split by crate. A high construction site count typically elevates effort from Small to Medium. When the raw construction-site count exceeds 50, run a spread-syntax spot-check before finalizing Large vs Medium effort: `rg -B 1 -A 1 '<StructName>\s*\{' <crate-path> | head -50` (or equivalent). If most sites use `..Default::default()` / `..StructName::default()` / spread syntax AND the added fields all have meaningful `Default` values, the count is informational — effort classification tracks semantic complexity rather than site count. If most sites explicitly enumerate fields without spread syntax, OR the added fields lack trivial defaults (so construction sites must supply values explicitly), the count is load-bearing and effort tracks it directly (>50 → Medium, >100 → Large). For counts above ~100 sites with no spread-syntax escape hatch, Large effort is typically warranted even when the work is mechanical, because review cost scales with touched-file count and the workspace-builds-after-each-ticket constraint prevents splitting. For specs adding fields to `AgentBeliefStore`, also check `BeliefStoreDiff` in `crates/worldwake-core/src/belief.rs` — it captures structural diffs for event-log delta compaction and may need corresponding diff fields for the new data.
  - **(e) Universal agent components**: for specs adding new universal agent components, verify whether `World::create_agent()` seeds defaults for existing universal profiles — if so, include the bootstrap seeding and `world_txn.rs` delta assertion update in the component-registration ticket.
  - **(f) Existing tests**: for specs modifying existing functions, grep the target module's `#[cfg(test)]` block for test names exercising the changed functions (not just sibling `tests/*.rs` files — the target module's own inline tests matter too). Record existing test names for inclusion in ticket Assumption Reassessments during Step 5 — this moves discovery to the validation phase where it's sequential, so results are available during parallel ticket writing.
  - **(g) Rename/removal blast radius**: for tickets that rename or remove a type, function, field, or schema-generated accessor, grep workspace-wide (`crates/`, `tests/`, and any other Rust source roots) for every symbol being renamed or removed — including macro-generated accessor variants (e.g., `get_component_X`, `insert_component_X`, `iter_Xs`, `entities_with_X`, `query_X`, `count_with_X`, plus struct/enum name and any value-wrapping helper like `sample_X`). Count sites per crate. If any crate has >0 sites but is not in the ticket's Files to Touch, either (i) add the crate's files to Files to Touch, or (ii) split those sites into a dedicated follow-up ticket with an explicit dependency. Common miss targets: `worldwake-sim`, `worldwake-systems`, `worldwake-cli/src/bin/observer.rs`, `worldwake-cli/src/handlers/*`, and every `tests/` directory across the workspace. The per-crate count is what distinguishes a renamed-type ticket from a net-new-type ticket — the former always has a blast radius, the latter typically doesn't.

  After the spot-checks, render the exercised sub-checks as a compact inline list before moving to Step 3 (e.g., `Spot-checks: (a) ✓, (b) ✓, (c) skipped — no new sibling specs, (d) 164 sites flagged, (e) N/A, (f) ✓, (g) N/A — no renames`). This proves each applicable sub-check ran and surfaces N/A cases explicitly.
- If `/reassess-spec` was run but some findings were deferred by the user, treat deferred items as out-of-scope for ticket decomposition. Note them in the Step 6 final summary as "deferred reassessment findings that may warrant separate tickets." Do not silently incorporate deferred findings into ticket scope.

### Step 3: Decompose the Spec

Analyze the spec and identify discrete work units:

- Each ticket must represent a **reviewable diff** — small enough for comfortable manual review
- Map **dependencies** between tickets (which must be done before which)
- Determine **priority ordering** (what to implement first)
- Ensure **every spec deliverable is covered** — no silent skipping. If a deliverable seems wrong or unnecessary, flag it to the user using the 1-3-1 rule instead of omitting it. Deliverables that explicitly state no changes are needed (e.g., "No new profile", "No new components") do not require tickets. Note their existence in the Step 4 summary if non-obvious.
- **Distributed deliverables**: when a deliverable's implementation is structurally inseparable from other deliverables (e.g., a cross-cutting read-access pattern that lands naturally inside multiple feature tickets, or a conceptual distinction whose mechanics materialize across several tickets), mark it "distributed" rather than either creating a redundant ticket or silently omitting it. In the Step 4 Scope column, name the distributed deliverable alongside the primary scope (e.g., `Scope: D3+D4, plus D8 (distributed — cross-ticket)`). In the Step 6 coverage mapping, use `D8 → 003+005+007 (distributed — no dedicated ticket; rationale: <one-line reason>)`. A distributed deliverable is never a substitute for silent skipping — the rationale (why dedicated-ticket placement would be redundant or non-compile-safe) must be stated, and every ticket that contributes to the deliverable must have the deliverable number in its Scope column.
- **Placeholder-replace pattern**: when a ticket defines a stub, baseline, or simplified path that a later ticket must replace to complete the semantic contract (common for compile-safe intermediate states where the real logic depends on later-ticket scope), make the relationship explicit in both directions: (a) name the placeholder in the earlier ticket's What to Change as "placeholder, replaced by ticket <later>" with a one-line description of what the placeholder does vs. what the real implementation will do, (b) list the replacement target as an Out of Scope item in the earlier ticket referencing the later ticket, and (c) reference the placeholder in the later ticket's What to Change with text like "Update ticket <earlier>'s placeholder `<symbol>`". This prevents reviewers of the earlier ticket from misreading the stub as the final contract and makes the dependency visible in both directions of the ticket sequence.
- Consider natural boundaries: type changes, new modules, test suites, integration points. When tests exercise multiple deliverables simultaneously and cannot be split per-deliverable, a single test ticket depending on all implementation tickets is a valid decomposition. Note the multi-dependency in the Step 4 summary
- When multiple spec deliverables share the same file set and cannot compile independently, merge them into a single ticket. Note merged deliverables in the Step 4 summary table notes
- Use the spec's "What This Does NOT Change" or equivalent non-goals section to populate tickets' Out of Scope fields — these are pre-validated non-goals from reassessment
- Ensure **workspace builds after each ticket** — if removing types/functions from a shared crate, all consumers must be updated in the same ticket. Splitting a migration across tickets is only valid when intermediate states compile.
- When all deliverables modify the **same file**, decompose by logical section or feature, not by file boundary. Each ticket targeting a different section of the same file is a valid reviewable diff. Note the shared file in the Step 4 summary rather than repeating it per-ticket.
- For **mechanical refactoring specs** (trait decomposition, enum splitting, interface migrations), recognize that multiple tickets may share an identical file set. In the summary table (Step 4), note the shared file set once and reference it from each ticket rather than forcing each ticket to independently discover the same list. Individual tickets should list only *additional* files beyond the shared set.

### Step 4: Present Summary for Approval

**Before writing any ticket files**, present a numbered summary table:

```
| # | Ticket ID | Title | Scope | Effort | Deps | FND | Notes |
|---|-----------|-------|-------|--------|------|-----|-------|
| 1 | <NS>-001  | ...   | <5-10 word scope> | Small  | None | — | — |
| 2 | <NS>-002  | ...   | <5-10 word scope> | Medium | 001  | P12,P27 | 34 construction sites |
| ...
```

Column roles:

- **Title** — human-readable ticket name (e.g., "Anomaly infrastructure and multi-agent rendering"). Matches the first-line `# <PREFIX-NNN>: <title>` that will be written to the ticket file in Step 5.
- **Scope** — deliverable mapping (e.g., "D1+D6") or acceptance surface (e.g., "adds detector X, emits anomaly Y"). Title and Scope should NOT duplicate each other; Scope answers "which spec pieces does this cover?" while Title answers "what is this ticket called?".
- **FND** — optional; populate only for tickets with notable FOUNDATIONS concerns (e.g., a ticket touching derived views should note P27). Use `—` for tickets with no specific concern.
- **Notes** — construction site counts, merged deliverables, shared file sets, or other decomposition-relevant details that don't fit in other columns.

If all tickets are independent, state this once rather than repeating `None` in every Deps cell. The Step 6 dependency graph can be a single sentence (e.g., "All tickets are independent — implement in any order").

**Wait for user approval or adjustments.** Do not write files until the user confirms.

**Auto mode interaction**: When auto mode is active and Step 2 validation surfaced no Issues (no stale references, no missing files, no renamed symbols, no deferred reassessment findings), auto-approve the summary table and proceed to Step 5. Announce the auto-approval inline (e.g., "Auto mode: no Issues surfaced in Step 2, proceeding to Step 5"). If Step 2 surfaced any Issue, the wait-for-user gate applies even in auto mode.

### Step 5: Write Ticket Files

For each approved ticket, write a file to `tickets/<NAMESPACE>-<NNN>.md` using the **exact structure** from `tickets/_TEMPLATE.md`.

Every ticket MUST include:

- **Status**: PENDING
- **Priority**: HIGH / MEDIUM / LOW (based on dependency order and criticality)
- **Effort**: Small / Medium / Large
- **Engine Changes**: None or list of affected areas
- **Deps**: Other tickets or specs this depends on
- **Problem**: What user-facing or architecture problem this solves
- **Assumption Reassessment**: Assumptions validated against current code (use today's date). Include items 1-3 from the template (always required) plus any domain-specific items from items 4-15 that match the ticket's scope. Omit inapplicable items silently — do not pad with "N/A" boilerplate.
  - **Renumber surviving items sequentially starting from 4**: items 1–3 are always required; items 4–15 are a menu, not a fill-in form. After selecting which menu items apply, renumber them sequentially — do not preserve the template's gaps. Example: if template items 1, 2, 3, and 14 apply to the ticket, the produced ticket lists them as `1, 2, 3, 4` (the former item-14 body becomes item 4). Lists like `1, 2, 3, 14` or `1, 2, 3, 5, 15` are malformed output and will need rework.
  - For tickets that modify behavior tested by existing focused/unit tests, grep the target module's `#[cfg(test)]` block for test names exercising the changed function or type. Name existing tests in the Assumption Reassessment and adjust the Test Plan accordingly (see `docs/precision-rules.md` Rule 3).
  - For pure structural refactoring tickets (no behavioral changes, no new actions/components), items 1-3 may be satisfied concisely by confirming: (a) the symbols being moved exist at stated locations, (b) the impl block count matches the spec's claim, (c) the shared boundary is the trait/struct under edit. Items 4-15 are typically all inapplicable for structural refactors.
  - For observer-only, CLI-only, or tooling-only specs (no engine changes, no simulation state mutations), items 1-3 are typically sufficient — items 4-15 apply only when the ticket touches simulation runtime, planning, or action systems.
  - When a spec proposes refactoring an existing function to delegate to a new superset method, verify that the delegation doesn't widen the original function's semantic contract. If the new method returns values for inputs the original explicitly handled as `None` or didn't cover, the refactoring must preserve the original's narrower scope — delegate only for the overlapping subset and document which goals/variants are intentionally excluded from delegation.
- **Architecture Check**: Why this approach is clean, how it preserves agnostic boundaries
- **Verification Layers**: Map each invariant to its proof surface (for mixed-layer or cross-system tickets: decision trace, action trace, event-log delta, authoritative world state)
- **What to Change**: Numbered sections with specific implementation details
- **Files to Touch**: Exact paths validated against the codebase (new or modify). When a ticket adds fields to an existing struct, grep for struct literal construction sites across the workspace during ticket writing and include all affected files. See `tickets/README.md` check #13 for known macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`)
- **Out of Scope**: Explicit non-goals — what this ticket must NOT change
- **Acceptance Criteria**:
  - **Tests That Must Pass**: Specific behavior tests
  - **Invariants**: Must-always-hold architectural and data contract invariants
- **Test Plan**:
  - **New/Modified Tests**: Paths with rationale
  - **Commands**: Targeted test commands and full suite verification

#### Batch All Write Calls in One Message

Send **one assistant message containing one Write tool call per ticket**, not N sequential messages. Each ticket file write is independent because each creates a new file, so they can run concurrently. A single message with N Write calls costs one round-trip; N sequential messages cost N round-trips. Compose every ticket's full content first, then emit all Write calls in one batch.

For spec decompositions producing large tickets (individual contents >5KB), the rule still applies — the composition cost is paid once regardless of message count, so the latency saving is the dominant factor.

After the parallel batch returns, verify every ticket file was created. If any Write call failed (typo in path, permissions error, or other I/O failure), retry that ticket with the corrected argument immediately — do not proceed to Step 6 until all ticket files exist at their intended paths.

### Step 6: Final Summary

After writing all files:

1. **Verify cross-ticket dependency consistency**: For each `Deps` reference, confirm the depended-on ticket actually produces what the dependent ticket needs (types, modules, files). If a dependency is broken (e.g., ticket 005 depends on a type from 003 but 003's scope doesn't define it), flag the inconsistency.

2. **Deliverable coverage mapping**: List each spec deliverable and the ticket that covers it (e.g., `D1→001, D2→001, D3→002`). Verify all spec deliverables are accounted for. If any deliverable is missing, flag it before finalizing. If the spec uses phases or named sections instead of numbered deliverables (e.g., `Phase 2a`, `Layer 0`), adapt the coverage mapping to use the spec's organizational scheme — the `D1→001` format is illustrative, not prescriptive.

3. List:
   - All ticket files created
   - The dependency graph (which tickets block which)
   - Suggested implementation order

Do NOT commit. Leave files for user review.

## Constraints

- **FOUNDATIONS alignment**: Every ticket must respect the principles in `docs/FOUNDATIONS.md` (maximal emergence, belief-only planning, system decoupling, no backward compatibility, etc.)
- **Template fidelity**: Every ticket must use the `tickets/_TEMPLATE.md` structure exactly — no ad-hoc sections or missing required fields
- **Ticket fidelity**: Never silently skip a spec deliverable. If something seems wrong, use the 1-3-1 rule (1 problem, 3 options, 1 recommendation) and ask the user
- **Codebase truth**: File paths and type references in tickets must be validated against the actual codebase, not assumed from the spec
- **Reviewable size**: Each ticket should be small enough to review as a single diff. When in doubt, split further
- **Explicit dependencies**: Use the `Deps` field to declare inter-ticket dependencies; never leave implicit ordering
- **Performance-optimization specs**: For P12-type specs that compress computation without changing world meaning, tickets should include regression guard acceptance criteria with concrete metric thresholds. The metric should match what the spec actually optimizes — e.g., candidate count per expansion, expansion count, branching factor, or wall-clock time — not necessarily wall-clock time alone, which may be non-deterministic or platform-dependent. Include regression guard commands (e.g., soak seed runs, candidate-count assertions in golden tests). Acceptance criteria should distinguish correctness (golden tests pass) from performance (metric thresholds met)
