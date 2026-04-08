# S75BELVDECOM-006: Extract SocialBeliefView + PoliticalBeliefView sub-traits

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — RuntimeBeliefView trait decomposition (largest batch: 36 methods)
**Deps**: S75BELVDECOM-001

## Problem

Extract SocialBeliefView (18 methods: beliefs, observations, tell, communication, epistemic) and PoliticalBeliefView (18 methods: offices, factions, loyalty, institutional, justice, violations) from RuntimeBeliefView. This is the largest extraction batch — the two domains together account for 36 of the original 113 methods.

## Assumption Reassessment (2026-04-08)

1. SocialBeliefView methods confirmed (18): `agent_belief_store`, `known_entity_beliefs`, `known_social_observations`, `believed_activity_of`, `agents_active_at`, `tell_profile`, `told_belief_memories`, `told_belief_memory`, `recipient_knowledge_status`, `ask_witness_memory`, `belief_confidence_policy`, `observation_fidelity`, `source_reliability`, `expectation_store`, `last_seen_memory`, `epistemic_disposition_profile`, `theft_disposition_profile`, `intention_disposition_profile`.
2. PoliticalBeliefView methods confirmed (18): `known_institutional_beliefs`, `factions_of`, `bandit_factions_of`, `locally_observed_bandit_camp_faction_at`, `violation_disposition_profile`, `active_violation_records`, `record_data`, `office_data`, `believed_office_holder`, `believed_force_controller`, `believed_membership`, `believed_faction_rally_point`, `offices_contested_by`, `loyalty_to`, `believed_support_declaration`, `believed_support_declarations_for_office`, `institutional_belief_claims`, `justice_disposition_profile`.
3. `justice_disposition_profile` is assigned to PoliticalBeliefView (not Social) per spec. `theft_disposition_profile` remains in Social.

## Architecture Check

1. Same supertrait pattern. No backward-compatibility shims.
2. This is the largest ticket in the decomposition. Despite its size, the work is mechanical (move method signatures, split impl blocks). No behavioral changes.

## Verification Layers

1. Social queries -> golden tests exercise belief store, tell, and social observation queries
2. Political queries -> golden tests exercise office, faction, and institutional queries
3. Compile-time proof -> `cargo build --workspace`

## What to Change

### 1. Define SocialBeliefView and PoliticalBeliefView sub-traits

Move 18 social and 18 political method signatures from RuntimeBeliefView.

### 2. Add supertrait bounds and remove methods from RuntimeBeliefView

After this ticket, RuntimeBeliefView's body should be empty — all 113 methods have been moved to sub-traits (assuming tickets 002-005 are also complete). RuntimeBeliefView becomes purely compositional.

### 3. Update all 18 impl blocks

### 4. Export new sub-traits

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify — exports)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- All 16 test mock files (modify)

## Out of Scope

- Other domain sub-trait extractions (should be complete by this point)
- SnapshotEntity sub-struct decomposition (ticket 007)
- GoalBeliefView changes (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `&dyn RuntimeBeliefView` usable at all existing call sites.
2. No behavioral change.
3. After all tickets 001-006: RuntimeBeliefView body is empty; all 113 methods live on 11 sub-traits.

## Test Plan

### New/Modified Tests

1. None — pure structural refactor.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
