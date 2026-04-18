# S108PERACTBIN-002: Strictness gate at `resolve_affordance` with `ExactIdentityRequired` rejection

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `resolve_affordance` refuses BestEffort synthesis when the action is `ExactIdentity` and the exact target was not reproduced; new `RequestResolutionRejectionReason::ExactIdentityRequired` variant; the rejection maps through `ActionStartFailureReason::from_action_error` to `BlockingFact::AssumptionFailed` pre-S109.
**Deps**: archive/tickets/S108PERACTBIN-001.md (needs `BindingStrictness`, `StrictnessGate`, `check_binding_strictness`, classified `ActionDef`s).

## Problem

After S108PERACTBIN-001 lands the strictness metadata and predicate, the actual BestEffort permissiveness is still in force: `crates/worldwake-sim/src/tick_step.rs::resolve_affordance` (lines 468–522) synthesizes a fresh `Affordance` from raw requested targets whenever `requested_affordance_matches` finds no existing affordance and mode is `BestEffort`. For `ExactIdentity` actions whose exact target has moved, died, or been unstaged, this silently substitutes a different entity at the same place — exactly the failure mode S108 exists to prevent (FND-4, FND-14, FND-21).

This ticket wires the strictness gate into `resolve_affordance`, extends `RequestResolutionRejectionReason` with an `ExactIdentityRequired` variant, and threads that rejection through the existing start-failure path to the pre-S109 `BlockingFact::AssumptionFailed` classification.

## Assumption Reassessment (2026-04-18)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `resolve_affordance` is a private function in `crates/worldwake-sim/src/tick_step.rs` at line 468–522. Its BestEffort synthesis arm is at line 504–514. It returns `Result<ResolvedRequest, RequestResolutionRejectionReason>`. Existing `#[cfg(test)]` tests exercising this function include `resolve_affordance_uses_shared_request_binding_rule` at line 2063 and the `RequestResolutionRejectionReason::NoMatchingAffordance` assertions at lines 1994 and 2111. The new rejection variant must be added as a distinct case in all exhaustive match sites.
2. `RequestResolutionRejectionReason` is defined at `crates/worldwake-sim/src/request_resolution_trace.rs:56` as a serializable enum. `RequestResolutionTraceEvent` at line 45 carries it in the trace. The variant is re-exported from `crates/worldwake-sim/src/lib.rs:134`.
3. Shared abstraction boundary: the request-resolution layer (`resolve_affordance` → `RequestResolutionOutcome::RejectedBeforeStart` → `TickStepError::RequestedAffordanceUnavailable`). This ticket adds a typed reason at the earliest failure boundary, per precision rule #9 (stale-request / start-failure boundaries).
4. Not applicable — no failing golden motivates this ticket directly; S108's design is the motivation.
5. Not applicable — not a planner- or golden-driven ticket.
6. AI regression intended layer: request resolution / affordance reproduction (runtime dispatch, not candidate generation). Harness: local `TickStepServices` test fixtures are sufficient; full action registries are not required for the unit-level gate proof. Goldens are exercised in T-005.
7. Not applicable — no ordering claim.
8. Not applicable — no heuristic removal. The gate adds a substrate (authorial binding contract), it does not bypass an existing one.
9. First failure boundary for `ExactIdentityRequired`: request resolution / affordance reproduction (precision rule #9). The gate fires in `resolve_affordance` before `scheduler.start_affordance` is called, so action-start and post-start-abort lifecycles are unaffected. Shared runtime symbols checked: `resolve_affordance` (`tick_step.rs:468`), `RequestResolutionRejectionReason` (`request_resolution_trace.rs:56`), `TickStepError::RequestedAffordanceUnavailable` (`tick_step.rs:287`), `ActionStartFailureReason::from_action_error` (`tick_step.rs:330`).
10. Not applicable — no political office claim.
11. Not applicable — no `ControlSource` manipulation.
12. Not applicable — no golden isolation here (goldens live in T-005).
13. Adjacent observation during reassessment: `RequestResolutionRejectionReason::NoMatchingAffordance` is already emitted for Strict-mode dispatch when no affordance matches. `ExactIdentityRequired` is a distinct class — it names the *reason* the BestEffort fallback refused, not the absence of enumeration matches. Classifying it as a sibling variant (rather than a subtype of `NoMatchingAffordance`) preserves that distinction and feeds S109's `Discrepancy::NoLegalBinding` cleanly.
14. No mismatch discovered. Reassessment-session corrections already landed in the spec.
15. Not applicable — no cumulative arithmetic.

## Architecture Check

1. The gate lives at the first failure boundary (request resolution), which is the cleanest architectural surface: failure occurs before any scheduler state is mutated, before any action instance is created, before any event is recorded. Alternative placements — inside `requested_affordance_matches`, inside `scheduler.start_affordance`, or post-start in an abort handler — would either widen an existing pure predicate's semantics (`requested_affordance_matches` stays pure per the reassessed spec) or defer the rejection past points where partial state has been mutated.
2. No backward-compatibility shim (FND-28). The existing `BlockingFact::AssumptionFailed` is reused as the interim bucket until S109's typed discrepancy taxonomy lands; the `ExactIdentityRequired` variant is a new authoritative signal, not an alias for an old one.

## Verification Layers

1. Request-resolution refusal for `ExactIdentity` + BestEffort + target gone -> focused runtime request-resolution coverage (new test in `tick_step.rs` under `#[cfg(test)]`).
2. Rejection reason round-trips through the request-resolution trace -> `RequestResolutionTraceEvent` assertion in the same test.
3. Start-failure classification for the rejection -> assert `ActionStartFailureReason::from_action_error` maps the resulting `TickStepError` path to `BlockingFact::AssumptionFailed`.
4. AI recovery / blocker reconciliation downstream of the rejection is deferred to T-005 (goldens) — the recovery chain is not this ticket's contract; the first failure boundary is (precision rule #9).

## What to Change

### 1. Extend `RequestResolutionRejectionReason`

`crates/worldwake-sim/src/request_resolution_trace.rs:56` — add a new variant:

```rust
pub enum RequestResolutionRejectionReason {
    UnknownActionDef,
    MissingHandler,
    NoMatchingAffordance,
    ExactIdentityRequired,   // NEW
}
```

Update `impl`s, `Serialize`/`Deserialize` derives (automatic), and any exhaustive matches in the crate (`Display`, trace rendering, etc.).

### 2. Gate the BestEffort synthesis in `resolve_affordance`

`crates/worldwake-sim/src/tick_step.rs` lines 502–516. Current code:

```rust
let (mut affordance, binding) = match reproduced {
    Some(affordance) => (affordance, RequestBindingKind::ReproducedAffordance),
    None if mode == crate::ActionRequestMode::BestEffort => (
        crate::Affordance { def_id, actor, bound_targets: targets.to_vec(), ... },
        RequestBindingKind::BestEffortFallback,
    ),
    None => return Err(RequestResolutionRejectionReason::NoMatchingAffordance),
};
```

Replace the `None if mode == BestEffort` arm with:

```rust
None if mode == crate::ActionRequestMode::BestEffort => {
    match crate::check_binding_strictness(def, mode) {
        crate::StrictnessGate::ExactIdentityRequired => {
            return Err(RequestResolutionRejectionReason::ExactIdentityRequired);
        }
        crate::StrictnessGate::SubstitutionAllowed(_class) => (
            crate::Affordance {
                def_id,
                actor,
                bound_targets: targets.to_vec(),
                payload_override: payload_override.clone(),
                explanation: None,
                contention_status: worldwake_core::ContentionStatus::Unmanaged,
            },
            RequestBindingKind::BestEffortFallback,
        ),
    }
}
```

The `_class` binding is intentional — T-004 populates the trace; this ticket keeps the class invisible at the sim layer beyond the gate itself.

### 3. Map `ExactIdentityRequired` to a start-failure reason

`crates/worldwake-sim/src/tick_step.rs` — the `input_action` arm that calls `resolve_affordance` (around line 270) handles rejection by converting it into a `RequestResolutionOutcome::RejectedBeforeStart { reason }` trace entry and returning `TickStepError::RequestedAffordanceUnavailable`. No structural change is needed at this level — the new variant follows the existing path.

However, `ExactIdentityRequired` rejections happen before `scheduler.start_affordance` is called, so `is_best_effort_start_failure` at line 458 (which maps `ActionError` variants emitted *by* the scheduler) does not apply. Instead, the caller recovers via the existing `RejectedBeforeStart` handling — no new mapping is needed in `ActionStartFailureReason::from_action_error`.

Verify (precision rule #9): the request-resolution layer already traces the rejection via `record_request_resolution_trace` with outcome `RejectedBeforeStart`. The downstream `BlockingFact::AssumptionFailed` equivalence surfaces through `handle_plan_failure` after the AI observes the `TickStepError::RequestedAffordanceUnavailable` return. T-005 verifies this chain end-to-end via goldens; this ticket proves the first boundary.

### 4. Unit tests in `tick_step.rs`

Add `#[cfg(test)]` tests covering:
- `resolve_affordance` with a BestEffort request against an action classified `ExactIdentity` whose target has been removed → returns `Err(RequestResolutionRejectionReason::ExactIdentityRequired)`.
- `resolve_affordance` with a BestEffort request against an action classified `FungibleEquivalentCommodity` where the exact item is gone but the enumeration still yields a match → returns `Ok(ResolvedRequest { binding: ReproducedAffordance, .. })` (unchanged behavior).
- `resolve_affordance` with a BestEffort request against an action classified `FungibleEquivalentCommodity` where no affordance enumerates but the fallback is permitted → returns `Ok(ResolvedRequest { binding: BestEffortFallback, .. })`.
- Strict mode + `ExactIdentity` + no match → returns `Err(RequestResolutionRejectionReason::NoMatchingAffordance)` (gate does not fire in Strict mode).

### 5. Update affected exhaustive matches

Grep for `RequestResolutionRejectionReason::` across `crates/` to find every match site. Known sites:
- `tick_step.rs:1994, 2111` (tests) — extend with new-variant cases.
- `request_resolution_trace.rs` — any `Display` or rendering matches.

## Files to Touch

- `crates/worldwake-sim/src/request_resolution_trace.rs` (modify — add variant, update renderers)
- `crates/worldwake-sim/src/tick_step.rs` (modify — gate insertion, new unit tests)
- `crates/worldwake-sim/src/lib.rs` (modify — verify `StrictnessGate` and `check_binding_strictness` are re-exported, added in T-001)

## Out of Scope

- Plan-revalidation gate — T-003.
- Decision trace field — T-004.
- Golden/integration tests — T-005.
- S109 typed discrepancy refinement (`Discrepancy::NoLegalBinding`) — separate spec.
- Changes to `requested_affordance_matches` signature — intentionally preserved per reassessed spec.

## Acceptance Criteria

### Tests That Must Pass

1. New unit tests in `crates/worldwake-sim/src/tick_step.rs` covering the four `resolve_affordance` cases above.
2. Existing `resolve_affordance_uses_shared_request_binding_rule` test (line 2063) continues to pass after the gate addition.
3. Existing `RequestResolutionRejectionReason::NoMatchingAffordance` assertions at lines 1994 and 2111 continue to pass — Strict mode behavior is unchanged.
4. Existing suite: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. `resolve_affordance` never synthesizes a substitute affordance for an action whose `binding_strictness` is `ExactIdentity` when mode is `BestEffort`.
2. The rejection surfaces through the existing `RequestResolutionOutcome::RejectedBeforeStart` trace path with reason `ExactIdentityRequired`, preserving trace fidelity (FND-29).
3. Strict-mode behavior is unchanged: `check_binding_strictness` returns `SubstitutionAllowed` for Strict regardless of class, so Strict requests still fail with `NoMatchingAffordance` when enumeration produces no match.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_step.rs` — four new `#[cfg(test)]` cases on `resolve_affordance` covering the gate behavior per class/mode combinations.

### Commands

1. `cargo test -p worldwake-sim tick_step::tests::resolve_affordance`
2. `cargo test -p worldwake-sim`
3. `cargo build --workspace && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
