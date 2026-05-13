# S137PLACAULIN-004: PlanGuard.causal_links field

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `PlanGuard` shape (consumed by planner emitter in ticket 006)
**Deps**: archive/tickets/S137PLACAULIN-001.md (CausalLink), archive/tickets/S137PLACAULIN-002.md (causal_links_per_step_cap on CognitiveProfile), archive/tickets/S137PLACAULIN-003.md (save-format baseline)

## Problem

S137 D4 extends `PlanGuard` with `causal_links: Vec<CausalLink>` so each step's precondition can carry provenance references to the step or evidence that supports it. Ticket 006's `plan_repair` module reads `causal_links` to identify the smallest failing prefix. Without the field, no localized repair search can locate broken provider links.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PlanGuard` is defined at `crates/worldwake-ai/src/plan_guard.rs:8-12` with three existing fields (`required_facts: Vec<RequiredFact>`, `min_confidence: Permille`, `invalidators: Vec<Invalidator>`). Derives `Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`. Test boundary at line 66. `CausalLink` landed in `archive/tickets/S137PLACAULIN-001.md` (`crates/worldwake-core/src/causal_link.rs`).
2. Spec `specs/S137-plan-causal-links-and-repair.md` D4 specifies `Vec<CausalLink>` (not `SmallVec` — smallvec is not a workspace dependency). The cap (`causal_links_per_step_cap: u8`) lands on `CognitiveProfile` in ticket 002 and is enforced at planner emit-time.
3. Shared boundary: the `PlanGuard` field shape consumed by `plan_revalidation.rs` (revalidation) and ticket 006's `plan_repair` module. `#[serde(default)]` handles text-serde/fixture omission, but saved runtime state requires a `SAVE_FORMAT_VERSION` bump because `PlanGuard` is nested under persisted AI runtime state.
4. Construction sites: 5 drafted sites in `crates/worldwake-ai/src/plan_revalidation.rs` plus live reassessment fallout in `crates/worldwake-ai/src/plan_guard_build.rs`, `crates/worldwake-ai/src/agent_tick/tests.rs`, and `crates/worldwake-visualizer/src/tabs/plan.rs`. The runtime guard builder now initializes the dormant field to `Vec::new()`; non-empty runtime population remains ticket 006.
5. Per spec Crates section: "extends `PlanGuard` (`crates/worldwake-ai/src/plan_guard.rs:8`) to carry `causal_links: Vec<CausalLink>` per plan step (capped by `CognitiveProfile.causal_links_per_step_cap`)". The field is dormant in this ticket — runtime construction (planner emitter populating the vec) lands in ticket 006.

## Architecture Check

1. **Additive field with current-format bump**: `#[serde(default)]` on `causal_links` keeps omitted text-serde fixtures lawful, while save/replay compatibility is handled by advancing the current save format and rejecting older versions. The field is read by ticket 006; until then it is harmless ballast on the PlanGuard struct.
2. **No back-compat shim**: net-new field; no legacy alternative path or migration shim coexists.

## Verification Layers

1. Field-shape + default value → focused unit tests in `plan_guard.rs` `#[cfg(test)]` asserting text-serde omitted-field defaulting to an empty `causal_links` vec and bincode roundtrip of a non-empty `CausalLink`.
2. Save-load version boundary → focused save-load test asserting the next `SAVE_FORMAT_VERSION` value and rejection of the prior version.
3. Single-layer ticket (struct field addition); the field's runtime consumption lives in tickets 006 and 007, where verification mapping applies separately.

## Landed Changes

### 1. Extend `PlanGuard` shape

In `crates/worldwake-ai/src/plan_guard.rs:8-12`, appended:

```rust
pub struct PlanGuard {
    pub required_facts: Vec<RequiredFact>,
    pub min_confidence: Permille,
    pub invalidators: Vec<Invalidator>,
    #[serde(default)]
    pub causal_links: Vec<CausalLink>,
}
```

Import `CausalLink` from `worldwake_core::CausalLink`.

### 2. Update test construction sites

In `crates/worldwake-ai/src/plan_revalidation.rs` at lines 1762, 1821, 1884, 1933, 2044, added `causal_links: Vec::new()` to each `PlanGuard { ... }` literal. Live constructor fallout also required the same default in the runtime `build_plan_guard` emitter, one agent-tick test fixture, and one visualizer plan-tab test fixture. The tests do not yet exercise causal-link behavior — that lands in ticket 006.

### 3. Tests in `plan_guard.rs`

Added `#[cfg(test)]` tests `plan_guard_causal_links_default_to_empty_via_serde` and `plan_guard_causal_links_roundtrip_through_bincode`.

### 4. SAVE_FORMAT_VERSION bump

In `crates/worldwake-sim/src/save_load.rs:6`, bumped `SAVE_FORMAT_VERSION` from the S137 ticket-003 baseline `81` to `82`.

## Files to Touch

- `crates/worldwake-ai/src/plan_guard.rs` (modify — struct + test)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — 5 test construction sites at 1762, 1821, 1884, 1933, 2044)
- `crates/worldwake-ai/src/plan_guard_build.rs` (modify — runtime guard builder initializes dormant field to empty)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — test fixture literal)
- `crates/worldwake-visualizer/src/tabs/plan.rs` (modify — test fixture literal)
- `crates/worldwake-ai/Cargo.toml` (modify — dev-only RON dependency for text-serde omitted-field proof)
- `crates/worldwake-sim/src/save_load.rs` (modify — save-format version bump)

## Out of Scope

- Planner emitter populating `causal_links` with real provenance — ticket 006.
- Repair search reading `causal_links` — ticket 006.
- Causal-link cap enforcement at construction time — ticket 006 (uses `CognitiveProfile.causal_links_per_step_cap` from ticket 002).
- Planner/runtime population and consumption of non-empty links; this ticket only adds the persisted field and current-format version bump.

## Acceptance Result

### Tests Passed

1. `cargo test -p worldwake-ai --lib plan_guard` — new field tests passed; existing plan-guard/build tests passed after construction-site updates.
2. `cargo test -p worldwake-ai --lib plan_revalidation` — updated construction sites passed.
3. `cargo test -p worldwake-sim --lib save_load` — save-format version bump and prior-version rejection passed.
4. Existing suite: `cargo test --workspace` passed.

## Outcome

Completed on 2026-05-13.

- Added `PlanGuard.causal_links: Vec<CausalLink>` with `#[serde(default)]`.
- Initialized the field to `Vec::new()` in the runtime guard builder and all explicit PlanGuard test/visualizer fixtures found by the constructor sweep.
- Added focused proof for omitted-field RON deserialization defaulting to an empty vec and bincode roundtrip of a non-empty causal link.
- Bumped `SAVE_FORMAT_VERSION` from `81` to `82`; older save headers now reject with `UnsupportedVersion` through the existing current-format gate.
- Runtime population, cap enforcement, repair search consumption, and non-empty planner-emitted links remain owned by ticket 006.

## Deviations

- Reassessment found live constructor fallout outside the drafted five `plan_revalidation.rs` literals: `plan_guard_build.rs`, `agent_tick/tests.rs`, and `worldwake-visualizer/src/tabs/plan.rs`.
- The focused bincode proof uses a non-empty `causal_links` vec, which is stronger than the draft's empty-roundtrip wording.

### Invariants

1. Omitted-field text-serde payloads deserialize with `causal_links: Vec::new()`.
2. No runtime emission of non-empty `causal_links` in this ticket — verified by inspection of construction sites.
3. `PlanGuard` derives unchanged; no derive widening.
4. `SAVE_FORMAT_VERSION` advances once from the then-current S137 baseline.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/plan_guard.rs` `#[cfg(test)]` — new test `plan_guard_causal_links_default_to_empty_via_serde`.
2. `crates/worldwake-ai/src/plan_revalidation.rs` `#[cfg(test)]` — 5 existing constructions updated to include the new field; no semantic change.
3. `crates/worldwake-sim/src/save_load.rs` `#[cfg(test)]` — existing save-load version assertions updated to the next current-format value.

### Commands Passed

1. `cargo test -p worldwake-ai --lib plan_guard`
2. `cargo test -p worldwake-ai --lib plan_revalidation`
3. `cargo test -p worldwake-sim --lib save_load`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-ai --lib plan_guard`
- Passed `cargo test -p worldwake-ai --lib plan_revalidation`
- Passed `cargo test -p worldwake-sim --lib save_load`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `python3 .codex/skills/implement-ticket/scripts/check_closeout.py tickets/S137PLACAULIN-004.md`
- Passed `git diff --check`
