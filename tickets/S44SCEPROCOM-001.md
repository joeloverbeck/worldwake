# S44SCEPROCOM-001: Add missing Default impls for 3 universal profiles

**Status**: PENDING
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
5. Golden tests that use these profiles set explicit values. Default values should be reasonable baselines that don't break existing behavior — agents without these profiles currently get no behavior from the corresponding systems (they're skipped via `if let Some(...)`). The defaults define "normal agent" baseline.
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
            stale_evidence_barrier_threshold: Permille::new_unchecked(500),
            witness_query_duration_ticks: NonZeroU32::new(3).unwrap(),
            ask_memory_retention_ticks: 50,
        }
    }
}
```

Calibration: check golden test setups that construct `EpistemicDispositionProfile` to find typical values. Use those as the baseline. The values above are reasonable starting points — adjust during implementation if golden tests use different values.

### 2. Add Default for IntentionDispositionProfile

In `crates/worldwake-core/src/intention_disposition.rs`:

```rust
impl Default for IntentionDispositionProfile {
    fn default() -> Self {
        Self {
            domain_patience: BTreeMap::new(),
            default_patience_ticks: NonZeroU32::new(20).unwrap(),
            commitment_switch_margin: Permille::new_unchecked(200),
        }
    }
}
```

Empty `domain_patience` means all domains use `default_patience_ticks`. Calibrate `commitment_switch_margin` against `ReasoningProfile::default().switch_margin` to ensure the two-tier precedence (S42) behaves sensibly with both at defaults.

### 3. Add Default for PreferenceProfile

In `crates/worldwake-core/src/experience.rs`:

```rust
impl Default for PreferenceProfile {
    fn default() -> Self {
        Self {
            route_caution_weight: Permille::new_unchecked(500),
            source_trust_weight: Permille::new_unchecked(500),
            route_memory_capacity: 20,
            source_memory_capacity: 20,
            memory_retention_ticks: 200,
        }
    }
}
```

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
