# Report Templates (Step 7)

If `reports/scenario-analysis-report.md` already exists, check `git status` for the file. If it has uncommitted changes, warn the user before overwriting. If committed (or untracked), overwrite directly — git history preserves the prior version.

## Standard Report Template

Use when the triage checkpoint found needs failures.

```markdown
# Scenario Analysis Report

## Run Summary
- **Scenario**: `[path]`
- **Scenario purpose**: [extracted from comments, or "none stated"]
- **Seed**: [N]
- **Ticks simulated**: [N]
- **Agents**: [names and starting locations]
- **Places**: [names]
- **Total events**: [N]
- **Deaths**: [agent at tick N (cause), or "None"]

### Pre-flight Warnings
[From Step 0. For each, note whether the run confirmed or contradicted the warning.]

### Observer Notes
[If the observer crashed mid-simulation: crash tick, error message, code fixes applied, whether dump is partial. Omit if no issues.]

---

## Layer 1: Behavioral Smell Analysis

### 1. Redundant Perception — [SEVERITY]
**Agent(s)**: [affected agents]
**Evidence**: [specific data]
**Root cause hypothesis**: [analysis]

### 2. Action Loops — [SEVERITY]
[same structure]

### 3. Stuck Agents — [SEVERITY]
### 4. Failed Action Spirals — [SEVERITY]
### 5. Sustained Critical Needs — [SEVERITY]
### 6. Unaddressed Needs — [SEVERITY]
### 7. Impossible Knowledge — [SEVERITY]
### 8. Belief Staleness — [SEVERITY]
### 9. Social Isolation — [SEVERITY]
### 10. Economic Stagnation — [SEVERITY]

Report all 10 categories regardless of severity. NONE findings should be brief (1–2 sentences). INCONCLUSIVE findings should explain the data limitation.

---

## Layer 2: Needs Diagnostics

### Agent Needs Overview

| Agent | Need | Max Value | Ticks >750 permille | Death? | Root Cause Category |
|-------|------|-----------|---------------------|--------|---------------------|

[One row per agent per need that exceeded 750 permille for 100+ ticks. Healthy agents get a single row: "all needs managed".]

### Failure Classifications

#### [Agent Name]
**Categories**: [list]
**Evidence**: [specific data]
**Confidence**: [HIGH/MEDIUM/LOW]
**Causal chain**: [A → B → C summary]

[Repeat for each affected agent]

### Damning Moments

[All captured damning moments in the format specified in the Layer 2 reference.]

### Proposed Solutions

#### [Category Name]
[Solutions for each category found. Omit categories not found.]

### Golden Test Recommendations

| Priority | Damning Moment | Test Name Suggestion | What It Guards Against |
|----------|---------------|---------------------|----------------------|
| 1 | DM-[N] | golden_[descriptive_name] | [regression description] |

[Ordered: deaths first, then sustained critical needs, then moderate failures.]

---

## Layer 3: Detection Meta-Analysis

### False Positives

| Smell | Agent(s) | Why It's False | Detector Improvement |
|-------|----------|----------------|---------------------|

[One row per false positive. If none, state "No false positives identified."]

### Detection Gaps

#### Gap [N]: [Pattern Name]
**Evidence**: [specific data]
**Agent(s)**: [affected]
**Why current detectors miss it**: [analysis]
**Impact**: [CRITICAL / HIGH / MEDIUM / LOW]

[Repeat for each gap found. If none, state "No detection gaps identified — current detector coverage appears adequate for this scenario."]

### Threshold Assessment

| Threshold | Current Value | Assessment | Recommendation |
|-----------|--------------|------------|----------------|
| Stuck agent idle ticks | 20 | [assessment] | [recommendation] |
| Redundant perception count | 10 | [assessment] | [recommendation] |
| Critical need threshold | 750 permille | [assessment] | [recommendation] |
| Sustained critical duration | 100 ticks | [assessment] | [recommendation] |
| Failed action spiral rate | >75% / 5+ attempts | [assessment] | [recommendation] |
| Unaddressed need average | 750 permille | [assessment] | [recommendation] |

### Proposed New Smell Categories

[For each MEDIUM+ gap, a concrete proposal as specified in the Layer 3 reference. Do not re-propose shipped S117 smells; use the graduation note in `layer-3-meta-analysis.md` as the boundary. If no new smells warranted, state "No new smell categories proposed — current coverage is adequate."]

---

## Cross-Cutting Patterns
[Patterns spanning multiple smells, layers, or agents. Entity pollution notes. Interactions between Layer 1 findings and Layer 2 root causes.]

## Planner Diagnostics
[Include only when any agent has budget-exhausted > 0.]

| Agent | Plans Found | Frontier Exhausted | Budget Exhausted | Top Failed Goal | Candidate Count | Max Depth |
|-------|------------|-------------------|-----------------|----------------|-----------------|-----------|

Assessment: [structural vs. parametric budget exhaustion]

## Trend Comparison
[Include only if a prior `scenario-analysis-report.md` exists in git history for the same scenario and seed.]

| Category | Prior Severity | Current Severity | Delta |
|----------|---------------|-----------------|-------|

[If no prior report, omit this section.]

## Summary Statistics
- Layer 1 findings: N (categories with severity other than NONE)
- By severity: N CRITICAL, N HIGH, N MEDIUM, N LOW
- Layer 2: [N damning moments / "not triggered (healthy scenario)"]
- Layer 3: [N false positives, N detection gaps, N new smell proposals]
- Agents with issues: [list]
- Clean agents: [list]
- Scenario purpose achieved: [Yes / No / Partially — brief explanation]
```

## Healthy Scenario Report Template

Use when the triage checkpoint found no needs failures.

```markdown
# Scenario Analysis Report

## Run Summary
- **Scenario**: `[path]`
- **Scenario purpose**: [extracted or "none stated"]
- **Seed**: [N]
- **Ticks simulated**: [N]
- **Agents**: [names and starting locations]
- **Places**: [names]
- **Total events**: [N]
- **Deaths**: None

### Pre-flight Warnings
[From Step 0. For each, note whether run confirmed or contradicted.]

---

## Layer 1: Behavioral Smell Analysis

[Same 10-category structure as standard template, but most will be NONE or LOW.]

---

## Layer 2: Needs Diagnostics

*Not triggered — no agent exceeded 750 permille for 100+ consecutive ticks.*

### Agent Needs Overview

| Agent | Closest-to-Threshold Need | Max Value | Margin to 750 | Planner Health |
|-------|--------------------------|-----------|---------------|----------------|

[One row per agent. "Margin to 750" = 750 - max value.]

### Survival Strategy Summary

For each agent: where they spent time, how they obtained food/water, wash frequency, key action counts, primary survival bases.

### Margins and Risk Observations

[Which needs closest to 750 threshold. What scenario changes could push agents over. Structural observations about resource distribution.]

Note total waste items per location from Section 6. If any location has >30 Waste items, flag as "waste accumulation risk — belief stores may be polluted in longer runs" and note the count. Cross-reference with agent belief stores (Section 5) to check whether Waste entities dominate known-item counts.

---

## Layer 3: Detection Meta-Analysis

[Same structure as standard template — false positives, gaps, thresholds, proposals.]

---

## Cross-Cutting Patterns
## Summary Statistics
- Scenario purpose achieved: [Yes / No / Partially]
```
