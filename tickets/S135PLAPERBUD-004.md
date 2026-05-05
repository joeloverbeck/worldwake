# S135PLAPERBUD-004: Discrepancy::Omission variant and revalidation wiring

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `Discrepancy` enum, hypothetical effect-sink revalidation
**Deps**: `archive/tickets/S135PLAPERBUD-001.md`, S135PLAPERBUD-002, S135PLAPERBUD-003

## Problem

Per S135 Goal 7, when an action handler revalidates against an entity the agent's belief store no longer holds AND that entity is in the agent's `ObservationOmissionLog`, the resulting `Discrepancy` must carry an attributable in-world reason (`Omission(OmissionReason)`) — not the generic `MissingObservation`. This ticket adds the new variant and wires the construction sites in `effect_sink_hypothetical.rs` that currently emit `MissingObservation` to consult the omission log first and emit `Omission(reason)` when applicable.

## Assumption Reassessment (2026-05-05)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `Discrepancy` enum lives at `crates/worldwake-core/src/discrepancy.rs:8` with 11 unit-only variants; derives `Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`. `OmissionReason` (added by ticket 001) derives `Copy` and the same compatible bounds, so `Discrepancy::Omission(OmissionReason)` payload variant preserves all existing derives.
2. Workspace-wide `Discrepancy` use sites: ~145. Most are construction `Err(Discrepancy::X)` sites in `crates/worldwake-ai/src/effect_sink_hypothetical.rs`, `crates/worldwake-systems/src/needs_actions.rs`, `crates/worldwake-systems/src/search_actions.rs`. Genuinely-exhaustive `match d { ... }` arm sites are the subset requiring new arms — confirmed match site at `crates/worldwake-ai/src/failure_handling.rs:1468` (`Discrepancy::BeliefStale => cognitive.stale_belief_backoff_ticks,` and surrounding arms). The exhaustive-match audit must enumerate every such site (via `rg 'match\s+\w+\s*\{[^}]*Discrepancy::' crates/`) and add the new arm.
3. Existing focused tests on `effect_sink_hypothetical.rs` revalidation paths exist in the file's own cfg-test block (validate during implementation by greping `#[cfg(test)]` and `#[test]` boundaries within the file). Existing tests that assert `Err(Discrepancy::MissingObservation)` are candidates for assertion-update or new-test addition: preserve the originals where the entity is genuinely unknown (not omitted), and add new tests where the entity is in `ObservationOmissionLog`.
4. Shared abstraction boundary under audit: the typed failure taxonomy carried by `Result<_, Discrepancy>` across handler revalidation, hypothetical effect-sink, and AI replan/recovery paths. The new variant must be addable without breaking ranking-sensitive `Ord`-derived arithmetic in `failure_handling.rs::compute_backoff_for_discrepancy` (or the equivalently-named function around line 1468).
5. AI-side reads of `ObservationOmissionLog` route through `GoalBeliefView::observation_omission_log` (added in ticket 002).

## Architecture Check

1. The new variant is introduced as a payload-bearing variant `Discrepancy::Omission(OmissionReason)`. `OmissionReason: Copy` keeps the enum's `Copy` derive intact — no `Clone` regression at construction sites.
2. The `Ord` derive needs an explicit ordering decision: `Omission` is placed at the end of the variant list so its `Ord` value is greater than every existing variant. Verify against `failure_handling.rs` backoff arithmetic during implementation — if the backoff lookup is by exhaustive match (not by `Ord`-driven comparison), placement order is informational.
3. No backward-compatibility shim — every exhaustive match gets a new arm; missing arms are compile errors.
4. Construction sites in `effect_sink_hypothetical.rs` consult `ObservationOmissionLog` via `GoalBeliefView::observation_omission_log` (FND-26 compliant; no direct world reads from AI).
5. **Information-path refactor**: pre-ticket, the same fact ("this entity is missing because perception dropped it") had no transport path — the failure was untyped at the `MissingObservation` level. Post-ticket, the typed reason travels via `Discrepancy::Omission(reason)` only. No legacy alias path remains.

## Verification Layers

1. `Discrepancy::Omission(OmissionReason)` round-trips serialization → focused unit test in `discrepancy.rs` cfg-test block.
2. Exhaustive match arms compile → workspace build is the test.
3. Revalidation against an omitted entity returns `Err(Discrepancy::Omission(reason))` instead of `Err(Discrepancy::MissingObservation)` → focused unit test in `effect_sink_hypothetical.rs` cfg-test block.
4. Revalidation against a never-observed entity (not in log) still returns `Err(Discrepancy::MissingObservation)` → focused unit test ensuring the new path doesn't subsume the old one.
5. Backoff arithmetic for the new arm matches the spec's intent (likely sharing `MissingObservation`'s window) → focused unit test in `failure_handling.rs` cfg-test block.

## What to Change

### 1. Add `Discrepancy::Omission(OmissionReason)` variant

In `crates/worldwake-core/src/discrepancy.rs:8`, add the new variant at the end of the enum (after `NeedHorizonExceeded`). Derives intact (`Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`). Doc comment: "The agent could not revalidate against an entity that perception had dropped from the belief store under the given salience-budget reason."

### 2. Update exhaustive match sites

Workspace-wide grep for `match` expressions on `Discrepancy` (`rg 'match\s+\w+\s*\{[^}]*Discrepancy::' crates/`). Confirmed site: `crates/worldwake-ai/src/failure_handling.rs:1468` (the surrounding match arms compute backoff per discrepancy). Add a `Discrepancy::Omission(reason) => ...` arm with the contextually appropriate value (likely sharing `MissingObservation`'s backoff window since the failure mode is similar — confirm during implementation against the spec's intent).

For any other exhaustive match sites discovered during the workspace grep, add the new arm with the contextually appropriate value.

### 3. Wire construction sites in effect-sink revalidation

In `crates/worldwake-ai/src/effect_sink_hypothetical.rs`, for each site that currently emits `Err(Discrepancy::MissingObservation)` because an expected entity is absent from the agent's belief store (sites at lines 112, 117, 120, 125, 165, 182, 187, 191, 205, 208, 348, 367, 500, etc., per `rg 'Discrepancy::MissingObservation' crates/worldwake-ai/src/effect_sink_hypothetical.rs`):

1. Consult `view.observation_omission_log(agent)` (ticket 002's accessor).
2. If the missing entity has an entry in the log within an activation-fresh tick window (use the same activation horizon the surrounding revalidation logic already references — likely `S101`'s `entity_activation_threshold` proxy), emit `Err(Discrepancy::Omission(entry.reason))` instead.
3. Otherwise preserve the existing `MissingObservation` return.

The "recent" threshold matches the activation-decay window (S101) — entries older than that horizon are stale and treated as plain missing observation (the agent has had time to re-perceive or forget).

### 4. Update existing tests

In `effect_sink_hypothetical.rs` cfg-test block: existing tests that assert `Err(Discrepancy::MissingObservation)` for entities the agent never observed remain unchanged (the entity is not in `ObservationOmissionLog`). Add new tests covering the omission-attribution path.

In `failure_handling.rs` cfg-test block (or the file owning the backoff arithmetic): add a test for the new arm's backoff value.

## Files to Touch

- `crates/worldwake-core/src/discrepancy.rs` (modify) — variant addition
- `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (modify) — construction site wiring at the ~13 `MissingObservation` sites
- `crates/worldwake-ai/src/failure_handling.rs` (modify) — match arm at line 1468
- Likely: `crates/worldwake-systems/src/needs_actions.rs`, `crates/worldwake-systems/src/search_actions.rs` — any exhaustive match arms (grep `rg 'match\s+\w+\s*\{[^}]*Discrepancy::' crates/worldwake-systems/` during reassessment to confirm)
- Likely: any test file asserting on the full `Discrepancy` value with pattern syntax (grep `rg 'assert_eq!\([^)]*Discrepancy::' crates/` during reassessment to confirm)

## Out of Scope

- Adding `OmissionReason` `Copy` constraint — already done in ticket 001.
- Reading `ObservationOmissionLog` directly from world from AI — must route through ticket 002's `GoalBeliefView` accessor.
- Surfacing the omission attribution on `RootCandidateTrace` → ticket 005.
- Surfacing the omission attribution in observer reports → ticket 006.
- Goldens → ticket 007.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core --lib discrepancy` passes (variant round-trip + `Ord` ordering).
2. `cargo test -p worldwake-ai --lib effect_sink_hypothetical` passes (revalidation against omitted entity returns typed reason; revalidation against never-observed entity still returns `MissingObservation`).
3. `cargo test -p worldwake-ai --lib failure_handling` passes (backoff arithmetic for new arm).
4. `cargo build --workspace` succeeds — no missing match arms anywhere in the workspace.

### Invariants

1. `Discrepancy` retains `Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize` derives (compile-time invariant).
2. Revalidation against an entity in `ObservationOmissionLog` (within activation horizon) returns `Err(Discrepancy::Omission(reason))`, where `reason` matches the log entry's `OmissionReason`.
3. Revalidation against an entity never observed returns `Err(Discrepancy::MissingObservation)` unchanged.
4. Revalidation against an entity in the log but past the activation horizon returns `Err(Discrepancy::MissingObservation)` (the omission is too stale to attribute).
5. Every exhaustive match on `Discrepancy` in the workspace accounts for `Omission` (compile-time invariant).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/discrepancy.rs` cfg-test block — new test: `Discrepancy::Omission(OmissionReason::OverBudget { budget: 5, candidates_seen: 10 })` round-trips through serde; `Ord` ordering places `Omission` after `NeedHorizonExceeded`.
2. `crates/worldwake-ai/src/effect_sink_hypothetical.rs` cfg-test block — new test: agent with `ObservationOmissionLog` entry for entity X (within activation horizon), attempt hypothetical revalidation against X, assert `Err(Discrepancy::Omission(reason))` with the entry's `OmissionReason`.
3. `crates/worldwake-ai/src/effect_sink_hypothetical.rs` cfg-test block — new test: agent with empty omission log, attempt revalidation against unknown entity, assert `Err(Discrepancy::MissingObservation)` unchanged.
4. `crates/worldwake-ai/src/effect_sink_hypothetical.rs` cfg-test block — new test: agent with a stale omission entry (past activation horizon), attempt revalidation, assert `Err(Discrepancy::MissingObservation)` (not `Omission`).
5. `crates/worldwake-ai/src/failure_handling.rs` cfg-test block — new test for the new match arm's backoff value at line 1468.

### Commands

1. `cargo test -p worldwake-core --lib discrepancy`
2. `cargo test -p worldwake-ai --lib`
3. `cargo build --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh`
