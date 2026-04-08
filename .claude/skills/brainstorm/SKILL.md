---
name: brainstorm
description: "Confidence-driven brainstorming skill. Interviews the user until 95% confidence about what they actually want, proposes approaches with tradeoffs, produces an approved design doc. Checks FOUNDATIONS.md alignment for implementation topics. Replaces the global superpowers:brainstorming for this repo."
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

## Step 1: Read Context

1. **Reference file**: If `reference_path` is provided, read the entire file. Extract key claims, proposals, and open questions from it. Summarize what it contains in 2-3 sentences before proceeding.

2. **Topic classification**: Determine whether this brainstorm is **implementation-related** (code changes, architecture, engine modifications, new features, bug fixes) or **non-implementation** (process, tooling config, workflow, strategy, skill design).

3. **If implementation-related**: Read `docs/FOUNDATIONS.md`. You will need it in Steps 3 and 4 to validate proposed approaches against architectural principles.

4. **Project context**: Briefly check relevant project state (recent files, existing specs/tickets in the area) only if the topic clearly relates to a specific part of the codebase. Do not do a broad exploration — keep it targeted.

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
2. **Prefer multiple-choice questions** when the answer space is bounded. Open-ended is fine when it isn't.
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

| Range | Meaning | Action |
|-------|---------|--------|
| 0-30% | Don't understand the problem yet | Ask about the problem, not the solution |
| 30-60% | Understand the problem, unclear on constraints | Ask about constraints, success criteria, scope |
| 60-80% | Understand problem + constraints, unclear on priorities | Ask about tradeoffs, what matters most |
| 80-95% | Clear picture, a few edge cases or preferences unknown | Ask targeted questions about specific gaps |
| 95%+ | Ready to propose | Transition to Step 3 |

### Early Exit

If the user says something like "just go" or "that's enough questions", respect it. Announce your current confidence, list remaining gaps as assumptions you'll make, and proceed to Step 3. Mark those assumptions explicitly in the design so the user can correct them.

## Step 3: Propose Approaches

Present **2-3 distinct approaches** with:

- **Name**: A short descriptive label
- **How it works**: 2-4 sentences
- **Tradeoffs**: What you gain, what you give up
- **Recommendation**: Lead with your recommended option and explain why

**If implementation-related**: For each approach, note which FOUNDATIONS.md principles it aligns with or tensions it creates. Use format: `Foundations: F1 (aligns), F8 (tensions — [reason])`.

**Wait for user to choose or ask questions.** Do not proceed until the user picks an approach (or asks you to refine/combine).

## Step 4: Present Design

Once an approach is chosen, present the design **section by section**. Scale each section to its complexity — a sentence for trivial parts, up to 200 words for nuanced parts.

Sections to cover (skip irrelevant ones):

1. **Overview**: What this design achieves in 1-2 sentences
2. **Architecture / Structure**: How the pieces fit together
3. **Key decisions**: Important choices and why
4. **Data flow / Process**: How information moves through the system
5. **Edge cases**: Known tricky scenarios and how they're handled
6. **Testing strategy**: How to verify this works (if implementation-related)
7. **FOUNDATIONS.md alignment**: Table of relevant principles and how the design respects them (if implementation-related)

**After each section**, ask: "Does this section look right?" Wait for confirmation before presenting the next section. If the user pushes back, revise that section before continuing.

## Step 5: Write Design Doc

Once all sections are approved, write the complete design to:

```
docs/plans/YYYY-MM-DD-<topic>-design.md
```

Where `<topic>` is a kebab-case short name derived from the brainstorm topic.

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

## Guardrails

- **YAGNI ruthlessly**: Remove unnecessary features from all designs. If a proposed approach has optional extras, strip them unless the user explicitly asked for them.
- **One question at a time**: Never batch questions. This is non-negotiable.
- **No implementation before approval**: The hard gate at the top means exactly what it says.
- **FOUNDATIONS.md is authoritative**: For implementation topics, if a proposed approach violates a Foundation principle, flag it immediately. Do not propose approaches that violate Foundations without explicitly calling out the violation and getting user sign-off.
- **Worktree discipline**: If working in a worktree, all file paths use the worktree root.
- **No scope inflation**: The design covers what was asked for. Resist the urge to add "while we're at it" improvements.
- **Respect early exit**: If the user wants to skip ahead, let them. List your assumptions clearly.
