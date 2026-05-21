# S158ACQLOCAL-001: Acquisition planner picks a recipe-infeasible remote harvest branch over a local trade, starving the buyer

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` acquisition candidate generation / GOAP plan ranking (and possibly agenda parking / `RevivalTrigger::CounterpartyAvailable` interaction)
**Deps**: Follow-up to S158 (belief-view remote truth-leak closure). Introducing change: `bedbce48` (`S158BELVIEWLEAK-001`), which gated `listed_sale_lots_at` / `seller_for_sale_lot` / `has_sale_listing` on local visibility in `crates/worldwake-sim/src/per_agent_belief_view.rs`.

## Problem

The golden `survival_trade_proves_substitute_market_branch` (`golden-survival / trade`) regressed after S158: Buyer Nila's hunger stays critical for **1254 / 1440 ticks** (max allowed 220), so she starves.

S158 is correct. It closed the remote sale-listing belief leak per FND-14/FND-15 (information locality): an agent can no longer read a remote market's live `SaleListing` to plan a purchase from afar. This is proven by the updated `merchant_selling` and `agent_tick` unit tests (which now assert remote/inferred listing beliefs must **not** select a seller-backed trade branch) and by every other golden family staying green.

What the closure **unmasked** is a pre-existing acquisition-planner defect that the leak had been silently compensating for:

- Buyer Nila starts co-located with Merchant Sera at Market Square; she trades twice and eats (T8/T10) while both agents are still co-present.
- Once their independent self-care routines desync and the merchant is not co-present at her decision tick, she can no longer "see" the market listing from afar (correct). Her `AcquireCommodity(Apple, SelfConsume)` goal then anchors at the **South Orchard** apple-resource harvest branch (`Travel → orchard`).
- **She lacks the recipe to harvest apples.** `scenarios/survival-trade.ron` gives Buyer Nila `known_recipes: ["Harvest Water"]` only — no "Harvest Apple". The orchard branch is therefore recipe-infeasible: she travels there and never harvests (zero apple harvests in her committed-action set).
- The result is a deterministic ~8-tick Market↔Orchard commute thrash (alternating Apple-at-orchard and Water-at-market goals), no further trades, and monotonic hunger climb to starvation.

On `main` the leak let her plan a market purchase from anywhere, so this recipe-infeasible orchard branch was never exercised. The bug is that acquisition candidate generation / GOAP ranking emits and prefers a remote resource-harvest branch the agent can never complete (missing recipe), instead of remaining local to trade (or parking the purchase to await the counterparty).

## Assumption Reassessment (2026-05-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Failing golden: `crates/worldwake-ai/tests/scenarios/survival_trade.rs::survival_trade_proves_substitute_market_branch` (`#[ignore]`, run via `golden-survival.yml`, job `golden-survival / trade`). Reproduced locally with `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 'scenarios::survival_trade::survival_trade_proves_substitute_market_branch'`. Identical signature to CI: "Buyer Nila hunger exceeded authored critical pm(840) for 1254 consecutive ticks (max allowed: 220)".
2. Root cause isolated by revert experiments against `HEAD`: reverting `crates/worldwake-ai/src/agent_tick/planning.rs` to `main` alone → still fails; reverting `crates/worldwake-sim/src/per_agent_belief_view.rs` to `main` alone → passes. The belief-view leak closure is necessary and sufficient to surface the failure; the `planning.rs` `pending_repair_context` change is not implicated.
3. Shared boundary under audit: the acquisition planner surface that turns an `AcquireCommodity(commodity, SelfConsume)` goal into anchored candidate branches (trade-at-market vs. harvest-at-resource) and ranks them — `crates/worldwake-ai/src/candidate_generation.rs` and the GOAP plan search/ranking in `search.rs` / `ranking.rs`. The contract under audit: a branch whose terminal action requires a recipe the agent does not know must not be generated as viable (or must be pruned/deprioritized below a viable local alternative).
4. Intended invariant of the failing scenario (from its header/`Proves` block): the buyer survives by **staying local** and buying the substitute (Apple) from the co-located merchant — "The proved food branch stays local." She is not meant to harvest at the orchard at all.
5. Live `GoalKind` under test: `AcquireCommodity { commodity: Apple, purpose: SelfConsume }`. Per-tick trace shows its selected plan's first step is `Travel` with `target_place = South Orchard`, never a `harvest` op against the orchard apple resource. Confirm the exact operator/affordance surface that anchors apple acquisition at the orchard resource vs. the market listing.
6. AI regression layer: candidate generation + plan ranking (not pure `agent_tick` runtime). A full action-registry golden is the right proof surface; the existing `survival_trade` golden already is that surface.
8. Heuristic/filter being audited: whatever lets a resource-harvest acquisition branch be generated without checking `known_recipes` feasibility for the harvesting agent. The missing substrate is a recipe-feasibility precondition (or its correct propagation into ranking). This ticket adds that substrate; it must not reopen the remote-listing leak S158 closed.
12. Isolation note: `survival-trade.ron` deliberately gives the buyer coin + a substitute list `[Apple, Grain]` and only `Harvest Water` — the intent is to force the **trade** branch. The orchard apple resource exists for scenario texture (and merchant restock), not as a buyer-harvest path.
13. Adjacent contradiction: this is a separate bug from S158, correctly classified as its own ticket (the skill's "FOUNDATIONS-alignment change unmasks a pre-existing defect" case).

## Architecture Check

1. The clean fix prunes/deprioritizes acquisition branches the planning agent is structurally incapable of executing (missing `known_recipes` entry for the harvest), so the planner falls back to the viable local trade — or, when the counterparty is transiently absent, parks the purchase via the existing `RevivalTrigger::CounterpartyAvailable` path (added under S158) and does other local self-care, rather than committing to a doomed remote round-trip. This restores "stays local" without reopening the leak.
2. No backward-compat shim: do not reintroduce remote sale-listing visibility. The fix lives entirely in candidate generation / ranking feasibility, not in belief visibility.

## Verification Layers

1. Recipe-infeasible harvest branch is not selected → decision trace at the buyer shows the selected `AcquireCommodity(Apple)` plan's terminal/first non-travel op is `Trade` (or the goal is parked pending) — never a `harvest` op anchored at a resource whose harvest recipe the agent lacks.
2. Buyer remains local / survives → `survival_trade_proves_substitute_market_branch` passes: hunger critical run ≤ contract limit; `successful_trade_count` grows beyond the two opening trades; no Market↔Orchard commute thrash.
3. Leak stays closed → `merchant_selling` and `agent_tick` unit tests added/updated under S158 still pass (remote/inferred listing beliefs select no trade branch).

## What to Change

### 1. Recipe-feasibility gate on resource-harvest acquisition branches

In acquisition candidate generation (`crates/worldwake-ai/src/candidate_generation.rs` and any anchor/branch enumeration it calls), require that a harvest-at-resource branch's harvesting agent knows the recipe the resource demands (`known_recipes`) before the branch is emitted as viable, or propagate that infeasibility into ranking so a viable local trade outranks it. Exact symbols to confirm during reassessment.

### 2. (Investigate) Local-wait / pending-purchase preference when counterparty is transiently absent

Confirm whether, with the infeasible harvest branch removed, the buyer correctly parks the `AcquireCommodity(Apple)` purchase via `RevivalTrigger::CounterpartyAvailable` (S158 `planning.rs` machinery) and stays local. If she instead idles or thrashes, extend the parking/revival interaction so a transiently-absent co-located seller is awaited locally rather than abandoned.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — likely)
- `crates/worldwake-ai/src/search.rs` and/or `ranking.rs` (modify — if feasibility flows through ranking)
- `crates/worldwake-ai/tests/scenarios/survival_trade.rs` (modify — only if a tighter sub-assertion is warranted; the existing assertions already capture the regression)

## Out of Scope

- Reopening or weakening S158's remote sale-listing leak closure.
- Granting Buyer Nila a "Harvest Apple" recipe in `survival-trade.ron` — the scenario intent is the local trade branch, not a buyer-harvest path.
- The `planning.rs` `pending_repair_context` snapshot change (verified not implicated).

## Acceptance Criteria

### Tests That Must Pass

1. `survival_trade_proves_substitute_market_branch` — buyer survives the full 1440 ticks within the authored critical-run contract and commits more than the two opening trades.
2. `survival_trade_replays_deterministically`.
3. A new focused candidate-generation/ranking unit test: an agent lacking the required harvest recipe does not receive a viable harvest-at-resource branch for `AcquireCommodity`, while a co-located trade branch (or pending-park) is preferred.
4. Existing suite: `cargo test -p worldwake-ai`; full `golden-survival` family via `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1`.
5. Leak-closure regression guard: the `merchant_selling` and `agent_tick` tests updated under S158 still pass.

### Invariants

1. The planner never commits a plan whose terminal action the acting agent is structurally incapable of executing (missing `known_recipes` entry).
2. Belief-only planning and information locality (FND-14/FND-15) remain intact — no remote `SaleListing` is read to form a trade branch.

## Test Plan

### New/Modified Tests

- New focused unit test in `worldwake-ai` candidate-generation/ranking covering recipe-infeasible harvest-branch pruning.
- Re-run `golden-survival / trade` to confirm green.

## Notes / Reproduction Evidence

- Per-tick buyer trace (instrumented, then reverted) over T20–T80 shows strict alternation: `AcquireCommodity(Apple)` → `Travel → South Orchard` and `AcquireCommodity(Water)` → `Travel → Market Square`, with `sees_apple_listing=true` on every market pass yet no trade, and hunger climbing 346 → 520+ continuously.
- `scenarios/survival-trade.ron`: Buyer Nila `known_recipes: ["Harvest Water"]` (no "Harvest Apple"); merchant stages apples at Market Square; both start at Market Square; orchard (South Orchard) holds the Apple resource via OrchardRow.
- Revert matrix (against branch `implement-S158-belief-view-remote-truth-leak-closure`): planning.rs→main = FAIL; per_agent_belief_view.rs→main = PASS.
