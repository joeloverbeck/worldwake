# S152COGARCSEE-005: Spawn-time archetype resolution

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — scenario spawn resolution (`worldwake-cli`)
**Deps**: archive/tickets/S152COGARCSEE-001.md, archive/tickets/S152COGARCSEE-002.md, archive/tickets/S152COGARCSEE-003.md, archive/tickets/S152COGARCSEE-004.md

## Problem

Before this ticket, the scenario schema could author archetype policies and per-agent overrides, but scenario spawn still seeded every agent with default universal profiles. This ticket landed the resolver that turns an archetype assignment policy (or per-agent override) into concrete profile values at agent spawn: determine the archetype deterministically, look up the template, apply deltas to the existing universal profiles, clamp, scale the backoff fields, insert disabled methods, set `CognitiveArchetypeComponent`, and emit `PersonalityAssigned` with the resolved-profile hash. This is the only emission site for the new event.

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Before implementation, `spawn_agent` (`crates/worldwake-cli/src/scenario/mod.rs`) took `(txn, recipes, agent_def, names, agent_locations)` — no assignment context, no index — and applied universal profiles via `unwrap_or_default()`. The agent-spawning loop used `for agent_def in &def.agents`, with no `enumerate`. The simulation `DeterministicRng` was constructed after the spawn loop. The landed implementation threads a stable `agent_index` plus an archetype assignment context into `spawn_agent`.
2. Existing inline spawn tests that exercised universal-profile application were extended or updated for archetype-resolved profile values: `test_spawn_agents_receive_default_universal_profiles`, `test_spawn_agent_applies_authored_opportunity_profiles`, `test_spawn_agent_applies_authored_s151_profiles`, and `test_spawn_agent_with_profile_overrides`.
3. Mixed-layer boundary under audit: scenario-load resolution writes authoritative profile components + the `CognitiveArchetypeComponent` (ticket 002) and emits the `PersonalityAssigned` event (ticket 003). The resolved-profile hash uses `canonical::hash_serializable` over the archetype-affected profiles (`canonical.rs:51`).
4. (Cumulative arithmetic) The resolver applies signed deltas to `Permille` fields and clamps to [0,1000]; integer fields (`max_plan_depth: u8`, `observation_budget: u8`, `ask_memory_retention_ticks: u32`) saturate at their non-negative ranges. `backoff_ticks_scale: BackoffScalePermille` multiplies each current `*_backoff_ticks`/`*_block_ticks` field on `CognitiveProfile`: `scaled = orig * scale / 1000`. Per-agent sub-seed is derived by hashing `def.seed` with `agent_index`, not naked addition (FND-2).
5. (Heuristic/precedence) Explicit author profile values are the base that archetype deltas modify; if no profile value is authored, the profile default is the base. `AgentDef.archetype` (Explicit) bypasses the policy draw.
6. (Scenario isolation) Determinism requires a stable per-agent index independent of `BTreeMap` iteration; use the `def.agents` slice position as `agent_index`.

## Architecture Check

1. A dedicated assignment-seed derivation before the spawn loop, keyed by canonical hashing of `def.seed` + stable index, gives reproducible assignment without entangling the simulation RNG ordering. Applying deltas to existing fields means the resolved values flow through the unchanged consuming systems — no new runtime read path.
2. No backwards-compatibility shim: the default policy is applied when none is authored (FND-28), not a legacy "all-identical" path.

## Verified Layers

1. Same scenario + seed → identical archetype per agent → identical resolved profile values -> `test_spawn_archetype_assignment_is_deterministic_for_seed`.
2. Resolved deltas land on the correct existing profile fields with clamping/saturation -> focused spawn assertions over `CognitiveProfile`, `RiskWeightProfile`, `PerceptionProfile`, S151 profiles, and authored-profile bases.
3. `PersonalityAssigned` emitted exactly once per agent with `source` and hash -> `test_spawn_applies_archetype_deltas_and_emits_assignment_events`.
4. Explicit `AgentDef.archetype` override pins the archetype regardless of policy -> `test_spawn_explicit_archetype_override_ignores_policy`.

## Landed Changes

### 1. Thread assignment context + index into spawn

The agent loop now uses `for (agent_index, agent_def) in def.agents.iter().enumerate()` and passes a `SpawnAgentContext` containing the scenario seed, effective archetype policy, and recipe registry into `spawn_agent`.

### 2. Resolve the archetype

If `agent_def.archetype` is `Some`, spawn uses it (`source = Explicit`). Otherwise spawn selects from the effective `ArchetypeAssignmentPolicy` (`def.archetype_assignment_policy.clone().unwrap_or_default()`) using a per-agent sub-seed derived by canonical hashing of `(def.seed, agent_index)` (`source = Policy(policy)`). This landed as direct deterministic hash-based selection rather than a mutable assignment RNG.

### 3. Apply the template

Spawn looks up `template_for(archetype)`, applies each delta to the authored-or-default base of the target profile, clamps/saturates the result, applies `backoff_ticks_scale` to current cognitive backoff/block tick fields, inserts `method_disable` ids into `AgentSchemaContextProfile.disabled_methods`, and sets the resolved profile components.

### 4. Set component and emit event

Spawn sets `CognitiveArchetypeComponent { archetype }`, computes `resolved_profile_hash` via `hash_serializable` over the resolved archetype-affected profiles, and emits one `PersonalityAssigned` event per spawned agent carrying `PersonalityAssignedPayload { agent, archetype, seed, source, resolved_profile_hash }`.

## Landed Files

- `crates/worldwake-cli/src/scenario/mod.rs` — spawn loop, `spawn_agent` signature/body, resolver helpers, event emission, focused spawn tests, and updated existing spawn-profile assertions.
- `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` — refreshed observer decision-history fixture for the now-seeded survival baseline.

## Out of Scope

- The `EventTag` variant / `EventPayload` field (ticket 003) and component registration (ticket 002).
- Observer rendering (006) and diagnostics (007).
- Any change to consuming systems — they read the resolved existing fields unchanged.

## Acceptance Result

### Passed Acceptance Checks

1. Passed: two agents under `DefaultUniformFive` with the same seed get identical archetypes and identical resolved profile values across runs.
2. Passed: a Cautious agent's resolved backoff ticks exceed a Bold agent's; a Cautious agent's resolved `RiskWeightProfile.threat_aversion` exceeds default.
3. Passed: `AgentDef.archetype: Some(Greedy)` yields archetype Greedy with `source = Explicit`, ignoring the weighted policy.
4. Passed: exactly one `PersonalityAssigned` event per agent is emitted at spawn.
5. Passed: authored profile values are used as the base before archetype deltas are applied.
6. Passed: existing CLI suite via `cargo test -p worldwake-cli`.

### Invariants

1. Assignment is deterministic and replayable from `def.seed` + agent index (FND-2); no naked-addition seed derivation.
2. Resolved `Permille` fields stay within [0,1000]; integer fields stay non-negative (clamped).
3. Archetype changes no action set or affordance — only resolved profile values (FND-19).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-cli/src/scenario/mod.rs` (`#[cfg(test)]`) — added determinism, delta application, explicit override, single-emission, and resolved-profile-hash assertions.
2. `test_spawn_agents_receive_default_universal_profiles` now asserts default-policy archetype resolution and resolved universal profile values.
3. Existing profile-override tests now assert the authored base plus archetype delta behavior.
4. Observer decision-history fixture was refreshed because S152 default archetype assignment intentionally changes survival-baseline behavior.

### Commands Run

1. Passed `cargo test -p worldwake-cli scenario -- --nocapture`
2. Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
3. Passed `cargo test -p worldwake-cli`

## Outcome

Completed on 2026-05-20.

- Landed deterministic spawn-time archetype assignment for scenario agents, including explicit override handling, default-policy selection, template resolution, profile delta application, clamping/saturation, method-disable insertion, `CognitiveArchetypeComponent` updates, and `PersonalityAssigned` event emission.
- The per-agent assignment seed is derived by canonical hashing of `(scenario_seed, agent_index)` rather than by constructing a mutable assignment RNG. This preserves deterministic replayability and keeps assignment independent from the simulation RNG.
- Refreshed the observer decision-history fixture for `survival-baseline.ron` because default archetype diversity intentionally changes the baseline's early decision trace.

## Verification Result

- Passed `cargo test -p worldwake-cli scenario -- --nocapture`
- Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
- Passed `cargo test -p worldwake-cli`
