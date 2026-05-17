# S151TESRELROU-005: Decision-history payload embedding + observer Section 3b rendering

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — two new summary types + payload-field extensions on `GoalCommittedPayload` and `GoalSuppressedPayload` + observer rendering
**Deps**: archive/tickets/S151TESRELROU-001.md

## Problem

S151's decision history needs to surface why a goal commit involved a particular witness's testimony or a particular route segment, and why a goal was suppressed due to source unreliability. Per Q3=(a) approved during reassessment, the contexts embed as optional fields on existing payload variants (following the `BeliefSnapshot` precedent at `decision_event_payload.rs:250-254`) rather than as new top-level enum variants.

## Assumption Reassessment (2026-05-17)

1. `DecisionEventPayload` at `crates/worldwake-core/src/decision_event_payload.rs:13` has 16 always-on always-emitted variants (lines 14-30 per Step 2 spot-check). `BeliefSnapshot` at lines 250-254 is the precedent for embedded summary types — `#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]`, embedded under `PlanInvalidatedPayload:298` and `BlockerRecordedPayload:484`.
2. `GoalCommittedPayload` at `decision_event_payload.rs:159-168` already carries `#[serde(default)] pub assumptions: Vec<PlanAssumptionRef>` (line 167) from S136 — the precedent for `#[serde(default)] Vec<_>` payload extensions.
3. `GoalCommittedPayload` construction sites (13 across `decision_event_payload.rs:582,917`, `save_load.rs:1087`, `agent_tick/planning.rs:1190,4023`, `golden_decision_payload.rs:100`, `golden_motive_sources.rs:42,172`, `observer.rs:6026,7817,7922,7949`). `GoalSuppressedPayload` construction: 1 site in `agent_tick/tests.rs:5477`. All need `testimony_trust_context: vec![]` (or equivalent) additions; save-format compatibility is coordinated by ticket 010.
4. Observer Section 3b at `crates/worldwake-cli/src/bin/observer.rs:932` renders decision history as a markdown table at lines 939-940 with header `| Tick | Agent | Event | Payload Summary |`. Rendering hook at line 959 (`decision_payload_summary(payload, Some(world))`). Multi-line detail continuation pattern exists at lines 962-971 for `GoalCommitted` motive sources and `RepairApplied` breach details — new contexts follow the same continuation pattern.
5. Both summary types are core-resident; field types (`EntityId`, `TopicScope` from ticket 001, `Permille`, `Tick`, `RouteSegment` from `blocker_scope.rs:67`) all resolve to `worldwake-core` — no cross-crate-residence issue.

## Architecture Check

1. Per FND-29: decision history surfaces the *why* of commits and suppressions. Embedding contexts on existing payloads (instead of new top-level variants) avoids inflating always-on event volume — `BeliefSnapshot` precedent.
2. `Vec<_>` rather than `Option<_>` because a single goal commit may reference multiple witnesses or multiple traversed segments — vector shape matches the multi-context reality.
3. `#[serde(default)]` on every new field documents omitted-field defaults for serde-compatible struct inputs, but current bincode save-stream compatibility is not claimed by this ticket. `SAVE_FORMAT_VERSION` and any old-save omission/compatibility decision remain deferred to ticket 010 alongside the runtime-store and component bumps.
4. Both summary types are `Copy + Eq + Hash + Ord + Serialize + Deserialize` per the `BeliefSnapshot` derive set — satisfies the existing payload struct's derive requirements.

## Verified Layers

1. Payload structural extension -> `crates/worldwake-core/src/decision_event_payload.rs` unit tests construct non-empty `GoalCommittedPayload` and `GoalSuppressedPayload` contexts and assert bincode round-trip preservation.
2. Payload defaulting boundary -> RON omitted-field tests prove the new `#[serde(default)]` vector fields default empty at the serde struct boundary. Full old-save compatibility remains ticket 010 scope because bincode save streams are positional.
3. Observer rendering -> `crates/worldwake-cli/src/bin/observer.rs` bin-local unit tests assert the new Section 3b continuation rows for testimony trust and route preference contexts.

## Landed Changes

### 1. Added decision-history summary types

Added `TestimonyTrustSummary` and `RoutePreferenceSummary` beside the existing decision payload summary/read-model types in `crates/worldwake-core/src/decision_event_payload.rs`, with `Copy + Hash + Ord + Serialize + Deserialize` coverage and crate-root re-exports from `crates/worldwake-core/src/lib.rs`.

### 2. Extended payload structs

Added `#[serde(default)]` context vectors to `GoalCommittedPayload`:

```rust
pub testimony_trust_context: Vec<TestimonyTrustSummary>;
pub route_preference_context: Vec<RoutePreferenceSummary>;
```

Added `#[serde(default)] pub testimony_trust_context: Vec<TestimonyTrustSummary>` to `GoalSuppressedPayload`.

### 3. Updated explicit constructors

Updated every live explicit `GoalCommittedPayload` / `GoalSuppressedPayload` literal found by the constructor sweep and confirmed by `cargo test --workspace --no-run`. Existing producers populate the new fields with empty vectors in this ticket; tickets 006 and 007 own runtime population.

### 4. Rendered observer continuation rows

Extended observer Section 3b to render non-empty testimony trust and route preference contexts as table continuation rows below `GoalCommitted` / `GoalSuppressed` events. The summary cell stays single-line; the detailed context rows use the existing Section 3b continuation-row pattern.

## Landed Files

- `crates/worldwake-core/src/decision_event_payload.rs` (summary types, payload fields, defaulting and round-trip tests)
- `crates/worldwake-core/src/lib.rs` (crate-root re-exports)
- `crates/worldwake-sim/src/save_load.rs` (payload fixture constructor fallout)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (goal-suppressed producer constructor fallout)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (goal-committed producer and test constructor fallout)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (goal-suppressed assertion constructor fallout)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (diagnostics test constructor fallout)
- `crates/worldwake-ai/tests/golden_decision_payload.rs` (golden payload constructor fallout)
- `crates/worldwake-ai/tests/golden_motive_sources.rs` (golden payload constructor fallout)
- `crates/worldwake-cli/src/bin/observer.rs` (observer continuation rendering and bin-local tests)

## Out of Scope

- Populating `testimony_trust_context` on `GoalCommittedPayload` at commit time — ticket 006 (observation hook + planner integration)
- Populating `testimony_trust_context` on `GoalSuppressedPayload` at suppression time — ticket 007 (ranking damping + emission suppression)
- Diagnostics aggregator reading these contexts — ticket 009
- `SAVE_FORMAT_VERSION` bump — ticket 010

## Acceptance Result

### Proved Acceptance Criteria

1. `TestimonyTrustSummary` and `RoutePreferenceSummary` satisfy `Copy + Hash + Ord + Serialize + Deserialize` at compile time.
2. `GoalCommittedPayload` with non-empty contexts round-trips through bincode without loss.
3. `GoalSuppressedPayload` with non-empty testimony trust context round-trips through bincode without loss.
4. RON omitted-field tests prove serde-compatible empty defaults for all new context vectors.
5. Observer Section 3b continuation helpers produce the expected trust and route rows when contexts are non-empty.
6. Existing suite passed via `cargo test --workspace --quiet`.

### Invariants

1. `#[serde(default)]` on every added field — omitted-field defaults are explicit at the payload struct boundary; pre-bump save-stream compatibility remains ticket 010 scope.
2. Contexts are populated only by their owning tickets (006, 007); this ticket leaves all sites with `vec![]`.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/decision_event_payload.rs#[cfg(test)]` — added payload context bounds, bincode round-trip, and RON omitted-field default tests.
2. `crates/worldwake-cli/src/bin/observer.rs` — added bin-local tests for testimony trust and route preference continuation rows.

## Outcome

Completed on 2026-05-17.

- Added the S151 decision-history context schema and observer rendering surface.
- Left all context producers empty by design; tickets 006 and 007 own runtime population.
- Added the crate-root re-export because the ticket's public core exposure claim was otherwise incomplete.
- Broadened constructor fallout beyond the drafted list to include `agent_tick/mod.rs` and `scenario_diagnostics/aggregator.rs`, as found by the all-target compile sweep.
- Did not bump `SAVE_FORMAT_VERSION`; ticket 010 remains the single save-format boundary for S151's cumulative serialized state.

## Verification Result

- Passed `cargo test --workspace --no-run`.
- Passed `cargo test -p worldwake-core --lib decision_event_payload -- --list`.
- Passed `cargo test -p worldwake-cli --bin observer context_lines -- --list`.
- Passed `cargo test -p worldwake-core --lib decision_event_payload`.
- Passed `cargo test -p worldwake-cli --bin observer context_lines`.
- Passed `cargo test -p worldwake-cli observer`.
- Passed `cargo fmt --all`.
- Passed `cargo fmt --all -- --check`.
- Passed `cargo test --workspace --quiet`.
- Passed `bash scripts/check_active_goal_removed.sh`.
- Passed `bash scripts/check_no_artifact_state.sh`.
- Passed `bash scripts/check_no_debug_view_in_ai.sh`.
- Passed `cargo clippy --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
- Waived direct `./scripts/verify.sh` invocation because every live `scripts/verify.sh` gate was run individually after inspecting the wrapper.
