# Phase 4: DETECT — Two Parallel Lenses

Run Lens A and Lens B in parallel. For large analyses (>100 modules), delegate each lens to separate Explore sub-agents.

## Lens A: Structural Scatter

Within the exercised modules, find cross-cutting concept clusters with structural debt signals.

**Step 1 — Find Concept Clusters**:

1. **Filename clustering** (primary method): Group exercised modules by dominant concept in their filename (e.g., `patrol.rs`, `patrol_actions.rs` -> "patrol" cluster; `goal_policy.rs`, `goal_model.rs`, `goal_dispatch_key.rs` -> "goal" cluster). Module filenames in this codebase reliably reflect concept boundaries.
2. **Import clustering** (supplement): For modules with generic names, scan their `use` statements and key `pub` exports to assign them to a concept cluster.
3. **Enum-centered clustering**: Grep for key enums (`pub enum`) in the exercised modules. Any enum whose variants are referenced in 3+ files is a cluster seed — name the cluster after the enum.
4. Name each cluster by its dominant concept fragment.
5. Filter to clusters exceeding the file-count threshold: >10% of analyzed files, or 5+ files, whichever is larger. For small analyses (<30 modules), use 5+ files as the floor.
6. Merge clusters with >50% module overlap.

**Step 2 — Quick Triage**:

If a cluster's symbols are predominantly single-component accessors (`get_component_*`, `effective_place`, `possessor`, `ground_location`, `commodity_quantity`, or similar read-only queries) and no enum in the cluster is being matched in multiple files, mark the cluster as "Acceptable — fundamental accessor" in the report without detailed signal measurement. Still note the cluster name and file count for completeness. Fundamental accessors appear in many files by design — the triage saves reporting cost, not detection cost.

**Step 3 — Measure Structural Signals**:

For clusters that did not early-exit, compute these metrics:

| Metric | How to measure | Signal strength |
|--------|---------------|-----------------|
| **File count** | Distinct source files containing the concept | Baseline — spread indicates cross-cutting concern |
| **Scattered match arms** | Grep for `match` expressions on the cluster's key enums; count files with similar-but-not-identical match logic on the same enum | **Strong** — callers re-deriving meaning the enum should carry |
| **Repeated predicate patterns** | Grep for recurring combinations of `has::<T>()`, `get::<T>()`, `.is_some()`, `.map()`, `matches!()`, `if let` chains that check the same component set in 2+ locations | **Strong** — missing derived/cached concept |
| **Cross-crate spread** | Count distinct crates (`worldwake-core`, `-sim`, `-systems`, `-ai`) containing the concept | **Moderate** — 3+ crates suggests boundary misplacement |
| **Derived state recomputation** | Functions that compute the same derived value from the same inputs in different modules (same parameter types, same return semantics, different call sites); same computation from same `&self` fields in multiple `impl` blocks | **Strong** — should be stored as state or a method on the type |
| **Clone-like redundancy** | Near-duplicate `impl` blocks or free functions with same structure, different concrete types | **Strong** — missing generic or trait abstraction |
| **Option lifecycle smell** | `Option<T>` fields modelling implicit phases; multiple `Option` fields that are correlated (if A is Some, B must be Some) without a state enum | **Strong** — missing lifecycle type |
| **Workaround indicators** | Grep for `// workaround`, `// hack`, `// TODO`, `// FIXME`, `// safety net`, `// fallback`, `// temporary`, `// HACK` (case-insensitive) | **Direct evidence** |

Three counts per cluster: defining files, consumer files, temporally-coupled files.

**Step 4 — Scenario Grounding**:

For each cluster, check whether it maps to at least one scenario family from Phase 2. A cluster that cannot explain any test scenario is demoted to "Needs Investigation" regardless of signal strength.

**Tool usage**: Glob for filenames, Grep for `pub enum` definitions, `match` expressions, predicate patterns, workaround comments. Read specific functions for manual comparison when grep finds potential matches.

## Lens B: Architectural Fractures

Use the temporal coupling matrix from Phase 1 to prioritize which boundary files to read. Start with the top 5 cross-crate file pairs by co-change frequency.

Scan the exercised code for these 8 fracture types:

| # | Fracture Type | What to look for | Rust-Specific Signals |
|---|--------------|-----------------|----------------------|
| 1 | **Split protocol** | The legal sequence of interactions is spread across multiple modules/crates. Module A decides "what", module B decides "when", module C decides "whether". | Trait implementations scattered across modules; a trait's methods implemented in different crates/modules. |
| 2 | **Authority leak** | Multiple modules write the same truth. Two or more places create/mutate/invalidate the same piece of state. | Multiple modules constructing/modifying the same struct; `pub` fields allowing uncontrolled mutation from outside the owning module. |
| 3 | **Projection drift** | Derived summaries or cached computations are recomputed everywhere. No single module owns the projection. Detect at ALL scales (intra-subsystem and cross-subsystem). | Same derived value computed from same struct fields in different modules (no shared method or associated function). |
| 4 | **Boundary inversion** | Higher crates own rules that belong in lower crates. `worldwake-ai` enforces what `worldwake-core` or `worldwake-systems` should prevent. | Higher-level crate enforcing invariants that should be in a lower-level crate's type system. |
| 5 | **Concept aliasing** | The same domain concept exists under different names/types in neighboring crates. | Type aliases or newtypes for the same concept with different names in different modules. |
| 6 | **Hidden seam** | Files across nominal crate boundaries repeatedly change together in git history, suggesting they belong in the same module or crate. | Language-agnostic — git co-change analysis. |
| 7 | **Overloaded abstraction** | One type/module carries several lifecycle roles that should be separated. | Enum with too many variants serving different lifecycle purposes; struct with fields for multiple disjoint use cases. |
| 8 | **Orphan compatibility layer** | A shim, fallback path, or "safety net" handler exists only to mask a deeper missing abstraction. | Wrapper types, `From`/`Into` impls, or compatibility modules that exist only to bridge a missing abstraction. |

**Evidence rule**: A fracture is NOT reported in main findings unless supported by at least two independent signals (e.g., import analysis + temporal coupling, or naming similarity + assertion patterns). Single-signal fractures go in "Needs Investigation."

**Rust-specific considerations**:
- **Trait coherence and orphan rules** create forced boundaries that may mask or create fractures.
- **Ownership and borrowing patterns** — authority confusion may manifest as lifetime complexity or excessive `.clone()` calls.
- **Module visibility (`pub`, `pub(crate)`, `pub(super)`)** — the visibility system IS the boundary system; use it for boundary analysis.
- **Crate boundaries** — in this multi-crate workspace, crate boundaries are the primary subsystem boundaries.
- **Derive macros and procedural macros** may hide structural patterns from grep-based detection.

**Sub-agent delegation**: For large exercised sets (>100 modules), delegate to 2-3 parallel Explore sub-agents, each analyzing a different crate boundary surface (e.g., ai<->sim, ai<->core, sim<->systems).

**Tool usage**: Grep for shared type names across crates, Grep for duplicate predicate patterns, Grep for `pub enum` and `match` expressions crossing module boundaries, Read key functions at boundary points, Bash for `git log` co-change analysis.

## Sub-agent Briefing Template

When delegating Lens A or Lens B to Explore sub-agents, each prompt must be self-contained — sub-agents have no prior context. Structure the prompt as:

1. **Context**: One sentence on what the analysis is about and which test file/directory triggered it.
2. **Exercised module list**: The key source modules from Phase 1 that the sub-agent should analyze, grouped by crate. Include file paths.
3. **Entry-point functions and types**: The specific functions and types the tests call directly (from Phase 1 symbol tracing).
4. **Scenario families**: The behavioral families from Phase 2, so the sub-agent can ground its findings.
5. **Search targets**: The specific structural signals (Lens A) or fracture types (Lens B) to check, with concrete grep patterns or file-reading instructions tailored to the exercised modules. Do not just repeat the generic signal table — translate it into actionable searches for this specific analysis.
6. **Thoroughness and output format**: Request "very thorough" exploration. Ask for file paths, line numbers, and counts for each signal found.
