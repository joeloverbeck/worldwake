# S146GOASCHGOA-005: `CandidateExtractor` trait + 20 impls + migrate candidate generation to registry dispatch

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — restructures candidate emission (P12 phase distinction: candidate generation phase); deletes the direct top-level `emit_*` dispatch list; rewires `candidate_generation.rs` to registry-driven dispatch
**Deps**: archive/tickets/S146GOASCHGOA-003.md, archive/tickets/S146GOASCHGOA-004.md

## Problem

S146 PR-2 migrates the 20 hand-coded top-level candidate-emission families in `crates/worldwake-ai/src/candidate_generation.rs` into a `CandidateExtractor` trait + registry indexed by `CandidateExtractorId` (ticket 004). This ticket lands the trait, the 20 impls, and the live `candidate_generation.rs` dispatch migration atomically — combining them per FND-28 avoids any transient state where impls exist as dead code alongside the old explicit live call list. The migration reads the per-agent `AgentSchemaContextProfile` (ticket 003) via the `GoalBeliefView::agent_schema_context_profile` accessor to honor extractor opt-outs.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The 20 top-level candidate-emission family functions existed in `crates/worldwake-ai/src/candidate_generation.rs`: `emit_need_candidates`, `emit_production_candidates`, `emit_opportunity_compiler_candidates`, plus 17 others (signatures grepped at reassessment). Each took `candidates: &mut Vec<GoalOffer>`, `diagnostics: &mut CandidateGenerationDiagnostics`, `ctx: &GenerationContext<'_>`, and optional goal-family-specific params (e.g., `needs: Option<HomeostaticNeeds>`, `thresholds: Option<DriveThresholds>`). The live dispatch sequence was in `candidate_generation.rs::generate_candidates_with_memories_with_travel_horizon_impl`, not `agent_tick/planning.rs`; `agent_tick/planning.rs` consumes candidate-generation output downstream. Existing focused tests in `candidate_generation.rs::#[cfg(test)]`: `emit_relieve_goal_produces_per_place_latrine_candidates_plus_wilderness`, `emit_relieve_goal_produces_only_wilderness_when_no_latrines_reachable`, `emit_wash_goal_produces_one_candidate_per_basin_at_place`, `emit_wash_goal_produces_zero_candidates_when_no_basins_reachable`, `emit_wash_goal_skips_known_remote_basin_without_state_carrier`, `reachable_remote_workstation_keeps_missing_input_produce_goal_emittable`, `missing_recipe_input_without_workstation_withholds_produce_goal`, `emit_bounty_posting_candidates_skips_when_accessor_returns_none`, `emit_bounty_posting_candidates_uses_accessor_returned_reward_source`, `emit_social_candidates_skips_agents_without_tell_profile`, `generate_candidates_orchestrates_all_domain_groups`, `produce_candidate_knowledge_path_records_workstation`, `generate_candidates_emits_exploration_for_hunger_without_known_food_path`, `generate_candidates_emits_proactive_exploration_for_comfortable_agent`, `generate_candidates_skip_proactive_exploration_when_need_or_cooldown_gate_fails`, `generate_candidates_skip_proactive_exploration_without_diversification_profile`, `generate_candidates_skips_exploration_when_food_path_is_known`.
2. Per `specs/S146-goal-schema-and-per-goal-budgets.md` D3 + D6: the trait surface is `fn extract(&self, ctx: &ExtractorContext<'_>) -> Vec<GoalOffer>` + `fn id(&self) -> CandidateExtractorId` + `fn is_enabled_for(&self, profile: &AgentSchemaContextProfile) -> bool` (with default impl `!profile.disabled_extractors.contains(&self.id())`). `ExtractorContext` wraps the existing `GenerationContext` and `CandidateGenerationDiagnostics` — no new context types are introduced. The trait deliberately routes through `GoalBeliefView` (via `ctx.generation.view`) to preserve parity with existing emit signatures; widening to `RuntimeBeliefView` was rejected at reassessment because it would force every migrated body to re-cast.
3. Shared abstraction boundary under audit: `CandidateExtractor` trait surface in `crates/worldwake-ai/src/candidate_generation.rs`, re-exported through `crates/worldwake-ai/src/goal_schema_registry/extractors.rs`. The contract is: each impl's `extract()` delegates to the corresponding renamed private helper, with prior candidate visibility preserved so families that depend on earlier candidate output retain behavior.
4. AI-regression layer: this ticket modifies the candidate-generation phase (P12 phase distinction). Intended verification layer is candidate-generation focused/unit coverage — the 17 existing tests named in item 1 prove per-extractor output equivalence. Runtime `agent_tick` decision-trace coverage is exercised by `generate_candidates_orchestrates_all_domain_groups:16928`. Golden E2E coverage is ticket 007's responsibility (parity fixtures + new golden).
5. Live `GoalKind` surface under test: all 25+ variants of `GoalKind` (`crates/worldwake-core/src/goal.rs:62`) — the per-extractor impl set covers the full set via the `CandidateExtractorId` → impl mapping. The current operator/affordance surface each scenario depends on is unchanged because the `extract()` body is the unchanged emit_* body.
6. AI-regression layer detail: the live migration is in the candidate-generation phase. Local needs-only harness is NOT sufficient — full action registries are required because the 20 extractors span enterprise, combat, political, and patrol families. Use `cargo test -p worldwake-ai` (the workspace integration tests there exercise the full action registry).
7. Ordering layer: this ticket preserves the existing extractor invocation order by deriving a deduped extractor sequence from `GoalSchema.candidate_extractors` in the current legacy order. It must not run extractors once per schema entry: several `GoalDispatchKey` entries share one extractor family and ticket 004 intentionally recorded multi-producer entries for `InvestigateViolation` and `ExploreLocation`. No search/budget ordering substrate is changed in this ticket — ticket 006 owns search/budget ordering changes.
8. Heuristic removal: no heuristic or filter is removed in this ticket. The migration is structurally invariant — every gate, threshold, and suppression rule in each `emit_*` function moves intact into its `extract()` impl body. The new `is_enabled_for` guard adds an extractor-level skip when `disabled_extractors.contains(&self.id())`, which is a new feature, not a removed one.
13. Adjacent contradictions:
   - The 17 existing tests in `candidate_generation.rs` exercise `emit_*` functions directly. After migration, the tests either (a) call the extractor impl's `extract()` method directly, or (b) call through the registry. Classified as **required consequence** — each test must be updated to call the new entry point. Tests that assert on candidate counts/contents per extractor are best routed through (a) (direct impl call) for tightness; tests that assert on orchestration (`generate_candidates_orchestrates_all_domain_groups:16928`) are best routed through (b).
   - Helper functions inside `candidate_generation.rs` (e.g., `emit_self_consume_candidates`, `emit_sleep_goal`, `emit_relieve_goal`, etc. — which `emit_need_candidates` calls internally) are unchanged and retained as module-private helpers; only the top-level 20 `emit_*` functions are migrated. Classified as **separate refactoring opportunity, deferred** — splitting the helpers per-extractor is a future cleanup.

## Architecture Check

1. FND-28 combined ticket: trait + impls + migration land atomically so no dual representations coexist. The 20 `emit_*` functions are deleted in this same ticket as the impls are wired through the registry. No transient dead-code state in the live authority path.
2. FND-26 (systems through state): extractors read belief views and snapshot state via `ctx.generation`; no system-to-system commands introduced. The registry is shared build-time data, not a command surface.
3. FND-22 (agent diversity): `AgentSchemaContextProfile.disabled_extractors` lets scenarios author per-agent extractor opt-outs (e.g., peasant with `disabled_extractors: BTreeSet::from([CandidateExtractorId::Enterprise])`).
4. FND-14 / FND-7 (locality + belief-only planning): profile read flows through `GoalBeliefView::agent_schema_context_profile`, not direct world-state access.

## Verified Layers

1. Each `emit_*` function's behavior is preserved under migration (candidate-generation phase) → existing focused tests in `candidate_generation.rs::#[cfg(test)]` (17 tests listed in Assumption Reassessment item 1) updated to exercise the new extractor impls; assertions on candidate sets remain identical
2. Registry-derived extractor order matches the pre-migration sequence in `candidate_generation.rs` and dedupes repeated extractor IDs → `generate_candidates_orchestrates_all_domain_groups` plus the full `candidate_generation` filter proved the live dispatch still emits all domain groups; deeper fixture parity remains ticket 007's responsibility
3. `disabled_extractors` honored at dispatch time → focused test in `candidate_generation.rs::disabled_enterprise_extractor_suppresses_enterprise_candidates`: agent with `disabled_extractors: BTreeSet::from([CandidateExtractorId::Enterprise])` emits no enterprise candidates even when the underlying belief state would produce them
4. Decision-trace surface preserves per-extractor suppression provenance → existing `CandidateSuppressionDiagnostic` flow through `CandidateGenerationDiagnostics` remains intact (no new trace field; existing diagnostics continue to record per-extractor suppression reasons)

## Landed Changes

1. `CandidateExtractor`, `ExtractorContext`, 20 extractor structs, `extractor_for`, `build_extractor_registry`, and `ordered_candidate_extractors_from_goal_schemas` landed in `crates/worldwake-ai/src/candidate_generation.rs`, beside the private helper bodies they delegate to.
2. `generate_candidates_with_memories_with_travel_horizon_impl` now builds a `BTreeMap<CandidateExtractorId, &'static dyn CandidateExtractor>`, derives the active extractor order from `GoalSchema.candidate_extractors`, reads `AgentSchemaContextProfile` through `GoalBeliefView::agent_schema_context_profile`, and skips disabled extractor families before extending the candidate list.
3. The renamed private helper bodies preserve cumulative prior-candidate visibility by seeding each extractor's local vector with prior candidates and returning only newly appended offers. This preserved existing behavior where later families suppress or shape output based on earlier candidates.
4. `crates/worldwake-ai/src/goal_schema_registry/` now exists as the requested schema-registry module surface, re-exporting the live extractor and registry owner from `candidate_generation.rs`.
5. The live top-level explicit dispatch list and its direct `emit_*` family function names were removed from the candidate-generation path; module-private helper functions that were already sub-family helpers remain intentionally in place.

## Landed Files

- `crates/worldwake-ai/src/goal_schema_registry/mod.rs` (new)
- `crates/worldwake-ai/src/goal_schema_registry/extractors.rs` (new)
- `crates/worldwake-ai/src/goal_schema_registry/registry.rs` (new)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — replace the explicit top-level dispatch list with registry-driven extractor dispatch; preserve module-private helper bodies; add the disabled-enterprise extractor regression)
- `crates/worldwake-ai/src/lib.rs` (modify — add `pub mod goal_schema_registry;` + re-exports)

## Out of Scope

- Search-side budget application — owned by ticket 006.
- Trace provenance field on `PlanAttemptTrace` — owned by ticket 006.
- New parity goldens or extractor-output fixtures — owned by ticket 007.
- Splitting the module-private helpers (`emit_self_consume_candidates`, `emit_sleep_goal`, etc.) per-extractor — future cleanup, not in this ticket.
- Suppression flow changes — uses existing `CandidateGenerationDiagnostics` mechanism unchanged; no new `SuppressionLog` type introduced.

## Acceptance Result

### Tests Passed

1. All 17 existing tests named in Assumption Reassessment item 1 pass under the migrated entry points
2. `disabled_enterprise_extractor_suppresses_enterprise_candidates` in `candidate_generation.rs::#[cfg(test)]`: agent with `disabled_extractors: BTreeSet::from([CandidateExtractorId::Enterprise])` emits no enterprise candidates
3. `cargo test -p worldwake-ai` (full ai-crate suite, including the workspace integration tests that exercise the full action registry)
4. `cargo test --workspace`
5. Clippy clean: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Candidate generation produces the same output Vec for the same input state, modulo per-extractor `disabled_extractors` skipping (parity invariant — ticket 007's parity fixtures formally enforce; this ticket's tests structurally enforce per-extractor).
2. Top-level `emit_*_candidates` functions are deleted (zero matches via `grep -rn "pub(crate) fn emit_.*_candidates" crates/worldwake-ai/src/candidate_generation.rs`).
3. Registry iteration is `BTreeMap`-ordered (`AGENTS.md` determinism).
4. Profile read flows exclusively through `GoalBeliefView::agent_schema_context_profile` — no direct `world.get_component_agent_schema_context_profile(actor)` call from inside candidate generation (FND-14).

## Test Plan Result

### Landed Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` `#[cfg(test)]` — update 17 existing tests listed in Assumption Reassessment item 1 to call the migrated extractor impls or registry dispatch
2. `crates/worldwake-ai/src/candidate_generation.rs` `#[cfg(test)]` — `disabled_enterprise_extractor_suppresses_enterprise_candidates` proves `CandidateExtractor::is_enabled_for` honors `disabled_extractors` through the live generation path
3. `crates/worldwake-ai/src/goal_schema_registry/` — module surface added as re-exports over the live candidate-generation owner; no separate tests were needed because the live registry dispatch is covered through `candidate_generation.rs`

### Commands Run

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `scripts/verify.sh` — waived for this ticket iteration; the harness final pre-push phase owns the repo wrapper, while this ticket ran the wrapper's relevant workspace test and all-target clippy gates directly.

## Verification Result

1. Passed: `cargo test -p worldwake-ai candidate_generation --quiet` — 232 candidate-generation tests passed, including `disabled_enterprise_extractor_suppresses_enterprise_candidates`.
2. Passed: `cargo test -p worldwake-ai --quiet` — full `worldwake-ai` crate suite passed.
3. Passed: `cargo clippy --workspace --all-targets -- -D warnings` — CI-shaped all-target clippy gate passed.
4. Passed: `cargo test --workspace --quiet` — workspace test gate passed.
5. Passed: `rg -n "pub\\(crate\\) fn emit_.*_candidates|generate_candidates_orchestrates_all_domain_groups|disabled_enterprise_extractor|ordered_candidate_extractors" crates/worldwake-ai/src/candidate_generation.rs crates/worldwake-ai/src/goal_schema_registry` — no `pub(crate) fn emit_*_candidates` functions remain; registry order and opt-out tests are present.
6. Waived: `scripts/verify.sh` — ticket iteration covered the relevant wrapper gates directly with `cargo test --workspace --quiet` and `cargo clippy --workspace --all-targets -- -D warnings`; final harness pre-push verification remains the wrapper owner.

## Outcome

Completed: 2026-05-17

The live candidate-generation phase now dispatches through a schema-derived `CandidateExtractorId` sequence backed by a `BTreeMap` extractor registry. The old explicit top-level call list in `generate_candidates_with_memories_with_travel_horizon_impl` was removed, and each extractor delegates to a renamed private helper that preserves the existing candidate-family body. `AgentSchemaContextProfile.disabled_extractors` is read through `GoalBeliefView::agent_schema_context_profile`, and a focused regression proves disabling `CandidateExtractorId::Enterprise` suppresses an otherwise-emitted restock candidate.

Deviations from the draft:

1. The live migration point was `crates/worldwake-ai/src/candidate_generation.rs`, not `crates/worldwake-ai/src/agent_tick/planning.rs`; current `agent_tick/planning.rs` consumes the generated candidates downstream and did not own the emitter call list.
2. Extractor impls stayed in `candidate_generation.rs` beside the existing private helper bodies so the migration did not need to expose or move thousands of lines of private helper logic. `crates/worldwake-ai/src/goal_schema_registry/` provides the requested module surface as re-exports over the live owner.
3. The new opt-out regression lives in `candidate_generation.rs::disabled_enterprise_extractor_suppresses_enterprise_candidates` rather than an `agent_tick/planning.rs` inline test because the contract under test is candidate extraction, not planning/search.
