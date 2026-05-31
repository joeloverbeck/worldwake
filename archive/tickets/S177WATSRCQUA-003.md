# S177WATSRCQUA-003: `WaterToleranceProfile` universal per-agent component

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes - `worldwake-core` component registration and agent seeding, `worldwake-sim` profile belief view accessor and save format bump, `worldwake-cli` scenario authoring, generated profile docs.
**Deps**: `archive/tickets/S177WATSRCQUA-001.md`

## Problem

The spec's D5 deliverable adds per-agent tolerance to water quality: a hardy agent suffers less from muddy water than a fragile agent. Per the FOUNDATIONS-aligned Q3 resolution from reassessment, this lives on a universal `WaterToleranceProfile` component rather than on `MetabolismProfile` or `CommodityConsumableProfile`. Before this ticket, every agent experienced identical water-quality consequences, collapsing the FND-22 agent-diversity surface that the spec's headline scenario depends on.

## Assumption Reassessment

1. The universal-profile contract required registration in `component_schema.rs`, an optional `AgentDef` field, scenario spawn defaults, core `World::create_agent` seeding, save/load coverage, and generated profile documentation.
2. Live `World::create_agent` did not seed every scenario-authored profile, so `MetabolismProfile` was a scenario-spawn precedent but not an exact core seeding precedent. This ticket still needed core seeding because `WaterToleranceProfile` is universal and later authoritative/AI readers rely on an infallible component on agents created through either path.
3. The live belief-view shape uses `ProfileBeliefView` for profile access and forwards through `GoalBeliefView`. The landed accessor follows that split rather than adding a one-off runtime-only method.
4. `WaterToleranceProfile` stores deterministic `BTreeMap<WaterQuality, Permille>` values. The default profile represents the average agent: Clean relief 1000 / dirtiness 0, Stale relief 700 / dirtiness 80, Muddy relief 450 / dirtiness 200.
5. Tickets 004 and 005 consume this surface for source ranking and Drink effects, so this ticket deliberately avoided adding those behavioral reads.

## Outcome

Landed a universal `WaterToleranceProfile` component in `crates/worldwake-core/src/water_tolerance_profile.rs`, exported it from core, registered it on `EntityKind::Agent`, and included it in component delta/value inventories. `World::create_agent` now attaches `WaterToleranceProfile::default()` and the create-agent/world-transaction tests assert that component is present.

Scenario authoring now supports `water_tolerance_profile: Option<WaterToleranceProfile>` on `AgentDef`. `spawn_agent` applies authored overrides or defaults, and the scenario coverage reporter recognizes the field. Existing explicit `AgentDef` literals were updated with `water_tolerance_profile: None`.

The sim layer now exposes `water_tolerance_profile(agent)` through `ProfileBeliefView` and the `GoalBeliefView` forwarding surface. `PerAgentBeliefView` returns the profile only for the actor's self-authoritative scope. `SAVE_FORMAT_VERSION` advanced from 112 to 113, and save/load roundtrip coverage now includes a non-default water tolerance profile.

Generated profile docs include `WaterToleranceProfile`.

## Deviations

The ticket's original reassessment described `MetabolismProfile` as an exact `World::create_agent` seeding precedent. Live code showed that precedent only applied to scenario spawn, so the implementation used the stronger universal-profile requirement as authority and added `WaterToleranceProfile` to both creation paths.

The belief accessor landed on both the canonical `ProfileBeliefView` and the forwarding `GoalBeliefView` surface. That matches the live trait layout and preserves profile-access consistency for future source-ranking work.

`./scripts/verify.sh` was not run for this individual ticket because the `implement-spec-tickets` harness reserves the full pre-PR wrapper for final branch closeout. The full workspace test gate passed for this ticket.

## Verified Acceptance

1. Added `water_tolerance_profile_default_values`, proving default Clean/Stale/Muddy relief and dirtiness values.
2. Added `water_tolerance_profile_accessor_methods_use_neutral_missing_values`, proving configured values and neutral fallback behavior.
3. Added `water_tolerance_profile_serialization_roundtrip`, proving bincode roundtrip for default and customized profiles.
4. Expanded create-agent and world-transaction tests to assert the new component is attached and emitted in creation deltas.
5. Expanded scenario tests to prove unauthored defaults and authored RON/struct overrides flow through `spawn_agent`.
6. Added `water_tolerance_profile_belief_view_returns_self_authoritative`, proving self-scope access through both profile and goal belief surfaces and `None` for another agent.
7. Verified `SAVE_FORMAT_VERSION` is 113 and full non-default save/load roundtrip preserves water tolerance data.
8. Verified deterministic `BTreeMap<WaterQuality, Permille>` storage and no Drink/source-ranking behavior changes in this ticket.

## Verification Result

1. Passed `python3 scripts/profile_docs.py --write` - regenerated `docs/profiles/all-profiles.md`; the script reported pre-existing documentation-gap warnings for unrelated profiles.
2. Passed `cargo fmt --all`.
3. Passed `cargo test -p worldwake-core water_tolerance_profile`.
4. Passed `cargo test -p worldwake-core create_agent`.
5. Passed `cargo test -p worldwake-sim water_tolerance_profile_belief_view_returns_self_authoritative`.
6. Passed `cargo test -p worldwake-cli spawn_agent`.
7. Passed `cargo test -p worldwake-sim save_format_version_is_113_after_water_tolerance_profile`.
8. Passed `cargo test -p worldwake-sim save_to_bytes_roundtrip_preserves_full_nondefault_state`.
9. Passed `cargo test -p worldwake-sim belief_view`.
10. Passed `cargo test -p worldwake-core`.
11. Passed `cargo test -p worldwake-sim`.
12. Passed `cargo test -p worldwake-cli`.
13. Passed `cargo test --workspace`.
14. Waived `./scripts/verify.sh` - final spec-branch closeout owns the full pre-PR wrapper.

## Out of Scope

- Drink reading `WaterToleranceProfile` remains owned by ticket 005.
- Source-rank composite reading tolerance for quality discount remains owned by ticket 004.
- Authored tolerance diversity in named scenarios remains owned by tickets 009 and 010.
- Role-specific tolerance remains outside S177-003; universal default plus scenario overrides cover this ticket's diversity surface.
