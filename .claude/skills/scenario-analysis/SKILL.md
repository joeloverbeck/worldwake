---
name: scenario-analysis
description: "Run a scenario headlessly with the observer binary, perform comprehensive behavioral smell analysis, needs diagnostics, and meta-analysis of detection gaps and false positives. Writes report to reports/scenario-analysis-report.md."
user-invocable: true
---

# Scenario Analysis

Run a scenario headlessly via the observer binary, read the structured dump, perform three layers of analysis, and write a unified report. This skill subsumes the former `simulation-observer` and `needs-starvation-diagnostic` skills.

**Three analysis layers**:
- **Layer 1 — Behavioral Smells**: All 10 smell categories (6 mechanical + 4 LLM-only) with severity ratings.
- **Layer 2 — Needs Diagnostics**: Root-cause classification, damning moments, golden test blueprints, and proposed solutions (conditional — only when needs failures are detected).
- **Layer 3 — Detection Meta-Analysis**: Evaluates the anomaly detection system itself — false positives, detection gaps, threshold assessment, and new smell proposals.

## Invocation

```
/scenario-analysis scenarios/cli-evaluation.ron
/scenario-analysis scenarios/cli-evaluation.ron --ticks 720
/scenario-analysis scenarios/cli-evaluation.ron --days 2
```

First argument: path to a `.ron` scenario file (required).
Optional `--ticks N` to override the default of 1440 ticks (1 simulated day).
Optional `--days N` as sugar for `--ticks N*1440` (e.g., `--days 2` = 2880 ticks, `--days 3` = 4320 ticks). Deeper runs surface more failure modes.

If no scenario path is provided, glob for `scenarios/*.ron` and present the list to the user. If exactly one scenario file exists, confirm it before proceeding. If none exist, stop and report.

## Process

Follow these steps in order. Do not skip any step.

1. **Steps 0–2 — Pre-flight, observer run, dump reading.** Load `references/observer-and-dump.md`. This covers scanning the scenario file for profiles and survival gaps, building and running the observer binary (with hard gates on build/dump), and reading the 7-section dump with the Section 7 dense-row protocol.

2. **Step 3 — Triage checkpoint.** Load `references/triage-checkpoint.md`. After reading Sections 1–3, decide whether any agent has a critical-needs failure. This gates Layer 2 and selects the report template.

3. **Step 4 — Layer 1 behavioral smell analysis.** Load `references/layer-1-behavioral-smells.md`. Analyze all 10 smell categories (6 mechanical + 4 LLM-only) with severity ratings and known pathology signatures. Record data gaps for Layer 3.

4. **Step 5 — Layer 2 needs diagnostics (conditional).** If Step 3 found any agent with a need >750 permille for 100+ consecutive ticks, load `references/layer-2-needs-diagnostics.md` and run the full classification, damning-moment capture, and solution-proposal procedure. Otherwise skip this step entirely.

5. **Step 6 — Layer 3 detection meta-analysis.** Load `references/layer-3-meta-analysis.md`. Always runs. Evaluates the detection system itself — false positives, detection gaps, threshold assessment, and proposals for new smell categories.

6. **Step 7 — Write the report.** Load `references/report-templates.md`. Use the Standard template when Layer 2 ran, the Healthy Scenario template otherwise. Check git status before overwriting an existing `reports/scenario-analysis-report.md`.

7. **Step 8 — Clean up.** Delete `reports/scenario-analysis-dump.md` — the dump is an intermediate artifact. The report in `reports/scenario-analysis-report.md` is the deliverable.

## Comparison Mode

When re-running the analysis after a fix (code change, scenario edit, or profile tuning):

1. **Read the previous report** (`reports/scenario-analysis-report.md`) before running the observer.
2. **Run the analysis normally** (Steps 0–8).
3. **Add comparison metadata** to the Run Summary:
   - **Changes since last run**: Specific changes made
   - **Previous run deaths**: [N] → **This run deaths**: [N]
4. **Add Delta column** to the Agent Needs Overview table:
   - `RESOLVED` — previously >750 for 100+ ticks, now below threshold
   - `IMPROVED` — still above threshold but fewer ticks or lower max
   - `UNCHANGED` — same or similar severity
   - `REGRESSED` — worse than previous run
   - `NEW` — failure not present in previous run
5. **Cross-reference prior DMs**: Note which prior damning moments were resolved, which persist, which are new.
6. **Layer 3 comparison**: Note which prior false positives still apply, which prior gaps are now detected, and whether threshold recommendations from the prior run were applied.

## Notes

- The observer binary outputs to stderr for progress; only the markdown dump file matters.
