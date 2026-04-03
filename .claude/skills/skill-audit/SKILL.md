---
name: skill-audit
description: Session-aware skill quality audit. Analyzes a skill file against the current session's work to find issues, improvements, and missing features. Cross-checks against FOUNDATIONS.md and CLAUDE.md. Invoke at end of session with the skill path as argument.
---

# Skill Audit

Analyze a skill file against the work done in the current Claude Code session to determine whether the skill has issues, could be improved, or needs new features. Report only — never modify the target skill.

## Invocation

```
/skill-audit <path-to-skill-directory>
```

Example: `/skill-audit .claude/skills/improve-loop`

The argument is the skill directory path. The framework automatically resolves `SKILL.md` within it.

## Checklist

1. **Read the target skill** — Read the SKILL.md file at the provided path. Parse its name, description, and full content. If the file does not exist or is not a skill file, stop and report the error.
2. **Read alignment documents** — Read `docs/FOUNDATIONS.md` — skip only if explicitly read earlier in this session (not from memory or training knowledge). `CLAUDE.md` is always available via system context injection and does not need explicit reading.
3. **Session reflection** — Review the current conversation context to identify the items below. If the target skill is skill-audit itself (self-audit), use session evidence from prior audit invocations in this session. The self-audit invocation provides no independent session evidence beyond confirming the skill's flow works.
   - Moments where the skill's instructions were unclear or ambiguous
   - Steps that were skipped, reordered, or worked around
   - Behaviors the skill didn't anticipate (edge cases, unexpected inputs)
   - Places where Claude had to improvise because the skill didn't provide guidance
   - Outcomes that diverged from what the skill intended
   - Steps that were not exercised this session (mark as "not exercised" — do not speculate about issues)
4. **Cross-check alignment** — For each finding from step 3, check whether the skill contradicts or fails to implement:
   - Principles from `docs/FOUNDATIONS.md` (reference by foundation number)
   - Conventions from `CLAUDE.md` (reference by section name)
5. **Classify findings** — Categorize each finding into one of three buckets:
   - **Issue**: Something broken, misleading, or contradictory in the skill
   - **Improvement**: A refinement to existing behavior that would make the skill more effective
   - **Feature**: A new capability that aligns with the skill's stated intent but is currently missing
6. **Severity-tag each finding** — CRITICAL / HIGH / MEDIUM / LOW
7. **Present the report** — Output the structured report using the template below.

## Report Template

Output this structure to the conversation (do not write to a file):

```markdown
# Skill Audit: <skill-name>

**Skill path**: <path>
**Session date**: YYYY-MM-DD
**Session summary**: <1-2 sentence description of what work was done with this skill>

## Alignment Check

- **FOUNDATIONS.md**: <aligned / N violations found>
- **CLAUDE.md**: <aligned / N deviations found>
[If violations: bullet list with specific foundation # or CLAUDE.md section + what conflicts]

## Issues

[If none: "No issues identified."]

1. **[SEVERITY]** <title>
   - **What happened**: <session evidence — what went wrong or was confusing>
   - **Skill gap**: <what the skill says or fails to say that caused this>
   - **Suggestion**: <how to fix the skill>

## Improvements

[If none: "No improvements identified."]

1. **[SEVERITY]** <title>
   - **Current behavior**: <what the skill currently says>
   - **Why improve**: <session evidence or reasoning>
   - **Suggestion**: <proposed change>

## Features

[If none: "No features identified."]

1. **[SEVERITY]** <title>
   - **What's missing**: <gap description>
   - **Why it fits**: <how this aligns with the skill's stated intent>
   - **Suggestion**: <proposed addition>

## Summary

**Total**: N issues, N improvements, N features — N CRITICAL, N HIGH, N MEDIUM, N LOW
```

## Guardrails

- **Report only** — Never modify the target skill file. Output the report to the conversation only.
- **No false positives** — If a step in the skill wasn't exercised during the session, note "not exercised this session" rather than speculating about potential issues.
- **FOUNDATIONS alignment is mandatory** — Any suggestion that would violate a principle in `docs/FOUNDATIONS.md` must be flagged and rejected, even if it would otherwise be an improvement.
- **Scope discipline** — Do not propose expanding the skill's scope beyond its stated intent. The audit evaluates the skill as written, not what it could become.
- **Session evidence required** — Every Issue and Improvement must cite specific session evidence (what happened, what was expected). Findings based purely on hypothetical scenarios belong in Features, not Issues.
- **Follow-up implementation** — After the report is presented, the user may request implementation of specific suggestions. At that point, edit the target skill file directly — the "report only" guardrail applies only to the audit phase, not to user-directed follow-up. When implementing multiple suggestions, apply edits in document order (top to bottom) to minimize the chance of line-number shifts invalidating later edits. After making follow-up edits, re-read the modified sections to verify: (a) edits don't overlap or contradict each other, (b) cross-references between sections remain valid (phase numbers, step numbers, file paths), (c) the skill's sequential flow still reads coherently end-to-end.
- **Cross-skill consistency** — If the target skill is part of a multi-skill workflow (e.g., plan/implement/evaluate cycle), scan sibling skills for inconsistent file references, terminology, or shared constants. Report cross-skill inconsistencies as Issues.
- **Repeated audit shortcut** — If the same skill has been audited 2+ times in the current session and the most recent audit found 0 findings, note "Skill stable — no new session evidence since last audit" and skip the full checklist unless the skill was modified between audits.
