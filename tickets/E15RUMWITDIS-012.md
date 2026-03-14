# E15RUMWITDIS-012: Enforce Required Agent Information Profiles

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — required-profile access in `worldwake-systems` and supporting core helpers if needed
**Deps**: E14 (agent belief/perception baseline), E15RUMWITDIS-003 (TellProfile), E15RUMWITDIS-006 (Tell commit logic), specs/E15-rumor-witness-discovery.md

## Problem

Live agents are created with `PerceptionProfile` and `TellProfile`, so those components are intended to be required agent state, not optional hints. Current Tell code still silently falls back to `TellProfile::default()` and `PerceptionProfile::default()` when components are missing. That weakens the architecture in two ways:

1. It hides broken world state behind synthetic behavior instead of surfacing the invariant violation.
2. It makes future profile changes harder because missing data can silently re-enter behavior via defaults.

The cleaner architecture is to treat these profiles as required components for live agents and to remove behavior-level default substitution from the Tell path.

## Assumption Reassessment (2026-03-14)

1. `World::create_agent()` attaches `AgentBeliefStore`, `PerceptionProfile`, and `TellProfile` by default in `crates/worldwake-core/src/world.rs`. Missing profiles are therefore not a normal runtime case.
2. `crates/worldwake-systems/src/tell_actions.rs` currently uses `unwrap_or_else(TellProfile::default)` and `unwrap_or_else(PerceptionProfile::default)` in both validation/commit helpers.
3. `crates/worldwake-systems/src/perception.rs` already does not fabricate a fallback `PerceptionProfile`; it skips agents/witnesses that lack one. That is better than synthetic defaults, but it still leaves the invariant implicit rather than explicit.
4. No remaining active E15 ticket currently owns this cleanup. `E15RUMWITDIS-007` reads `TellProfile`, but its scope is affordance enumeration, not enforcing required-component invariants.

## Architecture Check

1. Required agent profile access should be explicit and fail loudly when violated. That is cleaner than silently substituting defaults because it preserves the meaning of authored per-agent state.
2. The cleanup should avoid scattering bespoke `ok_or_else(... InternalError ...)` code everywhere. A small shared helper or accessor layer is preferable to repeated ad hoc checks.
3. This should not introduce compatibility aliases or dual behavior paths. Once required-profile access exists, default fallbacks should be removed rather than preserved behind flags or soft modes.

## What to Change

### 1. Introduce explicit required-profile access for live agents

Add a small helper layer in the most appropriate shared location (`worldwake-core` or a narrow systems helper module) for retrieving required agent information profiles:

```rust
fn required_tell_profile(world: &WorldTxn<'_>, agent: EntityId) -> Result<TellProfile, ActionError>;
fn required_perception_profile(world: &WorldTxn<'_>, agent: EntityId) -> Result<PerceptionProfile, ActionError>;
```

If a profile is missing for a live agent, return a structured failure instead of inventing a default.

The exact helper location should be chosen to minimize duplication without widening `worldwake-core` unnecessarily.

### 2. Remove Tell fallback defaults

Update `crates/worldwake-systems/src/tell_actions.rs` so Tell validation and commit use required-profile access instead of `unwrap_or_else(...::default)`.

Expected behavior:
- If the speaker lacks `TellProfile`, authoritative validation fails.
- If the listener lacks `TellProfile` or `PerceptionProfile` at commit time, the action aborts/fails structurally rather than fabricating behavior from defaults.

### 3. Make the invariant visible in tests

Add focused tests proving that missing required profiles are surfaced as errors in Tell rather than silently replaced with defaults.

If implementation reveals a better generic invariant check for live agents, keep it narrow and explicit. Do not broaden this into a full schema-audit framework unless the code genuinely needs it.

## Files to Touch

- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-core/src/world.rs` or `crates/worldwake-core/src/world_txn.rs` (modify only if a shared helper belongs there)

## Out of Scope

- Changing agent factory defaults
- Broad component-schema validation across the entire engine
- Reworking perception mismatch tickets (`E15RUMWITDIS-008`, `E15RUMWITDIS-009`)
- AI policy changes about when agents choose to Tell
- Any backward-compatibility fallback mode

## Acceptance Criteria

### Tests That Must Pass

1. Tell authoritative validation fails if the speaker lacks `TellProfile`
2. Tell commit fails or aborts structurally if the listener lacks `TellProfile`
3. Tell commit fails or aborts structurally if the listener lacks `PerceptionProfile`
4. Tell no longer uses `TellProfile::default()` or `PerceptionProfile::default()` as a behavior fallback in the action path
5. Existing Tell tests continue to pass after the invariant cleanup
6. Existing suite: `cargo test --workspace`
7. `cargo clippy --workspace`

### Invariants

1. Live-agent information-sharing behavior is driven only by actual attached profile components, never fabricated default substitutes
2. Missing required profiles are surfaced as invariant violations, not hidden behind silent behavior fallback

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — add tests that clear `TellProfile` or `PerceptionProfile` from otherwise valid agents and assert Tell fails structurally instead of falling back
2. `crates/worldwake-systems/src/tell_actions.rs` — keep existing Tell success-path tests to prove the stricter invariant does not regress valid behavior

### Commands

1. `cargo test -p worldwake-systems tell_actions`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
