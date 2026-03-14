# E15RUMWITDIS-012: Enforce Required Agent Information Components In Tell

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — required information-component access in `worldwake-systems` and supporting core helpers if needed
**Deps**: E14 (agent belief/perception baseline), E15RUMWITDIS-003 (TellProfile), E15RUMWITDIS-006 (Tell commit logic), specs/E15-rumor-witness-discovery.md

## Problem

Live agents are created with `PerceptionProfile` and `TellProfile`, so those components are intended to be required agent state, not optional hints. Current Tell code still silently falls back to `TellProfile::default()` and `PerceptionProfile::default()` when components are missing. That weakens the architecture in two ways:

1. It hides broken world state behind synthetic behavior instead of surfacing the invariant violation.
2. It makes future profile changes harder because missing data can silently re-enter behavior via defaults.

That problem is broader than profiles alone: the Tell commit path also fabricates a missing listener `AgentBeliefStore` with `unwrap_or_default()`, even though live agents are created with a belief store as part of their required information state. The cleaner architecture is to treat Tell's live-agent information components as required and to remove behavior-level default substitution from the authoritative Tell path.

## Assumption Reassessment (2026-03-14)

1. `World::create_agent()` attaches `AgentBeliefStore`, `PerceptionProfile`, and `TellProfile` by default in `crates/worldwake-core/src/world.rs`. Missing information components are therefore not a normal runtime case for live agents.
2. `crates/worldwake-systems/src/tell_actions.rs` currently uses `unwrap_or_else(TellProfile::default)` and `unwrap_or_else(PerceptionProfile::default)` in the authoritative Tell path, and it also uses `unwrap_or_default()` to fabricate a missing listener `AgentBeliefStore` during commit.
3. `crates/worldwake-systems/src/perception.rs` already does not fabricate a fallback `PerceptionProfile`; it skips agents or witnesses that lack one. That is better than synthetic defaults, but it still leaves the broader required-component invariant implicit rather than explicit.
4. Planner-side Tell affordance reads also still leak defaults through `RuntimeBeliefView` consumers (`enumerate_tell_payloads`, `PerAgentBeliefView`, and `planning_snapshot`). That is a real architectural issue, but it is a separate boundary because the current affordance-query API cannot surface structural invariant failures as `Result`.
5. No remaining active E15 ticket currently owns the authoritative Tell cleanup. `E15RUMWITDIS-007` reads `TellProfile`, but its scope is affordance enumeration, not required-component enforcement.

## Architecture Check

1. Required Tell-path information component access should be explicit and fail loudly when violated. That is cleaner than silently substituting defaults because it preserves the meaning of authored per-agent state.
2. The cleanup should avoid scattering bespoke `ok_or_else(... InternalError ...)` code everywhere. A small shared helper or accessor layer is preferable to repeated ad hoc checks.
3. This should not introduce compatibility aliases or dual behavior paths. Once required-component access exists, default fallbacks should be removed rather than preserved behind flags or soft modes.
4. This ticket should stay on the authoritative Tell path where structural failures can be represented directly. A broader affordance-planning invariant pass belongs in a follow-up ticket unless the implementation here exposes a clean, narrow hook.

## What to Change

### 1. Introduce explicit required-component access for live agents in the authoritative Tell path

Add a small helper layer in the most appropriate shared location (`worldwake-core` or a narrow systems helper module) for retrieving the required information components that Tell uses from live agents:

```rust
fn required_belief_store(world: &WorldTxn<'_>, agent: EntityId) -> Result<AgentBeliefStore, ActionError>;
fn required_tell_profile(world: &WorldTxn<'_>, agent: EntityId) -> Result<TellProfile, ActionError>;
fn required_perception_profile(world: &WorldTxn<'_>, agent: EntityId) -> Result<PerceptionProfile, ActionError>;
```

If one of these components is missing for a live agent, return a structured failure instead of inventing a default.

The exact helper location should be chosen to minimize duplication without widening `worldwake-core` unnecessarily.

### 2. Remove authoritative Tell fallback defaults

Update `crates/worldwake-systems/src/tell_actions.rs` so Tell validation and commit use required-component access instead of `unwrap_or_else(...::default)` / `unwrap_or_default()`.

Expected behavior:
- If the speaker lacks `TellProfile`, authoritative validation fails.
- If the speaker lacks `AgentBeliefStore` at commit time, the action fails structurally rather than silently no-oping.
- If the listener lacks `AgentBeliefStore`, `TellProfile`, or `PerceptionProfile` at commit time, the action fails structurally rather than fabricating behavior from defaults.

### 3. Preserve current planner-boundary scope

Do not widen this ticket into a full `RuntimeBeliefView` / planning-snapshot redesign unless the implementation reveals a very small, obviously correct change. The planner-side TellProfile default leak is real, but it should only be absorbed here if it can be fixed without broad API churn.

### 4. Make the invariant visible in tests

Add focused tests proving that missing required information components are surfaced as errors in Tell rather than silently replaced with defaults.

If implementation reveals a better generic invariant check for live agents, keep it narrow and explicit. Do not broaden this into a full schema-audit framework unless the code genuinely needs it.

## Files to Touch

- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-core/src/world.rs` or `crates/worldwake-core/src/world_txn.rs` (modify only if a shared helper belongs there)
- `crates/worldwake-systems/tests/` (modify only if Tell integration coverage needs to move out of the unit test module)

## Out of Scope

- Changing agent factory defaults
- Broad component-schema validation across the entire engine
- Reworking perception mismatch tickets (`E15RUMWITDIS-008`, `E15RUMWITDIS-009`)
- AI policy changes about when agents choose to Tell
- Any backward-compatibility fallback mode

## Acceptance Criteria

### Tests That Must Pass

1. Tell authoritative validation fails if the speaker lacks `TellProfile`
2. Tell commit fails structurally if the speaker lacks `AgentBeliefStore`
3. Tell commit fails structurally if the listener lacks `AgentBeliefStore`
4. Tell commit fails or aborts structurally if the listener lacks `TellProfile`
5. Tell commit fails or aborts structurally if the listener lacks `PerceptionProfile`
6. Tell no longer uses `TellProfile::default()`, `PerceptionProfile::default()`, or `AgentBeliefStore::default()` / `new()` as an authoritative Tell-path fallback
7. Existing Tell tests continue to pass after the invariant cleanup
8. Existing suite: `cargo test --workspace`
9. `cargo clippy --workspace`

### Invariants

1. Live-agent Tell behavior is driven only by actual attached information components, never fabricated default substitutes
2. Missing required Tell-path information components are surfaced as invariant violations, not hidden behind silent behavior fallback

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — add tests that clear `TellProfile`, `PerceptionProfile`, or `AgentBeliefStore` from otherwise valid agents and assert Tell fails structurally instead of falling back
2. `crates/worldwake-systems/src/tell_actions.rs` — keep existing Tell success-path tests to prove the stricter invariant does not regress valid behavior

### Commands

1. `cargo test -p worldwake-systems tell_actions`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - Corrected the ticket scope before implementation: the invariant leak was broader than profiles alone because Tell also fabricated a missing listener `AgentBeliefStore`.
  - Hardened the authoritative Tell path so validation rejects a missing speaker `TellProfile`, and commit now fails structurally if required live-agent Tell components (`AgentBeliefStore`, `TellProfile`, `PerceptionProfile`) are missing.
  - Added focused Tell tests covering missing speaker/listener components and retained the existing success-path coverage.
- Deviations from original plan:
  - The implementation stayed local to `crates/worldwake-systems/src/tell_actions.rs`; no shared core helper was necessary once the duplication was measured.
  - Planner-side `TellProfile` default leakage through `RuntimeBeliefView` consumers was confirmed during reassessment, but it remains a follow-up because that API currently cannot surface structural invariant failures cleanly.
- Verification results:
  - `cargo test -p worldwake-systems tell_actions`
  - `cargo test -p worldwake-systems`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
