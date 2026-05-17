# S146GOASCHGOA-004: Define `CandidateExtractorId` identity set + populate `GoalSchema` with 2 new fields

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `GoalSchema` gains 2 required fields populated atomically across all 41 entries; `CandidateExtractorId` finalized for 20 extractor identities
**Deps**: archive/tickets/S146GOASCHGOA-001.md, archive/tickets/S146GOASCHGOA-002.md, 003

## Problem

S146 PR-2 (data-driven `GoalSchema` registry) requires each schema entry to declare which extractor families produce its candidates (`candidate_extractors`) and which budget tier its plan search uses (`planning_budget`). Ticket 004 lands the field additions and populates all 41 `static DECL_*` entries atomically (required-field migration — both fields are non-`Option` without a meaningful `Default`). It also finalizes the `CandidateExtractorId` identity set (20 variants matching the eventual extractor impls in ticket 005) — extending the placeholder newtype from ticket 003.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `archive/tickets/S146GOASCHGOA-001.md` landed the in-place rename, so `GoalSchema` (renamed from `GoalDispatchDeclaration`) lives at `crates/worldwake-ai/src/goal_schema.rs:61` with 8 fields. After ticket 003 lands, `CandidateExtractorId` is a placeholder newtype `pub struct CandidateExtractorId(pub u16);` in `crates/worldwake-core/src/agent_schema_context_profile.rs`. After ticket 002 lands, `GoalPlanningBudget` is available at `crates/worldwake-core/src/goal_planning_budget.rs` with 5 preset constants. The 41 `static DECL_*: GoalSchema = GoalSchema { ... };` entries currently enumerate all 8 fields without spread syntax (verified — no `..Default::default()` in any entry).
2. Per `specs/S146-goal-schema-and-per-goal-budgets.md` D4: each existing entry is updated to populate the two new fields. The spec example shows `candidate_extractors: &[CandidateExtractorId::Need]` and `planning_budget: GoalPlanningBudget::SELF_CARE` for `DECL_CONSUME_OWNED_COMMODITY`. The 20 extractor identities correspond to the existing 20 `emit_*` functions in `candidate_generation.rs`.
3. Shared abstraction boundary under audit: `CandidateExtractorId` is the identity surface that links `GoalSchema.candidate_extractors` (this ticket) to `impl CandidateExtractor` (ticket 005). The semantic contract is "each `CandidateExtractorId` corresponds to exactly one `CandidateExtractor` impl that emits the same candidate family that the legacy `emit_*` function produced." Ticket 005 enforces the second half (`fn id(&self) -> CandidateExtractorId` per impl); this ticket establishes the canonical variant set.
4. Required-field migration: per `spec-to-tickets` skill's "Required-field migrations" rule, the foundation ticket must populate all construction sites in one shot. This ticket does that — adds both fields to `GoalSchema` and assigns real values to all 41 entries atomically. No placeholder values used; each entry's extractor mapping follows the existing call-graph in `candidate_generation.rs` (e.g., `Eat`-family entries → `Need`, `Produce*` entries → `Production`, etc.).
5. Existing focused tests: ticket 005 will reassess tests in `candidate_generation.rs` (`emit_relieve_goal_produces_per_place_latrine_candidates_plus_wilderness:11690`, etc.). This ticket's changes are field-population-only and do not alter test behavior — the 41 entries' existing 8-field values are unchanged.
13. Adjacent contradictions: the `CandidateExtractorId` shape change (newtype → enum) ripples into ticket 003's already-committed `BTreeSet<CandidateExtractorId>` usage. Classified as **required consequence** — the placeholder pattern explicitly anticipates ticket 004's refinement. `BTreeSet` and the derives (Copy, Clone, Debug, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize) hold under either representation.

## Architecture Check

1. Single-truth registry per FND-28: extending `GoalSchema` in place rather than introducing a parallel structure. Schema is build-time data, not authoritative world state (`static` const-style table).
2. Concrete typed identity per FND-3: `CandidateExtractorId` enum variants are concrete dispatch handles, not opaque scores. Each variant maps to exactly one trait impl (enforced by ticket 005's `fn id(&self)`).
3. No fossilized scaffolding: only fields backed by actual S146 deliverables are added (`candidate_extractors` because ticket 005 implements; `planning_budget` because ticket 006 reads). The other speculative fields the original spec listed (`satisfaction_predicate`, `invalidator_templates`, etc.) are deliberately NOT added — those duplicate existing typed surfaces or have no S146 backing.

## Verification Layers

1. All 41 `static DECL_*` entries compile after field addition → `cargo build -p worldwake-ai`
2. Every `GoalDispatchKey::ALL` variant has a populated `GoalSchema` entry → new runtime test in `crates/worldwake-ai/src/goal_schema.rs` `#[cfg(test)]`
3. `CandidateExtractorId` enum's 20 variants correspond 1:1 to the 20 existing `emit_*` functions in `candidate_generation.rs` (semantic invariant, enforced narratively here; ticket 005's impls structurally enforce it)
4. Single-layer ticket — no behavioral change in this ticket (the populated fields are read by tickets 005 and 006, not by anything committed here).

## What to Change

### 1. Finalize `CandidateExtractorId` enum

Replace the placeholder newtype in `crates/worldwake-core/src/agent_schema_context_profile.rs` with the 20-variant enum:

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

### 2. Add 2 fields to `GoalSchema`

In `crates/worldwake-ai/src/goal_schema.rs`, extend the struct:

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

Add imports: `use worldwake_core::{CandidateExtractorId, GoalPlanningBudget};`.

### 3. Populate all 41 `static DECL_*` entries

Each entry gets two new fields. Mapping rules:
- **Self-care needs**: `DECL_CONSUME_OWNED_COMMODITY`, `DECL_ACQUIRE_SELF_CONSUME`, `DECL_SLEEP`, `DECL_RELIEVE`, `DECL_WASH` → `&[CandidateExtractorId::Need]`, `GoalPlanningBudget::SELF_CARE`
- **Acquisition/travel-purchase**: `DECL_ACQUIRE_RECIPE_INPUT`, `DECL_ACQUIRE_RESTOCK`, `DECL_FREE_CARRY_CAPACITY`, `DECL_MOVE_CARGO`, `DECL_LOOT_CORPSE`, `DECL_BURY_CORPSE` → `&[CandidateExtractorId::Enterprise]` or `&[CandidateExtractorId::Disposal]`, `GoalPlanningBudget::TRAVEL_PURCHASE`
- **Production**: `DECL_PRODUCE_COMMODITY`, `DECL_SELL_COMMODITY`, `DECL_RESTOCK_COMMODITY` → `&[CandidateExtractorId::Production]` (and possibly `Enterprise` for sell), `GoalPlanningBudget::PRODUCTION`
- **Combat/raid**: `DECL_ENGAGE_HOSTILE`, `DECL_RAID_TARGET`, `DECL_REDUCE_DANGER`, `DECL_REGROUP_WITH_FACTION`, `DECL_ESTABLISH_BANDIT_CAMP`, `DECL_TREAT_WOUNDS` → `&[CandidateExtractorId::Combat]`, `GoalPlanningBudget::INVESTIGATION` (depth 20 for combat planning)
- **Bounty/escort**: `DECL_FULFILL_BOUNTY`, `DECL_POST_BOUNTY`, `DECL_ESCORT_TO_SAFETY`, `DECL_SEARCH_FOR_MISSING`, `DECL_REPORT_MISSING`, `DECL_REPORT_FOUND` → `&[CandidateExtractorId::Bounty]` / `&[CandidateExtractorId::Escort]` / `&[CandidateExtractorId::Search]` / `&[CandidateExtractorId::ReportFound]`, `GoalPlanningBudget::BOUNTY_ESCORT`
- **Crime/political/social**: `DECL_INVESTIGATE_VIOLATION`, `DECL_PATROL`, `DECL_CLAIM_OFFICE`, `DECL_SUPPORT_CANDIDATE_FOR_OFFICE`, `DECL_STEAL_ITEM`, `DECL_ACCUSE`, `DECL_PUNISH_ACCUSED`, `DECL_POST_NOTICE`, `DECL_SHARE_BELIEF`, `DECL_ASK_WITNESS`, `DECL_EXPLORE_LOCATION` → respective `Crime`/`Political`/`Patrol`/`RecordedViolation`/`Social`/`AskWitness`/`Exploration`/`ProactiveExploration`/`ArtifactPosting` extractor IDs, with `GoalPlanningBudget::INVESTIGATION` (depth 20) for investigation-class goals, `SELF_CARE` or `TRAVEL_PURCHASE` for the lighter ones

The implementer should grep `candidate_generation.rs` for the existing call-graph (which `emit_*` function produces each `GoalKind` variant) to confirm the per-entry extractor assignment is faithful to current behavior. Mark per-entry extractor + budget assignments as TODOs in the implementation pass if any are uncertain, then resolve with the user via 1-3-1 before merge.

### 4. Registry-coverage runtime test

Add to `crates/worldwake-ai/src/goal_schema.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn goal_schema_registry_covers_all_dispatch_keys() {
    for key in GoalDispatchKey::ALL {
        let schema = lookup_schema(*key);
        assert!(
            !schema.candidate_extractors.is_empty()
                || schema.frontier_exhaustion_strategy == FrontierExhaustionStrategy::Empty,
            "GoalDispatchKey::{:?} has no candidate_extractors entry",
            key,
        );
    }
}
```

(Adjust `lookup_schema` to whatever the registry's dispatch helper is named in the post-rename `goal_schema.rs`.)

## Files to Touch

- `crates/worldwake-core/src/agent_schema_context_profile.rs` (modify — replace newtype with enum)
- `crates/worldwake-ai/src/goal_schema.rs` (modify — add 2 fields to struct + populate all 41 `static DECL_*` entries + add registry-coverage test)
- `crates/worldwake-ai/src/lib.rs` (modify if `CandidateExtractorId` needs new re-export from ai crate)

## Out of Scope

- `CandidateExtractor` trait definition and 20 impls — owned by ticket 005.
- `agent_tick/planning.rs` migration to registry dispatch — owned by ticket 005.
- Search-side budget application — owned by ticket 006.
- Trace provenance field — owned by ticket 006.
- Removing or modifying any of the 8 existing `GoalSchema` fields — strictly additive.
- Behavioral changes to candidate emission — none in this ticket; the populated `candidate_extractors` field has no runtime consumer yet (ticket 005 wires it).

## Acceptance Criteria

### Tests That Must Pass

1. Workspace builds: `cargo build --workspace`
2. New registry-coverage test: `cargo test -p worldwake-ai goal_schema_registry_covers_all_dispatch_keys`
3. Existing test suite: `cargo test --workspace`
4. Clippy clean: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Every `GoalDispatchKey::ALL` variant has a populated `GoalSchema` entry with `candidate_extractors` and `planning_budget` set (runtime test enforces).
2. `CandidateExtractorId` enum has exactly 20 variants corresponding 1:1 to the 20 existing `emit_*` functions in `candidate_generation.rs` (semantic — verified narratively here, structurally by ticket 005's `fn id(&self)` impls).
3. Per-entry budget assignment respects `CognitiveProfile.max_plan_depth = 8` default ceiling: even though some presets exceed depth 8, ticket 006's `min()` clamp ensures default-cognitive-profile agents are not affected. Goldens that author higher cognitive ceilings can opt into deeper budgets.
4. No new `Permille` `new` (fallible) calls introduced — `GoalPlanningBudget` presets use `new_unchecked` exclusively (`AGENTS.md` determinism).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_schema.rs` `#[cfg(test)]` — `goal_schema_registry_covers_all_dispatch_keys` (registry coverage runtime assertion)
2. `crates/worldwake-core/src/agent_schema_context_profile.rs` `#[cfg(test)]` — extend the existing serde-roundtrip test (ticket 003) to cover the enum variant set

### Commands

1. `cargo test -p worldwake-ai goal_schema_registry_covers_all_dispatch_keys`
2. `cargo test -p worldwake-core`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh`
