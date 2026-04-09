---
name: reassess-spec
description: "Reassess a spec against the codebase and FOUNDATIONS.md. Validates assumptions, identifies issues/improvements/additions, asks clarifying questions, then writes the updated spec. Use when preparing a spec for ticket decomposition."
user-invocable: true
arguments:
  - name: spec_path
    description: "Path to the spec file (e.g., specs/S05-merchant-stock-storage-and-stalls.md)"
    required: true
---

# Reassess Spec

Validate a spec's proposed implementation against the actual codebase and FOUNDATIONS.md. Identify issues, improvements, and beneficial additions. Deliver an updated spec ready for ticket decomposition.

## Invocation

```
/reassess-spec <spec-path>
```

**Arguments** (required, positional):
- `<spec-path>` — path to the spec file (e.g., `specs/S05-merchant-stock-storage-and-stalls.md`)

If the argument is missing, ask the user to provide it before proceeding.

## Worktree Awareness

If working inside a worktree (e.g., `.claude/worktrees/<name>/`), ALL file paths — reads, writes, globs, greps — must use the worktree root as the base path.

## Plan Mode Awareness

If plan mode is active:

- **Steps 1-6** proceed normally (all read-only).
- **Step 6** includes the initial findings report and any question-resolution rounds.
- **After all questions are resolved**: Write a condensed summary to the plan file, then call ExitPlanMode. If question resolution produces new findings, the plan file reflects the final resolved state, not the initial report.
- **After plan approval**: Steps 7-8 execute. The user's approval covers both question resolutions and overall changes — no separate gate.
- **Pre-Apply Verification** runs after ExitPlanMode approval, before Step 7.
- If there are no questions, proceed from the Step 6 findings report directly to writing the plan file and calling ExitPlanMode.
- If the ExitPlanMode result contains user comments, treat them as binding modifications.

**Plan file structure**:
- **Context**: Which spec, why it's being reassessed
- **Approved Changes**: Organized by Issues Fixed / Improvements Applied / Additions Incorporated, each with severity tag
- **Critical Files**: Paths of files to be modified
- **Verification**: How to confirm the updated spec is correct after writing

The conversational report (Step 6) is the decision artifact. Present it as a normal message — do not write it to the plan file. The plan file is a separate condensed reference for implementation (Steps 7-8).

## Process

Follow these steps in order. Do not skip any step.

### Pre-Process: Spec Classification

Before beginning Steps 2-3, classify the spec:

- **(a) New system** — introduces new components, actions, goal kinds, or information paths. Full checklist applies.
- **(b) System extension** — extends existing components, actions, or enums without new systems. Steps 3.1-3.8, 4.4 apply. Skip 3.9 if no behavioral claims about runtime readers/writers. Section H updates only for new deliverable sections. For tooling-only specs, downstream consumer analysis (3.6) can be limited to the tooling binary.
- **(c) Structural refactor** — trait/module restructuring with no behavioral changes. Skip Steps 3.5, 3.9, 4.4; Section H is N/A. Focus on symbol existence, count accuracy, and blast radius.
- **(d) Test-only** — adds golden tests, benchmarks, or test infrastructure without modifying production code.
  - Steps 3.1-3.4 apply (validate referenced paths, types, functions, dependencies).
  - Skip 3.5-3.9 (no production code changes to trace).
  - Step 4 applies but 4.4 is N/A.
  - Section H updates are N/A unless the test reveals a missing causal hook.

**Hybrid specs**: Apply the union of applicable steps — use the most rigorous classification's checklist for shared steps.

**Re-reassessment shortcut**: If the same spec was reassessed earlier in this session and not externally modified, Steps 2-3 may scope to only references affected by the triggering change. Step 1 still applies.

**Self-authored spec note**: Full validation is required even for specs authored earlier in this session — authoring may introduce unchecked assumptions.

### Step 1: Mandatory Reads

Read ALL of these before any analysis:

1. **The spec file** (from the argument) — entire file
2. **`docs/FOUNDATIONS.md`** — skip if read earlier in this session and unmodified
3. **`docs/spec-drafting-rules.md`** (if the spec contains or should contain Section H) — skip if read earlier and unmodified

Parse the spec's metadata: Phase, Status, Priority, Crates, Dependencies, Goals/Design Goals, Non-Goals, FOUNDATIONS Alignment, and all deliverable sections.

### Step 2: Extract References

Extract every concrete codebase reference from the spec:

- **File paths** mentioned or implied
- **Type names** (e.g., `GoalKind`, `SaleListing`, `PlannerOpKind`)
- **Function names** (e.g., `generate_candidates`, `enumerate_trade_payloads`)
- **Crate/module names**
- **Test file paths or test names**
- **Other specs or tickets** in Dependencies

Build a validation checklist (internal). Prioritize references most likely to have drifted: dependency paths, function signatures, and types the spec extends. Stable types (`EntityId`, `Permille`, `Quantity`) can be spot-checked.

### Step 3: Codebase Validation

Validate every reference from Step 2. For specs with >10 references, consider parallel Explore agents (see Agent Delegation below).

#### 3.1 File Paths

Glob/Grep to confirm each path exists. If moved, renamed, or deleted, record the discrepancy and actual location.

#### 3.2 Types and Interfaces

Grep for each type. Confirm existence and current shape (fields, members). Check for:

- **Field existence and naming**: Flag fields the spec assumes but don't exist or have different names/types.
- **Numeric type accuracy**: Verify assumed types match actual types (`u32` vs `Permille` vs `i32`). If a formula combines different numeric types, flag as LOW Improvement.
- **Serialization**: If the spec proposes serializing a type, verify `Serialize`/`Deserialize` derives.
- **Hash functions**: If acceptance criteria reference hash functions, verify they exist and check input inclusion/exclusion.
- **Field additions to non-ECS structs** (belief-layer, snapshot types): Check serde derives, `#[serde(default)]`, Default impl impact, and whether derivation/construction functions (e.g., `derive_entity_summary()`) can populate the new field from their inputs. If a derivation function reconstructs from a data source lacking the new field, flag the propagation gap as an Issue.

#### 3.3 Functions and Methods

Grep for each function. Confirm signature, module location, and export status. Check for:

- **Signature differences** from what the spec assumes.
- **New function parameter sufficiency**: Validate that proposed parameters provide sufficient data at every call site. Flag if a parameter type lacks needed context.
- **Proposed modifications to existing functions**: Verify the function's parameters and local scope include variables the proposed code references. Flag out-of-scope variable usage as an Issue.
- **Symbol partitioning** (splitting traits/enums): Verify the partition is complete (all symbols accounted for) and disjoint (no symbol in two categories). Verify stated counts match listed names. Use automated scripts for large sets (>20 symbols).

#### 3.4 Dependencies (specs/tickets)

Verify each dependency lives in `specs/`, `archive/specs/`, `tickets/`, or `archive/tickets/`. Record correct paths. Note dependencies listed as incomplete but since implemented.

#### 3.5 Component Fields and ECS Registrations

Skip sub-steps 5a-5g if the spec does not add fields to components, create new components, or extend discriminator enums.

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
- **5h. Trait accessor propagation**: For new components read by the AI crate during candidate generation, goal ranking, or planning, check whether `GoalBeliefView` (`worldwake-sim/src/belief_view.rs`) or `BeliefView` needs a new accessor method. If so, flag the spec's crate list as needing update and note the `RuntimeBeliefView` impl + `impl_goal_belief_view!` macro forwarding required. This is a common pattern: new behavioral components almost always need a belief-view accessor for the AI crate to read them.

#### 3.6 Downstream Consumers

For types/interfaces the spec modifies, grep all import sites and usage points. Record blast radius. For new enum variants:

- **Trait bounds**: Check derives. Verify new variant fields satisfy existing bounds. Note `#[allow(clippy::large_enum_variant)]` size implications.
- **Exhaustive match analysis**: Grep for pattern matches on existing variants to find all match sites needing a new arm. Especially important for enums matched across multiple crates.

#### 3.6A Goal Infrastructure Validation

For specs adding new `GoalKind` variants, verify the spec addresses all mandatory integration points:

1. **GoalDispatchKey**: New variant in `GoalDispatchKey` enum, added to `ALL` constant, and `from_goal_kind` match arm (`goal_dispatch_key.rs`).
2. **GoalDispatchDeclaration**: Entry in `goal_dispatch_decl.rs` with `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`, `progress_barrier_ops`, and `family_policy`.
3. **GoalKindPlannerExt**: Implementation of all trait methods in `goal_model.rs` — `relevant_op_kinds`, `is_satisfied`, `apply_planner_step`, `goal_relevant_places`, `prerequisite_places`, `matches_binding`, `candidate_is_available`, `build_payload_override`, `ranked_goal_provenance_family`, `relevant_observed_commodities`, `is_progress_barrier`.
4. **Ranking integration**: Priority class (`GoalPriorityClass`) and `motive_score` computation formula.

Flag each missing item as a HIGH Issue. If the spec says "reuse existing travel planning" or similar, verify it still names the specific ops and dispatch types.

See `references/worldwake-validation-patterns.md` for additional project-specific patterns.

#### 3.7 Crate Boundary Validation

Verify proposed functions' parameter/return types are accessible from the target crate. Check `Cargo.toml` dependencies. Flag violations of workspace layering (`core -> sim -> systems -> ai -> cli`).

#### 3.8 Upstream Spec References

Grep active specs in `specs/` for references to this spec's deliverables. Note affected specs.

#### 3.9 Behavioral Claim Validation

For each claim about who reads/writes a type at runtime, grep all call sites and classify as runtime vs. test-only (`#[cfg(test)]`). Flag contradictions as CRITICAL. If technically wrong but safe (e.g., caller only reads current-tick data), note both the correction and safety argument.

#### Agent Delegation

For specs with many references, launch parallel Explore agents organized by theme (e.g., action/type references, AI/test references, dependencies/infrastructure). Choose themes to minimize cross-agent dependencies. Typical: 1 agent for 10-15 references with a single domain, 2-3 agents for 15+ references spanning multiple domains. Max 3 agents.

Guidelines:
- After results arrive, cross-reference findings against the spec's type assumptions and formulas. Agents validate existence; you validate semantic compatibility.
- For static lookup tables indexed by discriminator enums, verify key granularity matches discrimination needs.
- Spot-check agent claims with direct Grep/Read before including in findings — agent results are leads, not facts. Especially spot-check when an agent reports a referenced type as "does not exist" or "needs to be created" — verify whether the spec used a wrong name for an existing type before accepting the agent's conclusion.
- In plan mode, Explore agents are inherently compatible (read-only).
- For structural refactor specs (type c), direct agents toward discrepancy checking (counts, symbol existence, blast radius) rather than broad exploration.

Do not present findings yet. Collect everything for Step 4.

### Step 4: FOUNDATIONS.md Alignment Check

#### 4.0 Internal Contradictions

Before checking FOUNDATIONS, scan for contradictions between the spec's Design Goals, Non-Goals, FOUNDATIONS Alignment table, and Deliverables. If the spec includes a Stored State vs. Derived Read-Model table, verify consistency with FND-27 and FND-3.

#### 4.1 Alignment Table Verification

If the spec has a FOUNDATIONS Alignment table, verify each entry. Check that principle numbers match names in `docs/FOUNDATIONS.md` — misnumbered principles are common. Flag mismatches as Issues.

#### 4.2 Missing Principles

Identify Foundation principles the spec should address but doesn't. Pay particular attention to:
- **P1** (Maximal Emergence) — authored sequences or magic triggers?
- **P7** (Locality) — agents querying global state?
- **P14** (World State != Belief State) — agents reading authoritative state directly?
- **P26** (Systems Interact Through State) — cross-system direct calls?
- **P28** (No Backward Compatibility) — compatibility shims or deferred migration?
- **P30** (Causal Hooks Declaration) — count items from source each time (list may evolve). Full 18-item checklist for new system specs; bugfix/lifecycle/architecture-fix specs need only the relevant subset (typically: information-path, positive-feedback, stored state).

#### 4.3 Record Alignment Issues

Record each issue with specific Foundation number and conflict.

#### 4.4 Authoritative-to-AI Impact Rule

If the spec modifies action preconditions, `validate_*` functions, affordance generation (`enumerate_*_payloads`), or `can_exercise_control`, verify all 7 CLAUDE.md checklist points: `get_affordances`, `generate_candidates`, `search_plan`, `BestEffort` action start, `handle_plan_failure`, payload revalidation (`with_payload_override_validator`), and golden test pass.

### Step 5: Classify Findings

Organize findings from Steps 3 and 4 into:

- **Issues**: Factually wrong, stale, violates FOUNDATIONS, or proposes redundant deliverables when existing infrastructure suffices. Blocks ticket decomposition.
- **Improvements**: Not wrong, but a refinement would make implementation cleaner, safer, or more aligned.
- **Additions**: Beneficial features not in the spec that align with its goals. Apply YAGNI — only natural extensions of the spec's scope.

For each finding, record:
- What the spec says (or omits)
- What the codebase actually has (with file paths and line references)
- The recommended change

Tag severity: CRITICAL (blocks tickets), HIGH (fix before tickets), MEDIUM (improves quality), LOW (nice to fix).

### Step 6: Present Findings

Present in this format:

```
## Reassessment: <spec-name>

### Issues (must fix)
[If none: "No issues found."]
1. **[SEVERITY] <title>** — <spec says> vs. <codebase has>. Recommendation: <change>.

### Improvements (should fix)
[If none: "No improvements found."]
1. **[SEVERITY] <title>** — <current text> could be improved because <reason>. Recommendation: <change>.

### Additions (consider adding)
[If none: "No additions proposed."]
1. **[SEVERITY] <title>** — <what's missing> because <reason>. Recommendation: <new section>.

### FOUNDATIONS.md Alignment
- <Foundation N>: <aligned | see Issue #N [SEVERITY]>

### Authoritative-to-AI Impact Rule
[Only if Step 4.4 triggered: list 7 checklist points with pass / N-A / flag status. Otherwise omit.]

### Questions
[If none: "No questions."]
1. <question>
```

#### Question Handling

- **Initial report**: At most 3 questions. If more, prioritize blockers and defer rest to follow-up.
- **Interdependent questions**: Present as a single combined question with labeled option combinations.
- **Discrete options (2-4)**: Use `AskUserQuestion` with a recommended default.
- **Open-ended questions**: Present as plain text in the report.
- **Follow-up rounds**: One question at a time. If answers raise new questions or invalidate findings, present a follow-up round (same format). Repeat until resolved.
- **Delegated resolution**: If the user delegates (e.g., "you decide based on FOUNDATIONS"), resolve by reasoning against the referenced constraint. If resolution requires additional codebase investigation, perform a mini Step 3 scoped to the question. If none of the original options are ideal, propose a new option with justification — scope investigation to 1-3 targeted checks. If the new option affects the dependency graph or crate boundaries, present as a new finding first. In plan mode, the new option is included in the plan file and ExitPlanMode approval covers it.

Wait for user response before proceeding to Step 7. Findings are approved unless explicitly objected to.

### Step 7: Write the Updated Spec

After all findings are resolved and approved:

#### Pre-Apply Verification

Run targeted checks to confirm each finding still holds (e.g., grep confirming symbol presence/absence, count validation). If a finding is invalidated, re-present the corrected finding before applying. Do not silently substitute different changes.

#### Apply Changes

- Incorporate corrections from the user's plan approval or question responses.
- Preserve existing structure and voice. Change only what was agreed upon.
- When changes are numerous and spread throughout, a full Write is acceptable.
- If inserting a new deliverable, renumber subsequent deliverables.
- **New deliverable vs. amendment**: When a finding introduces substantial new logic (new mechanism, new type, new event tag), consider a new numbered deliverable rather than expanding an existing one. Criteria: (a) distinct implementation site, (b) independently implementable and testable, (c) would make existing deliverable unwieldy if inlined.
- If new deliverables introduce actions, components, or system functions, update Section H for P30 compliance. Also update Section H's information-path and stored-state entries when reassessment changes the causal mechanism.
- If the user requests corrections after reviewing, apply them and re-present affected sections.

#### Post-Apply Confirmation

Grep the updated spec for: (1) eliminated stale references (should return zero matches), and (2) corrected references (should return expected matches). Record results for Step 8.

### Step 8: Final Summary

Present:

- Number of issues fixed, improvements applied, additions incorporated
- Change inventory: all changes grouped by finding type (mirroring Step 6 structure)
- Post-Apply Confirmation results (e.g., "Verified: zero matches for eliminated references, N matches for corrected references")
- Deferred items the user chose not to address
- Items excluded by reassessment-driven scope changes (distinct from user-deferred) — note why. Omit if none.
- 1-3 sections that changed most substantially, with a note to review before proceeding
- Suggested next step: `/spec-to-tickets <spec-path> <NAMESPACE>`

Do NOT commit. Leave the file for user review.

## Guardrails

- **FOUNDATIONS alignment is mandatory**: Never approve a spec change that violates a Foundation principle, even if requested — flag the conflict instead.
- **Codebase truth**: All references in the updated spec must be validated. Never propagate stale paths, renamed types, or removed functions.
- **No scope creep**: The deliverable is the updated spec file. Do not write design docs, create tickets, or start implementation.
- **No approach proposals**: Validate and refine the existing design, not greenfield alternatives. Exception: when the approach violates a crate boundary, FOUNDATIONS principle, or critical invariant, propose minimum viable alternatives as part of the Issue finding.
- **Substantial redesign flag**: If reassessment changes >50% of deliverables' approach, flag in Step 6: "This reassessment proposes substantial redesign of N/M deliverables. Goals preserved but implementation path changes significantly."
