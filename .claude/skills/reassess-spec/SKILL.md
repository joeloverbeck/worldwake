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

If plan mode is active, load `references/plan-mode.md`.

## Process

Follow these steps in order. Do not skip any step.

### Pre-Process: Spec Classification

Before beginning Steps 2-3, classify the spec:

- **(a) New system** — introduces new components, actions, goal kinds, or information paths. Full checklist applies.
- **(b) System extension** — extends existing components, actions, or enums without new systems. Steps 3.1-3.8 apply. 4.4 applies if any deliverable modifies action preconditions, validation functions, affordance generation, or candidate emission. Skip 3.9 if no behavioral claims about runtime readers/writers. Section H may be abbreviated to cover only the declarations the extension changes (typically 4–8 of the 18); the original system spec(s) carry the full 18-point coverage. Section H updates only for new deliverable sections. Flag specs that claim full 18-point coverage but provide only a subset as a wording-fix Improvement.
  - **Tooling-only variant** (observer binary, CLI enhancements, diagnostic tools): Steps 3.1-3.4 fully apply. Steps 3.5-3.7 apply only if the spec extends existing types or enums; if the spec adds only new functions/structs local to the tooling binary, 3.5-3.7 are N/A. Step 3.8 (upstream spec references) still applies. Skip 3.9. Downstream consumer analysis (3.6) can be limited to the tooling binary. A brand-new crate qualifies as (b)-tooling-only when its scope is read-only consumption of existing public APIs and it introduces no new simulation state — the crate boundary is incidental, not load-bearing.
- **(c) Structural refactor** — trait/module restructuring with no behavioral changes. Skip Steps 3.5, 3.9, 4.4; Section H is N/A (do not require one; if the spec authored one, preserve it unchanged unless a finding targets it directly). Focus on symbol existence, count accuracy, and blast radius.
- **(d) Test-only** — adds golden tests, benchmarks, or test infrastructure without modifying production code.
  - Steps 3.1-3.4 apply (validate referenced paths, types, functions, dependencies).
  - Skip 3.5-3.9 (no production code changes to trace).
  - Step 4 applies but 4.4 is N/A.
  - Section H updates are N/A unless the test reveals a missing causal hook.

- **(e) Investigation/bugfix/optimization** — diagnoses a root cause and proposes targeted fixes, no new systems or components. Covers both hypothesis-driven investigations (conditional fixes) and proven-diagnosis specs (single concrete fix confirmed by existing tests). Also covers computation-optimization specs that add new planner-internal algorithms, heuristics, or filter/pruning logic without new world-facing state, as well as storage-layer performance fixes (deduplication, indexing, amortization) that change how data is stored or iterated without altering what the data means.
  - Steps 3.1-3.4 apply (validate all referenced paths, types, functions, dependencies).
  - Steps 3.5-3.8: For investigation/bugfix specs, apply only to proposed fix deliverables (not to hypothesis text). For computation-optimization specs, apply to all deliverables (there is no "hypothesis text" — all deliverables are implementation targets).
  - Step 3.9 applies if claims about runtime behavior are made.
  - Step 4 applies; 4.4 applies if any proposed fix touches action preconditions.
  - Section H updates only if the change introduces new causal hooks.
  - **Root-cause tracing (Step 2)**: The structured root-cause tracing substeps (a-d) apply to investigation/bugfix specs. For computation-optimization specs, skip root-cause tracing — instead prioritize validating that the spec's referenced types, functions, and integration points exist and have the assumed signatures and semantics.

- **(f) Retroactive reassessment** — reassessment concludes (via Step 3 validation) that all deliverables already landed through downstream tickets. This classification is **not pre-selected**; it activates when every deliverable verifies as implemented in code. The user hint "I suspect this already landed" is a soft signal, not a classification by itself — only Step 3 evidence can confirm (f).
  - Steps 3.1-3.4 apply rigorously to prove landing of every deliverable; cite file paths + line numbers as evidence. Skip Steps 3.5-3.8 (ripple/root-cause substeps) — the work has already shipped, so there is no ripple to trace.
  - Step 4 applies for Outcome-section honesty (does the delivered implementation still align with FOUNDATIONS?).
  - **Step 7 output shape switches to Outcome population + archival**, not deliverable refinement (see Step 7's retroactive branch).
  - **Step 8 suggested next step becomes archival per `docs/archival-workflow.md` + IMPLEMENTATION-ORDER.md reconciliation**, not `/spec-to-tickets` (see Step 8's retroactive path).
  - Classification shift from (a)/(b)/(c)/(d)/(e) → (f) is a legitimate and common outcome when a spec is reassessed after the work already shipped through downstream tickets. Name the shift explicitly in Step 8.
  - (f) does not participate in hybrid combinations — it is outcome-based rather than deliverable-based, and it supersedes the originally-assumed classification once Step 3 confirms full landing.

**Deliverable removal**: If validation reveals a deliverable should be removed entirely, skip remaining sub-steps for that deliverable and record the removal as a finding. If only part of a deliverable (enum variants, struct fields, sub-items) should be removed, record the partial removal as a finding but continue sub-step validation for the surviving parts — the surviving parts still need cross-reference and downstream-consumer checks. Continue validation for all surviving deliverables.

**Hybrid specs**: Apply the union of applicable steps — use the most rigorous classification's checklist for shared steps. Three-way and four-way hybrids compose the union of the named two-way rules; when constituent rules conflict on a shared step, apply the most rigorous per the leader sentence above. Common hybrids:
  - **(d)+(e)** (test triage with a bugfix): Steps 3.1-3.4 from both; 3.5-3.8 for bugfix deliverables only; 4.4 if bugfix touches candidate emission/preconditions; Section H only for bugfix deliverables.
  - **(b)+(d)** (system extension with golden tests): Full (b) checklist for production deliverables; (d) rules for test deliverables; 4.4 if any production deliverable modifies validation/emission.
  - **(b)-tooling-only + (d)** (tooling/report/observer enhancement with test-support helpers): Steps 3.1-3.4 apply fully; 3.5-3.7 apply only if the spec extends cross-crate types or enums; 3.3A applies if the spec proposes new observer/CLI output; 3.8 still applies; skip 3.9; Section H updates only for new causal hooks. Check the "Dual-Use Read-Model Types" pattern in `references/worldwake-validation-patterns.md` if the spec proposes types shared between tests and a non-test crate.
  - **(b)+(c)** (system extension with structural refactor — e.g., a refactor that introduces new public types or extends an existing struct with a new field as part of restructuring behavior): Full (b) checklist for the new types/fields and any cross-crate consumers (3.6 applies). (c) skip rules apply for shared steps that are non-behavioral: skip 3.5 if no new components are introduced, skip 3.9 if no behavioral claims about runtime readers/writers are made, skip 4.4 if no precondition/validation semantics change. Section H is N/A — if the spec authored one, preserve it unchanged unless a finding targets it directly.
  - **(c)+(d)** (structural refactor with compile-fail doctests or grep-regression tests): Full (c) checklist for production deliverables; (d) rules for test deliverables; Section H remains N/A.
  - **(a)+(d)** (new system with test infrastructure): Full (a) checklist; test deliverables validated per 3.1-3.4 only.
  - **(a)+(b)** (new system with migration of existing types/enums): Full (a) checklist. For migration deliverables, additionally verify existing call sites and exhaustive-match sites per 3.6 cross-crate analysis (applies rigorously even though (a) alone doesn't always trigger it) — migration deliverables need blast-radius accounting for every site that matches on the removed or renamed symbol. Step 3.9 applies only if the spec makes behavioral claims about the *current* runtime's reader/writer interactions (not claims about the proposed post-implementation behavior). Design-intent claims about future behavior are validated through pseudocode fidelity (3.3) and deliverable coherence, not 3.9.
  - **(a)+(b)** (flat-enum-to-axis migration): A spec that eliminates a flat discriminator enum on an existing component and replaces it with multiple typed fields on the same struct triggers this hybrid even when no entirely new component is introduced. The new field types and their supporting enums are the (a) component; the cross-crate elimination of every match/equality site on the removed flat enum is the (b) component. Apply 3.6 cross-crate consumer analysis on the eliminated enum's symbol — the consumer surface is typically much wider than the deliverable list suggests because flat-enum equality checks scatter across perception, belief, candidate generation, ranking, scenario load, and goldens. Belief-snapshot mirror types (e.g., `Believed<EnumName>`) are common second-tier consumers and need their own migration deliverable.
  - **(a)+(b)** (new event-tag and supporting payload types with cross-crate enum payload widening): Full (b) checklist applies. The new payload types are validated per (a)'s 3.5/3.6 rules even though no ECS component is introduced. Section H may be abbreviated to cover only the new declarations. The trigger is the event-payload class — historical-record types added to core to support a new `EventTag` variant, optionally combined with widening an existing variant on a sibling cross-crate enum (e.g., `BlockingFact`) to carry the new event reference. Apply the "Existing Variant Payload Widening" and "New Enum Variant on Cross-Crate Enum" patterns from `references/worldwake-validation-patterns.md` for the widened/extended enum surfaces.

**Emergent migration at Step 7**: If Step 7 edits introduce migration of an existing type across crates that was not part of the original spec (typically surfaced by the pre-apply verification table or the bundled-answer consistency check), re-promote the reassessment to `(a)+(b)` and run 3.6 cross-crate consumer analysis on the migrating type before finalizing edits. Record the scope extension in the pre-apply table per the scope-extending tier (see Step 7's Pre-Apply Verification Table section), so the user sees the classification shift in the same pass as the edit.

**Re-reassessment shortcut**: If the same spec was reassessed earlier in this session and not externally modified, Steps 2-3 may scope to only references affected by the triggering change. Step 1 still applies.

**Self-authored spec note**: Full validation is required even for specs authored earlier in this session — authoring may introduce unchecked assumptions.

### Step 1: Mandatory Reads

Read ALL of these before any analysis:

1. **The spec file** (from the argument) — entire file
2. **`docs/FOUNDATIONS.md`** — skip if read earlier in this session and unmodified. If the file exceeds the Read tool's token limit, read in sections (e.g., 200 lines each) to cumulatively cover the full document, or target specific principle sections relevant to the spec's domain.
3. **`docs/spec-drafting-rules.md`** (if the spec contains or should contain Section H) — skip if read earlier and unmodified. Skip for classification (f) — deliverable refinement and Section H editing do not occur in the retroactive branch.

Parse the spec's metadata: Phase, Status, Priority, Crates, Dependencies, Goals/Design Goals, Non-Goals, FOUNDATIONS Alignment, and all deliverable sections.

**User invocation directive parsing**: If the user's invocation arguments include a quoted directive after the spec path (typical pattern: a `"Note: …"` or instruction string following the path), parse the directive as an additional Step 3 verification target. Surface the directive's verification outcome in Step 6 (as an Issue, Improvement, or finding as appropriate) and reference the directive explicitly in Step 8 so the user sees how it was handled. Common directive shapes: dependency check ("verify no hidden dependency on unimplemented sibling specs"), scope boundary check ("confirm this spec doesn't overlap with X"), alignment check against an external doc ("validate against `docs/Y.md`"). If the directive prescribes a specific action conditional on a finding (e.g., "stop and update specs/IMPLEMENTATION* if dependencies detected"), honor the conditional explicitly in Step 8 — name the directive's condition, state whether it fired, and describe the action taken or not taken.

**Non-numbered deliverables**: If the spec uses phases or named sections instead of numbered deliverables (common in investigation/bugfix and test-infrastructure specs), treat each distinct implementation section as a deliverable for validation purposes. Adapt references to "deliverable numbers" throughout this skill to the spec's actual organizational scheme (phase labels, section headers).

### Step 2: Extract References

Extract every concrete codebase reference from the spec:

- **File paths** mentioned or implied
- **Type names** (e.g., `GoalKind`, `SaleListing`, `PlannerOpKind`)
- **Function names** (e.g., `generate_candidates`, `enumerate_trade_payloads`)
- **Crate/module names**
- **Test file paths or test names**
- **Other specs or tickets** in Dependencies
- **Code examples** (inline code blocks showing API usage, precondition lists, struct definitions) — extract for fidelity checking against actual source
- **Scenario and test configuration files** referenced by the spec (RON scenarios, test fixtures, seed values) — extract profile/parameter values the spec's claims depend on

Build a validation checklist. For specs with >15 references, use `TaskCreate` to add a checklist item per reference and `TaskUpdate` to mark each `validated | drifted | missing`; the external surface catches references that mental tracking can silently drop[^mental-tracking]. When Step 3 will dispatch parallel Explore agents organized by disjoint themes, the agents' report bodies serve as the external surface and per-reference `TaskCreate` may be skipped — agent dispatch is Step 3 validation work and must follow the mandatory reference reads (do not dispatch before reading the references and emitting their content-tied acknowledgments). Prioritize references most likely to have drifted: dependency paths, function signatures, and types the spec extends. Stable types (`EntityId`, `Permille`, `Quantity`) can be spot-checked.

`TaskCreate` may also be used for step-level orchestration (one task per major Step of this skill) when the reassessment is long-running and the operator wants progress visibility; this is independent of the per-reference checklist use and may coexist with it.

[^mental-tracking]: For specs with ≤15 references, mental tracking is acceptable because the working set fits in short-term audit scope. Above that threshold, the external surface is required — batched Grep coverage can mask a silently-dropped reference that only surfaces at post-apply grep (or not at all).

For investigation/bugfix specs (type e, investigation/bugfix subtype), also prioritize the root-cause hypothesis: trace the claimed failure path through actual code to confirm the spec's causal narrative, not just that the referenced symbols exist. Structured root-cause tracing:

- (a) identify each code path the spec claims participates in the bug
- (b) read the actual implementation of each path
- (c) verify the claimed divergence mechanism by comparing inputs, computation methods, and outputs across paths
- (d) check scenario/test configuration for parameter values that trigger the claimed failure

For computation-optimization specs (type e, optimization subtype), skip root-cause tracing — there is no bug hypothesis to validate. Instead prioritize: (a) that all referenced types, functions, and integration points exist with the assumed signatures, (b) that proposed integration sites have the structural shape the spec assumes (e.g., variable availability, loop structure, timing of data collection), and (c) that proposed new types satisfy existing trait bounds at their intended usage sites.

**Proven-diagnosis scoping**: If the spec's diagnosis is already confirmed by existing tests that demonstrate the specific failure mode or by profiling data that quantifies the specific bottleneck (e.g., golden tests asserting `BudgetExhausted` with concrete candidate counts, or profiling showing specific function hot-paths with measured growth rates), root-cause tracing may scope to verifying the fix's assumptions — that the proposed remedy targets the right code path and reads the right data — rather than re-proving the diagnosis from scratch. Sub-steps (a-b) still apply; (c-d) may be lighter-weight.

### Step 3: Codebase Validation

**Read `references/codebase-validation.md` and `references/worldwake-validation-patterns.md` now, with the Read tool, before any validation work.** These files carry the validation checklists and the pattern-specific triggers (new GoalKind variant, new component on Agent, new component read by AI crate, new action type, new cross-crate enum variant). Skipping these reads means pattern-specific checklists will be missed and findings produced in that state are incomplete.

After each Read, emit a **content-tied acknowledgment** with a concrete anchor from the file just loaded, not a free-form "Loaded: …" string. Multiple acks for consecutive Reads may be combined into one chat message provided each file's anchor appears separately:

- `Loaded codebase-validation.md — top section is "3.0 Cross-Crate Scope Establishment"`
- `Loaded worldwake-validation-patterns.md — first pattern is "New GoalKind Variant"`
- Combined-line form (when both files are read in sequence): `Loaded codebase-validation.md (top: "3.0 Cross-Crate Scope Establishment") and worldwake-validation-patterns.md (first pattern: "New GoalKind Variant")`

A generic "Loaded: codebase-validation.md, worldwake-validation-patterns.md" without per-file content anchors is treated as a skipped load, because it can be emitted without opening either file. Batching acknowledgments at report time (after Step 3 validation work is done) also defeats the audit trail — the ack must appear in chat before the validation work that depends on the loaded content. Then validate every reference from Step 2, applying any pattern-specific checklist the spec triggers.

**Agent-vs-spec contradiction discipline (early catch)**: When an Explore agent's finding contradicts the spec's claim about a file path, symbol location, crate ownership, or substrate-defining type's residence, direct-verify with `test -f`, `ls`, or a targeted `grep` BEFORE adopting either source into a finding. The spec's claim and the agent's claim are both leads, not facts. This is the early-catch counterpart to Step 7's pre-apply path-existence row — catching the discrepancy at Step 3 prevents the wrong source from shaping downstream classification, recommendations, and the entire finding surface that builds on it. The agent-delegation sub-bullet inside `references/codebase-validation.md` covers the mechanics in detail; this top-level callout exists so the discipline survives even if the operator skims past that sub-bullet.

Do not present findings yet. Collect everything for Step 4.

### Step 4: FOUNDATIONS.md Alignment Check

**Read `references/foundations-alignment.md` now, with the Read tool, before checking alignment.** Emit a content-tied acknowledgment immediately after the Read call — e.g., `Loaded foundations-alignment.md — opens with "4.0 Internal Contradictions"`. A bare "Loaded: foundations-alignment.md" is treated as a skipped load. Then check spec alignment against all applicable principles.

### Steps 5-6: Classify and Present Findings

**Read `references/findings-and-questions.md` now, with the Read tool, before classifying.** Emit a content-tied acknowledgment immediately after the Read call — e.g., `Loaded findings-and-questions.md — opens with "Step 5: Classify Findings"`. A bare "Loaded: findings-and-questions.md" is treated as a skipped load. The file prescribes the one-line finding format and the Step 6 presentation template; using your own format is not a substitute. Then classify all findings from Steps 3-4 and present to the user using that template.

**Option fidelity (mandatory before drafting Questions)**: Each Question option that names an existing type, function, file path, or integration target must be grounded in current code — grep the named symbol's actual implementation **at presentation time** and frame the option label by what the grep returned, not by an assumption about the target's iteration shape, timing, or call surface. A wrongly-framed option leads the user to approve a fix on a false premise; the resulting evidence-refining mismatch at Step 7 erodes the consent contract even when the recommendation still holds. Examples of premise traps to grep against: "runs every tick on every agent" (the SystemFn may iterate places, not agents), "currently iterates X" (the loop may be over Y), "lives in crate Z" (the symbol may have moved), "is a Default field" (the field may require explicit construction). The option-fidelity grep is in addition to the Step 3 codebase validation that already established the symbol exists.

**Redesign-count checkpoint**: Before presenting, count two values: (a) **redesign count** — deliverables whose approach materially changed (eliminated, replaced with a different mechanism, or restructured such that the implementation path is not a refinement of the original) versus total deliverables in the spec as drafted; (b) **addition count** — net-new deliverables the reassessment adds to the spec. A net-new deliverable counts toward the addition count regardless of whether its originating finding is an Issue, Improvement, or Addition — the count tracks deliverable-surface growth, not finding category. A deliverable whose text is reworded but whose approach remains a refinement of the original does not count toward either metric. The `### Substantial Redesign Flag` section is mandatory in the Step 6 output (placed immediately above `### Questions`) when **either** trigger fires: redesign count `> 50%` of original deliverables, **or** addition count `> 25%` of original deliverables (both strict — at-threshold cases like exactly 50% or exactly 25% do NOT fire the flag). Both signals reshape the ticket-decomposition surface; either alone warrants the flag. Emit both counts in your pre-draft notes even when neither trigger fires, so the decision is auditable. When a deliverable's redesign status depends on pending question resolution, emit the range (e.g., `2-3/6`) and name which deliverable(s) are conditional so the reader can resolve the count once the questions close.

Wait for user response before proceeding to Step 7. (In plan mode: after question resolution, write the plan file per `references/plan-mode.md`, then call ExitPlanMode. Steps 7-8 execute after approval.)

**Auto mode interaction**: When auto mode is active and the findings contain no Issues (CRITICAL/HIGH severity or FOUNDATIONS violations) and no open Questions, proceed directly to Step 7. Report the auto-mode auto-approval inline in Step 6 presentation (e.g., "Auto mode: no Issues, proceeding to Step 7"). If any Issue is present or any Question is open, the wait-for-user gate still applies even in auto mode.

**User-level "no clarifying questions" instruction interaction**: When the session carries a user-level "work without stopping for clarifying questions" instruction (typically delivered via a `<system-reminder>` or prior message) AND the reassessment surfaces substantial scoping or design-shape Issues, the Step 6 wait gate still applies — that instruction governs trivial calls and low-stakes judgment, not contract-shaping decisions where the user's binary approval is the only signal that disambiguates the next-step path. The "reasonable call and continue" pattern applies to trivial picks (minor symbol renames, severity-tier nudges), not to questions that change the spec's identity or split it across multiple specs.

When this condition fires, **MUST emit one framing line immediately before the `### Questions` heading** that names both (a) the active session-level no-clarifications instruction and (b) the specific reason the wait gate applies. Without this framing, the bare question presentation can be misread as the auditor having missed the no-clarifications instruction entirely. The framing is not optional and is not satisfied by adjacent text that merely closes with "Please respond inline" — the framing must explicitly name the instruction-versus-gate interaction. Example phrasing (adapt to the situation):

> Despite the session-level no-clarifications instruction, this scoping decision reshapes the deliverable surface and warrants approval — I'll proceed with the recommended defaults if no response within the loop.

Other valid phrasings name the instruction explicitly (e.g., "the active 'work without stopping' instruction governs trivial calls, not contract-shaping splits like this one — flagging for approval") and identify the warranting condition (deliverable-surface change, identity split, FOUNDATIONS-violating recommendation, etc.).

### Step 7: Write the Updated Spec

#### Pre-Apply Verification Table

Before editing, build a per-finding verification mini-table **and emit it in chat before calling Write/Edit**. For each finding (by its Step 6 key — `I1`, `I2`, `M1`, `F1`, etc.), run a targeted check (grep, count, path existence) and record both the command and the result. The table is the gate — a vague "I checked the findings" is not sufficient and will be treated as no verification.

**Spec-file enumeration row (mandatory for findings that eliminate or rename a named term)**: When a finding eliminates or renames a type, function, variant, struct field, or other named symbol from the spec, one row of the pre-apply table MUST grep the **spec file itself** for every appearance of the term so all edit sites are enumerated before the first Edit call. Codebase-evidence rows confirm the codebase reality (e.g., "no such type exists outside the spec"); a separate spec-file row enumerates the in-spec edit surface. Without this row, post-apply grep typically catches stale survivors only after edits have landed, forcing a second cleanup pass. Example row: `| I3 | grep -n "CognitiveProfileDef" specs/S130-...md | 3 sites: D9 line 305, Crates line 17, Component Registration line 377 — all need coordinated edits |`. The two row types are complementary, not substitutes.

**Full-Write strategy carve-out**: When the apply strategy is a full `Write` (the spec writing rules permit this when changes span >50% of deliverables, or when insertions cause cascading renumbering), the per-finding spec-file enumeration row may be replaced by the post-apply confirmation grep — the rule's purpose (enumerating all edit sites before the first `Edit` call) is moot when the entire spec is one `Write` call that replaces the file's content wholesale. The codebase-evidence rows and the path-existence row remain required regardless of strategy. State the strategy choice (full `Write` vs. targeted `Edit`s) explicitly when emitting the pre-apply table so the carve-out is auditable.

**Path-existence row (mandatory for any path that will be written into the new spec body)**: For each file path that will appear in newly-added or substantially-rewritten deliverable text — including paths surfaced by Step 3 Explore agents and adopted into deliverables, paths cited as locations of consumer migrations, and paths named in newly-introduced Cross-System Interaction prose — one row of the pre-apply table MUST run `test -f <path>` (or `ls <path>`) and record the result. The post-apply file-path check in `references/spec-writing-rules.md` exists as a safety net, but catching path errors at the pre-apply gate avoids writing them into the spec in the first place. This is especially important for agent-reported paths: agents commonly canonicalize a filename (e.g., `perception.rs`) without canonicalizing its containing crate (e.g., reporting it under `worldwake-ai/src/` when the actual file lives in `worldwake-systems/src/`). Example row: `| D5 | test -f crates/worldwake-systems/src/perception.rs && test -f crates/worldwake-core/src/belief.rs | both exist — paths corrected from agent's `worldwake-ai/src/` claim before adoption |`.

**Many-path batching allowance**: When the spec introduces 5+ new paths across multiple findings or deliverable sections, the path-existence verification may be performed via a single Bash batch (`test -f <path1> && test -f <path2> …` or a `for f in <paths>; do test -f "$f" …; done` loop) emitted in chat immediately before the pre-apply table, with per-finding rows then citing the verified paths by `path:line` anchor (e.g., `| F3 | crates/worldwake-core/src/belief.rs:69 for SaliencePolicy | confirmed by path-batch above |`). The literal `test -f` (or `ls`) command must still appear in chat so the verification is auditable — only its row-placement is relaxed. Below the 5-path threshold, inline `test -f` rows are still preferred because they keep each finding's evidence self-contained in the table.

**Full-Write strategy path-batching**: When the apply strategy is a full `Write` (rather than targeted `Edit`s), batch all file paths cited in the rewritten body — the full-Write rewrites the entire spec body so every cited path is effectively new relative to the surviving text, and per-path inline rows lose their evidentiary advantage. The post-apply file-path check confirms each landed correctly. The literal `test -f` (or `ls`) batch command must still appear in chat immediately before the pre-apply table so the verification is auditable.

Example:

| Finding | Check | Result |
|---------|-------|--------|
| I1 | `grep -n "pm(750)" crates/worldwake-ai/tests/golden_survival_*.rs` | 0 matches — confirms stale constant eliminated |
| I2 | `grep -rn "AnomalyKind::" crates/worldwake-cli/src/` | 17 matches, all in `bin/observer.rs` — no external consumers to migrate |
| Q1=(a) approved | Final variant set per Q1 resolution | confirmed: `RebindTarget, ReplaceProvider, InsertVerification, DowngradeToProgressBarrier, Abandon` (5 variants, `AlternateRoute` folds into `ReplaceProvider`) |
| M3 | `test -f specs/S118-stuck-agent-detector-active-frame-exclusion.md` | file exists — dependency path valid |
| F1 | `grep -n "## Section 11" crates/worldwake-cli/src/bin/observer.rs` | 0 matches — Section 11 unused; safe insertion identifier for the new addition |

If a check reveals a mismatch with a finding, classify the mismatch and respond accordingly:

- **Recommendation-changing mismatch**: the pre-apply check invalidates the finding's *recommendation* — the fix that was approved no longer applies, the target text/symbol has moved, or a different fix is now warranted. Re-present the corrected finding to the user and wait for confirmation before applying any edit **for that finding**. Do not silently drop or modify the finding. If the correction is a pure retraction (no substitute fix is warranted — the finding itself is withdrawn), note the retraction transparently in the pre-apply table with classification `retracted: <reason>` and proceed with the remaining approved findings; fresh re-approval is only required when a different fix is being substituted in place of the retracted one, not when the finding is simply withdrawn.
- **Evidence-refining mismatch**: the pre-apply check refines the finding's *supporting evidence* (e.g., a symbol the finding claimed was absent turns out to exist in a different location) but the recommendation still holds unchanged. Note the refinement inline in the Result column of the pre-apply table (e.g., "partial invalidation: symbol exists at <path>:<line>, not at spec-claimed location — recommendation unchanged") and proceed. The user sees the refinement in the emitted table, so this is not silent modification.
- **Scope-extending mismatch**: the approved recommendation still applies, but fulfilling it requires adding a new deliverable, migration, or subsystem change not discussed at question time. The recommendation was not refuted and the evidence was not merely refined — the scope grew. Note the scope extension inline in the Result column of the pre-apply table (e.g., "scope-extending: requires new D4 to relocate `MaterializationTag` from sim to core so the payload type can live in core — recommendation unchanged") and proceed. Additionally, surface the scope extension in the Step 8 summary under a dedicated line (or in the classification-shift note) so the user sees it in both the pre-apply table and the final summary. If the scope extension constitutes a cross-crate type migration, also apply the Pre-Process "Emergent migration at Step 7" guidance and run 3.6 cross-crate consumer analysis before finalizing edits.

Example rows for each tier:

| Finding | Check | Result |
|---------|-------|--------|
| I5 (evidence-refining) | `grep -rn "NEEDS_LOW_CEILING"` | exists at `observer.rs:1931`, not at spec-claimed `golden_survival_contested.rs` — recommendation (cite scenario-authored contract field instead) unchanged |
| I3 (recommendation-changing) | `grep -n "#[cfg(test)]"` at claimed line | boundary has moved; the targeted function is now runtime, not test-only — re-present to user before applying |
| D4 (scope-extending) | `grep -rn "ExpectationBasis::"` | 15+ match sites across sim/systems/ai; `ranking.rs:1133` is an exhaustive match — scope-extending: requires cascade arm in `ranking.rs` for the new `PlanStepCompletion` variant — recommendation unchanged |

The `Finding` column tier tag (`evidence-refining`, `recommendation-changing`) is required only when the pre-apply check detects a mismatch with the finding. Rows that confirm the finding exactly as written may use the compact descriptive form shown in the first example table (`I1`, `I2`, `M3`, optionally with a brief parenthetical anchor).

For findings resolved by delegated-question reasoning against FOUNDATIONS (rather than by a codebase symbol check — common for design-decision findings where Q3's answer was "you decide based on FOUNDATIONS"), cite the principle(s) applied and the chosen option in the `Check` column; `Result` names the option the reasoning selected. Example row: `| M2 | FND-21 + FND-28 + FND-20 reasoning; Q3 delegated | selected option (a): subsume `prioritize_same_goal_replan_candidates` into commitment slot — stable-commitment + no-parallel-path + bounded-search all favor (a) |`.

For findings resolved by *internal* reasoning during reassessment itself — no external command run, no user delegation, the finding's nature simply doesn't reduce to a grep (e.g., "this discrepancy field duplicates a value already carried by the assumption payload") — prefix the `Check` column with `Logical:` and state the reasoning concisely. Example row: `| M3 | Logical: `until_tick` in `FrameAssumption::NeedSafeUntilTick` is the same value as the would-be `plan_completion_tick` field in the discrepancy payload | Confirmed redundant; recommendation (drop the field) unchanged |`. The `Logical:` prefix signals to a reader that no codebase evidence was sought because the finding's resolution lives entirely inside the spec's own structure.

For findings resolved by *user-approved option* in a Question round (the user picked option (a)/(b)/etc. in a Q1/Q2/Q3 bundle), cite the question key and chosen option in the `Check` column (e.g., `Q2=(a) approved`); the `Result` column names the consequence the option entails. Example row: `| I5 | Q2=(a) approved | Per-basin split with OpportunityAnchor::Facility(basin_id); recommendation unchanged |`. This pattern is distinct from delegated-question reasoning (where the user delegated to FOUNDATIONS and the auditor selected the option) — here the user's direct selection is the binding answer, so neither FND citation nor `Logical:` reasoning applies.

For findings resolved by *soft-criterion delegated reasoning* (the user delegated with a guiding criterion like "cleanest", "simplest", "most robust", or "FOUNDATIONS-aligned" rather than a hard constraint), cite the delegation, name the criterion, and identify the lens(es) — FND principle, blast radius, FND-28 compliance, implementation cost — that drove the chosen option in the `Check` column. The `Result` column names the chosen option and its consequence. Example row: `| I3 | Q1 delegated with soft criterion "cleanest + FOUNDATIONS-aligned"; FND-28 lens (no parallel authority) selected option (a) | Rename in place; no GoalKindDiscriminant added |`. This pattern is distinct from hard-criterion FND-delegated reasoning (where the user delegated to a specific principle directly, and the row cites the principle but not a soft-criterion name) — here both the criterion and the lens are surfaced so the user can audit how soft guidance resolved to a binding choice.

For Addition (F-key) findings, the row's purpose is to verify the addition's *target site* (e.g., insertion identifier unused, slot exists, host file present) and its *necessity evidence* (typically a back-reference to a sibling I-key finding via `Logical:` reasoning), not an old-vs-new comparison. The check's success condition is "target available" rather than "stale content removed". Pure F-key rows do not carry tier tags (`evidence-refining`, `recommendation-changing`) because there is no existing recommendation to refute or refine.

**Bundled-answer consistency check**: When a single user response resolves multiple interdependent questions (e.g., "1) a, 2) b, 3) a" in one message), verify before building the verification table that the combined answers are internally consistent — no contradictory routing (the same symbol referenced by two answers is routed to the same destination), no dangling type references (a type referenced in one answer is defined by another), no split-brain conditions (a decision in one answer does not leave a remnant addressed by a different answer). Flag any detected contradiction as a recommendation-changing mismatch and re-present the affected findings for a follow-up round before proceeding. When the bundled answer resolves cleanly, emit a single-line acknowledgment in chat before the pre-apply verification table — e.g., `Bundled-answer consistency check: (a)/(b)/(b) are internally consistent (independent routing decisions, no shared symbol references).` This makes the check auditable in the conversation, paralleling the pre-apply table's visibility requirement. Mixed-form responses (some answers are flat picks like `(a)`, others are delegations like "you decide" or conditionals like "if X, choose Y") are valid bundles — apply the consistency check to the *resolved* answer set after delegated/conditional answers have been adjudicated by the auditor per Step 6's Delegated-resolution rule, not to the raw response text. Running the check on raw mixed-form text yields false split-brain reports when a delegation's resolution would have aligned with a sibling pick.

**Read `references/spec-writing-rules.md` now, with the Read tool, before writing.** Emit a content-tied acknowledgment immediately after the Read call — e.g., `Loaded spec-writing-rules.md — opens with "Pre-Apply Verification"`. A bare "Loaded: spec-writing-rules.md" is treated as a skipped load. The file carries the full pre-apply verification, apply-changes, and post-apply confirmation rules. Then apply all approved changes.

**Retroactive branch (classification (f))**: If Step 3 validation concluded all deliverables already landed, Step 7's output shape is **not** deliverable refinement. Instead:

1. Flip the spec's **Status** to `✅ COMPLETED`.
2. Populate the **Outcome** section with: completion date; landed changes (cite file paths + line numbers); delivering ticket(s); deviations from original plan (especially work absorbed by sibling specs); verification commands **re-run at reassessment time**, and their pass/fail status. Do not copy verification from the delivering ticket — rerun each command now to catch post-delivery regressions.
3. Mark historical **Motivating Evidence** as such — add a short parenthetical noting the drift described was resolved by the landed implementation, so future readers don't treat a stale condition as a live one.
4. Cross-reference any downstream specs that extended or absorbed original-spec scope (e.g., a later spec that added fields to the original's struct).
5. Do **not** apply structural refinements to deliverables that already shipped — the spec file is now a historical record, and editing D-sections to match current code would confuse the causal narrative.

After Step 7 completes for (f), Step 8 drives archival + IMPLEMENTATION-ORDER.md reconciliation rather than suggesting `/spec-to-tickets`.

### Step 8: Final Summary

Present:

- Number of issues fixed, improvements applied, additions incorporated
- Change inventory: all changes grouped by finding type (mirroring Step 6 structure)
- **Post-Apply Confirmation results**: for every finding that eliminated or renamed a reference, grep-prove it is gone and that corrected references resolve — e.g., "Verified: zero matches for eliminated references, N matches for corrected references". When stale references are caught by the post-apply grep, classify each as **"introduced by the Write"** (new content's internal cross-references — e.g., a `(D7)` reference in a freshly-written deliverable that should have been `(D8)`; only possible to catch post-Write) or **"carried over from the original spec"** (text the Write should have eliminated but didn't — typically caught by scalar/count consistency checks). Note the classification in the Step 8 summary so the operator can calibrate where each class typically slips. For retroactive reassessments (classification (f)), additionally grep every concrete artifact named in the spec's Motivating Evidence (symbols, constants, file-local numbers, old thresholds) and prove its absence or corrected form in the current codebase. This validates the Outcome section's claims are still true at archival time rather than at some earlier point.
- Deferred items the user chose not to address
- Items excluded by reassessment-driven scope changes (distinct from user-deferred) — note why. Omit if none.
- 1-3 sections that changed most substantially, with a note to review before proceeding
- **Classification shift note**: If the reassessment caused the spec's effective classification to shift, name the shift explicitly. Examples:
  - "(a) new system collapsed into (b) system extension after deliverable removal"
  - "(e) investigation was promoted to (a) after a new component proved necessary"
  - "(b) system extension shifted to (f) retroactive reassessment after Step 3 verified full landing"
  This surfaces the change so downstream handling is correct. Omit if the classification is unchanged.
- **Suggested next step**:
  - **Default path** (classifications (a)–(e)): `/spec-to-tickets <spec-path>` — the spec-to-tickets skill will prompt for the ticket namespace. If the reassessment removed or renamed content named in `specs/IMPLEMENTATION-ORDER.md`'s roadmap blurb for this spec (e.g., a deliverable, event tag, or component named in the blurb that was dropped or renamed in Step 7), flag the stale reference as a Step 8 follow-up note rather than editing IMPLEMENTATION-ORDER.md — that file is reconciled at archival time, not at reassessment time, since the reassessment's deliverable is the spec file.
    - **Sibling-spec back-reference staleness**: If Step 7 renamed or reshaped a symbol that active draft sibling specs forward-reference (found via the 3.8 grep of `specs/`), flag each stale sibling citation as a Step 8 follow-up note — do not edit the sibling specs; they reconcile on their own reassessment pass. This is distinct from the IMPLEMENTATION-ORDER.md staleness check above (which covers the roadmap blurb) and from 3.8's archived-sibling check (which refreshes *this* spec's references) — here an active draft sibling cites a symbol *this* reassessment changed.
    - **Speculative cross-spec reference handling**: If reassessment surfaces a soft reference to a type/symbol introduced by an unimplemented sibling spec, and the reference is in a speculative future-extension surface (not load-bearing for the headline scenario), prefer YAGNI/FND-28 removal of the speculative surface to creating a hard dependency. The latter would route this spec onto the Deferred path, requires re-sequencing in `specs/IMPLEMENTATION-ORDER.md` at the prerequisite's archival pass, and demotes the spec's wave. Surface the speculative-reference removal in the Step 8 summary so the user sees the dependency was eliminated, not deferred. This sub-path stays inside Default — `/spec-to-tickets <spec-path>` is still the immediate next step because no hard prerequisite remains.
    - **Scope-narrowing deferral sub-path**: If reassessment narrows the spec's scope by moving an in-progress deliverable cluster to a *future sibling spec* (because that cluster requires substrate the spec author underestimated — a new event tag, a new belief surface, a new access-right model, a new payload type), stay on Default. The narrowing target stays implementable from the surviving deliverables alone; no hard prerequisite exists yet because the deferred cluster has no draft spec. Required surface adjustments: (1) add a Non-Goals row in the surviving spec documenting the deferral and naming the future-sibling relationship (without committing to a spec ID — none exists yet); (2) note in the Step 8 summary that "a sibling spec is needed to cover <deferred cluster>" as a roadmap follow-up so the deferred scope is tracked even though no Dependencies entry points at it; (3) flag the deferred cluster's reasoning briefly so a future reader knows whether the deferral was substrate-driven, scope-driven, or risk-driven. This sub-path is distinct from the speculative-reference handling above (which deletes the surface entirely because it was speculative) and from the Deferred path below (which gates the *current* spec on an unimplemented prerequisite). `/spec-to-tickets <spec-path>` remains the immediate next step because the narrowed spec is ready to ticket on its own; the sibling spec is independent future work.
  - **Deferred path** (classifications (a)–(e), prerequisite-gated): selected when (a) reassessment promoted at least one dependency from soft to hard AND (b) the newly-hard prerequisite is unimplemented (not yet archived). Common in adjunct spec clusters where dependency designations were drafted as "all soft" but reassessment surfaced a true hard dep. Required actions:
    1. Name the prerequisite explicitly in the spec's Dependencies section (already done if the soft→hard promotion was applied in Step 7).
    2. Add a Status preamble note recording the deferral, e.g., `Status: Draft. **Implementation deferred** until <prereq-id> lands.` Place this in the Phase and Status section so future readers see the gating constraint before the deliverables.
    3. If the deferral has roadmap implications (the prerequisite lives in a later wave/phase than this spec), note in the same preamble that `specs/IMPLEMENTATION-ORDER.md` should be updated as part of the prerequisite's archival pass — do **not** edit IMPLEMENTATION-ORDER.md as part of this reassessment, since the deliverable is the spec file.
    4. Suggested next step text: "Wait for prerequisite `<spec-id>` to land. Once that ships, re-run `/reassess-spec <this-spec>` to confirm assumptions still hold, then `/spec-to-tickets <this-spec>`." `/spec-to-tickets` is **not** the immediate next step — running it now would generate tickets that cannot be implemented.
  - **Retroactive path** (classification (f)): `/spec-to-tickets` is **not** applicable. Instead, complete the archival flow:
    1. Archive the spec per `docs/archival-workflow.md` — move it from `specs/` to `archive/specs/`.
    2. **Reconcile `specs/IMPLEMENTATION-ORDER.md`**: find the spec's roadmap entry, verify it doesn't already say "✅ COMPLETED", and rewrite it using the canonical format used elsewhere in that file: `- **<ID>**: ✅ COMPLETED — archived at [archive/specs/<file>.md](...). <1–2 line summary of landed artifacts>.` Include delivering-ticket IDs and note any fallout absorbed by sibling specs.
    3. **Grep `specs/`, `archive/specs/`, `tickets/`, and `archive/tickets/`** for paths of the form `specs/<ID>-…` and rewrite them to `archive/specs/<ID>-…`. Include archive directories explicitly — prior archived specs and tickets often forward-reference the just-archived spec.

Do NOT commit. Leave the file for user review.

## Guardrails

- **FOUNDATIONS alignment is mandatory**: Never approve a spec change that violates a Foundation principle, even if requested — flag the conflict instead.
- **Codebase truth**: All references in the updated spec must be validated. Never propagate stale paths, renamed types, or removed functions.
- **No scope creep**: The deliverable is the updated spec file. Do not write design docs, create tickets, or start implementation.
- **No approach proposals**: Validate and refine the existing design, not greenfield alternatives. Exception: when the approach violates a crate boundary, FOUNDATIONS principle, or critical invariant, propose minimum viable alternatives as part of the Issue finding.
- **Substantial redesign flag**: If reassessment changes >50% of deliverables' approach, flag in Step 6: "This reassessment proposes substantial redesign of N/M deliverables. Goals preserved but implementation path changes significantly."
