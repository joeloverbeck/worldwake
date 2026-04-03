# S44SCEPROCOM-001: Add missing Default impls for 3 universal profiles

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — 3 new Default impls in worldwake-core
**Deps**: None

## Problem

Three universal profile components (`EpistemicDispositionProfile`, `IntentionDispositionProfile`, `PreferenceProfile`) lack `Default` impls. The scenario system (ticket 002) needs `unwrap_or_default()` to guarantee these profiles exist on every spawned agent. Without Default impls, the code won't compile.

## Assumption Reassessment (2026-04-03)

1. `EpistemicDispositionProfile` at `crates/worldwake-core/src/epistemic.rs:23` — fields: `stale_evidence_barrier_threshold: Permille`, `witness_query_duration_ticks: NonZeroU32`, `ask_memory_retention_ticks: u32`. No Default impl. Confirmed via Grep.
2. `IntentionDispositionProfile` at `crates/worldwake-core/src/intention_disposition.rs:17` — fields: `domain_patience: BTreeMap<IntentionDomainTag, NonZeroU32>`, `default_patience_ticks: NonZeroU32`, `commitment_switch_margin: Permille`. No Default impl. Confirmed.
3. `PreferenceProfile` at `crates/worldwake-core/src/experience.rs:121` — fields: `route_caution_weight: Permille`, `source_trust_weight: Permille`, `route_memory_capacity: u32`, `source_memory_capacity: u32`, `memory_retention_ticks: u64`. No Default impl. Confirmed.
4. All three derive `Serialize, Deserialize` — Default impls must produce valid serializable state.
5. Ticket says the example defaults are `500/3/50` for epistemic, `20/200` for intention patience/switch margin, and `500/500/20/20/200` for preference. Live code has stronger baseline fixtures: core schema/world samples already use epistemic `400/2/12`, intention samples and golden persistence use `30/200`, and `sample_preference_profile()` uses `300/200/24/18/400`. Correction applied: use the live fixture-aligned values instead of the earlier placeholder examples. Why safe: this is a low-risk factual calibration to current code expectations, not an architecture change.
6. `NonZeroU32` fields cannot be zero — must use `NonZeroU32::new(N).unwrap()` in Default impls, not `#[derive(Default)]`.

## Architecture Check

1. Adding Default impls is the minimal foundation change. It doesn't alter any existing behavior — it only enables `unwrap_or_default()` in the scenario system (ticket 002).
2. No backwards-compatibility shims. These are new impls on existing types.

## Verification Layers

1. Default values are valid — focused unit test: `EpistemicDispositionProfile::default()` fields are within valid ranges
2. Default values are serializable — `cargo test -p worldwake-core` (serde round-trip via existing infra)
3. Single-crate change — no cross-layer verification needed

## What to Change

### 1. Add Default for EpistemicDispositionProfile

In `crates/worldwake-core/src/epistemic.rs`:

```rust
impl Default for EpistemicDispositionProfile {
    fn default() -> Self {
        Self {
            stale_evidence_barrier_threshold: Permille::new_unchecked(400),
            witness_query_duration_ticks: NonZeroU32::new(2).unwrap(),
            ask_memory_retention_ticks: 12,
        }
    }
}
```

Calibration: the live core/world fixture surface already uses `400/2/12`, so use that as the baseline instead of the earlier placeholder values.

### 2. Add Default for IntentionDispositionProfile

In `crates/worldwake-core/src/intention_disposition.rs`:

```rust
impl Default for IntentionDispositionProfile {
    fn default() -> Self {
        Self {
            domain_patience: BTreeMap::new(),
            default_patience_ticks: NonZeroU32::new(30).unwrap(),
            commitment_switch_margin: Permille::new_unchecked(200),
        }
    }
}
```

Empty `domain_patience` means all domains use `default_patience_ticks`. Live fixtures already converge on `30` ticks plus `200` switch margin, so use that as the baseline.

### 3. Add Default for PreferenceProfile

In `crates/worldwake-core/src/experience.rs`:

```rust
impl Default for PreferenceProfile {
    fn default() -> Self {
        Self {
            route_caution_weight: Permille::new_unchecked(300),
            source_trust_weight: Permille::new_unchecked(200),
            route_memory_capacity: 24,
            source_memory_capacity: 18,
            memory_retention_ticks: 400,
        }
    }
}
```

Calibration: align with `sample_preference_profile()` in `crates/worldwake-core/src/test_utils.rs`, which is the live representative fixture already used across core/sim coverage.

### 4. Add unit tests for defaults

Add a test per profile verifying:
- `Default::default()` produces valid state (no panics)
- Key fields have expected values

## Files to Touch

- `crates/worldwake-core/src/epistemic.rs` (modify) — add Default impl + test
- `crates/worldwake-core/src/intention_disposition.rs` (modify) — add Default impl + test
- `crates/worldwake-core/src/experience.rs` (modify) — add Default impl + test

## Out of Scope

- AgentDef or spawn_agent changes (ticket 002)
- Runtime enforcement (ticket 004)
- Documentation (ticket 005)
- Changing any existing profile field types or names

## Acceptance Criteria

### Tests That Must Pass

1. `EpistemicDispositionProfile::default()` produces valid state with expected field values
2. `IntentionDispositionProfile::default()` produces valid state; `domain_patience` is empty
3. `PreferenceProfile::default()` produces valid state with expected field values
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All Default impls produce valid `Permille` values (0-1000 range)
2. All `NonZeroU32` fields are nonzero
3. No existing behavior changes — these profiles are currently skipped when absent

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/epistemic.rs` — unit test for Default impl
2. `crates/worldwake-core/src/intention_disposition.rs` — unit test for Default impl
3. `crates/worldwake-core/src/experience.rs` — unit test for Default impl

### Commands

1. `cargo test -p worldwake-core -- epistemic`
2. `cargo test -p worldwake-core -- intention_disposition`
3. `cargo test -p worldwake-core -- preference`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

- **Completed**: 2026-04-03
- Added `Default` impls for `EpistemicDispositionProfile`, `IntentionDispositionProfile`, and `PreferenceProfile` in `worldwake-core`.
- Added focused unit tests in the owning core modules proving the default baselines for all three profiles.
- Deviation from the original ticket examples: the landed defaults were calibrated to live fixture and schema baselines rather than the earlier placeholder values in the initial ticket draft. The final defaults are epistemic `400/2/12`, intention `empty + 30/200`, and preference `300/200/24/18/400`.
- Verification:
  - `cargo test -p worldwake-core -- epistemic`
  - `cargo test -p worldwake-core -- intention_disposition`
  - `cargo test -p worldwake-core -- preference`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
