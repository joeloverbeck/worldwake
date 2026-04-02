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

---

## EVALUATION #1

**Date**: 2026-04-02
**Scenario**: scenarios/cli-evaluation.ron
**Transcript**: reports/cli-evaluation-transcripts/eval-1.txt

### Session Notes

Launched the CLI with the 5-place, 4-agent evaluation scenario. Explored all 4 workflow sequences (Explore, Act, Control, Debug). The CLI is functional — commands work, the REPL loop is responsive, save/load is solid, and `needs` has genuinely good formatting with progress bars and urgency bands. However, two pervasive issues dominate the experience: (1) raw internal IDs (`Place#0`, `ItemLot#9`, `Facility#16`) leak through nearly every command, and (2) complex components dump raw `{:?}` debug format producing walls of `Permille(500)` text. The event detail view (`event 0`) is particularly extreme — 200+ lines of raw debug output for a single event. Multi-word entity names are broken across most commands due to Clap argument parsing. The action list is overwhelming (35 items post-tick) with internal operations like `store_stock` and `stage_stock_for_sale` that don't belong in a user-facing menu.

### Per-Command Analysis

**world**: Clean output. Place names, agent/item counts per place. No issues.

**places**: Clear. Tags and travel connections well-formatted. No issues.

**agents**: Good format but **location shown as `Place#0` instead of place name**. All 4 agents have this issue.

**goods**: Clean per-commodity breakdown with per-place totals. No issues.

**look**: Place header good. Connections good. **Entity list shows raw IDs**: `ItemLot#9 (ItemLot)`, `Facility#16 (Facility)` instead of meaningful descriptions like "10x Grain" or "Mill".

**inspect**: Agent name line good. `HomeostaticNeeds` line is readable. **UtilityProfile, DriveThresholds, MetabolismProfile, DeprivationExposure all dump raw `{:?}` format** — walls of `Permille(500)` text. Location shown as `Place#0`.

**relations**: Shows possession and placement clearly. **Place shown as `Place#0`**.

**inventory**: "20x Coin", "5x Water", "Load: 0/20" — clean and scannable. No issues.

**needs**: Best output in the entire CLI. Progress bars with urgency bands (`hunger: █████░░░░░ 500‰ [medium]`). No issues.

**inventory \<multi-word-name\>**: **Fails** — `inventory Merchant Vara` triggers Clap error "unexpected argument 'Vara'". Multi-word names cannot be passed to any command.

**actions**: **Raw Place# IDs for travel targets**. **Self-targeting actions** offered (attack self, fine self, exile self). **Duplicate entries** (bribe listed twice). **No duration** on `staff_market`. **35 items post-tick** with internal merchant operations (store_stock, collect_display_stock, stage_stock_for_sale, unstage_stock) polluting the list.

**do**: "Requested: sleep" — clear confirmation. No issues.

**tick**: "--- Tick 0 --- (5 events)" — functional but minimal. No indication of what actually happened.

**status**: Needs bars good. **Location as `Place#0`**.

**cancel**: Not tested (no action running at test time).

**switch**: **Multi-word names fail** same as inventory. Prefix matching requires matching the *start* of the full name ("Forager" works, "Lina" doesn't). **No suggestion of available names** on failure.

**observe**: "Observer mode — simulation runs without human control" — clear. No issues.

**events**: Event list with IDs, ticks, tags. Adequate but sparse — no human-readable event descriptions.

**event**: **CRITICAL** — 200+ lines of raw `{:?}` debug format for deltas. `EntityId { slot: 5, generation: 0 }`, `Permille(500)` everywhere. Completely unusable for understanding what happened.

**trace**: Works for causal chain walking. Tested on bootstrap event which has no chain — should have used a more interesting event. Format is adequate.

**save/load**: Clean messages, works correctly. No issues.

### Resolved Since Previous

No previous evaluation.

### Scores

| # | Metric | Score | Previous | Delta | Justification |
|---|--------|-------|----------|-------|---------------|
| 1 | Output Clarity | 3 | — | — | Pervasive raw IDs (Place#0, ItemLot#9) and {:?} debug format on complex components and event deltas. `needs` is the only well-formatted command. |
| 2 | Action Reliability | 5 | — | — | Actions execute when selected, but the action list includes self-targeting (attack self, fine self), duplicates (bribe x2), and internal merchant operations. No profile-error crashes observed this session. |
| 3 | State Legibility | 5 | — | — | `world`, `goods`, `needs`, `inventory` are good. But `inspect` is unusable (debug walls), `look` shows raw entity IDs, `agents`/`status`/`relations` show Place#0. Mixed bag. |
| 4 | Causal Traceability | 2 | — | — | `event` output is a wall of debug format — completely unusable. `events` list is sparse (tags only, no descriptions). `trace` works mechanically but the data it shows is incomprehensible. |
| 5 | Session Flow | 5 | — | — | Commands work and the REPL is responsive. But multi-word names are broken, the action list is overwhelming (35 items), and tick output gives no indication of what happened. |
| 6 | Error Recovery | 4 | — | — | Errors are reported but rarely helpful. "unexpected argument 'Vara'" doesn't suggest quoting. "No entity found matching 'Lina'" doesn't list available entities. No recovery guidance. |
| | **Average** | **4.0** | **—** | **—** | |

### Prioritized Recommendations

1. **[CRITICAL]** Replace raw `{:?}` debug format in `inspect` component display and `event` delta display with human-readable formatting. Components like UtilityProfile, DriveThresholds, MetabolismProfile should render as labeled key-value pairs (e.g., "hunger_weight: 500‰") not `UtilityProfile { hunger_weight: Permille(500), ... }`. Event deltas should show "Kael created" or "hunger: 500‰ → 502‰", not `Component(Set { entity: EntityId { slot: 5, generation: 0 }, ... })`. *(New)*

2. **[CRITICAL]** Resolve raw entity IDs (`Place#0`, `ItemLot#9`, `Facility#16`) to human-readable names throughout all commands. Places should show their name. ItemLots should show "10x Grain". Facilities should show their workstation type ("Mill"). Affects: `agents`, `status`, `look`, `inspect`, `relations`, `actions`, `switch`. *(New)*

3. **[HIGH]** Fix multi-word entity name parsing. Commands like `inventory Merchant Vara` and `switch Forager Lina` fail because Clap splits on spaces. Either accept quoted strings or join trailing arguments into a single name. *(New)*

4. **[HIGH]** Filter the action list to remove self-targeting actions (attack self, fine self, exile self), duplicates, and internal merchant operations (store_stock, collect_display_stock, stage_stock_for_sale, unstage_stock) that aren't meaningful user choices. *(New)*

5. **[MEDIUM]** Add human-readable event descriptions to `events` list output. Instead of just tags, show what happened: "Kael started sleeping", "Needs updated for Merchant Vara", "Forager Lina moved to Forest". *(New)*

6. **[MEDIUM]** Improve `switch` error recovery: when entity not found, list available agents. When multi-word name fails, suggest quoting or show prefix matches. *(New)*

7. **[MEDIUM]** Add a brief summary to `tick` output showing what the controlled agent (or AI agents) did during that tick, beyond just event count. *(New)*

8. **[LOW]** Add duration display for actions that currently show no duration (e.g., `staff_market`). *(New)*
