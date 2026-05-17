# S146GOASCHGOA-005: `CandidateExtractor` trait + 20 impls + migrate `agent_tick/planning.rs` to registry dispatch

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — restructures candidate emission (P12 phase distinction: candidate generation phase); deletes 20 `emit_*` functions; rewires `agent_tick/planning.rs` to registry-driven dispatch
**Deps**: 003, 004

## Problem

S146 PR-2 migrates the 20 hand-coded `emit_*` functions in `crates/worldwake-ai/src/candidate_generation.rs` into a `CandidateExtractor` trait + registry indexed by `CandidateExtractorId` (ticket 004). This ticket lands the trait, the 20 impls (each absorbing one `emit_*` function's body), and the `agent_tick/planning.rs` migration atomically — combining them per FND-28 avoids any transient state where impls exist as dead code alongside live `emit_*` functions. The migration reads the per-agent `AgentSchemaContextProfile` (ticket 003) via the `GoalBeliefView::agent_schema_context_profile` accessor to honor extractor opt-outs.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The 20 `emit_*` functions exist at named locations in `crates/worldwake-ai/src/candidate_generation.rs`: `emit_need_candidates:823`, `emit_production_candidates:841`, `emit_opportunity_compiler_candidates:522`, plus 17 others (signatures grepped at reassessment). Each takes `candidates: &mut Vec<GoalOffer>`, `diagnostics: &mut CandidateGenerationDiagnostics`, `ctx: &GenerationContext<'_>`, and optional goal-family-specific params (e.g., `needs: Option<HomeostaticNeeds>`, `thresholds: Option<DriveThresholds>`). The dispatch sequence lives in `agent_tick/planning.rs::generate_candidates_with_memories_with_travel_horizon_impl` (lines ~449–480). Existing focused tests in `candidate_generation.rs::#[cfg(test)]`: `emit_relieve_goal_produces_per_place_latrine_candidates_plus_wilderness:11690`, `emit_relieve_goal_produces_only_wilderness_when_no_latrines_reachable:11743`, `emit_wash_goal_produces_one_candidate_per_basin_at_place:11857`, `emit_wash_goal_produces_zero_candidates_when_no_basins_reachable:11898`, `emit_wash_goal_skips_known_remote_basin_without_state_carrier:11926`, `reachable_remote_workstation_keeps_missing_input_produce_goal_emittable:13103`, `missing_recipe_input_without_workstation_withholds_produce_goal:13366`, `emit_bounty_posting_candidates_skips_when_accessor_returns_none:15528`, `emit_bounty_posting_candidates_uses_accessor_returned_reward_source:15627`, `emit_social_candidates_skips_agents_without_tell_profile:16217`, `generate_candidates_orchestrates_all_domain_groups:16928`, `produce_candidate_knowledge_path_records_workstation:17960`, `generate_candidates_emits_exploration_for_hunger_without_known_food_path:20629`, `generate_candidates_emits_proactive_exploration_for_comfortable_agent:20844`, `generate_candidates_skip_proactive_exploration_when_need_or_cooldown_gate_fails:20892`, `generate_candidates_skip_proactive_exploration_without_diversification_profile:20953`, `generate_candidates_skips_exploration_when_food_path_is_known:21046`.
2. Per `specs/S146-goal-schema-and-per-goal-budgets.md` D3 + D6: the trait surface is `fn extract(&self, ctx: &ExtractorContext<'_>) -> Vec<GoalOffer>` + `fn id(&self) -> CandidateExtractorId` + `fn is_enabled_for(&self, profile: &AgentSchemaContextProfile) -> bool` (with default impl `!profile.disabled_extractors.contains(&self.id())`). `ExtractorContext` wraps the existing `GenerationContext` and `CandidateGenerationDiagnostics` — no new context types are introduced. The trait deliberately routes through `GoalBeliefView` (via `ctx.generation.view`) to preserve parity with existing emit signatures; widening to `RuntimeBeliefView` was rejected at reassessment because it would force every migrated body to re-cast.
3. Shared abstraction boundary under audit: `CandidateExtractor` trait surface in the new file `crates/worldwake-ai/src/goal_schema_registry/extractors.rs`. The contract is: each impl's `extract()` body is the function body of the corresponding `emit_*` function, with candidates collected into and returned from a local `Vec<GoalOffer>` rather than pushed into a `&mut Vec` parameter. Existing function bodies move into impls with minimal change.
4. AI-regression layer: this ticket modifies the candidate-generation phase (P12 phase distinction). Intended verification layer is candidate-generation focused/unit coverage — the 17 existing tests named in item 1 prove per-extractor output equivalence. Runtime `agent_tick` decision-trace coverage is exercised by `generate_candidates_orchestrates_all_domain_groups:16928`. Golden E2E coverage is ticket 007's responsibility (parity fixtures + new golden).
5. Live `GoalKind` surface under test: all 25+ variants of `GoalKind` (`crates/worldwake-core/src/goal.rs:62`) — the per-extractor impl set covers the full set via the `CandidateExtractorId` → impl mapping. The current operator/affordance surface each scenario depends on is unchanged because the `extract()` body is the unchanged emit_* body.
6. AI-regression layer detail: `agent_tick/planning.rs` migration is in the runtime `agent_tick` decision-trace layer. Local needs-only harness is NOT sufficient — full action registries are required because the 20 extractors span enterprise, combat, political, and patrol families. Use `cargo test -p worldwake-ai` (the workspace integration tests there exercise the full action registry).
7. Ordering layer: this ticket preserves the existing extractor invocation order (registry iteration order = `BTreeMap` natural order of `GoalDispatchKey`, then `candidate_extractors` slice order per schema entry). No ordering substrate is changed in this ticket — ticket 006 owns search/budget ordering changes.
8. Heuristic removal: no heuristic or filter is removed in this ticket. The migration is structurally invariant — every gate, threshold, and suppression rule in each `emit_*` function moves intact into its `extract()` impl body. The new `is_enabled_for` guard adds an extractor-level skip when `disabled_extractors.contains(&self.id())`, which is a new feature, not a removed one.
13. Adjacent contradictions:
   - The 17 existing tests in `candidate_generation.rs` exercise `emit_*` functions directly. After migration, the tests either (a) call the extractor impl's `extract()` method directly, or (b) call through the registry. Classified as **required consequence** — each test must be updated to call the new entry point. Tests that assert on candidate counts/contents per extractor are best routed through (a) (direct impl call) for tightness; tests that assert on orchestration (`generate_candidates_orchestrates_all_domain_groups:16928`) are best routed through (b).
   - Helper functions inside `candidate_generation.rs` (e.g., `emit_self_consume_candidates`, `emit_sleep_goal`, `emit_relieve_goal`, etc. — which `emit_need_candidates` calls internally) are unchanged and retained as module-private helpers; only the top-level 20 `emit_*` functions are migrated. Classified as **separate refactoring opportunity, deferred** — splitting the helpers per-extractor is a future cleanup.

## Architecture Check

1. FND-28 combined ticket: trait + impls + migration land atomically so no dual representations coexist. The 20 `emit_*` functions are deleted in this same ticket as the impls are wired through the registry. No transient dead-code state in the live authority path.
2. FND-26 (systems through state): extractors read belief views and snapshot state via `ctx.generation`; no system-to-system commands introduced. The registry is shared build-time data, not a command surface.
3. FND-22 (agent diversity): `AgentSchemaContextProfile.disabled_extractors` lets scenarios author per-agent extractor opt-outs (e.g., peasant with `disabled_extractors: BTreeSet::from([CandidateExtractorId::Enterprise])`).
4. FND-14 / FND-7 (locality + belief-only planning): profile read flows through `GoalBeliefView::agent_schema_context_profile`, not direct world-state access.

## Verification Layers

1. Each `emit_*` function's behavior is preserved under migration (candidate-generation phase) → existing focused tests in `candidate_generation.rs::#[cfg(test)]` (17 tests listed in Assumption Reassessment item 1) updated to exercise the new extractor impls; assertions on candidate sets remain identical
2. Registry iteration order matches the pre-migration sequence in `agent_tick/planning.rs` → focused test asserting registry-driven dispatch produces same `Vec<GoalOffer>` as legacy explicit-call sequence (parity fixture comparison; deeper coverage is ticket 007's responsibility)
3. `disabled_extractors` honored at dispatch time → focused test in `agent_tick/planning.rs::#[cfg(test)]`: agent with `disabled_extractors: BTreeSet::from([CandidateExtractorId::Enterprise])` emits no enterprise candidates even when the underlying belief state would produce them
4. Decision-trace surface preserves per-extractor suppression provenance → existing `CandidateSuppressionDiagnostic` flow through `CandidateGenerationDiagnostics` remains intact (no new trace field; existing diagnostics continue to record per-extractor suppression reasons)

## What to Change

### 1. Define `CandidateExtractor` trait

New file `crates/worldwake-ai/src/goal_schema_registry/mod.rs`:

```rust
pub mod extractors;
pub use extractors::*;
```

New file `crates/worldwake-ai/src/goal_schema_registry/extractors.rs`:

```rust
use worldwake_core::{AgentSchemaContextProfile, CandidateExtractorId};
use crate::candidate_generation::{CandidateGenerationDiagnostics, GenerationContext, GoalOffer};

pub struct ExtractorContext<'a> {
    pub generation: &'a GenerationContext<'a>,
    pub diagnostics: &'a mut CandidateGenerationDiagnostics,
}

pub trait CandidateExtractor: Send + Sync {
    fn extract(&self, ctx: &mut ExtractorContext<'_>) -> Vec<GoalOffer>;
    fn id(&self) -> CandidateExtractorId;
    fn is_enabled_for(&self, profile: &AgentSchemaContextProfile) -> bool {
        !profile.disabled_extractors.contains(&self.id())
    }
}
```

Note: `&mut ExtractorContext` is used so `diagnostics` remains `&mut` accessible from inside `extract()` bodies (matching the current `&mut CandidateGenerationDiagnostics` parameter on every `emit_*` function).

### 2. Implement 20 extractor structs

In the same `extractors.rs` (or split into sub-modules `extractors/need.rs`, `extractors/production.rs`, etc., if file size becomes unwieldy):

```rust
pub struct NeedExtractor;
impl CandidateExtractor for NeedExtractor {
    fn extract(&self, ctx: &mut ExtractorContext<'_>) -> Vec<GoalOffer> {
        let mut candidates = Vec::new();
        let needs = ctx.generation.actor_needs();
        let thresholds = ctx.generation.actor_drive_thresholds();
        // Body of current emit_need_candidates, with `candidates.push(...)`
        // replaced by collecting into the local `candidates` Vec.
        candidates
    }
    fn id(&self) -> CandidateExtractorId { CandidateExtractorId::Need }
}

// ... 19 more impls: ProductionExtractor, EnterpriseExtractor, DisposalExtractor,
// BountyExtractor, ArtifactPostingExtractor, CombatExtractor, CrimeExtractor,
// SocialExtractor, AskWitnessExtractor, PatrolExtractor, PoliticalExtractor,
// RecordedViolationExtractor, SearchExtractor, ReportFoundExtractor, EscortExtractor,
// ExplorationExtractor, ProactiveExplorationExtractor, ExpectationViolationExtractor,
// OpportunityCompilerExtractor.
```

Each impl absorbs the body of one current `emit_*` function. Helper functions (e.g., `emit_self_consume_candidates` called inside `emit_need_candidates`) remain in `candidate_generation.rs` as module-private helpers; only the top-level 20 functions are migrated.

### 3. Extractor registry

In `crates/worldwake-ai/src/goal_schema_registry/registry.rs`:

```rust
use std::collections::BTreeMap;
use worldwake_core::CandidateExtractorId;
use super::extractors::*;

pub fn build_extractor_registry()
    -> BTreeMap<CandidateExtractorId, Box<dyn CandidateExtractor>>
{
    let mut m: BTreeMap<CandidateExtractorId, Box<dyn CandidateExtractor>> = BTreeMap::new();
    m.insert(CandidateExtractorId::Need, Box::new(NeedExtractor));
    m.insert(CandidateExtractorId::Production, Box::new(ProductionExtractor));
    // ... 18 more
    m
}
```

`BTreeMap` per CLAUDE.md determinism invariant.

### 4. Migrate `agent_tick/planning.rs` candidate phase

Replace the explicit `emit_*` call sequence (lines ~449–480 in `agent_tick/planning.rs::generate_candidates_with_memories_with_travel_horizon_impl`) with registry-driven dispatch:

```rust
let registry = ai_runtime.goal_schema_registry();
let extractors = ai_runtime.extractor_registry();
let profile = ctx.view.agent_schema_context_profile(ctx.agent);
let mut candidates: Vec<GoalOffer> = Vec::new();
let mut diagnostics = CandidateGenerationDiagnostics::default();
for schema in registry.values() {
    for extractor_id in schema.candidate_extractors {
        let Some(extractor) = extractors.get(extractor_id) else { continue };
        if !extractor.is_enabled_for(profile) {
            continue;
        }
        let mut ext_ctx = ExtractorContext {
            generation: &ctx,
            diagnostics: &mut diagnostics,
        };
        candidates.extend(extractor.extract(&mut ext_ctx));
    }
}
```

The `ai_runtime.goal_schema_registry()` and `ai_runtime.extractor_registry()` accessors land on the appropriate runtime singleton in `worldwake-ai` (likely `AgentDecisionRuntime` or a sibling). Confirm the host during implementation.

### 5. Delete 20 `emit_*` functions

After all impls are wired and tests updated, delete the top-level `pub(crate) fn emit_*_candidates(...)` definitions from `candidate_generation.rs`. Keep the module-private helpers (`emit_self_consume_candidates`, `emit_sleep_goal`, etc.) — they're called from inside the extractor impl bodies.

### 6. Update 17 existing focused tests

Each test in the "Assumption Reassessment item 1" list either calls the new extractor impl directly (preferred for per-extractor tests) or calls through the registry (preferred for orchestration tests). Per-extractor test example:

```rust
#[test]
fn emit_relieve_goal_produces_per_place_latrine_candidates_plus_wilderness() {
    let extractor = NeedExtractor;
    let mut diagnostics = CandidateGenerationDiagnostics::default();
    let mut ctx = ExtractorContext { generation: &test_ctx, diagnostics: &mut diagnostics };
    let candidates = extractor.extract(&mut ctx);
    // existing assertions on `candidates` content
}
```

Orchestration test example:

```rust
#[test]
fn generate_candidates_orchestrates_all_domain_groups() {
    // existing setup
    let candidates = generate_candidates_with_memories_with_travel_horizon_impl(&ctx);
    // existing assertions
}
```

Tests that originally called `emit_*` directly with `&mut Vec` patterns need the `let mut diagnostics = ...; let mut ext_ctx = ...;` adapter.

## Files to Touch

- `crates/worldwake-ai/src/goal_schema_registry/mod.rs` (new)
- `crates/worldwake-ai/src/goal_schema_registry/extractors.rs` (new)
- `crates/worldwake-ai/src/goal_schema_registry/registry.rs` (new)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — delete 20 top-level `emit_*` functions; preserve module-private helpers; update 17 existing tests)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — replace explicit emit sequence with registry-driven loop in `generate_candidates_with_memories_with_travel_horizon_impl` lines ~449–480; update inline tests starting line 2602+ to exercise registry dispatch where applicable)
- `crates/worldwake-ai/src/lib.rs` (modify — add `pub mod goal_schema_registry;` + re-exports)
- Likely: runtime singleton that hosts `goal_schema_registry()` and `extractor_registry()` accessors — discover during implementation by `grep "AgentDecisionRuntime\|AiRuntime" crates/worldwake-ai/src/` and select the appropriate singleton

## Out of Scope

- Search-side budget application — owned by ticket 006.
- Trace provenance field on `PlanAttemptTrace` — owned by ticket 006.
- New parity goldens or extractor-output fixtures — owned by ticket 007.
- Splitting the module-private helpers (`emit_self_consume_candidates`, `emit_sleep_goal`, etc.) per-extractor — future cleanup, not in this ticket.
- Suppression flow changes — uses existing `CandidateGenerationDiagnostics` mechanism unchanged; no new `SuppressionLog` type introduced.

## Acceptance Criteria

### Tests That Must Pass

1. All 17 existing tests named in Assumption Reassessment item 1 pass under the migrated entry points
2. New `disabled_extractors_skip_disabled_families()` test in `agent_tick/planning.rs::#[cfg(test)]`: agent with `disabled_extractors: BTreeSet::from([CandidateExtractorId::Enterprise])` emits no enterprise candidates
3. `cargo test -p worldwake-ai` (full ai-crate suite, including the workspace integration tests that exercise the full action registry)
4. `cargo test --workspace`
5. Clippy clean: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Candidate generation produces the same output Vec for the same input state, modulo per-extractor `disabled_extractors` skipping (parity invariant — ticket 007's parity fixtures formally enforce; this ticket's tests structurally enforce per-extractor).
2. Top-level `emit_*_candidates` functions are deleted (zero matches via `grep -rn "pub(crate) fn emit_.*_candidates" crates/worldwake-ai/src/candidate_generation.rs`).
3. Registry iteration is `BTreeMap`-ordered (CLAUDE.md determinism).
4. Profile read flows exclusively through `GoalBeliefView::agent_schema_context_profile` — no direct `world.get_component_agent_schema_context_profile(actor)` call from inside candidate generation (FND-14).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` `#[cfg(test)]` — update 17 existing tests listed in Assumption Reassessment item 1 to call the migrated extractor impls or registry dispatch
2. `crates/worldwake-ai/src/agent_tick/planning.rs` `#[cfg(test)]` — new `disabled_extractors_skip_disabled_families()`, new `registry_dispatch_preserves_candidate_order()`
3. `crates/worldwake-ai/src/goal_schema_registry/extractors.rs` `#[cfg(test)]` — focused unit tests for `NeedExtractor::is_enabled_for` (default impl honors `disabled_extractors`)

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `scripts/verify.sh`
