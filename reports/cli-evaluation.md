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

### Score Trend (include if 3+ evaluations exist)

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

---

## EVALUATION #2

**Date**: 2026-04-03
**Scenario**: scenarios/cli-evaluation.ron (fixed: added missing `side_benefit_weight` to all 4 UtilityProfiles)
**Transcript**: reports/cli-evaluation-transcripts/eval-2.txt

### Session Notes

Launched the CLI against the evaluation scenario after patching a stale `side_benefit_weight` field. Exercised all 4 workflow sequences plus adaptive edge cases (`help`, `do 999`, `inspect Nonexistent`). Significant improvements from Eval #1: multi-word entity names now work everywhere, raw `Place#0` IDs are resolved to place names in `agents`/`status`/`relations`/`look`, and `inspect` component display is now human-readable with labeled key-value pairs instead of `{:?}` walls. The `needs` and `inventory` commands remain excellent.

However, two major issues persist: (1) event delta display (`event N`) still dumps raw `{:?}` debug format with `EntityId { slot: 5, generation: 0 }` and full struct dumps — completely unreadable; (2) the action list still includes self-targeting actions (attack self, fine self, exile self), duplicates (bribe x2, attack x2), and internal merchant operations (store_stock, collect_display_stock, etc.). New issues found: `status` shows "(no location)" for in-transit agents without saying where they're going, `help` exits with error code 1 and leaks Clap internal doc-comments, `UtilityProfile` display omits the `side_benefit_weight` field, and `inspect` shows raw internal entity IDs in the header (`#6`).

### Per-Command Analysis

**world**: Clean. Place names and counts. No issues.

**places**: Clean. Tags, connections, travel ticks. No issues.

**agents**: **FIXED** — locations now show place names instead of `Place#0`. Clean output.

**goods**: Clean. Per-commodity, per-place breakdown. No issues.

**look**: **FIXED** — entities now show descriptive names (`20× Coin`, `Mill`) instead of raw `ItemLot#9`. Clean and useful.

**inspect**: **MAJOR IMPROVEMENT** — component display now uses labeled key-value pairs. UtilityProfile shows `hunger: 300‰` instead of `UtilityProfile { hunger_weight: Permille(300), ... }`. DriveThresholds, MetabolismProfile, DeprivationExposure all readable. **Remaining issues**: (a) `side_benefit_weight` omitted from UtilityProfile display, (b) entity header shows raw internal ID `#6`.

**relations**: **FIXED** — shows place name instead of `Place#0`.

**inventory**: Clean. Works with multi-word names now (**FIXED** from Eval #1).

**needs**: Excellent. Progress bars with urgency bands. Best output in CLI.

**actions**: 40 items listed. **Persistent issues**: self-targeting (attack/fine/exile self), duplicates (bribe x2), no duration on travel/staff_market/defend/steal, internal merchant operations polluting the list. Travel actions don't show their tick duration even though the data exists.

**do**: Clean confirmation. No issues.

**tick**: Minimal — just event count per tick. No indication of what happened.

**status**: Clean when at a location. **Issue**: shows "(no location)" for in-transit agents — should say "in transit to X" or "traveling to X (N ticks remaining)".

**cancel**: Clean. Works correctly — agent returns to origin.

**switch**: **FIXED** — multi-word names work. Clean confirmation with location.

**observe**: Clean. No issues.

**events**: Event list with IDs, ticks, tags, actors. Some events show "(no tags)" with no further description.

**event**: Header portion (tick, tags, cause, actor, place, targets, witnesses) is clean and readable. **CRITICAL**: delta section still dumps raw `{:?}` format — `EntityId { slot: 5, generation: 0 }`, `InTransitOnEdge(InTransitOnEdge { edge_id: TravelEdgeId(5), ... })`, `RouteExperience(RouteExperience { edges: { ... } })`. Completely unusable.

**trace**: Works mechanically but showed only a single-event chain for the tested event. No parent events visible.

**save/load**: Clean. Works correctly with tick confirmation on load.

**help**: **New issue** — exits with error code 1 and prints to stderr. Leaks Clap internal doc-comment: "Clap wrapper for REPL command parsing. `multicall = true`...".

### Resolved Since Previous

- **Raw entity IDs in `agents`, `status`, `relations`, `look`** — was [CRITICAL] in Eval #1, now **fixed**. Place names, item descriptions, and facility types shown instead of `Place#0`, `ItemLot#9`, `Facility#16`.
- **Raw `{:?}` debug format in `inspect` component display** — was [CRITICAL] in Eval #1, now **fixed**. Components render as labeled key-value pairs.
- **Multi-word entity name parsing** — was [HIGH] in Eval #1, now **fixed**. `inventory Merchant Vara`, `switch Guard Theron`, `inspect Merchant Vara` all work.

### Scores

| # | Metric | Score | Previous | Delta | Justification |
|---|--------|-------|----------|-------|---------------|
| 1 | Output Clarity | 6 | 3 | +3 | Major improvement: no more Place#0 IDs, inspect is readable. But event deltas still raw {:?}, UtilityProfile omits side_benefit_weight, inspect header shows raw #N ID, help leaks Clap internals. |
| 2 | Action Reliability | 5 | 5 | 0 | Actions execute when selected. But self-targeting, duplicates, and internal merchant ops still pollute the list. No crashes. |
| 3 | State Legibility | 7 | 5 | +2 | world/places/goods/agents/needs/inventory/look all clean and scannable. inspect greatly improved. Remaining gap: "(no location)" for in-transit. |
| 4 | Causal Traceability | 3 | 2 | +1 | Event headers improved (actor names, targets readable). But event deltas are still raw {:?} walls. trace shows shallow chains. tick gives no summary. |
| 5 | Session Flow | 6 | 5 | +1 | Multi-word names fixed removes major friction. Commands flow naturally. Remaining gaps: tick gives no summary, no in-transit status, action list overwhelming. |
| 6 | Error Recovery | 5 | 4 | +1 | "invalid action number, run 'actions' first" is helpful. "no entity matching X" is clear. But help exits with error, no entity suggestions on failure, no recovery guidance for most errors. |
| | **Average** | **5.3** | **4.0** | **+1.3** | |

### Prioritized Recommendations

1. **[CRITICAL]** Replace raw `{:?}` debug format in `event` delta display with human-readable formatting. Deltas should show "Kael arrived at Thornwall Village" or "LocatedIn: added (Kael → Thornwall Village)", not `Component(Removed { entity: EntityId { slot: 5, generation: 0 }, ... })`. This is the single largest readability gap remaining. *(Recurring: 2 consecutive evaluations — was CRITICAL #1 in Eval #1, partially fixed for inspect but event deltas still raw)*

2. **[HIGH]** Filter the action list to remove self-targeting actions (attack self, fine self, exile self), duplicates (bribe x2, attack x2), and internal merchant operations (store_stock, collect_display_stock, stage_stock_for_sale, unstage_stock). *(Recurring: 2 consecutive evaluations)*

3. **[HIGH]** Show in-transit status clearly: replace "(no location)" with "in transit to X (N ticks remaining)" in `status` and `switch` output. *(New)*

4. **[MEDIUM]** Add `side_benefit_weight` to UtilityProfile display in `inspect`. The field exists on the struct but the CLI formatter omits it. *(New)*

5. **[MEDIUM]** Fix `help` command: should exit with code 0 (not 1), print to stdout (not stderr), and remove the Clap internal doc-comment ("Clap wrapper for REPL command parsing. `multicall = true`..."). *(New)*

6. **[MEDIUM]** Add human-readable event descriptions to `events` list and improve `tick` output with a brief summary of what happened. *(Recurring: 2 consecutive evaluations)*

7. **[MEDIUM]** Remove raw internal entity ID from `inspect` header (e.g., `Merchant Vara (Agent) #6` → `Merchant Vara (Agent)`). *(New)*

8. **[LOW]** Add duration display for actions that currently show no duration: travel, staff_market, defend, steal, relieve_wilderness. *(Recurring: 2 consecutive evaluations)*

9. **[LOW]** Improve error recovery: when entity not found, list available entities. When `help` fails, handle gracefully. *(Recurring: 2 consecutive evaluations)*

---

## EVALUATION #3

**Date**: 2026-04-03
**Scenario**: scenarios/cli-evaluation.ron (updated: added Forge, Medicine, Bow for commodity variety)
**Transcript**: reports/cli-evaluation-transcripts/eval-3.txt

### Session Notes

This evaluation measures the impact of the 6 CLI improvements implemented between Eval #2 and #3 in the same session. Exercised all 4 workflow sequences plus `help`. Every CRITICAL and HIGH recommendation from Eval #2 has been resolved: event deltas now show human-readable formatted text instead of `{:?}` walls, the action list is filtered to remove self-targeting/duplicates/internal ops (from 40 items to 8-22), in-transit agents show "in transit to X (N ticks remaining)", `inspect` no longer shows raw entity IDs and now includes `side_benefit_weight`, and `help` exits cleanly with code 0 and a user-friendly header.

One new issue discovered: resource source entities at Eldergrove Forest display as `Facility#20` in both `look` and `actions` output. This is because resource sources are facilities without a `WorkstationMarker`, so `entity_display_name()` falls through to the raw `EntityKind#slot` format. This affects only the forest location in the current scenario.

### Per-Command Analysis

**world**: Clean. No issues.

**places**: Clean. No issues.

**agents**: Clean. Names, control, location, status all readable.

**goods**: Clean. Now shows 9 commodities including Medicine and Bow from scenario update.

**look**: Clean at Thornwall Village. **Issue at Eldergrove Forest**: resource source entity shows as `Facility#20 (Facility)` instead of a meaningful name.

**inspect**: **FIXED** — no raw `#N` ID in header. `side_benefit_weight` now shown in UtilityProfile. All components readable.

**relations**: Clean.

**inventory**: Clean. Multi-word names work. Guard Theron now shows Sword + Bow.

**needs**: Excellent. Progress bars with urgency bands.

**actions**: **MAJOR IMPROVEMENT** — self-targeting removed, duplicates eliminated (bribe x1 now), internal merchant ops filtered. 8 items at start (was 11 in Eval #2), 22 after ticking (was 40). **Remaining**: `Facility#20` raw ID in `queue_for_facility_use` at Eldergrove Forest. Some actions still show no duration (travel, staff_market, defend).

**do**: Clean.

**tick**: Minimal — event count only.

**status**: **FIXED** — "in transit to Hearthstone Inn (3 ticks remaining)" instead of "(no location)".

**cancel**: Clean.

**switch**: Clean. Multi-word names work.

**observe**: Clean.

**events**: Event list with IDs, ticks, tags, actors. Some "(no tags)" events still lack description.

**event**: **FIXED** — deltas now show "AgentBeliefStore: set on Kael", "PossessedBy: added (8× Apple → Forager Lina)" instead of raw `{:?}` debug format.

**trace**: Shallow — single event shown with no causal parent chain.

**save/load**: Clean.

**help**: **FIXED** — "Worldwake CLI commands" header, exit code 0, no Clap internal text.

### Resolved Since Previous

- **Raw `{:?}` debug format in event delta display** — was [CRITICAL] in Eval #2, now **fixed**. Deltas show human-readable component/relation changes with resolved entity names.
- **Action list pollution (self-targeting, duplicates, internal merchant ops)** — was [HIGH] in Eval #2, now **fixed**. Action count reduced from 40 to 8-22.
- **"(no location)" for in-transit agents** — was [HIGH] in Eval #2, now **fixed**. Shows "in transit to X (N ticks remaining)".
- **Missing `side_benefit_weight` in UtilityProfile display** — was [MEDIUM] in Eval #2, now **fixed**.
- **`help` command exits with error code 1 and leaks Clap internals** — was [MEDIUM] in Eval #2, now **fixed**.
- **Raw entity ID `#N` in inspect header** — was [MEDIUM] in Eval #2, now **fixed**.

### Scores

| # | Metric | Score | Previous | Delta | Justification |
|---|--------|-------|----------|-------|---------------|
| 1 | Output Clarity | 8 | 6 | +2 | Nearly all output is human-readable. Event deltas fixed. inspect clean. Only gap: `Facility#20` raw ID for resource source entities. |
| 2 | Action Reliability | 8 | 5 | +3 | Dramatic improvement: self-targeting, duplicates, and internal ops filtered. Actions work when selected. Only gap: some actions lack duration. |
| 3 | State Legibility | 8 | 7 | +1 | In-transit status now clear. inspect shows all profile fields. Only gap: `Facility#20` in look/actions at forest. |
| 4 | Causal Traceability | 6 | 3 | +3 | Event deltas now readable. Event headers clear. But tick gives no summary, events list has "(no tags)" entries, and trace shows shallow chains. |
| 5 | Session Flow | 7 | 6 | +1 | Action list is manageable (8-22 items vs 40). In-transit status clear. Help works. Tick output still minimal. |
| 6 | Error Recovery | 6 | 5 | +1 | Help fixed. "invalid action number" is helpful. But no entity suggestions on not-found, no recovery guidance for most errors. |
| | **Average** | **7.2** | **5.3** | **+1.9** | |

### Score Trend

| Eval | Avg | Delta |
|------|-----|-------|
| #1 | 4.0 | — |
| #2 | 5.3 | +1.3 |
| #3 | 7.2 | +1.9 |

### Prioritized Recommendations

1. **[MEDIUM]** Resolve `Facility#20` raw ID for resource source entities. `entity_display_name()` falls through to `EntityKind#slot` for facilities without a `WorkstationMarker`. Resource sources should display as their commodity source type (e.g., "Apple Orchard" or "Apple source"). Affects `look` and `actions` at Eldergrove Forest. *(New)*

2. **[MEDIUM]** Add human-readable event descriptions to `events` list and improve `tick` output with a brief summary of what happened. Events with "(no tags)" give no indication of their content. *(Recurring: 3 consecutive evaluations)*

3. **[MEDIUM]** Improve `trace` command to show deeper causal chains. Currently shows only the target event with no parent events. *(New — first time trace depth was specifically tested)*

4. **[LOW]** Add duration display for actions that currently show no duration: travel, staff_market, defend, steal, relieve_wilderness. *(Recurring: 3 consecutive evaluations)*

5. **[LOW]** Improve error recovery: when entity not found, list available entities. *(Recurring: 3 consecutive evaluations — help portion now resolved)*
