# S127QUAAWAACQ-002: GoalKind::AcquireCommodity quantity field + workspace-wide migration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — extends `GoalKind::AcquireCommodity` payload, migrates ~344 destructure/construction sites, surfaces quantity in decision trace, bumps `SAVE_FORMAT_VERSION`
**Deps**: S127QUAAWAACQ-001

## Problem

S127's quantity-aware acquisition is anchored on `GoalKind::AcquireCommodity` carrying an `AcquisitionQuantity { desired_min, desired_target, horizon_ticks }` payload (D2). Every ranker, search arm, action selector, and decision trace must see the same intent (Design Goal 1). Per FND-28, the migration is atomic — no shim is added beside the old quantity-implicit path. This ticket lands the variant extension, the `~344` destructure/construction site migration (D3), the decision-trace surfacing of the new fields (D11 part a), the `is_satisfied` semantic change (`inventory >= desired_min`), and the `SAVE_FORMAT_VERSION` bump because `GoalKind` is part of agent state and the bincode encoding changes.

## Assumption Reassessment (2026-04-26)

1. `crates/worldwake-core/src/goal.rs:28-31` defines `GoalKind::AcquireCommodity { commodity: CommodityKind, purpose: CommodityPurpose }` — confirmed during reassessment. `GoalKind` derives `Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize` (line 24). `AcquisitionQuantity` from ticket 001 derives `Copy`, so adding it preserves `GoalKind: Copy`.
2. `specs/S127-quantity-aware-acquisition.md` D2/D3 prescribe the variant shape and migration scope. Design Goal 9 requires `GoalKey::from(GoalKind)` to ignore `quantity` so goal identity remains `(commodity, purpose)`.
3. Shared boundary under audit: `GoalKind` (authoritative goal identity in `worldwake-core`) and its three downstream contracts — `GoalDispatchKey::from_goal_kind` (`crates/worldwake-ai/src/goal_dispatch_key.rs:99-106`, three-way payload-aware split into `AcquireSelfConsume`/`AcquireRecipeInput`/`AcquireRestock`), `GoalKindPlannerExt` (12 methods in `crates/worldwake-ai/src/goal_model.rs`, with `is_satisfied` semantics changing from "agent has any of the commodity" to "agent inventory >= desired_min"), and `GoalDispatchDeclaration::reify` constructors at `goal_dispatch_decl.rs:738-746` (must default to `AcquisitionQuantity::single()`).
4. Construction-site spot-check: `grep -rn "GoalKind::AcquireCommodity {" crates/ | wc -l` → 344 sites. Most ranker destructures use `{ commodity, .. }` (safe), but ~30 sites in `ranking.rs` and dozens of test-fixture sites enumerate fields explicitly and must add `quantity:`. Constructions cannot use spread syntax (enum variants don't support `..Default::default()`), so the count is load-bearing → Large effort.
5. `is_satisfied` lives at `crates/worldwake-ai/src/goal_model.rs:1362` (`GoalKind::AcquireCommodity { commodity, purpose } => match purpose {…}`). Current implementation reads only `commodity`/`purpose`; changing it to compare against `desired_min` requires reading the agent's believed inventory of the commodity through the existing `GoalBeliefView` accessor (`inventory_of(agent, commodity)` or equivalent — to be confirmed during implementation).
6. `SAVE_FORMAT_VERSION` is at `crates/worldwake-sim/src/save_load.rs:6` (currently `48`). Adding a bincode-serialized field to `GoalKind` breaks the format → bump to `49`.
7. Existing focused tests exercising `is_satisfied` for `AcquireCommodity`: grep `crates/worldwake-ai/src/goal_model.rs` `#[cfg(test)]` block for `is_satisfied` and `AcquireCommodity` together — record names during implementation and update Test Plan if any need extending.
13. Adjacent contradictions: none — the spec's other deliverables (D7 partial-success, D8 candidate-gen) explicitly depend on this ticket's variant shape and is_satisfied semantics; they are downstream consumers, not contradictions.

## Architecture Check

1. Atomic migration honors FND-28 (no backwards-compatibility shim) — the old quantity-implicit path is removed, not preserved beside the new one. All 344 sites are updated in one ticket because intermediate compile-safe states cannot exist (the variant either has the field or it doesn't).
2. `GoalKey::from(GoalKind)` ignoring `quantity` keeps goal identity stable: two acquisition goals with same commodity+purpose but different `desired_target` share a key, so the planner does not double-emit, and `GoalDispatchKey::from_goal_kind` continues to route on `purpose` only (Design Goal 9).
3. `is_satisfied` semantic change ("inventory >= desired_min" instead of "any of the commodity") preserves FND-14 (belief-only planning — read believed inventory through `GoalBeliefView`, not authoritative world state).

## Verification Layers

1. `is_satisfied` returns true iff believed inventory >= `desired_min` → focused unit test in `goal_model.rs` `#[cfg(test)]`.
2. `GoalKey::from` ignores `quantity` → focused unit test asserting two `AcquireCommodity { commodity: c, purpose: p, quantity: q1 }` and `{ commodity: c, purpose: p, quantity: q2 }` with `q1 != q2` produce equal `GoalKey`.
3. Decision-trace emits `desired_min`, `desired_target`, `horizon_ticks` for `AcquireCommodity` → decision-trace assertion in a focused planner-runtime test.
4. Save format bumps to `49` and old saves with version `48` are rejected with `SaveError::VersionMismatch` → existing save-load test infrastructure handles this when the version constant changes; verify failing-load test still passes.
5. Workspace compiles after migration → `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` are the verification surfaces (no targeted test for "all 344 sites compile" — the build itself is the proof).

## What to Change

### 1. Extend `GoalKind::AcquireCommodity` in `crates/worldwake-core/src/goal.rs`

Add `quantity: AcquisitionQuantity` to the variant. Update `GoalKey::from(GoalKind)` (or equivalent identity extraction) to destructure with `{ commodity, purpose, quantity: _ }` so `quantity` is ignored. Add a focused test confirming two variants differing only in `quantity` produce equal `GoalKey`.

### 2. Migrate `goal_dispatch_decl.rs:738-746`

The three `GoalDispatchKey → GoalKind::AcquireCommodity` constructors (`AcquireSelfConsume`, `AcquireRecipeInput`, `AcquireRestock`) must add `quantity: AcquisitionQuantity::single()` as the default reified payload.

### 3. Migrate all `GoalKindPlannerExt` impl sites in `crates/worldwake-ai/src/goal_model.rs`

12 methods touch `AcquireCommodity`. Most destructures (`{ commodity, .. }` patterns) compile unchanged after the migration because `..` ignores the new field. Update sites that enumerate fields explicitly (e.g., `{ commodity: _, purpose: _ }` at line 1362) to add `quantity: _`. Implement the **`is_satisfied` semantic change**: for `AcquireCommodity { commodity, purpose, quantity }`, return `true` iff the agent's believed inventory of `commodity` (read via `GoalBeliefView::inventory_of(agent, commodity)` or the analogous existing accessor) is `>= quantity.desired_min.get() as u32`. Preserve any existing purpose-specific gating (e.g., `RecipeInput` may have additional satisfaction checks — confirm during implementation).

### 4. Migrate `feasibility.rs` and `ranking.rs` constructions/destructures

`feasibility.rs:1031, 1140` — test-fixture constructions add `quantity: AcquisitionQuantity::single()`. `ranking.rs` destructures using `{ commodity, .. }` patterns are unaffected; the ~30 explicit-field destructures and ~20 construction sites all need updates. Ranking does not yet read `desired_target` (that lands in ticket 007) — this ticket only ensures compile-cleanliness.

### 5. Migrate `candidate_generation.rs:2972, 3036`

Existing emitters construct `GoalKind::AcquireCommodity { commodity, purpose: CommodityPurpose::SelfConsume }` — add `quantity: AcquisitionQuantity::single()`. Ticket 007 will replace `single()` with computed quantity from agent state; this ticket only keeps the workspace compiling.

### 6. Migrate `display.rs` and any other `worldwake-cli` sites

Update `crates/worldwake-cli/src/display.rs` `AcquireCommodity` formatting to include the quantity tuple (e.g., `AcquireCommodity({commodity}, {purpose}, min={n}/target={n}/horizon={t})`).

### 7. Migrate all goldens and unit tests under `crates/worldwake-ai/tests/`, `crates/worldwake-systems/tests/`, and any other test fixture sites

Replace bare `GoalKind::AcquireCommodity { commodity, purpose }` constructions with the quantity-aware shape using `AcquisitionQuantity::single()` for compile-time fixtures.

### 8. Surface `quantity` fields in decision trace (D11 part a)

Locate the existing decision-trace emitter that formats `AcquireCommodity` (likely in `crates/worldwake-ai/src/decision_trace.rs` or a goal-formatting helper — confirm during implementation). Extend the formatted line to include `desired_min`, `desired_target`, `horizon_ticks` alongside the existing `commodity` and `purpose`.

### 9. Bump `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs:6` — change from `48` to `49`. Confirm the failing-load test for old versions still passes by mismatch detection.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify — 12 method impls + `is_satisfied` semantic change)
- `crates/worldwake-ai/src/feasibility.rs` (modify — test fixtures)
- `crates/worldwake-ai/src/ranking.rs` (modify — ~50 sites of destructures + constructions)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — emitter constructions at 2972, 3036)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — quantity formatting; **Likely:** confirm the exact module hosting the `AcquireCommodity` decision-trace formatter via `grep -n "AcquireCommodity" crates/worldwake-ai/src/decision_trace*` during reassessment)
- `crates/worldwake-cli/src/display.rs` (modify — display formatting)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump SAVE_FORMAT_VERSION)
- All `crates/worldwake-ai/tests/golden_*.rs` files constructing `AcquireCommodity` (modify — workspace-wide grep covers these)
- All `crates/worldwake-systems/tests/*.rs` files constructing `AcquireCommodity` (modify — workspace-wide grep covers these)

## Out of Scope

- Replacing `AcquisitionQuantity::single()` with computed quantity from agent state (need projection, headroom) — ticket 007.
- Ranker reading `desired_target` for tiebreaking — ticket 007.
- Belief-view accessors for new components — tickets 004/005.
- Save migration logic for older versions — per FND-28, the bump rejects old saves; no migration path is added.

## Acceptance Criteria

### Tests That Must Pass

1. `goal_key_ignores_quantity` — two `GoalKind::AcquireCommodity` variants differing only in `quantity` produce equal `GoalKey`.
2. `is_satisfied_acquire_commodity_below_desired_min` — agent with believed inventory `< desired_min` is not satisfied.
3. `is_satisfied_acquire_commodity_at_desired_min` — agent with believed inventory `>= desired_min` is satisfied.
4. `decision_trace_emits_quantity_fields` — runtime `agent_tick` decision-trace test asserts the formatted line for `AcquireCommodity` contains `desired_min`, `desired_target`, `horizon_ticks`.
5. Existing suite: `cargo test --workspace`.
6. Lint: `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. After migration, no construction or destructure site references `GoalKind::AcquireCommodity { commodity, purpose }` without `quantity` — verified by build success.
2. `GoalKey::from(GoalKind::AcquireCommodity { … quantity: q })` is independent of `q` (Design Goal 9).
3. `is_satisfied` reads believed inventory only (FND-14 — no authoritative world-state read).
4. `SAVE_FORMAT_VERSION = 49`; bincode-encoded saves at version `48` fail to load with the existing version-mismatch error.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` `#[cfg(test)]` — `goal_key_ignores_quantity` focused test.
2. `crates/worldwake-ai/src/goal_model.rs` `#[cfg(test)]` — `is_satisfied_acquire_commodity_below_desired_min` and `is_satisfied_acquire_commodity_at_desired_min` focused tests; also extend any existing `is_satisfied` AcquireCommodity tests recorded during reassessment to use `AcquisitionQuantity::single()` constructions.
3. `crates/worldwake-ai/tests/` — at least one runtime `agent_tick` decision-trace test asserting the new fields appear in the formatted line. Likely extends an existing decision-trace golden rather than creating a new one — confirm during implementation.

### Commands

1. `cargo test -p worldwake-core acquisition_quantity goal_key_ignores_quantity`
2. `cargo test -p worldwake-ai is_satisfied_acquire_commodity`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh`
