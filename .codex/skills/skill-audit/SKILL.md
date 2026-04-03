---
name: skill-audit
description: "Audit a skill file against the current Codex session's actual work. Use when asked to review a skill under `.codex/skills/` or `.claude/skills/` for quality, missing guidance, unclear steps, or repo-rule alignment without editing it."
---

# Skill Audit

Analyze a skill against the work done in the current Codex session and report where the skill is unclear, incomplete, inconsistent, or misaligned with repo rules. Report only. Do not modify the target skill unless the user explicitly asks for follow-up edits after the audit.

Read [AGENTS.md](../../../AGENTS.md) and [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md) before producing findings if they were not already read earlier in this session.

## Workflow

### 1. Load the target skill

1. Read the target skill's `SKILL.md`.
2. Parse its `name`, `description`, and workflow/guardrail sections.
3. If the provided path is not a skill directory or `SKILL.md` is missing, stop and report the error.

### 2. Load alignment context

1. Read [AGENTS.md](../../../AGENTS.md) if it was not already read earlier in this session.
2. Read [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md) if it was not already read earlier in this session.
3. If the target is a Codex skill under `.codex/skills/`, keep Codex skill structure expectations in mind:
   - concise `SKILL.md`
   - trigger description that clearly states when to use the skill
   - references or scripts only when they reduce repeated agent work

### 3. Reflect on session evidence

Review the current session and extract only evidence-backed observations:
- places where the skill's instructions were unclear, ambiguous, or missing
- steps that had to be skipped, reordered, or improvised
- edge cases or inputs the skill did not anticipate
- outcomes that diverged from what the skill appeared to intend
- steps that were not exercised this session

If auditing `skill-audit` itself, treat the current invocation as weak evidence. Do not invent findings that were not observed.
For self-audits, default to `No issues identified` unless a gap was directly observed in a previous non-self invocation during the same session.

### 4. Cross-check alignment

For each observed gap, check whether the skill:
- contradicts or omits repo rules from [AGENTS.md](../../../AGENTS.md)
- suggests behavior that would violate a principle in [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md)
- uses stale Claude-specific assumptions when the target is meant for Codex
- would create repo-specific operational hazards such as dirtying tracked worktree paths, colliding with special directories, or leaving cleanup obligations implicit

Reference concrete rule sources in findings:
- `AGENTS.md` section names
- FOUNDATIONS principle numbers
- relevant repo paths or tracked-state evidence when the hazard is operational rather than purely textual

### 5. Classify findings

Place each finding in exactly one bucket:
- **Issue**: broken, misleading, contradictory, or likely to cause incorrect behavior
- **Improvement**: existing guidance works but should be refined
- **Feature**: missing capability that fits the skill's stated scope

Also tag severity:
- `CRITICAL`
- `HIGH`
- `MEDIUM`
- `LOW`

### 6. Present the audit

Return the report in the conversation using this structure:

```markdown
# Skill Audit: <skill-name>

**Skill path**: <path>
**Session date**: YYYY-MM-DD
**Session summary**: <1-2 sentences on how the skill was used or evaluated>

## Alignment Check

- **AGENTS.md**: <aligned / N deviations found>
- **FOUNDATIONS.md**: <aligned / N violations found>
- **Codex fit**: <aligned / N Codex-specific mismatches found>

## Issues

[If none: "No issues identified."]

1. **[SEVERITY]** <title>
   - **What happened**: <specific session evidence>
   - **Skill gap**: <what the skill says or fails to say>
   - **Why it matters**: <behavioral or alignment impact>
   - **Suggestion**: <specific skill change>

## Improvements

[If none: "No improvements identified."]

1. **[SEVERITY]** <title>
   - **Current behavior**: <what the skill currently says>
   - **Why improve**: <session evidence or tight reasoning>
   - **Suggestion**: <specific refinement>

## Features

[If none: "No features identified."]

1. **[SEVERITY]** <title>
   - **What's missing**: <missing guidance or capability>
   - **Why it fits**: <why this stays inside the skill's stated scope>
   - **Suggestion**: <specific addition>

## Not Exercised

- <step or area not exercised this session>

## Summary

**Total**: N issues, N improvements, N features — N CRITICAL, N HIGH, N MEDIUM, N LOW
```

## Guardrails

- Report only during the audit phase. Do not edit the target skill unless the user explicitly asks for implementation after seeing the report.
- Do not speculate. If a step was not exercised this session, place it under `Not Exercised` instead of turning it into a finding.
- Every Issue and Improvement must be backed by concrete session evidence.
- Reject any proposed suggestion that would violate [docs/FOUNDATIONS.md](../../../docs/FOUNDATIONS.md).
- Keep scope discipline. Audit the skill as written; do not expand it into a different tool.
- If the skill participates in a multi-skill workflow, check sibling skills for conflicting terminology, paths, or handoff assumptions and report concrete inconsistencies.
- When follow-up edits are requested, apply them in document order and then re-read the changed sections to confirm the workflow still reads coherently.
