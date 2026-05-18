# S148PORMOTBAC-008: Remove max_candidates_to_plan from CognitiveProfile and ReasoningProfile

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — removes `max_candidates_to_plan` field from both `CognitiveProfile` (`crates/worldwake-core/src/cognitive_profile.rs:25`) and `ReasoningProfile` (`crates/worldwake-ai/src/lib.rs:174`); migrates all 15+ reader sites to route through `PortfolioWeightsProfile.max_plans_for_mode(runtime.operating_mode)`
**Deps**: `archive/tickets/S148PORMOTBAC-002.md`, `archive/tickets/S148PORMOTBAC-003.md`, `archive/tickets/S148PORMOTBAC-004.md`, `specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

`max_candidates_to_plan` currently lives in two parallel stores — `CognitiveProfile.max_candidates_to_plan: u8` at `cognitive_profile.rs:25` (default `2`) and `ReasoningProfile.max_candidates_to_plan: u8` at `crates/worldwake-ai/src/lib.rs:174` (default `2`). The two are kept in sync by hand at construction sites like `decision_runtime.rs:426` (`max_candidates_to_plan: reasoning.max_candidates_to_plan`). After ticket 002 lifts portfolio weights to `PortfolioWeightsProfile` with per-mode plan caps (`max_plans_normal`, `max_plans_emergency`, `max_plans_idle`) and ticket 003 adds `OperatingMode` per-tick derivation cached on `AgentDecisionRuntime`, the legacy single-cap field becomes obsolete. Per FND-28, the two parallel stores are removed atomically and all readers migrate to `PortfolioWeightsProfile.max_plans_for_mode(runtime.operating_mode)`.

## Assumption Reassessment (2026-05-17)

1. Reader site inventory (15+ sites confirmed by grep): `crates/worldwake-core/src/cognitive_profile.rs:297` (test assertion `max_candidates_to_plan == 2`), `crates/worldwake-cli/src/handlers/inspect.rs:300` (handler reads `cognitive.max_candidates_to_plan`), `crates/worldwake-cli/src/scenario/types.rs:1779` (scenario test assertion `cognitive.max_candidates_to_plan == 4`), `crates/worldwake-ai/src/failure_handling.rs:1941` (`max_candidates_to_plan: reasoning.max_candidates_to_plan`), `crates/worldwake-ai/src/decision_runtime.rs:426` (same pattern), `crates/worldwake-ai/src/agent_tick/planning.rs:660` (the primary runtime consumer: `let candidate_cap = usize::from(cognitive.max_candidates_to_plan);`), `crates/worldwake-ai/src/agent_tick/planning.rs:2469, 2932, 6391` (additional fixture/access sites), `crates/worldwake-ai/src/search/tests.rs:53`, `crates/worldwake-ai/src/goal_model.rs:2625`, `crates/worldwake-ai/src/agent_tick/tests.rs:172`. Total >15; both fields removed in this ticket.
2. After ticket 002: `PortfolioWeightsProfile` carries `max_plans_normal: u8`, `max_plans_emergency: u8`, `max_plans_idle: u8` with defaults 5/3/5. Replacement read path: `weights.max_plans_for_mode(mode)` where `mode` is `runtime.operating_mode` (cached per-tick by ticket 003 and refreshed by ticket 004's wiring). Add the helper `max_plans_for_mode` to `PortfolioWeightsProfile`:
   ```rust
   pub fn max_plans_for_mode(&self, mode: OperatingMode) -> u8 {
       match mode {
           OperatingMode::Normal    => self.max_plans_normal,
           OperatingMode::Emergency => self.max_plans_emergency,
           OperatingMode::Idle      => self.max_plans_idle,
       }
   }
   ```
3. Shared abstraction under audit: the per-tick planning cap surface. The primary runtime consumer at `agent_tick/planning.rs:660` is the load-bearing site — the rest are fixture constructions and a CLI inspect/scenario test. The runtime swap is mechanical: `cognitive.max_candidates_to_plan` → `weights.max_plans_for_mode(runtime.operating_mode)`.
4. Test fixtures that currently construct `CognitiveProfile { max_candidates_to_plan: N, … }` or `ReasoningProfile { max_candidates_to_plan: N, … }` literally need to drop the field at the construction site. Tests that asserted on `max_candidates_to_plan == 2` (e.g., `cognitive_profile.rs:297`) either delete the assertion (the field no longer exists) or rewrite to assert on the new field set (`PortfolioWeightsProfile.max_plans_normal == 5`).
5. CLI surface impact: `crates/worldwake-cli/src/handlers/inspect.rs:300` formats `cognitive.max_candidates_to_plan` for human inspection. After removal, the handler reads the new field via the agent's `PortfolioWeightsProfile` (printed as `max_plans (normal/emergency/idle)`) — the inspect output schema changes; downstream consumers (if any — likely none, since inspect output is operator-facing) need awareness.

## Architecture Check

1. FND-28 alignment: the two parallel stores are removed atomically rather than left as parallel-but-deprecated. No `pub fn max_candidates_to_plan() -> u8 { self.max_plans_normal }` shim; no `#[deprecated]` aliases; no transient dual-truth.
2. The replacement read path (`weights.max_plans_for_mode(mode)`) is the single source of truth for per-tick planning caps. The cap can now vary per-mode without per-agent fields multiplying — agent diversity is expressed via the per-mode `max_plans_<mode>` field set on `PortfolioWeightsProfile` per FND-22.
3. The reader migration touches three concentric layers — runtime planning (ai), CLI handlers (cli), test fixtures (core/ai/cli) — but every site reads the same new accessor with the same mode-aware semantics. No leakage of the legacy cap concept into the new design.

## Verification Layers

1. Field removal completeness → workspace compilation under `cargo clippy --workspace --all-targets -- -D warnings` — every former reader is migrated or compilation fails
2. New cap read semantics → focused unit test asserting `weights.max_plans_for_mode(Normal) == 5`, `max_plans_for_mode(Emergency) == 3`, `max_plans_for_mode(Idle) == 5`
3. Per-tick planning cap is mode-aware → focused unit test in `agent_tick/planning.rs` constructing a runtime with `operating_mode = Emergency` and asserting the planning cap reduces to 3

## What to Change

### 1. Add `max_plans_for_mode` helper to `PortfolioWeightsProfile`

In `crates/worldwake-core/src/portfolio_weights_profile.rs` (added by ticket 002), add:

```rust
impl PortfolioWeightsProfile {
    pub fn max_plans_for_mode(&self, mode: OperatingMode) -> u8 {
        match mode {
            OperatingMode::Normal    => self.max_plans_normal,
            OperatingMode::Emergency => self.max_plans_emergency,
            OperatingMode::Idle      => self.max_plans_idle,
        }
    }
}
```

### 2. Remove `max_candidates_to_plan` from `CognitiveProfile`

In `crates/worldwake-core/src/cognitive_profile.rs:25`: delete the field, the corresponding serde annotations, and the initializer in `CognitiveProfile::default()`. Remove the test assertion at line 297. Drop the field from the serde round-trip tests at line 504+.

### 3. Remove `max_candidates_to_plan` from `ReasoningProfile`

In `crates/worldwake-ai/src/lib.rs:174`: delete the field and any associated initializer at line 191. If `ReasoningProfile` only exists to relay this field (verify during reassessment), consider removing `ReasoningProfile` entirely; otherwise just drop the field.

### 4. Migrate reader sites

Per the inventory in Assumption Reassessment item 1:

- `crates/worldwake-ai/src/agent_tick/planning.rs:660` (primary runtime consumer): change
  ```rust
  let candidate_cap = usize::from(cognitive.max_candidates_to_plan);
  ```
  to
  ```rust
  let weights = belief_view.portfolio_weights_profile(agent);
  let candidate_cap = usize::from(weights.max_plans_for_mode(runtime.operating_mode));
  ```
  (or thread `weights` and `mode` from the call site where ticket 004 already established them — the change reuses the values already in scope rather than refetching).
- `crates/worldwake-ai/src/agent_tick/planning.rs:2469, 2932, 6391`: migrate fixture construction sites — drop the `max_candidates_to_plan` field from `CognitiveProfile`/`ReasoningProfile` literals.
- `crates/worldwake-ai/src/decision_runtime.rs:426`, `crates/worldwake-ai/src/failure_handling.rs:1941`: same fixture migration.
- `crates/worldwake-ai/src/search/tests.rs:53`, `crates/worldwake-ai/src/goal_model.rs:2625`, `crates/worldwake-ai/src/agent_tick/tests.rs:172`: same.
- `crates/worldwake-cli/src/handlers/inspect.rs:300`: rewrite the handler's inspect-output line to read `weights.max_plans_normal/emergency/idle` from the agent's `PortfolioWeightsProfile`. Adjust the human-readable output format accordingly.
- `crates/worldwake-cli/src/scenario/types.rs:1779`: rewrite the test assertion to reference the new `PortfolioWeightsProfile` fields rather than `cognitive.max_candidates_to_plan`.

### 5. Golden test audit

Existing portfolio and planning golden tests at `crates/worldwake-ai/tests/golden_portfolio_planning.rs` assert specific budget-exhaustion behavior under the old default of `2`. Audit each test:
- If the test's intent is "verify N candidates get planned per tick," update the asserted N to match the new default of `5` (or pin a fixture `max_plans_normal` value to the original N).
- If the test's intent is "verify budget-exhaustion behavior," ensure the new mode-aware machinery still allows the test to construct a scenario that exhausts the cap.

This audit is light-touch — the full golden migration lives in ticket 010, which inherits this ticket as a dependency. This ticket's responsibility is to leave the goldens in a state where they still pass after the field removal (even if the assertions change shape).

## Files to Touch

- `crates/worldwake-core/src/portfolio_weights_profile.rs` (modify — add `max_plans_for_mode` helper)
- `crates/worldwake-core/src/cognitive_profile.rs` (modify — remove field + default + tests at line 297 + serde round-trip line 504+)
- `crates/worldwake-ai/src/lib.rs` (modify — remove field from `ReasoningProfile` at line 174 + default at line 191)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — runtime consumer at line 660 + fixtures at 2469, 2932, 6391)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — fixture at line 426)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — fixture at line 1941)
- `crates/worldwake-ai/src/search/tests.rs` (modify — fixture at line 53)
- `crates/worldwake-ai/src/goal_model.rs` (modify — fixture at line 2625)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — fixture at line 172)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify — inspect-output rewrite at line 300)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — test assertion at line 1779)
- `crates/worldwake-ai/tests/golden_portfolio_planning.rs` (modify — light-touch audit; full migration in ticket 010)
- Likely: any RON scenario file under `scenarios/` that explicitly enumerates `cognitive_profile:` with `max_candidates_to_plan:` (grep during implementation; drop the field from those scenarios if found)

## Out of Scope

- Full golden migration to the 5-slot taxonomy + new variant names (ticket 010)
- Adding new tests covering operating-mode-aware planning cap behavior (the focused tests in ticket 004 already establish mode wiring; ticket 010 adds the goldens)
- Observer rendering of the new plan-cap structure (ticket 009)
- Removing `ReasoningProfile` entirely if it becomes vestigial after this ticket — defer to a future cleanup if warranted; in-scope here is only the field removal

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core portfolio_weights_profile::tests::max_plans_for_mode_*` — new tests assert each mode returns its corresponding field
2. `cargo test -p worldwake-ai agent_tick::planning` — fixture-migrated tests pass; primary runtime consumer reads through the new accessor
3. `cargo test -p worldwake-cli` — scenario types test (rewritten) and inspect handler tests pass
4. Existing suite: `cargo test --workspace`
5. Lint: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `CognitiveProfile.max_candidates_to_plan` does not exist anywhere in the workspace after this ticket lands.
2. `ReasoningProfile.max_candidates_to_plan` does not exist anywhere in the workspace after this ticket lands.
3. The per-tick planning cap is read from `PortfolioWeightsProfile.max_plans_for_mode(runtime.operating_mode)` at every reader site — no parallel cap source survives.
4. No shim, no `#[deprecated]` alias, no `pub fn max_candidates_to_plan(&self) -> u8` accessor that resolves to the new field set.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/portfolio_weights_profile.rs::tests` — extend ticket 002's tests with `max_plans_for_mode_returns_correct_field_per_mode`
2. `crates/worldwake-ai/src/agent_tick/planning.rs::tests` — add or extend a focused test asserting the planning cap shifts with `runtime.operating_mode` (e.g., `planning_cap_drops_to_three_under_emergency_mode`)
3. `crates/worldwake-core/src/cognitive_profile.rs::tests` — drop existing assertions on `max_candidates_to_plan` field; surrounding `CognitiveProfile` assertions remain
4. `crates/worldwake-cli/src/scenario/types.rs` — rewrite the existing test at line 1779 to assert on the new `PortfolioWeightsProfile` fields

### Commands

1. `cargo test -p worldwake-core portfolio_weights_profile cognitive_profile`
2. `cargo test -p worldwake-ai agent_tick::planning decision_runtime failure_handling goal_model`
3. `cargo test -p worldwake-cli scenario handlers`
4. `./scripts/verify.sh`
