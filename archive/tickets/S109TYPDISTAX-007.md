# S109TYPDISTAX-007: Wire repair and learned-opportunity memory semantics

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — define explicit retention substrate for `RepairMemory` / `LearnedOpportunityMemory`, wire writer sites in `worldwake-ai`, and consume both memories during candidate ranking
**Deps**: archive/tickets/S109TYPDISTAX-002.md, specs/S109-typed-discrepancy-taxonomy.md

## Problem

S109 treats `RepairMemory` and `LearnedOpportunityMemory` as real per-agent learning surfaces, not inert containers: successful alternate repairs should be recorded and later boost the alternative; opportunities discovered in transit should be recorded, decay over time, and later influence ranking. T002 only landed the additive component shells. Reassessment during T002 showed those memories lacked retention metadata, so their `expire(current_tick)` methods were implemented as no-op placeholders. The remaining S109 tickets do not own the missing retention contract, writer paths, or reader consumption. If left unresolved, S109 would ship dead authoritative memories whose documented decay and ranking effects never occur.

## Assumption Reassessment (2026-04-19)

1. `RepairMemory` and `LearnedOpportunityMemory` now exist in `worldwake-core` from T002, but their entry shapes only carry `observed_tick`, not retention metadata. Their `expire(current_tick)` methods are currently API-preserving no-ops documented in `tickets/S109TYPDISTAX-002.md`.
2. The active S109 follow-up tickets do not own this slice today. T003 only adds read-only belief-view accessors for the two memories; T004, T005, and T006 do not currently mention writer sites, ranking reads, or real expiry semantics for them.
3. The active S109 spec still describes these memories as semantic surfaces, not placeholders: `specs/S109-typed-discrepancy-taxonomy.md:26` routes successful alternate repairs into `RepairMemory` and discovered opportunities into `LearnedOpportunityMemory`; `:44` says learned opportunities have explicit decay; `:147-158` defines both memory types; `:228` names mutable writer accessors in `failure_handling.rs` and related AI sites; `:296` says candidate generation boosts alternatives via both memories; `:364-365` requires overwrite/eviction behavior.
4. Shared abstraction boundary under audit: authoritative stored state in `crates/worldwake-core/src/repair_memory.rs` and `learned_opportunity_memory.rs`; local read/write threading inside `worldwake-ai` through `agent_tick/mod.rs`, `agent_tick/observation.rs`, `agent_tick/active_action.rs`, `agent_tick/execution.rs`, and `ranking.rs`. The current branch does not yet expose these memories through `GoalBeliefView`, and the live ranking seam is `rank_candidates` / `ranked_motive_score` in `ranking.rs`, not a `preferred_operator_boost` helper.
5. `archive/tickets/S109TYPDISTAX-002.md` is complete and already landed the additive component shells. The active `Deps` line previously pointed at the stale pre-archive path and listed T003/T004 as blockers, but the live branch does not actually need either sibling ticket to land the narrow memory-semantics slice honestly.
6. T003's drafted belief-view accessor work is larger than what this branch needs today because `GoalBeliefView` currently uses blanket forwarding in `worldwake-sim`. Widening that shared trait surface just to read two local runtime memories would be extra cross-crate fallout, not the narrowest truthful implementation seam.
7. T004's blocker/discrepancy classifier migration is not required to own the narrower writer semantics here. `RepairMemory` can be recorded on successful alternate completion using the selected plan/opportunity the runtime already knows, and `LearnedOpportunityMemory` can be recorded from read-phase candidate discovery while the agent is in transit without waiting for the discrepancy split.
13. Adjacent contradiction classification: this is a required consequence of T002, not optional cleanup. The additive substrate is present, but the live ticket set currently has no owner for the behavior that justifies those memories existing.
14. Mismatch + correction: the drafted S109 ticket sequence omitted the implementation slice that turns these memories from empty storage into functioning learning surfaces. This ticket is added to own that missing scope explicitly.
15. Retention math is currently undefined in live code for these memories. This ticket must make it explicit with profile-driven TTL knobs and stored expiry state rather than inferring decay from ad hoc read-time heuristics.

## Architecture Check

1. Explicit per-entry retention state plus profile-driven TTLs is the cleanest shape because it keeps the dampener concrete in authoritative state, aligns with S109's FND-11 / FND-22A claims, and lets later readers answer "is this still a live learned fact?" without reconstructing history from unrelated traces.
2. No backward-compatibility aliasing or shadow paths. The placeholder no-op semantics from T002 should be replaced directly; do not add alternate legacy readers or fallback memory paths.

## Verification Layers

1. Retention, overwrite, and eviction semantics for `RepairMemory` / `LearnedOpportunityMemory` -> focused unit tests in `worldwake-core`.
2. Successful alternate-resolution path records a `RepairMemory` entry -> focused runtime/AI test at the writer boundary.
3. Observation of a new opportunity during another task records a `LearnedOpportunityMemory` entry -> focused runtime/AI or observation-path test at the writer boundary.
4. Candidate ranking / alternative selection consults these memories and changes preference ordering only while entries are live -> focused candidate-generation or ranking tests, plus the strongest available decision-trace assertion when the reader effect is easier to prove through planner output than through internal helper state alone.
5. Single golden/E2E addition is warranted only if the final reader/writer path spans more than one local decision boundary; otherwise focused runtime coverage is the primary proof surface.

## What to Change

### 1. Define the missing retention contract

Update the active S109 contract and code together so `RepairMemory` and `LearnedOpportunityMemory` have explicit, profile-driven retention semantics rather than no-op `expire` methods. The implementation must store enough authoritative state to support real expiry. If the live spec still lacks the exact fields or TTL knobs required, correct `specs/S109-typed-discrepancy-taxonomy.md` in-scope before landing code.

Concretely, this ticket should:

- add explicit retention data to `RepairEntry` and `OpportunityEntry` (for example an `expires_tick` field or an equivalent stored expiry contract),
- add per-agent TTL knobs for these memories to the appropriate profile surface,
- make `expire(current_tick)` remove truly stale entries instead of acting as a placeholder.

### 2. Wire writer sites for learned repairs and opportunities

Find the real authoritative writer boundaries in `worldwake-ai` and record these memories there:

- when an agent completes a `GoalSatisfied` / `CombatCommitment` plan through a concrete alternate opportunity anchor, record/update `RepairMemory` at the plan-completion boundary using the successful selected opportunity and any still-live blocker context for the same goal,
- when an agent is in transit for one goal and candidate generation surfaces a different concrete opportunity, record/update `LearnedOpportunityMemory` from the read-phase result during the write phase,
- preserve overwrite-on-fresher-observation semantics and capacity enforcement from T002.

Do not invent a parallel event-log-only path; these memories are the per-agent authoritative learning surface.

### 3. Consume both memories during candidate ranking

Update the AI readers so these memories actually matter:

- ranking should boost a viable alternative when `RepairMemory` says that alternate previously succeeded,
- ranking should consider live learned opportunities from `LearnedOpportunityMemory` where the S109 spec expects opportunistic reuse,
- expired entries must stop influencing ranking immediately once their retention window lapses.

## Files to Touch

- `specs/S109-typed-discrepancy-taxonomy.md` (modify — only if needed to make the retention contract explicit before code)
- `crates/worldwake-core/src/repair_memory.rs` (modify)
- `crates/worldwake-core/src/learned_opportunity_memory.rs` (modify)
- `crates/worldwake-core/src/cognitive_profile.rs` or other owning profile surface (modify — add TTL knobs if this is the chosen contract)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — local memory threading and lifecycle)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — if plan-completion repair recording lands here)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — if observation-time opportunity recording lands here)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — persistence of the two memories)
- `crates/worldwake-ai/src/ranking.rs` (modify — read-side integration)
- `crates/worldwake-ai/src/goal_explanation.rs` (modify — if `rank_candidates` signature changes)

## Out of Scope

- The additive type/component introduction from T002.
- The blocker/discrepancy classifier migration from T004.
- Trace-struct renames from T005.
- Removal of `BlockingFact::Unknown` / `AssumptionFailed` from T006.
- Scenario-authored initialization of these memories; they remain runtime-generated universal components.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core repair_memory`
2. `cargo test -p worldwake-core learned_opportunity_memory`
3. `cargo test -p worldwake-ai candidate_generation`
4. `cargo test -p worldwake-ai agent_tick`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`

### Invariants

1. `RepairMemory` and `LearnedOpportunityMemory` no longer have placeholder no-op expiry semantics in live code.
2. Their retention windows are profile-driven and stored concretely enough that liveness does not depend on hidden heuristics or wall-clock time.
3. Reader paths only benefit from entries that are still live; expired entries cannot continue to bias ranking.
4. No duplicate learning path is introduced outside these memories for the same repair/opportunity fact.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/repair_memory.rs` `#[cfg(test)]` — add or extend tests for real expiry, fresher-entry overwrite, and capacity eviction.
2. `crates/worldwake-core/src/learned_opportunity_memory.rs` `#[cfg(test)]` — add or extend tests for real expiry, fresher-entry overwrite where applicable, and capacity eviction.
3. `crates/worldwake-ai/src/ranking.rs` — add focused tests proving live entries boost alternatives and expired entries stop doing so.
4. `crates/worldwake-ai/src/agent_tick/...` writer-boundary tests — add focused tests proving repair/opportunity entries are recorded at the intended authoritative boundary.
5. `crates/worldwake-ai/src/ranking.rs` focused filters — run the new repair/opportunity liveness tests directly so the landed read seam is proven at the exact helper boundary, not only through broader crate filters.

### Commands

1. `cargo test -p worldwake-core repair_memory`
2. `cargo test -p worldwake-core learned_opportunity_memory`
3. `cargo test -p worldwake-ai candidate_generation`
4. `cargo test -p worldwake-ai agent_tick`
5. `cargo test -p worldwake-ai repair_memory_boosts_matching_alternative_only_while_live`
6. `cargo test -p worldwake-ai learned_opportunity_memory_boosts_matching_opportunity_only_while_live`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test --workspace`

## Outcome

Implemented the missing repair/opportunity memory semantics end to end. `RepairEntry` and `OpportunityEntry` now carry authoritative `expires_tick` state, `expire(current_tick)` prunes stale entries, and `CognitiveProfile` now owns `repair_memory_ticks` / `learned_opportunity_memory_ticks` defaults (`120` / `60`). The active S109 spec was updated to document that retention contract.

On the AI side, the live seam landed as local `worldwake-ai` memory threading rather than `GoalBeliefView` widening. `process_agent` now loads, expires, and persists both memories; successful terminal plan completion records `RepairMemory` when a concrete alternate succeeds against still-live blocker context for the same goal; in-transit read phases record `LearnedOpportunityMemory` when a different concrete opportunity is discovered. The read-side effect landed in `ranking.rs`, where live repair/opportunity entries add a bounded motive bonus and expired entries stop affecting ordering immediately.

Adding `repair_memory_ticks` / `learned_opportunity_memory_ticks` to `CognitiveProfile` also caused the expected shared-literal fallout: existing explicit `CognitiveProfile { ... }` construction sites, delta samples, and core/AI test helpers were updated so the new profile contract compiles and the broadened workspace verification remains truthful.

## Deviations

1. The landed read seam is `ranking.rs` (`rank_candidates_with_memories` / `ranked_motive_score`), not a `preferred_operator_boost` hook.
2. This ticket did not widen `GoalBeliefView` with repair/opportunity accessors. Reassessment showed that was extra cross-crate fallout on this branch, so the memories are threaded locally inside `worldwake-ai` instead. T003 remains additive follow-up work, but it is no longer a blocker for the honest T007 slice.
3. Repair recording is intentionally narrow: only successful `GoalSatisfied` / `CombatCommitment` completions with a concrete anchor record a repair, and only when there is still-live blocker context for the same goal but a different place/target. This avoids inventing a broader “repair” classifier before T004 lands.

## Verification Result

- Passed `cargo test -p worldwake-core repair_memory`
- Passed `cargo test -p worldwake-core learned_opportunity_memory`
- Passed `cargo test -p worldwake-ai candidate_generation`
- Passed `cargo test -p worldwake-ai agent_tick`
- Passed `cargo test -p worldwake-ai repair_memory_boosts_matching_alternative_only_while_live`
- Passed `cargo test -p worldwake-ai learned_opportunity_memory_boosts_matching_opportunity_only_while_live`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
