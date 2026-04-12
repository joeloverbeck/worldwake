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

1. **Reference file**: If `reference_path` is provided, read the entire file. Extract key claims, proposals, and open questions from it. Summarize what it contains in 2-3 sentences before proceeding. If the user references files inline in their request text (rather than via the `reference_path` argument), treat those files as reference material with the same read-and-summarize treatment. Multiple inline references are common; read all of them.

2. **Topic classification**: Determine whether this brainstorm is **implementation-related** (code changes, architecture, engine modifications, new features, bug fixes) or **non-implementation** (process, tooling config, workflow, strategy, skill design).

3. **If implementation-related OR if the topic directly concerns FOUNDATIONS.md principles**: Read `docs/FOUNDATIONS.md`. You will need it in Steps 3 and 4 to validate proposed approaches against architectural principles.

4. **Confidence calibration from reference material and request**: If the reference file provides a comprehensive design (rationale, decisions, structure, adaptation notes), set initial confidence based on how much of the problem space it covers. A thorough reference file may start confidence at 70-85%, reducing the interview to closing operational gaps (naming, cleanup, customization preferences). The same calibration applies to the user's initial request text — if the request includes detailed problem analysis, specific evidence, root cause identification, and a clear ask, calibrate initial confidence from the request itself, not just from reference files. If the user's request includes root cause analysis, proposed solution, code locations, and FOUNDATIONS justification, set initial confidence to 85-95%. The interview becomes a gap-closing exercise (1-2 targeted questions about scope or edge cases), not a discovery process. Do not ask motivational questions ("what problem does this solve?") when the user has already demonstrated deep understanding of the problem. Research findings (sub-step 6 below) also contribute to confidence by narrowing the solution space before the interview begins.

5. **Interview skip threshold**: If Step 1 exploration and research bring confidence to 95%+ before the interview starts, skip Step 2 entirely and proceed directly to Step 3. Announce the confidence level and note that exploration resolved all gaps. This is common for well-specified requests where the user provides root cause analysis, code locations, and evidence — codebase exploration confirms rather than discovers.

6. **External research**: If the topic requires domain knowledge beyond the codebase (academic algorithms, industry best practices, competing architectures, scaling solutions), launch research agents BEFORE the interview. The user's request may explicitly call for research ("research this online", "look for solutions") or the problem may implicitly require it (novel algorithms, scaling problems, unfamiliar domains). Summarize findings for the user before asking interview questions. Research findings inform both the confidence calibration (what solution space exists) and the approach proposal (what concrete options are available). If codebase exploration (sub-step 7) produces a clear root cause with concrete code-path evidence, external research may be skipped even if the user suggested it. Note the skip decision when presenting findings.

7. **Project context**: Explore existing implementations relevant to the topic before starting the interview — this context informs better questions. For tooling/process topics, examine existing instances of the thing being designed (e.g., existing skills, configs, workflows — their structure, size, patterns). For codebase topics, check relevant files, specs, and tickets. Launch Explore agents for broad surveys when needed. Keep exploration targeted to what informs the interview.

## Step 2: Confidence-Driven Interview

This is the core of the skill. Your goal is to reach **95% confidence** about what the user actually wants before proposing solutions.

### The Protocol

After each user answer, display a confidence block:

```
Confidence: X%
Gaps: [list of remaining unknowns]
```

Keep asking questions until confidence reaches 95%. Then announce: "I'm at 95% confidence. Moving to approaches."

### Interview Rules

1. **One question per message.** Never ask multiple questions at once.
2. **Prefer multiple-choice questions** when the answer space is bounded. Open-ended is fine when it isn't. Use `AskUserQuestion` with labeled options for multiple-choice interview questions, approach selection, and section approval prompts. In plan mode, inline numbered options are acceptable since the conversation flow is faster.
3. **Probe motivations before solutions.** Ask "What problem does this solve?" and "What happens if we don't do this?" before "What do you want built?" The user's first request often describes a solution, not the problem. Your job is to find the problem.
4. **Challenge premature specificity.** If the user jumps to implementation details early, ask why that specific approach matters. Often the constraint is softer than stated.
5. **Detect "should want" vs "actually want".** Watch for:
   - Buzzword-heavy descriptions (the user may be echoing best practices they read, not their real need)
   - Over-scoped requests (wanting everything when they need one thing)
   - Vague success criteria ("it should be good" — probe for what "good" means concretely)
   - Solutions stated as requirements ("I need a microservice" — do they need a microservice, or do they need X capability?)
6. **Name your uncertainty.** When you display gaps, be specific: "I don't know whether this needs to handle edge case X" is useful. "I need more information" is not.
7. **Respect user expertise.** If the user gives a clear, well-reasoned answer, don't re-ask the same thing in different words. Advance.

### Confidence Scoring Guide

Confidence increases from **both user answers AND research findings**. If external research (Step 1, sub-step 5) narrows the solution space before or during the interview, factor that into the confidence score and note which gaps were closed by research vs. which require user input.

| Range | Meaning | Action |
|-------|---------|--------|
| 0-30% | Don't understand the problem yet | Ask about the problem, not the solution |
| 30-60% | Understand the problem, unclear on constraints | Ask about constraints, success criteria, scope |
| 60-80% | Understand problem + constraints, unclear on priorities | Ask about tradeoffs, what matters most |
| 80-95% | Clear picture, a few edge cases or preferences unknown | Ask targeted questions about specific gaps |
| 95%+ | Ready to propose | Transition to Step 3 |

### Plan Mode Interview

In plan mode, the confidence block is still required at the transition from interview to approach proposal. Display confidence and gaps at least once — when announcing the move to approaches. The transition announcement may be a prose statement (e.g., "Confidence at 95%, no remaining gaps") rather than the formal block format, provided it clearly states the confidence level and that gaps are resolved. Even in plan mode, use a visually distinct transition marker (bold heading, horizontal rule, or the standard phrase "I'm at 95% confidence. Moving to approaches.") when the confidence announcement is embedded in a longer analytical message. The reader should be able to scan and find it. Intermediate per-answer confidence blocks may be omitted if the interview is 1-2 questions. Pre-question confidence statements (e.g., showing initial calibration from Step 1) are optional context-setting and need not follow the formal block format — the required display is at the transition to approaches.

When initial confidence from Step 1 is >= 85% (detailed user request with evidence, root cause analysis, and clear deliverable), the expected plan-mode interview is 0-2 targeted questions. The confidence announcement, approach rationale, and design presentation may all appear in a single message sequence if no course correction is needed.

**Fast-track plan-mode flow**: When all three conditions are met — plan mode active, initial confidence >= 85%, single viable approach — Steps 2-4 may collapse into a single message sequence: confidence announcement, approach rationale, key design decisions, and plan file write. This is the expected flow for well-specified diagnostic-to-spec brainstorms where the user provides root cause analysis, evidence, and a clear deliverable type.

### Early Exit

If the user says something like "just go" or "that's enough questions", respect it. Announce your current confidence, list remaining gaps as assumptions you'll make, and proceed to Step 3. Mark those assumptions explicitly in the design so the user can correct them.

## Step 3: Propose Approaches

Present **2-3 distinct approaches** with:

- **Name**: A short descriptive label
- **How it works**: 2-4 sentences
- **Tradeoffs**: What you gain, what you give up
- **Recommendation**: Lead with your recommended option and explain why

**If implementation-related**: For each approach, note which FOUNDATIONS.md principles it aligns with or tensions it creates. Use format: `Foundations: F1 (aligns), F8 (tensions — [reason])`.

**If the problem space is fully constrained** (e.g., a reference document provides a proven design, or requirements eliminate alternatives), state why only one approach exists and present it directly. Do not invent artificial alternatives. In plan mode with a single viable approach, the approach rationale may be embedded in the plan file's Context section rather than presented as a separate conversational step.

**Wait for user to choose or ask questions.** Do not proceed until the user picks an approach (or asks you to refine/combine).

## Step 4: Present Design

**Plan mode**: Skip per-section gates. Present key decisions in 1-2 messages with conversation-level checkpoints, then write to plan file. See plan-mode details at the end of this section.

Once an approach is chosen, present the design **section by section**. Scale each section to its complexity — a sentence for trivial parts, up to 200 words for nuanced parts.

Sections to cover (skip irrelevant ones):

1. **Overview**: What this design achieves in 1-2 sentences
2. **Architecture / Structure**: How the pieces fit together
3. **Key decisions**: Important choices and why
4. **Data flow / Process**: How information moves through the system
5. **Edge cases**: Known tricky scenarios and how they're handled
6. **Testing strategy**: How to verify this works (if implementation-related)
7. **FOUNDATIONS.md alignment**: Table of relevant principles and how the design respects them (if implementation-related)

Section names are suggestions. Rename or combine sections to match the topic's natural structure. The key requirement is per-section approval, not specific section names.

**After each section**, ask: "Does this section look right?" Wait for confirmation before presenting the next section. If the user pushes back, revise that section before continuing.

**If plan mode is active**: Per-section approval is replaced by whole-plan approval via `ExitPlanMode`. Present the key design decisions inline in the conversation before writing the plan file, so the user can course-correct before the plan is finalized. For complex designs, present in 1-2 messages, grouping related sections. Pause after the first message to check for course corrections before continuing. The goal is conversation-level checkpoints, not per-section gates. In plan mode, the confidence-reached announcement and approach proposal may be folded into the same message as the design presentation when the approach is architecturally constrained (single viable option).

## Step 5: Write Design Doc

After design approval, do NOT apply changes or implement the design until the user selects an implementation option from the Step 6 menu. The design doc is the deliverable of this skill — implementation is a separate act that requires the user's explicit choice.

**Deliverable classification**:
- If the brainstorm topic is itself a skill design, the deliverable is the skill file (written to the appropriate skills directory, e.g., `.claude/skills/<name>/SKILL.md`). Skip the `docs/plans/` design doc — the skill file IS the design. Adjust the Step 6 menu to reflect this (omit "create a spec" option, replace with "run skill-audit on the new skill").
- If the brainstorm topic is modifying or reconciling existing skill files, the deliverable is the modified skill file(s) themselves. Skip the `docs/plans/` design doc — the edits ARE the design. If merging multiple skills, the deliverable includes the new unified skill file, deletion of superseded skill directories, and updating any cross-references in other skills or configuration files.
- If the brainstorm produces a deliverable that **replaces** an existing artifact (skill, spec, config), the replacement plan should include: (a) confirming deletion of the old artifact, (b) checking for cross-references to the old artifact in other skills, CLAUDE.md, or MEMORY.md, (c) noting the replacement in the design doc or plan.
- If the brainstorm topic produces a system spec (architectural change, new subsystem, planner modification, or any change requiring formal spec-drafting-rules compliance), the deliverable is the spec file in `specs/`. Skip the `docs/plans/` design doc — the spec IS the design. Read `docs/spec-drafting-rules.md` and ensure the spec includes all mandatory sections. Before writing the spec, verify completeness against this checklist: H.1 (motivating gap), H.2 (entities/relations), H.3 (mutations), H.4 (information/observability), H.5 (conserved quantities), H.6 (contention), H.7 (partial failures), plus the analysis sections: information-path, positive-feedback, stored-state vs. derived read-model list. Adjust the Step 6 menu accordingly (omit "create a spec" option, replace with "reassess spec" or "decompose into tickets"). For spec deliverables, the "Brainstorm Context" header is replaced by the spec's Problem Statement section, which should include the motivation, evidence sources, and key interview decisions that shaped the spec design.
- If the brainstorm produces **hybrid deliverables** (e.g., both implementation code AND a spec), the plan file describes the full implementation sequence — code changes, spec writing, and any other artifacts. The spec is still written after plan approval, but the plan may describe implementation steps for non-spec deliverables at normal detail. Keep the plan file under 100 lines when the spec is the primary deliverable; non-spec deliverables (tooling enhancements, data-gathering steps) may extend this slightly.
- If the brainstorm reveals that the deliverable **requires data that doesn't yet exist** (e.g., new instrumentation, enhanced diagnostics, or data-gathering tooling), the plan should include a pre-deliverable data-gathering phase. In plan mode, the plan file describes both the tooling enhancement and the final deliverable. The tooling work is executed after plan approval but before the spec/design doc is written, since the spec content depends on the gathered data.

Once all sections are approved, write the complete design:

- **If plan mode is active**: Write the design to the plan file (the only writable file in plan mode). The plan file serves as the design doc. When plan mode is active AND the deliverable is a spec: the spec cannot be written to `specs/` until after `ExitPlanMode` is called and the plan is approved. Write the plan file first with the spec design (deliverables, FOUNDATIONS alignment, verification). After plan approval, write the spec to `specs/` as the first implementation step. The plan file references the spec and summarizes the implementation sequence — it is not the design itself. Keep the plan file under 100 lines when the spec is the primary deliverable; the plan should summarize intent, list deliverables, and describe the implementation sequence without duplicating the full spec content.
- **Otherwise**: Write to `docs/plans/YYYY-MM-DD-<topic>-design.md`, where `<topic>` is a kebab-case short name derived from the brainstorm topic.

The design doc should consolidate all approved sections into a clean document. Include a "Brainstorm Context" header at the top noting:
- The original request
- Reference file (if any)
- Key interview insights that shaped the design
- Final confidence score and any assumptions made

Do NOT commit the file. Leave it for user review.

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

Use AskUserQuestion to present this as a proper choice. If the user picks an option that invokes another skill, invoke it. If they pick "done", end the session.

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
