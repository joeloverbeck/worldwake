---
name: brainstorm
description: "Use when starting a new feature, design, or architectural decision that needs requirements discovery before implementation. Triggers: vague requests, exploration keywords, uncertainty about what to build, need for external research before designing."
user-invocable: true
arguments:
  - name: request
    description: "The brainstorming topic or question (string). Can be a simple sentence or a detailed description."
    required: true
  - name: reference_path
    description: "Optional path to a reference file (report, brainstorming doc, analysis) to read as context before starting the interview."
    required: false
---

# Brainstorm

Confidence-driven collaborative brainstorming. Interviews you until it understands what you **actually want** — not what you think you should want — then proposes approaches, builds a design, and lets you choose what happens next.

<HARD-GATE>
Do NOT write any code, scaffold any project, invoke any implementation skill, or take any implementation action until you have presented a design and the user has explicitly approved it. This applies to EVERY topic regardless of perceived simplicity.
</HARD-GATE>

## Process Flow

```
Read context (reference file + detect topic type)
         |
         v
Announce classification to user
         |
         v
Confidence-driven interview loop (target: 95%)
         |
         v
Propose 2-3 approaches with tradeoffs
         |
         v
Present design section by section, get approval per section
         |
         v
[If implementation topic] Validate against FOUNDATIONS.md
         |
         v
Write design doc to docs/plans/
         |
         v
Next-steps menu (user chooses)
```

**In plan mode**: design doc writes to the plan file instead of `docs/plans/`, and `ExitPlanMode` replaces the next-steps menu.

## Step 1: Read Context

1. **Reference file**: If `reference_path` is provided, read the entire file. Extract key claims, proposals, and open questions from it. Summarize what it contains in 2-3 sentences before proceeding. If the user references files inline in their request text (rather than via the `reference_path` argument), treat those files as reference material with the same read-and-summarize treatment. Multiple inline references are common; read all of them. If inline references include sibling skills (skills in the same pipeline or workflow), note their input/output interfaces (consumed/produced files, shared terminology, intermediate artifacts) and ensure the new design is compatible. When sub-step 7 surfaces sibling skills not named in the request, apply the same compatibility check to those siblings — verify that intermediate artifact paths, output paths, and shared dump or report formats do not collide. For diagnostic/analysis reference files, treat claims about codebase state (e.g., "agent lacks profile X", "component Y is opt-in") as hypotheses to verify during codebase exploration (sub-step 7), not as established facts. Flag any claims that contradict what exploration reveals — these corrections should be prominently communicated to the user before proceeding to triage or approach proposal.

2. **Topic classification**: Determine which category this brainstorm falls into:
   - **implementation-related**: Changes that alter simulation behavior — engine modifications, new systems, planner changes, action handlers, component logic, new features, bug fixes in simulation code.
   - **implementation-adjacent**: Code changes that do not affect simulation behavior — developer tooling, documentation generators, build scripts, test infrastructure, doc comment enrichment, CLI cosmetics.
   - **non-implementation**: No code changes — process, workflow, strategy, skill design, tooling configuration.

   If the brainstorm is primarily one category but produces a secondary-category artifact (e.g., non-implementation doc + implementation-adjacent generator binary, or implementation-related spec + non-implementation workflow doc), state the primary classification AND name the secondary component explicitly (e.g., "Topic classified as non-implementation (primary: roadmap doc) with an implementation-adjacent secondary component (generator binary + generated file)"). Downstream sections (testing strategy, FOUNDATIONS alignment, deliverable classification in Step 5) apply per-component, not per-brainstorm — e.g., the primary-doc component skips FOUNDATIONS alignment while the secondary-binary component treats it as optional per the implementation-adjacent rule.

   **Announce before proceeding.** After classifying, state the classification to the user in a single short sentence (or, for hybrids, one sentence naming primary + secondary) **BEFORE launching research, reading FOUNDATIONS.md, or starting the interview.** This gives the user an early redirect opportunity before downstream decisions lock in. Do not skip this step even under auto mode or when initial confidence is high.

3. **If implementation-related OR if the topic directly concerns FOUNDATIONS.md principles**: Read `docs/FOUNDATIONS.md`. You will need it in Steps 3 and 4 to validate proposed approaches against architectural principles. For implementation-adjacent topics, reading FOUNDATIONS.md is optional — only read it if the topic touches engine architecture or component registration.

4. **Confidence calibration from reference material and request**: If the reference file provides a comprehensive design (rationale, decisions, structure, adaptation notes), set initial confidence based on how much of the problem space it covers. A thorough reference file may start confidence at 70-85%, reducing the interview to closing operational gaps (naming, cleanup, customization preferences). The same calibration applies to the user's initial request text — if the request includes detailed problem analysis, specific evidence, root cause identification, and a clear ask, calibrate initial confidence from the request itself, not just from reference files. If the user's request includes root cause analysis, proposed solution, code locations, and FOUNDATIONS justification, set initial confidence to 85-95%. The interview becomes a gap-closing exercise (1-2 targeted questions about scope or edge cases), not a discovery process. Do not ask motivational questions ("what problem does this solve?") when the user has already demonstrated deep understanding of the problem. Research findings (sub-step 6 below) also contribute to confidence by narrowing the solution space before the interview begins. When the brainstorm is invoked mid-conversation after prior investigation (observer runs, code analysis, debugging), calibrate initial confidence from the accumulated conversation context — not just from reference files or the request text. Prior evidence gathering in the same session is a first-class confidence source.

5. **Interview skip threshold**: If Step 1 exploration and research bring confidence to 95%+ before the interview starts, skip Step 2 entirely and proceed directly to Step 3. Announce the confidence level and note that exploration resolved all gaps. This is common for well-specified requests where the user provides root cause analysis, code locations, and evidence — codebase exploration confirms rather than discovers.

   **Directed design question carve-out (non-plan-mode)**: If initial confidence is 85-94% AND the user's request is framed as a specific design decision with enumerated alternatives (i.e., the user has already stated the problem and proposed options, turning the brainstorm into an approach-selection task rather than a discovery task), skip the discovery interview and go directly to Step 3. Announce the confidence level, name any remaining gaps as assumptions to carry into the approach proposal, and let the user correct them during approach selection. This mirrors the plan-mode fast-track but applies in non-plan-mode when the user has effectively pre-conducted the interview themselves.

6. **External research**: If the topic requires domain knowledge beyond the codebase (academic algorithms, industry best practices, competing architectures, scaling solutions), launch research agents BEFORE the interview. The user's request may explicitly call for research ("research this online", "look for solutions") or the problem may implicitly require it (novel algorithms, scaling problems, unfamiliar domains). Summarize findings for the user before asking interview questions. Research findings inform both the confidence calibration (what solution space exists) and the approach proposal (what concrete options are available). If codebase exploration (sub-step 7) produces a clear root cause with concrete code-path evidence, external research may be skipped even if the user suggested it. Note the skip decision when presenting findings. If the user's interview answer explicitly requests research or reveals a domain that requires it, pause the interview to launch research agents before continuing. Present research findings before the next interview question or approach proposal. This mid-interview research follows the same summarize-before-asking pattern as pre-interview research.

7. **Project context**: Explore existing implementations relevant to the topic before starting the interview — this context informs better questions. For tooling/process topics, examine existing instances of the thing being designed (e.g., existing skills, configs, workflows — their structure, size, patterns). For skills, also check the on-disk layout of sibling skills — conventional subdirectories like `references/` and `scripts/` indicate the expected multi-file pattern. For codebase topics, check relevant files, specs, and tickets. Launch Explore agents for broad surveys when needed. Keep exploration targeted to what informs the interview. For brainstorms triggered by frustration or repeated failure, also explore the history of attempted fixes — not just the current state. Patterns of repeated remediation specs, escalating complexity, or accumulated workarounds are signals that the approaches (Step 3) should include radical options (reset, strip, rebuild from scratch) alongside incremental fixes. A long chain of narrowly-scoped fixes that didn't resolve the underlying problem is evidence that the problem may be structural, not tactical.

## Step 2: Confidence-Driven Interview

This is the core of the skill. Your goal is to reach **95% confidence** about what the user actually wants before proposing solutions.

### The Protocol

After each user answer, display the current confidence and the remaining gaps. The default form is a fenced block:

```
Confidence: X%
Gaps: [list of remaining unknowns]
```

A single-line prose form (e.g., `Confidence: 90% — remaining gaps: X, Y.`) is equally acceptable in both plan and non-plan mode, provided the percentage and remaining gaps are both named. Use the fenced block when the interview is ≥ 3 questions deep or when a visual scan for the transition moment would help the reader; use prose when the interview is 1-2 questions deep or when the confidence note is embedded in a larger analytical message.

Keep asking questions until confidence reaches 95%. Then announce: "I'm at 95% confidence. Moving to approaches."

### Interview Rules

1. **One question per message.** Never ask multiple questions at once.
2. **Prefer multiple-choice questions** when the answer space is bounded. Open-ended is fine when it isn't. Use `AskUserQuestion` with labeled options for multiple-choice interview questions, approach selection, and section approval prompts when its schema is already available in the session. Inline numbered options are an acceptable fallback when AskUserQuestion is deferred and fetching its schema would add friction. In plan mode, inline numbered options are preferred since the conversation flow is faster.
3. **Probe motivations before solutions.** Ask "What problem does this solve?" and "What happens if we don't do this?" before "What do you want built?" The user's first request often describes a solution, not the problem. Your job is to find the problem.
4. **Challenge premature specificity.** If the user jumps to implementation details early, ask why that specific approach matters. Often the constraint is softer than stated.
5. **Detect "should want" vs "actually want".** Watch for:
   - Buzzword-heavy descriptions (the user may be echoing best practices they read, not their real need)
   - Over-scoped requests (wanting everything when they need one thing)
   - Vague success criteria ("it should be good" — probe for what "good" means concretely)
   - Solutions stated as requirements ("I need a microservice" — do they need a microservice, or do they need X capability?)
6. **Name your uncertainty.** When you display gaps, be specific: "I don't know whether this needs to handle edge case X" is useful. "I need more information" is not.
7. **Respect user expertise.** If the user gives a clear, well-reasoned answer, don't re-ask the same thing in different words. Advance.
8. **Handle delegation gracefully.** If the user says "I'm not sure" or "you decide," treat it as a delegation: re-evaluate the options against FOUNDATIONS.md and project constraints, present your reasoned recommendation, and advance confidence accordingly. Do not re-ask the same question — the user has told you they trust your judgment on this point.
9. **Present empirical findings before asking questions.** When codebase exploration or simulation runs produce concrete findings (data tables, root cause evidence, confirmed/refuted hypotheses), present a concise evidence summary before the first interview question. Structure: hypothesis, evidence (table or list), verdict per hypothesis, then the question. This lets the user evaluate your analysis before being asked to decide on scope or approach.

### Confidence Scoring Guide

Confidence increases from **both user answers AND research findings**. If external research (Step 1, sub-step 6) narrows the solution space before or during the interview, factor that into the confidence score and note which gaps were closed by research vs. which require user input.

| Range | Meaning | Action |
|-------|---------|--------|
| 0-30% | Don't understand the problem yet | Ask about the problem, not the solution |
| 30-60% | Understand the problem, unclear on constraints | Ask about constraints, success criteria, scope |
| 60-80% | Understand problem + constraints, unclear on priorities | Ask about tradeoffs, what matters most |
| 80-95% | Clear picture, a few edge cases or preferences unknown | Ask targeted questions about specific gaps |
| 95%+ | Ready to propose | Transition to Step 3 |

### Plan Mode Interview

In plan mode, the confidence block is still required at the transition from interview to approach proposal. Display confidence and gaps at least once — when announcing the move to approaches. The transition announcement may be a prose statement (e.g., "Confidence at 95%, no remaining gaps") rather than the formal block format, provided it clearly states the confidence level and that gaps are resolved. Even in plan mode, use a visually distinct transition marker (bold heading, horizontal rule, or the standard phrase "I'm at 95% confidence. Moving to approaches.") when the confidence announcement is embedded in a longer analytical message. The reader should be able to scan and find it. The "visually distinct marker" requirement applies specifically to the interview-to-approaches transition (the moment confidence reaches 95%), not to initial confidence calibration displays from Step 1 — those are optional context-setting. Intermediate per-answer confidence blocks may be omitted if the interview is 1-2 questions. When the interview question IS the approach-selection question (the user's answer simultaneously closes the last gap and selects an approach), the transition announcement may be omitted — the approach selection implicitly demonstrates 95%+ confidence.

When initial confidence from Step 1 is >= 85% (detailed user request with evidence, root cause analysis, and clear deliverable), the expected plan-mode interview is 0-2 targeted questions. The confidence announcement, approach rationale, and design presentation may all appear in a single message sequence if no course correction is needed.

**Fast-track plan-mode flow**: When all three conditions are met — plan mode active, initial confidence >= 85%, single viable approach — Steps 2-4 may collapse into a single message sequence: confidence announcement, approach rationale, key design decisions, and plan file write. The Step 3 "wait for user to choose" gate is also collapsed — when there's a single viable approach, presenting it and proceeding to the plan file write is one flow. The user's approval comes via ExitPlanMode, not a separate approach-selection step. This is the expected flow for well-specified diagnostic-to-spec brainstorms where the user provides root cause analysis, evidence, and a clear deliverable type.

**Fast-track assumption disclosure**: When initial confidence is 85-94% (high but not complete), list remaining gaps as **named assumptions** in the design presentation — not just in the confidence block. Format: "Assumptions (unresolved gaps): (1) X — assuming Y, (2) ..." The user can correct them before ExitPlanMode. At 95%+ no assumption disclosure is needed since all gaps are resolved.

### Early Exit

If the user says something like "just go" or "that's enough questions", respect it. Announce your current confidence, list remaining gaps as assumptions you'll make, and proceed to Step 3. Mark those assumptions explicitly in the design so the user can correct them.

### Recovery/Reset Brainstorms

When the brainstorm is triggered by frustration indicators ("huge mistake", "wrong approach", "start over", "strip everything", "remove all"), adjust the interview:

1. **Validate the diagnosis before accepting it.** The user may be catastrophizing after a bad run, or missing that the system is more salvageable than they think. Present what exploration revealed — both what's broken AND what's working — before agreeing with a scorched-earth instinct.
2. **Focus confidence on "what's actually broken?"** before "what's the fix?" The confidence target shifts from requirements discovery to failure-mode agreement. Key gaps to close: is the problem structural (architecture) or tactical (configuration, scenario design, missing profiles)? Is the user's frustration proportional to the evidence?
3. **Always include a radical option.** When proposing approaches (Step 3), ensure at least one option represents the user's most aggressive instinct (strip, delete, rebuild). Even if you recommend a less radical path, validating the radical option as a real choice respects the user's judgment and prevents the feeling of being talked out of something.

## Step 3: Propose Approaches

Present **2-3 distinct approaches** with:

- **Name**: A short descriptive label
- **How it works**: 2-4 sentences
- **Tradeoffs**: What you gain, what you give up
- **Recommendation**: Lead with your recommended option and explain why

**If implementation-related** (not implementation-adjacent): For each approach, note which FOUNDATIONS.md principles it aligns with or tensions it creates. Use format: `Foundations: F1 (aligns), F8 (tensions — [reason])`.

**If the problem space is fully constrained** (e.g., a reference document provides a proven design, or requirements eliminate alternatives), state why only one approach exists and present it directly. Do not invent artificial alternatives. In plan mode with a single viable approach, the approach rationale may be embedded in the plan file's Context section rather than presented as a separate conversational step.

**For triage/analysis brainstorms** where the deliverable is a set of work items (tickets, specs) derived from evaluating a report or findings, the "approaches" step may be replaced by presenting the triage recommendation: which items warrant action, which are dismissed, and why. The user's approval of the triage recommendation serves the same gating purpose as choosing an approach.

When triage produces **multiple deliverables** (N specs, N tickets, or a mix), the single triage approval gates all N — do not re-prompt for approval per deliverable. Each deliverable still independently satisfies its own format requirements (spec-drafting-rules.md for specs, `tickets/_TEMPLATE.md` for tickets), but no per-item user approval is required. The Step 6 Next Steps menu is presented once per triage, summarizing all N produced deliverables. For spec deliverables, `specs/IMPLEMENTATION-ORDER.md` gets a single update encompassing all N specs (typically a new Adjunct Wave — see Step 5 spec-deliverable branch).

**If the user challenges an option's dismissal**, do a fresh analysis from first principles rather than defending the prior reasoning. If the new analysis reverses the dismissal, state explicitly where the prior reasoning was incomplete. A revised approach (e.g., phased combination) is a valid output of this re-analysis.

**Wait for user to choose or ask questions.** Do not proceed until the user picks an approach (or asks you to refine/combine).

## Step 4: Present Design

**Plan mode**: Skip per-section gates. Present key decisions in 1-2 messages with conversation-level checkpoints, then write to plan file. See plan-mode details at the end of this section.

**Classification pivot check**: If the design reveals a deliverable type that differs from the Step 1 classification (e.g., "tooling configuration" now requires a new Rust crate, or a spec now includes live code changes) OR reveals a hybrid structure the Step 1 classification didn't name (primary category plus a secondary-category supporting artifact — e.g., non-implementation doc + implementation-adjacent generator binary), re-state the refined classification to the user before presenting Section 1 of the design. Downstream sections (testing strategy, FOUNDATIONS alignment) apply per-component when hybrid, not per-brainstorm.

Once an approach is chosen, present the design **section by section**. Scale each section to its complexity — a sentence for trivial parts, up to 200 words for nuanced parts.

Sections to cover (skip irrelevant ones):

1. **Overview**: What this design achieves in 1-2 sentences
2. **Architecture / Structure**: How the pieces fit together
3. **Key decisions**: Important choices and why
4. **Data flow / Process**: How information moves through the system
5. **Edge cases**: Known tricky scenarios and how they're handled
6. **Testing strategy**: How to verify this works (if implementation-related or implementation-adjacent)
7. **FOUNDATIONS.md alignment**: Table of relevant principles and how the design respects them. Mandatory for implementation-related. Optional for implementation-adjacent — include when the design touches derived views, data persistence, debuggability surfaces, or other concepts mapped to specific FOUNDATIONS principles; a 3–5 row table is usually sufficient. Omit when truly irrelevant (e.g., cosmetic CLI tweaks).

Section names are suggestions. Rename or combine sections to match the topic's natural structure. The key requirement is per-section approval, not specific section names.

**After each section**, ask: "Does this section look right?" Wait for confirmation before presenting the next section. If the user pushes back, revise that section before continuing. If the user approves 3+ sections consecutively without pushback or revisions, batch remaining sections into groups of 2-3, pausing for approval after each group rather than each section. Under auto mode, this threshold may shift to 2 consecutive approvals to reduce round-trips. Exception: keep a remaining section standalone if it is substantially more complex or higher-risk than the prior ones. Announce the batching: "You're clearly aligned — I'll present the remaining sections in groups." When multiple sections are presented in a single message — whether under this batching shortcut, the post-approach-selection single-shot sub-case, or auto-mode aggressive batching — replace the per-section "Does this section look right?" prompt with a single end-of-message prompt covering the batch (e.g., "Does the design above look right? If yes, I'll <next concrete action>."). Per-section prompts retained inside a batched delivery are redundant — the user can only respond to the batch as a whole.

**Fast-track non-plan-mode flow**: When all three conditions are met — non-plan-mode, confidence ≥ 95%, and a single viable approach fully constrained by the reference material or request — Step 4's per-section approval may collapse into a single design message ending with one approval prompt. The user's yes/no gates the whole design, not sections individually.

**Post-approach-selection single-shot**: Once the user has selected an approach from Step 3 (even when multiple approaches were proposed), a single-shot design message is also permitted if both of the following hold: (a) the chosen approach fully constrains the design — no open architectural decisions remain, only concrete edit sites follow from the approach, and (b) the scope is narrow — roughly < 8 concrete edit sites, no novel abstractions, and no cross-system impact beyond the named target artifacts. This sub-case covers situations where the pre-approach fast-track did not apply (e.g., confidence was 85-94%, or 2-3 approaches were on the table) but approach selection itself collapsed the design space. When using this sub-case, announce briefly in the design message that single-shot is being used (e.g., "Presenting as a single batched change since the approach fully determines the edits"). The user's yes/no still gates the whole design. Auto mode does not otherwise relax section approval; the post-approach-selection sub-case must be actively justified when invoked.

**If plan mode is active**: Per-section approval is replaced by whole-plan approval via `ExitPlanMode`. Present the key design decisions inline in the conversation before writing the plan file, so the user can course-correct before the plan is finalized. For complex designs, present in 1-2 messages, grouping related sections. Pause after the first message to check for course corrections before continuing. The goal is conversation-level checkpoints, not per-section gates. In plan mode, the confidence-reached announcement and approach proposal may be folded into the same message as the design presentation when the approach is architecturally constrained (single viable option).

## Step 5: Write Design Doc

After design approval, do NOT apply changes or implement the design until the user selects an implementation option from the Step 6 menu. The design doc is the deliverable of this skill — implementation is a separate act that requires the user's explicit choice.

**Deliverable classification**:
- If the brainstorm topic is itself a skill design, the deliverable is the skill file (written to the appropriate skills directory, e.g., `.claude/skills/<name>/SKILL.md`). Skip the `docs/plans/` design doc — the skill file IS the design. Adjust the Step 6 menu to reflect this (omit "create a spec" option, replace with "run skill-audit on the new skill"). When plan mode is active AND the deliverable is a skill file: the skill file cannot be written until after `ExitPlanMode` is called and the plan is approved. Write the plan file first with the skill design (process overview, key decisions, verification). After plan approval, write the full SKILL.md as the first implementation step. Keep the plan file under 120 lines for skill deliverables.
- If the brainstorm topic is modifying or reconciling existing skill files, the deliverable is the modified skill file(s) themselves. Skip the `docs/plans/` design doc — the edits ARE the design. If merging multiple skills, the deliverable includes the new unified skill file, deletion of superseded skill directories, and updating any cross-references in other skills or configuration files.
- If the brainstorm topic is modifying an existing hand-authored documentation file (planning doc, roadmap, reference doc, or similar — excluding specs, tickets, and skills, which have their own branches above, and excluding files under `docs/generated/` which are produced by tooling rather than hand-authored), the deliverable is the edits to that file. Skip the `docs/plans/` design doc — the edits ARE the design. Adjust the Step 6 menu per the existing hand-authored-doc substitution rule. In plan mode, the plan file describes the intended edits; the target file cannot be edited until after `ExitPlanMode` is approved.
- If the brainstorm produces a deliverable that **replaces** an existing artifact (skill, spec, config), the replacement plan should include: (a) confirming deletion of the old artifact, (b) checking for cross-references to the old artifact in other skills, CLAUDE.md, or MEMORY.md, (c) noting the replacement in the design doc or plan.
- If the brainstorm topic produces a system spec (architectural change, new subsystem, planner modification, or any change requiring formal spec-drafting-rules compliance), the deliverable is the spec file in `specs/`. Skip the `docs/plans/` design doc — the spec IS the design. **Filename prefix**: Before writing, grep `specs/` and `archive/specs/` for the current spec-prefix scheme. If the project uses a single consistent series (e.g., `S###` for everything including tooling/observer specs), continue that sequence. Only introduce a new prefix after user approval in the design presentation. Read `docs/spec-drafting-rules.md` and ensure the spec includes all mandatory Section H analyses: (1) information-path, (2) positive-feedback, (3) concrete dampeners, (4) stored-state vs. derived read-model list. Also verify CLAUDE.md mandatory spec requirements: Permille for all [0,1] or [0,1000] range values, profile-driven parameters (no hardcoded numeric constants), SystemFn Integration section, Component Registration section, cross-system interactions documented via FND-26. **Tooling-boundary specs**: If the spec is a tooling-boundary spec (dev tool, debugger, diagnostic, visualizer — observer pattern per FND-29 introducing no new simulation state, laws, components, actions, systems, or feedback loops), the simulation-system requirements in `docs/spec-drafting-rules.md` do not apply. Instead include an "Explicit Opt-Outs" table listing each rule (Section H causal-hooks, Permille, profile-driven parameters, SystemFn integration, component registration, cross-system interactions via FND-26) marked N/A with one-line reasoning per rule. Adjust the Step 6 menu accordingly — omit the "create a spec" option and use this template:

  ```
  Specs written: <list each spec with its full path>
  IMPLEMENTATION-ORDER.md updated.
  
  What would you like to do next?
  1. Reassess each spec (/reassess-spec specs/<Name>.md)
  2. Decompose each spec into tickets (/spec-to-tickets specs/<Name>.md)
  3. Start implementing a specific spec directly
  4. Done for now — specs are ready for review
  ```
  
  The menu is presented once per triage, not once per spec. After writing the spec(s), update `specs/IMPLEMENTATION-ORDER.md`. For specs derived from a specific report, scenario run, or triage session, create a new `### Adjunct Wave: <topic>` section in the appropriate Phase, with a one-paragraph derivation preamble stating the source report/scenario, its key findings, and which proposals were accepted vs. dismissed. Place the spec list as bullets under the wave heading with dependency status. Use existing "Adjunct Wave: ..." sections in `IMPLEMENTATION-ORDER.md` as the template. Do not scatter triage-derived specs into unrelated waves. **For specs derived from a user feature request (no triage, no report)**: create the appropriate section — a new top-level `## Developer Tooling` for tooling-boundary specs (if one does not already exist), or the relevant Phase for simulation specs — and place the spec bullet under a newly-created `### Adjunct Wave: <topic>` heading. The derivation preamble is replaced by a one-paragraph motivation preamble stating the brainstorm date, the user-stated problem, and the key interview decisions that shaped the spec. For spec deliverables, the "Brainstorm Context" header is replaced by the spec's Problem Statement section, which should include the motivation, evidence sources, and key interview decisions that shaped the spec design.
- If the brainstorm is a **triage that produces ≥ 2 specs or ≥ 3 tickets**, additionally write a short `docs/triage/YYYY-MM-DD-<topic>-triage.md` summarizing: the source report/scenario/finding set, accepted items (with the full path to each written spec or ticket and a one-line rationale), dismissed items (with a one-line reason each), and any follow-up work items the triage identified but did not action. Keep the triage file under 80 lines. Do not duplicate spec or ticket content — reference by path. The triage file makes the brainstorm's decisions durable and shareable without re-running the brainstorm. For triage brainstorms producing a single spec or fewer than 3 tickets, skip this file — the individual deliverables and the `IMPLEMENTATION-ORDER.md` update are sufficient history.
- If the brainstorm produces **hybrid deliverables** (e.g., both implementation code AND a spec), the plan file describes the full implementation sequence — code changes, spec writing, and any other artifacts. The spec is still written after plan approval, but the plan may describe implementation steps for non-spec deliverables at normal detail. Keep the plan file under 100 lines when the spec is the primary deliverable; for plans with N independent deliverables, the line budget scales to approximately 100 + 20*(N-1) lines to accommodate per-deliverable summaries. Investigation findings that change the deliverable set (items added, dropped, or reframed) should be captured in the plan's Context section.
- If the brainstorm reveals that the deliverable **requires data that doesn't yet exist** (e.g., new instrumentation, enhanced diagnostics, or data-gathering tooling), the plan should include a pre-deliverable data-gathering phase. In plan mode, the plan file describes both the tooling enhancement and the final deliverable. The tooling work is executed after plan approval but before the spec/design doc is written, since the spec content depends on the gathered data.
- If the brainstorm produces **enhanced diagnostics or investigative tooling** (observer improvements, new trace capabilities, analysis instrumentation) where the investigation results drive immediate code fixes rather than a separate spec, the deliverable is the plan file describing both the tooling enhancement and the evidence-driven fixes. Write the plan file with: (1) what to instrument/enhance, (2) what to run and analyze, (3) what to fix based on findings. No separate design doc or spec is needed — the plan IS the design. The Step 6 menu is replaced by `ExitPlanMode` in plan mode, or by a summary of what was done if the investigation completed inline. This deliverable type may also **emerge during implementation** of other deliverable types. When a scenario design, spec implementation, or ticket implementation triggers an investigation cycle (repeated test → diagnose → fix), recognize the pattern and reclassify the deliverable as hybrid: original deliverable + investigation-driven fixes. Update the plan file to capture investigation findings alongside the original design.
- If the brainstorm produces **implementation tickets** (bounded fixes to existing code, not requiring full spec-drafting-rules compliance), the deliverable is the ticket file(s) in `tickets/`. Skip the `docs/plans/` design doc — the tickets ARE the design. Read `tickets/README.md` and `tickets/_TEMPLATE.md` to ensure ticket format compliance. Adjust the Step 6 menu accordingly (omit "create a spec" option, replace with "implement ticket" or "reassess ticket").
- If the brainstorm produces a **scenario file with golden tests** (new `.ron` scenario + corresponding `golden_*.rs` test file), the deliverable is the plan file describing the scenario design: topology, agent profiles, resource placement, and test assertions. No separate design doc or spec is needed. The plan should list critical files, verification commands, and any engine assumptions the scenario depends on (e.g., planner budget requirements, exploration profile tuning). Scan an existing `golden_*.rs` file during the plan phase to capture codebase naming/format conventions the doc-regen scripts expect (e.g., `// Scenario N:` comment headers with sequentially chosen numbers that do not collide with existing tests). For deliverables with numeric thresholds (tolerances, capacities, budgets), include a flex clause in the plan stating the target value, the acceptable flex window, and the investigation trigger beyond that window. During implementation, treat empirical adjustments within the flex window as routine — document the landing and structural reason in code/test comments, not the plan. Adjustments that exceed the flex window should pause implementation and surface the finding, since they may signal a behavioral gap the deliverable should not paper over. Adjust the Step 6 menu to include "run observer on the scenario" alongside standard verification.
- If the brainstorm produces a **new dev-tooling crate, workspace member, or external-interface tool** (e.g., MCP server, debugging binary, analysis script), the deliverable is `docs/plans/YYYY-MM-DD-<topic>-design.md`. Adjust the Step 6 menu to include "scaffold the new crate" alongside standard options.

**Deliverable pivot**: If the user redirects the deliverable type mid-brainstorm (e.g., "actually, make this a spec" or "create a spec for this"), reclassify using the rules above and adjust the flow accordingly. In plan mode, rewrite the plan file to match the new deliverable type before calling ExitPlanMode. Do not ask the user to confirm the pivot — they just told you what they want.

Once all sections are approved, write the complete design:

- **If plan mode is active**: Write the design to the plan file (the only writable file in plan mode). The plan file serves as the design doc. When plan mode is active AND the deliverable is a spec: the spec cannot be written to `specs/` until after `ExitPlanMode` is called and the plan is approved. Write the plan file first with the spec design (deliverables, FOUNDATIONS alignment, verification). After plan approval, write the spec to `specs/` as the first implementation step. The plan file references the spec and summarizes the implementation sequence — it is not the design itself. Keep the plan file under 100 lines when the spec is the primary deliverable; the plan should summarize intent, list deliverables, and describe the implementation sequence without duplicating the full spec content. If investigation during implementation changes the deliverable set (items added, dropped, or reframed), update the plan file's deliverables section to reflect the final state before presenting results to the user.
- **Otherwise**: Write to `docs/plans/YYYY-MM-DD-<topic>-design.md`, where `<topic>` is a kebab-case short name derived from the brainstorm topic.

The design doc should consolidate all approved sections into a clean document. Include a "Brainstorm Context" header at the top noting:
- The original request
- Reference file (if any)
- Key interview insights that shaped the design
- Final confidence score and any assumptions made

Do NOT commit the file. Leave it for user review.

**Post-implementation plan update**: If the brainstorm was in plan mode and implementation changed the deliverable set (engine fixes discovered, new tickets created, scope expanded beyond the original plan), update the plan file's deliverable list before the final summary to the user. The plan file should reflect what was actually delivered, not just what was planned. This applies even when the original plan was approved via ExitPlanMode — the plan is a living document during implementation.

## Step 6: Next Steps Menu

Present the user with options for what to do next:

```
Design doc written to docs/plans/YYYY-MM-DD-<topic>-design.md

What would you like to do next?
1. Write an implementation plan (invoke writing-plans skill)
2. Create a spec from this design (write to specs/)
3. Start implementing directly
4. Done for now — I'll review the design doc later
```

Use AskUserQuestion when its schema is already available in the session. Inline numbered options (as shown above) are an acceptable fallback when AskUserQuestion is deferred and fetching its schema would add friction. If the user picks an option that invokes another skill, invoke it. If they pick "done", end the session.

**If the deliverable is a hand-authored documentation file** (e.g., `docs/<name>.md`, and the "implementation" is literally writing that markdown file rather than producing code): substitute option 3 with `Write the actual <target file path>`. If the design also includes a secondary-component generator binary or script (per the hybrid classification in Step 1), add `Scaffold the supporting tooling first, then write the doc` and `Do both together` as additional options. Keep option 4 (Done for now) unchanged. This mirrors the per-deliverable-type menu adjustments already documented for specs and skills.

**If plan mode is active**: Call `ExitPlanMode` instead of presenting the next-steps menu. The user will direct next steps after approving the plan.

**If implementation was completed inline**: If the task was simple enough that implementation was completed during or immediately after design approval, skip the menu and summarize what was done.

## Guardrails

- **YAGNI ruthlessly**: Remove unnecessary features from all designs. If a proposed approach has optional extras, strip them unless the user explicitly asked for them.
- **One question at a time**: Never batch questions. This is non-negotiable.
- **No implementation before approval**: The hard gate at the top means exactly what it says.
- **FOUNDATIONS.md is authoritative**: For implementation topics, if a proposed approach violates a Foundation principle, flag it immediately. Do not propose approaches that violate Foundations without explicitly calling out the violation and getting user sign-off.
- **Worktree discipline**: If working in a worktree, all file paths use the worktree root.
- **No scope inflation**: The design covers what was asked for. Resist the urge to add "while we're at it" improvements.
- **Respect early exit**: If the user wants to skip ahead, let them. List your assumptions clearly.
- **Blocker discovery during implementation**: If implementation reveals an engine limitation or architectural issue that blocks the brainstorm's deliverable, apply CLAUDE.md's 1-3-1 rule: present the blocker, options, and recommendation. If the fix is small enough to do inline (< 50 lines of engine change), fix it and continue. If the fix is a separate architectural concern, create a ticket and either (a) work around the limitation in the current deliverable with a documented constraint, or (b) implement the fix if the user approves the scope expansion. Update the plan file to reflect the expanded deliverable set.
- **Auto mode interaction**: Under auto mode, (a) present the Next Steps menu unless implementation was already completed inline, (b) approach selection and section approval gates still hold — auto mode does not bypass user alignment, (c) the batching threshold in Step 4 may shift from 3 to 2 consecutive approvals to reduce round-trips. The HARD-GATE at the top of the skill is never relaxed by auto mode.
