# S117CONMAIOBS-006: Section 2 supplementary tables — Maintenance rates and Recipe usage

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `archive/tickets/S117CONMAIOBS-004.md`, `specs/S117-convergence-maintenance-observer-smells.md`

## Problem

When a `MAINTENANCE_STARVATION` or `RECIPE_MONOCULTURE` anomaly fires, the analyst has to re-derive the supporting evidence (per-need accumulation/relief totals; per-recipe commit counts) by hand from Section 4's raw event log. This ticket surfaces both as per-agent tables in Section 2 so the analyst can cross-check detector output against raw aggregates without parsing events manually. The tables are also useful on their own — they make maintenance cadence and recipe repertoire visible even when no anomaly fires.

## Assumption Reassessment (2026-04-18)

1. Section 2 per-agent rendering happens at `bin/observer.rs:1602-1688` (Needs trajectory, Ticks above 750‰, Behavioral transitions, Locations visited, Max consecutive idle ticks). This ticket adds two table blocks inside that render loop, between "Locations visited" and "Max consecutive idle ticks".
2. `AgentStats.needs_samples` is already collected per-tick — the same data 003's detector reads. Per-run accumulation and relief totals are a straightforward aggregation over per-tick deltas across the full run.
3. Recipe commit counts are already retained in `AgentStats.actions_committed: BTreeMap<String, u32>`, but live Section 2 rendering does not currently receive a `RecipeRegistry`. Rendering deterministic recipe rows with canonical names therefore requires threading the live `RecipeRegistry` through `format_report()` or a private helper it calls.
4. Shared abstraction boundary under audit: the `format_report()` function's per-agent Section 2 rendering block plus the private helper surface it uses for recipe-name resolution. This remains observer-only read-side work, but the landed seam is not strictly “new `writeln!` calls only” because the report function must now accept the recipe registry.
5. This ticket is independent of the new `AnomalyKind` variants, but the recipe-usage table does depend on the live `RecipeRegistry` seam already verified in 004. It is not honestly parallel with 004 on the current branch.

## Architecture Check

1. The tables are derived views over authoritative read data (per-tick need samples + action-trace commit counts). FND-27 (Derived Summaries Are Caches): no stored state, recomputed every run. No backward-compatibility shim.
2. Adding rows to Section 2 preserves the `/scenario-analysis` skill's expected dump structure — the skill parses by section headers and table headers, not by fixed line counts. Table headers are new and distinct (`**Maintenance rates**` and `**Recipe usage**`), so no conflict with existing parsing.
3. Tables use the same `|` Markdown format already established in Section 2 (matches "Needs trajectory" and "Locations visited" style).

## Verification Layers

1. Maintenance rates table renders for an agent with non-empty `needs_samples` → focused unit test on the render helper, or assertion on the observer dump for a known short scenario.
2. Recipe usage table renders for an agent with non-empty `KnownRecipes` and commit history, using live registry-backed recipe names and preserving deterministic ordering → same verification shape.
3. Tables are omitted (or render an explicit "none" row) for an agent with no samples / no known recipes → focused unit test.
4. Single-layer ticket (observer read-side rendering); no action-trace or event-log proof surface applies.

## What to Change

### 1. Maintenance rates table renderer

In `format_report()` per-agent block at `bin/observer.rs:1602-1688`, after the "Locations visited" table (around line 1679), add:

```
**Maintenance rates** (‰)

| Need | Accumulation | Relief | Net |
|------|--------------|--------|-----|
| Hunger | {h_accum} | {h_relief} | {h_net} |
| Thirst | {t_accum} | {t_relief} | {t_net} |
| Fatigue | {f_accum} | {f_relief} | {f_net} |
| Bladder | {b_accum} | {b_relief} | {b_net} |
| Dirtiness | {d_accum} | {d_relief} | {d_net} |
```

Where for each need the observer computes:

- `accum = sum over consecutive per-tick deltas of max(0, delta)`
- `relief = sum over consecutive per-tick deltas of max(0, -delta)`
- `net = accum - relief` (signed; negative means the agent kept up)

If `needs_samples` is empty, skip the entire block.

### 2. Recipe usage table renderer

Immediately after the Maintenance rates block, add:

```
**Recipe usage**

| Recipe | Commits |
|--------|---------|
| {recipe_name} | {count} |
```

Rows are emitted for each recipe in the agent's `KnownRecipes.recipes` (iterating the `BTreeSet<RecipeId>` in its natural deterministic order). Recipes the agent knows but never committed show `0`. Recipes the agent committed but does not currently have in `KnownRecipes` (e.g., forgotten) are also listed with their commit count and a `(unknown)` suffix on the name — this edge case is rare but should not drop data. If the agent has neither known recipes nor commits, skip the block.

### 3. Helper functions

Both tables benefit from small helpers:

- `fn compute_maintenance_rates(samples: &[NeedsSample]) -> [(HomeostaticNeedId, u32, u32, i64); 5]` returning (need, accum, relief, net) per need.
- `fn recipe_usage_rows(agent_stats: &AgentStats, known_recipes: Option<&KnownRecipes>, registry: &RecipeRegistry) -> Vec<(String, u32)>` returning deterministic render rows: known recipes first in `RecipeId` order, then committed action names that do not currently map to a known recipe rendered with an ` (unknown)` suffix in deterministic name order.

Both helpers are private to `bin/observer.rs`.

### 4. Focused unit tests

Add to the existing `#[cfg(test)] mod tests`:

- `test_compute_maintenance_rates_tracks_accumulation_and_relief` — synthetic samples with known deltas; assert per-need accum and relief match expected sums.
- `test_recipe_usage_rows_iteration_order_is_deterministic` — construct synthetic known-recipes plus commit data; assert known rows iterate in `RecipeId` order and unknown committed rows are appended deterministically.
- `test_maintenance_rates_table_renders_for_sampled_agent` — invoke the per-agent Section 2 renderer with a non-empty samples vector; assert the rendered string contains "**Maintenance rates**" and the expected table headers.
- `test_recipe_usage_table_renders_for_agent_with_known_recipes` — similar pattern for recipe usage.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Rolling-window aggregations (003's detector reads 200-tick windows; this table reports whole-run totals).
- Per-anomaly cross-reference in the table (e.g., "this row triggered anomaly 7") — keep the tables simple; Section 3 already names the anomalies.
- Integration with external dashboards.

## Acceptance Criteria

### Tests That Must Pass

1. `test_compute_maintenance_rates_tracks_accumulation_and_relief` passes.
2. `test_recipe_usage_rows_iteration_order_is_deterministic` passes.
3. `test_maintenance_rates_table_renders_for_sampled_agent` passes.
4. `test_recipe_usage_table_renders_for_agent_with_known_recipes` passes.
5. Existing integration: `test_observer_mode_simulation_runs` still passes (observer dump structure remains parseable).
6. Existing suite: `cargo test -p worldwake-cli`.

### Invariants

1. Table iteration order is deterministic: needs in fixed `HomeostaticNeedId` declaration order (Hunger, Thirst, Fatigue, Bladder, Dirtiness); recipes in `BTreeSet<RecipeId>` natural order.
2. Tables use the same `**Header**` + Markdown-pipe-table convention as existing Section 2 blocks.
3. Whole-run totals equal the sum of per-tick deltas — a numerical consistency check between Section 2's table and Section 3's MaintenanceStarvation anomaly descriptions (spanning the same run).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` — four new focused unit tests above.

### Commands

1. `cargo test -p worldwake-cli --bin observer maintenance_rates`
2. `cargo test -p worldwake-cli --bin observer recipe_usage`
3. `cargo test -p worldwake-cli`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-18.

- Extended `crates/worldwake-cli/src/bin/observer.rs` with whole-run Section 2 supplementary tables for `Maintenance rates` and `Recipe usage`, rendered inside the per-agent `format_report()` block between `Locations visited` and `Max consecutive idle ticks`.
- Added private helper surfaces `compute_maintenance_rates()`, `render_maintenance_rates_table()`, `recipe_usage_rows()`, and `render_recipe_usage_table()` so the new tables stay deterministic and testable beside the live observer seam.
- Threaded the live `RecipeRegistry` through `format_report()` so recipe rows render canonical registry-backed names while still preserving deterministic fallback rows for committed recipes no longer present in `KnownRecipes`.
- Added focused observer unit coverage for maintenance-rate arithmetic, deterministic recipe-row ordering, maintenance-table rendering, and recipe-usage rendering.

## Deviations

- The drafted ticket claimed the change was “purely additive” inside `format_report()` with no signature fallout. Live reassessment showed the recipe-usage table needs the live `RecipeRegistry` to resolve canonical recipe names, so the landed seam widened `format_report()` to accept the registry and updated the local call sites/tests accordingly.
- The drafted helper sketch `commits_per_recipe(...) -> BTreeMap<RecipeId, u32>` could not honestly represent the ticket’s own “committed but not currently known” edge case. The landed helper is `recipe_usage_rows(...) -> Vec<(String, u32)>`, which preserves `RecipeId` order for known rows and appends deterministic ` (unknown)` rows for registry-backed committed recipes outside current `KnownRecipes`.
- The drafted ticket claimed independence from `001`–`005`. Live reassessment corrected that boundary: while the tables do not depend on new anomaly variants, the recipe-usage table does depend on the live `RecipeRegistry` seam already exercised by `004`, so the ticket `Deps` were corrected before implementation.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer maintenance_rates`
- Passed `cargo test -p worldwake-cli --bin observer recipe_usage`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
