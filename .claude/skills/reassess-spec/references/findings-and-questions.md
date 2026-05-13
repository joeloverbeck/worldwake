# Findings Classification and Presentation (Steps 5-6)

## Step 5: Classify Findings

Organize findings from Steps 3 and 4 into:

- **Issues**: Factually wrong, stale, violates FOUNDATIONS, or proposes redundant deliverables when existing infrastructure suffices. Blocks ticket decomposition.
- **Improvements**: Not wrong, but a refinement would make implementation cleaner, safer, or more aligned.
- **Additions**: Beneficial features not in the spec that align with its goals. Apply YAGNI — only natural extensions of the spec's scope.

For each finding, record:
- What the spec says (or omits)
- What the codebase actually has (with file paths and line references)
- The recommended change

Tag severity: CRITICAL (blocks tickets), HIGH (fix before tickets), MEDIUM (improves quality), LOW (nice to fix).

## Step 6: Present Findings

**Redesign-count checkpoint (before drafting the presentation)**: Count two values: (a) **redesign count** — deliverables whose approach was materially changed by the reassessment (eliminated, replaced with a different mechanism, or restructured such that the implementation path is not a refinement of the original), as `N_r / total_original`; (b) **addition count** — net-new deliverables the reassessment adds to the spec, as `N_a` (also expressed as percentage of `total_original`, e.g., `4 added (44% of original)`). The Substantial Redesign Flag section below MUST appear in the output immediately above Questions when **either** trigger fires: redesign count `N_r / total_original > 50%`, **or** addition count `N_a / total_original > 25%`. If neither trigger fires, omit the Substantial Redesign Flag section entirely. Emit both counts as one-line notes at the top of the Step 6 `### Classification` block regardless of whether the flag fires — e.g., `Redesign count: 1/6 deliverables materially changed (below 50% threshold). Addition count: 4 added (67% of original — exceeds 25% addition trigger; Substantial Redesign Flag fires).` — so the checkpoint's decision is auditable from the user-facing report.

**Material-change boundary — anchoring examples**: Borderline cases appear frequently, so the material-vs-refinement boundary needs concrete anchors. Examples of *refinements that do not count as material*: field renames (`observed_at` → `acquired_tick`), type-shape adjustments that preserve the deliverable's read-model role (`BeliefSet<Vec<T>>` → `Vec<BeliefValue<T>>`), signature-preserving parameter reorderings, prose rewording of the deliverable's framing without changing what the implementation does. Examples of *material changes*: elimination of a deliverable, replacement of its mechanism (e.g., derived from stored field X becomes computed from physical process Y), restructuring that changes the set of crates or call sites the deliverable touches, changing the read/write direction of data flow, introducing a new authoritative state where the original was a derived view (or vice versa).

**Worked example — redesign and addition counting**: Suppose a spec has 7 original deliverables and the reassessment surfaces the following:

| Deliverable | Disposition | Material change or refinement? |
|-------------|-------------|--------------------------------|
| D1 (extend `Discrepancy` with `clearing_condition()` method) | Eliminated — existing per-instance `DiscrepancyClearing` already serves the role | **material** (elimination) |
| D2 (define new `CausalLink` type) | Kept; collection type swapped `SmallVec` → `Vec` | refinement (signature-preserving substitution) |
| D3 (extend `PlanGuard` with new field) | Kept; field-type narrowed | refinement (field-type adjustment) |
| D4 (new `plan_repair` module) | Kept; module's variant set restructured from 6 to 5 variants with cross-crate migration of existing variants | **material** (restructuring changes the set of crates and call sites touched) |
| D5 (revalidation routing) | Kept; routing now passes through `attempt_repair_then_replan` before `handle_current_step_failure` | **material** (changes read/write data flow) |
| D6 (extend `RepairMemory` with new fields) | Kept; existing field is migrated, not augmented | **material** (replaces mechanism — single-truth migration vs. dual-field coexistence) |
| D7 (extend observer rendering) | Kept; format tweaked | refinement (prose rewording) |

Redesign count: **4/7 = 57%** (D1, D4, D5, D6) — exceeds 50% threshold, fires the Substantial Redesign Flag.

Additions surfaced by reassessment (net-new deliverables added on top of the 7 original):
- D8 (migrate existing `RepairKind` variants across ~20 call sites) — addition
- D9 (define new `BreachSignature` type) — addition
- D10 (extend `decision_trace.rs` with `RepairAttemptTrace`) — addition
- D11 (extend `CognitiveProfile` with `repair_budget_fraction`) — addition
- D12 (extend `PlanningFact`/`RecordTopic` typing) — addition
- D13 (Section H abbreviation note) — addition

Addition count: **6/7 = 86%** — exceeds 25% threshold, also fires the Substantial Redesign Flag (compounded).

Note that D2 and D3 above land in the "refinement" column even though both modify a deliverable's pseudocode — the criterion is whether the implementation path remains a refinement of the original (signature swaps, field-type narrowing) versus a material change to the mechanism, crate set, or data flow. Borderline cases (e.g., D7 — adds a new rendering case to existing observer section) default to refinement when the rendering site already exists and the change is additive at the prose level.

Present in this format:

```
## Reassessment: <spec-name>

### Classification
<spec type (a)-(e)> — <one-line description>. Steps applied: <list>. Steps skipped: <list>.

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

### Authoritative-to-AI Impact Analysis
[Only if Step 4.4 triggered. Otherwise omit. Canonical heading text for both the Step 6 findings report and the Step 7 spec-file section is `Authoritative-to-AI Impact Analysis` — use this exact wording in both surfaces so downstream spec-to-tickets literal-match heading checks resolve cleanly. Format each point as:]
1. `get_affordances` — pass | N/A | **flag** (reason)
2. `generate_candidates` — pass | N/A | **flag** (reason)
3. `search_plan` — pass | N/A | **flag** (reason)
4. `BestEffort` action start — pass | N/A | **flag** (reason)
5. `handle_plan_failure` — pass | N/A | **flag** (reason)
6. Payload revalidation — pass | N/A | **flag** (reason)
7. Golden tests — pass | N/A | **flag** (reason)

### Substantial Redesign Flag
[If redesign trigger fires (>50% of original deliverables change approach): "This reassessment proposes substantial redesign of N_r/M deliverables. Goals preserved but implementation path changes significantly."]
[If addition trigger fires (>25% of original deliverables added as net-new): "This reassessment expands the deliverable surface by N_a additions on top of M original deliverables. Goals preserved but ticket-decomposition surface grows significantly."]
[If both triggers fire: combine both lines into one paragraph.]
[If not triggered: omit section.]

### Questions
[If none: "No questions."]
1. <question>
```

**Finding-key convention**: In Step 7's Pre-Apply Verification table and Step 8's status reporting, Issues are keyed `I1, I2, …`; Improvements are keyed `M1, M2, …`; Additions are keyed `F1, F2, …`. Preserve the within-category number from this section (e.g., the third Improvement listed here becomes `M3` in Step 7).

## Question Handling

- **Option fidelity**: Each option that names an existing type, field, or function must cite its current definition (grepped at presentation time), not a summary characterization. The user's approval binds to the option label, so an imprecise label — e.g., describing a field as `BTreeSet<T>` when the actual type is `Vec<WrapperT>` — produces an ambiguously approved fix that the Step 7 pre-apply check must then disambiguate. Ground every option in current code before presenting. When an option's viability depends on a storage mechanism (ECS component vs. runtime struct vs. belief-view accessor), visibility qualifier, or cross-crate reachability, grep the precedent pattern — how comparable types are currently registered, where their definitions live, which crates see them — before presenting. The option's label must accurately describe the mechanism the user would end up approving; a mechanism mismatch discovered at Step 7 pre-apply verification forces a mid-apply reframe and erodes the consent contract. When an option describes a *transformation* that yields a target set (rename, subsume, add, remove), enumerate the exact final set explicitly in the option label — e.g., "Final 5 variants: A, B, C, D, E" — rather than letting the final count be inferred from a sketch of renames and subsumptions. Implicit subsumption math (where the count given in the option label and the count derivable from the transformation sketch disagree) produces consistency-check ambiguity at Step 7 that the user's binary approval cannot arbitrate.
- **Initial report**: At most 3 questions. If more, prioritize blockers and defer rest to follow-up.
- **Interdependent questions**: Present as a single combined question with labeled option combinations.
- **Discrete options (2-4), single question**: Use `AskUserQuestion` with a recommended default.
- **Discrete options (2-4), bundled (2–3 questions in one review round)**: Prefer plain-text bullets with labeled options `(a)/(b)/…` and a recommendation per question, under a single `### Questions` heading. This reads more cleanly than multiple `AskUserQuestion` calls and lets the user answer inline (e.g., "1) a, 2) b, 3) proceed with recommendation"). Use `AskUserQuestion` only when a single discrete-option question stands alone.
- **Open-ended questions**: Present as plain text in the report.
- **Follow-up rounds**: One question at a time. If answers raise new questions or invalidate findings, present a follow-up round (same format). Repeat until resolved.
- **Delegated resolution**: If the user delegates (e.g., "you decide based on FOUNDATIONS"), resolve by reasoning against the referenced constraint. If the user delegates with a stated soft criterion rather than a hard constraint (e.g., "choose what's best for completeness" / "pick whichever is simpler" / "go with whatever is cleanest"), treat the criterion as one lens among others (FOUNDATIONS principles, blast radius, FND-28 compliance, implementation cost) and explicitly cite the lens that drove the chosen option in the pre-apply table's `Check` column so the user can audit the reasoning. If resolution requires additional codebase investigation, perform a mini Step 3 scoped to the question. When that mini-investigation produces evidence that drives the final choice (grep results, signature checks, call-site counts), emit the key findings in chat before committing to an option — this parallels the pre-apply table's visibility convention and lets the user see the evidence behind the auditor's pick. A delegated resolution may require investigation comparable in scope to an original Step 3 sub-step (e.g., tracing through 3+ files across multiple crates). If the investigation touches >3 files, consider launching a focused Explore agent rather than manual reads. If none of the original options are ideal, propose a new option with justification — scope investigation to 1-3 targeted checks. If the new option affects the dependency graph or crate boundaries, present as a new finding first. In plan mode, the new option is included in the plan file and ExitPlanMode approval covers it.
- **Conditional approval**: If the user's answer approves an option contingent on a verifiable premise (e.g., "proceed with (a) as long as we truly need those three new accessors", "go with (b) if X is the right entry point"), treat the premise as a mini Step 3 scoped to the condition — grep/read the relevant code, confirm or refute the premise, and proceed only if confirmed. Surface the verification outcome explicitly in the Step 7 Pre-Apply Verification Table row for the affected finding (e.g., `| I1 | need-verification per Q1 condition | confirmed: all three accessors fill gaps not covered by existing accessors — citations <file:line>, <file:line> |`). If the premise is refuted, re-present the affected finding with a corrected recommendation and wait for follow-up approval before applying edits. A conditional approval is not a blanket approval: the condition is part of the contract.

Wait for user response before proceeding to Step 7. (In plan mode, this wait is replaced by ExitPlanMode approval — see `references/plan-mode.md`.) Findings are approved unless explicitly objected to.
