# S137PLACAULIN-004: PlanGuard.causal_links field

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `PlanGuard` shape (consumed by planner emitter in ticket 006)
**Deps**: archive/tickets/S137PLACAULIN-001.md (CausalLink), 002 (causal_links_per_step_cap on CognitiveProfile)

## Problem

S137 D4 extends `PlanGuard` with `causal_links: Vec<CausalLink>` so each step's precondition can carry provenance references to the step or evidence that supports it. Ticket 006's `plan_repair` module reads `causal_links` to identify the smallest failing prefix. Without the field, no localized repair search can locate broken provider links.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PlanGuard` is defined at `crates/worldwake-ai/src/plan_guard.rs:8-12` with three existing fields (`required_facts: Vec<RequiredFact>`, `min_confidence: Permille`, `invalidators: Vec<Invalidator>`). Derives `Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`. Test boundary at line 66. `CausalLink` landed in `archive/tickets/S137PLACAULIN-001.md` (`crates/worldwake-core/src/causal_link.rs`).
2. Spec `specs/S137-plan-causal-links-and-repair.md` D4 specifies `Vec<CausalLink>` (not `SmallVec` — smallvec is not a workspace dependency). The cap (`causal_links_per_step_cap: u8`) lands on `CognitiveProfile` in ticket 002 and is enforced at planner emit-time.
3. Shared boundary: the `PlanGuard` field shape consumed by `plan_revalidation.rs` (revalidation) and ticket 006's `plan_repair` module. Save-load compatibility through `#[serde(default)]`.
4. Construction sites: 5 sites in `crates/worldwake-ai/src/plan_revalidation.rs` at lines 1762, 1821, 1884, 1933, 2044 — all inside `#[cfg(test)]` (test boundary at line 486). No runtime construction sites outside test code.
5. Per spec Crates section: "extends `PlanGuard` (`crates/worldwake-ai/src/plan_guard.rs:8`) to carry `causal_links: Vec<CausalLink>` per plan step (capped by `CognitiveProfile.causal_links_per_step_cap`)". The field is dormant in this ticket — runtime construction (planner emitter populating the vec) lands in ticket 006.

## Architecture Check

1. **Forward-compatible additive field**: `#[serde(default)]` on `causal_links` ensures pre-ticket save/replay streams deserialize with an empty vec. The field is read by ticket 006; until then it is harmless ballast on the PlanGuard struct.
2. **No back-compat shim**: net-new field; no legacy alternative path coexisting.

## Verification Layers

1. Field-shape + default value → focused unit test in `plan_guard.rs` `#[cfg(test)]` asserting bincode roundtrip with empty `causal_links` vec.
2. Save-load tolerance → focused unit test asserting a pre-S137 byte fixture deserializes to a `PlanGuard` with `causal_links: Vec::new()`.
3. Single-layer ticket (struct field addition); the field's runtime consumption lives in tickets 006 and 007, where verification mapping applies separately.

## What to Change

### 1. Extend `PlanGuard` shape

In `crates/worldwake-ai/src/plan_guard.rs:8-12`, append:

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

In `crates/worldwake-ai/src/plan_revalidation.rs` at lines 1762, 1821, 1884, 1933, 2044, add `causal_links: Vec::new()` to each `PlanGuard { ... }` literal. The tests do not yet exercise causal-link behavior — that lands in ticket 006.

### 3. Tests in `plan_guard.rs`

Add a `#[cfg(test)]` test `plan_guard_causal_links_default_to_empty_via_serde` asserting that bincode-deserialized `PlanGuard` from a pre-field-addition byte stream lands with an empty `causal_links` vec.

## Files to Touch

- `crates/worldwake-ai/src/plan_guard.rs` (modify — struct + test)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — 5 test construction sites at 1762, 1821, 1884, 1933, 2044)

## Out of Scope

- Planner emitter populating `causal_links` with real provenance — ticket 006.
- Repair search reading `causal_links` — ticket 006.
- Causal-link cap enforcement at construction time — ticket 006 (uses `CognitiveProfile.causal_links_per_step_cap` from ticket 002).
- `SAVE_FORMAT_VERSION` bump — not required because `#[serde(default)]` handles backward compatibility.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai plan_guard` — new field test passes; existing tests pass after construction-site updates.
2. `cargo test -p worldwake-ai plan_revalidation` — 5 updated construction sites pass.
3. Existing suite: `cargo test --workspace`.

### Invariants

1. Pre-S137 `PlanGuard` bincode byte streams deserialize successfully with `causal_links: Vec::new()`.
2. No runtime emission of non-empty `causal_links` in this ticket — verified by inspection of construction sites.
3. `PlanGuard` derives unchanged; no derive widening.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_guard.rs` `#[cfg(test)]` — new test `plan_guard_causal_links_default_to_empty_via_serde`.
2. `crates/worldwake-ai/src/plan_revalidation.rs` `#[cfg(test)]` — 5 existing constructions updated to include the new field; no semantic change.

### Commands

1. `cargo test -p worldwake-ai plan_guard`
2. `cargo test -p worldwake-ai plan_revalidation`
3. `cargo test --workspace`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
