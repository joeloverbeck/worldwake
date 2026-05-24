# S166OPPCMPSRCFID-001: Lift `belief_status_tag_for_claim` into shared helper

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` helper extraction only; no behavior change at existing call sites.
**Deps**: spec `archive/specs/S166-opportunity-compiler-source-fidelity.md` (D4)

## Problem

The free function `belief_status_tag_for_claim` is duplicated verbatim across
`crates/worldwake-ai/src/agent_tick/frame.rs:708-728` and
`crates/worldwake-ai/src/agenda_manager.rs:488-508`. Both copies map a
`(refutation state, effective confidence, per-agent threshold)` triple onto
`BeliefStatusTag` using identical arithmetic. This is a pre-existing FND-28
violation: two live authoritative representations of the same derivation. S166's
D1 (ticket 003) reuses this derivation in `opportunity_compiler/compile.rs` —
without consolidation here, 003 would add a third copy. This ticket lifts the
single function into a shared helper module so 003 (and any future call site)
consumes one definition.

## Assumption Reassessment (2026-05-24)

1. `belief_status_tag_for_claim` exists at exactly two sites, with identical bodies, both inside `worldwake-ai`:
   - `crates/worldwake-ai/src/agent_tick/frame.rs:708-728` (called at line 705 inside `agent_tick/frame.rs::any_claim_has_status`).
   - `crates/worldwake-ai/src/agenda_manager.rs:488-508` (called at line 485 inside an `any_claim_has_status` analog at `agenda_manager.rs:464+`).
   Workspace-wide grep finds no other definitions or call sites.
2. Both copies depend on the same `RuntimeBeliefView` surface (`belief_confidence_policy(agent)`, `claim_confidence_threshold(agent)`) plus `effective_claim_confidence(claim, tick, &policy)` from `worldwake-core::belief`. No additional inputs differ between the copies; the lift is a pure refactor.
3. Shared abstraction boundary under audit: a single `pub(crate) fn belief_status_tag_for_claim(view: &dyn RuntimeBeliefView, agent: EntityId, claim: &EntityBeliefClaim, tick: Tick) -> BeliefStatusTag` in a new `crates/worldwake-ai/src/belief_status.rs` module. The trait dispatch through `&dyn RuntimeBeliefView` matches the existing call shape at both sites.
4. Information-path classification: the same derivation today travels through two independent lawful paths (frame.rs's `any_claim_has_status` and agenda_manager.rs's analog). Canonical path after the change: the shared helper. The duplicate paths are removed in-scope (both file-local definitions deleted in this ticket); no follow-up is deferred.
5. Adjacent contradictions: none surfaced beyond the pre-existing duplicate this ticket resolves.

## Architecture Check

1. Lifting the function into a single module eliminates the FND-28 violation (two live authoritative copies of the same derivation) cleanly: there is exactly one definition after the change, and both existing call sites consume it via the same `pub(crate)` symbol. No shim or alias is introduced.
2. The new module sits inside `worldwake-ai` because the function reads `RuntimeBeliefView` (a `worldwake-sim` trait) and `EntityBeliefClaim` (a `worldwake-core` type) — `worldwake-ai` is the lowest crate that already depends on both. Placing the helper in core would require core to depend on sim's trait, violating the workspace layering.

## Verified Layers

1. Behavioral equivalence at the two existing call sites — focused unit test in `belief_status.rs` covers each `BeliefStatusTag` input case (refuted; effective >= 2*threshold; effective >= threshold; effective < threshold) and asserts the shared derivation returns the expected tag.
2. Compile equivalence — both former call sites import the shared helper via `use crate::belief_status::belief_status_tag_for_claim`.
3. No-regression at existing call sites — the AI crate's existing test suite passed unchanged.

## Landed Changes

### 1. Created `crates/worldwake-ai/src/belief_status.rs`

Added a new module exposing:

```rust
use worldwake_core::{
    BeliefConfidencePolicy, BeliefStatusTag, EntityBeliefClaim, EntityId, Permille, Tick,
    effective_claim_confidence,
};
use worldwake_sim::RuntimeBeliefView;

pub(crate) fn belief_status_tag_for_claim(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    claim: &EntityBeliefClaim,
    tick: Tick,
) -> BeliefStatusTag {
    belief_status_tag_for_claim_parts(
        claim,
        tick,
        &view.belief_confidence_policy(agent),
        view.claim_confidence_threshold(agent),
    )
}

fn belief_status_tag_for_claim_parts(
    claim: &EntityBeliefClaim,
    tick: Tick,
    policy: &BeliefConfidencePolicy,
    threshold: Permille,
) -> BeliefStatusTag {
    if claim.refuted_at_tick.is_some() {
        return BeliefStatusTag::Contradicted;
    }

    let effective = effective_claim_confidence(claim, tick, policy);
    let threshold = threshold.value();
    let certain_floor = threshold.saturating_mul(2).min(1000);
    if effective >= certain_floor {
        BeliefStatusTag::Certain
    } else if effective >= threshold {
        BeliefStatusTag::Probable
    } else {
        BeliefStatusTag::Stale
    }
}
```

The implementation imports `effective_claim_confidence` and the belief types through
the live `worldwake_core` re-export surface. The module is registered in
`crates/worldwake-ai/src/lib.rs` as `pub(crate) mod belief_status;`. No public
re-export was added.

### 2. Deleted the duplicate in `crates/worldwake-ai/src/agent_tick/frame.rs`

Removed the file-local `fn belief_status_tag_for_claim(...)` definition and added
`use crate::belief_status::belief_status_tag_for_claim;`. The existing
`belief_status_matches` call keeps the same call shape.

### 3. Deleted the duplicate in `crates/worldwake-ai/src/agenda_manager.rs`

Removed the file-local `fn belief_status_tag_for_claim(...)` definition and added
`use crate::belief_status::belief_status_tag_for_claim;`. The existing
`belief_status_matches` call keeps the same call shape.

## Landed Files

- `crates/worldwake-ai/src/belief_status.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add `mod belief_status;`)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — delete duplicate, add import)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify — delete duplicate, add import)

## Out of Scope

- Changes to either existing call site's surrounding logic (e.g., `any_claim_has_status` in either file). This ticket only lifts the inner function.
- Adding the third call site from `opportunity_compiler/compile.rs` — that belongs to ticket 003 (D1).
- Promoting `belief_status_tag_for_claim` to a method on `RuntimeBeliefView`. The free-function shape is sufficient; trait-method placement would expand the trait's surface without a current need.

## Acceptance Result

### Tests Passed

1. Passed focused test `belief_status::tests::derives_tag_for_each_status_class` in `crates/worldwake-ai/src/belief_status.rs`, covering the 4 input branches (`refuted_at_tick.is_some()` -> `Contradicted`; `effective >= 2*threshold` -> `Certain`; `effective >= threshold` -> `Probable`; `effective < threshold` -> `Stale`).
2. Passed existing AI crate suite: `cargo test -p worldwake-ai`.
3. Passed AI crate clippy gate: `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.

### Invariants

1. Exactly one `fn belief_status_tag_for_claim` definition exists in `worldwake-ai` after the change, verified by `rg -n 'fn belief_status_tag_for_claim|belief_status_tag_for_claim\(' crates/worldwake-ai/src`.
2. Both former call sites in `frame.rs` and `agenda_manager.rs` compile and execute through the shared helper with no behavior change at either site.

## Test Plan Result

### Focused Tests

1. `crates/worldwake-ai/src/belief_status.rs` (inline `#[cfg(test)] mod tests` block) — focused unit test asserting the 4-branch tag derivation matrix.
2. No existing tests required expectation changes; behavior at the lifted sites is unchanged.

### Commands Run

1. `cargo test -p worldwake-ai --lib belief_status::tests::derives_tag_for_each_status_class -- --exact`
2. `cargo test -p worldwake-ai`
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
4. Waived `./scripts/verify.sh` for this per-ticket closeout because the `implement-spec-tickets` final branch phase owns the full pre-PR gate before push.

## Outcome

Completed on 2026-05-24.

- Added `crates/worldwake-ai/src/belief_status.rs` with the single crate-local `belief_status_tag_for_claim` helper.
- Removed the duplicate helper definitions from `agent_tick/frame.rs` and `agenda_manager.rs`.
- Registered the helper module in `lib.rs`.
- Added focused coverage for the status derivation branches.

## Deviations

- The focused helper test covers the derivation through an internal pure function rather than a `RuntimeBeliefView` mock. The public crate-local helper signature remains `belief_status_tag_for_claim(view: &dyn RuntimeBeliefView, agent: EntityId, claim: &EntityBeliefClaim, tick: Tick) -> BeliefStatusTag`, and both existing runtime call sites consume it.
- The ticket's drafted example used `CommodityKind::Food`; the live enum uses `CommodityKind::Bread`, so the test fixture uses `Bread`.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib belief_status::tests::derives_tag_for_each_status_class -- --exact`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
- Passed `rg -n 'fn belief_status_tag_for_claim|belief_status_tag_for_claim\(' crates/worldwake-ai/src` with one function definition and two runtime call sites.
- Waived `./scripts/verify.sh` for this per-ticket closeout because the harness final branch phase owns the full pre-PR gate before push.
