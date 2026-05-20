# S152COGARCSEE-001: Core archetype value types and template table

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new core value types (`worldwake-core`)
**Deps**: None

## Problem

S152 introduces cognitive archetypes as concrete state templates that diversify agents at spawn (FND-22). The foundation is a closed `CognitiveArchetype` enum, a delta-template struct that modulates existing universal profile fields, the build-time table of ten templates, the per-scenario assignment policy, and the event payload type. These are pure value types with no runtime wiring; defining them first lets every downstream ticket reference real symbols.

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The spec's delta targets are real fields on existing core profiles (verified this session during `/reassess-spec`): `CognitiveProfile` has `max_plan_depth`, `repair_budget_fraction`, `switch_margin`, `planning_switch_margin`, `guard_min_confidence_ceiling`, and eleven `*_backoff_ticks`/`*_block_ticks` fields (`crates/worldwake-core/src/cognitive_profile.rs`); `RiskWeightProfile.threat_aversion` (`risk_weight_profile.rs:13`); `TestimonyTrustProfile.confirmation_weight`/`refutation_penalty` (`testimony_trust_profile.rs:9,11`); `RoutePreferenceProfile.dangerous_traversal_penalty` (`route_preference_profile.rs:11`); `EpistemicDispositionProfile.ask_memory_retention_ticks` (`epistemic.rs:30`); `PortfolioWeightsProfile.need_survival`/`pain_care`/`obligation_duty`/`economic_opportunity`/`social_motive` (`portfolio_weights_profile.rs:8-16`). The template stores *deltas keyed by these real names*; it does not store the fictional names from the original spec draft.
2. `Permille` (`crates/worldwake-core/src/numerics.rs:25`, `Copy`, ctor `Permille::new(u16) -> Result<…>`, range [0,1000]) and `MethodSchemaId` (`crates/worldwake-core/src/method_schema_id.rs:4`) exist and are `worldwake-core`-resident. `StateHash([u8; 32])` and `canonical::hash_serializable<T>` exist (`canonical.rs:31,51`).
3. This is a pure type-definition ticket; the shared boundary under audit is the new module `crates/worldwake-core/src/cognitive_archetype.rs` and its `lib.rs` re-exports. No existing function changes. Re-export pattern follows `pub mod cognitive_profile; pub use cognitive_profile::CognitiveProfile;` (`lib.rs:39,163`).
4. (Cumulative arithmetic) Template deltas are signed `i8`/`i32`; the resolver (ticket 005) clamps `Permille` results to [0,1000] and integer fields to non-negative ranges. Reassessment found the drafted `backoff_ticks_scale: Permille` contract invalid because `Permille` cannot represent the spec's above-identity `1500` Cautious scale, while existing `MultiplierPermille` cannot represent below-identity Bold scales. Per the 2026-05-20 FOUNDATIONS check, this ticket adds `BackoffScalePermille` as a core numeric value type where `1000 == 1x` and values above/below identity are both lawful. The scale is applied multiplicatively to the eleven existing backoff/block tick fields at resolution; the clamp/scale math lives in ticket 005.

## Architecture Check

1. Expressing archetype effects as deltas over *existing* fields (rather than introducing new behavioral fields) means no consuming system needs new wiring — `failure_handling.rs`, `route_threat.rs`, `candidate_generation.rs`, and ranking already read these fields. This is the FND-3 (concrete state) and FND-28 (no redundant parallel state) choice settled in reassessment.
2. No backwards-compatibility shims: all symbols are net-new. The template is build-time data analogous to S146's `GoalSchema` registry.

## Verified Layers

1. Type/trait-bound correctness (`Copy`/`Serialize`/`Eq`/`Ord` where required) -> focused unit test that constructs each template and asserts derive-required trait bounds.
2. Template determinism (same archetype → identical template) -> focused unit test comparing `*_template()` outputs.
3. Single-layer ticket: no decision/action/event-log layer is touched because these are inert value types not yet stored or emitted; runtime proof surfaces are mapped by tickets 005 and 008.

## Landed Changes

### 1. `CognitiveArchetype` enum

Added `crates/worldwake-core/src/cognitive_archetype.rs` with the closed 10-variant enum (Cautious, Bold, Stubborn, Methodical, Opportunistic, Sociable, Skeptical, Dutiful, Greedy, Fearful) deriving `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.

### 2. `ArchetypeProfileTemplate`

Added the D2 delta fields keyed by real field names (`backoff_ticks_scale: BackoffScalePermille`; `ask_memory_retention_ticks_delta: i32`; five portfolio slot deltas; `method_disable: Vec<MethodSchemaId>`), deriving `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. No `method_enable` direction was added.

### 3. Template table (D3)

Added ten `*_template() -> ArchetypeProfileTemplate` functions plus a `template_for(CognitiveArchetype) -> ArchetypeProfileTemplate` dispatch as build-time data.

### 4. `ArchetypeAssignmentPolicy` and `ArchetypeAssignmentSource`

```rust
pub enum ArchetypeAssignmentPolicy {
    #[default]
    DefaultUniformFive,
    Uniform(BTreeSet<CognitiveArchetype>),
    Weighted(BTreeMap<CognitiveArchetype, u32>),
}
pub enum ArchetypeAssignmentSource { Policy(ArchetypeAssignmentPolicy), Explicit }
```
The policy derives `Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize`; the source derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `DefaultUniformFive`'s curated set (Cautious/Bold/Methodical/Opportunistic/Sociable) is available as a `const`/helper for the resolver.

### 5. `PersonalityAssignedPayload`

```rust
pub struct PersonalityAssignedPayload {
    pub agent: EntityId,
    pub archetype: CognitiveArchetype,
    pub seed: u64,
    pub source: ArchetypeAssignmentSource,
    pub resolved_profile_hash: StateHash,
}
```
The payload derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. EventPayload integration remains owned by ticket 003; hash computation remains owned by ticket 005.

### 6. `BackoffScalePermille` numeric value

Added `BackoffScalePermille` to `crates/worldwake-core/src/numerics.rs` and exported it from `lib.rs`. It stores a non-zero `u16` scale where `1000 == 1x`, unlike bounded `Permille` (`0..=1000`) or `MultiplierPermille` (`>=1000` only). Focused numeric tests cover trait bounds, below/identity/above acceptance, zero rejection, and bincode round-trip.

### 7. `lib.rs` re-exports

`lib.rs` now declares `pub mod cognitive_archetype;`, re-exports `CognitiveArchetype`, `ArchetypeProfileTemplate`, `ArchetypeAssignmentPolicy`, `ArchetypeAssignmentSource`, `PersonalityAssignedPayload`, and `template_for`, and re-exports `BackoffScalePermille` from `numerics`.

## Landed Files

- `crates/worldwake-core/src/cognitive_archetype.rs` (new)
- `crates/worldwake-core/src/numerics.rs` (modify — add `BackoffScalePermille`)
- `crates/worldwake-core/src/lib.rs` (modify — module + re-exports)
- `archive/specs/S152-cognitive-archetypes-seeded-diversity.md` (modify — correct backoff scale type)

## Out of Scope

- Component registration of `CognitiveArchetypeComponent` (ticket 002).
- `EventTag::PersonalityAssigned` variant and the `EventPayload` field (ticket 003).
- Any resolver/clamp/scale logic, RNG, or delta application (ticket 005).
- Any change to consuming systems — they read the existing fields unchanged.

## Acceptance Result

### Tests Passed

1. Each of the ten templates constructs and `template_for` returns the matching `archetype` field.
2. `ArchetypeAssignmentPolicy::default() == DefaultUniformFive`; `DefaultUniformFive` curated set is exactly {Cautious, Bold, Methodical, Opportunistic, Sociable}.
3. `PersonalityAssignedPayload` round-trips through `serde` (bincode) unchanged.
4. `BackoffScalePermille` accepts below-identity, identity, and above-identity scales, rejects zero, and round-trips through bincode.
5. Existing suite: `cargo test -p worldwake-core`

### Invariants Preserved

1. `CognitiveArchetype` remains a closed enum — adding a variant requires a follow-up spec.
2. No new behavioral profile field is introduced (FND-28); the template only carries deltas to existing fields.
3. Backoff scaling is represented by one typed numeric value that supports both Cautious longer waits and Bold shorter waits without misusing bounded `Permille`.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/cognitive_archetype.rs` (`#[cfg(test)]`) — template construction, `template_for` dispatch, default-five set, payload round-trip.
2. `crates/worldwake-core/src/numerics.rs` (`#[cfg(test)]`) — `BackoffScalePermille` constructor, serialization, and trait-bound coverage.

### Commands Run

1. `cargo test -p worldwake-core cognitive_archetype`
2. `cargo test -p worldwake-core backoff_scale`
3. `cargo clippy -p worldwake-core --all-targets -- -D warnings`
4. `cargo test -p worldwake-core`

## Outcome

Completed on 2026-05-20.

- Added `BackoffScalePermille` to `crates/worldwake-core/src/numerics.rs` as a typed non-zero integer scale where `1000 == 1x`, with focused constructor, round-trip, and trait-bound tests.
- Added `crates/worldwake-core/src/cognitive_archetype.rs` with the closed `CognitiveArchetype` enum, `ArchetypeProfileTemplate`, all ten deterministic archetype templates, assignment policy/source value types, default-five helper, and `PersonalityAssignedPayload`.
- Exported the new numeric and archetype symbols from `worldwake-core`.
- Updated S152 to use `BackoffScalePermille` for backoff scaling after reassessment showed the drafted `Permille` scale could not represent above-identity values.

## Deviations

- Replaced the drafted `backoff_ticks_scale: Permille` with `backoff_ticks_scale: BackoffScalePermille`. This was required for FOUNDATIONS alignment because S152 needs one concrete typed value that can express both Cautious longer waits and Bold shorter waits without misusing bounded `Permille` or one-sided `MultiplierPermille`.
- `./scripts/verify.sh` was not run for this ticket iteration; the harness reserves that full pre-PR gate for final branch push. The ticket-owned proof was covered by the focused core tests, core clippy, and the full `worldwake-core` test suite.

## Verification Result

- Passed `cargo test -p worldwake-core cognitive_archetype`
- Passed `cargo test -p worldwake-core backoff_scale`
- Passed `cargo clippy -p worldwake-core --all-targets -- -D warnings`
- Passed `cargo test -p worldwake-core`
