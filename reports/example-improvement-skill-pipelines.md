# Example Improvement Skill Pipelines

The following are skills that together form a pipeline to improve aspects of a different repository. These skills are meant as examples for us to create local skill sets that also serve as pipelines for improvement.

---
name: train-operation-ui-evaluate
description: Use when new UI screenshots have been captured and need evaluation. Reads screenshots/fitl-train-*.png, scores 6 readability metrics, and appends the next EVALUATION #N to reports/ui-readability-evaluation.md. Invoke after manually capturing screenshots of the Train operation.
---

# UI Readability Evaluation

Score the current UI state from screenshots and append a structured evaluation to the report.

## Checklist

1. Read `reports/ui-readability-evaluation.md` — absorb the rubric and the last 2-3 evaluations. The file grows with each evaluation; use this strategy:
   - Read the first ~30 lines for the rubric and scoring guide
   - Count total lines (`wc -l`), then read from `offset = totalLines - 200` to get the last 2-3 evaluations in one pass (each evaluation is ~70-100 lines)
   - To build the Score Trend table efficiently, grep for `\*\*Average\*\*` in the report file — this returns all historical averages in one pass
   - Skip intermediate evaluations unless checking recurring issue history
2. Glob for `screenshots/fitl-train-*.png` to discover all available screenshots, then read them in **parallel batches of 5-6** (use multiple Read tool calls in a single message). This minimizes tool call rounds.
3. Determine the next evaluation number from the last `## EVALUATION #N` heading
4. **If the screenshot count changed** from the previous evaluation, note this prominently. Explain what new screenshots capture and add a comparability caveat (see Screenshot Set Changes below).
5. For each screenshot, write a paragraph describing what's shown and listing specific issues
6. Score all 6 metrics (1-10) with brief justification per metric
7. Compute score deltas from the previous evaluation
8. List resolved issues from the previous evaluation (see template)
9. Write prioritized recommendations tagged CRITICAL / HIGH / MEDIUM / LOW
10. Flag recurring issues — note how many consecutive evaluations each issue has persisted
11. If 5+ evaluations exist, include a Score Trend table (see template)
12. Append the complete evaluation section to `reports/ui-readability-evaluation.md`

## Evaluation Template

Append exactly this structure:

```markdown
---

## EVALUATION #N

**Date**: YYYY-MM-DD
**Screenshots analyzed**: fitl-train-1.png through fitl-train-N.png
[If screenshot count changed: **Screenshot set change**: Expanded from M to N screenshots. New screenshots capture [brief description]. Scores may reflect newly visible issues, not regressions — see comparability note below.]

### Screenshot Analysis

#### fitl-train-1.png — [Brief title]
**What's shown**: [1-2 sentences]
**Issues observed**: [bullet list]

[...repeat for each screenshot...]

### Resolved Since Previous

- [Issue description] — was [SEVERITY] in Eval #M, now fixed. [Brief description of the fix.]
[If none: "No issues from the previous evaluation were resolved."]

### Scores

| # | Metric | Score | Previous | Delta | Justification |
|---|--------|-------|----------|-------|---------------|
| 1 | Decision Prompt Clarity | X | Y | +/-Z | [brief] |
| 2 | Option Legibility | X | Y | +/-Z | [brief] |
| 3 | Breadcrumb Navigability | X | Y | +/-Z | [brief] |
| 4 | Error Communication | X | Y | +/-Z | [brief] |
| 5 | Information Density | X | Y | +/-Z | [brief] |
| 6 | Visual Hierarchy | X | Y | +/-Z | [brief] |
| | **Average** | **X.X** | **Y.Y** | **+/-Z.Z** | |

[If screenshot set changed: **Comparability note**: This evaluation covers N screenshots (previous: M). Score changes may partly reflect expanded coverage revealing pre-existing issues rather than regressions introduced since the last evaluation.]

### Score Trend (include if 5+ evaluations exist)

| Eval | Avg | Delta |
|------|-----|-------|
| #N-4 | X.X | +/-Z.Z |
| #N-3 | X.X | +/-Z.Z |
| #N-2 | X.X | +/-Z.Z |
| #N-1 | X.X | +/-Z.Z |
| #N   | X.X | +/-Z.Z |

[If the trend shows oscillation (alternating positive/negative deltas for 4+ evaluations), note this explicitly: "The score is oscillating — fixes are likely introducing regressions. Consider a more cautious implementation approach."]

### Prioritized Recommendations

1. **[CRITICAL]** ... *(Recurring: N consecutive evaluations)* | *(New regression — major: -2 or more on metric X)*
2. **[HIGH]** ...
3. **[MEDIUM]** ...
4. **[LOW]** ...
```

## Scoring Guide

- **1-3**: Unusable — raw internal names, incomprehensible layout
- **4-5**: Poor — partially readable but confusing
- **6-7**: Adequate — functional but not intuitive
- **8-9**: Good — clear, intuitive, well-organized
- **10**: Excellent — a player unfamiliar with the game could understand the UI

## What to Look For

- Raw internal identifiers exposed to the player (AST paths, binding names, internal jargon)
- Breadcrumb entries that are walls of unreadable text or lack iteration context
- Missing or misleading error explanations
- Formatting artifacts on labels (duplicated prefixes, trailing suffixes, "None" appended to names)
- Cramped layout where breadcrumbs dominate over the actual decision
- Unclear selection ranges without context
- Visual clutter competing with the primary decision
- Semantically misleading styling (e.g., strikethrough on selected items)
- **Regressions** — issues that were absent in previous evaluations but appeared after recent changes

## Screenshot Set Changes

When the number of screenshots changes between evaluations:
- Note the change in the evaluation header
- Describe what the new (or removed) screenshots capture
- Update the **Screenshot Reference** section near the top of the report file to describe all current screenshots
- Mark issues found only in new screenshots as "newly visible" rather than "regression" — these issues may have always existed but were not captured before
- Scores may drop due to expanded coverage without any code change. Add the comparability note to the scores section to prevent misinterpreting this as a regression.

## Regression Severity

Classify regressions by the metric score drop they cause:
- **Major regression** (metric drops by 2+): Tag as `*(Major regression: -N on [Metric])*`. These indicate a fix broke something substantially.
- **Minor regression** (metric drops by 1): Tag as `*(Regression: -1 on [Metric])*`. These may be acceptable trade-offs.
- Regressions that affect multiple metrics simultaneously are especially concerning — call this out.

## Recurring Issue Tracking

When writing recommendations, check prior evaluations to determine if each issue is new or recurring:
- If an issue appeared in the previous evaluation, note it as "Recurring: N consecutive evaluations"
- Issues persisting for 3+ evaluations should be *considered* for escalation — weigh both persistence and impact severity when deciding (a LOW cosmetic issue persisting for 5 evaluations doesn't automatically become CRITICAL)
- New regressions (issues not present in the previous evaluation) should be called out explicitly as regressions
- If a previously reported issue is now resolved, note this in the "Resolved Since Previous" section

## Stagnation Detection

Stagnation occurs when **both** conditions are met:
1. The same issue has been the top actionable recommendation for 3+ consecutive evaluations
2. The average score has not improved by 0.5+ points across those evaluations

When stagnation is detected, note it explicitly and suggest shifting to the `train-operation-ui-implement` skill to address the structural issues before running another evaluation cycle.

If the Score Trend shows oscillation (alternating positive/negative deltas for 4+ evaluations), this suggests fixes are introducing regressions. Note this pattern and recommend a more cautious, incremental implementation approach.

## Report File Maintenance

When the report file exceeds ~500 lines or ~10 evaluations, archive older evaluations.

**What to keep in the active file**: The rubric/header (everything before the first `---` separator) and the last 5 evaluations.

**Archival procedure**:
1. Identify which evaluations to archive — keep the rubric + last 5 evaluations
2. Grep for `^## EVALUATION #` to find all evaluation line numbers
3. Find the line number of the `---` separator immediately before the oldest evaluation to keep
4. Read the content to be archived (everything between the rubric and that separator)
5. Write or append to `reports/ui-readability-evaluation-archive.md`:
   - If the archive file does not exist, create it with a header: `# UI Readability Evaluation — Archive` and a brief description line
   - If it already exists, append the new archived evaluations after the existing content
6. Archive evaluations **verbatim** — do not condense or summarize. The archive is a historical record.
7. Remove the archived evaluations from the active file using bash: `head -N file > trimmed && tail -n +M file >> trimmed && mv trimmed file` (where N = last rubric line, M = first line of the oldest kept evaluation's `---` separator)
8. Verify: grep for `## EVALUATION #` in both files to confirm the correct split

## Graduation

If the average score reaches **8.0+** and no CRITICAL or HIGH recommendations remain, note in the evaluation that the UI has graduated to acceptable quality. Further evaluations are optional — invoke only after significant UI changes.

## Scope

This skill is scoped to the FITL Train operation (`screenshots/fitl-train-*.png`). To evaluate other operations or games, create a copy with adjusted screenshot glob pattern and report file path. The evaluation methodology and 6 metrics apply unchanged.

----

---
name: train-operation-ui-implement
description: Use when the latest UI evaluation is ready and improvements need to be implemented. Reads the most recent EVALUATION from reports/ui-readability-evaluation.md, brainstorms solutions for the lowest-scoring metrics, and implements changes to the runner UI choice panel components.
---

# UI Readability Implementation

Improve the runner UI based on the latest evaluation's scores and recommendations.

> **Note**: The skill is named "train-operation" because Train is the evaluation vehicle, but the choice panel is shared across all operations. Fixes should target shared primitives — not Train-specific logic. After implementing, check other operations for the same gaps (see Cross-Operation Check below).

## Checklist

1. Read `reports/ui-readability-evaluation.md` — focus on the latest EVALUATION #N
2. Identify the CRITICAL and HIGH recommendations. If none exist, target the top 2-3 MEDIUM recommendations instead.
3. Note which metrics scored lowest — these are the priority targets
4. Read the relevant source files (see Key Files below)
5. Trace the data flow (see Data Flow Reference) and use Fix Category Triage to classify each issue as data pipeline or display logic
6. For the top 2-3 recommendations, identify the specific file and function to change before writing code
7. If a fix approach is ambiguous, apply the 1-3-1 rule (1 problem, 3 options, 1 recommendation) before proceeding — per Foundation #10 (Architectural Completeness)
8. Implement changes, focusing on the highest-impact items first
9. **Cross-Operation Check**: After implementing visual config additions, grep for similar param names across other operations in `visual-config.yaml`. If a param like `subAction` or `pacLevels` appears in Rally, Sweep, etc., add config entries for those too.
10. Run verification: `pnpm turbo typecheck` and `pnpm -F @ludoforge/runner test`
11. Do NOT update the evaluation report — that happens in the next evaluate session

## Key Files

| File | What It Controls |
|------|-----------------|
| `packages/runner/src/model/render-model.ts` | Type definitions for `RenderChoiceContext`, `RenderModel`, `RenderChoiceOption`, etc. |
| `packages/runner/src/model/runner-frame.ts` | Intermediate types — `RunnerChoiceStep` (breadcrumb entry shape), `RunnerChoiceContext`, `RunnerFrame` |
| `packages/runner/src/model/project-render-model.ts` | Display name resolution for zones, tokens, breadcrumbs, choice options. Contains `resolveIterationEntityDisplayName` (zone-only iteration label lookup) and `resolveChoiceOptionDisplayName` (visual config option overrides). |
| `packages/runner/src/model/derive-runner-frame.ts` | Derives `RunnerFrame` from engine state — source of `choiceContext`, breadcrumb steps, `iterationEntityId`. Contains `isKnownZone` (base-ID prefix matching for zone validation). |
| `packages/runner/src/model/iteration-context.ts` | `parseIterationContext()` — extracts iteration index, total, and entity ID from decision keys and the choice stack |
| `packages/runner/src/ui/ChoicePanel.tsx` | Choice panel layout, breadcrumb rendering, option buttons, header, multi-select counter bounds |
| `packages/runner/src/ui/ChoicePanel.module.css` | All choice panel styling — colors, spacing, typography |
| `packages/runner/src/utils/format-display-name.ts` | ID-to-display-name conversion (kebab-case, camelCase, snake_case) |
| `packages/runner/src/model/choice-value-utils.ts` | Choice value formatting with fallback strategies |
| `packages/runner/src/config/visual-config-types.ts` | Zod schemas for visual config (`ActionChoiceVisualSchema`, `ActionVisualSchema`, etc.) — must be updated when extending the config contract |
| `packages/runner/src/config/visual-config-provider.ts` | Visual config accessor methods (zone labels, display names, choice prompts, choice labels, choice option display names) |
| `packages/runner/src/ui/bottom-bar-mode.ts` | Maps `choiceUi.kind` to `ChoicePanelMode` — determines whether the panel shows pending choice, confirm, or invalid state |
| `packages/runner/src/ui/GameContainer.tsx` | Top-level layout that positions the choice panel |
| `packages/runner/src/store/store-types.ts` | Defines `RenderContext` — the input shape for all `derive-runner-frame.ts` derivation functions (`selectedAction`, `choicePending`, `choiceStack`, etc.) |
| `data/games/fire-in-the-lake/visual-config.yaml` | FITL-specific visual configuration overrides |

## Key Test Files

| File | What It Covers |
|------|---------------|
| `packages/runner/test/model/project-render-model-state.test.ts` | Render model projection tests — `choiceContext`, `decisionPrompt`, `decisionLabel`, breadcrumbs, `iterationLabel` |
| `packages/runner/test/ui/ChoicePanel.test.ts` | ChoicePanel component rendering — `ChoiceContextHeader`, breadcrumb display, multi-select mode |
| `packages/runner/test/ui/helpers/render-model-fixture.ts` | Central `RenderModel` fixture factory — must be updated when adding fields to `RenderModel` |

## Data Flow Reference

Understanding where values originate is critical for fixing display issues:

```
Engine ChoicePending (name, decisionKey, options)
  -> derive-runner-frame.ts -> RunnerFrame.choiceContext / choiceBreadcrumb
       (sets: decisionParamName, iterationEntityId, iterationGroupId)
    -> project-render-model.ts -> RenderModel.choiceContext / choiceBreadcrumb
         (resolves: decisionLabel, decisionPrompt, iterationLabel, displayName)
      -> ChoicePanel.tsx -> ChoiceContextHeader / CollapsedBreadcrumb
           (composes final display text from label + prompt + bounds + iteration)
```

Key transform points:
- `derive-runner-frame.ts` extracts `iterationEntityId` from `parseDecisionKey(decisionKey)` — this is where forEach iteration context enters. The breadcrumb entry type is `RunnerChoiceStep` (from `runner-frame.ts`).
- `iteration-context.ts` contains `parseIterationContext()` which maps decision key iteration paths to entity IDs via the choice stack — the core function for breadcrumb forEach context.
- `project-render-model.ts` resolves display names via visual config -> zone lookup -> null (non-zone entities are suppressed)
- `ChoicePanel.tsx` (`ChoiceContextHeader`) concatenates `decisionLabel`, `decisionPrompt`, bounds, `iterationLabel`, and `iterationProgress` into the final prompt string

### Multi-Select Counter Bounds

The multi-select counter ("Selected: X of Y") computes its bounds separately from the header prompt. The bounds path is:

```
ChoicePanel.tsx effectiveContext useMemo
  -> deriveMultiSelectBounds(min, max, effectiveLegalCount) -> boundsText for prompt
MultiSelectMode component
  -> deriveMultiSelectBounds(min, max, effectiveOptionCount) -> bounds for counter text
```

Both call `deriveMultiSelectBounds(min, max, optionCount)` where `optionCount` caps the `max` value. When computing `optionCount`, **include options that are already selected** — otherwise the count drops to 0 when all options are selected and their legality changes to `illegal` (since they can't be re-added). The effective count formula is: options where `legality !== 'illegal'` OR `choiceValueId` is in the selected set.

### Fix Category Triage

Most eval issues fall into one of two categories:

- **Data pipeline fix** (wrong *content*): The rendered values are incorrect, duplicated, or missing. Fix in `derive-runner-frame.ts` (entity extraction) or `project-render-model.ts` (display name resolution). Examples: raw AST paths, missing iteration labels, duplicated label suffixes.
- **Display logic fix** (wrong *presentation*): The values are correct but shown poorly. Fix in `ChoicePanel.tsx` (layout, concatenation) or `ChoicePanel.module.css` (spacing, colors, typography). Examples: cramped breadcrumbs, weak visual distinction, layout hierarchy issues.
- **Model extension** (missing *plumbing*): The data exists in the derivation context (`RenderContext` in `store-types.ts`) but isn't exposed on `RunnerFrame` or `RenderModel`. Add the field to the interface, set it in `derive-runner-frame.ts`, resolve the display name in `project-render-model.ts`, and update all test fixtures (~9 files construct `RunnerFrame` or `RenderModel` manually). Examples: exposing `selectedActionId` for confirm screen prompts, adding new display name fields.

### Confirm Screen State

On the final confirmation screen (`choiceUi.kind === 'confirmReady'`), `choicePending` is null but `selectedAction` is still available in the derivation context. This means:
- `choiceContext` on `RunnerFrame` and `RenderModel` is `null` (no pending decision)
- `selectedActionId` on `RunnerFrame` is still set (the action is selected, just fully parameterized)
- `selectedActionDisplayName` on `RenderModel` resolves the action's display name
- `ChoicePanel.tsx` receives `mode === 'choiceConfirm'` (from `bottom-bar-mode.ts`)
- The `ChoiceContextHeader` is skipped when `choiceContext` is null — any confirm-specific prompt must be rendered separately

### Breadcrumb Group Header Labels

Breadcrumb group headers (e.g., "Additional Space (1x)") get their display text from:
1. `CollapsedBreadcrumb` in `ChoicePanel.tsx` reads `firstStep.displayName` from the first step in the group (line ~410)
2. `displayName` is set in `project-render-model.ts` breadcrumb mapping — it checks visual config `getChoiceLabel()` first, then falls back to `humanizeDecisionParamName(step.name)`
3. Visual config labels (e.g., `subActionSpaces.label: "Additional Space"`) override the auto-generated Title Case conversion

To add a new group header override: add a `label` field under the action's `choices.<paramName>` in `visual-config.yaml`. The breadcrumb projection will pick it up automatically.

## Architecture Context

The choice panel display name resolution has 3 layers:
1. **Visual config override** — `visualConfigProvider.getZoneLabel(zoneId)` checks game-specific config
2. **Render model projection** — `projectRenderModel()` resolves display names for zones, tokens, breadcrumbs
3. **Fallback formatter** — `formatIdAsDisplayName()` converts raw IDs to Title Case

Raw `$variable` names and AST paths appear when the render model falls back to formatting raw internal decision keys as display names. The fix usually involves either:
- Adding display name resolution logic in `project-render-model.ts`
- Adding visual config overrides in the game's `visual-config.yaml`
- Improving the fallback formatting in `format-display-name.ts`
- Note: we have comprehensive raw AST humanization code (`humanizeDecisionParamName` in `format-display-name.ts`) that extracts the last meaningful segment from AST paths. Use this instead of `formatIdAsDisplayName` when the input might be an AST path.

Per Foundation #3 (Visual Separation), when a display issue involves a *missing* visual config entry, the fix is always a config addition + optional accessor wiring in the runner — never an engine change or GameSpecDoc modification.

### Extending Visual Config

When the existing visual config schema doesn't have the field you need (e.g., overriding a decision *label* rather than just a *prompt*), follow this pattern:

1. **Schema** (`visual-config-types.ts`): Add the field to the relevant Zod schema (e.g., `ActionChoiceVisualSchema`). Use `z.string().optional()` for new optional fields.
2. **Accessor** (`visual-config-provider.ts`): Add a getter method (e.g., `getChoiceLabel(actionId, paramName)`) that reads the new field from `this.config`.
3. **Consumer** (`project-render-model.ts`): Call the new accessor in the projection function, preferring the config value over the auto-generated fallback.
4. **Game config** (`data/games/fire-in-the-lake/visual-config.yaml`): Add the actual override values under the appropriate action/choice path.

### Option Display Name Overrides

Choice option display names (e.g., "None" -> "Skip") are resolved through visual config. The flow:

1. `ActionChoiceVisualSchema` has an `options` field: `z.record(z.string(), ActionChoiceOptionVisualSchema)` where each entry has an optional `displayName`.
2. `visualConfigProvider.getChoiceOptionDisplayName(actionId, paramName, optionValue)` reads the override.
3. `resolveChoiceOptionDisplayName()` in `project-render-model.ts` checks the visual config override FIRST, before the zone/token/fallback chain.

To add a new option override, add it to `visual-config.yaml`:
```yaml
actionId:
  choices:
    paramName:
      options:
        optionValue:
          displayName: Human-Friendly Label
```

### iterationLabel rendering path

`iterationLabel` comes from `iterationEntityId` in `derive-runner-frame.ts`. The render model resolves it via `resolveIterationEntityDisplayName()`:
- If the entity matches a zone in `zonesById` (exact match or base-ID prefix match), the zone's display name is used.
- If the entity is NOT a known zone (e.g., a decision param name fallback), `null` is returned — the label is suppressed to prevent internal jargon from leaking into the UI.
- The `ChoiceContextHeader` renders non-null `iterationLabel` as ` — ${iterationLabel}` after the prompt.

Breadcrumb entries use the same zone-only resolution. When `iterationLabel` is null for grouped breadcrumb entries, the fallback rendering shows `(1/3)` numbering instead.

The `deriveChoiceBreadcrumb` function has three fallbacks for setting `iterationEntityId`:
1. `parseIterationContext()` — forEach path in decision key
2. `resolvedBind` from `parseDecisionKey()` — only if the bind is a known zone (validated via `isKnownZone()`)
3. Array-index lookup — walks backward through the choice stack, finds the most recent array-valued choice, and indexes into it using the step's position within its group

If fallback #2 sets a non-zone value (e.g., a param name), it blocks fallback #3 from running. That's why #2 validates against `zonesById` first.

## Known Gotchas

These are hard-won lessons from previous implementation sessions. Check this section before implementing any fix.

### Zone ID composite key mismatch

`zonesById` in the render model uses **composite keys** (`zoneId:owner`, e.g., `table:none`, `binh-dinh:none`). But engine iteration entities and choice stack values use **base zone IDs** (`table`, `binh-dinh`). When checking if an entity is a known zone:
- Try `zonesById.get(entityId)` first (exact match)
- Fall back to prefix matching: check if any zone ID starts with `entityId + ':'`
- Helper functions exist: `isKnownZone()` in `derive-runner-frame.ts` and `resolveIterationEntityDisplayName()` in `project-render-model.ts`

### Counter bounds must include selected items

`deriveMultiSelectBounds(min, max, legalOptionCount)` uses `legalOptionCount` to cap `max`. When all options are selected, the engine may mark them `illegal` (can't add more), dropping `legalOptionCount` to 0. This produces "Selected: 3 of 0". Always compute `legalOptionCount` as options that are either legal OR in the selected set.

### iterationLabel deduplication still applies

`projectChoiceContext` suppresses `iterationLabel` when it matches `decisionLabel` (e.g., both resolve to "Target Spaces"). Don't remove this check or all prompts will show a redundant trailing "— Label" suffix.

### resolvedBind is not always a zone

`parseDecisionKey(key).resolvedBind` can be a zone ID (e.g., `binh-dinh`) or a decision param name (e.g., `trainChoice`). The 2nd fallback in `deriveChoiceBreadcrumb` must validate it against `zonesById` before using it as `iterationEntityId` — otherwise the 3rd fallback (array-index lookup, which correctly finds the target zone) is blocked.

## Common Pitfalls

- **Label duplication**: Don't embed labels into `decisionPrompt` if `iterationLabel` or the `ChoiceContextHeader` also renders a label. There should be exactly one place that controls label display.
- **AST path fallback**: `formatIdAsDisplayName()` does NOT strip AST path prefixes. Use `humanizeDecisionParamName()` when the input might be an AST path (e.g., `iterationEntityId` fallback, breadcrumb step names).
- **Prompt composition check**: When changing how `decisionPrompt`, `decisionLabel`, or `iterationLabel` are set, always verify what `ChoiceContextHeader` concatenates — it combines multiple fields into one visible string.
- **Test field contracts**: `RenderChoiceContext` is constructed directly in `ChoicePanel.test.ts`, and `RunnerFrame` / `RenderModel` are constructed manually in ~9 test fixture files (`canvas-updater.test.ts`, `table-overlay-renderer.test.ts`, `render-model-types.test.ts`, `presentation-scene.test.ts`, `project-table-overlay-surface.test.ts`, `GameContainer.test.ts`, `bottom-bar-mode.test.ts`, `render-model-fixture.ts`, `project-render-model-victory-standings.test.ts`). Adding a new field to any of these interfaces requires updating all constructing fixtures.

## Scope Constraints

- Changes should improve ALL operations, not just Train — focus on shared primitives
- Do not modify engine code (`packages/engine/`) — UI-only changes (Foundation #3: Visual Separation)
- Do not change game logic or game spec YAML — only rendering and display
- Keep CSS changes within the existing design token system (`--bg-panel`, `--accent`, etc.)
- The proposed changes should align with docs/FOUNDATIONS.md

---

---
name: map-representation-evaluate
description: Use when new map screenshots have been captured and need evaluation. Reads screenshots/fitl-game-map*.png and screenshots/fitl-map-editor*.png, scores 4 map representation metrics, and appends the next EVALUATION #N to reports/map-representation-evaluation.md. Invoke after manually capturing screenshots of the FITL game map and map editor.
---

# Map Representation Evaluation

Score the current FITL map rendering state from screenshots and append a structured evaluation to the report.

## Checklist

1. Read `reports/map-representation-evaluation.md` — absorb the rubric and the last 2-3 evaluations. The file grows with each evaluation; use this strategy:
   - If the file has fewer than 400 lines, read the entire file in one pass — the two-pass strategy below is unnecessary.
   - Otherwise: read the first ~40 lines for the rubric, metrics, and scoring guide. Count total lines (`wc -l`), then read from `offset = totalLines - 200` to get the last 2-3 evaluations in one pass (each evaluation is ~60-80 lines).
   - To build the Score Trend table efficiently, grep for `\*\*Average\*\*` in the report file — this returns all historical averages in one pass.
   - Skip intermediate evaluations unless checking recurring issue history.
2. Discover and read all current screenshots:
   - Glob `screenshots/fitl-game-map*.png` and `screenshots/fitl-map-editor*.png` to find all current screenshots. The minimum expected set is `fitl-game-map.png` and `fitl-map-editor.png` — any additional matches (e.g., `fitl-game-map-overview.png`) must also be read and evaluated.
   - Run `ls -la screenshots/fitl-*.png screenshots/fitl-*.jpg` to verify freshness — if any screenshot is older than 24 hours, warn the user that it may not reflect the current rendering state. (The Glob discovers the file list; `ls` is solely for timestamps.)
   - Read all discovered screenshots in **parallel** (all Read tool calls in a single message). Also read `screenshots/FITL_SC1.jpg` (physical board reference) — **required** for the first evaluation to establish the ground truth baseline. For subsequent evaluations, read it when any of these apply: (a) evaluating geographic layout fidelity, (b) the _previous_ evaluation's recommendations reference the physical board layout, or (c) there is a significant layout change (e.g., territory coverage extended to new provinces, major shape rework).
3. Determine the next evaluation number from the last `## EVALUATION #N` heading.
4. **If the screenshot count changed** from the previous evaluation, note this prominently. Explain what new screenshots capture, add a comparability caveat (see Screenshot Set Changes below), and update the **Screenshot Reference** section at the top of the report file to describe all current screenshots.
5. For each screenshot, write a paragraph describing what's shown and listing specific issues related to the 4 metrics.
6. Score all 4 metrics (1-10) with brief justification per metric.
7. Compute score deltas from the previous evaluation. "Previous evaluation" means the most recent *scored* evaluation — skip any No Change stubs when looking up previous scores. For the first evaluation, use `—` for Previous and Delta columns.
8. List resolved issues from the previous evaluation (see template). For the first evaluation, write: "No previous evaluation exists — this is the baseline evaluation."
9. Write prioritized recommendations tagged CRITICAL / HIGH / MEDIUM / LOW.
10. Flag recurring issues — note how many consecutive evaluations each issue has persisted.
11. If 3+ evaluations exist, include a Score Trend table (see template).
12. Append the complete evaluation section to `reports/map-representation-evaluation.md`.

## Evaluation Template

Append exactly this structure:

```markdown
---

## EVALUATION #N

**Date**: YYYY-MM-DD
**Screenshots analyzed**: fitl-game-map.png, fitl-map-editor.png
[If screenshot count changed: **Screenshot set change**: Expanded from M to N screenshots. New screenshots capture [brief description]. Scores may reflect newly visible issues, not regressions — see comparability note below.]

### Screenshot Analysis

For each screenshot analyzed, add a section:

#### [screenshot-filename] — [View Description]
**What's shown**: [1-2 sentences describing the view state]
**Issues observed**: [bullet list of specific issues related to the 4 metrics]

### Cross-View Consistency

[Note any discrepancies between the game canvas and editor rendering of the same geographic area — e.g., different polygon shapes, missing routes, color mismatches, or elements visible in one view but absent in the other. If views are consistent, write: "Game canvas and editor views are consistent for the overlapping area."]

### Resolved Since Previous

- [Issue description] — was [SEVERITY] in Eval #M, now fixed. [Brief description of the fix.]
[If none: "No issues from the previous evaluation were resolved." Optionally add context: e.g., "rendering unchanged", "no implementation cycle between evaluations".]

### Scores

| # | Metric | Score | Previous | Delta | Justification |
|---|--------|-------|----------|-------|---------------|
| 1 | Adjacency Clarity | X | Y | +/-Z | [brief] |
| 2 | Road/River Integration | X | Y | +/-Z | [brief] |
| 3 | Terrain Distinction | X | Y | +/-Z | [brief] |
| 4 | Label/Token Readability | X | Y | +/-Z | [brief] |
| | **Average** | **X.X** | **Y.Y** | **+/-Z.Z** | |

[If screenshot set changed: **Comparability note**: This evaluation covers N screenshots (previous: M). Score changes may partly reflect expanded coverage revealing pre-existing issues rather than regressions introduced since the last evaluation.]

[If rendering changed but some metrics are unchanged: briefly explain why the visual change didn't affect those metrics — see "Visual Change Without Score Movement" section.]

[If territory rendering is being tracked: **Territory coverage**: N/M province zones rendered as territories (vs. rectangles). Approximate counts from visual inspection are acceptable — use `~` prefix for estimates.]

### Score Trend (include if 3+ evaluations exist)

| Eval | Avg | Delta |
|------|-----|-------|
| #N-4 | X.X | +/-Z.Z |
| #N-3 | X.X | +/-Z.Z |
| #N-2 | X.X | +/-Z.Z |
| #N-1 | X.X | +/-Z.Z |
| #N   | X.X | +/-Z.Z |

[If the trend shows oscillation (alternating positive/negative deltas for 4+ evaluations), note this explicitly: "The score is oscillating — fixes are likely introducing regressions. Consider a more cautious implementation approach."]

### Prioritized Recommendations

1. **[CRITICAL]** ... *(Recurring: N consecutive evaluations)* | *(New regression — major: -2 or more on metric X)*
2. **[HIGH]** ...
3. **[MEDIUM]** ...
4. **[LOW]** ...
```

## Correction Protocol

If the user disputes part of an already-appended evaluation:
1. Do NOT append a new evaluation — edit the existing one in-place.
2. Re-read any additional screenshots or evidence the user points to.
3. Update the specific observations, scores, and recommendations that are affected.
4. Re-verify the average and delta calculations after any score change.
5. Add a `**Corrections**` line immediately after the `**Date**` line: `**Corrections**: [YYYY-MM-DD] Revised [metric name] score from X to Y after reviewing [screenshot/evidence]. [Brief reason.]`
6. Move any newly-resolved items to the "Resolved Since Previous" section if the correction reveals they were already fixed.

### Replacing a No Change Stub

If the user disputes a No Change stub (indicating rendering did change):
1. Replace the entire stub with a full evaluation following the standard Evaluation Template.
2. Add a `**Corrections**` line immediately after the `**Date**` line: `**Corrections**: [YYYY-MM-DD] Replaced No Change stub after reviewing [screenshot/evidence]. [Brief reason.]`
3. Proceed with the full evaluation checklist (screenshot analysis, scores, deltas, recommendations).

## Scoring Guide

- **1-3**: Unusable — rectangles with disconnected lines, no spatial relationship between provinces
- **4-5**: Poor — some improvement but provinces still feel like isolated boxes
- **6-7**: Adequate — provinces have territory-like shapes, adjacencies partially implied by borders
- **8-9**: Good — provinces share borders naturally, routes flow through territories, terrain is clear
- **10**: Excellent — a player familiar with the physical board would recognize the map immediately

## What to Look For

- Provinces that lack shared borders — isolated shapes (rectangles, disconnected polygons) with gaps between them (worst case: uniform rectangles floating in empty space)
- Adjacency conveyed only through explicit lines rather than implied by geography (shared borders, proximity)
- Routes (roads, rivers) that terminate at province edges rather than flowing naturally through territory
- Route types that are visually indistinguishable from each other (roads vs. rivers should have distinct styling)
- Terrain types that are indistinguishable (all same shade/color regardless of terrain category)
- Province labels obscured by shape borders, tokens, or adjacency lines
- Label background pills that clip text, render over tokens, or produce garbled characters
- Token stacks that overflow province boundaries
- Wasted space between provinces where borders should be shared
- Missing or misleading adjacency connections
- Routes that cross provinces they shouldn't pass through
- Cities (circles) feeling disconnected from their surrounding provinces
- Province shapes that don't support natural route flow-through
- Visual congestion in areas with many small provinces — overlapping territory borders, route lines, and tokens that make individual zones hard to distinguish (score under Adjacency Clarity or Label/Token Readability as appropriate)
- **Regressions** — issues absent in previous evaluations that appeared after recent changes

## Screenshot Set Changes

When the number of screenshots changes between evaluations:
- Note the change in the evaluation header
- Describe what the new (or removed) screenshots capture
- Update the **Screenshot Reference** section near the top of the report file to describe all current screenshots
- Mark issues found only in new screenshots as "newly visible" rather than "regression"
- Add the comparability note to the scores section

## Regression Severity

Classify regressions by the metric score drop they cause:
- **Major regression** (metric drops by 2+): Tag as `*(Major regression: -N on [Metric])*`. These indicate a fix broke something substantially.
- **Minor regression** (metric drops by 1): Tag as `*(Regression: -1 on [Metric])*`. These may be acceptable trade-offs.
- Regressions that affect multiple metrics simultaneously are especially concerning — call this out.

## Recurring Issue Tracking

When writing recommendations, check prior evaluations to determine if each issue is new or recurring:
- If an issue appeared in the previous evaluation, note it as "Recurring: N consecutive evaluations"
- When tagging recurring issues, note whether the associated metric is stable, improving, or declining. Escalation at 3+ evaluations is a consideration trigger, not an automatic action — weigh persistence alongside metric trajectory
- New regressions should be called out explicitly
- If a previously reported issue is now resolved, note this in the "Resolved Since Previous" section

## Stagnation Detection

### Overall stagnation

Stagnation occurs when **both** conditions are met:
1. The same issue has been the top actionable recommendation for 3+ consecutive evaluations
2. The average score has not improved by 0.5+ points across those evaluations

When stagnation is detected, note it explicitly and suggest that the `map-representation-plan` skill research alternative approaches before the next implementation cycle.

### Per-metric stagnation

If any individual metric has a delta of 0 (unchanged score) for 3+ consecutive evaluations, note this in the recommendations section as per-metric stagnation — even if the issue is not the top recommendation and the overall average is improving. Example: "Road/River Integration has been unchanged at 5 for 3 evaluations — consider focused attention in the next plan."

Per-metric stagnation does not automatically escalate the recommendation's severity, but it should be called out so the plan skill can prioritize it.

### Oscillation

If the Score Trend shows oscillation (alternating positive/negative deltas for 4+ evaluations), this suggests fixes are introducing regressions. Note this pattern and recommend a more cautious, incremental implementation approach.

## Report File Maintenance

When the report file exceeds ~500 lines or ~10 evaluations, archive older evaluations.

**What to keep in the active file**: The rubric/header (everything before the first `---` separator) and the last 5 evaluations.

**Archival procedure**:
1. Identify which evaluations to archive — keep the rubric + last 5 evaluations
2. Grep for `^## EVALUATION #` to find all evaluation line numbers
3. Find the line number of the `---` separator immediately before the oldest evaluation to keep
4. Read the content to be archived
5. Write or append to `reports/map-representation-evaluation-archive.md`:
   - If the archive file does not exist, create it with a header: `# Map Representation Evaluation — Archive`
   - If it already exists, append the new archived evaluations after the existing content
6. Archive evaluations **verbatim** — do not condense or summarize
7. Remove the archived evaluations from the active file
8. Verify: grep for `## EVALUATION #` in both files to confirm the correct split

## Graduation

If the average score reaches **8.0+** and no CRITICAL or HIGH recommendations remain, note in the evaluation that the map representation has graduated to acceptable quality. Further evaluations are optional — invoke only after significant rendering changes.

## Screenshot Expectations

Screenshots should capture the full visible map at default zoom level. If only a portion of the map is visible, note which region is shown and caveat that scores reflect the visible portion only. Both the game canvas and map editor screenshots should show the same (or overlapping) geographic area to enable cross-view comparison.

## Score Adjustment Policy for Coverage Changes

When expanded screenshot coverage reveals pre-existing issues not visible before, scores SHOULD be adjusted to reflect the full-map reality. Tag each adjusted score with `*(Comparability adjustment — not a code regression)*` in the justification. This prevents artificial inflation from partial visibility while preserving trend interpretability through explicit tagging.

Conversely, if screenshots are removed (e.g., a view is deprecated), do not inflate scores to compensate — note the reduced coverage and score only what is visible.

## Unchanged Rendering

If the rendering appears unchanged from the previous evaluation but the screenshot set changed, proceed with a full evaluation — the new screenshots may reveal previously invisible issues.

Before concluding "no change," explicitly check each metric against the most recent scored evaluation's description:

1. **Adjacency Clarity**: Are province borders/shared edges the same shape and style? Any new gaps or overlaps?
2. **Road/River Integration**: Do routes render the same way — same z-order (under vs. over territory fills), same line style, same termination points (edge vs. through-territory)?
3. **Terrain Distinction**: Are terrain colors, count of distinct colors, and any texture/pattern overlays the same?
4. **Label/Token Readability**: Are label sizes, background pill styles, token sizes, and token shapes the same?

Only if all 4 checks confirm no visible change should the no-change stub be used. If any metric's rendering differs, proceed with a full evaluation.

If both rendering AND screenshots are unchanged since the previous evaluation, append a brief stub instead of a full evaluation:

```markdown
---

## EVALUATION #N — No Change

**Date**: YYYY-MM-DD
**Screenshots analyzed**: [list]

Rendering and screenshot set unchanged since Eval #N-1. No new evaluation needed. Re-evaluate after the next implementation cycle.
```

## Visual Change Without Score Movement

If rendering changed visibly (e.g., shape quality, color palette, layout) but most metric scores remain unchanged, note this explicitly in the evaluation. For each unchanged metric, briefly explain why the visual change didn't affect that metric — e.g., "Shape quality improved (organic curves replaced angular polygons) but route rendering — measured by Road/River Integration — was not affected by the shape change since routes still terminate at shape edges." This prevents future evaluators from misinterpreting stable scores as "nothing happened."

## Coverage Metrics

When territory rendering progress is a key differentiator (as in FITL), include a coverage line after the scores table:

```markdown
**Territory coverage**: N/M province zones rendered as territories (vs. rectangles).
```

This gives a quantitative progress indicator beyond the subjective 1-10 scores and directly tracks progress on recommendations to extend territory treatment. Approximate counts from visual inspection are acceptable — use `~` prefix for estimates. If precision is needed, cross-reference the zone count from the visual config or GameDef.

Optionally, when Road/River Integration is a focus area, include a route integration line:

```markdown
**Route integration**: N/M visible route segments flow through territory (vs. edge-to-edge).
```

This is harder to count precisely from screenshots than territory coverage — use approximate counts with `~` prefix. Include when Road/River Integration changed by +/-2 or more, or when per-metric stagnation was previously flagged for this metric.

## Scope

This skill is scoped to the FITL game map (`screenshots/fitl-game-map*.png` and `screenshots/fitl-map-editor*.png`). The 4 metrics are specific to map territory rendering. For other evaluation needs (e.g., UI readability), use the appropriate evaluation skill.


----

---
name: map-representation-implement
description: Use when the latest map plan is ready and improvements need to be implemented. Reads reports/map-representation-plan.md and reports/map-representation-evaluation.md, then implements the planned changes to the runner canvas renderers and visual config.
---

# Map Representation Implementation

Improve the FITL game map rendering based on the latest plan's recommendations.

## Checklist

> **Plan mode note**: If plan mode is active when this skill is invoked, steps 1-3 serve as the exploration phase. During exploration, also identify the specific file paths from the plan's implementation steps and read them via Explore agents to front-load context for the plan file. Write your execution plan to the plan file, exit plan mode, then continue with steps 4-13.

1. Read `reports/map-representation-evaluation.md` — focus on the latest EVALUATION #N for context on what needs improving.
2. Read `reports/map-representation-plan.md` — the implementation plan to execute. This is the primary guide for this session.
3. Read `docs/FOUNDATIONS.md` — verify alignment before writing any code. Pay special attention to:
   - **Foundation #3** (Visual Separation): All changes in runner/visual-config, never in engine or GameSpecDoc
   - **Foundation #7** (Immutability): State transitions return new objects, no mutation
   - **Foundation #9** (No Backwards Compatibility): No shims or deprecated fallbacks
   - **Foundation #10** (Architectural Completeness): Complete solutions, not patches
4. Collect the unique file paths (source files, config files, and test files with golden assertions) from all Implementation Steps in the plan. Read them in parallel (batch) to front-load context before starting edits. If the plan is data-only (e.g., visual-config.yaml vertex authoring), read the target data file(s) instead. If the plan doesn't list test files (common for constants-only iterations), search for the old literal values being changed (e.g., grep for `28` when changing `DEFAULT_TOKEN_SIZE = 28`) across `packages/runner/test/` to pre-identify test files that may need updating.
5. Follow the plan's implementation steps **in order**, respecting noted dependencies.
6. If vertices were authored or modified, verify shared borders: for each adjacent pair, confirm that converting relative vertices back to absolute world coordinates (`absoluteX = relativeX + centerX`) produces matching points on both sides of the shared edge.
7. If a step is ambiguous or you discover the plan's assumptions about the code are wrong, apply the **1-3-1 rule** (1 problem, 3 options, 1 recommendation) before proceeding — per Foundation #10.
8. If the plan includes map editor changes, implement those too.
9. **Pre-flight test impact analysis**: For each changed constant or default value, grep `packages/runner/test/` for the old literal value. Classify each hit as:
   - **Golden assertion** (derives from production default or real FITL YAML) → must update to match the new value.
   - **Independent fixture** (arbitrary test data that happens to use the same number) → leave unchanged.
   Key judgment: if a test loads the real FITL YAML (`loadVisualConfig('data/games/fire-in-the-lake/visual-config.yaml')`) or asserts against a resolved default with no explicit override, it's a golden assertion. If a test constructs its own fixture data (e.g., `{ size: 28 }` in a mock or `{ laneGap: 24 }` in a hand-built config), it's an independent fixture.
10. Update golden test assertions based on the impact analysis from Step 9. For schema/rendering changes, also check: `visual-config-files.test.ts` (attribute rules, colors, override counts), `layers.test.ts` (z-order indices if layer order changed), and `connection-route-renderer.test.ts` (route geometry expectations if route constants changed).
11. Run verification: `pnpm turbo typecheck` and `pnpm -F @ludoforge/runner test`.
12. Visual verification: Run `pnpm -F @ludoforge/runner dev` and inspect the map in the browser. Verify: all targeted zones render with the new shapes, terrain colors apply correctly, tokens render inside polygon bounds, adjacency lines connect to polygon edges, and the map editor shows the same changes. Report any visual anomalies to the user before concluding.
13. Do NOT update either report file — that happens in the next evaluate invocation.

## Key Files

### Frequently Modified

| File | What It Controls |
|------|-----------------|
| `packages/runner/src/canvas/layers.ts` | Layer z-order hierarchy — controls rendering order of background, regions, adjacency, zones, routes, and overlays |
| `packages/runner/src/canvas/renderers/zone-renderer.ts` | Game canvas zone rendering — shape, fill, stroke, labels, badges, hidden stack visual |
| `packages/runner/src/canvas/renderers/shape-utils.ts` | Shape drawing primitives — `drawZoneShape()` dispatches shapes, `getEdgePointAtAngle()` computes edge intersections |
| `packages/runner/src/canvas/renderers/adjacency-renderer.ts` | Adjacency line rendering — dashed segments between zone edges, highlighting |
| `packages/runner/src/canvas/renderers/region-boundary-renderer.ts` | Region boundary rendering — convex hull, label alpha, border styles |
| `packages/runner/src/config/visual-config-types.ts` | Zod schemas for visual config — must update when extending the config contract |
| `packages/runner/src/config/visual-config-defaults.ts` | `ZoneShape` type union, default dimensions, faction palette |
| `packages/runner/src/config/visual-config-provider.ts` | `ResolvedZoneVisual` interface, `resolveZoneVisual()` cascade, `applyZoneStyle()` |
| `data/games/fire-in-the-lake/visual-config.yaml` | FITL visual configuration — zone shapes, positions, colors, routes |
| `packages/runner/src/canvas/renderers/connection-route-renderer.ts` | Road/river route rendering — Bezier curves, wave effects, stroke styles, route endpoint geometry |
| `packages/runner/src/presentation/presentation-scene.ts` | Presentation layer — resolves zone render specs (label positioning, fill color, stroke, badges) from visual config + interaction state |
| `packages/runner/src/canvas/text/bitmap-font-registry.ts` | Bitmap font installation — base font size and resolution for all BitmapText labels |
| `packages/runner/src/map-editor/map-editor-zone-renderer.ts` | Map editor zone rendering (if plan requires editor changes) |

### Reference Only

| File | What It Controls |
|------|-----------------|
| `packages/runner/src/canvas/geometry/dashed-segments.ts` | Dashed line segment algorithm — `buildDashedSegments()` |
| `packages/runner/src/canvas/renderers/stroke-dashed-segments.ts` | Rendering dashed segments to PixiJS Graphics |
| `packages/runner/src/config/visual-config-loader.ts` | Loads and parses visual config YAML |
| `packages/runner/src/layout/world-layout-model.ts` | Layout model types — `ZonePositionMap`, zone dimensions |
| `packages/runner/src/map-editor/map-editor-adjacency-renderer.ts` | Map editor adjacency lines (if plan requires editor changes) |

## Key Test Files

| File | What It Covers |
|------|---------------|
| `packages/runner/test/canvas/layers.test.ts` | Layer z-order golden assertions — `boardGroup.children` indices must be updated when layer order changes |
| `packages/runner/test/canvas/renderers/zone-renderer.test.ts` | Zone container children by index, label stroke width/fill, shape drawing assertions |
| `packages/runner/test/canvas/renderers/token-renderer.test.ts` | Token positioning, hitArea radius, lane centering offsets. **Warning**: `createColorProvider()` mock has hardcoded `size: 28` fallback (line ~348) that shadows `DEFAULT_TOKEN_SIZE` — must be updated when default changes |
| `packages/runner/test/canvas/renderers/token-render-style-provider.test.ts` | Default token visual assertions (`size`, `shape`) — tests both `DefaultTokenRenderStyleProvider` and `VisualConfigTokenRenderStyleProvider` |
| `packages/runner/test/canvas/renderers/connection-route-renderer.test.ts` | Route geometry expectations — midpoint coordinates, label rotation, stroke width |
| `packages/runner/test/canvas/text/bitmap-font-registry.test.ts` | Master bitmap font size and stroke width assertions |
| `packages/runner/test/config/visual-config-files.test.ts` | **Golden assertions** on FITL visual-config.yaml structure and values — must be updated whenever YAML attribute rules, colors, lane spacing, or override counts change |
| `packages/runner/test/config/visual-config-provider.test.ts` | Default token size resolution, real FITL lane layout assertions (loads actual YAML), zone token layout resolution |
| `packages/runner/test/presentation/presentation-scene.test.ts` | Token grouping y-offsets (affected by token size changes), label line height |

## Architecture Context

### Zone Shape Drawing

The `drawZoneShape()` function in `shape-utils.ts` is the central shape dispatcher. It receives a `Graphics` object, dimensions, and a shape type string, then draws the appropriate shape. See the `ZoneShape` type union in `visual-config-defaults.ts` for the full list of supported shapes.

To add a new shape type (e.g., `polygon` with arbitrary vertices):
1. Add the shape name to the shape type union in `visual-config-types.ts`
2. Add a case in `drawZoneShape()` in `shape-utils.ts`
3. Update `visual-config.yaml` zone entries to use the new shape
4. Ensure the adjacency renderer can compute edge intersection points for the new shape

### Vertex Smoothing

`smoothPolygonVertices()` in `shape-utils.ts` applies Chaikin's corner-cutting algorithm (2 iterations by default) to all polygon vertices. It is called in both `drawZoneShape()` (for rendering) and `getEdgePointAtAngle()` (for adjacency line edge intersection). This ensures the drawn shape and the computed edge attachment points always match. The function is a pure transform: `readonly number[] → number[]`. It preserves shared-edge alignment between adjacent polygons because Chaikin's is a local operation — each output vertex depends only on two adjacent input vertices, so the same edge in two polygons produces identical smoothed points independently.

### Adjacency Edge Computation

Adjacency lines connect from edge point to edge point, not center to center. The edge point calculation is shape-specific — it finds the intersection of the line from center-to-center with the shape boundary. When adding a new shape, you must also update the edge intersection logic or the adjacency lines will connect to the wrong points.

### Connection Route Rendering

Routes (roads, rivers) use Bezier curves with configurable geometry. Route geometry is defined in `visual-config.yaml` via `connectionRoutes` entries with `points` arrays (zone or anchor endpoints) and `segments` arrays (straight or quadratic). The resolver (`connection-route-resolver.ts`) computes absolute positions from zone centers and configured anchors; the renderer (`connection-route-renderer.ts`) consumes pre-resolved paths. Route endpoints can be extended past polygon edges via `extendRouteEndpoints()` to create the impression of routes flowing through territory. When province shapes change, route anchor positions may need repositioning.

### Game Canvas vs Map Editor

Both flows reuse `drawZoneShape()` from `shape-utils.ts`. The game canvas adds labels, badges, selection highlighting, and token rendering on top. The map editor adds drag handles and selection highlighting. A change to `drawZoneShape()` affects both flows — verify both after changes. Label font size constants are NOT shared — `zone-renderer.ts` and `map-editor-zone-renderer.ts` each have their own. Check both when the plan modifies label sizing. The bitmap font master size in `bitmap-font-registry.ts` IS shared by both flows via `installLabelBitmapFonts()` — if runtime font sizes increase, the master size must be at least as large as the largest runtime size to avoid blurry upscaling.

### Interaction vs. Config Stroke Resolution

The zone renderer receives two stroke color sources that must coexist:

- **Interaction stroke** (`zone.render.stroke`): Set by `resolveZoneStroke()` in `presentation-scene.ts`. Values: highlight (yellow), selectable (blue), or default (`#111827`, width 1, alpha 0.7). This is driven by game interaction state.
- **Config stroke** (`zone.visual.strokeColor`): Set by `resolveZoneVisual()` in `visual-config-provider.ts`. Comes from terrain attribute rules or per-zone overrides. This is purely visual config data.

The zone renderer uses a `DEFAULT_STROKE_SIGNATURE` pattern to detect whether an interaction stroke is active: if the render stroke matches `{ color: '#111827', width: 1, alpha: 0.7 }` exactly, it's the default and the visual config's `strokeColor` takes precedence. Otherwise, the interaction stroke wins. The map editor uses a simpler pattern: `isSelected ? SELECTED_STROKE_COLOR : visual.strokeColor ?? DEFAULT_STROKE_COLOR`.

### Polygon Vertex Design

When defining polygon vertices for province shapes:

1. **Coordinate system**: Vertices are relative to the zone's center `(0, 0)`. The zone container is positioned at the zone's world `(x, y)` coordinates. Vertices use the flat alternating format `[x1, y1, x2, y2, ...]` that `Graphics.poly()` expects.
2. **Shared borders**: Adjacent provinces must share identical border coordinates. To achieve this:
   - Define shared boundary points in **absolute world coordinates** first (e.g., the triple-point where three provinces meet).
   - Convert to zone-relative coordinates by subtracting each zone's center position: `relativeX = worldX - zoneCenterX`, `relativeY = worldY - zoneCenterY`.
   - In adjacent polygon vertex lists, the shared segment appears in **opposite winding order** (province A has points P1→P2, province B has P2→P1).
3. **Verification**: After computing vertices, verify all shared borders by converting back to world coords and confirming the same absolute segment appears in both polygons.
4. **External boundaries**: Non-shared edges (outer boundaries) can be placed freely to create a reasonable territory shape.

### Batch Vertex Authoring

When a plan requires authoring polygon vertices for many zones (10+), follow this workflow:

1. **Compute all midpoints first**: For every adjacent province pair, compute `midpoint = ((A.x + B.x) / 2, (A.y + B.y) / 2)` in absolute world coordinates. Build a lookup table.
2. **Identify 3-way junctions**: Where 3 provinces meet, compute the centroid of the 3 centers: `junction = ((A.x + B.x + C.x) / 3, (A.y + B.y + C.y) / 3)`.
3. **Author in geographic groups**: Work outward from existing polygons or from one end of the map. This maintains spatial coherence and makes shared-border alignment easier to verify.
4. **Spot-check after each group**: After authoring a group, pick 2-3 shared borders and manually verify that the absolute world coordinates match on both sides (`absolute = relative + center`).
5. **Round all coordinates to integers**: Avoids floating-point alignment drift between adjacent polygons.

### Tooling

For iterations that require authoring polygon vertices for many zones, consider writing a temporary Node.js script that reads zone center positions and adjacency data from visual-config.yaml / GameSpecDoc, computes midpoints and junction points, and outputs vertex arrays in YAML format. Delete the script after use per workspace hygiene rules.

## Extending Visual Config

When the plan requires new config fields (e.g., polygon vertex data, terrain texture settings), follow these steps in order, **skipping any that don't apply** to your field type:

1. **Schema** (`visual-config-types.ts`): Add the field to the relevant Zod schema (e.g., `ZoneVisualStyleSchema`). Use `.optional()` for new fields to maintain backward compatibility with other games.
2. **Type union** (`visual-config-defaults.ts`): *(Only if adding a new enum/shape value.)* Add it to the `ZoneShape` (or equivalent) type union here. The type union and the Zod enum must stay in sync. Skip this step for plain string/number fields.
3. **Interface** (`visual-config-provider.ts`): Add the field to `ResolvedZoneVisual` (or the relevant resolved interface). This is the contract that renderers consume.
4. **Cascade** (`visual-config-provider.ts`): Thread the field through `resolveZoneVisual()` (initialize with a default) and `applyZoneStyle()` (copy from source when present). This is the style-merge pipeline that applies category → attribute rules → overrides.
5. **Consumer** (renderer files): Use the new field where needed — pass it to drawing functions, edge calculations, hit areas, etc.
6. **Game config** (`data/games/fire-in-the-lake/visual-config.yaml`): Add the actual values.

**Test breakage warning**: Adding a required field to `ResolvedZoneVisual` breaks ~37 literal constructions across ~17 test files. To bulk-fix: search for `vertices: null` (the last field before the new one) and append `, newField: null`. **Watch for two patterns**: inline (`connectionStyleKey: null, vertices: null`) and multi-line (`vertices: null,` on its own line followed by `}`). Running both sed and replace_all risks double-insertion — verify with `grep 'newField: null, newField: null'` afterward and deduplicate any hits.

## Common Pitfalls

- **Edge intersection for new shapes**: If you add polygon-based provinces, the adjacency renderer's edge point calculation must handle arbitrary polygons. Without this, adjacency lines will connect to wrong points or pass through the shape interior.
- **Token positioning**: Tokens are positioned relative to zone center and dimensions. If province shapes change from rectangles to irregular polygons, ensure tokens still render inside the shape. Token layout may need a bounding-box or centroid-based approach.
- **Label positioning**: Zone labels are positioned in `presentation-scene.ts:resolveZoneRenderSpec()`. Non-circle shapes place labels at `y: 0` (inside the zone); circles place labels below at `y: bottomEdge + LABEL_GAP`. A semi-transparent background pill in `zone-renderer.ts` ensures contrast on all terrain colors. If shapes become concave polygons, the geometric center may not be inside the shape — consider a point-in-polygon check.
- **Map editor sync**: `drawZoneShape()` is shared, but the map editor has its own stroke colors and interaction handlers. Test both flows after shape changes.
- **Visual config backward compatibility**: Other games (Texas Hold'em) also use visual-config. New schema fields must be optional so other games don't break. Test with `pnpm turbo typecheck` to catch schema issues.
- **PixiJS Graphics API**: PixiJS 8 uses `Graphics.poly(points)` for arbitrary polygons where `points` is a flat array `[x1,y1, x2,y2, ...]`. Ensure the polygon is closed (first point = last point) or use `closePath()`.
- **TypeScript exactOptionalPropertyTypes**: This project enables `exactOptionalPropertyTypes`. When adding optional fields that receive `foo ?? undefined`, the type must include `| undefined` explicitly. E.g., `readonly vertices?: readonly number[] | undefined`, not just `readonly vertices?: readonly number[]`.
- **Vertex transforms affect edge intersection tests**: If smoothing or other vertex transformations are applied to `drawZoneShape()`, they must also be applied in `getEdgePointAtAngle()`, AND existing polygon edge intersection tests will need updated expectations since the shape boundary changes. The `smoothPolygonVertices()` function rounds corners inward, so edge intersection points move closer to center.
- **Route overlap margin affects geometry tests**: Changing `ROUTE_OVERLAP_MARGIN` in `connection-route-renderer.ts` extends route endpoints, which shifts the sampled midpoint position and tangent direction. This breaks assertions on midpoint coordinates and label rotation in `connection-route-renderer.test.ts`. The rotation normalization can produce values near 2π (equivalent to 0) — test assertions must handle modular equivalence.
- **Test mock default token size**: `token-renderer.test.ts` has a `createColorProvider()` mock with a hardcoded `size: 28` fallback (not imported from `DEFAULT_TOKEN_SIZE`). When changing the default token size, this mock must also be updated or hitArea/positioning assertions will fail with non-obvious errors — the mock still returns the old size while the test expects dimensions based on the new default.
- **Zone renderer child ordering**: `zone-renderer.test.ts` accesses zone container children by numeric index (`children[0]` = base, `children[1]` = hiddenStack, etc.). Adding or reordering children in `createZoneVisualElements()` / `addChild()` shifts all subsequent indices. After modifying the child list, update indices in the test using Python or manual edits — do **not** use sequential sed replacements (e.g., `[2]→[3]` then `[3]→[4]`) as this causes double-shifting. Process from highest index to lowest, or use a script that replaces all in one pass. Also update any `toHaveLength(N)` assertions on `container.children`.

## Scope Constraints

- Do not modify engine code (`packages/engine/`) — Foundation #3 (Visual Separation)
- Do not change game logic or GameSpecDoc YAML — Foundation #1 (Engine Agnosticism)
- All rendering changes must be in runner source (`packages/runner/src/`) or visual config (`data/games/*/visual-config.yaml`)
- Follow the plan's implementation steps — don't scope-creep beyond what was planned
- If you discover the plan is wrong or incomplete, apply the 1-3-1 rule rather than improvising
- The proposed changes should align with `docs/FOUNDATIONS.md`


----

---
name: map-representation-plan
description: Use when the latest map evaluation is ready and a plan for improvements is needed. Reads the most recent EVALUATION from reports/map-representation-evaluation.md, researches rendering techniques, brainstorms solutions, and produces a concrete implementation plan in reports/map-representation-plan.md.
---

# Map Representation Planning

Read the latest evaluation, research rendering approaches, and produce a concrete implementation plan for the next improvement iteration.

**This skill produces `reports/map-representation-plan.md` as its sole artifact.** If invoked within plan mode, the plan mode file is a working scratchpad — the report file is the deliverable. Do not proceed to implementation after writing the report — the `map-representation-implement` skill consumes this plan in a separate invocation.

## Checklist

1. Read `reports/map-representation-evaluation.md` — focus on the latest EVALUATION #N. Note the scores, CRITICAL/HIGH recommendations, and any recurring or stagnating issues. Determine the iteration number: `max(latest_evaluation_number, latest_plan_iteration_number) + 1` (counting "No Change" stubs as evaluations for numbering purposes). If the previous plan file exists and its iteration number equals `latest_evaluation_number + 1` (i.e., the slot is already taken by an implemented plan), use `latest_plan_iteration_number + 1` and note the numbering context in the Context section. Gaps between evaluation and plan numbers are acceptable when evaluations are added without corresponding plans, or when a "No Change" stub is corrected after a plan was already created and implemented. **Large file handling**: If the file exceeds read limits, use `offset` to read from the end — evaluations are appended chronologically, so the latest is always at the bottom. A grep for `## EVALUATION #` can identify where each evaluation starts. Read the latest evaluation in full plus the Score Trend table (if present) for historical context.
2. Read `docs/FOUNDATIONS.md` — **all proposals must align** with these principles. Pay special attention to:
   - **Foundation #1** (Engine Agnosticism): No game-specific logic in engine code
   - **Foundation #3** (Visual Separation): All visual changes in visual-config.yaml or runner code, never in GameSpecDoc or engine
   - **Foundation #7** (Immutability): State transitions return new objects
   - **Foundation #9** (No Backwards Compatibility): No shims or deprecated fallbacks
   - **Foundation #10** (Architectural Completeness): Solutions address root causes, not symptoms
3. Identify the CRITICAL and HIGH recommendations from the evaluation. If none exist, target the top 2-3 MEDIUM recommendations.
4. **Stalled iteration check**: If the previous evaluation shows no progress since the evaluation before it, check whether the previous plan was implemented. If not, decide whether to carry forward its recommendations (if still valid), supersede them (if priorities shifted), or incorporate them into the new plan. Note the decision in the Context section. Also review the previous plan's Deferred Items section (if present) and carry forward any items that are still relevant into the new plan's Deferred Items table.
   - **Implemented but not re-evaluated**: If the previous evaluation says "No Change" or its screenshots predate the implementation (verify by grepping for the specific changes listed in the previous plan's Implementation Verification Checklist — e.g., constant values, layer order — rather than re-reading entire files), note this in the Context section and proceed to the next priority tier. Use the previous plan's Implementation Verification Checklist as a batch grep/read target — if all items check out, note "Iteration N fully implemented, awaiting re-evaluation" and move on. The next evaluation cycle will capture the implemented changes alongside this iteration's changes.
   - **Implemented but ineffective**: If the previous plan was implemented but targeted metrics did not improve, diagnose the root cause before proposing the next change. Common causes: insufficient magnitude (e.g., color contrast too low — compute RGB Euclidean distance, target >80 for reliable distinction), wrong lever (e.g., static font size increase when the problem is zoom-dependent), or evaluator visibility (e.g., screenshot coverage doesn't capture the change). Note the diagnosis in the Context section, then decide whether to supersede the approach (different strategy) or amplify it (same strategy, stronger parameters).
5. Read the renderer source files relevant to the identified problems (see Key Files). Extract: key type definitions with line numbers, function signatures that will be modified, data flow from config through presentation to renderer. Scope the exploration to the specific problems identified in step 3 — do not request a full pipeline analysis of subsystems that are not targeted this iteration (e.g., if routes are deferred, don't explore the route renderer). Use Explore sub-agents for parallel codebase exploration only when the investigation requires open-ended search across unknown files. When the Key Files table identifies the relevant files, prefer direct Read/Grep calls for efficiency. The goal is to populate the "Current Code Architecture" section of the plan output.
   - **Data-only fast-path**: For iterations that only add values to existing YAML config fields (e.g., adding `color`/`strokeColor` to per-zone overrides, adjusting lane spacing), skip renderer source files. Instead, verify the relevant Zod schema (`visual-config-types.ts`) and provider resolution pipeline (`visual-config-provider.ts`) support the fields being used. The architecture section should document this pipeline to confirm no code changes are needed, rather than focusing on renderer functions.
   - **New config patterns**: If proposing attribute rules or config patterns not already present in visual-config.yaml, verify the Zod schema and provider matching logic support them before recommending them. Note: `attributeContainsValue()` (in `visual-config-provider.ts`) only matches string and string-array attribute values. Boolean attributes (e.g., `coastal: true`) cannot be matched via `attributeContains` — they return `false` unconditionally. Use per-zone overrides instead for zone-specific visual distinctions that can't be expressed via string attributes.
   - **Hybrid iterations** (code + data changes): Include both the architecture section (for code changes) and the reference data section (for data changes). Implementation steps should clearly separate code steps from data-authoring steps.
6. Optionally read the game's physical reference image (e.g., `screenshots/FITL_SC1.jpg`) for design inspiration when planning visual changes. Use it as a target aesthetic, not a rigid specification.
7. **Research phase** (if needed): If the identified problems require techniques not already present in the codebase, use Tavily web search and/or Context7 to research rendering techniques. Skip external research when the solution extends existing patterns — if skipped, note in the Research Sources section why it was unnecessary (e.g., "All solutions extend existing PixiJS Graphics and BitmapText patterns already in the codebase"). Examples of research topics:
   - Voronoi tessellation / Delaunay triangulation in PixiJS or 2D canvas
   - Polygon-based territory rendering in strategy games
   - Procedural map border generation algorithms
   - Terrain coloring and texture techniques in 2D renderers
   - Route rendering through irregular polygons
   - PixiJS Graphics polygon drawing, mesh rendering, or shader approaches
   - How other digital COIN-series implementations render maps
8. For the top 1-3 problems (prioritize CRITICAL items — a single large CRITICAL may consume the entire iteration), brainstorm **2-3 solution approaches** each, with trade-offs:
   - Feasibility (how much code change, how many files)
   - Visual impact (how much does it improve the metric)
   - Risk (what could break, what regressions are possible)
   - Foundation alignment (does it respect all relevant principles)
9. Select the recommended approach for each problem, applying the **1-3-1 rule**: 1 clearly defined problem, 3 potential options, 1 recommendation. If the best recommendation combines elements of multiple approaches, present it as a hybrid with clear attribution (e.g., "Approach 1 + partial Approach 2"). Explain which elements are taken from each and why the combination is better than either alone.
10. **Map editor scope assessment**: For each proposed change, assess whether the map editor (`packages/runner/src/map-editor/`) needs updating in this iteration:
   - If the change is purely rendering (e.g., drawing polygons instead of rectangles from the same position data), the editor may just need to call the same drawing function — include it.
   - If the change requires new editor interaction patterns (e.g., vertex dragging for polygons), defer to a future iteration — note what's deferred and why.
11. Write the new plan to `reports/map-representation-plan.md` (overwrites any existing file). The plan is **overwritten** each iteration, not appended.
12. **Stop.** This skill's sole output is `reports/map-representation-plan.md`. Do not proceed to implementation — the `map-representation-implement` skill consumes this plan in a separate invocation.

## Plan Output Format

Write `reports/map-representation-plan.md` with this structure:

```markdown
# Map Representation Plan — Iteration N

**Date**: YYYY-MM-DD
**Based on**: EVALUATION #N (average score: X.X)
[If the latest evaluation is a "No Change" stub with no scores, reference the last evaluation
with actual scores: `EVALUATION #N (no change; effective scores from EVALUATION #M, average: X.X)`]
**Problems targeted**: [list of CRITICAL/HIGH/MEDIUM items addressed]

## Context

[1-3 sentences: why this change is needed, what prompted it, and the intended outcome]

## Deferred Items

Track items explicitly deferred from previous iterations to prevent silent drops.

| Item | First recommended | Deferred since | Target iteration |
|------|-------------------|---------------|-----------------|
| [description] | Eval #N | Iteration M | [N+1 or "no target yet"] |

If no items are deferred, write: "No deferred items."

If any items from the previous plan are being **superseded** (replaced with a different approach
rather than carried forward unchanged), note them in the Context section with the reason for
supersession (e.g., "Iteration 5's Laos color #2d5a3a superseded — RGB distance 25 from base
jungle was insufficient; replaced with #6b8f7b at distance 120"). This helps the implementer
understand why previously-implemented work is being replaced.

## Foundations Alignment

| Foundation | Relevance | How This Plan Respects It |
|-----------|-----------|--------------------------|
| #1 Engine Agnosticism | [relevant/not relevant] | [brief explanation] |
| #3 Visual Separation | Always relevant | [how changes stay in runner/visual-config] |
| #7 Immutability | [relevant/not relevant] | [brief explanation] |
| #9 No Backwards Compat | [relevant/not relevant] | [brief explanation] |
| #10 Architectural Completeness | Always relevant | [root cause vs symptom] |

For data-only iterations (no code changes), the Foundations Alignment table may be replaced with a single line: "All changes are visual-config.yaml data — no foundation concerns."

## Current Code Architecture (reference for implementer)

Document the exact interfaces, function signatures, and data flow relevant to the
problems targeted. This section must make the plan self-sufficient — an implementer
reading only this file should not need to re-explore the codebase.

Include:
- Key type/interface definitions with file paths and line numbers
- Function signatures that will be modified
- Data flow from config → presentation → renderer
- Coordinate systems and conventions the implementer must follow
- Current code snippets showing what will change (before state)
- Schema inheritance relationships (e.g., override schemas extending base schemas)

## Reference Data (optional — for iterations with data authoring)

Include reference tables the implementer needs to author data correctly: province lists,
color palettes, adjacency maps, coordinate positions, terrain assignments, design constraints.
Include this section for any iteration that involves data authoring in visual-config.yaml,
regardless of whether the iteration also includes code changes. Omit only for pure code iterations.

**Data change validation heuristics**: When a previous iteration's data changes failed to move
the metric, quantify the gap before proposing new values:
- **Colors**: Compute RGB Euclidean distance (√((R₁-R₂)² + (G₁-G₂)² + (B₁-B₂)²)). Target >80 for cross-category distinction (colors that must never be confused, e.g., highland vs. jungle). For within-category shade variation (e.g., coastal highland vs. inland highland), 40-60 is acceptable — the purpose is subtle geographic differentiation, not categorical separation. Values within 30 are indistinguishable at overview zoom on dark backgrounds. Include distances and the target range justification in the Reference Data table.
- **Sizes**: Compare against the smallest zone dimensions to ensure visibility at 1:1 scale. Font sizes below 16px are unreadable at overview zoom.
- **Spacing/margins**: Verify proportionality against average zone size (e.g., a 35px overlap margin is ~10% of a 360px zone — may be too subtle).

## Problem 1: [Problem title from evaluation]

**Evaluation score**: Metric X = Y/10
**Root cause**: [Why this problem exists in the current rendering code]

### Approaches Considered

1. **[Approach A]**: [description]
   - Feasibility: [LOW/MEDIUM/HIGH]
   - Visual impact: [LOW/MEDIUM/HIGH]
   - Risk: [description of what could break]

2. **[Approach B]**: [description]
   - Feasibility: [LOW/MEDIUM/HIGH]
   - Visual impact: [LOW/MEDIUM/HIGH]
   - Risk: [description]

3. **[Approach C]**: [description]
   - Feasibility: [LOW/MEDIUM/HIGH]
   - Visual impact: [LOW/MEDIUM/HIGH]
   - Risk: [description]

### Recommendation: [Approach X]

**Why**: [reasoning]

[Repeat for Problem 2, 3...]

## Implementation Steps

Ordered steps with dependencies noted. If all steps target the same file, state the file
once here and use the steps to describe logical groupings (e.g., by geographic cluster or
component subsystem).

1. [Step description] — **File**: `path/to/file.ts` — **Depends on**: none
2. [Step description] — **File**: `path/to/file.ts` — **Depends on**: Step 1
...

## Map Editor Scope

**Included in this iteration**:
- [List of editor changes included, if any]

**Deferred to future iteration**:
- [List of editor changes deferred, with reasoning]

## Visual Config Changes

[If visual-config.yaml or visual-config-types.ts schema changes are needed, list them explicitly]

## Verification

1. `pnpm turbo typecheck` — must pass
2. `pnpm -F @ludoforge/runner test` — must pass
3. Visual check: [what to look for when running the game after implementation]

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| [risk description] | LOW/MEDIUM/HIGH | [what breaks] | [how to prevent or recover] |

## Implementation Verification Checklist

Machine-readable list of specific changes for the evaluator to cross-reference against the
codebase (independent of visual assessment). Helps diagnose "implemented but metric unchanged"
in the next evaluation cycle.

- [ ] `<file>`: <what changed> (e.g., "`layers.ts`: route layer moved above zone layer")
- [ ] `<file>`: <what changed>

## Test Impact (optional)

If the plan modifies visual-config.yaml values that are golden-asserted in tests (attribute rules,
colors, override structure, lane spacing), note which test files are likely affected. Cross-reference
the implement skill's Key Test Files table. For data-only YAML changes, a single line suffices, e.g.:
"Expected test impact: `visual-config-files.test.ts` may assert override structure or color values."

Omit this section for pure code iterations where test impact is obvious from the modified files.

## Research Sources

- [URL or description of research that informed the plan]
```

## Key Files

| File | What It Controls |
|------|-----------------|
| `packages/runner/src/canvas/layers.ts` | Canvas layer z-ordering (rendering order of zones, routes, adjacency, regions) |
| `packages/runner/src/canvas/renderers/zone-renderer.ts` | Game canvas zone rendering (shape, fill, stroke, labels, badges) |
| `packages/runner/src/canvas/renderers/shape-utils.ts` | Shape drawing primitives (`drawZoneShape()` — rectangle, circle, polygon, etc.) |
| `packages/runner/src/canvas/renderers/adjacency-renderer.ts` | Adjacency line rendering (dashed segments between zone edges) |
| `packages/runner/src/canvas/renderers/connection-route-renderer.ts` | Road/river route rendering (Bezier curves, wave effects) |
| `packages/runner/src/canvas/geometry/dashed-segments.ts` | Dashed line algorithm |
| `packages/runner/src/canvas/renderers/stroke-dashed-segments.ts` | Rendering dashed segments to PixiJS Graphics |
| `packages/runner/src/config/visual-config-types.ts` | Zod schemas for visual config (zone shapes, stroke styles) |
| `packages/runner/src/config/visual-config-defaults.ts` | ZoneShape type union, default dimensions, default token size |
| `packages/runner/src/config/visual-config-provider.ts` | Visual config accessor methods |
| `packages/runner/src/canvas/renderers/region-boundary-renderer.ts` | Region boundary rendering (convex hull, labels, watermark alpha) |
| `packages/runner/src/layout/world-layout-model.ts` | Layout model types (zone positions) |
| `data/games/fire-in-the-lake/visual-config.yaml` | FITL-specific visual configuration |
| `packages/runner/src/presentation/presentation-scene.ts` | Label positioning, zone render spec construction, fill color resolution |
| `packages/runner/src/canvas/renderers/zone-presentation-visuals.ts` | Marker labels, badge visuals |
| `packages/runner/src/canvas/text/bitmap-font-registry.ts` | Bitmap font installation and configuration |
| `packages/runner/src/canvas/renderers/token-renderer.ts` | Token rendering, sizing, and positioning |
| `packages/runner/src/canvas/renderers/token-shape-drawer.ts` | Token shape drawing functions (circle, square, triangle, etc.) |
| `packages/runner/src/presentation/token-presentation.ts` | Token render spec resolution (dimensions, radius, symbol sizing, stroke states) |
| `packages/runner/src/map-editor/map-editor-zone-renderer.ts` | Map editor zone rendering |
| `packages/runner/src/map-editor/map-editor-adjacency-renderer.ts` | Map editor adjacency lines |

## Research Guidelines

When using Tavily or Context7:
- Search for **PixiJS-specific** techniques first (the renderer uses PixiJS 8)
- Look for **strategy game map rendering** examples and open-source implementations
- Check for **lightweight libraries** that could provide Voronoi/Delaunay without heavy dependencies
- Prefer solutions that work with **Graphics primitives** (polygon, path) over shader-based approaches for maintainability
- Note the **license** of any library considered — the project is GPL-3.0

## Vertex Design Guidelines

When the plan proposes custom polygon shapes for zones:

- **Coordinate system**: Vertices are relative to zone center `(0, 0)`, matching how all existing shapes draw. The zone container is positioned at the zone's world `(x, y)` coordinates.
- **Format**: Flat alternating `[x1, y1, x2, y2, ...]` array matching `Graphics.poly()` input.
- **Shared borders**: Adjacent zones must share border edge coordinates (same vertex pair in reverse order) so territories tile without gaps.
- **Vertex count**: 5-8 vertices per zone is reasonable for territory shapes. More adds visual fidelity but increases YAML verbosity.
- **Coverage strategy**: If the rendering pipeline is already validated (polygons render correctly for existing zones), author vertices for all remaining zones in a single iteration to avoid a jarring visual split between polygon and rectangle provinces. Group zones by geographic cluster for authoring order but include all in the same plan. Only use an incremental approach (4-5 zones at a time) when validating a new rendering technique for the first time.
- **Size reference**: Current default province rectangles are 360x220. Polygon vertices should produce shapes of comparable area.
- **Validation**: For each adjacent province pair sharing vertices, verify that absolute coordinates match: `centerA + relativeVertexA == centerB + relativeVertexB`. Include this validation as a step in the Verification section of the plan.

## Scope Constraints

- Do not propose engine code changes (`packages/engine/`) — Foundation #3
- Do not propose GameSpecDoc YAML changes — Foundation #1
- All rendering changes must be in runner source or visual-config
- Focus on the evaluation's top 2-3 recommendations — don't scope-creep
- If a proposed change is too large for one iteration, split it and note what's deferred
