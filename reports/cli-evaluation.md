# CLI Readability & Usability Evaluation

Evaluation report for the Worldwake CLI app (`crates/worldwake-cli/`).

Each evaluation is produced by the `cli-improvement:evaluate` skill, which
interactively uses the CLI against `scenarios/cli-evaluation.ron` and scores
6 metrics. Evaluations are appended below the rubric.

## Metrics

| # | Metric | What It Measures |
|---|--------|-----------------|
| 1 | Output Clarity | Human-readable output, no `{:?}` debug format, no raw IDs |
| 2 | Action Reliability | Listed actions work when selected, no missing-profile errors |
| 3 | State Legibility | World state is scannable and well-labeled |
| 4 | Causal Traceability | Can trace WHY something happened with clear explanations |
| 5 | Session Flow | Command sequence feels natural, transitions smooth |
| 6 | Error Recovery | Errors explain what went wrong and suggest alternatives |

## Scoring Guide

- **1-3**: Unusable — debug format, cryptic errors, incomprehensible
- **4-5**: Poor — partially functional but confusing
- **6-7**: Adequate — works but not intuitive
- **8-9**: Good — clear, intuitive, well-organized
- **10**: Excellent — a developer unfamiliar with the project could understand everything

## Graduation

Average score >= 8.0 and no CRITICAL or HIGH recommendations remaining.
Re-enter the loop when new simulation features add CLI surface area.

## What to Look For

- Raw internal identifiers or debug format (`{:?}`) exposed to the user
- Actions listed as available that error when selected (missing profiles, precondition failures)
- Component output that is machine-readable but not human-friendly
- Event deltas printed in raw/debug format without context
- Implicit rules the user must memorize (e.g., run `actions` before `do`)
- Missing error context or recovery suggestions
- Unclear timing of action execution vs. enqueuing
- Stale affordances after ticking without warning
- Regressions from previous evaluations

## Evaluation Template

Each evaluation appended below follows this structure:

---

## EVALUATION #N

**Date**: YYYY-MM-DD
**Scenario**: scenarios/cli-evaluation.ron
**Transcript**: reports/cli-evaluation-transcripts/eval-N.txt

### Session Notes

[Narrative of the interactive CLI session — what was tried, what happened, what was confusing]

### Per-Command Analysis

[For each command exercised, note: output quality, issues found, suggestions]

### Resolved Since Previous

- [Issue description] — was [SEVERITY] in Eval #M, now fixed.
[If first evaluation or none resolved: "No previous evaluation." or "No issues resolved."]

### Scores

| # | Metric | Score | Previous | Delta | Justification |
|---|--------|-------|----------|-------|---------------|
| 1 | Output Clarity | X | — | — | [brief] |
| 2 | Action Reliability | X | — | — | [brief] |
| 3 | State Legibility | X | — | — | [brief] |
| 4 | Causal Traceability | X | — | — | [brief] |
| 5 | Session Flow | X | — | — | [brief] |
| 6 | Error Recovery | X | — | — | [brief] |
| | **Average** | **X.X** | **—** | **—** | |

### Score Trend (include if 5+ evaluations exist)

| Eval | Avg | Delta |
|------|-----|-------|
| #N | X.X | — |

### Prioritized Recommendations

1. **[CRITICAL]** ... *(New)*
2. **[HIGH]** ...
3. **[MEDIUM]** ...
4. **[LOW]** ...
