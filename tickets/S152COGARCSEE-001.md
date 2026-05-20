# S152COGARCSEE-001: Core archetype value types and template table

**Status**: PENDING
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
4. (Cumulative arithmetic) Template deltas are signed `i8`/`i32`; the resolver (ticket 005) clamps `Permille` results to [0,1000] and integer fields to non-negative ranges. `backoff_ticks_scale: Permille` (1000 = unchanged) is applied multiplicatively to the eleven existing backoff/block tick fields at resolution — defining the field here carries no arithmetic; the clamp/scale math lives in ticket 005.

## Architecture Check

1. Expressing archetype effects as deltas over *existing* fields (rather than introducing new behavioral fields) means no consuming system needs new wiring — `failure_handling.rs`, `route_threat.rs`, `candidate_generation.rs`, and ranking already read these fields. This is the FND-3 (concrete state) and FND-28 (no redundant parallel state) choice settled in reassessment.
2. No backwards-compatibility shims: all symbols are net-new. The template is build-time data analogous to S146's `GoalSchema` registry.

## Verification Layers

1. Type/trait-bound correctness (`Copy`/`Serialize`/`Eq`/`Ord` where required) -> focused unit test that constructs each template and asserts derive-required trait bounds.
2. Template determinism (same archetype → identical template) -> focused unit test comparing `*_template()` outputs.
3. Single-layer ticket: no decision/action/event-log layer is touched because these are inert value types not yet stored or emitted; runtime proof surfaces are mapped by tickets 005 and 008.

## What to Change

### 1. `CognitiveArchetype` enum

Add `crates/worldwake-core/src/cognitive_archetype.rs`. Define the closed 10-variant enum (Cautious, Bold, Stubborn, Methodical, Opportunistic, Sociable, Skeptical, Dutiful, Greedy, Fearful) deriving `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.

### 2. `ArchetypeProfileTemplate`

Struct with the delta fields enumerated in spec D2 (deltas keyed by real field names; `backoff_ticks_scale: Permille`; `ask_memory_retention_ticks_delta: i32`; five portfolio slot deltas; `method_disable: Vec<MethodSchemaId>`). Derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. No `method_enable` direction.

### 3. Template table (D3)

Ten `*_template() -> ArchetypeProfileTemplate` functions, plus a `template_for(CognitiveArchetype) -> ArchetypeProfileTemplate` dispatch. Build-time data.

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
Derive `Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize` (policy); `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` (source). Provide `DefaultUniformFive`'s curated set (Cautious/Bold/Methodical/Opportunistic/Sociable) as a `const`/helper for the resolver.

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
Derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. (Field is consumed by the EventPayload integration in ticket 003; the hash is computed by the resolver in ticket 005.)

### 6. `lib.rs` re-exports

`pub mod cognitive_archetype;` plus `pub use cognitive_archetype::{CognitiveArchetype, ArchetypeProfileTemplate, ArchetypeAssignmentPolicy, ArchetypeAssignmentSource, PersonalityAssignedPayload};` and `template_for`.

## Files to Touch

- `crates/worldwake-core/src/cognitive_archetype.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — module + re-exports)

## Out of Scope

- Component registration of `CognitiveArchetypeComponent` (ticket 002).
- `EventTag::PersonalityAssigned` variant and the `EventPayload` field (ticket 003).
- Any resolver/clamp/scale logic, RNG, or delta application (ticket 005).
- Any change to consuming systems — they read the existing fields unchanged.

## Acceptance Criteria

### Tests That Must Pass

1. Each of the ten templates constructs and `template_for` returns the matching `archetype` field.
2. `ArchetypeAssignmentPolicy::default() == DefaultUniformFive`; `DefaultUniformFive` curated set is exactly {Cautious, Bold, Methodical, Opportunistic, Sociable}.
3. `PersonalityAssignedPayload` round-trips through `serde` (bincode) unchanged.
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `CognitiveArchetype` remains a closed enum — adding a variant requires a follow-up spec.
2. No new behavioral profile field is introduced (FND-28); the template only carries deltas to existing fields.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_archetype.rs` (`#[cfg(test)]`) — template construction, `template_for` dispatch, default-five set, payload round-trip.

### Commands

1. `cargo test -p worldwake-core cognitive_archetype`
2. `cargo clippy -p worldwake-core --all-targets -- -D warnings`
3. `./scripts/verify.sh` (before PR push)
