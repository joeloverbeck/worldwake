# Codebase Validation (Step 3)

Validate every reference from Step 2. For specs with >10 references, consider parallel Explore agents (see Agent Delegation below).

## 3.1 File Paths

Glob/Grep to confirm each path exists. If moved, renamed, or deleted, record the discrepancy and actual location.

## 3.2 Types and Interfaces

Grep for each type. Confirm existence and current shape (fields, members). Check for:

- **Field existence and naming**: Flag fields the spec assumes but don't exist or have different names/types.
- **Numeric type accuracy**: Verify assumed types match actual types (`u32` vs `Permille` vs `i32`). If a formula combines different numeric types, flag as LOW Improvement.
- **Serialization**: If the spec proposes serializing a type, verify `Serialize`/`Deserialize` derives.
- **Hash functions**: If acceptance criteria reference hash functions, verify they exist and check input inclusion/exclusion.
- **Field additions to non-ECS structs** (belief-layer, snapshot types): Check serde derives, `#[serde(default)]`, Default impl impact, and whether derivation/construction functions (e.g., `derive_entity_summary()`) can populate the new field from their inputs. If a derivation function reconstructs from a data source lacking the new field, flag the propagation gap as an Issue.

## 3.3 Functions and Methods

Grep for each function. Confirm signature, module location, and export status. Check for:

- **Signature differences** from what the spec assumes.
- **New function parameter sufficiency**: Validate that proposed parameters provide sufficient data at every call site. Flag if a parameter type lacks needed context.
- **Proposed modifications to existing functions**: Verify the function's parameters and local scope include variables the proposed code references. Flag out-of-scope variable usage as an Issue.
- **Symbol partitioning** (splitting traits/enums): Verify the partition is complete (all symbols accounted for) and disjoint (no symbol in two categories). Verify stated counts match listed names. Use automated scripts for large sets (>20 symbols).
- **Code example fidelity**: If the spec includes Before/After code snippets, verify they match the actual code's control flow structure (e.g., imperative loops vs. iterator chains, match arms vs. if-let chains). Style mismatches in code examples mislead implementers.
- **Reuse opportunities**: For each new function or trait method the spec proposes to create, grep the codebase for existing functions serving the same purpose. A proposed new method that duplicates existing functionality should be flagged as an Issue (prefer reuse) or Improvement (note the existing alternative).

## 3.4 Dependencies (specs/tickets)

Verify each dependency lives in `specs/`, `archive/specs/`, `tickets/`, or `archive/tickets/`. Record correct paths. Note dependencies listed as incomplete but since implemented.

## 3.5 Component Fields and ECS Registrations

Skip sub-steps 5a-5g if the spec does not add fields to components, create new components, or extend discriminator enums. Note: 5h (Trait accessor propagation) is NOT covered by this skip — it applies whenever the spec reads any profile or component through GoalBeliefView, even if no new fields are added.

- **5a. Shape validation**: Grep component structs in `worldwake-core`, verify fields/types. Check `component_schema.rs` for registration.
- **5b. Trait bounds**: Check derive macros and trait bounds on types/enums the spec extends. Record constraints new additions must satisfy (`Copy`, `Serialize`, `Ord`).
  - **Derive propagation**: For new types with derives (`Hash`, `Ord`, `Copy`, etc.), verify all field types also derive those traits. Flag missing derives on embedded types as CRITICAL Issues.
  - **Derive widening**: If the spec shows a modified type with new derive attributes, compare against current derives. Note explicitly so implementers don't treat them as copy-paste artifacts.
- **5c. Default and constructors**: For field additions, check `Default` impl and builder/constructor functions.
- **5d. Downstream consumers**: For field type changes or removals, perform full downstream consumer analysis (3.6).
- **5e. Scalar-to-collection migrations**: Grep for equality comparisons (`== field_value`) that would need `.contains()`.
- **5f. Semantic overlap**: Two sub-checks:
  - *Spec-acknowledged overlap*: If the spec documents the relationship between a new field and an existing field, note "overlap acknowledged by spec" and skip the grep.
  - *Unacknowledged overlap*: Grep for semantically similar field names across all components. Also check functional overlap — fields serving the same purpose with different names. Flag as P28 migration candidates. For new components, apply the **novel-domain test**: a component is novel if no existing component serves the same downstream consequence (P5). Novel-domain components focus on functional overlap; domain-extension components also need field name similarity checks.
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

## 3.8 Upstream Spec References

Grep active specs in `specs/` for references to this spec's deliverables. Note affected specs.

## 3.9 Behavioral Claim Validation

For each claim about who reads/writes a type at runtime, grep all call sites and classify as runtime vs. test-only (`#[cfg(test)]`). Flag contradictions as CRITICAL. If technically wrong but safe (e.g., caller only reads current-tick data), note both the correction and safety argument.

## Agent Delegation

In plan mode, Explore agents are the primary validation mechanism (read-only, inherently compatible). Launch 2-3 agents organized by theme for specs with >10 references.

For specs with many references, launch parallel Explore agents organized by theme (e.g., action/type references, AI/test references, dependencies/infrastructure). Choose themes to minimize cross-agent dependencies. Typical: 1 agent for 10-15 references with a single domain, 2-3 agents for 15+ references spanning multiple domains. Max 3 agents.

Guidelines:
- After results arrive, cross-reference findings against the spec's type assumptions and formulas. Agents validate existence; you validate semantic compatibility.
- For static lookup tables indexed by discriminator enums, verify key granularity matches discrimination needs.
- Spot-check agent claims with direct Grep/Read before including in findings — agent results are leads, not facts. Especially spot-check when an agent reports a referenced type as "does not exist" or "needs to be created" — verify whether the spec used a wrong name for an existing type before accepting the agent's conclusion.
- For structural refactor specs (type c), direct agents toward discrepancy checking (counts, symbol existence, blast radius) rather than broad exploration.

After Explore agents return, a Plan agent may be used to organize and classify findings, cross-reference agent results against the spec's type assumptions, and identify gaps the Explore agents missed. This is optional and most useful when findings are numerous (>5) or span multiple domains.

## Conditional Deliverable Validation

For specs with conditional deliverables ("If root cause X is confirmed, do Y"), validate:

1. **Diagnostic sufficiency** — the investigation steps can distinguish between hypotheses (e.g., each hypothesis predicts different observable outcomes)
2. **Fix correctness** — each proposed fix references correct types, functions, and file paths, regardless of whether it will ultimately be selected
3. **Architectural soundness** — each proposed fix respects crate boundaries and FOUNDATIONS principles even though it is conditional. Flag fixes that violate constraints even if conditional — a conditional violation is still a spec defect.
