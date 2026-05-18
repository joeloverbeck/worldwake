# S148PORMOTBAC-004: Slot assembly extension composing OperatingMode with PortfolioWeightsProfile

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes - extended `agent_tick/portfolio.rs::assemble_portfolio` to consume `&PortfolioWeightsProfile` and `OperatingMode`; added motive-backed slot classification through `primary_motive_slot`; cached operating mode in the planning pipeline; preserved same-seam political and reporting/notice goldens exposed by the broader proof run
**Deps**: `archive/tickets/S148PORMOTBAC-001.md`, `archive/tickets/S148PORMOTBAC-002.md`, `archive/tickets/S148PORMOTBAC-003.md`, `specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

Tickets 001-003 landed the substrate: five `SlotKind` variants plus `motive_source_slot_for`, `PortfolioWeightsProfile`, and `OperatingMode` derivation. Ticket 004 connected those pieces. `assemble_portfolio` now iterates the five slots, picks each candidate's primary slot from the highest-weight motive-source contribution, composes profile weights with operating mode, and emits one winner per eligible slot through the existing portfolio-selection mechanism.

## Assumption Reassessment (2026-05-18)

1. Before this ticket, `assemble_portfolio` in `crates/worldwake-ai/src/agent_tick/portfolio.rs` accepted only `(ranked, committed, probe)`, selected three legacy goal-kind buckets, and left `PainCare` plus `SocialMotive` dormant even though the five-slot taxonomy already existed.
2. S148 D5 specified the extended shape: `assemble_portfolio(ranked, committed, &PortfolioWeightsProfile, OperatingMode, probe) -> Portfolio`. The landed implementation follows that contract and keeps motive-less candidates in the economic fallback slot.
3. Shared abstraction under audit: the per-tick decision pipeline in `crates/worldwake-ai/src/agent_tick/planning.rs`. This ticket extended the existing portfolio-weight threading to include `derive_operating_mode(&view, agent, ranked_candidates)` and stores the result on `AgentDecisionRuntime.operating_mode`.
4. Broad AI proof exposed two same-seam assumptions in nearby code. Political exact-target candidates needed a route/local-evidence distinction in the feasibility probe once motive-backed slots made those candidates live earlier. Obligation-shaped reporting and notice goals needed default motive-source mapping without losing their existing domain-specific motive scoring.

## Architecture Check

1. The existing `Portfolio::plausible_slots_by_score` mechanism remains the scoring surface. The change extends `assemble_portfolio` to compose with the five-slot taxonomy instead of adding a parallel selector.
2. Operating-mode degradation is represented by effective weights: Emergency mode zeroes `EconomicOpportunity` and `SocialMotive` while preserving slot identity.
3. Primary-motive selection is deterministic. Highest contribution weight wins; equal weights prefer the older `introduced_tick`; missing motive sources fall back to `EconomicOpportunity`.
4. Feasibility remains a pre-search probe concern. The same-seam adjustment distinguishes exact political targets that have local evidence from stale exact-target location beliefs without local support, and it prevents `SupportCandidateForOffice` candidate IDs from being treated as route destinations.

## Verified Layers

1. `assemble_portfolio` slot iteration correctness: focused unit coverage proves each populated slot can emit one winner under Normal mode.
2. Operating-mode-modulated weights: focused unit coverage proves Emergency mode zeroes exactly `EconomicOpportunity` and `SocialMotive`.
3. Primary motive tie-break: focused unit coverage proves highest contribution wins and equal contribution weights favor older `introduced_tick`.
4. Planning integration: focused planning coverage proves portfolio assembly still runs with the new signature and updated motive-backed fixtures.
5. Same-seam golden preservation: focused and broad AI tests prove political exact-target, obligation/reporting motive mapping, ranking, and motive-source golden behavior survived the new slot assembly path.

## Landed Changes

1. `crates/worldwake-ai/src/agent_tick/portfolio.rs`
   - Extended `assemble_portfolio` to accept `&PortfolioWeightsProfile` and `OperatingMode`.
   - Iterates `NeedSurvival`, `PainCare`, `ObligationDuty`, `EconomicOpportunity`, and `SocialMotive` in a fixed deterministic order.
   - Added `primary_motive_slot`, which maps the highest-weight `AgendaEntry.motive_source_contributions` entry through `motive_source_slot_for`.
   - Added `apply_mode`, which zeroes economic and social slots under Emergency mode.
   - Preserved committed-opportunity preference inside the per-slot selector path.
2. `crates/worldwake-ai/src/agent_tick/planning.rs`
   - Derived operating mode from the runtime belief view before candidate-plan construction.
   - Cached the derived mode on `AgentDecisionRuntime.operating_mode`.
   - Passed both profile weights and operating mode into portfolio assembly.
   - Updated planning fixtures to seed motive-source contributions.
3. `crates/worldwake-ai/src/feasibility_probe.rs`
   - Allowed exact-target political candidates with local evidence to reach search instead of failing only because target-location belief is stale.
   - Added `route_place_target` so `SupportCandidateForOffice` treats the goal-key place as the candidate entity, not a route destination.
4. `crates/worldwake-ai/src/motive_source_mapping.rs`
   - Mapped `PostNotice`, `PostBounty`, `ReportMissing`, and `ReportFound` to `OfficeDuty` defaults so obligation-shaped goals land in the obligation slot.
5. `crates/worldwake-ai/src/ranking.rs`
   - Preserved existing post/report motive scoring while those goals now use `OfficeDuty` as their default motive source.
6. `crates/worldwake-ai/tests/golden_motive_sources.rs`
   - Updated the greed-weight golden to use `SellCommodity`, keeping the test aligned with the new obligation mapping for notices.
7. `specs/S148-portfolio-and-motive-backed-intentions.md`
   - Corrected D5 status so plan-cap migration stays owned by ticket 008.

## Out of Scope

- `PainCare` and `SocialMotive` golden scenario coverage remains owned by ticket 010.
- Replacing `CognitiveProfile.max_candidates_to_plan` with mode-specific plan caps remains owned by ticket 008.
- Observer rendering of slot winners and operating mode remains owned by ticket 009.

## Acceptance Result

1. Five-slot portfolio assembly is motive-backed and deterministic.
2. Emergency mode suppresses economic and social slots through effective weights.
3. Planning derives and caches operating mode before portfolio assembly.
4. Same-seam political, obligation/reporting, ranking, and motive-source goldens pass after the new slot path is active.

## Test Plan Result

1. Added or updated focused portfolio tests for all five slots, Emergency suppression, primary-motive highest-weight selection, older-tick tie-breaking, and mode-weight application.
2. Updated focused planning tests to keep portfolio assembly covered through the new signature and motive-backed candidate fixtures.
3. Added feasibility-probe tests for local exact-target evidence, political exact-target stale-location handling, and `SupportCandidateForOffice` route target semantics.
4. Updated motive-source and ranking coverage for obligation-shaped reporting/notice goals plus the greed-weight golden.

## Outcome

Completed on 2026-05-18.

- Extended `assemble_portfolio` so five-slot assembly is driven by `AgendaEntry.motive_source_contributions` through `motive_source_slot_for`.
- Added deterministic primary-motive selection and Emergency-mode slot suppression through effective weights.
- Wired planning to derive and cache `OperatingMode`, then pass mode and profile weights into portfolio assembly.
- Preserved broad AI behavior by adjusting exact-target feasibility and obligation/reporting motive-source mapping in the same seam.
- Updated S148 D5 so the plan-cap migration boundary remains truthful for ticket 008.

## Deviations

- The drafted combined command `cargo test -p worldwake-ai agent_tick::portfolio agent_tick::planning` was invalid Cargo syntax because Cargo accepts a single test-name filter. The focused selectors were run separately.
- Additional invalid multi-filter attempts were made while narrowing fallout in `golden_offices`, `motive_source_mapping`, and `ranking` tests. Those attempts did not prove behavior; the valid commands are listed in `Verification Result`.
- Broad gates exposed same-seam fixes in `feasibility_probe`, `motive_source_mapping`, `ranking`, and `golden_motive_sources` beyond the initial portfolio/planning files.
- Plan-cap reads still use `CognitiveProfile.max_candidates_to_plan`; ticket 008 remains the owner for replacing that path with `PortfolioWeightsProfile.max_plans_<mode>`.

## Verification Result

- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-ai agent_tick::portfolio`.
- Passed `cargo test -p worldwake-ai agent_tick::planning`.
- Passed `cargo test -p worldwake-ai agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning`.
- Passed `cargo test -p worldwake-ai feasibility_probe::tests`.
- Passed `cargo test -p worldwake-ai --test golden_offices`.
- Passed `cargo test -p worldwake-ai --test golden_portfolio_planning portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal`.
- Passed `cargo test -p worldwake-ai motive_source_mapping::tests`.
- Passed `cargo test -p worldwake-ai ranking::tests`.
- Passed `cargo test -p worldwake-ai --test golden_motive_sources`.
- Passed `cargo test --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
