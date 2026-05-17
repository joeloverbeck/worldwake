# S146GOASCHGOA-004: Define `CandidateExtractorId` identity set + populate `GoalSchema` with 2 new fields

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `GoalSchema` gains 2 required fields populated atomically across all 41 entries; `CandidateExtractorId` finalized for 20 extractor identities
**Deps**: archive/tickets/S146GOASCHGOA-001.md, archive/tickets/S146GOASCHGOA-002.md, archive/tickets/S146GOASCHGOA-003.md

## Problem

Before this ticket, S146 PR-2 (data-driven `GoalSchema` registry) required each schema entry to declare which extractor families produce its candidates (`candidate_extractors`) and which budget tier its plan search uses (`planning_budget`). Ticket 004 landed the field additions and populated all 41 `static DECL_*` entries atomically (required-field migration — both fields are non-`Option` without a meaningful `Default`). It also finalized the `CandidateExtractorId` identity set (20 variants matching the eventual extractor impls in ticket 005), replacing the placeholder newtype from ticket 003.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Before this ticket, `GoalSchema` (renamed from `GoalDispatchDeclaration` by `archive/tickets/S146GOASCHGOA-001.md`) lived in `crates/worldwake-ai/src/goal_schema.rs` with 8 fields, `CandidateExtractorId` was a placeholder newtype in `crates/worldwake-core/src/agent_schema_context_profile.rs`, and `GoalPlanningBudget` was already available from `archive/tickets/S146GOASCHGOA-002.md`.
2. Per `archive/specs/S146-goal-schema-and-per-goal-budgets.md` D4: each existing entry now populates the two added fields. The 20 extractor identities correspond to the top-level extractor families in `candidate_generation.rs`, with multi-producer entries where the live branch has multiple producer families for one `GoalDispatchKey`.
3. Shared abstraction boundary under audit: `CandidateExtractorId` is the identity surface that links `GoalSchema.candidate_extractors` (this ticket) to `impl CandidateExtractor` (ticket 005). The semantic contract is "each `CandidateExtractorId` corresponds to exactly one `CandidateExtractor` impl that emits the same candidate family that the legacy `emit_*` function produced." Ticket 005 enforces the second half (`fn id(&self) -> CandidateExtractorId` per impl); this ticket establishes the canonical variant set.
4. Required-field migration: both fields were added to `GoalSchema` and assigned real values across all 41 entries atomically. No placeholder values were used; each entry's extractor mapping follows the existing call graph in `candidate_generation.rs`.
5. Existing focused tests: ticket 005 still owns migration of the direct `candidate_generation.rs` emitter tests (`emit_relieve_goal_produces_per_place_latrine_candidates_plus_wilderness:11690`, etc.). This ticket's changes were field-population-only and did not alter candidate emission behavior — the 41 entries' existing 8-field values are unchanged.
13. Adjacent contradictions: the `CandidateExtractorId` shape change (newtype → enum) ripples into ticket 003's already-committed `BTreeSet<CandidateExtractorId>` usage. Classified as **required consequence** — the placeholder pattern explicitly anticipates ticket 004's refinement. `BTreeSet` and the derives (Copy, Clone, Debug, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize) hold under either representation.

## Architecture Check

1. Single-truth registry per FND-28: extending `GoalSchema` in place rather than introducing a parallel structure. Schema is build-time data, not authoritative world state (`static` const-style table).
2. Concrete typed identity per FND-3: `CandidateExtractorId` enum variants are concrete dispatch handles, not opaque scores. Each variant maps to exactly one trait impl (enforced by ticket 005's `fn id(&self)`).
3. No fossilized scaffolding: only fields backed by actual S146 deliverables are added (`candidate_extractors` because ticket 005 implements; `planning_budget` because ticket 006 reads). The other speculative fields the original spec listed (`satisfaction_predicate`, `invalidator_templates`, etc.) are deliberately NOT added — those duplicate existing typed surfaces or have no S146 backing.

## Verified Layers

1. All 41 `static DECL_*` entries compile after field addition → `cargo build --workspace`
2. Every `GoalDispatchKey::ALL` variant has a populated `GoalSchema` entry → runtime test in `crates/worldwake-ai/src/goal_schema.rs` `#[cfg(test)]`
3. `CandidateExtractorId` enum's 20 variants correspond 1:1 to the 20 existing `emit_*` functions in `candidate_generation.rs` (semantic invariant, enforced narratively here; ticket 005's impls structurally enforce it)
4. Single-layer ticket — no behavioral change in this ticket (the populated fields are read by tickets 005 and 006, not by anything committed here).

## Landed Changes

### 1. Finalized `CandidateExtractorId` enum

Replaced the placeholder newtype in `crates/worldwake-core/src/agent_schema_context_profile.rs` with the 20-variant enum plus an `ALL` constant:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CandidateExtractorId {
    Need,
    Production,
    Enterprise,
    Disposal,
    Bounty,
    ArtifactPosting,
    Combat,
    Crime,
    Social,
    AskWitness,
    Patrol,
    Political,
    RecordedViolation,
    Search,
    ReportFound,
    Escort,
    Exploration,
    ProactiveExploration,
    ExpectationViolation,
    OpportunityCompiler,
}
```

Variant names mirror the existing `emit_*` function name root in `candidate_generation.rs` (e.g., `emit_need_candidates` → `Need`, `emit_opportunity_compiler_candidates` → `OpportunityCompiler`).

### 2. Added 2 fields to `GoalSchema`

In `crates/worldwake-ai/src/goal_schema.rs`, extended the struct:

```rust
pub struct GoalSchema {
    // existing 8 fields unchanged
    pub provenance_family: Option<RankedGoalProvenanceFamily>,
    pub trace_label: &'static str,
    pub relevant_ops: &'static [PlannerOpKind],
    pub invalidation_strategy: InvalidationStrategy,
    pub feasibility_strategy: FeasibilityStrategy,
    pub frontier_exhaustion_strategy: FrontierExhaustionStrategy,
    pub family_policy: GoalFamilyPolicy,
    pub progress_barrier_ops: &'static [PlannerOpKind],
    // new fields for S146
    pub candidate_extractors: &'static [CandidateExtractorId],
    pub planning_budget: GoalPlanningBudget,
}
```

Imported `CandidateExtractorId` and `GoalPlanningBudget` from `worldwake_core`.

### 3. Populated all 41 `static DECL_*` entries

Each entry now has two new fields. Mapping rules used:
- **Self-care needs**: `DECL_CONSUME_OWNED_COMMODITY`, `DECL_ACQUIRE_SELF_CONSUME`, `DECL_SLEEP`, `DECL_RELIEVE`, `DECL_WASH` → `&[CandidateExtractorId::Need]`, `GoalPlanningBudget::SELF_CARE`
- **Acquisition/travel-purchase**: `DECL_ACQUIRE_RECIPE_INPUT` and `DECL_ACQUIRE_RESTOCK` → `OpportunityCompiler` because the live `candidate_generation.rs` path does not emit those purposes through `emit_enterprise_candidates`; `DECL_FREE_CARRY_CAPACITY` → `Disposal`; `DECL_MOVE_CARGO` → `Enterprise`; `DECL_LOOT_CORPSE` and `DECL_BURY_CORPSE` → `Combat`.
- **Production and enterprise**: `DECL_PRODUCE_COMMODITY` → `Production`; `DECL_SELL_COMMODITY`, `DECL_RESTOCK_COMMODITY`, and `DECL_MOVE_CARGO` → `Enterprise`.
- **Combat/raid**: `DECL_ENGAGE_HOSTILE`, `DECL_RAID_TARGET`, `DECL_REDUCE_DANGER`, and `DECL_TREAT_WOUNDS` → `Combat`, `GoalPlanningBudget::INVESTIGATION`; `DECL_LOOT_CORPSE` and `DECL_BURY_CORPSE` → `Combat`, `GoalPlanningBudget::TRAVEL_PURCHASE`.
- **Social faction**: `DECL_REGROUP_WITH_FACTION` and `DECL_ESTABLISH_BANDIT_CAMP` → `Social`, `GoalPlanningBudget::TRAVEL_PURCHASE`.
- **Bounty/escort**: `DECL_FULFILL_BOUNTY`, `DECL_POST_BOUNTY`, `DECL_ESCORT_TO_SAFETY`, `DECL_SEARCH_FOR_MISSING`, `DECL_REPORT_MISSING`, `DECL_REPORT_FOUND` → `&[CandidateExtractorId::Bounty]` / `&[CandidateExtractorId::Escort]` / `&[CandidateExtractorId::Search]` / `&[CandidateExtractorId::ReportFound]`, `GoalPlanningBudget::BOUNTY_ESCORT`
- **Crime/political/social**: `DECL_INVESTIGATE_VIOLATION`, `DECL_PATROL`, `DECL_CLAIM_OFFICE`, `DECL_SUPPORT_CANDIDATE_FOR_OFFICE`, `DECL_STEAL_ITEM`, `DECL_ACCUSE`, `DECL_PUNISH_ACCUSED`, `DECL_POST_NOTICE`, `DECL_SHARE_BELIEF`, `DECL_ASK_WITNESS`, `DECL_EXPLORE_LOCATION` → respective `Crime`/`Political`/`Patrol`/`RecordedViolation`/`Social`/`AskWitness`/`Exploration`/`ProactiveExploration`/`ArtifactPosting` extractor IDs, with `GoalPlanningBudget::INVESTIGATION` (depth 20) for investigation-class goals, `SELF_CARE` or `TRAVEL_PURCHASE` for the lighter ones

The implementation used a live `candidate_generation.rs` call-graph sweep to confirm the per-entry extractor assignments. `InvestigateViolation` now lists both `RecordedViolation` and `ExpectationViolation`; `ExploreLocation` now lists both `Exploration` and `ProactiveExploration`.

### 4. Added registry-coverage runtime tests

Added to `crates/worldwake-ai/src/goal_schema.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn goal_schema_registry_covers_all_dispatch_keys() {
    for key in GoalDispatchKey::ALL {
        let declaration = key.declaration();

        assert!(
            !declaration.candidate_extractors.is_empty(),
            "GoalDispatchKey::{key:?} has no candidate_extractors entry"
        );
    }
}
```

## Landed Files

- `crates/worldwake-core/src/agent_schema_context_profile.rs` (modify — replace newtype with enum)
- `crates/worldwake-ai/src/goal_schema.rs` (modify — add 2 fields to struct + populate all 41 `static DECL_*` entries + add registry-coverage test)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — update test fixture from tuple newtype construction to enum variant)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — update test fixture from tuple newtype construction to enum variant)
- `crates/worldwake-sim/src/save_load.rs` (modify — update save/load test fixture from tuple newtype construction to enum variant)
- `crates/worldwake-ai/src/lib.rs` (checked — no change required; `CandidateExtractorId` remains re-exported from `worldwake-core`)

## Out of Scope

- `CandidateExtractor` trait definition and 20 impls — owned by ticket 005.
- `agent_tick/planning.rs` migration to registry dispatch — owned by ticket 005.
- Search-side budget application — owned by ticket 006.
- Trace provenance field — owned by ticket 006.
- Removing or modifying any of the 8 existing `GoalSchema` fields — strictly additive.
- Behavioral changes to candidate emission — none in this ticket; the populated `candidate_extractors` field has no runtime consumer yet (ticket 005 wires it).

## Acceptance Result

### Tests That Passed

1. Workspace builds: `cargo build --workspace`
2. New registry-coverage test: `cargo test -p worldwake-ai goal_schema_registry_covers_all_dispatch_keys`
3. Existing test suite: `cargo test --workspace`
4. Clippy clean: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Every `GoalDispatchKey::ALL` variant has a populated `GoalSchema` entry with `candidate_extractors` and `planning_budget` set (runtime test enforces).
2. `CandidateExtractorId` enum has exactly 20 variants corresponding 1:1 to the 20 existing `emit_*` functions in `candidate_generation.rs` (semantic — verified narratively here, structurally by ticket 005's `fn id(&self)` impls).
3. Per-entry budget assignment respects `CognitiveProfile.max_plan_depth = 8` default ceiling: even though some presets exceed depth 8, ticket 006's `min()` clamp ensures default-cognitive-profile agents are not affected. Goldens that author higher cognitive ceilings can opt into deeper budgets.
4. No new `Permille` `new` (fallible) calls introduced — `GoalPlanningBudget` presets use `new_unchecked` exclusively (`AGENTS.md` determinism).

## Test Plan Result

### Focused Tests

1. `crates/worldwake-ai/src/goal_schema.rs` `#[cfg(test)]` — `goal_schema_registry_covers_all_dispatch_keys` (registry coverage runtime assertion)
2. `crates/worldwake-ai/src/goal_schema.rs` `#[cfg(test)]` — `goal_schema_assigns_expected_extractor_and_budget_examples` (multi-producer and opportunity-compiler mapping examples)
3. `crates/worldwake-core/src/agent_schema_context_profile.rs` `#[cfg(test)]` — extended serde-roundtrip fixture to use enum variants and added `candidate_extractor_id_all_covers_variant_set`

### Command Results

1. Passed `cargo test -p worldwake-ai goal_schema_registry_covers_all_dispatch_keys`
2. Discarded non-proof `cargo test -p worldwake-core candidate_extractor_id_all_covers_variant_set serde_roundtrip_preserves_overrides` because Cargo accepts only one test-name selector in that position.
3. Passed `cargo test -p worldwake-core`
4. Passed `cargo build --workspace`
5. Passed `cargo test --workspace`
6. Passed `cargo clippy --workspace --all-targets -- -D warnings`
7. `scripts/verify.sh` not run for this ticket iteration because its constituent gates relevant to this ticket were run directly here; final harness push remains responsible for the pre-PR wrapper.

## Outcome

Completed on 2026-05-17.

- `CandidateExtractorId` is now a concrete 20-variant enum with deterministic ordering and serde/bincode coverage.
- `GoalSchema` now carries `candidate_extractors` and `planning_budget`, populated across all 41 `GoalDispatchKey` declarations.
- The registry coverage test proves every dispatch key has extractor metadata; example assertions pin the self-care, opportunity-compiler, violation, and exploration multi-producer mappings.
- Test fixtures in CLI, sim save/load, and per-agent belief-view code were updated from tuple-newtype construction to enum variants.

## Deviations

- Live reassessment showed `AcquireRecipeInput` and `AcquireRestock` are not emitted by `emit_enterprise_candidates`; they are mapped to `OpportunityCompiler` for this ticket.
- Live reassessment showed `InvestigateViolation` and `ExploreLocation` each have two lawful producer families, so their schema entries carry both extractor IDs.

## Verification Result

- Passed `cargo test -p worldwake-ai goal_schema_registry_covers_all_dispatch_keys`.
- Passed `cargo test -p worldwake-core`.
- Passed `cargo build --workspace`.
- Passed `cargo test --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `rg -n 'CandidateExtractorId\(' crates` with zero matches.
