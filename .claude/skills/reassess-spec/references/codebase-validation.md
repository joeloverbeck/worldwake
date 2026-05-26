# Codebase Validation (Step 3)

Validate every reference from Step 2. For specs with >10 references, consider parallel Explore agents (see Agent Delegation below).

## 3.0 Cross-Crate Scope Establishment

For patterns referenced across multiple files (e.g., field assignments like `expires_at: None`, enum variant matches, trait method calls), run a cross-crate count grep first to establish the full scope before per-file analysis. Compare the spec's claimed locations against the actual count. This catches files the spec missed and prevents incomplete deliverables.

## 3.1 File Paths

Glob/Grep to confirm each path exists. If moved, renamed, or deleted, record the discrepancy and actual location.

## 3.2 Types and Interfaces

Grep for each type. Confirm existence and current shape (fields, members). Check for:

- **Field existence and naming**: Flag fields the spec assumes but don't exist or have different names/types.
- **Numeric type accuracy**: Verify assumed types match actual types (`u32` vs `Permille` vs `i32`). If a formula combines different numeric types, flag as LOW Improvement.
- **Serialization**: If the spec proposes serializing a type, verify `Serialize`/`Deserialize` derives.
- **Design table exhaustiveness**: If the spec includes a lookup table or mapping indexed by an enum (e.g., priority-per-EntityKind, handling-per-GoalKind), verify the table covers all current enum variants. Missing variants will require either explicit entries or a documented catch-all default.
- **Hash functions**: If acceptance criteria reference hash functions, verify they exist and check input inclusion/exclusion.
- **Field additions to non-ECS structs** (belief-layer, snapshot types): Check serde derives, `#[serde(default)]`, Default impl impact, and whether derivation/construction functions (e.g., `derive_entity_summary()`) can populate the new field from their inputs. If a derivation function reconstructs from a data source lacking the new field, flag the propagation gap as an Issue.
- **Aggregation-key fidelity**: When the spec uses a type as a `BTreeMap`/histogram/distribution key, verify the type's variants are payload-free. If it has payload-bearing variants, keying on the full type fragments the histogram by payload value — flag that the spec must specify discriminant-grouping or introduce a discriminant key type. This applies even when the key type itself is a clean rename of a fictional spec name (e.g., `DiscrepancyKind` → real `Discrepancy`): the rename is trivial, but the payload-bearing-variant wrinkle is not, and it surfaces at Step 7 if not caught here.

## 3.3 Functions and Methods

Grep for each function. Confirm signature, module location, and export status. Line-number references in specs are informational aids, not authoritative. Verify they point to the claimed content. If accurate, leave them — they help implementers navigate. A single spec may cite parameter-declaration lines in some files and fn-declaration lines in others (both are valid anchors for migration deliverables — parameter-line citations point at the type surface being migrated, fn-declaration citations point at the function being modified). Convention-mixing within a spec is not drift — flag it only if the cited line does not resolve to any meaningful content at all. If drifted, either correct them or replace with function/type names that are grep-stable.

**Large-file handling**: For files exceeding the Read tool's token limit (typically engine modules >25k tokens like `crates/worldwake-ai/src/planning_snapshot.rs` or `crates/worldwake-cli/src/scenario/mod.rs`), prefer Grep with `output_mode=content` and `-n=true` to locate the target symbols, then targeted offset/limit Read calls rather than chunked full-file Reads. Full reads of large files waste context that is better spent on cross-file validation.

Check for:

- **Signature differences** from what the spec assumes.
- **New function parameter sufficiency**: Validate that proposed parameters provide sufficient data at every call site. Flag if a parameter type lacks needed context.
- **Data-surface compatibility**: For proposed shared helpers or unified abstractions, verify that the input type (e.g., `PlanningState`, `GoalBeliefView`, `GenerationContext`) is accessible at ALL intended call sites. Different pipeline stages may use different trait surfaces for the same underlying data. Flag when a helper's proposed data surface is not available at one or more call sites.
- **Layer-capability fidelity**: When a deliverable places new logic inside an existing function or layer, verify the host has the *behavioral capabilities* the logic assumes — search/planning access, mutable state, event emission, the relevant call surface — not merely that the data types are reachable. This is distinct from data-surface compatibility (which type is in scope) and from pseudocode execution fidelity (whether the control flow runs): it asks whether the host layer *can perform the operation the deliverable requires at all*. A spec that places goal-to-steps synthesis inside a pure composition layer (e.g., a repair assembler that only arranges pre-built steps and holds no planner handle) is unimplementable even when every named symbol resolves and every type is reachable; the fix relocates the logic to a layer that has the capability (e.g., the upstream seam where candidates are constructed with belief-view access). Flag the layer-capability mismatch as an Issue and name the layer that does have the capability.
- **Proposed modifications to existing functions**: Verify the function's parameters and local scope include variables the proposed code references. Flag out-of-scope variable usage as an Issue.
- **Symbol partitioning** (splitting traits/enums): Verify the partition is complete (all symbols accounted for) and disjoint (no symbol in two categories). Verify stated counts match listed names. Use automated scripts for large sets (>20 symbols).
- **Code example fidelity**: If the spec includes Before/After code snippets, verify they match the actual code's control flow structure (e.g., imperative loops vs. iterator chains, match arms vs. if-let chains). Style mismatches in code examples mislead implementers. For new system specs (type a) with proposed pseudocode, validate each API call against the nearest existing analog: if the spec says "follows the evidence_decay pattern," read the analog's actual function signature, constructor arguments, field names, and commit flow, then verify the pseudocode matches. Treat each function call, field access, method invocation, and constructor in proposed pseudocode as a codebase reference subject to the same validation as any other Step 3.3 reference.
- **Pseudocode execution fidelity**: When pseudocode proposes extending an existing function, read the existing function's control flow end-to-end and verify the proposed code would actually execute under the spec's intended conditions. Flag early returns, guard clauses, type-narrowing checks, and `return None` paths that would short-circuit the proposed extension. A spec whose pseudocode is structurally correct but unreachable under its target inputs has a behavioral defect even though every named symbol resolves. Common forms: a discount/penalty function that returns `None` when the existing axis is zero (so a new axis added "alongside" never runs); a match arm that handles only one variant of an enum the new logic intends to span; an `if let Some(...)` guard that filters out the precise case the new branch was written for. Cite the short-circuiting line(s) in the finding so the implementer knows what to restructure.
- **Runtime vs. test-only classification**: For deliverables that list specific code locations to modify, check whether each location is inside `#[cfg(test)]` or conditional compilation. Grep for `#[cfg(test)]` in each file to find the boundary line number, then classify each spec-referenced line as runtime (before boundary) or test-only (after boundary). If a deliverable frames test-only locations as runtime targets, flag as an Issue. If a deliverable mixes runtime and test locations without distinction, flag as an Improvement (separate runtime changes from test fixture updates).
- **Reuse opportunities**: For each new function or trait method the spec proposes to create, grep the codebase for existing functions serving the same purpose. A proposed new method that duplicates existing functionality should be flagged as an Issue (prefer reuse) or Improvement (note the existing alternative).
- **Pseudocode dependency completeness**: For each function call, type constructor, or method invocation in spec pseudocode, verify it either (a) exists in the codebase, or (b) is defined or proposed elsewhere in the spec as a new deliverable. Functions that are neither existing nor spec-defined are incomplete deliverables — flag as Issues. This rule also applies to doc-comments on proposed types, functions, and trait methods: any symbol named in a `///` comment as an alternative constructor, entrypoint, or contract partner must be either existing or proposed as a deliverable in the same spec.
- **Proposed reuse fidelity**: When a spec claims to reuse an existing function, field, or mechanism, verify the reuse is semantically compatible. Read the existing implementation and compare its behavior (input filtering, edge cases, output semantics) against the spec's proposed usage. Flag semantic divergences that would require a new function or a modified version rather than direct reuse.
- **Existing behavior overlap**: For deliverables that propose modifying existing functions, verify the proposed change isn't already implemented. If the current code already exhibits the described behavior (e.g., locality scoping via a trait implementation the spec doesn't acknowledge), flag as an Issue — the deliverable should either be eliminated, merged into a sibling deliverable, or rewritten as an explanatory note about existing behavior. This requires reading beyond the function signature into the implementation and its call chain. For proposed changes to functions called from multiple code paths, trace which paths are actually active for the spec's scenario. A function may exist and match the spec's description but never be reached in the described failure mode.
- **Access-chain reachability**: When the spec describes a value reached through nested data (`X.Y.Z` or "the W attached to V"), grep every step of the chain. Each named type may exist independently while the documented access path does not — flag missing-middle as an Issue. Example: a spec that mentions "the `PlanGuard` attached to the current step" without naming the host field requires grepping for the field on the actual step type (e.g., `PlannedStep.guard: Option<PlanGuard>`); confirming `PlanGuard` exists is not sufficient if its attachment site is unstated.
- **Migration-scope claims for action registration**: When a spec claims a per-handler migration count (e.g., "~25 action registrations"), distinguish between public registration *functions* (typically one or a few per file) and individual *action definitions* (composites like `register_needs_actions`, `register_craft_actions`, `register_harvest_actions`, `register_office_actions`, `register_artifact_actions` register multiple `ActionDef`s internally — sometimes per-commodity or per-recipe). Cite both counts when relevant — the registration-function count understates migration cost by ~1.5–2× because each composite expands to multiple `ActionDef` constructions that the migration must touch separately.

## 3.3A Output Format Fidelity

For specs that propose new output or report sections in existing tooling (observer binary, CLI, diagnostic tools), verify the proposed format (section headers, delimiters, label conventions) matches the target file's existing formatting patterns. Grep for existing section markers and formatting conventions in the target file. Flag mismatches as Issues.

## 3.3B Scenario Content Validation

For specs that design new RON scenarios or propose specific scenario configurations, validate all proposed values against actual codebase definitions:

- **WorkstationTag values**: Grep the `WorkstationTag` enum and confirm every proposed workstation exists as a variant.
- **PlaceTag values**: Grep the `PlaceTag` enum and confirm every proposed place tag exists. Note the distinction between PlaceTags (place-level properties like `Latrine`, `Forest`) and WorkstationTags (facility-level like `Well`, `FieldPlot`).
- **Recipe names**: Grep the action registry or existing scenarios for recipe name format. Worldwake uses Title Case with spaces (e.g., `"Harvest Grain"`, `"Harvest Apples"`), not camelCase or snake_case.
- **AgentDef fields**: Cross-reference proposed agent profile fields against `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs`.
- **Commodity names**: Verify proposed commodity references against `CommodityKind` enum variants.
- **Format conventions**: Cross-reference with existing scenarios (glob `scenarios/*.ron`) for structural conventions (facility definitions, resource source fields, agent profile structure).
- **Need coverage**: If the scenario claims to prove survival or need satisfaction, verify it covers all `HomeostaticNeedId` variants and that the proposed facilities/tags can satisfy each need's action preconditions.

## 3.4 Dependencies (specs/tickets)

Verify each dependency lives in `specs/`, `archive/specs/`, `tickets/`, or `archive/tickets/`. Record correct paths. Note dependencies listed as incomplete but since implemented.

## 3.5 Component Fields and ECS Registrations

Skip sub-steps 5a-5g if the spec does not add fields to components, create new components, extend discriminator enums, or change the visibility of existing public fields. Note: 5h (Trait accessor propagation) is NOT covered by this skip — it applies whenever the spec reads any profile or component through GoalBeliefView, even if no new fields are added.

- **5a. Shape validation**: Grep component structs in `worldwake-core`, verify fields/types. Check `component_schema.rs` for registration.
- **5b. Trait bounds**: Check derive macros and trait bounds on types/enums the spec extends. Record constraints new additions must satisfy (`Copy`, `Serialize`, `Ord`).
  - **Derive propagation**: For new types with derives (`Hash`, `Ord`, `Copy`, etc.), verify all field types also derive those traits. Flag missing derives on embedded types as CRITICAL Issues.
  - **Derive widening**: If the spec shows a modified type with new derive attributes, compare against current derives. Note explicitly so implementers don't treat them as copy-paste artifacts.
- **5c. Default and constructors**: For field additions, check `Default` impl and builder/constructor functions. For field additions to components or structs that are deserialized from any external source (scenario files via RON, save state, replay state), verify the field has a serde default (`#[serde(default)]` or `#[serde(default = "...")]`) so existing serialized data continues to deserialize. Grep existing scenarios for the component name to confirm whether scenario deserialization is a concern. For belief store structs (e.g., `AgentBeliefStore`) and snapshot types, save/replay compatibility is always a concern — new fields need `#[serde(default)]`.
- **5d. Downstream consumers**: For field type changes, field removals, or public-to-narrower visibility demotions (e.g., `pub` → `pub(crate)`, `pub(crate)` → file-private), perform full downstream consumer analysis (3.6). Grep every direct-field-access site across the workspace — visibility demotion can break external consumers the same way a removal does, even when the field's type and name are unchanged. For field removals specifically, also grep each removed field name across the workspace to find all direct usage sites — not just the type's consumers, but any code reading that specific field. This catches cases where sibling logic within the same function depends on the removed field (e.g., social observation pruning depending on `memory_retention_ticks` within `enforce_capacity`).
- **5e. Scalar-to-collection migrations**: Grep for equality comparisons (`== field_value`) that would need `.contains()`.
- **5f. Semantic overlap**: Two sub-checks:
  - *Spec-acknowledged overlap*: If the spec documents the relationship between a new field and an existing field, note "overlap acknowledged by spec" and skip the grep.
  - *Unacknowledged overlap*: Grep for semantically similar field names across all components. Also check functional overlap — fields serving the same purpose with different names. Flag as P28 migration candidates. For new components, apply the **novel-domain test**: a component is novel if no existing component serves the same downstream consequence (P5). Novel-domain components focus on functional overlap; domain-extension components also need field name similarity checks.
- **5f-extra. Non-ECS runtime state overlap**: For new ECS components that track runtime state (counters, caches, trackers), also grep for similar tracking in non-ECS runtime structures (e.g., `AgentDecisionRuntime`, action handler state). Overlap between ECS and non-ECS tracking is a common source of confusion — flag it as an Improvement so the spec can acknowledge the relationship and explain why both are needed (e.g., different granularity).
- **5g. EntityKind variant overlap**: Check whether existing enum variants overlap semantically with proposed additions. Flag empty/unused variants that fragment the same domain as P28 candidates.
- **5h. Trait accessor propagation**: For new components read by the AI crate during candidate generation, goal ranking, or planning, check whether `GoalBeliefView` (`worldwake-sim/src/belief_view.rs`) or `BeliefView` needs a new accessor method. If so, flag the spec's crate list as needing update and note the `RuntimeBeliefView` impl and `GoalBeliefView` blanket impl forwarding required. This is a common pattern: new behavioral components almost always need a belief-view accessor for the AI crate to read them.

## 3.6 Downstream Consumers

For types/interfaces the spec modifies, grep all import sites and usage points. Record blast radius. For new enum variants:

- **Trait bounds**: Check derives. Verify new variant fields satisfy existing bounds. Note `#[allow(clippy::large_enum_variant)]` size implications.
- **Exhaustive match analysis**: Grep for pattern matches on existing variants to find all match sites needing a new arm. Especially important for enums matched across multiple crates.

## 3.6A Goal Infrastructure Validation

For specs adding new `GoalKind` variants, verify the spec addresses all mandatory integration points:

1. **GoalDispatchKey**: New variant in `GoalDispatchKey` enum, added to `ALL` constant, and `from_goal_kind` match arm (`goal_dispatch_key.rs`).
2. **GoalDispatchDeclaration**: Entry in `goal_dispatch_decl.rs` with `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`, `progress_barrier_ops`, and `family_policy`.
3. **GoalKindPlannerExt**: Implementation of all trait methods in `goal_model.rs` — `relevant_op_kinds`, `is_satisfied`, `apply_planner_step`, `goal_relevant_places`, `prerequisite_places`, `matches_binding`, `candidate_is_available`, `build_payload_override`, `ranked_goal_provenance_family`, `relevant_observed_commodities`, `is_progress_barrier`.
4. **Ranking integration**: Priority class (`GoalPriorityClass`) and `motive_score` computation formula.

Flag each missing item as a HIGH Issue. If the spec says "reuse existing travel planning" or similar, verify it still names the specific ops and dispatch types.

Verify the `GoalKindPlannerExt` method list above against the current trait definition in `goal_model.rs` — methods may have been added or removed since this skill was last updated.

See `references/worldwake-validation-patterns.md` for additional project-specific patterns.

## 3.7 Crate Boundary Validation

Verify proposed functions' parameter/return types are accessible from the target crate. Check `Cargo.toml` dependencies. Flag violations of workspace layering (`core -> sim -> systems -> ai -> cli`).

## 3.7A Workspace-External Dependency Check

For specs that mention type names suggesting external (non-workspace) crates — e.g., `SmallVec`, `IndexMap`, `IndexSet`, `Cow`, `Bytes`, `DashMap`, `Arc`-wrappers from non-`std` crates, `tokio::*`, `crossbeam::*`, `parking_lot::*`, `rayon::*`, anything else that is not `std`, not `serde`, not `bincode`, not `rand_chacha`, and not `blake3` — grep all workspace `Cargo.toml` files (`crates/*/Cargo.toml` and the workspace root `Cargo.toml`) for the crate name to confirm whether it is currently a dependency.

If the crate is absent, flag as a HIGH Issue with two routing options:

- **(a) Add the crate as an explicit deliverable**. The spec must commit to a Cargo.toml addition with version, features, and rationale. Note that `worldwake-core` is the most dependency-restricted crate (currently `serde`, `bincode`, `blake3` only) — adding any external crate to core has higher review weight than to higher crates.
- **(b) Substitute the workspace-native equivalent**. Common substitutions: `Vec<T>` + per-agent soft-cap (e.g., `cognitive.decision_history_alternatives`) replaces `SmallVec<T, N>`; `BTreeMap`/`BTreeSet` replaces `IndexMap`/`IndexSet`/`DashMap` (also satisfies the determinism invariant per CLAUDE.md); `&[T]` borrows replace `Cow<[T]>`. Cite the existing precedent in the codebase when proposing the substitution.

The check is independent of the spec's claimed crate row. A spec that says `worldwake-cli` extends an observer feature with `IndexMap` still triggers this check even though `worldwake-cli` already has plenty of dependencies — the question is whether `IndexMap` itself is in the workspace, not whether *some* external crate is.

**Skip condition**: the type name resolves to a workspace-native synonym defined elsewhere in the codebase (e.g., a project-local `SmallSet` wrapper). Grep the codebase for `pub struct <TypeName>` or `pub type <TypeName>` to rule this out before flagging.

## 3.8 Upstream Spec References

Grep active specs in `specs/` **and archived specs in `archive/specs/`** for references to this spec's deliverables. Note affected specs.

Archived-spec matches are informational (the dependency already landed) — use them to refresh the Dependencies section, Motivating Evidence, and any "this spec depends on X" prose with accurate archival paths. Archived matches do not block reassessment; they catch stale "X has not landed yet" claims and surface forward-references that the archived sibling made back to this spec (common when observer/diagnostic specs land in waves and a later adjunct spec is reassessed after its siblings archive).

**Sibling-spec citation accuracy**: If the spec under reassessment cites named types, functions, components, enum variants, or event tags attributed to a sibling spec (typically in Non-Goals, Dependencies, Cross-System Interactions, or Motivating Evidence), grep both the codebase AND the sibling spec file to confirm the citation. The sibling spec may not have landed yet — in which case the cited surface should at minimum match the sibling spec's *current draft* surface, not a stale earlier draft. For specs in active sibling clusters (multiple draft specs at the same phase, often co-derived from one assessment), this check is mandatory because draft surfaces drift across the cluster as each spec is reassessed in turn. Flag mismatches as Issues — the spec's prose names a type that exists in neither the codebase nor the sibling spec — or as Improvements when the sibling spec uses a slightly different name for the same concept.

## 3.8A Cross-System and SystemFn Section Validation

For specs that include Cross-System Interactions or SystemFn Integration sections, verify each crate attribution: confirm the described behavior (commit handler, system function, ranking logic, etc.) actually resides in the named crate and module. Flag misattributed crates as Issues — these are prose claims about responsibility that drift when code moves between crates.

Beyond crate attribution accuracy, evaluate whether each cross-system interaction is architecturally appropriate. A system reading another domain's profile (e.g., needs system reading ExplorationProfile) may indicate cross-concern coupling. Flag as Improvement with an alternative that keeps domain-awareness self-contained (e.g., moving the logic to the domain's own crate).

**Motivating-scenario substrate fidelity**: For specs that cite a concrete in-game scenario as Motivating Evidence (e.g., "Agent A and B competing at North Orchard with `BlockingFact(ReservationConflict)` events"), identify the actual entity types and components the scenario uses (e.g., `ResourceSource`, `ResourceExtractionQueues`) and confirm the spec's named hook fires on that scenario's substrate. Multi-substrate domains commonly tempt specs to author a hook on one substrate while the motivating example uses another — for example, hooking facility-queue grant promotions (`ContentionQueue` in `facility_queue.rs`) when the motivating scenario runs through resource-extraction queues (`ResourceExtractionQueues` in `production_actions.rs`), or hooking goal-emission when the scenario is shaped by goal-ranking. Trace the scenario through actual code: which component holds the contested state, which handler emits the relevant event, which crate owns the mutation. Flag mismatches as HIGH Issues — the spec's named hook will never fire for its headline scenario.

## 3.9 Behavioral Claim Validation

For each claim about who reads/writes a type at runtime, grep all call sites and classify as runtime vs. test-only (`#[cfg(test)]`). Flag contradictions as CRITICAL. If technically wrong but safe (e.g., caller only reads current-tick data), note both the correction and safety argument.

## Agent Delegation

In plan mode, Explore agents are the primary validation mechanism (read-only, inherently compatible). Launch 2-3 agents organized by theme for specs with >10 references.

For specs with many references, launch parallel Explore agents organized by theme (e.g., action/type references, AI/test references, dependencies/infrastructure). Choose themes to minimize cross-agent dependencies. Typical: 1 agent for 10-15 references with a single domain, 2-3 agents for 15+ references spanning multiple domains. Max 3 agents for pure (a)/(b)/(c)/(d)/(e) specs; up to 4 agents acceptable for hybrid classifications (e.g., (a)+(b), (a)+(d), (e)+(b)) when the broader scope produces 4+ disjoint themes — artificially merging into 3 produces larger, less-focused prompts and risks the subagent-overreach failure mode this file warns about. Beyond 4, prefer merging adjacent themes or sequencing rather than spawning a 5th concurrent agent — coordination cost outweighs parallelism benefit. For type (a) new system specs, 3 parallel agents is typical — the reference count and domain spread (AI crate, core types, belief/dependency infrastructure) usually justify full parallelism.

Guidelines:
- When agents validate code locations referenced by a spec's deliverables, instruct them to report the `#[cfg(test)]` boundary line number for each file so runtime vs. test classification can be done without follow-up reads.
- If agents return conflicting results for the same reference, spot-check with direct Grep/Read. Trust the direct tool result over the agent claim.
- After results arrive, cross-reference findings against the spec's type assumptions and formulas. Agents validate existence; you validate semantic compatibility.
- For static lookup tables indexed by discriminator enums, verify key granularity matches discrimination needs.
- Spot-check agent claims with direct Grep/Read before including in findings — agent results are leads, not facts. Especially spot-check when an agent reports a referenced type as "does not exist" or "needs to be created" — verify whether the spec used a wrong name for an existing type before accepting the agent's conclusion. A common cause of false-negative agent reports is *shape mismatch* — the spec mischaracterizes a `GoalKind` variant as a struct, or a trait method as a free function, and the agent's grep (shaped to the spec's framing) misses the existing-but-differently-shaped symbol. When grepping to verify an agent's "does not exist" claim, search for the symbol name independent of shape (`grep -rn "<name>"` rather than `grep "pub struct <name>"`).
- When agents return file-path claims, spot-check the path with `test -f` or `ls` directly before adopting it into a finding or deliverable — agents commonly canonicalize a filename without canonicalizing its containing crate (e.g., reporting `crates/worldwake-ai/src/perception.rs` for a file that actually lives at `crates/worldwake-systems/src/perception.rs`). The filename half is usually correct; the crate half is the failure mode. This spot-check is in addition to the symbol-existence checks above and is most important for paths that will be written into the new spec body rather than just consulted during validation.
- Inversely, spot-check when an agent reports a spec-referenced method or type as *existing* — grep the exact symbol to confirm. Agents sometimes confabulate existence to match the spec's Before/After framing, and a single unchecked confirmation can propagate the spec's false premise through the entire audit. This failure is at least as common as the "does not exist" case above and more dangerous because the confirmation feels reassuring. When two agents agree a symbol is absent and a third reports it present, trust the absence-reporters and verify with direct Grep.
- For structural refactor specs (type c), direct agents toward discrepancy checking (counts, symbol existence, blast radius) rather than broad exploration.
- When a spec cites line numbers for `&[SomeType]` parameter sites (typical for D3-style parameter-migration deliverables), instruct agents to grep the target surface (`SomeType>` or `some_field: &\[SomeType\]`) at each cited line directly — do not accept an agent's "fn declaration is at line N, parameter at line M, therefore drift of M−N" reasoning. Specs often cite the parameter line as the migration target, and the offset between fn-declaration and parameter is convention, not drift. Spot-check with direct Grep before promoting any agent-reported "line drift" to a finding.

After Explore agents return, a Plan agent may be used to organize and classify findings, cross-reference agent results against the spec's type assumptions, and identify gaps the Explore agents missed. This is optional and most useful when findings are numerous (>5) or span multiple domains.

## Conditional Deliverable Validation

For specs with conditional deliverables ("If root cause X is confirmed, do Y"), validate:

1. **Diagnostic sufficiency** — the investigation steps can distinguish between hypotheses (e.g., each hypothesis predicts different observable outcomes)
2. **Fix correctness** — each proposed fix references correct types, functions, and file paths, regardless of whether it will ultimately be selected
3. **Architectural soundness** — each proposed fix respects crate boundaries and FOUNDATIONS principles even though it is conditional. Flag fixes that violate constraints even if conditional — a conditional violation is still a spec defect.
