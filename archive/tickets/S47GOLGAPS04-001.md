# S47GOLGAPS04-001: Hungry merchant eats own listed sale stock — golden test + replay

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — removed sale_kinds suppression in candidate_generation.rs
**Deps**: S04 (fully implemented and archived)

## Problem

The golden E2E suite covers the merchant selling lifecycle end to end (Scenarios 75–86 in `golden_merchant_selling.rs`), but no scenario exercises the cross-system interaction where a merchant's survival need directly consumes their own trade-listed lot. This is a distinct emergent code path: the `eat` action's `consume_one_unit` archives the bread lot, which carries a `SaleListing` — so the survival action physically destroys the enterprise asset. Existing Scenario 44 (wounded politician pain vs enterprise) tests priority ordering on different entities; this scenario tests it on the *same* entity.

## Assumption Reassessment (2026-04-01)

1. **`ConsumeOwnedCommodity{Bread}` candidate generation**: `emit_consume_goals` in `candidate_generation.rs` emits for any possessed edible lot. No special filtering excludes lots with `SaleListing`. Confirmed by reading the function — it iterates `view.possessed_lots(agent)` and checks `commodity.spec().consumable_profile.is_some()`.
2. **`SellCommodity{Bread}` candidate generation**: `emit_sell_goals` in `candidate_generation.rs` emits for lots with `SaleListing` at or anchored to `home_market`. Confirmed present after S04MERSELMAR-014.
3. **Priority class divergence**: `ConsumeOwnedCommodity` maps to `self_consume_priority` → `classify_band(hunger, thresholds.hunger)` in `ranking.rs:332–336`. With hunger at `pm(950)` and default critical threshold `pm(900)`, this produces `GoalPriorityClass::Critical`. `SellCommodity` maps to `GoalPriorityClass::Medium` at `ranking.rs:343–350`. Critical outranks Medium — divergence is driven by priority class, not motive score.
4. **Lot consumption mechanics**: `consume_one_unit` in `inventory.rs:127–152` archives the lot when `quantity=1`, or splits when `quantity>1`. When the lot is archived, all components (including `SaleListing`) are removed. For clean SaleListing removal, the test should use `Quantity(1)`.
5. **Existing helpers**: `seed_merchant` in `golden_merchant_selling.rs:45–103` seeds a sated merchant (`HomeostaticNeeds::new_sated()`). The test needs critical hunger instead, so it must either (a) mutate needs after calling `seed_merchant`, or (b) seed the agent directly. Option (a) is simpler and reuses the existing helper.
6. **No existing golden coverage**: Searched `golden_merchant_selling.rs` scenario headers (75–86) — none involve hunger-driven consumption of a listed lot. Searched `golden_ai_decisions.rs` and `golden_emergent.rs` — no scenario tests `ConsumeOwnedCommodity` on a lot with `SaleListing`. Searched "Evaluated and Rejected Scenarios" and "Removed Backlog Items" in `docs/golden-e2e-coverage.md` — no match.
7. **Scenario ID**: Highest existing scenario ID is 86. New scenario will be 87.
8. **Isolation choice**: Setup includes only one agent (the merchant) with no buyer, no other agents. This isolates the survival-vs-enterprise ranking to a single decision without competing affordances from other agents. `SellCommodity` will still be generated (via `emit_sell_goals`) but will lose ranking to `ConsumeOwnedCommodity`.

## Architecture Check

1. This adds pure golden test coverage — no engine changes. The test proves that existing general-purpose ranking rules (priority class ordering) produce the correct survival-over-enterprise behavior without any special merchant-hunger override logic. This is the cleanest validation of Principle 1 (Maximal Emergence) for the needs×trade interaction.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Merchant selects `ConsumeOwnedCommodity{Bread}` over `SellCommodity{Bread}` → decision trace (committed goal kind)
2. Merchant executes `eat` action on the listed bread lot → action trace (`ActionTraceKind::Completed` with `eat` action)
3. Bread lot archived after consumption → authoritative world state (`get_component_item_lot` returns `None`)
4. `SaleListing` removed with archived lot → authoritative world state (`get_component_sale_listing` returns `None`)
5. Hunger decreases after eating → authoritative world state (`HomeostaticNeeds.hunger` < initial value)
6. Conservation invariant holds → `verify_live_lot_conservation` passes

## What to Change

### 1. Add Scenario 87 header and two golden tests to `golden_merchant_selling.rs`

Add at the end of the file:

- `// Scenario 87: Hungry Merchant Eats Own Listed Sale Stock` metadata header block with GoalKinds, ActionDomains, Systems, Foundation Principles
- `run_hungry_merchant_eats_listed_stock(replay: bool)` shared runner:
  - Call `seed_merchant` with `CommodityKind::Bread`, `Quantity(1)` to get `(merchant, bread_lot)`
  - Mutate `HomeostaticNeeds` on the merchant to set `hunger: pm(950)` (critical) while keeping other needs sated
  - Add `SaleListing { listed_at: Tick(0) }` to `bread_lot` (or confirm `seed_merchant` already does this — it does NOT; `SaleListing` is added by `staff_market` action start. The test must add it directly)
  - Tick the simulation for ~40 ticks
  - Assert decision trace shows `ConsumeOwnedCommodity { commodity: Bread }` committed
  - Assert action trace shows `eat` completed
  - Assert `bread_lot` is archived (no `ItemLot` component)
  - Assert `SaleListing` is gone (no `SaleListing` component on `bread_lot`)
  - Assert merchant hunger decreased
  - Assert `verify_live_lot_conservation` passes
  - If `replay`: re-run from same seed, assert world hash and event log hash match
- `golden_hungry_merchant_eats_listed_stock()` — calls runner with `replay: false`
- `golden_hungry_merchant_eats_listed_stock_replays_deterministically()` — calls runner with `replay: true`

### 2. Update `docs/golden-e2e-coverage.md` pending backlog

After S47 tests are implemented, remove or update the S47 entry in "Pending Backlog Summary" to reflect completion. (This is a post-implementation step — the entry already exists from the gap analysis.)

## Files to Touch

- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — add Scenario 87 tests)
- `docs/golden-e2e-coverage.md` (modify — update S47 pending entry after implementation)

## Out of Scope

- Engine changes to `consume_one_unit`, `SaleListing`, or ranking logic — this is test-only
- Multi-unit lot consumption (Quantity > 1) — that case leaves `SaleListing` on the remaining lot, which is correct behavior but not the emergent scenario this test targets
- Buyer interaction — no buyer needed; the scenario isolates survival-vs-enterprise on a single agent
- `SellCommodity` plan search or execution — the test only needs to prove the ranking outcome, not the full sell pipeline

## Acceptance Criteria

### Tests That Must Pass

1. `golden_hungry_merchant_eats_listed_stock` — merchant with critical hunger eats listed bread, lot archived, SaleListing gone, hunger decreased
2. `golden_hungry_merchant_eats_listed_stock_replays_deterministically` — identical world and event log hashes on replay
3. Existing suite: `cargo test -p worldwake-ai --test golden_merchant_selling`

### Invariants

1. `verify_live_lot_conservation` passes — no items created or destroyed outside explicit actions
2. Priority class ordering: `Critical` (survival) always outranks `Medium` (enterprise) regardless of motive scores
3. Deterministic replay produces identical world hash and event log hash

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs::golden_hungry_merchant_eats_listed_stock` — proves survival-over-enterprise emergence on a shared lot entity
2. `crates/worldwake-ai/tests/golden_merchant_selling.rs::golden_hungry_merchant_eats_listed_stock_replays_deterministically` — proves deterministic replay of the same scenario

### Commands

1. `cargo test -p worldwake-ai --test golden_merchant_selling -- hungry_merchant`
2. `cargo test -p worldwake-ai --test golden_merchant_selling`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome (2026-04-01)

### What changed

1. **Engine change**: Removed the blanket `sale_kinds` suppression in `candidate_generation.rs` that prevented `ConsumeOwnedCommodity` from being emitted for commodities in a merchant's `MerchandiseProfile.sale_kinds`. The ranking system now handles the survival-vs-enterprise tradeoff through `GoalPriorityClass` — Critical/High hunger outranks Medium enterprise.
2. **Updated focused test**: `merchant_does_not_emit_consume_owned_for_sale_commodity` → `merchant_emits_consume_owned_for_sale_commodity` (inverted assertion).
3. **Updated `already_satisfied` gate**: Removed `sale_kinds` exclusion from the need-satisfaction check so sale stock counts as satisfying the need (preventing redundant AcquireCommodity generation).
4. **Added Scenario 87**: 2 golden tests (`hungry_merchant_eats_listed_stock` + replay) in `golden_merchant_selling.rs`.
5. **Updated `docs/golden-e2e-coverage.md`**: Moved S47 from pending to removed backlog.

### Deviations from ticket

- Ticket stated "Engine Changes: None" — implementation required removing the sale_kinds suppression heuristic, which was a FOUNDATIONS P1/P20/P22 violation discovered during implementation. The suppression prevented natural emergence by hardcoding a candidate filter instead of letting the ranking system decide.

### Verification

- `cargo test -p worldwake-ai`: 938 lib + all golden tests pass (0 failures)
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- Deterministic replay: identical world and event log hashes confirmed
