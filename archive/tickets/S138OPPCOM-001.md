# S138OPPCOM-001: Foundation types and decision-trace surface for opportunity compiler

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None (pure type definitions; runtime behavior arrives in 006)
**Deps**: None

## Problem

S138 introduces a bottom-up opportunity compiler with several typed enums, an ai-side `Opportunity` rich record, a per-tick `PerceivedOpportunityIndex` read-model, and decision-trace surface extensions for source attribution and load reporting. Landing the types as a pure-definitions foundation lets downstream tickets (005 `EffectSchemaIndex`, 006 compiler core, 009 observer) reference them without circular construction or premature integration. No runtime logic is introduced here — every new type is unused at landing except where existing `RootCandidateTrace` construction sites must populate a default `CandidateSource::Emitter`.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage: `crates/worldwake-ai/src/decision_trace.rs` has inline `#[cfg(test)]` adjacent to `RootCandidateTrace` (defined at line 820), plus construction sites at `decision_trace.rs:4620` (vec!-style `root_candidates`) and `crates/worldwake-ai/src/search/candidates.rs:144-145` (direct struct literal). No existing test asserts the absence of a `source` field, so the addition is non-regressing.
2. Spec/doc reference: `specs/S138-opportunity-compiler.md` deliverable sections "New typed enums and read-models" and "Decision-trace surface".
3. Shared abstraction boundary under audit: the `Opportunity` rich record + `PerceivedOpportunityIndex` per-tick view + `RootCandidateTrace.source` attribution — all live in `worldwake-ai`, no cross-crate type leakage.
4. `EffectFact` lives at `crates/worldwake-sim/src/effect_schema.rs:209` with 6 variants (`CommodityTransfer`, `PartialQuantity`, `WoundApplied`, `ExpectationFulfilled`, `ContentionGrantConsumed`, `EventEmitted`); `EffectFactKey` must 1:1 mirror these names as payload-free variants. `ViolationKind` (referenced by `RiskFact::CriminalLiability`) exists at `crates/worldwake-core/src/violation.rs:24`.

## Architecture Check

1. Pure type-definition foundations avoid premature binding: each downstream ticket consumes the types without coupling to the foundation's implementation rhythm. The alternative — defining types alongside their first consumer — would force the compile-time graph to follow the implementation graph, blocking parallel work on 005 and 009.
2. No backward-compatibility shims: `RootCandidateTrace.source` is a new field; both construction sites are updated atomically, no Option wrapping. `CandidateSource::Emitter` matches existing behavior so trace semantics are preserved.
3. Discriminant-mirror pattern (`EffectFactKey`) keeps the cross-crate enum out of core: sim's payload-bearing `EffectFact` stays in sim; the ai-side discriminant is independent and lightweight, suitable for use as a `BTreeMap` key in ticket 005.

## Verification Layers

1. Type compilation + derive integrity — focused unit test (bincode roundtrip per type)
2. `RootCandidateTrace.source` defaults to `Emitter` for existing emission sites — focused unit test reading a constructed trace at search/candidates.rs:144
3. `EffectFactKey` variant set matches `EffectFact` (sim) — focused unit test using exhaustive match on `EffectFact` to map each variant to `EffectFactKey`, asserting all 6 map cleanly

## What to Change

### 1. New module `crates/worldwake-ai/src/opportunity_compiler/`

Create `mod.rs` and `types.rs`. `types.rs` defines:

- `EffectFactKey` — 6-variant payload-free enum mirroring `EffectFact` variant names
- `RiskFact` — payload-bearing enum: `CriminalLiability { violation_kind: ViolationKind }`, `SocialShameRisk`, `ThreatPresence { source: EntityId }`, `InjuryRisk`, `PropertyForfeitureRisk`
- `ClaimTopic` — payload-bearing enum: `EntityLocation { subject: EntityId }`, `CommodityAvailability { commodity: CommodityKind, place: EntityId }`, `OwnershipClaim { item: EntityId }`, `HostilePresence { place: EntityId }`, `RouteSafety { from: EntityId, to: EntityId }`
- `BelievedLegalStatus` — `BelievedOwned { owner: EntityId }`, `BelievedUnclaimed`, `BelievedContested`, `SociallyOpenToRequest`, `Forbidden { jurisdiction: EntityId }`
- `SocialExposureBand` — `Private`, `Public`, `PublicWithCriminalRisk`, `PublicWithShameRisk`
- `Opportunity` — struct with `key: OpportunityKey`, `perceived_at: Tick`, `source_belief: BeliefRef`, `possible_effects: Vec<EffectFactKey>`, `possible_information: Vec<ClaimTopic>`, `required_actions: Vec<PlannerOpKind>`, `legal_status: BelievedLegalStatus`, `social_exposure: SocialExposureBand`, `risks: Vec<RiskFact>`, `salience: Permille`
- `OpportunityHandle(pub u32)` — dense index
- `PerceivedOpportunityIndex` — `Default`-deriving struct with `by_place: BTreeMap<EntityId, Vec<OpportunityHandle>>`, `by_anchor: BTreeMap<EntityId, OpportunityHandle>`, `all: Vec<Opportunity>`

All payload-free enums derive `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`. Payload-bearing enums and `Opportunity` derive `Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize` (no `Copy` because of `Vec` fields).

`mod.rs` re-exports the types: `pub use types::*;`.

### 2. Expose module in `crates/worldwake-ai/src/lib.rs`

Add `pub mod opportunity_compiler;` next to existing pub-mod declarations.

### 3. Decision-trace surface extension

Modify `crates/worldwake-ai/src/decision_trace.rs`:

- Add `pub enum CandidateSource { Emitter, OpportunityCompiler }` adjacent to `RootCandidateTrace` definition (line 820), deriving `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.
- Add `pub source: CandidateSource` field to `RootCandidateTrace` after the existing fields.
- Add `pub struct OpportunityCompilerLoad` with `compiled_count: u32`, `salience_floored: u32`, `learned_memory_damped: u32`, `cap_truncated: u32`. Derive `Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.

### 4. Update `RootCandidateTrace` construction sites

- `crates/worldwake-ai/src/decision_trace.rs:4620` — add `source: CandidateSource::Emitter` to the literal
- `crates/worldwake-ai/src/search/candidates.rs:144-145` — same

## Files to Touch

- `crates/worldwake-ai/src/opportunity_compiler/mod.rs` (new)
- `crates/worldwake-ai/src/opportunity_compiler/types.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — add types + field, update construction)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — update one construction site)

## Out of Scope

- `compile_opportunities` function and `agent_tick` integration — lands in ticket 006
- `EffectSchemaIndex` module — lands in ticket 005
- Per-tick recording of `OpportunityCompilerLoad` on the decision-trace sink — the struct is defined here; the recording behavior is in 006
- `Authority` enum + `relevant_ops_authority()` — lands in ticket 004
- New universal-on-Agent components (`RiskWeightProfile`, `LawAbidingProfile`) — lands in ticket 002
- Profile field additions on `CognitiveProfile`/`PerceptionProfile` — lands in ticket 003
- Observer rendering of opportunities — lands in ticket 009

## Acceptance Criteria

### Tests That Must Pass

1. New test: bincode roundtrip for `Opportunity`, `EffectFactKey`, `RiskFact`, `ClaimTopic`, `BelievedLegalStatus`, `SocialExposureBand`, `OpportunityHandle`, `PerceivedOpportunityIndex` — each round-trips identity
2. New test: `EffectFact → EffectFactKey` exhaustive mapping covers all 6 variants (uses `match` on `EffectFact` to force compile failure if a sim variant is added without the ai-side mirror)
3. New test: `RootCandidateTrace` constructed at `search/candidates.rs:144` resolves to `source: CandidateSource::Emitter`
4. Existing suite: `cargo test -p worldwake-ai`
5. Workspace builds: `cargo build --workspace`

### Invariants

1. Adding `source: CandidateSource` to `RootCandidateTrace` does not change semantics of any existing trace consumer — all current sites resolve to `Emitter`
2. `EffectFactKey` variant names match `EffectFact` (sim) 1:1 — drift triggers the exhaustive-match test failure
3. New types live entirely in `worldwake-ai/src/opportunity_compiler/types.rs`; no leakage into `worldwake-core` or `worldwake-sim`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/opportunity_compiler/types.rs` (inline `#[cfg(test)]`) — bincode roundtrip per new type; variant-count assertion via exhaustive `EffectFact` match
2. `crates/worldwake-ai/src/search/candidates.rs` (inline `#[cfg(test)]`) — `root_candidate_trace_from_candidate` defaults to `CandidateSource::Emitter`

### Commands

1. `cargo test -p worldwake-ai opportunity_compiler::types`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-11.

- Added `worldwake-ai::opportunity_compiler` with the staged `Opportunity` record, `EffectFactKey`, risk/information/legal/exposure enums, `OpportunityHandle`, and `PerceivedOpportunityIndex`.
- Added `CandidateSource` and `OpportunityCompilerLoad` to the decision-trace type surface, and set both existing `RootCandidateTrace` construction sites to `CandidateSource::Emitter`.
- Added focused bincode roundtrip coverage for the new opportunity types, compile-time `EffectFact -> EffectFactKey` mirror coverage, and a constructor-level regression proving existing root candidates default to `Emitter`.
- Runtime opportunity compilation, load recording on the trace sink, and `OpportunityCompiler` attribution for opportunity-derived candidates remain deferred to S138OPPCOM-006 as planned.

## Deviations

- The `CandidateSource::Emitter` focused assertion landed in `crates/worldwake-ai/src/search/candidates.rs` beside the live constructor instead of `decision_trace.rs`, because that is the strongest local proof for the existing emission site named by the ticket.
- `SAVE_FORMAT_VERSION` was not bumped: the new opportunity types are ai-side derived/read-model substrate, and `RootCandidateTrace` is decision-trace diagnostic state rather than the `worldwake-sim` save/load payload.

## Verification Result

- Passed `cargo test -p worldwake-ai opportunity_compiler::types`
- Passed `cargo test -p worldwake-ai --lib search::candidates::tests::root_candidate_trace_from_candidate_defaults_to_emitter_source -- --exact`
- Passed `cargo fmt --all`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo build --workspace`
- Passed `cargo test -p worldwake-ai`
