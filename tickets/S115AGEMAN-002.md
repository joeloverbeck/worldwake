# S115AGEMAN-002: Agenda types + GroundedGoal/RankedGoal/ActiveGoal migration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — renames `GroundedGoal`→`GoalOffer` and `RankedGoal`→`AgendaEntry`; removes `ActiveGoal` component; adds `AgendaState` to `AgentDecisionRuntime`; bumps save format version
**Deps**: [specs/S115-agenda-manager.md](../specs/S115-agenda-manager.md) D1+D2

## Problem

S115 introduces an agenda lifecycle that requires: (a) richer candidate structs carrying lifecycle information (`GoalOffer` absorbing the old `GroundedGoal` plus obligation/invalidator fields), (b) a unified entry type (`AgendaEntry` absorbing `RankedGoal` scoring fields plus lifecycle phase/origin/triggers), and (c) a single authority for the committed goal (`AgendaState.committed` replacing the now-redundant `ActiveGoal` component). Per FND-28, these must land as one atomic migration — two live authoritative representations of the agent's committed goal cannot coexist. The renames touch 216 `GroundedGoal` + 52 `RankedGoal` construction sites across `worldwake-ai`, and the `ActiveGoal` removal crosses 20 files across 4 crates. Splitting the migration across tickets would leave the workspace non-compiling at intermediate states, violating the workspace-builds-after-each-ticket constraint.

## Assumption Reassessment (2026-04-22)

1. `GroundedGoal` is defined at `crates/worldwake-ai/src/goal_model.rs:2306` with fields `key: GoalKey`, `anchor: OpportunityAnchor`, `evidence_entities: BTreeSet<EntityId>`, `evidence_places: BTreeSet<EntityId>`. Construction sites: 216 across 24 files in `worldwake-ai`. No construction sites in other crates.
2. `RankedGoal` is defined at `crates/worldwake-ai/src/goal_model.rs:2528` with fields `grounded: GroundedGoal`, `priority_class: GoalPriorityClass`, `motive_score: u32`, `provenance: Option<RankedGoalProvenance>`, `source_reliability_discount: Option<SourceReliabilityDiscount>`, `competition_discount: Option<CompetitionDiscount>`, `feasibility: FeasibilityHint`. Construction sites: 52 across 18 files (ai + 1 in cli). `OrderedRanked<'a>` at `crates/worldwake-ai/src/ranking.rs:69` wraps `&[RankedGoal]` and must be updated to wrap `&[AgendaEntry]`.
3. `ActiveGoal` at `crates/worldwake-core/src/intention.rs:14` has fields `goal_key: GoalKey`, `adopted_at: Tick`. ECS registration at `crates/worldwake-core/src/component_schema.rs:1693-1716`. Consumer blast radius: 7 files in worldwake-core, 1 in worldwake-systems, 11 in worldwake-ai, 1 in worldwake-cli — 20 files / 4 crates. Key consumer functions take `&mut Option<ActiveGoal>` parameters; migration rewires them to `&mut AgendaState` or direct `AgentDecisionRuntime` access.
4. The shared boundary under audit is the serialized shape of `AgentDecisionRuntime` (`crates/worldwake-ai/src/decision_runtime.rs:151`). Adding `AgendaState` as a field changes its bincode layout. `SAVE_FORMAT_VERSION` at `crates/worldwake-sim/src/save_load.rs:6` is currently `40`; must bump to `41` and verify `load_current_format` handles the new shape.
5. `AgendaEntry` new fields (absorbing `RankedGoal` scoring data plus lifecycle data) mean **every** existing `RankedGoal {` construction site must supply the new lifecycle fields. Default lifecycle values: `phase: AgendaPhase::Pending`, `origin: AgendaOrigin::NeedDrive` (caller refines when specific origin is known), `introduced_tick` and `last_reconsidered_tick` from current tick, `revival_trigger: None`, `kill_condition: KillCondition::External`. Ranking-emitting code can use these safe defaults; commit/revival sites set phase/origin explicitly.
6. `AgendaState` new fields: `committed: Option<AgendaEntry>`, `pending: BTreeMap<AgendaEntryKey, AgendaEntry>`, `suspended: BTreeMap<AgendaEntryKey, AgendaEntry>`. `Default::default()` yields all-empty and `committed: None` — compatible with save/load default-on-migration.
7. `AgendaEntryKey` is a type alias: `pub type AgendaEntryKey = worldwake_core::OpportunityKey;` (per spec D1; `OpportunityKey` already exists in core and is the key used for committed-goal tracking in `build_candidate_plans`).
8. `AgendaOrigin::Obligation { artifact }`, `AgendaOrigin::SocialCommitment { expectation }`, `AgendaOrigin::Opportunity { evidence }` fields reference `EntityId` / `ExpectationId` which are in core — no new core types needed.
9. Adjacent contradictions: migrating `ActiveGoal` removes `set_component_active_goal` / `get_component_active_goal` / `has_component_active_goal` macro-generated accessors. Blast-radius confirmed via workspace-wide grep: 7 core + 1 systems + 11 ai + 1 cli = 20 files. All are in-scope for this ticket.
10. `SAVE_FORMAT_VERSION` bump follows the existing bump pattern at `save_load.rs:6`. No save migration helper is expected for new fields with `Default` — bincode + serde default handle absent fields on load.

## Architecture Check

1. Atomic rename + removal keeps workspace-compiles invariant per ticket. Any split would require either (a) temporary alias shim (violates FND-28) or (b) intermediate broken state (violates ticket authoring contract). The migration is mechanical — type names change, new fields default — so Large effort is review cost, not design cost.
2. `AgendaState` lives inside `AgentDecisionRuntime` (embedded field), which is already serde-persisted per-agent. No new storage mechanism is introduced; the precedent is established and tested (`decision_runtime.rs:438` asserts "not-a-component"). This aligns with S115's deliberate rejection of ECS-component placement for `AgendaState` (which would require moving the type to core and cascading `Invalidator` out of ai).
3. `ActiveGoal` removal eliminates FND-28 violation: post-ticket there is one authoritative representation of the committed goal (`AgendaState.committed`). The spec's FND-28 alignment row depends on this removal landing in this ticket.

## Verification Layers

1. Rename correctness — `cargo check --workspace` compiles with zero `GroundedGoal` / `RankedGoal` symbols remaining (grep-verified) and all call sites use the new names.
2. `ActiveGoal` removal — grep workspace-wide for `ActiveGoal`, `get_component_active_goal`, `set_component_active_goal`, `has_component_active_goal`, `insert_component_active_goal`, `remove_component_active_goal`, `iter_active_goals`, `entities_with_active_goal`, `query_active_goal`, `count_with_active_goal`: zero matches in production paths (`cfg(test)` excluded only where the test itself is migrated to `AgendaState.committed`).
3. Save/load stability — `SAVE_FORMAT_VERSION` bumped to `41`; bincode round-trip test `save_load_round_trip_preserves_agent_decision_runtime` (or equivalent) validates new shape.
4. Field absorption — unit assertion that `AgendaEntry { .., introduced_tick, .. }` populated at commit time matches what `ActiveGoal.adopted_at` held pre-migration. Spot-check via migration of `cargo_satisfaction_at_destination_while_carrying` test (at `crates/worldwake-ai/src/agent_tick/tests.rs:4710`) — updated to read `AgendaState.committed` instead of `get_component_active_goal`.
5. Behavior-preservation — existing golden tests (`cargo test -p worldwake-ai`) all pass with identical outcomes. The rename must not change agent decisions.

## What to Change

### 1. Define agenda types in `worldwake-ai`

Create `crates/worldwake-ai/src/agenda_types.rs` (or inline in `goal_model.rs` — choose based on file-size balance):

- `AgendaState { committed: Option<AgendaEntry>, pending: BTreeMap<AgendaEntryKey, AgendaEntry>, suspended: BTreeMap<AgendaEntryKey, AgendaEntry> }` with `Default`.
- `AgendaEntry` with fields: `key: AgendaEntryKey`, `offer: GoalOffer`, `phase: AgendaPhase`, `origin: AgendaOrigin`, `introduced_tick: Tick`, `last_reconsidered_tick: Tick`, `revival_trigger: Option<RevivalTrigger>`, `kill_condition: KillCondition`, and the absorbed scoring fields (`priority_class`, `motive_score`, `provenance`, `source_reliability_discount`, `competition_discount`, `feasibility`).
- `AgendaPhase { Committed, Pending, Suspended }` — `Copy`, derivable.
- `AgendaOrigin { NeedDrive, Obligation { artifact: EntityId }, SocialCommitment { expectation: ExpectationId }, Opportunity { evidence: EntityId }, Exploration, Enterprise }`.
- `RevivalTrigger { CommodityAvailable { place, kind, min }, TargetPresent { target, place }, RouteLearned { from, to }, CounterpartyAvailable { counterparty, place }, TickElapsed { at_tick } }`.
- `KillCondition { TickExpiry { at_tick }, ObligationResolved { expectation }, TargetDead { target }, External }`.
- `pub type AgendaEntryKey = worldwake_core::OpportunityKey;`

All types derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Enums also derive `Ord, PartialOrd` where BTreeMap keying requires.

Re-export from `crates/worldwake-ai/src/lib.rs`.

### 2. Rename `GroundedGoal` → `GoalOffer` with new fields

At `crates/worldwake-ai/src/goal_model.rs:2306`: rename struct and add fields:

```rust
pub struct GoalOffer {
    pub key: GoalKey,
    pub anchor: OpportunityAnchor,
    pub evidence_entities: BTreeSet<EntityId>,
    pub evidence_places: BTreeSet<EntityId>,
    pub obligation_source: Option<EntityId>,
    pub commitment_impact_if_ignored: Permille,
    pub required_information_gaps: Vec<BeliefClaimKey>,
    pub invalidators: Vec<Invalidator>,
    pub learned_expectation_refs: Vec<ExpectationId>,
}
```

Migrate all 216 construction sites across `worldwake-ai`. Sites that don't carry obligation/invalidator context provide empty defaults (`None`, `Permille::ZERO`, `Vec::new()`) — these are the evidence path's natural values when no obligation signal is present. Sites that DO carry such context (obligation-driven candidate emission, invalidator-aware generation) set the fields explicitly.

### 3. Rename `RankedGoal` → `AgendaEntry` with absorbed fields

At `crates/worldwake-ai/src/goal_model.rs:2528`: rename struct to `AgendaEntry`, drop the old `grounded: GroundedGoal` composition (it becomes `offer: GoalOffer`), absorb all scoring fields flat, and add lifecycle fields from Change 1 above. Migrate all 52 construction sites.

Update `OrderedRanked<'a>` at `crates/worldwake-ai/src/ranking.rs:69` to wrap `&'a [AgendaEntry]`. Rename is internal to the wrapper — callers see the method surface unchanged. Update `ranking::sort_in_place` signature: `pub fn sort_in_place(entries: &mut Vec<AgendaEntry>) -> OrderedRanked<'_>`.

### 4. Remove `ActiveGoal` component

- Delete `crates/worldwake-core/src/intention.rs` `ActiveGoal` struct (keep `IntentionFrame` if that file contains both; otherwise delete the file).
- Remove the `active_goals` registration block at `crates/worldwake-core/src/component_schema.rs:1693-1716`.
- Remove the `ActiveGoal` re-export from `crates/worldwake-core/src/lib.rs`.
- Migrate all 20 consumer files to read `AgendaState.committed` from the agent's `AgentDecisionRuntime`:
  - `crates/worldwake-ai/src/agent_tick/planning.rs:1103,1152,1184,1364` — parameters `active_goal: &mut Option<ActiveGoal>` become reads/writes on `AgendaState.committed`.
  - `crates/worldwake-ai/src/agent_tick/execution.rs:694-711` — `set_component_active_goal` writes become mutations on the runtime-map `AgendaState.committed`.
  - `crates/worldwake-ai/src/agent_tick/active_action.rs:42,56,206,240` — same migration pattern.
  - `crates/worldwake-ai/src/agent_tick/observation.rs:472` — same.
  - All `cfg(test)` sites in these files update to the new pattern.

### 5. Add `agenda_state: AgendaState` to `AgentDecisionRuntime`

At `crates/worldwake-ai/src/decision_runtime.rs:151`, add field:

```rust
#[serde(default)]
pub agenda_state: AgendaState,
```

`#[serde(default)]` allows save files without the field (pre-migration saves) to load with empty agenda. `Default` impl on `AgendaState` yields empty maps and `committed: None`.

Update the "is_not_registered_as_a_component" test at `decision_runtime.rs:438` only if the test implementation lists fields individually — otherwise no change.

### 6. Bump SAVE_FORMAT_VERSION

At `crates/worldwake-sim/src/save_load.rs:6`, change `40` → `41`. Update `load_current_format` version match if needed.

## Files to Touch

- `crates/worldwake-ai/src/agenda_types.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — re-exports)
- `crates/worldwake-ai/src/goal_model.rs` (modify — rename + field additions)
- `crates/worldwake-ai/src/ranking.rs` (modify — `OrderedRanked<'_>` + `sort_in_place` over `AgendaEntry`)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add `agenda_state` field)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — 216 construction sites)
- `crates/worldwake-ai/src/agent_tick/{planning,execution,active_action,observation,mod,tests}.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (modify — `RankedGoal` → `AgendaEntry` references)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — decision-runtime usage)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — `RankedGoal` references)
- `crates/worldwake-ai/src/feasibility_probe.rs` (modify — `RankedGoal` parameter type)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — `RankedGoal` references)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — `RankedGoal` / decision-runtime references)
- `crates/worldwake-ai/src/plan_guard.rs` (modify if it references either type)
- Remaining `worldwake-ai` files from the blast-radius grep (target list: every file in `cargo grep "GroundedGoal\|RankedGoal"`)
- `crates/worldwake-ai/tests/*.rs` (modify — all goldens and integration tests using either type)
- `crates/worldwake-core/src/intention.rs` (modify — remove `ActiveGoal`)
- `crates/worldwake-core/src/component_schema.rs` (modify — remove `active_goals` block)
- `crates/worldwake-core/src/lib.rs` (modify — remove `ActiveGoal` re-export)
- `crates/worldwake-core/src/world_txn.rs` (modify if `create_agent()` delta assertion references `ActiveGoal`)
- Other 7 `worldwake-core` files referencing `ActiveGoal` (blast radius)
- 1 `worldwake-systems` file referencing `ActiveGoal`
- `crates/worldwake-cli/src/bin/observer.rs` and any other cli files referencing `RankedGoal` / `ActiveGoal`
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` bump)

## Out of Scope

- `tick_agenda` flow implementation (ticket 003)
- D4A `classify_rejection` and S112 carve-out removal (ticket 004)
- `agenda_tick_system` SystemFn wiring (ticket 005)
- S74 switch-margin reading `AgendaState.committed.motive_score` (ticket 005)
- New unit/integration tests for lifecycle behavior (ticket 006)
- Golden agenda-lifecycle scenario (ticket 007)
- Ranking-scoring algorithm changes — this ticket preserves current ranking behavior byte-for-byte; only the type name changes

## Acceptance Criteria

### Tests That Must Pass

1. `cargo check --workspace` passes.
2. `cargo test --workspace` passes with identical pass/fail outcomes as pre-ticket (no behavioral regression). Ranking tests, feasibility tests, agent-tick tests, and all goldens produce the same decisions.
3. `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying` passes after migrating from `get_component_active_goal` to `runtime.agenda_state.committed.as_ref().map(|entry| entry.key.goal_key)`.
4. `crates/worldwake-ai/tests/golden_portfolio_planning.rs::portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` still passes with no change in outcome.
5. Save/load round-trip: `cargo test -p worldwake-sim -- save_load` passes with the bumped version; bincode serialization of `AgentDecisionRuntime` with populated `AgendaState` is stable.

### Invariants

1. Zero remaining references to `GroundedGoal`, `RankedGoal`, `ActiveGoal`, `get_component_active_goal`, `set_component_active_goal`, `has_component_active_goal`, `insert_component_active_goal`, `remove_component_active_goal`, `iter_active_goals`, `entities_with_active_goal`, `query_active_goal`, `count_with_active_goal` in production code (non-comment, non-migration-history, non-archive).
2. Single authoritative representation of the committed goal: `AgendaState.committed` (FND-28).
3. `AgentDecisionRuntime` serializes deterministically; bincode round-trip preserves `AgendaState`.
4. `AgendaEntryKey == OpportunityKey` (type alias, not a parallel key taxonomy).
5. Ranking output order (by `OrderedRanked<'_>`) is byte-identical to pre-rename for the same input candidate set — this ticket is a rename, not an algorithmic change.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` (new inline `#[cfg(test)]`) — construction + field-roundtrip for `GoalOffer` and `AgendaEntry` including new fields. Bincode round-trip for each.
2. `crates/worldwake-ai/src/agenda_types.rs` (new inline `#[cfg(test)]`) — Default `AgendaState`, Default `AgendaEntry` with lifecycle fields, enum variant serialization.
3. `crates/worldwake-ai/src/decision_runtime.rs` (modify) — extend `agent_decision_runtime_bincode_round_trip_preserves_all_fields` to populate `agenda_state` with one committed + one pending + one suspended entry and verify round-trip.
4. `crates/worldwake-ai/src/agent_tick/tests.rs::cargo_satisfaction_at_destination_while_carrying` (modify) — migrate assertion from `get_component_active_goal` to `AgendaState.committed` read.
5. All other tests touching the renamed types — migrate mechanically.

### Commands

1. `cargo test -p worldwake-ai -- cargo_satisfaction`
2. `cargo test -p worldwake-ai -- goal_model decision_runtime`
3. `cargo test -p worldwake-sim -- save_load`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `./scripts/verify.sh`
