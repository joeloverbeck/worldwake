# S151TESRELROU-010: SAVE_FORMAT_VERSION bump 87 → 88

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — single-line `SAVE_FORMAT_VERSION` constant bump and accompanying save/load coverage
**Deps**: archive/tickets/S151TESRELROU-002.md, archive/tickets/S151TESRELROU-003.md, archive/tickets/S151TESRELROU-005.md

## Problem

S151's D13 required a `SAVE_FORMAT_VERSION` bump from 87 to 88 to cover the cumulative serialized-state additions across tickets 002 (runtime store fields on `AgentDecisionRuntime`), 003 (universal components on `EntityKind::Agent`), and 005 (payload context fields on `GoalCommittedPayload` / `GoalSuppressedPayload`). Ticket 002 proved format-88 bincode round-trip and default initialization, not old bincode save compatibility. This ticket coordinated the single visible save-format boundary and documented the intentional no-migration boundary for version-87 bytes.

## Assumption Reassessment (2026-05-17)

1. `SAVE_FORMAT_VERSION` lived at `crates/worldwake-sim/src/save_load.rs:6` and was `87` before this ticket. It is used at line 101 (`bytes.extend_from_slice(&SAVE_FORMAT_VERSION.to_le_bytes())`) for writes and line 129 (`SAVE_FORMAT_VERSION => load_current_format(payload)`) for reads.
2. Per Step 2 spot-check (b): `SAVE_FORMAT_VERSION` had not changed since the reassessment; bump target was `87 → 88` exactly.
3. The cumulative new serialized state covers:
   - Ticket 002: `AgentDecisionRuntime.testimony_reliability: TestimonyReliability` and `AgentDecisionRuntime.route_preference: RoutePreference` (current-format round-trip and `AgentDecisionRuntime::default()` empty-store behavior proved; old bincode save loading remains ticket-010 scope).
   - Ticket 003: `TestimonyTrustProfile` and `RoutePreferenceProfile` components on `EntityKind::Agent` (bootstrap-seeded by `World::create_agent`; old saves load into Default values via the component-registration replay path).
   - Ticket 005: `GoalCommittedPayload.testimony_trust_context: Vec<_>`, `GoalCommittedPayload.route_preference_context: Vec<_>`, `GoalSuppressedPayload.testimony_trust_context: Vec<_>` (payload omitted-field defaults are explicit, but bincode save-stream compatibility must be verified or documented here).
4. The bump is the official "save format now includes S151 state" boundary. Do not assume Default-derived or `#[serde(default)]` fields make pre-87 bincode save streams loadable without a fixture.
5. Load-migration code is still expected to be unnecessary unless the version-87 fixture proves otherwise. If old bincode bytes cannot deserialize after the shape change, record that as the intentional FND-28 no-backward-compatibility boundary rather than adding a shim.

## Architecture Check

1. Per CLAUDE.md Determinism: bumping the format constant in a single location preserves the single-source-of-truth invariant for save format version.
2. Per FND-28: no shim, no dual-format support. The bump is the natural format boundary after S151's runtime, component, and payload state lands.
3. Per FND-12 (performance compresses computation, never causality): version-88 saves must preserve populated S151 state byte-for-byte through save/load. Version-87 behavior must be either proved loadable into legal default S151 state or documented as intentionally unsupported per FND-28.
4. Single-ticket bump (rather than per-deliverable cascade) keeps the format-version surface coherent — reviewers see one bump for the whole S151 surface, not three independent bumps that would force a cascade discipline.

## Verified Layers

1. Constant value → `save_format_version_is_88_after_s151_state_landing` and a direct grep prove `SAVE_FORMAT_VERSION == 88`.
2. Round-trip → `save_to_bytes_roundtrip_preserves_full_nondefault_state`, `save_to_bytes_roundtrip_preserves_decision_event_payloads`, and `save_to_bytes_roundtrip_preserves_runtime_payload` prove version-88 save/load preserves the S151 world/profile fields, decision-event context fields, and opaque runtime payload bytes.
3. Version-boundary probe → `load_rejects_pre_s151_version_87_without_migration_shim` documents the intentional no-migration boundary: version-87 bytes are rejected with `UnsupportedVersion`, not shimmed.

## Landed Changes

### 1. Bumped the constant (`crates/worldwake-sim/src/save_load.rs:6`)

```rust
pub const SAVE_FORMAT_VERSION: u32 = 88;
```

### 2. Verified load path

Confirmed `load_current_format` at `save_load.rs:129` handles version-88 saves with the new fields. Version-87 bincode bytes are intentionally rejected instead of loaded through default semantics.

### 3. Save/load round-trip coverage

Updated focused unit coverage in `crates/worldwake-sim/src/save_load.rs#[cfg(test)]`:

- `save_to_bytes_roundtrip_preserves_full_nondefault_state` now asserts non-default `TestimonyTrustProfile` and `RoutePreferenceProfile` values survive the full state round-trip.
- `save_to_bytes_roundtrip_preserves_decision_event_payloads` now uses and asserts non-empty S151 `testimony_trust_context` and `route_preference_context` payloads.
- `save_to_bytes_roundtrip_preserves_runtime_payload` continues to prove the sim save layer preserves opaque runtime bytes losslessly; the concrete AI runtime S151 store bincode contract remains covered by `worldwake-ai` runtime tests from ticket 002.

### 4. Version-boundary regression

Added the version-boundary test:

```rust
#[test]
fn load_rejects_pre_s151_version_87_without_migration_shim() {
    // version-88 save bytes with header rewritten to 87 are rejected as unsupported
}
```

This records the intentional no-migration boundary under the repository's no-backward-compatibility policy.

## Landed Files

- `crates/worldwake-sim/src/save_load.rs` (modify — single-line constant bump + new tests)

## Out of Scope

- Runtime store additions — ticket 002
- Universal profile additions — ticket 003
- Payload context additions — ticket 005
- Any production-code migration shim unless the user explicitly changes the no-backward-compatibility policy

## Acceptance Criteria

### Acceptance Results

1. `SAVE_FORMAT_VERSION == 88` after the bump.
2. Round-trip test: populated S151 state survives save → load → equality check.
3. Version-boundary test/probe: pre-bump save loads into default S151 state, or the no-migration outcome is documented with rationale.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. Save format version is a single integer at a single source location.
2. Saves written by version-88 binaries load by version-88 binaries with byte equality preserved.
3. Version-87 save behavior is explicitly proved or explicitly documented as unsupported under the no-backward-compatibility policy.

## Verification Result

### Modified Tests

- Passed: `crates/worldwake-sim/src/save_load.rs#[cfg(test)]` now covers the version-88 constant, S151 profile and decision payload state in save/load round-trips, opaque runtime bytes, and version-87 rejection.

### Commands Passed

- Passed: `cargo fmt --all`
- Passed: `cargo test -p worldwake-sim save_load`
- Passed: `cargo test --workspace`
- Passed: `cargo clippy --workspace --all-targets -- -D warnings`
- Passed: `./scripts/verify.sh`

Merge note: This ticket bumps SAVE_FORMAT_VERSION 87→88 as a single-coordinated step after tickets 002, 003, and 005 land their serialized state. No sibling tickets bump the version; do not infer old bincode save compatibility from `#[serde(default)]` alone.

## Outcome

Completed: 2026-05-17

`SAVE_FORMAT_VERSION` is now `88`. The save/load tests now prove version-88 bytes preserve S151's world/profile and decision-event payload state, while the sim save layer continues to preserve opaque runtime bytes losslessly. The version-87 boundary is intentionally unsupported and is covered by `load_rejects_pre_s151_version_87_without_migration_shim`, matching the repo's no-backward-compatibility policy.

Deviation from the draft: this ticket does not deserialize a real pre-S151 version-87 bincode fixture into default S151 state. The landed contract is the intentional no-migration branch: version-87 save bytes are rejected by the version header before bincode payload interpretation.
